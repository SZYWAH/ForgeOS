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

## 2026-07-11 — ForgeOS D1.3 Development Session Kernel

### 当前阶段

D1.3：Development Session Kernel 完成与冻结验收

### 目标

让 ForgeOS 能够回答“一次研发活动发生了什么”，以 Project 为归属记录开始目标、
过程 Notes、结果、验证和下一次接手信息，并使进行中会话与只读历史保持明确边界。

### 完成内容

- 新增前向 Migration `0004_development_session.sql`，未修改 `0001`、`0002` 或 `0003`。
- 新增 `development_sessions` 表、Project 外键、状态与生命周期约束、历史索引和
  每 Project 唯一 Open Session 的部分唯一索引。
- 实现 `DevelopmentSession` 和 `SessionStatus`，并将状态机限定为
  `open → completed | abandoned`。
- 实现 Goal 必填、Markdown Notes、版本递增、时间单调、Project 归属和终态不可变规则。
- Complete 时必须人工确认 Outcome、Validation 和 Handoff；Abandon 时不伪造结果与验证，
  但必须保留 Handoff。
- 实现 `start_session`、`get_current_session`、`list_sessions`、`update_session`、
  `complete_session` 和 `abandon_session` 六个 Application Use Case 与类型化 IPC Command。
- 所有 Session mutation 都使用 `expectedVersion` 保护；陈旧页面不能覆盖较新内容或终态事实。
- 已归档 Project 不能开始新 Session 或继续编辑 Goal/Notes；归档前遗留的 Open Session
  仅允许 Complete 或 Abandon 收尾，避免永久悬空。
- Project Overview 增加 Current Development Session 区域，展示 Goal、开始时间、Open 状态和版本。
- 新增 Session 页面，支持 Start、Update、Complete、Abandon、Current Session 和只读 History。
- 路由切换和所有异步写操作均使用 generation guard，防止迟到响应污染其他 Project 页面。
- 更新 README 和数据库设计文档，并新增 `ADR-004-development-session-lifecycle.md`。
- 未实现 AI 总结、自动 Session、Git/GitHub、Commit 关联、Codex Context Pack、Evidence、
  Timeline 或时间统计。
- 未执行 Git commit 或 push。

### 验证结果

- `npm run typecheck`：通过。
- `npm run build`：通过，Vite 转换 42 个模块。
- `cargo fmt --check`：通过。
- `cargo check`：通过。
- `cargo test`：15 个单元/迁移测试和 D1.1、D1.2、D1.3 各 1 个集成测试全部通过，0 失败。
- D1.3 集成测试覆盖唯一/并发 Open Session、Project 归属、版本冲突、终态不可变、
  归档写保护、收尾例外和数据库重开持久化。
- 实际 Tauri 桌面验收：CodeLens-Pro、ForgeOS 和测试项目均成功创建 Session。
- CodeLens-Pro：Open v1 → 更新 v2 → Completed v3，Outcome、Validation、Handoff 在 History 中完整保留。
- ForgeOS：Open v1 → Abandoned v2，不伪造 Outcome/Validation，Handoff 完整保留。
- 测试项目：Open v1 → 更新 v2，保持为唯一 Current Session。
- Tauri 完整停止并使用同一数据库重启后，上述三种状态、版本、Notes 和收尾字段均保持不变。
- 验收数据库：`integrity_check = ok`、`foreign_key_check = 0`、4 个 Migration、
  1 个 Workspace、3 个 Project、3 条 Session 记录；仅测试项目保留 1 条 Open Session。
- 验收数据位于被 Git 忽略的 `.data/d13-acceptance-live`，未写入生产默认数据目录。
- 额外 Clippy 检查因本机未安装 `cargo-clippy` 而未执行；未自动安装，且它不属于本阶段规定验收项。

### 下一步计划

等待用户确认 D1.3 Development Session Kernel 的实现与冻结结果。

确认前不提交 Git，不进入 Context Pack、AI 或任何后续领域开发。

## 2026-07-11 — ForgeOS D1.4 Evidence Kernel

### 当前阶段

D1.4：Evidence Kernel 完成

### 目标

让 ForgeOS 能够回答“为什么这个状态、成果或研发判断值得相信”，以 Project 为归属
人工保存证据来源、验证状态，并显式关联 Project、Snapshot 与 Development Session。

### 完成内容

- 新增前向 Migration `0005_evidence_kernel.sql`，未修改 `0001` 至 `0004`。
- 新增 `evidences` 与 `evidence_links`，包含 Project/Evidence 外键、枚举 CHECK、
  乐观并发版本、最近证据/验证状态/关联查询索引和重复关联唯一约束。
- 对多态 EvidenceLink 使用 Application 目标归属校验和 SQLite trigger 双层保护，
  防止关联不存在或其他 Project 的 Snapshot、Session、Project。
- 实现 Evidence、EvidenceDetails、EvidenceLink、EvidenceType、EvidenceSourceType、
  VerificationStatus、EvidenceTargetType 与 EvidenceRelationType 领域模型。
- 新 Evidence 固定为 unverified；内容更新自动重置 unverified；verify 与 invalidate
  使用专用领域操作并递增 version。
- 实现 create、list、get、update、verify、invalidate、link 七个 Application Use Case、
  Repository Port、SQLite Repository 和 camelCase 类型化 Tauri IPC Command。
- 所有 Evidence mutation 使用 `expectedVersion`；已归档 Project 的 Evidence 只读。
- Project Overview 增加 Evidence Summary，显示数量、Verified 数量、待复核数量和最近证据。
- 新增 Evidence 管理页面，支持创建、编辑来源、人工验证/失效，以及关联 Project、
  Snapshot 与 Development Session。
- 本地附件以路径引用保存，外部来源以 URL 保存；不复制文件、不抓取 URL、不自动验证。
- 新增 D1.4 领域测试与完整集成测试，并更新 README、数据库设计和 ADR-005。
- 未实现 Git/GitHub 自动采集、Commit 同步、AI 分析、Context Pack、MCP、Codex、
  Timeline、自动验证或知识图谱。
