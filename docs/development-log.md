# ForgeOS Development Log

本文件记录 ForgeOS 的阶段目标、真实完成内容、验证结果和下一步计划。
计划不能提前写成已完成事实。

## 2026-07-10 — ForgeOS Initialization

### 当前阶段

D0：工程基础初始化

### 目标

建立可持续扩展的 React、Tauri、Rust 与 SQLite 工程基础，验证 Windows
构建和运行链路，不实现业务功能。

### 完成内容

- 建立 React 19 + TypeScript + Vite 前端基础。
- 建立 Tauri 2 桌面壳和 Rust 后端。
- 创建仅用于 D0 的 Hello ForgeOS 基础窗口。
- 建立 `bootstrap`、`ipc`、`application`、`domain`、`ports`、
  `infrastructure`、`read_models` 和 `jobs` 模块边界。
- 创建 SQLite Migration 目录和首个 Migration。
- 首个 Migration 只创建 `workspaces` 与 `projects` 业务表。
- 建立 Rust SQLite 连接、自动 Migration、Schema 健康检查和只读状态 IPC。
- 建立产品、架构、开发、数据库和 ADR 文档。
- 创建 Tauri Windows 构建所需的无品牌占位图标。
- 未实现 Dashboard、Project 页面、AI、Git/GitHub 或 Codex 集成。

### 验证结果

- Node.js：24.14.1，已通过。
- npm：11.11.0，已通过。
- Rust：rustc 1.96.1，已通过。
- Cargo：1.96.1，已通过。
- WebView2 Runtime：150.0.4078.48，已安装。
- Visual Studio Build Tools：2022 17.14.35，已安装。
- Windows SDK：10.0.26100.0，已安装。
- Tauri CLI：缺失；按照用户限制未自动安装。
- rustfmt：缺失；按照用户限制未自动安装。
- 前端依赖：安装 25 个包，审计为 0 个漏洞。
- 前端生产构建：通过，Vite 8.1.4 成功输出 `frontend/dist`。
- Rust 编译：`cargo check` 通过。
- Rust 测试：1 个测试通过，0 个失败。
- SQLite Migration：通过。
- Rust 数据库读取：通过，Workspace 和 Project 数量均为 0。
- 实际数据库：`.data/forgeos.db`。
- 实际数据库表：`_sqlx_migrations`、`workspaces`、`projects`。
- 前端开发服务：HTTP 200，验证后已停止。
- Tauri 运行：`forgeos.exe` 成功启动，窗口标题为 `ForgeOS`，进程响应正常；
  验证后已停止。
- Git：未提交、未推送。

### 已知限制

- 标准 `npm run tauri:dev` 需要用户确认后执行根目录 `npm install`，以安装
  项目本地 Tauri CLI。
- `cargo fmt` 需要用户确认后安装 `rustfmt` 组件。
- 当前图标仅用于满足 Windows 构建资源要求，不代表正式品牌设计。
- 当前数据库为空，不包含任何业务数据。

### 下一步计划

等待用户确认 D0 初始化结果。

确认前不进入 D1，不实现 Project、Current Snapshot 或其他业务功能。

## 2026-07-10 — D0.5 Engineering Freeze

### 当前阶段

D0.5：工程收尾与冻结验收

### 目标

补齐项目本地 Tauri CLI 和 Rust 格式化工具，验证标准桌面开发命令与格式化
门禁，确认忽略规则完整，为进入业务开发建立稳定基线。

### 完成内容

- 安装项目本地 `@tauri-apps/cli` 2.11.4。
- 生成根目录 `package-lock.json`，锁定项目本地 CLI 依赖。
- 安装 `rustfmt` 1.9.0-stable。
- 使用 `cargo fmt` 对现有 Rust 基础代码进行机械格式化。
- 使用标准 `npm run tauri:dev` 完成桌面运行验证。
- 复核 `.gitignore`，现有规则已覆盖依赖、构建、数据库、WAL、日志、
  临时目录和秘密配置，无需修改。
- 未新增业务功能，未创建业务页面或额外业务表。
- 未执行 Git commit 或 push。

### 验证结果

