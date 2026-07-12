# ADR-010：ForgeOS MCP 第一阶段采用独立 STDIO 只读架构

- 状态：已接受
- 日期：2026-07-12
- 决策范围：ForgeOS MCP Server、外部 AI 上下文读取与安全边界
- 前置决策：ADR-007（Context Pack 与 AI 隔离）、ADR-009（Git 只读集成）

## 背景

ForgeOS 已保存 Project Identity、Current Snapshot、Development Session、Evidence、Git Evidence 和 Context Pack。Codex 等外部 AI 工具需要读取这些研发语义事实，但不应因此获得修改 ForgeOS 正式事实、控制 Git 或读取 AI Provider 凭据的能力。

MCP 可以把 ForgeOS 作为本地上下文服务接入 AI 客户端。如果第一版同时提供写工具、HTTP 服务或自动任务，ForgeOS 的事实来源边界会与外部模型的推断和执行能力混合，难以证明哪些状态经过用户确认。

## 候选方案

### 方案一：在 Tauri 进程内提供 HTTP MCP Server

复用桌面进程生命周期，但需要端口监听、会话管理、网络访问控制和更大的攻击面；桌面应用未运行时也无法独立提供上下文。

### 方案二：独立 STDIO MCP Server，同时提供读写 Tools

本地集成简单，但写工具会让外部 AI 绕过 ForgeOS UI、Candidate 治理和人工确认，破坏正式事实层可信度。

### 方案三：独立 STDIO MCP Server，仅调用专用 Query Kernel

MCP 进程可由客户端按需启动，不开放网络端口；专用 Application Query Kernel 和只读 SQLite 连接形成双重写保护。

## 决策

选择方案三。

`forgeos-mcp` 是独立 Rust executable，使用 MCP STDIO Transport。它复用 ForgeOS Core 的 Repository Ports、SQLite Repository 和 Context Renderer，但只依赖新建的 `McpQueryKernel`。MCP 层不持有 ProjectKernel、ContextPackKernel 或其他包含 Mutation 的 Application Service。

第一版只公开五个 Tools：

- `get_project_identity`
- `get_current_snapshot`
- `get_recent_sessions`
- `list_evidence`
- `build_context_pack`

所有输入都包含 `project_id`。输出采用专用 DTO 白名单，不返回 Evidence source URI、Repository path、AI Provider 配置、API Key、Credential 或 Token。`build_context_pack` 只在内存中复用现有 Renderer，不保存 ContextExport。

数据库以 SQLite read-only 模式打开，不创建文件、不执行 Migration。STDOUT 只承载 MCP JSON-RPC，服务不监听 HTTP 或 TCP。

## 优点

- 外部 AI 可以获得 ForgeOS 独有的研发语义上下文，而不重复 GitHub MCP 的仓库托管能力。
- Query-only Application API 与 SQLite read-only 连接提供两层写保护。
- STDIO 无监听端口，进程生命周期由本地 MCP Client 管理。
- 专用输出 DTO 默认最小披露，降低私有路径和凭据泄露风险。
- Context Pack 复用既有确定性 Renderer，避免 MCP 复制或漂移业务规则。
- MCP Server 可独立构建、测试和启动，不要求 Tauri 桌面窗口运行。

## 缺点

- 当前只能按内部 Project ID 查询，客户端需要从 ForgeOS UI 或未来受控发现机制获得 ID。
- 桌面应用升级 Migration 后，旧 MCP binary 可能因 Schema 校验失败而需要同步升级。
- STDIO 适合本机客户端，不提供远程访问。
- 只读模式下不会自动修复或迁移旧数据库；用户必须先用兼容版本的 ForgeOS Desktop 完成 Migration。
- 通过路径依赖复用当前 Core 会连带编译 Tauri 依赖，独立构建体积和时间高于拆分纯 Core crate。

## 长期影响

- 后续如需 MCP 写入，必须新建 ADR、权限模型、用户确认流程和审计机制，不能向当前 Query Kernel 添加 Mutation。
- Agent、Codex Task、MCP Tasks 和后台调用不得通过扩展现有只读工具名称悄然进入。
- 若未来拆分 `forgeos-core` crate，MCP 的依赖方向保持 `MCP → Application Query → Ports → Infrastructure`，不得把 SQL 移入 MCP。
- GitHub MCP 与 ForgeOS MCP 保持互补：前者提供托管平台和代码协作事实，后者提供用户确认的项目语义与研发记忆。
- 私有路径或更详细 Evidence 来源若未来需要返回，必须采用显式授权字段和默认关闭策略。