- 未执行 Git commit 或 push。

### 验证结果

- `npm run typecheck`：通过。
- `npm run build`：通过，Vite 转换 46 个模块。
- `cargo fmt --check`：通过。
- `cargo check`：通过。
- `cargo test`：17 个单元/迁移测试及 D1.1、D1.2、D1.3、D1.4 各 1 个集成测试
  全部通过，0 个失败。
- D1.4 集成测试为 CodeLens-Pro、ForgeOS、测试项目各创建 2 条 Evidence，并覆盖
  Project/Snapshot/Session 关联、verified、内容更新重置、invalid、版本冲突和数据库重开。
- 跨 Project 关联在 Application 层被拒绝；绕过 Application 的直接 SQL 由 trigger 拒绝。
- 孤立 Evidence 由 Project 外键拒绝。
- `PRAGMA integrity_check = ok`，`pragma_foreign_key_check = 0`。
- 前端生产构建成功，未增加新依赖。

### 下一步计划

等待用户确认 D1.4 Evidence Kernel 的实现与验收结果。

确认前不提交 Git，不进入 Context Pack、AI 或任何后续领域开发。

## 2026-07-11 — AI Provider Foundation

### 当前阶段

D1.4 扩展：AI Provider Foundation 完成

### 目标

允许 ForgeOS 管理任意 OpenAI-compatible Provider，并在用户显式操作时验证调用，
同时确保 AI 不自动运行、不生成正式事实、不修改数据库业务记录。

### 完成内容

- 新增 `0006_ai_provider_foundation.sql` 和 `ai_providers` 非敏感配置表。
- 实现 AiProvider Domain、Repository Port、SQLite Repository、独立 AiProviderKernel 和类型化 IPC。
- 支持自定义名称、Base URL、Model、Provider 启停与乐观并发版本。
- API Key 通过 ApiKeyStore Port 保存到 Windows Credential Manager，SQLite 和 DTO 不含密钥。
- 实现 OpenAI-compatible Chat Completions HTTP Adapter，支持 API 根路径或完整请求地址。
- 支持 OpenAI、DeepSeek、Azure OpenAI、Ollama、LM Studio 和其他兼容接口的自定义配置。
- 新 Provider 默认 Disabled；Disabled Provider 在 HTTP 请求前被拒绝。
- 新增 AI Providers 页面、预设填充、密钥替换/清除、启停与手动 Invocation。
- 手动 Prompt 与 Response 只存在于页面内存，不保存调用历史，不写研发事实。
- 新增 AI Provider 单元/集成测试和 ADR-006。
- 未实现自动调用、AI 总结、事实生成、数据库写入、Agent、MCP 或 Context Pack。
- 未执行 Git commit 或 push。

### 验证结果

- `npm run typecheck`：通过。
- `npm run build`：通过，Vite 转换 48 个模块。
- `cargo check`：通过。
- `cargo test`：22 个单元/迁移测试与 5 个集成测试通过，0 个失败。
- Provider 集成测试使用内存密钥库和 Fake Client，不访问网络或真实 Windows 凭据。
- 验证 Disabled 不调用 Client、显式调用只执行一次、API Key 不存在于 SQLite schema。
- 验证 Provider 配置、启停、版本冲突、密钥状态和数据库重开持久化。
- `integrity_check = ok`，`foreign_key_check = 0`。

### 下一步计划

等待用户确认 Evidence Kernel 与 AI Provider Foundation 的联合验收结果。

确认前不提交 Git，不进入自动 AI、Context Pack、Agent 或 MCP 开发。

## 2026-07-11 — ForgeOS D1.5 Context Pack Kernel + Manual AI Invocation

### 当前阶段

D1.5：Context Pack Kernel + Manual AI Invocation 完成

### 目标

把 ForgeOS 中已确认的单 Project 研发事实确定性转换为结构化 Markdown，并允许用户主动选择已启用的 AI Provider 进行一次性分析，同时保持 AI 与正式事实层隔离。

### 完成内容

- 新增向前 Migration `0007_context_pack.sql`，未修改 `0001` 至 `0006`。
- 新增 `context_exports`，保存 Project 归属、五种模板、Snapshot 来源版本、Session 来源 ID、Markdown、SHA-256、发送状态与乐观版本。
- 实现 ContextExport Domain 的来源校验、内容哈希、Project 归属与 `generated → sent_to_ai` 状态规则。
- 实现无状态 MarkdownContextRenderer，固定 Project、Snapshot、Recent Session、Evidence 章节顺序和 Evidence 稳定排序。
- 实现 ContextPackKernel 的 generate、list、get 与手动 AI invocation 用例；React 和 IPC 不直接读取 SQLite。
- AI 调用复用既有 AiProviderKernel；Disabled Provider 在网络前拒绝，Enabled Provider 仅在用户点击时调用。
- AI 成功后只更新 ContextExport 发送状态；Prompt 与 Response 只存在于当前页面状态，不进入数据库。
- 新增 Context Pack 页面、Project Overview 入口、五种模板选择、导出历史、Markdown Preview、Copy Markdown 和单次 AI Analysis。
- 新增 ADR-007，并更新 README、总体架构和数据库设计。
- 未实现 Agent、MCP、Tool Calling、后台 AI、自动事实写入、多轮对话或 Codex 自动任务协同。
- 未执行 Git commit 或 push。

### 验证结果

