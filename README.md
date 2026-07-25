# ForgeOS

## Windows v0.2.0 预览版

ForgeOS 提供 Windows x64 当前用户安装包。v0.2.0 新增公开 GitHub 项目的规则/AI 分析、主流 AI 服务兼容和人工审核导入；安装包同时包含只读的 `forgeos-mcp.exe`，无需单独下载或手动启动。

安装完成后：

1. 打开“应用设置”。
2. 在“Codex 连接”中点击“连接 Codex”。
3. 重启 Codex，让 `forgeos` MCP 生效。

ForgeOS MCP 使用 STDIO 按需启动，只读取本机 ForgeOS 数据库；它不会执行代码、修改 Git 或写入 ForgeOS。安装包未签名，Windows SmartScreen 可能显示“未知发布者”。目标电脑缺少 WebView2 时，安装器会联网下载 Microsoft Bootstrapper。

本地构建预览安装包：

```powershell
npm run release:windows
```

产物位于 `dist-release/`。详细流程见 `docs/release-v0.2.0.md`。

> 当前阶段：D9 GitHub Project Intelligence。ForgeOS 可以只读分析公开 GitHub 仓库，并在用户审核完整项目草稿后创建相互隔离的个人研发项目。

ForgeOS（Forge Operating System）是面向独立开发者的个人 AI 辅助软件研发操作系统。

它用于在多个软件项目之间保存可信的研发上下文、当前状态、技术决策、问题排查过程与成果证据，并生成可供 Codex 使用的 Context Pack。

ForgeOS 不是 Todo 工具、Jira 替代品、普通笔记软件或 AI 聊天工具。

## 项目目标

- 保存相互隔离的项目研发上下文
- 维护可信的 Current Snapshot
- 记录可交接的 Development Session
- 保存技术决策及其理由
- 沉淀问题排查和解决过程
- 建立可追溯的成果证据链
- 为 Codex 生成单项目上下文包

ForgeOS 的长期方向是成为个人开发者的 AI Engineering Chief of Staff。当前优先建立可信的研发记忆系统。

## 当前阶段

当前处于 **D9：GitHub Project Intelligence + Human Import**。

本阶段只负责：

- 初始化单用户本地 Workspace
- 创建、查看和更新软件项目身份
- 管理 Project 状态与研发生命周期阶段
- 通过 Dashboard、Projects 和 Project Overview 查看项目
- 使用乐观并发版本保护正式 Project 事实
- 软归档 Project，并保留只读项目身份
- 人工创建和确认 Project 的 Current Snapshot
- 以只追加版本保留 Snapshot 历史，并阻止陈旧页面静默覆盖
- 在 Project Overview 优先展示当前目标、状态、成果、阻塞、最多三个下一步和确认时间
- 开始并维护每个 Project 唯一的 Open Development Session
- 使用 `open → completed | abandoned` 单向状态机保留研发过程
- 记录 Goal、Markdown Notes、Outcome、Validation 和 Handoff
- 使用 `expectedVersion` 保护 Session 并发写入，并将终态 Session 保持只读
- 在 Project Overview 显示 Current Development Session，并提供 Session History
- 人工创建 Document、Screenshot、Link、Test Result、Note 与 Artifact Evidence
- 保存本地文件路径、外部 URL 或人工记录来源，不复制或自动采集附件
- 通过 `unverified / verified / invalid` 状态明确证据可信度
- 使用统一 EvidenceLink 将证据关联到 Project、Snapshot 或 Development Session
- 内容修改后自动回到 unverified，避免旧验证状态继续背书已变化内容
- 管理自定义 OpenAI-compatible Base URL、Model 与 Provider 启停状态
- 将 API Key 保存到 Windows Credential Manager，而不是 SQLite
- 通过用户显式操作执行最小 Chat Completions 调用，并只返回临时结果
- 从 Project、Current Snapshot、最近 Development Session 与 Evidence 确定性生成 Markdown Context Pack
- 保存 ContextExport 的来源版本、SHA-256 内容哈希、模板、发送状态与乐观并发版本
- 支持 Planning、Coding、Debugging、Review 与 Handoff 五种模板
- 由用户显式选择 Enabled Provider，将 Context Pack 与单次问题发送给 AI
- AI Prompt 与 Response 只保留在当前页面状态，不进入正式事实层或调用历史
- 由用户将当前 AI Response 显式保存为 Pending Candidate，或选择仅查看
- 管理 Snapshot Update、Session Summary、Evidence Suggestion、Risk Finding 与 General Note 候选类型
- 通过 `pending → accepted | rejected | archived` 单向状态机记录人工审核结果
- Accept 只记录用户认可，不自动修改 Project、Snapshot、Session 或 Evidence
- 注册并观察属于 Project 的本地 Git Repository
- 通过固定只读 Git CLI 命令读取分支、HEAD、dirty 状态、diff summary 和最近提交
- 每次刷新追加不可变 WorkingCopy 观察记录，不修改本地 Git 仓库
- 由用户明确把 Commit 或 WorkingCopy 状态转换为 Git Evidence
- 使用 Repository/Evidence 复合 Project 外键阻止跨项目 Git Evidence
- 通过官方 Codex App Server STDIO 接口按 Repository 只读导入结构化任务活动
- 通过只读 Git 检查分支、HEAD、Dirty、Upstream 与 Ahead/Behind，不执行 fetch 或写操作
- 自动生成待审核 Project Pulse，并清楚区分系统观察与人工确认事实
- 在项目首页展示同步阶段、扫描计数、部分失败原因与最新 Git 工作区状态
- 由用户审核后在单个事务中创建新版 Snapshot，并开始或更新 Development Session
- 可选按 Project 启用 AI 结构化分析；失败时自动回退确定性规则摘要
- MCP 和新 Context Pack 可读取最近保存的 Pulse，但不会触发同步或修改 ForgeOS

