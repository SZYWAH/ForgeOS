# ADR-005：Evidence 独立于 Snapshot 与 Development Session

- 状态：已接受
- 日期：2026-07-11
- 决策范围：ForgeOS D1.4 的证据边界、验证状态与统一关联模型
- 前置决策：ADR-002（Project Identity Kernel）、ADR-003（Current Snapshot 版本化）、ADR-004（Development Session 生命周期）

## 背景

Project 描述“项目是什么”，Current Snapshot 描述“项目现在在哪里”，Development Session 描述“一次研发活动发生了什么”。这些记录都可以包含用户陈述的事实，但它们本身不能回答“为什么这个事实值得相信”。

例如，Snapshot 可以写“流式响应优化已经完成”，Session 可以写“cargo test 通过”，但支持这些判断的 README、测试输出、截图、外部链接或构建产物仍需要稳定保存。如果把证据路径直接塞进 Snapshot 或 Session 文本，会产生几个问题：

- 同一份证据无法同时支持 Project、Snapshot 与 Session。
- Snapshot 版本化会重复复制证据信息，Session 终态不可变也不适合后补引用。
- 证据是否经过人工确认、是否已经失效无法独立表达。
- 修改或移除来源信息可能静默改变已经确认的研发事实。
- 未来 Git、GitHub 或 AI 能力容易绕过正式领域边界，直接改写状态或过程记录。

D1.4 需要建立独立 Evidence Kernel，但必须保持人工创建、本地优先和可信事实边界，不引入自动采集、自动验证、Context Pack 或知识图谱。

## 候选方案

### 方案一：把证据字段直接加入 Snapshot 与 Session

实现简单，读取目标记录时即可看到证据，但会让 Snapshot 与 Session 同时承担状态、过程和来源管理职责。同一证据需要重复保存，验证状态也会依附在不同实体上，后续难以统一查询和复用。

### 方案二：只保存自由 Markdown 引用

用户可以在 Snapshot、Session 或普通笔记中粘贴路径和 URL。该方案灵活，但无法稳定区分证据类型、来源类型、验证状态和关联目标，也不能用数据库约束防止跨 Project 混淆。

### 方案三：独立 Evidence 实体与统一 EvidenceLink

Evidence 直接归属于 Project，保存标题、说明、证据类型、来源引用、人工验证状态、时间和版本。EvidenceLink 使用统一的 `target_type + target_id + relation_type` 结构关联 Project、Snapshot 或 Session。Application 和数据库共同验证所有目标属于同一 Project。

## 决策

ForgeOS 选择方案三：Evidence 是独立领域实体，EvidenceLink 是统一的显式关联模型。

### Evidence 边界

- Evidence 必须直接归属于一个 Project，不能成为跨 Project 的全局附件。
- Evidence 保存支持事实的来源，不复制 Snapshot 当前状态，也不复制 Session 研发过程。
- Evidence 类型限定为 `document`、`screenshot`、`link`、`test_result`、`note` 与 `artifact`。
- 来源限定为本地文件路径、外部 URL 或人工记录。D1.4 只保存引用，不复制文件、不抓取 URL，也不检查来源当前是否可访问。
- Evidence 不直接修改 Project、Snapshot 或 Session。关联只表达“支持”或“相关”，不会触发目标状态变化。
- Evidence 没有删除用例；不再可信的记录标记为 `invalid`，保留其历史存在和引用关系。

### 人工验证状态

Evidence 使用三个状态：

```text
unverified → verified
     │           │
     └───────────┴──→ invalid
```

- 新建 Evidence 固定为 `unverified`。
- `verify_evidence` 表示用户已经人工确认该来源能够支持所记录事实。
- `invalidate_evidence` 表示该来源错误、过期、损坏或不再应被用作支持依据。
- Evidence 内容发生更新时自动回到 `unverified`。旧验证只针对旧内容，不能静默延续到修改后的标题、说明、类型或来源。
- 验证与失效都是显式 Application Use Case，不由前端直接写状态，也不由 AI 或自动化代替用户确认。

### 并发与审计

