use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_ESCAPE, VK_F4, VK_LWIN, VK_RWIN, VK_TAB,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, WH_KEYBOARD_LL,
};
use winreg::enums::*;
use winreg::RegKey;

static mut HOOK_LL: isize = 0;
static mut BLOCK_WIN_KEYS: bool = false;
static mut BLOCK_ALT_TAB: bool = false;
static mut BLOCK_ALT_F4: bool = false;
static mut BLOCK_CTRL_ESC: bool = false;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    exam_url: Option<String>,
    fullscreen: Option<bool>,
    always_on_top: Option<bool>,
    disable_taskmgr: Option<bool>,
    disable_lockworkstation: Option<bool>,
    disable_change_password: Option<bool>,
    block_win_keys: Option<bool>,
    block_alt_tab: Option<bool>,
    block_alt_f4: Option<bool>,
    block_ctrl_esc: Option<bool>,
    enable_state_check: Option<bool>,
    admin_hash: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            exam_url: None,
            fullscreen: Some(true),
            always_on_top: Some(true),
            disable_taskmgr: Some(true),
            disable_lockworkstation: Some(true),
            disable_change_password: Some(true),
            block_win_keys: Some(true),
            block_alt_tab: Some(true),
            block_alt_f4: Some(true),
            block_ctrl_esc: Some(true),
            enable_state_check: Some(true),
            admin_hash: Some("$2y$12$yo8M7GzPhHAQhfw29IXC7OBEU5bQyMmY5BVhiun.SyYpIt8T0C3pS".into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct StateFile {
    status: String, // "normal" or any other value
}

impl StateFile {
    fn normal() -> Self {
        Self {
            status: "normal".into(),
        }
    }

    fn is_normal(&self) -> bool {
        self.status == "normal"
    }
}

/// 固定密钥用于加密状态文件
const STATE_ENCRYPTION_KEY: &[u8; 32] = b"exam_state_encryption_key_v_2026";

/// 加密状态文件
fn encrypt_state_file(state: &StateFile) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new(STATE_ENCRYPTION_KEY.into());

    let nonce_bytes: [u8; 12] = rand::thread_rng().gen();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let json_str = serde_json::to_string(state).map_err(|e| e.to_string())?;
    let ciphertext = cipher
        .encrypt(nonce, json_str.as_bytes())
        .map_err(|e| format!("加密失败: {}", e))?;

    // Format: nonce (12 bytes) + ciphertext
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// 解密状态文件
fn decrypt_state_file(encrypted_data: &[u8]) -> Result<StateFile, String> {
    if encrypted_data.len() < 12 {
        return Err("加密数据格式错误".into());
    }

    let cipher = Aes256Gcm::new(STATE_ENCRYPTION_KEY.into());

    let nonce = Nonce::from_slice(&encrypted_data[0..12]);
    let ciphertext = &encrypted_data[12..];

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("解密失败: {}", e))?;

    let json_str = String::from_utf8(plaintext).map_err(|e| e.to_string())?;
    let state: StateFile = serde_json::from_str(&json_str).map_err(|e| e.to_string())?;

    Ok(state)
}

/// 获取状态文件路径
/// 搜索顺序：
/// 1. 当前工作目录
/// 2. 可执行文件所在目录
/// 3. 可执行文件上一级目录
/// 4. src-tauri 目录
fn get_state_file_path() -> PathBuf {
    // 1. 尝试当前工作目录
    if let Ok(cwd) = env::current_dir() {
        let p = cwd.join(".exam_state");
        if p.parent().map_or(true, |parent| parent.exists()) {
            println!("[state] 从当前目录查找状态文件: {}", p.display());
            return p;
        }
    }

    // 2. 尝试可执行文件目录
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join(".exam_state");
            if p.parent().map_or(true, |parent| parent.exists()) {
                println!("[state] 从exe目录查找状态文件: {}", p.display());
                return p;
            }

            // 3. 尝试可执行文件上一级目录
            if let Some(pdir) = dir.parent() {
                let p2 = pdir.join(".exam_state");
                if p2.parent().map_or(true, |parent| parent.exists()) {
                    println!("[state] 从exe上级目录查找状态文件: {}", p2.display());
                    return p2;
                }
            }
        }
    }

    // 4. 尝试 src-tauri 目录
    if let Ok(cwd) = env::current_dir() {
        let p = cwd.join("src-tauri").join(".exam_state");
        if p.parent().map_or(true, |parent| parent.exists()) {
            println!("[state] 从src-tauri目录查找状态文件: {}", p.display());
            return p;
        }
    }

    println!("[state] 使用默认路径: .exam_state");
    PathBuf::from(".exam_state")
}

