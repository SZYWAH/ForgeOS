# ForgeOS v0.1.0 Preview

ForgeOS 是面向个人开发者的本地优先研发记忆助手。本预览版可管理多个项目的身份、当前状态、研发历史、成果资料和 AI 项目上下文，并让 Codex 通过只读 MCP 恢复项目上下文。

## 安装

- 支持 Windows x64。
- 使用当前用户安装，不需要管理员权限。
- 安装包已包含 `forgeos-mcp.exe`，无需单独下载。
- 安装后打开“应用设置 → Codex 连接”，点击“连接 Codex”，然后重启 Codex。
- 目标电脑缺少 WebView2 时，安装器需要联网下载 Microsoft WebView2 Bootstrapper。

## 安全边界

- ForgeOS MCP 只读，不执行代码，不修改 Git，也不写入 ForgeOS。
- MCP 通过 STDIO 由 Codex 按需启动，不监听网络端口。
- ForgeOS 不把 API Key、Credential、绝对本地路径或原始 Codex 对话返回给 MCP。
- 普通卸载只移除完全匹配且由 ForgeOS 管理的 Codex MCP 配置。
- 卸载默认保留 ForgeOS 数据库和应用偏好，便于重装恢复。

## 预览版限制

- 本安装包未进行代码签名，Windows SmartScreen 可能显示“未知发布者”。
- 当前不提供自动更新器。
- 当前不包含 Codex 执行、Agent、MCP 写入或 Git 写操作。

请使用同一 Release 中的 `SHA256SUMS.txt` 校验安装包完整性。
