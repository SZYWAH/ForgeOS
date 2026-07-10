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

当前处于 **D0：工程基础初始化**。

本阶段只负责：

- 建立 React、TypeScript、Vite 前端基础
- 建立 Tauri 2 桌面壳
- 建立 Rust 模块边界
- 验证 SQLite 连接与迁移
- 建立工程文档和开发记录
- 验证 Windows 构建链路

本阶段不实现业务页面、AI、Git/GitHub、Codex MCP 或任务系统。

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
│     └─ ADR-001-tauri-rust-sqlite.md
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

D0 只创建：

- `workspaces`
- `projects`

数据库迁移位于 `database/migrations`。当前不创建 Snapshot、Session、Decision、Problem、Evidence 等业务表。

## 文档

- `docs/product-design.md`：冻结后的产品边界
- `docs/architecture.md`：总体架构和模块职责
- `docs/database-design.md`：D0 数据库设计
- `docs/development-guide.md`：开发与 Git 规范
- `docs/adr`：架构决策记录
- `docs/development-log.md`：项目开发日志

## 项目状态

ForgeOS 尚处于工程基础阶段，不应用于正式项目数据管理。