/// 检查状态文件是否为"正常"状态
/// 返回 true: 正常退出, false: 不正常（包括文件不存在、无法解密、状态不正常）
fn check_state_file() -> Result<bool, String> {
    let state_path = get_state_file_path();

    // 文件不存在也被视为不正常
    if !state_path.exists() {
        println!("[state] 状态文件不存在，视为异常");
        return Ok(false);
    }

    // 尝试读取和解密状态文件
    let encrypted_data = fs::read(&state_path).map_err(|e| e.to_string())?;

    match decrypt_state_file(&encrypted_data) {
        Ok(state) => {
            println!("[state] 状态文件解密成功，状态: {}", state.status);
            Ok(state.is_normal())
        }
        Err(e) => {
            // 解密失败（数据损坏或被篡改）
            println!("[state] 状态文件解密失败: {}", e);
            Ok(false)
        }
    }
}

/// 设置状态文件为"正常"状态
fn set_state_normal() -> Result<(), String> {
    let state_path = get_state_file_path();
    let state = StateFile::normal();
    let encrypted = encrypt_state_file(&state)?;
    fs::write(&state_path, encrypted).map_err(|e| e.to_string())?;
    println!("[state] 状态文件已设置为正常");
    Ok(())
}

/// 设置状态文件为"异常"状态（启动时使用）
fn set_state_abnormal() -> Result<(), String> {
    let state_path = get_state_file_path();
    let state = StateFile {
        status: "abnormal".into(),
    };
    let encrypted = encrypt_state_file(&state)?;
    fs::write(&state_path, encrypted).map_err(|e| e.to_string())?;
    println!("[state] 状态文件已设置为异常");
    Ok(())
}

/// 从当前工作目录及相关位置读取配置文件
/// 搜索顺序：
/// 1. 当前工作目录
/// 2. 可执行文件所在目录
/// 3. 可执行文件上一级目录
/// 4. src-tauri 目录
fn read_config_from_cwd() -> Result<Config, String> {
    // 搜索位置: 当前目录、可执行文件目录、项目根目录、src-tauri目录
    let mut tried = Vec::new();

    if let Ok(cwd) = env::current_dir() {
        let p = cwd.join("exam_config.json");
        tried.push(p.clone());
        if p.exists() {
            let s = fs::read_to_string(&p).map_err(|e| e.to_string())?;
            let cfg: Config = serde_json::from_str(&s).map_err(|e| e.to_string())?;
            println!("[config] loaded from {}", p.display());
            return Ok(cfg);
        }
    }

    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("exam_config.json");
            tried.push(p.clone());
            if p.exists() {
                let s = fs::read_to_string(&p).map_err(|e| e.to_string())?;
                let cfg: Config = serde_json::from_str(&s).map_err(|e| e.to_string())?;
                println!("[config] loaded from {}", p.display());
                return Ok(cfg);
            }
            // also try parent of exe (one level up)
            if let Some(pdir) = dir.parent() {
                let p2 = pdir.join("exam_config.json");
                tried.push(p2.clone());
                if p2.exists() {
                    let s = fs::read_to_string(&p2).map_err(|e| e.to_string())?;
                    let cfg: Config = serde_json::from_str(&s).map_err(|e| e.to_string())?;
                    println!("[config] loaded from {}", p2.display());
                    return Ok(cfg);
                }
            }
        }
    }

    // try src-tauri folder relative to current exe/cwd
    if let Ok(cwd) = env::current_dir() {
        let p = cwd.join("src-tauri").join("exam_config.json");
        tried.push(p.clone());
        if p.exists() {
            let s = fs::read_to_string(&p).map_err(|e| e.to_string())?;
            let cfg: Config = serde_json::from_str(&s).map_err(|e| e.to_string())?;
            println!("[config] loaded from {}", p.display());
            return Ok(cfg);
        }
    }

    println!(
        "[config] not found, tried: {:?}",
        tried
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    );
    Ok(Config::default())
}

