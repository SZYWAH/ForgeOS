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