本阶段不实现 Decision、Problem、AI 自动调用、自动总结、AI 事实生成、Agent 执行、MCP 写入、
Git 写操作、GitHub、Codex 自动协同、Search、Timeline、对话历史或任务系统。

## 技术栈

- Frontend：React + TypeScript + Vite
- Desktop：Tauri 2
- Backend：Rust
- Database：SQLite
- Platform：Windows
- Architecture：模块化单体

## 项目结构

```text
ForgeOS
├─ README.md
├─ docs
│  ├─ product-design.md
│  ├─ architecture.md
│  ├─ development-guide.md
│  ├─ database-design.md
│  ├─ development-log.md
│  └─ adr
│     ├─ ADR-001-tauri-rust-sqlite.md
│     ├─ ADR-002-project-identity-model.md
│     ├─ ADR-003-current-snapshot-versioning.md
│     ├─ ADR-004-development-session-lifecycle.md
│     ├─ ADR-005-evidence-model.md
│     ├─ ADR-006-ai-provider-foundation.md
│     ├─ ADR-007-context-pack-and-ai-invocation.md
│     ├─ ADR-008-ai-candidate-governance.md
│     ├─ ADR-009-git-readonly-integration.md
│     └─ ADR-010-mcp-readonly-architecture.md
├─ forgeos-mcp
│  ├─ src
│  │  ├─ main.rs
│  │  ├─ server.rs
│  │  ├─ protocol.rs
│  │  └─ tools
│  └─ tests
├─ frontend
├─ src-tauri
├─ database
│  └─ migrations
├─ assets
└─ .gitignore
```

## 已验证的本机环境

- Node.js 24.14.1
- npm 11.11.0
- rustc 1.96.1
- Cargo 1.96.1
- WebView2 Runtime 150.0.4078.48
- Visual Studio Build Tools 2022 17.14.35
- Windows SDK 10.0.26100.0
- 项目本地 Tauri CLI 2.11.4
- rustfmt 1.9.0-stable

工程使用项目本地 `@tauri-apps/cli`，不要求安装全局 Tauri CLI。D0.5 已验证
标准 `npm run tauri:dev`，Rust/Tauri 也可以通过 `cargo` 直接运行。

## 运行

### 首次安装项目依赖

```powershell
npm install
npm --prefix frontend install
```

### 启动前端开发服务器

```powershell
npm --prefix frontend run dev
```

### 构建前端

```powershell
npm --prefix frontend run build
```

### 检查与测试 Rust

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

### 启动 Tauri 桌面应用

