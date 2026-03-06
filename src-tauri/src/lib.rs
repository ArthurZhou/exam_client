use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_F4, VK_LWIN, VK_RWIN, VK_TAB};
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
            admin_hash: Some("$2y$12$yo8M7GzPhHAQhfw29IXC7OBEU5bQyMmY5BVhiun.SyYpIt8T0C3pS".into()),
        }
    }
}

fn read_config_from_cwd() -> Result<Config, String> {
    use std::env;
    use std::fs;
    // search locations: current_dir, executable dir, project root (two levels up), src-tauri
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
        // HC_ACTION
        let kbd = *(lparam as *const KBDLLHOOKSTRUCT);
        let vk = kbd.vkCode as i32;
        let flags = kbd.flags as i32;
        let is_alt = (flags & 0x20) != 0; // LLKHF_ALTDOWN
        let is_ctrl = (flags & 0x0008) != 0; // LLKHF_CTRLDOWN
        let is_injected = (flags & 0x10) != 0; // LLKHF_INJECTED
        let is_up = (flags & 0x80) != 0; // LLKHF_UP

        // Block Win keys - CRITICAL: Block BOTH key down and key up events!
        // This is the key to preventing Windows from opening Start menu
        if (vk == VK_LWIN as i32 || vk == VK_RWIN as i32) && BLOCK_WIN_KEYS {
            return 1;
        }

        // Only process other keys on key down, not key up
        if !is_up && !is_injected {
            // Block Alt+Tab
            if vk == VK_TAB as i32 && is_alt && BLOCK_ALT_TAB {
                return 1;
            }

            // Block Alt+F4
            if vk == VK_F4 as i32 && is_alt && BLOCK_ALT_F4 {
                return 1;
            }

            // Block Ctrl+Esc
            if vk == VK_ESCAPE as i32 && is_ctrl && BLOCK_CTRL_ESC {
                return 1;
            }
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
        .unwrap_or_else(|| "$2y$12$yo8M7GzPhHAQhfw29IXC7OBEU5bQyMmY5BVhiun.SyYpIt8T0C3pS".into());

    println!("[request_exit] 正在校验授权码...");

    // 使用 bcrypt 进行安全校验
    match bcrypt::verify(&password, &hash) {
        Ok(true) => {
            println!("[request_exit] 校验通过，正在退出");
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
        .invoke_handler(tauri::generate_handler![request_exit, get_config])
        .device_event_filter(tauri::DeviceEventFilter::Always)
        .setup(move |app| {
            // 校验 cfg.exam_url 的合法性
            // 如果合法则记录该 URL 供后续使用，初始页面统一加载 index.html
            let validated_url = cfg
                .exam_url
                .as_ref()
                .and_then(|u| tauri::Url::parse(u).ok()) // 尝试解析 URL
                .map(|_| "index.html") // 如果配置合法，初始进入引导页
                .unwrap_or("empty.html"); // 如果不合法或不存在，进入错误/空白页

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
