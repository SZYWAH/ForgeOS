# ADR-013：Codex Execution 与 Task Proposal 分离

- 状态：已接受
- 日期：2026-07-13
- 决策范围：Codex 执行请求、生命周期记录、结果保存与未来 Runner 接入

## 背景

D4 的 CodexTaskProposal 冻结任务目标、上下文、允许范围、禁止范围、验收标准和验证命令，并通过人工批准表达未来执行授权。批准不是执行结果。如果直接在 Proposal 上追加运行状态和输出，任务契约会与一次具体尝试混在一起，失败重试也会覆盖历史。

D5 需要建立执行基础层，但当前禁止调用 `codex exec`、运行验证命令、修改文件、执行 Git 操作或引入 Agent。

## 候选方案

1. 在 Proposal 上增加运行字段：结构较少，但一个 Proposal 无法安全保存多次执行尝试，审批语义也会被运行结果污染。
2. 批准 Proposal 时自动启动 Codex：步骤较短，但把人工授权与外部副作用合并，违反显式执行和可审计原则。
3. 建立独立 CodexExecutionRecord 与 CodexRunner Port：Proposal 保持任务契约，Execution 表示一次可追踪尝试，Runner 作为未来执行适配器边界。

## 决策

选择方案三。

- 只有同一 Project 内状态为 `approved` 的 Proposal 可以创建 Execution。
- Execution 生命周期固定为 `pending → running → success | failed | cancelled`，并允许 `pending → cancelled`。
- 每次执行尝试创建独立记录；失败和取消不会被后续重试覆盖。
- 状态变化使用乐观版本控制，并在同一 SQLite 事务中追加状态历史。
- D5 使用 `RecordOnlyCodexRunner`。它只接收执行请求元数据并立即返回，不启动进程、不读取或修改工作区、不执行验证命令。
- UI 中所有状态按钮都明确标记为“记录状态”，不得暗示 Codex 已真实运行。
- Execution 写入不持有 Project、Snapshot、Session 或 Evidence 的写入用例，因此执行状态不能改变正式研发事实。

## 优点

- Proposal 的人工审批语义不会被运行状态污染。
- 同一 Proposal 可以保留多次成功、失败和取消尝试。
- 未来真实 Runner 可以通过 Port 接入，而不把进程控制写入 Domain、IPC 或 React。
- 输出摘要和验证结果与具体执行尝试绑定，便于审计和重启恢复。
- Project 与 Proposal 复合外键阻止跨项目执行请求。

## 缺点

- D5 只能记录执行请求和外部提供的结果，不能产生真实代码修改。
- 当前状态需要显式记录，尚无后台任务进度和 Runner 回调。
- 真正执行前仍需设计工作目录授权、取消协议、输出流、进程恢复和安全隔离。

## 长期影响

- 真实 `codex exec` 必须由新的 Runner Adapter 和独立 ADR 引入，不得修改 `RecordOnlyCodexRunner` 来绕过审批。
- Runner 执行、进度事件与数据库状态之间需要可靠交付机制；未来应评估本地 Outbox 或可恢复 Job，而不是在数据库事务中启动进程。
- Validation Command 目前只是 Proposal 契约文本。未来执行它们需要单独权限和结构化结果，不得把用户输入直接交给 Shell。
- 自动 Commit、Push、Merge、MCP 写入和 Agent 仍属于后续独立阶段。