```powershell
npm run tauri:dev
```

`npm run tauri:dev` 依赖首次安装后生成的项目本地 Tauri CLI，不要求安装全局 CLI。

### 未安装 Tauri CLI 时的已验证启动方式

先在一个终端启动前端：

```powershell
npm --prefix frontend run dev
```

再在另一个终端启动桌面应用：

```powershell
$env:FORGEOS_DATA_DIR = Join-Path $PWD ".data"
cargo run --manifest-path src-tauri/Cargo.toml
```

## 数据库

D2 使用十二个 SQLite 表：

- `workspaces`
- `projects`
- `project_snapshots`
- `development_sessions`
- `evidences`
- `evidence_links`
- `ai_providers`
- `context_exports`
- `ai_candidates`
- `repositories`
- `working_copies`
- `repository_evidence_links`

数据库迁移位于 `database/migrations`。`0002_project_identity_kernel.sql` 在不修改
D0 Migration 的前提下完善 Project 身份、产品上下文、技术信息、状态、生命周期、
优先级与版本字段；`0003_current_snapshot.sql` 增加按 Project 隔离、只追加版本的
Current Snapshot；`0004_development_session.sql` 增加 Session 状态机、Project 归属、
单 Open Session 部分唯一索引和历史查询索引。当前不创建 Decision、Problem、
Timeline、AI 生成内容或调用历史表。`0005_evidence_kernel.sql` 增加 Evidence、统一关联表、
验证状态约束、查询索引和多态关联目标校验 trigger；
`0006_ai_provider_foundation.sql` 只保存非敏感 Provider 配置。API Key 不进入数据库；
`0007_context_pack.sql` 保存确定性导出内容、来源版本、SHA-256、模板、发送状态与乐观版本；
`0008_ai_candidate.sql` 保存 AI 候选建议及人工审核状态，并以复合外键保护来源 Project 一致性；
`0009_git_evidence.sql` 保存 Repository、WorkingCopy 观察与 Repository-Evidence 来源关系，并扩展 Git Evidence 类型。

## 文档

- `docs/product-design.md`：冻结后的产品边界
- `docs/architecture.md`：总体架构和模块职责
- `docs/database-design.md`：D2 数据库设计
- `docs/development-guide.md`：开发与 Git 规范
- `docs/adr`：架构决策记录
- `docs/development-log.md`：项目开发日志

## 项目状态

ForgeOS 已可在本地管理 Project 身份、生命周期和人工确认的 Current Snapshot，
保留可追溯的 Snapshot 历史，并以 Development Session 记录一次研发活动的目标、
过程、结果、验证和接手信息。Evidence Kernel 已能保存人工证据、验证状态与
Project/Snapshot/Session 关联。Context Pack Kernel 已能按固定章节和稳定排序生成可验证的
Markdown，并通过用户选择的 Enabled Provider 执行一次性分析。用户可把临时 Response 明确保存为
Pending Candidate，并人工 Accept、Reject 或 Archive；这些操作只记录治理状态，不生成或修改正式事实。
Git Evidence Integration 已能以只读 Git CLI 观察本地 Repository，并由用户明确生成 Commit/Status Evidence。
ForgeOS 不提供任何 Git 写命令；D3 MCP 仅提供研发语义上下文读取，GitHub、Agent、MCP 写入与自动 AI 任务仍未实现。

## ForgeOS MCP Read-only Server

D3 提供独立的 `forgeos-mcp` Rust 可执行程序。它通过 STDIO MCP 向本地 AI 客户端公开五个只读工具，不监听 HTTP/TCP 端口，也不修改 Codex 配置。

构建与测试：

```powershell
npm run mcp:build
npm run mcp:test
```

默认读取 `%APPDATA%\com.forgeos.desktop\database\forgeos.db`。也可以显式指定数据库：

```powershell
$env:FORGEOS_DATABASE_PATH = "D:\path\to\forgeos.db"
npm run mcp:run
```

或直接执行：

```powershell
Set-Location forgeos-mcp
cargo run -- --database "D:\path\to\forgeos.db"
```

STDOUT 专用于 MCP JSON-RPC；服务器只读打开既有数据库，不执行 Migration，不创建数据库，也不持久化 `build_context_pack` 的结果。