- `npm run typecheck`：通过。
- `npm run build`：通过。
- `cargo fmt --check`：通过。
- `cargo check`：通过。
- `cargo test`：D1.5 单元、迁移、集成测试及既有回归测试全部通过，0 个失败。
- 集成测试为 CodeLens-Pro、ForgeOS、测试项目建立各自事实，并验证相同输入 Markdown/hash 一致、三个 Project 严格隔离。
- 验证来源 Snapshot 版本与最近 Session ID 正确保存，导出重开数据库后仍存在。
- 验证 Disabled Provider 不调用 Client，Enabled Provider 只在显式操作时调用。
- 验证 AI 调用前后 Project、Snapshot、Session、Evidence 数量和版本签名完全一致。
- 验证 API Key、User Prompt 与 AI Response 不进入 SQLite；`integrity_check = ok`、`foreign_key_check = 0`。

### 下一步计划

等待用户确认 D1.5 Context Pack Kernel + Manual AI Invocation 的实现与验收结果。

确认前不提交 Git，不进入 Agent、MCP、自动 AI 任务或任何后续开发。

## 2026-07-11 — ForgeOS D1.6 AI Candidate Foundation

### 当前阶段

D1.6：AI Candidate Foundation 完成

### 目标

在临时 AI Response 与正式研发事实之间建立独立 Candidate 治理层，使 AI 建议可以由用户明确保存、审核和保留，同时不获得任何正式领域写权限。

### 完成内容

- 新增向前 Migration `0008_ai_candidate.sql`，未修改 `0001` 至 `0007`。
- 新增 `ai_candidates`，保存 Project、来源 ContextExport、五种候选类型、标题、Markdown、审核状态、版本和时间。
- 使用 Project 外键与 `(source_context_export_id, project_id)` 复合外键双重保护归属，防止跨 Project Candidate。
- 实现 AICandidate Domain，并将创建状态固定为 Pending；生命周期限定为 `pending → accepted | rejected | archived`。
- 实现 Candidate Repository Port、SQLite Repository、AiCandidateKernel、六个 Application Use Case 和类型化 IPC Commands。
- 所有 Review 使用 expectedVersion；Repository UPDATE 同时要求当前状态仍为 Pending。
- AiCandidateKernel 不拥有 Snapshot、Session、Evidence 写入 Port；Accept 只记录认可，不修改正式事实。
- Manual AI 页面新增“仅查看或保存为 Candidate”的第二次显式操作，AI 调用本身仍不自动持久化 Response。
- 新增 AI Candidates Review 页面、Project Overview 入口、状态筛选、来源展示与 Accept/Reject/Archive 操作。
- 新增 ADR-008，并更新 README、总体架构和数据库设计。
- 未实现自动接受、Candidate 自动应用、Agent、MCP、Codex 执行、后台 AI、自动 Snapshot/Session/Evidence 写入。
- 未执行 Git commit 或 push。

### 验证结果

- `npm run typecheck`：通过。
- `npm run build`：通过。
- `cargo fmt --check`：通过。
- `cargo check`：通过。
- `cargo test`：D1.6 单元、Migration、集成测试与既有 Kernel 回归测试全部通过，0 个失败。
- 集成测试使用 CodeLens-Pro、ForgeOS、测试项目，通过真实 Context Pack 与 Fake AI Provider Response 各创建一条 Pending Candidate。
- 验证 Accept、Reject、Archive 均保留历史，终态 version 为 2，陈旧 expectedVersion 被拒绝。
- 验证 Application 跨 Project 来源被拒绝，绕过 Application 的直接 SQL 由复合外键拒绝。
- 验证创建 Pending 和 Accept 前后 Project、Snapshot、Session、Evidence 的数量与版本签名完全一致。
- 验证数据库重启后 Accepted、Rejected、Archived 状态和内容保持。
- `integrity_check = ok`，`foreign_key_check = 0`。

### 下一步计划

等待用户确认 D1.6 AI Candidate Foundation 的实现与验收结果。

确认前不提交 Git，不进入 Candidate 自动应用、Agent、MCP、Codex 自动任务或后续开发。

## 2026-07-12 — ForgeOS D2 Git Evidence Integration

### 当前阶段

D2：Git Evidence Integration 完成

### 目标

让 ForgeOS 以只读观察者身份读取本地 Git Repository、WorkingCopy 与 Commit 信息，并由用户明确把外部 Git 事实转换为 Project Evidence。

### 完成内容

- 新增向前 Migration `0009_git_evidence.sql`，未修改 `0001` 至 `0008`。
- 新增 Repository、WorkingCopy 与 RepositoryEvidenceLink 数据结构。
- 在 0009 中安全重建 Evidence/EvidenceLink，保留既有数据、索引和 trigger，并扩展 `git_commit`、`git_status` 类型。
- 实现 Repository、WorkingCopy、DiffSummary Domain 与 Project 归属、绝对路径、SHA、版本和归档规则。
- 实现 GitAdapter Port 与 GitCliAdapter，只使用固定只读命令、机器可读输出、参数数组、无可选锁、无交互认证和超时。
- 支持 Repository 检查、根路径、Branch、HEAD、Dirty、Diff Summary、Origin、默认分支和最近十条 Commit。
- 实现 RepositoryStore、事务化注册/刷新、WorkingCopy 追加历史、Git Evidence 来源关联和跨 Project 复合外键。
- 实现 inspect、register、list、refresh、create commit/status evidence、list repository evidence Application Use Cases 与类型化 IPC。
- 通用 Evidence Create/Update 禁止创建或改写 Git Evidence；Verify/Invalidate 仍保留人工判断能力。
- Project Overview 新增 Repository Card；新增 Repository 页面、显式 Inspect/Register/Refresh、Commit/Status Evidence 和 Git Evidence 列表。
- 新增 ADR-009，并更新 README、总体架构与数据库设计。
- 未实现任何 Git 写命令、GitHub API、GitHub 同步、Codex 执行、MCP、Agent 或 AI Git 分析。
- 未执行 Git commit 或 push。

### 验证结果

