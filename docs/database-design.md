# ForgeOS 数据库设计

## 当前范围

D1.2 数据库承载 ForgeOS 的 Project Identity Kernel 与 Current Snapshot Kernel，包含：

- 单用户的唯一 `Workspace`
- 归属于该 Workspace 的多个 `Project`
- 归属于 Project、按版本只追加保存的 `ProjectSnapshot`

Current Snapshot 只表达“项目现在真实处于什么状态”，不是 Todo、Task、进度百分比或看板。
本阶段不创建 Development Session、Decision、Problem、Evidence、AI、Git、GitHub、
Codex Context Pack、Search 或 Timeline 相关表。

## 技术选择

- 数据库：SQLite
- 访问层：Rust + SQLx
- Migration：由应用启动流程执行
- 前端访问：禁止，React 必须通过类型化 Tauri IPC 调用 Rust
- Migration 源文件：`database/migrations`
- 运行时数据库：默认位于 Tauri 应用数据目录，也可通过 `FORGEOS_DATA_DIR` 指定；本地验证可使用被 Git 忽略的 `.data/forgeos.db`

## 通用约定

- 表名和列名使用 `snake_case`
- ID 使用 `TEXT`，由 Rust 生成
- 时间使用 UTC RFC 3339 文本
- SQLite 连接开启外键约束
- 所有查询使用参数化 SQL
- Migration 只向前追加，已经应用的 Migration 不得修改
- 开发数据库和 Migration 源文件必须分离
- SQLite 数据库及 WAL/SHM 文件不提交到 Git
- 删除与归档是不同语义；Project 归档通过状态变更完成，不删除记录
- 结构化文本列表在 SQLite 中保存为 JSON 文本，由 Domain 与 Repository 边界负责校验和转换

## `workspaces`

表示 ForgeOS 当前的顶层工作空间。

| 字段 | 类型 | 约束 | 说明 |
|---|---|---|---|
| `id` | TEXT | PRIMARY KEY | Workspace ID |
| `name` | TEXT | NOT NULL | Workspace 名称 |
| `timezone` | TEXT | NOT NULL | 用户显示时区 |
| `created_at` | TEXT | NOT NULL | UTC 创建时间 |
| `updated_at` | TEXT | NOT NULL | UTC 更新时间 |

### Workspace 规则

- D1.1 只支持单个 Workspace，不实现多用户、Workspace 切换或权限。
- `name` 和 `timezone` 必须是去除首尾空白后的非空文本。
- 创建 Workspace 时，应用用例先检查当前 Workspace；已存在时拒绝再次创建。
- 数据库通过唯一常量表达式索引提供第二层单例保护，避免并发请求插入第二条 Workspace。
- Workspace 已被 Project 引用时不得静默删除；D1.1 不提供删除 Workspace 的用例。

## `projects`

Project 表示 ForgeOS 管理的软件项目身份，也是后续研发记录的归属边界。

| 字段 | 类型 | 约束 | 说明 |
|---|---|---|---|
| `id` | TEXT | PRIMARY KEY | Project ID |
| `workspace_id` | TEXT | NOT NULL, FOREIGN KEY | 所属 Workspace |
| `slug` | TEXT | NOT NULL | Workspace 内稳定的机器可读标识 |
| `name` | TEXT | NOT NULL | 项目名称 |
| `description` | TEXT | NOT NULL | 项目简短描述 |
| `vision` | TEXT | NOT NULL | 项目长期愿景 |
| `target_user` | TEXT | NOT NULL | 目标用户 |
| `scope` | TEXT | NOT NULL | 当前产品范围 |
| `non_goals` | TEXT | NOT NULL | 明确排除的目标 |
| `tech_stack` | TEXT | NOT NULL | 技术栈说明 |
| `status` | TEXT | NOT NULL, CHECK | 项目生命周期状态 |
| `lifecycle_stage` | TEXT | NOT NULL, CHECK | 当前研发阶段 |
| `priority` | INTEGER | NOT NULL, CHECK | Workspace 内的相对优先级，范围 `0..100` |
| `created_at` | TEXT | NOT NULL | UTC 创建时间 |
| `updated_at` | TEXT | NOT NULL | UTC 更新时间 |
| `version` | INTEGER | NOT NULL, CHECK | 从 `1` 开始的乐观并发版本号 |

### Project 状态枚举

`status` 只允许：

- `idea`
- `planned`
- `active`
- `paused`
- `maintenance`
- `archived`

`lifecycle_stage` 只允许：

