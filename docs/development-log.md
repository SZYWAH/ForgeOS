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