- `npm run typecheck`：通过。
- `npm run build`：通过。
- `cargo fmt --check`：通过。
- `cargo check`：通过。
- `cargo test`：D2 单元、Migration、集成测试与 D1.1-D1.6 回归测试全部通过，0 个失败。
- GitCliAdapter 对当前真实 ForgeOS 工作区成功读取 `master`、HEAD、dirty 状态和最近 Commit；没有执行 Git 写命令。
- D2 集成测试以 CodeLens-Pro、ForgeOS、测试项目验证 Repository 注册、路径唯一、Branch/HEAD/Dirty、WorkingCopy 追加、Commit/Status Evidence、版本冲突和数据库重开。
- 验证 Application 跨 Project 访问被拒绝，绕过 Application 的 RepositoryEvidenceLink 由复合外键拒绝。
- 验证 Git Evidence 正确归属于 Project 和 Repository，Reject/AI/正式事实层均未参与。
- `integrity_check = ok`，`foreign_key_check = 0`。
- 本机 `D:\wenjian` 范围未找到 CodeLens-Pro 实际目录，因此 CodeLens-Pro 使用隔离测试适配器完成领域验收；未伪报真实 CLI 验收。

### 下一步计划

等待用户确认 D2 Git Evidence Integration 的实现与验收结果。

确认前不提交 Git，不进入 GitHub、MCP、Agent、Codex 自动任务或后续开发。

## 2026-07-12 — ForgeOS D3 MCP Read-only Server

### 当前阶段

D3：ForgeOS MCP Read-only Server 完成

### 目标

建立独立的本地 STDIO MCP Server，让 Codex 等外部 AI 客户端读取 ForgeOS 已确认的研发语义上下文，同时保持 Project 隔离、数据库只读和敏感信息保护。

### 完成内容

- 新增独立 `forgeos-mcp` Rust executable，复用 `forgeos_lib` 的 Application、Ports、Infrastructure 与 Context Renderer，不复制领域逻辑。
- 新增 Core `McpQueryKernel` 和只读查询 DTO，仅暴露 Project Identity、Current Snapshot、Recent Sessions、Evidence 与内存 Context Pack 查询。
- 新增 SQLite `connect_read_only`，不创建数据库、不运行 Migration、不改变 Journal Mode，底层写操作由 SQLite 拒绝。
- 使用官方 Rust MCP SDK `rmcp` 和 STDIO Transport，实现 `tools/list` 与 `tools/call` 协议链路，不监听 HTTP/TCP。
- 固定五个只读 Tools：`get_project_identity`、`get_current_snapshot`、`get_recent_sessions`、`list_evidence`、`build_context_pack`。
- 每个 Tool 显式要求 `project_id`，Application 先在当前 Workspace 中确认 Project，再读取其子域数据。
- MCP 输出省略 Evidence source URI、Repository 本地路径和全部 AI Provider/Secret 信息。
- `build_context_pack` 使用既有确定性 Renderer，只在内存中生成 Markdown 和 SHA-256，不写入 `context_exports`。
- 新增协议级测试，覆盖 MCP 初始化、工具列表、CodeLens-Pro/ForgeOS 隔离、Snapshot/Session/Evidence 查询、Git Evidence、写保护与敏感信息不返回。
- 新增 ADR-010，并更新 README 与总体架构。
- 未修改 Codex 配置、`.codex` 文件或任何 Codex 数据；未实现 MCP 写入、HTTP MCP、Agent、Tasks 或 Codex 自动任务。
- 未执行 Git commit 或 push。

### 验证结果

- `npm run mcp:test`：MCP Tool Surface 单元测试和 STDIO 端到端协议测试全部通过。
- MCP 客户端成功列出且仅列出五个只读工具；不存在 create/update/delete/write/accept 工具。
- CodeLens-Pro 查询不包含 ForgeOS 的 Project、Snapshot、Session 或 Evidence 内容。
- `list_evidence` 同时返回 manual Evidence 和 Git Evidence，但不返回 `source_uri` 或私有绝对路径。
- `build_context_pack` 返回稳定 Markdown 与 Hash，调用前后 `context_exports` 数量不变。
- 只读连接上的 `UPDATE projects` 被 SQLite 拒绝。
- AI Provider 配置、API Key、Credential 和 Token 未进入任何 MCP Result DTO。

### 下一步计划

等待用户确认 D3 MCP Read-only Server 的实现与验收结果。

确认前不提交 Git，不修改 Codex 配置，不进入 MCP 写入、Agent 或 Codex 自动任务开发。

## 2026-07-12 — ForgeOS D4 Codex Task Proposal Foundation

### 当前阶段

D4：Codex Task Proposal Foundation 完成

### 目标

建立执行前任务提案层，用固定 ContextExport、范围约束、验收标准和人工审批描述未来 Codex 编码请求，但不执行 Codex 或修改代码。

### 完成内容

- 新增 `CodexTaskProposal` Domain、Migration `0010`、Proposal 主表和只追加状态历史表。
- 实现 `draft → proposed → approved | rejected`、`approved → archived` 状态机。
- 复合外键保证 Proposal 与 ContextExport 属于同一 Project。
- 实现创建、列表、详情、提交、批准、拒绝和归档 Use Cases、Repository 与类型化 IPC。
- 状态变化使用 expectedVersion，并在事务中追加历史。
- 新增 Codex Tasks 页面与 Project Overview 入口，明确显示 “Proposal only” 和“不会执行代码”。
- 未实现 Codex 执行、Agent、MCP 写入、Sandbox、分支、Commit、Push 或 Merge。
- 未执行 Git commit。

### 验证结果

- 集成测试覆盖创建、Context 绑定、跨 Project 拒绝、版本冲突、Approve/Reject、历史和重开持久化。
- Approve 前后 Project、Snapshot、Session 与 Evidence 事实保持不变。

### 下一步计划

等待用户确认 D4。确认前不进入 Codex 执行、Agent 或 MCP 写入。

## 2026-07-12 — ForgeOS D4.5 GitHub Connector Foundation

### 当前阶段

D4.5：GitHub Connector / Project Import Foundation 完成

