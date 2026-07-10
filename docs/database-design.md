# ForgeOS 数据库设计

## 当前范围

D0 数据库仅用于验证：

- SQLite 连接
- Migration 执行
- Rust 数据访问
- Workspace 与 Project 的最小归属关系

当前只建立：

- `workspaces`
- `projects`

不创建 Snapshot、Session、Decision、Problem、Evidence、AI、Git 或 Codex 表。

## 技术选择

- 数据库：SQLite
- 访问层：Rust
- Migration：由应用启动流程执行
- 前端访问：禁止
- Migration 源文件：`database/migrations`
- 运行时数据库：默认位于 Tauri 应用数据目录，也可通过
  `FORGEOS_DATA_DIR` 指定；D0 验证使用被 Git 忽略的 `.data/forgeos.db`

## 通用约定

- 表名和列名使用 `snake_case`
- ID 使用 `TEXT`
- ID 由 Rust 生成
- 时间使用 UTC RFC 3339 文本
- 开启外键约束
- 使用参数化查询
- Migration 只向前追加
- 开发数据库和 Migration 源文件必须分离
- SQLite 文件不提交到 Git

## `workspaces`

表示 ForgeOS 的顶层单用户工作空间。

| 字段 | 类型 | 约束 | 说明 |
|---|---|---|---|
| `id` | TEXT | PRIMARY KEY | Workspace ID |
| `name` | TEXT | NOT NULL | Workspace 名称 |
| `timezone` | TEXT | NOT NULL | 显示时区 |
| `created_at` | TEXT | NOT NULL | UTC 创建时间 |
| `updated_at` | TEXT | NOT NULL | UTC 更新时间 |

D0 不实现多用户或 Workspace 权限。

## `projects`

表示受 ForgeOS 管理的软件项目身份。

| 字段 | 类型 | 约束 | 说明 |
|---|---|---|---|
| `id` | TEXT | PRIMARY KEY | Project ID |
| `workspace_id` | TEXT | NOT NULL, FOREIGN KEY | 所属 Workspace |
| `slug` | TEXT | NOT NULL | Workspace 内稳定标识 |
| `name` | TEXT | NOT NULL | 项目名称 |
| `description` | TEXT | NULL | 简短描述 |
| `created_at` | TEXT | NOT NULL | UTC 创建时间 |
| `updated_at` | TEXT | NOT NULL | UTC 更新时间 |

约束：

- `workspace_id` 引用 `workspaces.id`
- `workspace_id + slug` 唯一
- 删除 Workspace 时不得静默丢失 Project
- D0 不实现 Project 业务页面和完整状态机

## Migration 设计

首个 Migration 负责：

1. 创建 `workspaces`
2. 创建 `projects`
3. 创建必要唯一索引
4. 创建 Project 外键

Migration 文件是数据库结构的执行事实源，本文件负责解释设计意图。

## 启动流程

```text
确定数据库路径
→ 创建父目录
→ 打开 SQLite
→ 开启 foreign_keys
→ 设置连接超时
→ 执行未应用 Migration
→ 查询数据库健康状态
→ 返回启动结果
```

D0 的 Rust 访问验证可以读取：

- 当前 Migration 状态
- Workspace 数量
- Project 数量

不应为了验证连接自动插入虚假业务数据。

## 事务规则

D0 的 Migration 在启动阶段执行。

后续业务写入必须满足：

- 相关记录在同一事务中提交
- 网络、AI、大文件复制不得发生在数据库事务内
- 失败时必须完整回滚
- UI 不管理事务

## 后续演进

以下概念已在产品设计中冻结，但不在 D0 建表：

- Project Snapshot
- Development Session
- Decision
- Problem 与 Investigation Attempt
- Evidence
- Record Revision
- Activity Event
- Context Export

它们应在进入相应开发阶段时通过新 Migration 增加，不提前创建空表。
