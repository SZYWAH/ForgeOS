# ForgeOS 开发指南

## 开发原则

- 严格遵守已冻结的产品边界
- 优先完成可信、可验证、可恢复的基础能力
- 不为未来需求提前引入复杂框架
- 所有正式数据写入必须经过 Rust
- 保持核心功能离线可用
- 未经确认不得自动提交 Git 变更

## 已验证环境

| 工具 | 版本或状态 |
|---|---|
| Windows | Windows 10/11 兼容构建环境 |
| Node.js | 24.14.1 |
| npm | 11.11.0 |
| rustc | 1.96.1 |
| Cargo | 1.96.1 |
| Rust target | `stable-x86_64-pc-windows-msvc` |
| Tauri CLI | 项目本地 2.11.4 |
| rustfmt | 1.9.0-stable |
| WebView2 Runtime | 150.0.4078.48 |
| Visual Studio Build Tools | 2022 17.14.35 |
| MSVC Toolset | 14.44.35207 |
| Windows SDK | 10.0.26100.0 |

MSVC 工具本身已经安装，但普通 PowerShell 不一定包含 `cl.exe`。Cargo 的 MSVC 构建流程通常能够发现已安装的 Build Tools；若需要手工开发者环境，可使用 Visual Studio Build Tools 的 `vcvars64.bat`。

项目使用本地 Tauri CLI，不要求全局安装。Rust 代码在提交前执行
`cargo fmt --check`。

## 首次安装

```powershell
npm install
npm --prefix frontend install
```

第一条命令安装项目本地 Tauri CLI，第二条命令安装前端依赖。不要为了运行项目安装全局 Tauri CLI。

## 常用命令

### 前端

```powershell
npm --prefix frontend run dev
npm --prefix frontend run build
```

### Rust

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
```

### Tauri

```powershell
npm run tauri:dev
```

未安装项目本地 Tauri CLI 时，可以使用已经验证过的双终端方式：

```powershell
# 终端一
npm --prefix frontend run dev