### 目标

以只读方式接管已有 GitHub 软件项目，通过 Import Draft 与人工审核把托管平台事实安全转换为 ForgeOS Project 和 Evidence。

### 完成内容

- 新增 Migration `0011_github_readonly_import.sql`，未修改 `0001` 至 `0010`。
- 新增 GitHubConnector、GitHubImportDraft、Repository/Commit/Release Fact Domain 与人工审核状态机。
- 新增只读 GitHub REST Adapter，读取 Repository、Languages、Topics、README、最近 Commit 与最新 Release。
- GitHub Token 使用独立 Windows Credential Manager 安全存储；SQLite、日志和 IPC 返回均不包含 Token。
- 新增 Preview、List、Review、Reject、Import 与 Project 来源查询 Application Use Cases 和类型化 IPC。
- Import 使用单事务创建 Project、更新 Draft 并创建 GitHub Repository/Commit/Release Evidence。
- 新增 GitHub Import 页面、Connector 安全配置、Import Preview、Draft Review 与 Project Overview GitHub Card。
- Reject 不创建 Project；API 失败不保存 Draft；无后台同步、Webhook 或 GitHub 写操作。
- 新增 ADR-012。

### 验证结果

- `npm run frontend:typecheck`、`npm run frontend:build`、`cargo fmt --check`、`cargo check` 全部通过。
- `cargo test` 通过 39 个单元/Migration 测试与 11 个集成测试，0 失败；网络测试默认 Ignore。
- 公开仓库 `octocat/Hello-World` 的真实 GitHub REST 只读烟雾测试单独执行通过；匿名出口触发限流后使用进程级 GitHub CLI Token 完成验证，Token 未持久化。
- 集成测试覆盖 Token 隔离、Draft 审核、Reject、原子 Import、跨 Project 隔离、API 失败无脏数据和重启持久化。
- Migration 保留既有 Manual/Git Evidence、EvidenceLink 与 RepositoryEvidenceLink；`integrity_check = ok`、`foreign_key_check = 0`。
- D3 MCP 回归测试通过，工具面仍严格保持五个只读 Tool。

### 下一步计划

等待用户确认 D4.5。确认前不提交 Git，不开发 GitHub 写入、Webhook、自动同步、Codex 执行或 Agent。

## 2026-07-13 — ForgeOS D5 Codex Execution Foundation

### 当前阶段

D5：Codex Execution Foundation 完成

### 目标

在不执行 Codex、不修改代码的前提下，把 Approved Task Proposal 转换为可审计的 Execution Request、Execution Record 与 Execution Result。

### 完成内容

- 新增 Migration `0012_codex_execution_foundation.sql`，未修改 `0001` 至 `0011`。
- 新增 `CodexExecutionRecord` Domain，支持 `pending/running/success/failed/cancelled` 状态机、时间规则和乐观版本。
- 新增只追加的 `codex_execution_status_history`，状态更新与 History 在同一事务提交。
- 只有同一 Project 的 Approved Proposal 可以创建 Execution；跨 Project 与未批准 Proposal 均被拒绝。
- 新增 `CodexRunner` Port 与 `RecordOnlyCodexRunner` Infrastructure Adapter。当前只接收请求元数据，不启动任何进程。
- 新增创建、列表、详情、开始、成功、失败和取消 Use Cases 与类型化 IPC。
- 新增 Codex 执行记录页面，支持关联 Proposal、查看 Execution History、结果与验证信息。
- 前端所有状态操作明确为记录模式，不暗示 Codex 或验证命令已经真实执行。
- 新增 ADR-013，记录 Proposal 与 Execution 分离以及未来 Runner 接入边界。
- 未实现 `codex exec`、文件修改、Agent、MCP 写入、自动 Commit/Push/Merge 或后台任务。

### 验证结果

- `npm run build` 通过，包含 TypeScript 构建与 Vite 生产打包。
- `cargo fmt --check` 与 `cargo check` 通过。
- `cargo test` 执行 43 个单元/Migration 测试与 12 个集成测试，55 个通过、0 个失败；GitHub 真实网络测试按设计保持 Ignore。
- D5 专用集成测试覆盖 Approved Proposal 约束、跨 Project 拒绝、状态流转、版本冲突、失败/取消历史、事实隔离和重启持久化。
- Migration 升级测试确认 Approved Proposal 保留，复合外键拒绝跨 Project Execution。
- `integrity_check = ok`，`foreign_key_check = 0`。

### 下一步计划

等待用户确认 D5。确认前不提交 Git，不进入真实 Codex Runner、Agent、MCP 写入或 Git 自动化。

## 2026-07-13 — ForgeOS D5 MCP Intelligence Layer

### 当前阶段

D5：MCP Intelligence Layer 完成，等待用户验收

### 方向调整

停止继续开发 Codex Execution。已存在的 Execution Foundation 仅保留为记录层，不接入真实 Runner。当前方向改为增强 ForgeOS MCP 的只读研发知识查询，让 Codex 负责代码工作，ForgeOS 负责提供经过 Project 隔离的可靠上下文。

### 本阶段范围

- 在既有 D3 只读 MCP Tool 兼容面上新增 `get_project_overview`、`get_current_state`、`get_development_history`、`get_recent_git_activity`、`get_project_evidence_summary` 和 `get_recommended_next_steps`。
- 所有查询继续经过 `forgeos-mcp → McpQueryKernel → Application Query → Repository Port → SQLite read-only connection`，MCP 层不直接执行 SQL。
- Project Overview 返回 Project 身份、技术栈、生命周期和状态；Current State 返回当前 Snapshot 及其版本。
- Development History 返回固定数量的最近 Session；Git Activity 返回经过安全投影的 Commit 与 Working Copy 状态摘要。
- Evidence Summary 返回 Evidence 类型、验证状态与关系摘要，不返回 `source_uri` 或本地绝对路径。
- Recommended Next Steps 只根据 Current Snapshot、Open Session、Pending Candidate 与尚未确认的 Evidence 执行固定规则；不调用 AI，不把建议写回正式领域。
- `build_context_pack` 增强为稳定组织 Project Overview、Snapshot、Session、Evidence Summary 与 Git Activity，并继续只在内存中生成 Markdown 与哈希。
- 新增 ADR-014，记录 Query Intelligence、确定性建议和安全 DTO 的架构决策。