- `discovery`
- `definition`
- `design`
- `implementation`
- `validation`
- `release`
- `operation`
- `retrospective`

枚举值同时由 Domain 类型和数据库 `CHECK` 约束保护。数据库约束负责阻止非法持久化，具体业务校验由 Domain 负责。

### Project 约束与索引

- `workspace_id` 引用 `workspaces.id`。
- 删除 Workspace 时采用 `RESTRICT` 语义，避免静默丢失 Project。
- `workspace_id + slug` 使用不区分大小写的唯一索引；不同 Workspace 的同名 slug 为未来扩展保留语义，但 D1.1 仍只有一个 Workspace。
- `slug` 由小写 ASCII 字母、数字和连字符组成，长度为 `2..64`，由 Domain 校验。
- `name` 必须是去除首尾空白后的非空文本。
- `priority` 取值 `0..100`，默认值为 `50`。
- `version` 初始值为 `1`；每次成功更新或归档后递增。
- 为 `workspace_id + updated_at` 建立列表查询索引，支持 Dashboard 和 Projects 页面按最近更新时间读取。
- 为 `workspace_id + status` 建立状态过滤索引，为后续受控查询扩展保留基础。

### 更新与归档

Project 更新使用乐观并发控制：

```text
读取 Project(version = N)
→ 用户提交更新并携带 expected_version = N
→ UPDATE ... WHERE id = ? AND version = N
→ 成功时 version = N + 1
→ 未更新任何行时返回并发冲突，不覆盖较新的事实
```

归档是将 `status` 改为 `archived` 的专用用例，同样检查预期版本并递增 `version`。普通更新用例不接受目标状态 `archived`，已归档 Project 也不能通过普通更新继续编辑。归档不会删除 Project，也不会破坏未来研发记录的归属与审计链。

## `project_snapshots`

ProjectSnapshot 保存某个 Project 在一次人工确认时的真实状态。每次更新都插入新版本，既有版本不原地修改；某个 Project 的最高版本就是其 Current Snapshot。

| 字段 | 类型 | 约束 | 说明 |
|---|---|---|---|
| `id` | TEXT | PRIMARY KEY | Snapshot 版本的稳定 ID |
| `project_id` | TEXT | NOT NULL, FOREIGN KEY | 所属 Project |
| `version` | INTEGER | NOT NULL, CHECK | Project 内从 `1` 开始连续递增的版本号 |
| `current_goal` | TEXT | NOT NULL | 当前最重要目标 |
| `status_summary` | TEXT | NOT NULL | 当前整体状态总结 |
| `achievements` | TEXT | NOT NULL | JSON 文本数组，保存最近验证成果 |
| `blockers` | TEXT | NOT NULL | JSON 文本数组，保存当前阻塞 |
| `next_steps` | TEXT | NOT NULL | JSON 文本数组，保存最多三个下一步方向 |
| `confirmed_at` | TEXT | NOT NULL | 用户确认该版本事实的 UTC 时间 |
| `created_at` | TEXT | NOT NULL | 该版本创建的 UTC 时间 |
| `updated_at` | TEXT | NOT NULL | 该版本写入的 UTC 时间；不可变版本不会再被更新 |

### Snapshot 归属与内容规则

- `project_id` 必须引用真实存在的 `projects.id`；外键采用 `ON UPDATE CASCADE` 与 `ON DELETE RESTRICT`，禁止产生孤立 Snapshot 或静默删除历史。
- 一个 Project 在首次创建前可以没有 Snapshot，创建后在任一时刻最多只有一个 Current Snapshot；Current 不是可变标记，而是按 `project_id` 读取 `version` 最大的记录所得的确定性投影。
- 首次创建 Snapshot 时版本必须为 `1`；后续更新只能追加 `N + 1`，不能覆盖、删除或跳过已有版本。
- `achievements`、`blockers` 与 `next_steps` 在 Domain 中表现为结构化字符串列表，在 SQLite 中编码为 JSON 文本数组。
- `current_goal` 与 `status_summary` 去除首尾空白后必须非空，单字段最长 20,000 个字符；数据库也拒绝空白内容。
- 列表项会去除首尾空白、忽略空项并按首次出现去重；单项最长 2,000 个字符。数据库使用 `json_valid` 与 `json_type = 'array'` 防止非数组 JSON 持久化。
- `next_steps` 最多包含三项；超过限制时 Domain 必须在持久化之前拒绝。
- `confirmed_at` 表示用户确认状态事实的时间，不由 AI、Git 或其他自动化来源代替确认。
- 已归档 Project 的身份与 Snapshot 历史仍可读取，但不能继续创建或更新 Snapshot。
- Snapshot 不包含任务完成状态、负责人、截止日期、进度百分比或看板列。

