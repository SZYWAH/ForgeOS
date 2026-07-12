# ADR-008：AI 输出必须经过 Candidate 治理层

- 状态：已接受
- 日期：2026-07-11
- 决策范围：AI Response 持久化、人工审核与正式事实隔离
- 前置决策：ADR-005（Evidence 独立模型）、ADR-006（AI Provider Foundation）、ADR-007（Context Pack 与手动 AI 调用）

## 背景

D1.5 已建立从可信研发事实到 Context Pack，再到用户显式 AI 调用的完整读取链路。AI Response 当前只存在于页面内存，因此不会污染正式事实，但有价值的风险发现、总结和草稿在页面关闭后也会丢失。

直接把 AI Response 写成 Snapshot、Session、Evidence 或 Decision，会把模型推断伪装成人工确认事实，并绕过现有版本、来源与验证规则。ForgeOS 需要一个独立治理层，让建议可保存、可审核、可拒绝，同时不获得正式事实写权限。

## 候选方案

### 方案一：AI Response 继续完全不持久化

安全边界清晰，但有价值的建议无法进入后续人工复盘，也不能记录用户是否认可或拒绝。

### 方案二：AI 根据候选类型直接更新对应领域对象

操作快捷，但 Snapshot Update、Session Summary 或 Evidence Suggestion 会绕过人工编辑和确认，把不可靠推断写入可信记忆层。

### 方案三：独立 AICandidate 与人工审核状态机

用户在查看 Response 后执行第二次显式保存，形成 Pending Candidate。Accept、Reject、Archive 只更新 Candidate 自身；进入正式领域需要未来独立的人工采纳用例。

## 决策

选择方案三。

### Candidate 身份与来源

- Candidate 必须属于一个 Project，并引用生成它时使用的 ContextExport。
- Application 校验来源归属；SQLite 使用 `(source_context_export_id, project_id)` 复合外键再次保护。
- 支持 Snapshot Update、Session Summary、Evidence Suggestion、Risk Finding 和 General Note 五种类型。
- 类型只描述建议意图，不授予对应正式领域的写权限。

### 创建规则

- AI 调用仍只返回临时 Response。
- 用户可以仅查看，也可以显式选择类型、标题并点击 Save as Candidate。
- Domain 的创建入口固定生成 `pending / version = 1`，调用者不能指定 accepted 等初始状态。
- Candidate 保存 AI Response 内容，不保存完整 Conversation 或 Prompt History。

### 审核生命周期

```text
pending → accepted
pending → rejected
pending → archived
```

- 三种转换都是终态，不允许再次改变。
- Review 使用乐观并发版本；Repository 还检查数据库状态仍为 Pending。
- Reject 与 Archive 保留完整记录，不执行物理删除。
- Accept 只记录“用户认可建议”，不等同于事实已经更新。

### 正式事实隔离

- AiCandidateKernel 不持有 SnapshotRepository、SessionRepository 或 EvidenceRepository 的写入能力。
- Accept 不调用 `update_snapshot`、`update_session`、`create_evidence` 或 Project 更新。
- Candidate 内容不能自动改变 Project status、lifecycle 或优先级。
- 未来 Candidate 采纳必须新增独立、类型化、可预览且要求人工确认的用例。

## 优点

- 有价值的 AI 输出可以保留和追溯，同时继续明确标记为非事实。
- Pending、Accepted、Rejected、Archived 提供可审计的人工判断记录。
- 双层 Project 归属约束防止跨项目候选串线。
- Application 权限边界使 Accept 无法误调用正式领域写入。
- 为未来“建议 → 人工编辑 → 正式事实”流程提供稳定来源，但不提前实现自动采纳。

## 缺点

- Accepted Candidate 与正式项目状态可能长期不一致，需要用户理解“认可建议”不等于“已应用”。
- 当前不能编辑 Candidate 内容，也不能记录审核备注或审核人。
- Response 保存为 Candidate 是第二次操作，增加少量交互成本。
- 不保存完整 Prompt History，未来复现 AI 推理仍以 ContextExport 和 Candidate 内容为主。

## 长期影响

- 任何 AI 写入正式领域的需求都必须从 Candidate 或等价候选层进入，禁止从 Provider 回调直接写事实表。
- Candidate 采纳阶段必须允许用户编辑最终内容，并继续遵守目标领域的版本和验证规则。
- Agent、MCP、Codex 自动任务不得把 Candidate 的 Accepted 状态解释为自动执行授权。
- 若未来增加审核备注、来源模型或调用审计，必须继续排除 API Key，并明确隐私与保留策略。