# 终端二
$env:FORGEOS_DATA_DIR = Join-Path $PWD ".data"
cargo run --manifest-path src-tauri/Cargo.toml
```

### 数据库

Migration 由 Rust 启动流程自动执行。禁止手工修改已经在真实数据上执行过的迁移。

## 代码组织

### TypeScript

- 启用严格类型检查
- 业务组件不得直接调用底层 `invoke`
- Tauri 调用集中在 IPC Client
- 正式领域数据不存入 `localStorage`
- 页面负责组合，Feature 负责交互，Shared 不依赖业务模块
- 不使用 `any` 绕过 IPC 合同

### Rust

- `ipc` 只处理边界转换
- `application` 负责编排
- `domain` 保存领域规则
- `ports` 保存抽象接口
- `infrastructure` 保存具体实现
- 使用明确的 `Result` 和错误类型
- 禁止在业务代码中使用无说明的 `unwrap`
- SQL 不散落在 IPC 或 UI 相关模块
- 异步任务必须考虑取消、超时和错误传播

## 数据库规范

- 表名使用复数 `snake_case`
- 列名使用 `snake_case`
- ID 使用应用生成的文本 ID
- 时间统一保存为 UTC
- 外键必须开启
- Migration 只向前追加
- 每个 Migration 只承担一个明确目的
- 非简单增加型迁移前必须设计备份策略
- D0 仅允许 `workspaces` 和 `projects` 业务表

## 文档规范

以下变化必须同步更新文档：

- 产品边界变化
- 模块依赖变化
- 数据结构变化
- 开发或构建命令变化
- 重要技术选择

重要架构选择使用 ADR 记录。已接受 ADR 不原地重写结论；若决策改变，应新增 ADR 并标记替代关系。

## Git 规范

### 分支

- 主分支保持可构建
- 功能分支建议使用：
  - `chore/...`
  - `feat/...`
  - `fix/...`
  - `docs/...`
  - `refactor/...`

### Commit

使用简洁的 Conventional Commit 风格：

- `chore: initialize ForgeOS architecture`
- `feat: add project snapshot persistence`
- `fix: handle database migration failure`
- `docs: update database design`

一个 Commit 应对应一个清晰目的。不得把无关改动混入同一 Commit。

AI 或自动化工具不得在没有用户明确确认时执行：

- `git commit`
- `git push`
- merge
- rebase
- release

### 提交前检查

- 查看 `git status`
- 确认没有凭据或本地数据库
- 执行相关格式化和构建检查
- 更新必要文档
- 确认没有超出当前阶段范围

## 安全规范

不得提交：

- API Key
- OAuth Token
- `.env` 中的秘密
- 本地 SQLite 数据库
- 临时日志
- 构建产物
- 用户项目中的隐私数据

## D0 完成定义

D0 工程初始化只有在以下条件满足时才算完成：

- 前端能够启动和构建
- Rust 能通过编译检查
- Tauri 桌面窗口能够启动
- SQLite 能连接
- Migration 能执行
- Rust 能完成一次数据库读取
- 工程模块和文档存在
- Git 工作区状态已报告
- 没有自动提交

## Localization Contract

用户可见文案必须遵守以下规则：

- 使用 `i18next` 稳定语义键，禁止以中文或英文原句作为翻译键。
- 新增或修改用户可见文本时，必须同时更新 `zh-CN` 与 `en-US`；自动测试必须验证两套资源键完全一致。
- 页面使用通俗标题、一句话用途说明、一个首要操作、数据内容和默认折叠的技术信息。关键解释不能只放在 Tooltip。
- 导航和主要按钮不直接暴露 Snapshot、Session、Evidence、Context Pack 等内部架构名；需要对照时使用 `TechnicalTerm`，且只在中文和用户显式开启术语提示时显示。
- 数据库枚举通过中央显示字典转换，禁止在页面内重复维护状态映射。
- 日期和相对时间必须使用当前应用语言，不能硬编码 `zh-CN`。
- IPC 错误优先根据稳定 `code` 本地化，并提供用户可执行的下一步；英文原始消息与错误码进入 `TechnicalDetails`。
- 不翻译或重写用户输入、Git/GitHub 原文、AI 回复、Codex 内容、历史 Context Pack 或 MCP 数据。
- 新页面必须覆盖中英文、空状态、错误状态和长文本状态测试。

## Content Presentation Contract

ForgeOS 的页面只负责展示摘要和支持决策，数据库中的原始事实不得因界面长度预算而截断或改写。

- 当前目标默认显示 2 行，状态总结默认显示 3 行。
- 成果、阻塞、下一步等列表默认显示 3 项，每项最多显示 2 行。
- 只有内容真实溢出时才显示“展开全文”；展开后的普通正文最高 480px，Markdown、AI 响应和执行结果最高 580px，超出后在区域内部滚动。
- Snapshot、Session、Pulse 等历史记录默认只展示紧凑记录头，同一列表一次只能展开一条，首批渲染 10 条。
- 路径、URL、Hash 和 ID 使用单行省略显示，并保留完整值提示与复制能力。
- 所有用户输入、Markdown 和外部文本容器必须具有 `min-width: 0`、`max-width: 100%`、`overflow-wrap: anywhere` 与 `word-break: break-word`。
- 桌面宽度大于 900px 时，`.content-scroll` 是唯一页面级滚动区域；窄窗口恢复自然文档滚动。
- 概览允许清理旧 Codex 附件包装形成可读预览，但展开后必须显示未经修改的原始字段值。
- 新增长内容页面优先复用 `CollapsibleText`、`BoundedList`、`RecordDisclosure` 和 `CompactIdentifier`，不得重新引入分散的固定高度。

## Visual System Contract

视觉样式的详细契约见 `docs/visual-system.md`。新增或修改页面时：

- 必须复用视觉 Token 与共享组件，不得在页面样式中硬编码颜色、圆角、阴影或控件高度。
- 必须选择 Index、Workbench、List Detail 或 Form 中的一种页面模式。
- 每页首屏最多一个主操作；空状态不得保留无意义的详情面板或等高卡片。
- 必须保留键盘焦点、语义状态、长内容边界和 `prefers-reduced-motion`。
- 修改共享 Shell、Panel、表单或按钮时，必须回归所有已注册路由。

## Development Memory Contract

- 项目一级导航只包含概览、研发历史、成果与资料、AI 项目上下文。
- 页面首屏最多一个主操作；高级配置和兼容功能不得重新占据一级导航。
- 研发历史必须使用共享 `ActivityEpisodeBuilder`，禁止在前端或另一个 Kernel 中复制 Episode 规则。
- Dashboard 使用批量 Project Memory Summary IPC，禁止前端按项目发起 Pulse/Repository N+1 请求。
- Git 历史只允许保存 Commit Subject、作者名称、SHA 和时间；禁止保存邮箱、Diff、Body 或文件内容。
- 系统整理出的成果只能保存为 Draft，必须经过用户确认才能成为正式成果。
- MCP 的 Repository Path 只用于本次定位，不得记录、返回或写入 Context。
- Candidate、Task Proposal 与 Execution 已冻结；不得在新页面重新引入创建、审批或状态流转入口。

## Memory Content Integrity Contract

- 所有进入研发历史、AI 项目上下文、Context Export 或 MCP 的自由文本必须经过 `MemoryContentPolicy`；禁止在 IPC、Renderer 或页面中复制清洗规则。
- 清洗只作用于读模型和新草稿，不得回写旧 Snapshot、Observation、Session、Evidence 或 ContextExport。
- Activity 与 Outcome 必须语义独立；同一 Agent 最终回答不能同时填充两个字段。Manual Session Notes 不得同时冒充“为什么这么做”。
- README 类 Scope 只允许输出定位、功能和范围摘要；安装命令、环境变量、代码块、备份说明和绝对路径不得进入 AI Context。
- 疑似真实 Secret 和未清除提示包装属于 blocking；必须禁用复制、导出和 MCP Markdown 返回。示例值可以不阻断，但不得进入 Scope 摘要。
- Snapshot 修复必须先生成 Repair Draft，由用户编辑并通过现有版本检查保存为新版本；禁止原地覆盖历史。
- 结构化列表必须使用独立项目编辑器。Next Steps 最多三项，不得把整段 Markdown 按行拆成大量条目。
- AI Context 页面只保留一个主要复制入口；原始安全 Markdown、Hash 和技术范围默认折叠。