### Snapshot 版本与并发

Snapshot 更新采用追加版本与乐观并发检查：

```text
读取 Current Snapshot(version = N)
→ 用户编辑并提交 expected_version = N
→ Application 确认 expected_version = N 并校验领域规则
→ INSERT version = N + 1
→ Current Snapshot 确定性地变为最高版本 N + 1
→ 并发写入由 UNIQUE(project_id, version) 拒绝重复版本
→ 回读最新版本并返回并发冲突，不覆盖较新的事实
```

`project_id + version` 必须唯一。该约束既保护每个 Project 的版本序列，也使并发写入无法创建两个相同版本。历史查询按版本倒序返回完整记录；Current 查询使用同一版本序列的最高版本，不维护额外的可变指针、`is_current` 标志或 Project 表字段。

### Snapshot 索引

- `UNIQUE(project_id, version)` 保证 Project 内版本唯一，并支持按 Project 查询版本。
- 为 `project_id + version DESC` 的 Current 与历史读取提供索引能力；具体索引由 Migration 定义，避免扫描其他 Project 的历史。
- Snapshot 写入只通过 Application 用例执行，并且只能追加 `INSERT`；Repository 不向前端暴露 SQL 或 JSON 存储细节。

## Migration 设计

### `0001_create_workspaces_and_projects.sql`

D0 Migration，负责创建 Workspace、Project 的最小表结构和初始外键关系。该文件保持不变。

### `0002_project_identity_kernel.sql`

D1.1 通过新的向前 Migration：

1. 为 Workspace 增加数据库级单例约束。
2. 重建并完善 Project 表，增加产品信息、技术信息、状态、优先级和版本字段。
3. 为 Project 状态、生命周期、优先级和版本增加约束。
4. 保留 `workspace_id` 外键和 Workspace 内 slug 唯一约束。
5. 增加列表查询与状态查询所需索引。
6. 将已有 D0 Project 行迁移到 D1.1 默认值，不丢弃既有数据。

### `0003_current_snapshot.sql`

D1.2 通过新的向前 Migration：

1. 创建只追加版本的 `project_snapshots` 表。
2. 通过外键将每个 Snapshot 严格归属于一个 Project，并阻止静默级联删除历史。
3. 为版本号增加正整数约束，并通过 `UNIQUE(project_id, version)` 保证 Project 内版本唯一。
4. 创建 `project_snapshots_project_version_index (project_id, version DESC)`，支持 Current Snapshot 与历史版本查询。
5. 将成果、阻塞和下一步以 JSON 文本数组持久化，并使用 SQLite JSON CHECK 拒绝非法或非数组 JSON；列表含义与数量规则由 Domain 负责。
6. 不增加可变 Current 指针、Task 表或生产测试数据。

Migration 文件是数据库结构的执行事实源。后续结构变化必须增加新 Migration，
不得回写 `0001`、`0002` 或 `0003`。

## 启动流程

```text
确定数据库路径
→ 创建父目录
→ 打开 SQLite
→ 开启 foreign_keys
→ 设置连接超时与 WAL
→ 执行未应用 Migration
→ 查询数据库健康状态
→ 组装 Repository 与 Application Service
→ 注册 Tauri Commands
```

Migration 失败时应用必须报告明确错误，不得继续以正常状态运行。

## 数据访问与事务规则

- React 不直接读写 SQLite，也不在 `localStorage` 保存正式 Project 事实。
- IPC 不执行 SQL，写入必须经过 Application 用例、Domain 校验和 Repository 端口。
- Repository 负责 SQLx 查询、行映射和持久化错误转换。
- 相关写入失败时必须完整回滚。
- 网络、AI 和大文件复制不得发生在数据库事务内。
- D1.2 测试数据仅由测试或人工验收流程创建，不在生产启动时自动插入。

## 后续演进

以下概念已冻结，但不在 D1.2 建表：

- Development Session
- Decision
- Problem 与 Investigation Attempt
- Evidence
- Context Export

它们必须在对应阶段通过新的 Migration 增加，并以 `project_id` 建立清晰归属。
D1.2 不提前创建空表，也不实现通用 Task 数据模型。Current Snapshot 当前仅由人工维护；
AI 生成、Git/GitHub 同步、Codex Context Pack 与 Timeline 留待独立阶段。