### 安全边界

- MCP 保持只读，不创建、更新或删除 Project、Snapshot、Session、Evidence、Candidate 或 ContextExport。
- 不返回 API Key、Credential、Token、Provider Secret、Repository 私有绝对路径或 Evidence 来源路径。
- 不调用 AI，不运行 Agent，不执行 Codex，不启动 Shell，不执行 Git 读取或写入命令。
- 不新增 MCP 写 Tool、HTTP Server、后台任务、自动代码修改、自动 Commit、Push 或 Merge。

### 验证结果

- `npm run build` 通过：TypeScript 编译和 Vite 生产构建成功。
- `src-tauri` 的 `cargo fmt --check`、`cargo check`、`cargo test` 通过：58 个测试通过，0 失败，1 个真实 GitHub 网络测试按设计保持 Ignore。
- `forgeos-mcp` 的 `cargo fmt --check`、`cargo check`、`cargo test` 通过：Tool Surface 单元测试与 STDIO Intelligence 端到端测试共 2 个测试通过，0 失败。
- MCP 测试确认 11 个 Tool 均为只读工具，并覆盖新旧工具兼容、CodeLens-Pro/ForgeOS Project 隔离、Evidence 关系、Git Activity 安全投影和确定性推荐。
- 连续两次生成相同 Project/Template 的 Context Pack，返回内容与哈希保持一致；调用前后 Project、Snapshot、Session、Evidence、Candidate 和 ContextExport 指纹不变。
- 敏感信息测试确认 API Key/Provider Token、作者邮箱、Evidence `source_uri`、Repository 私有绝对路径，以及嵌入 Git Evidence 标题、Message 和描述的路径均不进入 MCP 输出；普通 HTTP/HTTPS URL 保持可用。
- SQLite read-only 连接拒绝 `UPDATE`；测试数据库 `integrity_check = ok`、`foreign_key_check = 0`。

### 下一步计划

等待用户确认 D5 MCP Intelligence Layer 验收结果。确认前不提交 Git，不进入 Codex Execution、Agent、MCP 写入、Shell/Git 自动化或其他后续开发。
# 2026-07-13 — ForgeOS D6 Codex Activity Intake + Project Pulse

## 当前阶段

D6：自动观察与人工确认闭环实现。

## 目标

把“人工维护研发记录”调整为“ForgeOS 观察 Codex 与 Git，用户审核后确认正式事实”。

## 完成内容

- 新增 0013/0014 前向 Migration、Codex Connector、Project Activity 设置、SyncRun、Cursor、Observation、Pulse 与来源链。
- 新增官方 Codex App Server STDIO 只读 Adapter，固定四个方法白名单，并按 Repository 精确隔离任务。
- Git WorkingCopy 增加 upstream/ahead/behind 观察，不执行 fetch 或任何 Git 写操作。
- 新增确定性规则 Pulse、可选 AI 固定 JSON 增强、一次修复和失败回退。
- 新增 Project Pulse 审核、编辑、忽略和 Snapshot/Session 单事务确认。
- 新增打开项目自动同步、手动同步、30 分钟进程内调度、SyncRun 恢复和 Tauri 进度事件。
- Project Overview 调整为 Pulse 主入口，Project Identity 移入设置页，Dashboard 改为关注队列。
- Context Pack 新增明确标识的 Observed Activity 章节；MCP 新增只读 `get_project_pulse`。
- 新增 ADR-015 与 ADR-016。

## 安全结果

- 不保存完整 Codex 对话、reasoning、完整命令输出、环境变量、凭据、完整 Diff 或 App Server 原始 JSONL。
- AI 与 Pulse 均不能直接修改正式事实。
- MCP 不触发同步，不返回 Token、绝对路径或原始对话，并继续使用 SQLite read-only 连接。

## 验证结果

- `npm run typecheck` 与 `npm run build` 通过；Vite 生产构建完成 66 个模块转换。
- `cargo fmt --check` 与 `cargo check` 通过。
- 主 Rust 测试共发现 75 个测试：73 个通过，0 个失败，2 个按设计忽略；其中真实 GitHub 网络测试默认忽略。
- Windows 实机 `Codex App Server` 只读 Smoke Test 单独执行通过，验证本机可发现并调用官方 App Server。
- `forgeos-mcp` 的格式检查、编译检查与测试通过：2 个测试通过，0 个失败。
- D6 集成测试覆盖 Observation 幂等、Project 隔离、Pulse 状态与来源链、Git Ahead/Behind、确认事务、SQLite `integrity_check = ok` 与 `foreign_key_check = 0`。
- 前端仅依赖 Tauri IPC；普通浏览器开发服务器无法脱离 Tauri WebView 完整加载真实数据页面，生产构建与类型检查已作为本轮前端自动验证结果。

## 下一步计划

等待 D6 验收。验收前不进入 Codex 执行、Agent、MCP 写入或 Git 自动化。

# 2026-07-13 — ForgeOS D6.1 Project Pulse Quality Patch

## 当前阶段

D6.1：Pulse 语义质量修正与旧数据安全升级。

## 完成内容

