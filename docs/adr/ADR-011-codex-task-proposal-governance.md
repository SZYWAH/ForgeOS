# ADR-011：Codex Task Proposal 与任务执行分离

- 状态：已接受
- 日期：2026-07-12
- 决策范围：Codex 编码任务描述、人工审批与未来执行授权

## 背景

ForgeOS 已能组织可信研发上下文并通过只读 MCP 提供给外部 AI。下一步需要表达“希望 Codex 完成什么”，但如果任务定义同时触发代码执行，用户将无法在执行前确认目标、非目标、允许范围、禁止范围和验收标准。

## 候选方案

1. 直接从 Context Pack 启动 Codex：流程短，但把上下文选择、任务审批和代码执行合并，缺少治理边界。
2. 使用普通 Todo/Issue：能够记录标题，却无法冻结 ContextExport、执行范围和验证命令，也会让 ForgeOS 退化为任务管理器。
3. 建立独立 Proposal 层：任务请求先成为版本化、可审核的 Proposal，批准只记录未来执行授权。

## 决策

选择方案三。CodexTaskProposal 必须绑定同一 Project 的 ContextExport。生命周期固定为 `draft → proposed → approved | rejected`，仅 `approved → archived`。所有状态变化追加历史记录并使用乐观版本控制。

Approved 不启动 Codex、不修改代码、不创建分支、不运行命令、不提交 Git，也不修改 Project、Snapshot、Session 或 Evidence。未来执行能力必须使用新的执行领域、权限模型和 ADR，不得扩展当前审批方法来隐式执行。

## 优点

- 执行前可以人工检查完整任务契约和固定上下文版本。
- 明确区分研发事实、AI 建议、任务提案和未来执行结果。
- 允许后续审计批准过的语义范围，而不把批准误认为任务已经完成。
- ContextExport 复合外键阻止跨 Project 上下文串线。

## 缺点

- 创建 Proposal 比直接输入一句提示更慢。
- D4 不能产生代码成果，Approved 仍需未来独立执行流程。
- 当前没有 Draft 编辑用例；错误的 Draft 需要重新创建。

## 长期影响

- Codex 执行、Sandbox、Branch、Commit、Push 和 Merge 必须位于 Proposal 之外。
- 未来执行记录必须引用 Approved Proposal 的固定版本和 ContextExport。
- MCP 第一阶段继续只读；不得通过 MCP 绕过人工审批。
