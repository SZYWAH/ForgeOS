# ADR-007：Context Pack 确定性生成与 AI 输出隔离

- 状态：已接受
- 日期：2026-07-11
- 决策范围：Context Pack 来源、Renderer、持久化与手动 AI 调用边界
- 前置决策：ADR-003（Snapshot 版本化）、ADR-004（Session 生命周期）、ADR-005（Evidence 独立模型）、ADR-006（AI Provider Foundation）

## 背景

ForgeOS 已能保存 Project 身份、人工确认的 Current Snapshot、Development Session 与 Evidence，但这些事实分散在不同实体中。Codex 或其他 AI 若直接读取数据库、临时拼接上下文或自行判断事实，会造成项目混淆、不可复现输入和错误事实被放大的风险。

D1.5 需要建立第一条完整但克制的 AI 协作链路：ForgeOS 先把某一 Project 的已确认事实组织为可检查的 Markdown，用户预览后再主动选择 Provider 和问题。AI 只返回分析，不获得事实写权限。

## 候选方案

### 方案一：前端临时拼接上下文并直接调用 Provider

实现简单，但业务规则散落在 React，刷新后无法追溯实际发送内容，也难以证明项目隔离和确定性。

### 方案二：让 AI 自行查询数据库并自动更新 Snapshot

自动化程度高，但 AI 可以选择、解释甚至覆盖事实，违反可信研发记忆和用户确认原则，也提前引入 Agent 与 Tool Calling。

### 方案三：Rust 确定性 Renderer、持久化 ContextExport、用户显式单次调用

Application 层读取限定 Project 的来源，由无状态 Renderer 按固定章节和排序生成 Markdown；保存来源版本与 SHA-256。用户预览后显式发送，AI Response 只返回页面内存。

## 决策

选择方案三。

### 确定性 Context Pack

- 输入只来自目标 Project 的 Project Identity、Current Snapshot、最近 Development Session 与全部 Evidence。
- 章节顺序固定，不包含生成时间、Export ID 或其他不稳定值。
- Evidence 按标题和 ID 稳定排序；缺失来源使用固定文本表达。
- Planning、Coding、Debugging、Review、Handoff 模板只改变明确的 Purpose，不改变事实章节结构。
- Markdown 使用 SHA-256 生成小写十六进制 `content_hash`；相同来源必须得到相同内容和哈希。

### 来源与持久化

- ContextExport 记录 `snapshot_version` 与 `generated_from_session_ids`，便于识别输入版本。
- ContextExport 是派生记录，不是 Project 当前事实；内容创建后不可编辑。
- `status` 只允许 `generated → sent_to_ai`，状态更新使用乐观版本。
- 不保存 Prompt、Response、Conversation 或 Tool Call 历史。

### 手动 AI 调用

- 只有用户点击 Send to AI 才执行网络请求，不存在后台、定时或自动触发。
- 只能选择 Enabled Provider；Disabled Provider 在 HTTP 请求前被拒绝。
- 调用内容为固定分析边界 System Message、完整 Context Pack 和用户单次问题。
- AI Response 只显示为当前页面的 Temporary Response；切换或刷新页面后消失。

### 正式事实边界

- AI 不更新 Snapshot、Session、Evidence 或 Project。
- AI 不创建正式 Decision，不生成 Evidence，也不执行任务。
- 调用成功唯一允许的数据库变化是 ContextExport 状态变为 `sent_to_ai`。
- 未来若需要采纳 AI 建议，必须新增人工审阅和确认用例，不能复用当前调用接口直接写事实。

## 优点

- Context Pack 可预览、复制、哈希验证和重现，适合后续 Codex 接入。
- Project 归属校验和测试防止多项目上下文串线。
- Renderer 与 Provider 解耦，同一 Markdown 可复制使用或发送给任意兼容 Provider。
- AI 没有事实 Repository 写权限，错误分析不会污染可信研发记忆。
- 不引入聊天、Agent、MCP 或任务执行复杂度。

## 缺点

- 当前只选取一个最近 Session，尚不支持用户选择时间范围或来源集合。
- Context Pack 内容较大时可能受到 Provider 上下文窗口或现有消息长度限制。
- Prompt 与 Response 不持久化，关闭页面后无法复查当次分析。
- SHA-256 只能证明内容一致，不能证明事实本身正确；事实仍依赖人工维护和 Evidence。

## 长期影响

- Codex Markdown Context Pack 应直接复用本 Renderer 输出，不另建一套拼接规则。
- AGENTS.md、MCP 或自动协同必须复用 Application/Domain 边界，并另行评审授权、审计和写入策略。
- AI 主动分析只能在后续阶段新增，默认仍不得自动修改正式事实。
- 若未来持久化 AI 建议，应使用独立候选层并要求人工确认，不能把 Response 直接映射到 Snapshot、Session、Evidence 或 Decision。