- 根目录 npm 安装：新增 2 个包，0 个漏洞。
- 项目本地 Tauri CLI：`tauri-cli 2.11.4`，通过。
- rustfmt：`rustfmt 1.9.0-stable`，通过。
- `cargo fmt --check`：通过。
- `cargo check`：通过。
- `cargo test`：1 个测试通过，0 个失败。
- `npm run tauri:dev`：通过。
- Tauri BeforeDevCommand：成功启动 Vite 8.1.4。
- Tauri DevCommand：成功编译并运行 `forgeos.exe`。
- 桌面窗口：标题为 `ForgeOS`，窗口存在且进程响应正常。
- SQLite：在 `.data/d05/forgeos.db` 成功执行 Migration。
- 运行日志确认数据库初始化完成。
- 验证结束后，ForgeOS、Cargo、Vite 和 WebView2 子进程均已关闭。
- 端口 1420 已释放。
- `.data`、根/前端 `node_modules`、`frontend/dist`、`src-tauri/target`、
  `.env` 和日志文件均通过 Git ignore 检查。

### 下一步计划

等待用户确认 D0.5 冻结结果。

确认前不提交 Git，不进入 D1，不实现 Project、Current Snapshot 或其他业务功能。

## 2026-07-10 — ForgeOS D1.1 Project Identity Kernel

### 当前阶段

D1.1：Project Identity Kernel 完成与冻结验收

### 目标

让 ForgeOS 首次具备管理真实软件项目的能力，建立单 Workspace 下稳定、隔离、
可持久化的 Project 身份与生命周期内核，同时保持在 Project Overview 边界内。

### 完成内容

- 新增前向 Migration `0002_project_identity_kernel.sql`，未修改 D0 Migration。
- 通过重建 `projects` 表补齐产品信息、技术信息、状态、生命周期、优先级和
  乐观并发版本，并保留已有 D0 数据。
- 为 Workspace 增加数据库级单例保护，为 Project 保留外键、Workspace 内
  不区分大小写的 Slug 唯一约束和列表查询索引。
- 实现 Workspace、Project Entity，以及状态、生命周期、Slug、优先级、归档、
  版本与时间单调性规则。
- 实现 Workspace 获取/创建，以及 Project 创建、列表、详情、更新和归档用例。
- 实现 Repository Ports、SQLite Repository、`ProjectKernel` Application Service
  和 Bootstrap 依赖组装。
- 新增 7 个 camelCase 类型安全 Tauri Commands；IPC 只调用 Application Service，
  不直接访问 SQLite。
- 实现单 Workspace 首次初始化、Dashboard、Projects 和 Project Overview 页面。
- 实现专用软归档入口；普通更新不能归档，归档后的 Project 在 D1.1 中只读。
- 实现版本冲突保护与重新载入入口，避免陈旧页面静默覆盖正式事实。
- 新增 `react-router-dom`、`async-trait`、`chrono`、`uuid` 和测试依赖
  `tempfile`。
- 更新数据库设计文档并新增 `ADR-002-project-identity-model.md`。
- 更新 README 的当前阶段、现有能力和数据库范围说明。
- 未实现 Current Snapshot、Development Session、Decision、Problem、Evidence、
  AI、Codex、Git、GitHub、Search 或 Timeline。
- 未执行 Git commit 或 push。

### 验证结果

- `npm run typecheck`：通过。
- `npm run build`：通过，Vite 转换 36 个模块。
- `cargo fmt --check`：通过。
- `cargo check`：通过。
- `cargo test`：6 个单元/迁移测试和 1 个集成测试通过，0 个失败。
- D0 → D1.1 Migration：旧 Project 保留，空描述与新增字段默认值回填通过。
- 集成测试：CodeLens-Pro、ForgeOS、测试项目均创建成功，列表、详情、更新、
  版本冲突、归档、归档只读和数据库重开持久化全部通过。
- 实际 Tauri 桌面验收：成功创建 1 个 Workspace 和上述 3 个 Project。
- 实际页面更新：CodeLens-Pro 更新为 `active / implementation / priority 90 / v2`。
- 实际页面归档：测试项目更新为 `archived / v2`，表单进入只读状态。
- Tauri 完整重启后：Dashboard 仍显示 3 个项目，其中 Active 2、Archived 1。
- 验收数据库：`integrity_check = ok`，`foreign_key_check` 无错误，2 个 Migration
  均成功。