- Evidence 初始 `version = 1`。
- 内容更新、验证和失效都携带 `expected_version`，成功后版本递增。
- Repository 使用 `WHERE id = ? AND project_id = ? AND version = ?` 更新，陈旧窗口返回明确版本冲突，不覆盖较新的内容或验证判断。
- `updated_at` 记录最近一次内容或验证状态变化。D1.4 尚未建立完整 Record Revision，因此当前版本只保护并发，不保存每个旧版本的完整副本。
- 已归档 Project 的 Evidence 保持可读，但不能继续创建、编辑、验证、失效或增加关联。

### 统一关联模型

EvidenceLink 字段为：

- `evidence_id`
- `target_type`：`project`、`snapshot` 或 `session`
- `target_id`
- `relation_type`：`supports` 或 `relates_to`

选择统一关联表而不是三张专用表，可以在不修改 Evidence 本体的情况下支持现阶段三种目标，并让读取一条 Evidence 的关联保持单一接口。

多态 `target_id` 无法同时声明三个普通 SQLite 外键，因此采用双层保护：

1. Application 读取目标实体，确认目标存在并与 Evidence 属于同一 Project。
2. SQLite `BEFORE INSERT` 与受保护字段的 `BEFORE UPDATE` trigger 再次检查目标与 Project 归属，防止绕过 Application 或并发边界产生错误关联。

EvidenceLink 对 `evidence_id` 仍使用正式外键，并通过唯一约束阻止完全重复的关联。

## 优点

- 状态、过程与证据各自拥有清晰领域职责，不需要在 Snapshot 或 Session 中复制附件信息。
- 一条 Evidence 可以同时支持 Project、特定 Snapshot 和 Development Session。
- 验证状态独立可见，用户能区分尚未检查、已经确认和已经失效的来源。
- 内容更新自动取消旧验证，减少“来源已变化但仍显示 verified”的错误事实。
- Project 归属、乐观并发、Application 校验和数据库 trigger 共同降低跨项目混淆风险。
- 本地路径、URL 和人工记录覆盖 MVP 所需来源，同时不提前引入文件仓库或网络采集基础设施。
- `invalid` 保留历史，不通过删除掩盖曾经依赖过的错误证据。

## 缺点

- 用户仍需人工创建、维护和验证 Evidence，记录质量依赖使用习惯。
- 本地文件只保存路径，文件移动、磁盘变化或另一台机器上的路径差异不会自动修复。
- 外部 URL 不会自动检查可访问性、内容漂移或真实性。
- 多态关联无法使用纯普通外键表达，需要 Application 校验与 SQLite trigger 同时维护。
- Evidence 更新只有当前版本，D1.4 不能查看完整内容修订历史。
- 没有删除与解除关联用例，错误关联只能在后续经独立设计后修正。
- `verified` 表示用户人工确认，而不是密码学证明或第三方认证。

## 长期影响

- Snapshot 与 Session 不得增加重复的附件字段；未来读取证据应通过 EvidenceLink 投影。
- Git Commit、GitHub PR、CI 结果或构建产物未来可作为 Evidence 来源适配器，但自动采集结果必须先进入受控候选或 `unverified` 状态，不能直接修改 Snapshot、Session 或把自己标记为 verified。
- AI 可以建议 Evidence 标题、说明或关联，但所有正式写入和验证状态变更必须由用户确认，并继续通过 Application、Domain 与版本检查。
- Context Pack 未来可以读取与当前 Snapshot、当前 Session 或 Project 关联的 Evidence，但必须标明验证状态和来源，不得把 `unverified` 或 `invalid` 内容表述为已确认事实。
- 若需要保存截图或构建产物副本，应在后续设计 File Store Port、内容哈希、相对路径和生命周期策略，不能在 D1.4 中把任意外部文件静默复制进应用数据目录。
- 若需要修正 Evidence 历史或解除错误关联，应引入可审计的 Revision/Unlink 语义，而不是直接执行不可追踪的 SQL 删除。
- Evidence 目标类型增加时，必须同步更新 Domain 枚举、Application 归属校验、Migration 约束与 trigger，避免只有 UI 认识新类型。
