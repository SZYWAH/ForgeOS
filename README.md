# ForgeOS

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

当前处于 **D1.1：Project Identity Kernel**。

本阶段只负责：

- 初始化单用户本地 Workspace
- 创建、查看和更新软件项目身份
- 管理 Project 状态与研发生命周期阶段
- 通过 Dashboard、Projects 和 Project Overview 查看项目
- 使用乐观并发版本保护正式 Project 事实
- 软归档 Project，并保留只读项目身份

本阶段不实现 Current Snapshot、Development Session、Decision、Problem、Evidence、AI、
Git/GitHub、Codex Context Pack、Search、Timeline 或任务系统。

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
│     └─ ADR-002-project-identity-model.md
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

D1.1 使用两个业务表：

- `workspaces`
- `projects`

数据库迁移位于 `database/migrations`。`0002_project_identity_kernel.sql` 在不修改
D0 Migration 的前提下完善 Project 身份、产品上下文、技术信息、状态、生命周期、
优先级与版本字段。当前不创建 Snapshot、Session、Decision、Problem 或 Evidence 表。

## 文档

- `docs/product-design.md`：冻结后的产品边界
- `docs/architecture.md`：总体架构和模块职责
- `docs/database-design.md`：D1.1 数据库设计
- `docs/development-guide.md`：开发与 Git 规范
- `docs/adr`：架构决策记录
- `docs/development-log.md`：项目开发日志

## 项目状态

ForgeOS 已可在本地管理 Workspace、Project 身份和生命周期，并通过乐观并发保护正式事实。
Current Snapshot、Development Session 及其他后续领域仍需按阶段实现、冻结和验收。