- 验收数据位于被 Git 忽略的 `.data/d11-acceptance-live`，未写入生产默认数据目录。
- 验收结束后，ForgeOS、Cargo、Vite、WebView2 子进程均已关闭，端口 1420 和
  9333 已释放。

### 下一步计划

等待用户确认 D1.1 Project Identity Kernel 的实现与冻结结果。

确认前不提交 Git，不进入 Current Snapshot 或任何后续领域开发。

## 2026-07-10 — ForgeOS D1.2 Current Snapshot Kernel

### 当前阶段

D1.2：Current Snapshot Kernel 完成与冻结验收

### 目标

让 ForgeOS 能够回答“这个项目现在真实处于什么状态”，通过人工确认的当前目标、
状态总结、成果、阻塞、最多三个下一步和确认时间，支持暂停项目后快速恢复上下文。

### 完成内容

- 新增前向 Migration `0003_current_snapshot.sql`，未修改 `0001` 或 `0002`。
- 新增 `project_snapshots` 表、Project 外键、`project_id + version` 唯一约束、
  JSON 列表校验和历史查询索引。
- 实现不可原地更新的 `ProjectSnapshot` Entity；首次创建 v1，后续修改只追加
  vN+1，Current 由 Project 内最高版本确定。
- 实现 `current_goal`、`status_summary`、`achievements`、`blockers`、
  `next_steps`、确认时间、项目归属、时间单调和最多三个下一步规则。
- 实现 Snapshot Repository Port、SQLite 只追加 Repository、Application Use Cases
  和 `expectedVersion` 并发保护。
- 新增 `create_snapshot`、`get_current_snapshot`、`get_snapshot_history` 和
  `update_snapshot` 四个 camelCase 类型安全 Tauri Commands。
- Project Overview 首屏增加 Current Snapshot Card，优先展示项目当前可信状态。
- 新增人工 Snapshot 编辑与历史页面，支持创建、追加版本、倒序查看完整历史、
  冲突后重新载入和归档项目只读。
- 路由切换时清空 Project 范围状态，避免短暂显示其他项目的 Snapshot。
- 更新 README、数据库设计文档并新增
  `ADR-003-current-snapshot-versioning.md`。
- 未实现 Development Session、AI 生成/总结、Git/GitHub、Codex Context Pack、
  Timeline、Evidence 或 Task 系统。
- 未执行 Git commit 或 push。

### 验证结果

- `npm run typecheck`：通过。
- `npm run build`：通过，Vite 转换 39 个模块。
- `cargo fmt --check`：通过。
- `cargo check`：通过。
- `cargo test`：10 个领域/迁移测试、1 个 D1.2 集成测试和 1 个 D1.1 回归测试
  全部通过，0 个失败。
- D1.1 → D1.2 Migration：已有 Project 保留，Snapshot 表初始化成功；JSON、版本
  唯一和 Project 外键约束验证通过。
- 集成测试：三个项目的 Snapshot 创建、Current、历史、并发冲突、Next Steps
  上限、项目归属、归档只读和数据库重开持久化全部通过。
- 实际 Tauri 桌面验收：CodeLens-Pro、ForgeOS、测试项目均成功创建 v1 Snapshot。
- 实际版本验收：CodeLens-Pro 追加 v2；History 同时保留 v2 和 v1，Current 为 v2。
- Project Overview 首屏正确显示 CodeLens-Pro v2 的目标、状态、成果、阻塞、
  三个下一步和最后确认时间。
- Next Steps 输入第 4 项时，界面显示 `4/3` 并禁用保存。
- Tauri 完整重启后：三个项目的 Current Snapshot 与 CodeLens-Pro 两个历史版本
  均保持不变。
- 验收数据库：`integrity_check = ok`、`foreign_key_check = 0`、3 个 Migration、
  1 个 Workspace、3 个 Project、4 条 Snapshot 记录。
- 验收数据位于被 Git 忽略的 `.data/d12-acceptance-live`，未写入生产默认数据目录。
- 验收结束后，ForgeOS、Cargo、Vite、WebView2 子进程均已关闭，端口 1420 和
  9334 已释放。

### 下一步计划

等待用户确认 D1.2 Current Snapshot Kernel 的实现与冻结结果。

确认前不提交 Git，不进入 Development Session 或任何后续领域开发。