- 新增前向 Migration `0015_project_pulse_quality.sql`；旧 Pulse 自动标记为 `legacy-v1`，原来源链、确认关系、Snapshot 和 Session 均不覆盖。
- 新增运行时 `ActivityEpisodeBuilder` 与 `PulseWorkingSetSelector`，Pulse 只采用当前活动和近 14 天最多两个完成活动。
- 新增 `activity_type`、`analysis_revision`、来源时间窗、采用/可用 Observation 计数和只读质量警告。
- rules-v2 使用字段专用清洗、Unicode 安全截断、精简成果/验证/阻塞/下一步规则，并隔离历史 Handoff、Evidence/Candidate 与普通 Git Dirty。
- AI 输入限制为 Working Set 中最多 20 条 Observation，固定 JSON 增加 Activity Type，非法结果修复一次后回退 rules-v2。
- 审核页显示分析范围和旧版警告；Snapshot 默认选中，只有 implementation/debugging 默认应用 Session，完成 Session 始终默认关闭。
- MCP `get_project_pulse` 增加活动类型、分析修订和来源计数；新 Context Pack 的未确认活动章节增加 Activity Type。

## 安全边界

- 不重写 Observation，不新增 Episode 表，不修改或删除旧正式事实。
- 不执行 Codex，不新增 MCP 写工具，不执行 Git 写操作，不自动创建 Evidence。
- 存在提示词包装或超长字段时，后端和前端共同阻止确认，要求用户先编辑草稿。

## 验证结果

- `npm run typecheck` 与 `npm run build` 通过；Vite 生产构建完成 66 个模块转换。
- `cargo fmt --check`、`cargo check` 与主程序全量 `cargo test --jobs 2` 通过：67 项单元测试通过、1 项需外部环境的常规测试忽略，全部集成测试通过。
- `forgeos-mcp` 的格式、编译、单元测试与 STDIO 只读集成测试通过，工具面保持精确的 12 个只读工具。
- Windows Codex App Server 只读 Smoke Test 实机通过。
- SQLite `integrity_check` 返回 `ok`，`foreign_key_check` 无记录。
- Daymark 实机重新同步生成 `rules-v2` 待审核 Pulse：活动类型为 `support_question`，从 250 条可用 Observation 中只选择 3 条；旧 `legacy-v1` Pulse 的 249 条来源、Snapshot v1 与 Open Session 均保持不变。
- 审核页实机确认：支持咨询默认只勾选创建新版 Snapshot，更新 Session 与完成 Session 均默认关闭；本阶段未代替用户确认，因此 Snapshot v2 尚未创建。

## 下一步计划

等待用户审核 Daymark 的新 Pulse；确认时可只创建 Snapshot v2，并由用户明确决定旧 Open Session 的保留、更新或放弃。

# 2026-07-13 — ForgeOS D6.2 页面内容治理与工作台稳定性调整

## 当前阶段

D6.2：统一长内容预算、详情展开与桌面工作台滚动边界。

## 完成内容

- 新增 `CollapsibleText`、`BoundedList`、`RecordDisclosure` 与 `CompactIdentifier`，统一正文折叠、列表预算、历史单条展开和路径/Hash 紧凑展示。
- 新增 `formatFactPreview`，旧 Snapshot 概览优先展示 `My request for Codex` 的实际目标；附件路径和 Markdown 包装只从预览隐藏，原始事实永久保留。
- 桌面宽度大于 900px 时固定应用壳层，由 `.content-scroll` 负责唯一页面级滚动；窄窗口继续使用自然文档滚动。
- Project Overview、Pulse、Snapshot、Session、Evidence、AI Provider、Context Pack、Candidate、GitHub Import、Repository、Codex Task 与 Execution 统一接入内容高度 Token。
- Snapshot 与 Session 历史改为单条展开、首批 10 条；Pulse 来源首批 5 条，历史首批 10 条；设置与历史默认折叠。
- Pulse 审核页在宽屏下将确认操作保持可见，正式事实对比独立滚动，避免长事实把核心按钮推离视口。
- 引入 Vitest、Testing Library、jest-dom 与 jsdom 开发测试栈，新增 `npm run test`。

## 验证结果

- `npm run typecheck`、`npm run test` 与 `npm run build` 通过；9 项前端测试覆盖旧包装预览、真实溢出、10,000 字正文、长路径/Hash、列表计数、单条历史展开、原文保留和页面值切换重置。
- `cargo fmt --check`、`cargo check` 与主程序 `cargo test --jobs 2` 通过：67 项单元测试及全部集成测试通过，2 项依赖外部环境的测试按设计忽略。
- `forgeos-mcp` 的格式、编译和测试通过：12 个只读工具保持不变，STDIO 安全与隔离测试通过。
- SQLite `integrity_check = ok`，`foreign_key_check = 0`；D6.2 未新增 Migration、Rust Domain 或 IPC。
- Windows 实机检查覆盖 960px 级桌面窗口与最大化宽屏：无文字跨列、无页面级横向滚动、侧栏和顶栏稳定；Daymark 旧 Snapshot 显示实际用户问题而不是附件路径；Pulse 主操作、审核确认、来源摘要与次级折叠区域均可发现。

## 下一步计划

等待用户验收 D6.2。当前不提交 Git，不进入后端分页、虚拟列表、数据库全文搜索或新的业务领域开发。

# 2026-07-15 — ForgeOS D6.4 全局语言设置与通俗化界面

## 当前阶段

D6.4：应用级中英文切换、领域术语显示边界与全页面通俗表达调整。

## 完成内容

- 新增前向 Migration `0016_application_preferences.sql` 与 `app_preferences` 单例设置表，默认跟随系统语言并关闭英文术语说明。
- 新增 AppPreferences Domain、Repository、Application Kernel、IPC 与乐观版本冲突处理；设置读取失败不阻止应用启动，保存冲突会重新读取而不覆盖其他窗口。
- 引入 `i18next` 与 `react-i18next`，在 Workspace 加载前完成语言解析；支持 `system / zh-CN / en-US` 并在应用设置页即时切换、重启保持。
- 新增 `PageIntro`、`EmptyState`、`TechnicalTerm` 与 `TechnicalDetails`，统一页面用途、主要操作、空状态和技术信息层级。
- 项目六个主要页面调整为“概览、当前状态、开发记录、项目资料、代码仓库、AI 协作”，默认不依赖 Snapshot、Session、Evidence、Repository、Context Pack 等内部术语才能理解。
- 同步调整工作区、项目列表、项目设置、审核项目状态、AI 服务、AI 建议、GitHub 导入、给 Codex 的任务、执行记录、启动页、404 和全局导航。
- 建立中央状态/日期显示层与基于 IPC 错误码的本地化；用户输入、Git/GitHub 原文、AI 回复、既有 Context Pack 与 MCP 数据保持原文，不因界面语言切换而改写或重算。
- 新增 ADR-017，并更新 Architecture、Database Design 与 Development Guide 的本地化契约。