fn apply_registry_restrictions_from_config(cfg: &Config) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\System";
    if let Ok((key, _)) = hkcu.create_subkey(path) {
        if let Some(val) = cfg.disable_taskmgr {
            let _ = key.set_value("DisableTaskMgr", &(if val { 1u32 } else { 0u32 }));
        }
        if let Some(val) = cfg.disable_lockworkstation {
            let _ = key.set_value("DisableLockWorkstation", &(if val { 1u32 } else { 0u32 }));
        }
        if let Some(val) = cfg.disable_change_password {
            let _ = key.set_value("DisableChangePassword", &(if val { 1u32 } else { 0u32 }));
        }
    }
}

// modify registry to enable/disable system restrictions
fn toggle_system_restrictions(disable: bool) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Policies\\System";
    if let Ok((key, _)) = hkcu.create_subkey(path) {
        let val: u32 = if disable { 1 } else { 0 };
        let _ = key.set_value("DisableTaskMgr", &val);
        let _ = key.set_value("DisableLockWorkstation", &val);
        let _ = key.set_value("DisableChangePassword", &val);
    }
}

// low-level keyboard hook callback: return 1 to swallow event
// The trick is to return 1 for BOTH key down and key up events for Win key
// This prevents the system from processing the key at all
unsafe extern "system" fn keyboard_proc_ll(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == 0 {
        let kbd = *(lparam as *const KBDLLHOOKSTRUCT);
        let vk = kbd.vkCode as i32;
        let flags = kbd.flags as i32;

        let is_alt = (flags & 0x20) != 0; // LLKHF_ALTDOWN
                                          // 使用 GetAsyncKeyState 检测 Ctrl 键状态 (最高位为 1 表示按下)
        let is_ctrl = (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;

        if (vk == VK_LWIN as i32 || vk == VK_RWIN as i32) && BLOCK_WIN_KEYS {
            return 1;
        }

        if vk == VK_TAB as i32 && is_alt && BLOCK_ALT_TAB {
            return 1;
        }

        if vk == VK_F4 as i32 && is_alt && BLOCK_ALT_F4 {
            return 1;
        }

        if vk == VK_ESCAPE as i32 && is_ctrl && BLOCK_CTRL_ESC {
            return 1;
        }
    }
    CallNextHookEx(HOOK_LL, code, wparam, lparam)
}

#[tauri::command]
fn request_exit(app: tauri::AppHandle, password: String) -> Result<(), String> {
    // 获取当前内存中的配置（或者重新读取文件）
    let cfg = read_config_from_cwd().map_err(|e| e.to_string())?;

    // 获取哈希值，如果没有则使用默认硬编码哈希
    let hash = cfg
        .admin_hash
        .unwrap_or_else(|| "$2y$12$yo8M7GzPhHAQhfw29IXC7OBEU5bQyMmY5BVhiun.SyYpIt8T0C3pS".into()); // 默认：admin888

    println!("[request_exit] 正在校验授权码...");

    // 使用 bcrypt 进行安全校验
    match bcrypt::verify(&password, &hash) {
        Ok(true) => {
            println!("[request_exit] 校验通过，设置状态为正常并退出");

            // 设置状态文件为正常
            if let Err(e) = set_state_normal() {
                println!("[request_exit] 设置状态文件为正常失败: {}", e);
            }

            unsafe {
                if HOOK_LL != 0 {
                    UnhookWindowsHookEx(HOOK_LL);
                    HOOK_LL = 0;
                }
            }
            toggle_system_restrictions(false);
            app.exit(0);
            Ok(())
        }
        _ => {
            println!("[request_exit] 授权码错误");
            Err("授权码验证失败".into())
        }
    }
}

/// 验证密码并重置状态文件为正常
#[tauri::command]
fn verify_and_reset_state(app: tauri::AppHandle, password: String) -> Result<bool, String> {
    let cfg = read_config_from_cwd().map_err(|e| e.to_string())?;

    // 获取哈希值
    let hash = cfg
        .admin_hash
        .unwrap_or_else(|| "$2y$12$yo8M7GzPhHAQhfw29IXC7OBEU5bQyMmY5BVhiun.SyYpIt8T0C3pS".into());

    println!("[verify_and_reset_state] 正在校验密码...");

    // 验证密码
    match bcrypt::verify(&password, &hash) {
        Ok(true) => {
            println!("[verify_and_reset_state] 密码验证通过，重置状态文件...");
            match set_state_normal() {
                Ok(()) => {
                    println!("[verify_and_reset_state] 状态文件重置成功");
                    toggle_system_restrictions(false);
                    app.exit(0);
                    Ok(true)
                }
                Err(e) => {
                    println!("[verify_and_reset_state] 状态文件重置失败: {}", e);
                    // 即使设置状态失败，密码已验证，也认为成功
                    Ok(true)
                }
            }
        }
        _ => {
            println!("[verify_and_reset_state] 密码验证失败");
            Ok(false)
        }
    }
}

#[tauri::command]
fn get_config() -> Result<Config, String> {
    read_config_from_cwd()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 1. 编译时静态嵌入。路径相对于当前 .rs 文件。
    const PRELOAD_SCRIPT: &str = include_str!("../../dist/main.js");

    let cfg = match read_config_from_cwd() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to read config: {}", e);
            Config::default()
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            request_exit,
            get_config,
            verify_and_reset_state
        ])
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .setup(move |app| {
            // 检查状态文件
            let mut state_check_failed = false;
            if cfg.enable_state_check.unwrap_or(false) {
                match check_state_file() {
                    Ok(true) => {
                        println!("[startup] 状态文件正常，继续启动");
                        // 立即设置为异常，等待正常退出时再设置为正常
                        if let Err(e) = set_state_abnormal() {
                            println!("[startup] 设置状态为异常失败: {}", e);
                        }
                    }
                    Ok(false) => {
                        println!("[startup] 状态文件异常，需要验证密码");
                        state_check_failed = true;
                    }
                    Err(e) => {
                        println!("[startup] 状态文件检查出错: {}", e);
                        state_check_failed = true;
                    }
                }
            }

            // 如果状态检查失败，显示锁屏窗口
            if state_check_failed {
                let lock_url = tauri::WebviewUrl::App("lock.html".into());
                let _lock_window = tauri::webview::WebviewWindowBuilder::new(app, "lock", lock_url)
                    .title("系统锁定")
                    .decorations(false)
                    .always_on_top(cfg.always_on_top.unwrap_or(false))
                    .fullscreen(cfg.fullscreen.unwrap_or(false))
                    .build()
                    .expect("failed to build lock window");

                return Ok(());
            }

            // 校验 cfg.exam_url 的合法性
            let validated_url = cfg
                .exam_url
                .as_ref()
                .and_then(|u| tauri::Url::parse(u).ok())
                .map(|_| "index.html")
                .unwrap_or("empty.html");

            let url = tauri::WebviewUrl::App(validated_url.into());
            let mut window_builder = tauri::webview::WebviewWindowBuilder::new(app, "main", url)
                .title("考试客户端")
                .decorations(false)
                .initialization_script(PRELOAD_SCRIPT);

            // 应用配置
            if let Some(full) = cfg.fullscreen {
                window_builder = window_builder.fullscreen(full);
            }
            if let Some(aot) = cfg.always_on_top {
                window_builder = window_builder.always_on_top(aot);
            }

            let _window = window_builder.build().expect("failed to build window");

            // 应用注册表限制
            apply_registry_restrictions_from_config(&cfg);

            // 键盘钩子逻辑
            let need_hook = cfg.block_win_keys.unwrap_or(false)
                || cfg.block_alt_tab.unwrap_or(false)
                || cfg.block_alt_f4.unwrap_or(false)
                || cfg.block_ctrl_esc.unwrap_or(false);

            if need_hook {
                unsafe {
                    BLOCK_WIN_KEYS = cfg.block_win_keys.unwrap_or(false);
                    BLOCK_ALT_TAB = cfg.block_alt_tab.unwrap_or(false);
                    BLOCK_ALT_F4 = cfg.block_alt_f4.unwrap_or(false);
                    BLOCK_CTRL_ESC = cfg.block_ctrl_esc.unwrap_or(false);

                    HOOK_LL = SetWindowsHookExW(
                        WH_KEYBOARD_LL,
                        Some(keyboard_proc_ll),
                        windows_sys::Win32::Foundation::HWND::default() as _,
                        0,
                    );
                }
            }

            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
