# exam_client

基于 Tauri 2 和 Rust 构建的网页考试监控客户端，专为 Windows 系统设计。可以屏蔽快捷键、禁用系统功能、全屏考试页面。包含自动注入的 OSD 浮窗，提供导航和管理员退出功能，支持密码保护和异常退出检测。

## 开发
```bash
git clone <repository-url>
cd exam_client
pnpm install
pnpm run tauri dev
```

构建: `pnpm run tauri build`

***！！注意：建议构建生产版本前修改`lib.rs`中的如下字段以确保state文件安全加密**
```rust
/// 固定密钥用于加密状态文件               |----修改这里，注意长度必须一致----|
const STATE_ENCRYPTION_KEY: &[u8; 32] = b"exam_state_encryption_key_v_2026";
```

## 配置
```json
{
  "exam_url": "https://example.com/exam",
  "fullscreen": false,
  "always_on_top": false,
  "disable_taskmgr": false,
  "disable_lockworkstation": true,
  "disable_change_password": true,
  "block_win_keys": true,
  "block_alt_tab": false,
  "block_alt_f4": true,
  "block_ctrl_esc": true,
  "enable_state_check": true,
  "admin_hash": "$2y$12$..."
}
```

exam_url 是要访问的考试地址。fullscreen、always_on_top 控制窗口模式。disable_* 禁用相应系统功能。block_* 屏蔽相应快捷键。enable_state_check 启用异常退出检测。admin_hash 是用 bcrypt (Cost Factor 12) 加密的管理员密码。

## 使用

应用启动时会自动注入一个 OSD 浮窗到考试页面，位置在右上角。点击浮窗中的导航按钮可以返回主页、后退、前进或刷新。点击"管理"可以打开退出菜单，输入管理员密码验证后可以关闭应用。

如果 enable_state_check 为 true，上次程序未正常退出（如关机、注销）时会显示锁定屏幕，需要输入管理员密码才能继续。
