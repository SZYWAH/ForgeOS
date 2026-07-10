# ADR-004：Development Session 作为研发过程最小单位

- 状态：已接受
- 日期：2026-07-11
- 决策范围：ForgeOS D1.3 的研发活动边界、Session 生命周期与并发写入语义
- 前置决策：ADR-002（Project Identity Kernel）、ADR-003（Current Snapshot 版本化）

## 背景

Project Identity 回答“这个项目是什么”，Current Snapshot 回答“这个项目现在在哪里”，但两者都不解释一次具体研发活动为什么开始、做了什么、取得了什么结果、如何验证，以及下一次如何继续。

如果 ForgeOS 只保存当前状态，项目可以恢复上下文，却无法还原状态形成的过程；如果把过程记录降格为普通日志、日记或自由笔记，又会缺少稳定边界，很难判断哪段内容属于同一次研发活动，是否已经收尾，以及结果是否经过验证。

Development Session 因此需要成为一等领域实体。它必须满足：

- 每次研发活动有明确 Project 归属和目标。
- 研发过程中可以持续补充 Notes。
- 完成时明确记录结果、验证与 Handoff；放弃时也必须留下 Handoff，说明下次如何继续或为何转向。
- 用户能快速识别当前仍在进行的活动，并查看已经结束的历史。
- 陈旧页面和多窗口不能覆盖较新的内容或重复开始两个 Open Session。
- 已结束的事实不能被普通编辑静默重写。

Session 不是 Todo、时间追踪、Commit 容器或每日流水。D1.3 也不引入 AI 总结、Git/GitHub、Evidence、Timeline 或 Codex Context Pack。

## 候选方案

### 方案一：使用连续开发日志

每个 Project 维护一份按时间追加的 Markdown 日志。写入简单，也适合自由记录，但日志没有明确开始和结束边界，无法可靠区分当前活动与历史活动，结果、验证和 Handoff 只能依赖文本约定。它最终容易退化为日记或普通笔记。

### 方案二：以 Task、Commit 或时间段作为记录单位

Task 可以表达待完成事项，Commit 可以表达代码变化，计时记录可以表达持续时长，但它们都不能完整表达一次研发活动的动机、过程、非代码结果、验证方式与接手上下文。该方案还会把 ForgeOS 推向任务管理、Git 管理或工时统计，偏离可信研发记忆的产品边界。

### 方案三：显式 Development Session 聚合与单向状态机

每次活动创建独立 Session，从 `open` 开始，只能进入 `completed` 或 `abandoned`。Open 状态允许维护目标与 Notes；完成时填写 Outcome、Validation 和 Handoff；放弃时保留 Notes 并填写 Handoff，但不虚构 Outcome 或 Validation。终态只读，所有记录严格归属于 Project。

## 决策

ForgeOS 选择方案三：Development Session 是研发过程的最小领域单位，并采用 `open → completed | abandoned` 的单向生命周期。

### 聚合边界

- 一个 Session 代表一次有明确开始原因和收尾语义的研发活动。
- Session 直接归属于 Project，不直接归属于某个 Snapshot 版本。Snapshot 表达项目当前状态，Session 表达状态形成过程，两者保持不同事实边界。
- `goal` 表达为什么开始，`notes` 保存过程信息，`outcome` 表达完成结果，`validation` 表达如何确认结果有效，`handoff` 保存下一次继续所需信息。
- Session 不拆分 Task，不统计用时，不自动关联 Git Commit，也不充当 Evidence 或 Timeline。

### 生命周期

```text
             ┌─────────── completed
open ────────┤
             └─────────── abandoned
```

- Session 只能以 `open` 状态开始。
- Open Session 可以更新 `goal` 与 Markdown `notes`。
- `complete_session` 要求非空 Outcome、Validation 与 Handoff，确保完成结果经过说明、验证且能够被下一次研发活动接手。
- `abandon_session` 保留已有 Goal 与 Notes，Outcome 和 Validation 为空，但 Handoff 必须非空，用于记录再次尝试、转向其他方案或理解放弃原因所需的上下文。
- `completed` 和 `abandoned` 都是不可逆终态，不允许重新打开、相互转换或修改核心内容。
- `completed_at` 在两种终态中都记录离开 Open 状态的时间；Open Session 中必须为空。
- Project 归档后不能开始新 Session，也不能继续编辑 Goal 或 Notes；归档前遗留的 Open Session 仍允许完成或放弃，确保历史能够进入明确终态。

