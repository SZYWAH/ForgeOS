# ADR-001：采用 Tauri 2、Rust 与 SQLite

- 状态：已接受
- 日期：2026-07-10
- 决策范围：ForgeOS 桌面运行时、后端和本地数据存储
- 实施门槛：D0 必须通过 Windows 构建验证

## 背景

ForgeOS 是面向单个独立开发者的本地优先研发记忆系统。

应用需要：

- 在 Windows 桌面运行
- 访问本地文件和未来的 Git 仓库
- 保存结构化研发数据
- 在无网络和无 AI 时完整工作
- 长期保持清晰的业务边界
- 控制安装体积和运行资源
- 为未来的 MCP、Git 和 AI 适配器保留扩展能力

ForgeOS 当前不需要多人并发、独立服务器、远程数据库或微服务。

## 候选方案

### 方案一：Tauri 2 + Rust + SQLite

React 负责界面，Tauri 提供桌面壳，Rust 负责应用和数据访问，SQLite 保存本地结构化数据。

### 方案二：Electron + Node.js + SQLite

React 运行在 Electron 中，由 Node.js 负责系统能力和数据库访问。

### 方案三：Web 应用 + 独立服务端数据库

前端通过 HTTP API 访问独立后端和数据库。

### 方案四：Windows 原生 UI 技术

使用 WinUI、WPF 或其他 Windows 专属技术实现桌面应用。

## 决策

ForgeOS 选择：

- Tauri 2 作为桌面运行时
- Rust 作为后端和领域能力承载语言
- SQLite 作为本地结构化数据库
- React + TypeScript + Vite 作为前端

整体采用模块化单体，不引入独立服务器。

## 优点

- 符合本地优先和离线可用目标
- Rust 适合实现清晰的领域边界和系统能力
- Tauri 能直接承载 React 前端
- 相比 Electron，运行时和安装体积通常更小
- SQLite 无需独立数据库服务
- SQLite 适合单用户、本地、短事务场景
- 后续可在 Rust 中扩展 Git、文件、MCP 和后台任务能力
- 数据可以通过 SQLite 与文件导出长期保存

## 缺点

- Windows 构建依赖 WebView2 和 MSVC Build Tools
- Rust 和 Tauri 的开发门槛高于纯 Web 应用
- Tauri 插件和生态规模小于 Electron
- React 与 Rust 之间需要维护稳定的 IPC 合同
- SQLite 不适合未来的多人高并发写入
- 若错误混合领域代码与 Tauri/SQLx，会增加后续拆分成本

## 长期影响

- React 不直接访问 SQLite，所有正式写入通过 Rust。
- Rust 必须保持 `ipc`、`application`、`domain`、`ports` 和 `infrastructure` 的依赖边界。
- SQLite 是单用户本地阶段的事实源。
- 若未来出现真实的多人或多设备并发需求，应通过新 ADR 评估 PostgreSQL 或同步架构。
- 若未来实现 ForgeOS MCP Server，应复用 Rust 领域与应用核心。
- Windows 构建链路是 D0 阶段门槛。
- 如果 Tauri 在目标环境中无法稳定构建或运行，不得静默切换技术栈；必须新增 ADR 评估 Electron 等替代方案。
