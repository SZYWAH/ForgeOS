# ForgeOS v0.1.0 Windows 预发布流程

## 交付物

- `ForgeOS-0.1.0-windows-x64-setup.exe`
- `SHA256SUMS.txt`

安装器面向 Windows x64，使用 NSIS、当前用户安装和 LZMA 压缩。WebView2 采用在线 Bootstrapper，因此安装包保持轻量；缺少 WebView2 的目标电脑在安装时需要联网。

## 构建

```powershell
npm run release:windows
```

构建脚本会：

1. 校验 Node、Tauri、桌面 Rust 和 MCP Rust 版本均为 `0.1.0`。
2. 构建 Release MCP，并复制为 Tauri 平台 Sidecar 名称。
3. 验证 Sidecar 版本与 SHA-256。
4. 构建前端、桌面程序和 NSIS 安装器。
5. 验证 Tauri 暂存的 Sidecar 与新构建二进制 Hash 一致。
6. 生成规范名称的安装包和 SHA-256 文件。
7. 拒绝超过 30 MiB 的安装包。

生成的二进制、安装包和测试目录不会进入 Git。

## 安装器验收

在没有现有 ForgeOS 安装注册时执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-windows-installer.ps1
```

脚本使用隔离的安装目录和 `CODEX_HOME`，验证：

- 安装后的 MCP Hash、版本和数据库只读诊断。
- Codex STDIO MCP 注册。
- 模拟旧版本升级时保留完全匹配的 MCP 连接。
- 普通卸载时删除完全匹配的 MCP 连接。
- 测试结束后卸载并清理隔离目录。

脚本检测到已有 ForgeOS 安装时会停止，避免覆盖用户安装。

## Codex 一键连接

应用通过官方 Codex CLI 管理用户级 `forgeos` MCP：

```powershell
codex mcp get forgeos --json
codex mcp add forgeos -- "<安装目录>\forgeos-mcp.exe"
codex mcp remove forgeos
```

配置前备份 `config.toml`，最多保留最近五份。添加后会重新读取验证；失败时恢复备份。同名配置指向其他程序时必须由用户确认替换。普通卸载只移除名称、命令、空参数和 STDIO Transport 均完全匹配的配置。

开发版默认禁止把 `target` 路径写入正式 Codex 配置。只有显式设置 `FORGEOS_MCP_PATH` 并通过诊断时才允许测试。

## 已知限制

- v0.1.0 未签名，SmartScreen 可能提示未知发布者。
- 不提供自动更新器。
- 不发布独立 MCP 二进制。
- 未内置完整 WebView2 Runtime。
- MCP 保持只读，不执行 Codex、Agent、Shell 或 Git 写操作。
- 卸载默认保留 ForgeOS SQLite 数据和应用偏好。