## 验证结果

- `npm run typecheck`、`npm run test`、`npm run build` 通过；4 个测试文件共 36 项测试通过，覆盖系统语言解析、双语资源键一致、即时切换、持久化版本、并发冲突重载、术语提示边界和现有工作台回归。
- `cargo fmt --check` 与 `cargo check` 通过。
- 主程序 `cargo test --target-dir target-d64 --jobs 2 -q` 通过：70 项单元测试通过，1 项外部依赖测试按设计忽略，全部数据库与应用集成测试通过。
- `cargo test --manifest-path forgeos-mcp/Cargo.toml -q` 通过；MCP 只读协议与 STDIO 集成测试保持不变。
- 本机现有 ForgeOS SQLite 已执行 Migration 0016；`app_preferences` 默认记录为 `system / false / v1`，`integrity_check = ok`，`foreign_key_check = 0`。
- `git diff --check` 无空白错误。生产构建仅保留既有的前端主包大于 500 kB 提示，本阶段未改变路由拆包策略。
- 已完成自动化布局与双语回归；960×720、1280×820、1920×1080 及 100%/125%/150% 的完整人工视觉矩阵留给 D6.4 实机验收，不在日志中虚报已完成。

## 下一步计划

等待用户验收 D6.4。当前不自动提交 Git，也不进入 Codex 执行、Agent、MCP 写入、自动翻译或新的业务领域开发。

# 2026-07-15 — ForgeOS D7 研发记忆助手聚焦重构

## 当前阶段

D7：统一研发历史、结构化成果与四页项目工作台。

## 完成内容

- 新增 Migration 0017：安全 Commit 元数据观察、近一年/2,000 条首次回溯、增量游标与 SyncRun Commit 计数。
- 从 Pulse 提取共享 `ActivityEpisodeBuilder`，新增 `ProjectMemoryQueryKernel`，将 Codex Episode、Git Commit 与人工 Session 聚合为游标分页研发历史。
- 新增 Migration 0018 与 Achievement Domain，支持系统成果草稿、人工确认、来源链、重点成果、简历表述和归档。
- Pulse 确认 eligible 活动时在同一事务创建一个成果草稿；重复确认不会重复生成，失败完整回滚。
- 全局导航收敛为项目、AI 服务、应用设置；项目一级页面收敛为概览、研发历史、成果与资料、AI 项目上下文。
- 项目总览改为批量记忆摘要；概览补充项目身份、最近活动和已确认成果；研发历史不再平铺 Codex Turn。
- AI 页面以固定项目上下文与复制为主，应用内 AI 和旧 Context Export 收入折叠区域，移除 Candidate 保存入口。
- Candidate、Task Proposal、Execution 从导航移除并冻结为只读兼容页面；Tauri 取消注册对应写 Command。
- MCP 收敛为五个只读工具，并支持 Project ID 或本地仓库目录二选一定位；不返回路径、Secret、原始对话、Diff 或作者邮箱。
- 新增 ADR-018 与 ADR-019。

## 安全边界

- 不删除或回改 0001–0016，不迁移旧 Evidence 为 Achievement。
- 不读取 Codex reasoning 或完整对话，不保存完整命令输出。
- 不执行 Git 写操作、Codex、Agent 或 MCP 写入。
- 自动成果始终为 Draft；AI 与 MCP 均不能修改正式事实。

## 验证结果

- `npm run typecheck`、`npm run test -- --run` 与 `npm run build` 通过；4 个测试文件共 36 项测试通过。生产构建仅保留既有的主包大于 500 kB 提示。
- `cargo fmt --check`、主程序与 MCP `cargo check` 通过。
- 主程序 `cargo test` 通过：73 项单元测试中 72 项通过，1 项需要本机 Codex 历史的 Smoke Test 按设计忽略；全部领域和数据库集成测试通过。
- `cargo test --manifest-path forgeos-mcp/Cargo.toml` 通过；五工具白名单、仓库根目录/子目录匹配、Windows 大小写归一、嵌套仓库最长路径、项目隔离与敏感信息保护均通过。
- 本机现有数据库从 Migration 0016 前向升级至 0018；升级前后 2 个 Project、355 条 Codex Observation、10 条 Pulse、13 条 Evidence、1 条 Snapshot 与 1 条 Session 保持不变。
- 真实数据库 `integrity_check = ok`，`foreign_key_check = 0`；新 Commit Observation 与 Achievement 表初始为空，等待下一次项目同步和人工审核填充。
- 发现 D6 开发期曾在 Migration 0013 应用后调整文件注释/内容，SQLx 校验和与真实库不一致。确认 0013 的表、列、索引和外键结构完全一致后，已先备份数据库，再对已知旧校验值执行受保护的元数据修复；0014–0016 校验值原本一致。
- `git diff --check` 无空白错误。

## 待实机验收

- 在两个真实项目中分别执行一次“同步最新进展”，验证近一年 Commit 首次回溯和 355 条 Observation 的活动聚合展示。
- 审核一个 eligible Pulse，确认只生成一个成果草稿；补充影响和简历表述后再人工确认。
- 重启 Codex MCP，确认工具列表只显示新的五个只读工具，并从仓库子目录定位正确项目。
