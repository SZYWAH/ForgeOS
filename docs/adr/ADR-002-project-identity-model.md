# ADR-002：将 Project Identity Kernel 作为首个领域核心

- 状态：已接受
- 日期：2026-07-10
- 决策范围：ForgeOS D1.1 的首个业务领域、数据归属边界与生命周期表达
- 前置决策：ADR-001（Tauri 2、Rust 与 SQLite）

## 背景

ForgeOS 的不可替代价值是保存项目隔离、人工确认、证据可追溯的研发记忆，并将其转换为开发者和 Codex 都能可靠使用的上下文。

Current Snapshot、Development Session、Decision、Problem、Evidence 与 Context Export 都必须明确属于某个软件项目。若没有稳定的 Project 身份和边界，这些记录将无法可靠隔离，后续可能出现跨项目上下文污染、事实归属不清和复盘材料混合。

D1.1 需要让 ForgeOS 第一次能够管理真实软件项目，但仍必须保持克制：本阶段只实现单 Workspace、Project 及 Project Overview，不提前实现研发过程对象、AI、Codex、Git、GitHub、Search 或 Timeline。

## 候选方案

### 方案一：先建立 Workspace 与结构化 Project Identity Kernel

由单例 Workspace 承载个人工作空间，Project 保存稳定身份、产品定义、技术栈、生命周期状态、优先级和并发版本。后续所有研发记录以 Project 作为归属边界。

### 方案二：先实现通用 Task 或项目看板

通过任务、列表和状态看板快速产生可操作界面，再逐步补充项目定义和研发记忆对象。

### 方案三：先实现自由文本笔记或文件夹

使用笔记和标签保存项目资料，减少初期结构化字段与领域规则。

### 方案四：先实现 Current Snapshot 或 Development Session

直接记录当前进度或每日会话，在记录中临时使用项目名称或标签区分归属。

## 决策

ForgeOS 选择方案一，将 Project Identity Kernel 作为首个领域核心：

- MVP 只支持一个 Workspace，由应用用例和数据库约束共同保证单例。
- 一个 Workspace 可以包含多个 Project。
- Project 保存稳定的 `id` 与 Workspace 内唯一 `slug`，用于建立可靠身份。
- Project 保存 `vision`、`target_user`、`scope`、`non_goals` 与 `tech_stack`，使项目身份不仅是名称，而是可恢复的产品和技术上下文。
- `status` 表达项目总体状态，`lifecycle_stage` 表达当前研发阶段；二者使用明确枚举，不使用任意文本。
- `archived` 是软归档状态，不删除 Project。
- `version` 用于乐观并发控制，更新和归档不得静默覆盖较新的事实。
- 业务规则进入 Domain，Application 编排用例，Repository 负责 SQLite 持久化，IPC 只承担类型化边界。
- D1.1 不引入通用 Task、研发过程表或任何 AI/Git 集成。

## 优点

- 从第一项业务能力开始建立严格的项目隔离边界。
- 为 Snapshot、Session、Decision、Problem、Evidence 和 Context Pack 提供统一归属基础。
- 产品、技术和生命周期信息可以支持 60 秒内恢复项目基础上下文。
- 状态枚举与 Domain 规则避免任意文本造成含义漂移。
- 软归档保留历史身份，为未来审计、复盘和职业材料提供连续性。
- 乐观并发版本避免多个窗口或陈旧页面静默覆盖正式事实。
- Workspace 与 Project 的边界允许未来扩展，同时不增加当前单用户操作复杂度。
- 不建立 Task 模型，从结构上避免 ForgeOS 在起步阶段退化为普通任务管理工具。

## 缺点

- 相比只有名称和描述的项目列表，结构化字段带来更多表单和校验工作。
- 单 Workspace 约束未来若扩展多 Workspace，需要新的产品决策和 Migration。
- `tech_stack`、`scope` 等字段当前以文本保存，暂不支持结构化筛选或关系查询。
- 状态和生命周期枚举的调整需要同时维护 Domain、IPC、前端类型与数据库 Migration。
- 乐观并发要求客户端携带版本，并处理明确的冲突错误。
- Project Identity Kernel 本身不能回答项目当前阻塞、验证成果或下一步；这些能力必须等待对应领域阶段。

## 长期影响

- 所有后续核心研发记录必须通过 `project_id` 归属于一个 Project。
- Context Pack 默认只能从单一 Project 的已确认记录生成，不得跨项目隐式拼接。
- Project 的 `id` 是内部稳定引用；`slug` 可修改时必须保持 Workspace 内唯一，不能替代数据库外键。
- Project 归档不级联删除任何历史记录；未来若增加删除能力，必须通过新 ADR 规定审计、备份和引用处理。
- Project 更新继续使用版本检查；未来 AI 或外部适配器也不得绕过用例静默修改正式事实。
- 新状态或生命周期阶段必须通过显式领域与数据库演进完成，不得直接写入未定义值。
- Dashboard 在 D1.1 只展示项目身份与生命周期信息；后续首页仍应围绕可信状态、阻塞、成果和新鲜度演进，而不是任务数量。
- Current Snapshot 及其他领域必须等待独立阶段实现，不能借 Project 字段提前模拟。