### 单一 Open Session

- 一个 Project 最多有一个 Open Session。
- Application 在开始前检查当前 Session，以便返回清晰错误。
- SQLite 使用 `(project_id) WHERE status = 'open'` 的部分唯一索引作为最终并发保护；终态记录不受该唯一限制，因此完整历史可以持续积累。
- 该规则约束的是用户当前研发焦点，而不是全局只能开发一个项目；不同 Project 可以各自拥有一个 Open Session。

### 并发与持久化

- Session 初始 `version = 1`。
- 每次 Open 编辑、完成或放弃都携带 `expected_version`，并通过 `WHERE status = 'open' AND version = expected_version` 更新。
- 成功修改后 `version` 递增；若记录已经变化或进入终态，返回版本冲突或终态错误，不覆盖较新的事实。
- 终态转换一次性写入状态、收尾字段、`completed_at`、`updated_at` 与新版本，避免产生“结果已保存但状态仍 Open”等中间事实。
- React 只通过类型化 IPC 调用 Application，用例和 Domain 持有状态机规则，Repository 负责 SQL 与冲突识别。

### 与 Current Snapshot 的关系

完成或放弃 Session 不会自动改写 Current Snapshot。Session 收尾形成可复核的研发过程记录，用户随后可以人工确认新的项目状态并创建 Snapshot 新版本。

这一边界避免把 Session Outcome 未经确认地提升为“项目当前事实”，也为未来受控的 Snapshot 建议保留空间。D1.3 不自动总结、不自动生成 Snapshot，AI 和外部工具不能直接修改正式事实。

## 优点

- 每次研发活动都有稳定、可查询的边界，既能显示 Current Session，也能形成清晰历史。
- Goal、Notes、Outcome、Validation 与 Handoff 覆盖从开始到接手的完整最小闭环。
- `abandoned` 保留未完成尝试，不把无结果活动从历史中删除，也不伪装成成功。
- 终态不可变，能够保护已经确认的结果与验证信息，提升研发记忆可信度。
- 单一 Open 约束让用户明确当前活动，同时保留多个项目分别进行研发的能力。
- 版本检查和数据库部分唯一索引共同阻止陈旧写入与并发重复开始。
- 模型不包含优先级、截止时间、工时或完成百分比，避免退化为任务管理工具。
- 与 Snapshot 分离后，当前状态和研发过程可以各自演进，不会出现一份文本承担两种事实语义。

## 缺点

- 用户需要主动开始并收尾 Session，记录质量仍依赖人工维护。
- 每个 Project 只能有一个 Open Session，不能原生表达同一项目中并行进行的多条个人研发活动。
- 终态不可修改意味着发现笔误后不能直接修正；D1.3 尚未提供正式修订机制。
- `notes` 是 Markdown 大文本，数据库不能对其中的行动进行结构化查询。
- `completed_at` 同时表示完成或放弃时间，读取时必须结合 `status` 解释。
- Session 只能说明用户记录的验证方式，不等同于独立 Evidence，也不自动证明代码或部署状态。
- Session 收尾不会自动更新 Snapshot，用户需要额外执行一次状态确认才能完成 Project 状态闭环。

## 长期影响

- Development Session 历史属于正式研发记忆，Project 归档、开始新 Session 或未来清理功能不得静默删除终态记录。
- 未来 AI 可以基于 Session 提出摘要、Handoff 或 Snapshot 更新建议，但建议必须由用户确认，不能绕过 Domain、版本检查或终态不可变规则。
- Git Commit、分支与 GitHub 信息若未来接入，应作为来源或关联信息扩展，不能取代 Session 的目标、结果、验证和接手语义。
- Evidence 应保持独立领域模型。Session 的 `validation` 描述“如何验证”，未来 Evidence 才保存可审计的证明材料或引用。
- Context Pack 可以读取 Current Snapshot、当前 Open Session 和最近终态 Session，但必须保持 Project 隔离并标明来源；D1.3 不实现该能力。
- 若未来确需修正终态事实，应引入可审计的 Revision 机制，而不是恢复普通编辑或原地覆盖。
- 若一个 Project 需要并行工作流，应通过新的产品决策重新评估“一个 Open Session”约束，不能在 Repository 层绕过部分唯一索引。
- Timeline 若后续实现，应投影 Session 生命周期事件，而不是让 Session 退化为通用事件表。
