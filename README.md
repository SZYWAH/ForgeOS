# ForgeOS

[GitHub 主页](https://github.com/SZYWAH) · [Windows 预览安装包](https://github.com/SZYWAH/ForgeOS/releases/tag/v0.2.0) · [v0.2.0 说明](docs/release-v0.2.0.md) · [开发指南](docs/development-guide.md)

ForgeOS 是面向个人开发者的本地优先研发记忆助手。它保存多个项目的身份、当前状态、研发历史与成果资料，并通过 Context Pack 和只读 MCP，帮助自己与 Codex 恢复项目上下文。

## 当前版本

**v0.2.0 Preview · Windows x64**。当前工程阶段为 **D9：GitHub Project Intelligence + Human Import**，已发布当前用户安装包，安装包内置 `forgeos-mcp.exe`。

本版支持公开 GitHub 仓库的规则分析、可选 AI 增强，以及经过人工审核的项目导入。详细功能、已知限制和发布验收记录见 [v0.2.0 发布说明](docs/release-v0.2.0.md)。

## 能做什么

| 方向 | 当前能力 |
| --- | --- |
| 多项目上下文 | 管理项目身份、已确认状态、目标、阻塞与下一步 |
| 研发历史 | 汇总 Development Session、只读 Git 观察与 Codex 活动 |
| 成果与资料 | 保存 Evidence、支持性资料与经过人工确认的成果 |
| AI 项目上下文 | 生成按项目隔离、可追溯的 Context Pack |
| Codex 连接 | 通过 STDIO MCP 读取本机保存的项目记忆 |
| GitHub 项目分析 | 只读分析公开仓库，将来源约束的报告转换为可编辑草稿，审核后导入项目 |

AI 分析、外部观察与人工确认的项目事实分开保存；AI 结果不会静默改写正式研发记忆。ForgeOS 不执行 Git 写操作，MCP 不执行代码、不修改 Git，也不写入 ForgeOS。

## 安装与连接 Codex

1. 从 [v0.2.0 Release](https://github.com/SZYWAH/ForgeOS/releases/tag/v0.2.0) 下载 Windows x64 安装包，并按同页 `SHA256SUMS.txt` 校验。
2. 安装并启动 ForgeOS。
3. 打开“应用设置 → Codex 连接”，点击“连接 Codex”。
4. 重启 Codex，使 `forgeos` MCP 生效。

当前安装包未签名。目标电脑缺少 WebView2 时，安装器会联网下载 Microsoft Bootstrapper。普通卸载默认保留数据库和应用偏好；其他安装与升级说明见 [发布文档](docs/release-v0.2.0.md)。

## 技术栈

- 桌面端：Tauri 2
- 前端：React、TypeScript、Vite
- 后端与 MCP：Rust
- 数据库：SQLite
- 当前交付平台：Windows x64

## 本地开发

在 Windows 开发环境中准备 Node.js、Rust / Cargo、WebView2 和相应构建工具，详细说明见 [开发指南](docs/development-guide.md)。项目使用本地 Tauri CLI。

安装依赖并启动：

```powershell
npm install
npm --prefix frontend install
npm run tauri:dev
```

仅启动前端：

```powershell
npm run frontend:dev
```

检查和构建：

```powershell
npm run typecheck
npm run test -- --run
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
npm run mcp:test
```

构建 Windows 预览安装包：

```powershell
npm run release:windows
```

产物位于 `dist-release/`。完整发布流程见 [docs/release-v0.2.0.md](docs/release-v0.2.0.md)。

## 仓库结构

| 目录 | 用途 |
| --- | --- |
| [frontend/](frontend/) | React 界面与前端测试 |
| [src-tauri/](src-tauri/) | Rust / Tauri 桌面后端 |
| [forgeos-mcp/](forgeos-mcp/) | 独立的只读 MCP 服务 |
| [database/migrations/](database/migrations/) | SQLite 版本迁移 |
| [scripts/](scripts/) | 构建、安装和发布验证 |
| [docs/](docs/) | 架构、设计、开发记录与 ADR |

## MCP 独立开发

```powershell
npm run mcp:build
npm run mcp:test
```

默认读取 `%APPDATA%\com.forgeos.desktop\database\forgeos.db`，也可以指定既有数据库：

```powershell
$env:FORGEOS_DATABASE_PATH = "D:\path\to\forgeos.db"
npm run mcp:run
```

MCP 通过 STDIO 按需启动，只读打开数据库，不运行 Migration，也不创建数据库。单独构建用于开发；普通安装用户直接使用安装包内置的 MCP。

## 文档导航

- [产品设计与演进](docs/product-design.md)：包含早期基线及后续阶段调整
- [系统架构](docs/architecture.md)
- [数据库设计](docs/database-design.md)
- [开发指南](docs/development-guide.md)
- [架构决策记录](docs/adr/)
- [开发日志](docs/development-log.md)
- [GitHub 分析契约](docs/github-analysis-contract.md)
- [v0.2.0 发布与验收](docs/release-v0.2.0.md)
- [v0.1.0 历史发布说明](RELEASE_NOTES.md)

阅读带有 D0、D2、D3 等标记的文档时，请结合其阶段和后续变更；当前入口以本 README 与 v0.2.0 发布说明为准。

