# ADR-009：Git 集成采用只读观察而不是仓库控制

- 状态：已接受
- 日期：2026-07-12
- 决策范围：本地 Git Repository、WorkingCopy 观察与 Git Evidence
- 前置决策：ADR-005（Evidence 独立模型）、ADR-007（确定性 Context Pack）

## 背景

ForgeOS 已能记录用户确认的项目状态、研发过程和证据，但 Commit、当前分支、HEAD 与 Working Tree 状态仍需要人工复制。这些信息是有价值的外部研发事实，适合转换为 Evidence。

如果 ForgeOS 同时提供 commit、checkout、pull、merge 等控制能力，就会从研发记忆系统扩张为 Git 客户端，并可能修改用户正在工作的仓库。D2 的目标只是可靠观察和捕获证据，因此必须冻结只读边界。

## 候选方案

### 方案一：完整 Git 客户端

可以在 ForgeOS 内完成常见 Git 操作，但需要处理冲突、凭据、网络、rebase、submodule、worktree 和恢复流程，风险与当前产品价值不匹配。

### 方案二：使用 libgit2 读取和写入仓库

提供结构化 API，但引入原生依赖和 Git 行为差异；一旦暴露写接口，仍存在修改仓库的风险。

### 方案三：固定白名单的 Git CLI 只读 Adapter

复用用户已安装的 Git，通过参数数组执行少量机器可读查询。Application 只接收结构化结果，不能传入任意 Git 子命令。

## 决策

选择方案三。

### 命令边界

允许的 Git 子命令仅包括：

- `rev-parse`：验证工作树、读取根路径和 HEAD。
- `symbolic-ref`：读取当前分支与 Origin HEAD。
- `remote get-url`：读取可选 Origin URL。
- `status --porcelain=v1 -z`：读取 dirty 与文件分类。
- `diff --numstat HEAD`：读取插入和删除摘要。
- `log`：读取最近十条 Commit。
- `show -s`：按完整 SHA 读取单条 Commit。

Adapter 不提供通用 `run(args)` Port，也不包含 commit、push、pull、merge、checkout、reset 或 stash 分支。

### 执行安全

- 使用进程参数数组，不经过 PowerShell、cmd 或 shell 字符串拼接。
- Repository path 和 Commit SHA 都作为独立参数；SHA 只接受 40/64 位小写十六进制。
- 设置 `GIT_OPTIONAL_LOCKS=0`、`GIT_TERMINAL_PROMPT=0` 和十秒超时。
- Windows 后台进程隐藏窗口，避免刷新状态时弹出终端。
- HTTP(S) Remote URL 移除 user-info 后才进入 SQLite，防止嵌入式 Token 持久化。
- 不执行网络型 Git 操作。

### 持久化模型

- Repository 保存 Project、规范化根路径、Remote、默认分支、最后检查时间和版本。
- 同一根路径只能注册一次，避免一个 Working Tree 被错误归入多个 Project。
- WorkingCopy 每次检查追加一条不可变记录，保存分支、HEAD、dirty 与 diff summary。
- Refresh 使用乐观并发版本，并在事务中同时更新 Repository 与插入 WorkingCopy。

### Git Evidence

- `git_commit` 保存完整 SHA、Message、Author、Date 与 Repository path。
- `git_status` 保存分支、HEAD、dirty、文件分类、插入/删除与检查时间。
- Evidence 默认仍为 Unverified；用户可以人工 Verify 或 Invalidate。
- Git Evidence 内容只能由 RepositoryKernel 创建，通用 Evidence 用例不能伪造或改写。
- RepositoryEvidenceLink 使用双复合外键确保 Evidence 与 Repository 属于同一 Project。

## 优点

- ForgeOS 能保存可追溯 Git 事实，同时不承担 Git 客户端职责。
- 白名单和无通用命令入口使产品层无法执行写操作。
- 使用用户现有 Git，兼容本机仓库配置、对象格式和 worktree 行为。
- WorkingCopy 历史可以解释 Evidence 捕获时的工作区状态。
- Git Evidence 继续复用既有验证、Project 隔离与 Context Pack 流程。

## 缺点

- 依赖本机安装 Git，且可能受到 safe.directory 或损坏仓库影响。
- 多次 CLI 查询比单次 libgit2 调用开销更高。
- D2 不读取 submodule、tag、stash、reflog 或完整文件 Diff。
- 未跟踪文件不会进入 insertions/deletions，只计入 untracked 数量。
- Repository path 变更后需要未来独立的重定位流程；D2 不提供编辑或删除。

## 长期影响

- 后续 GitHub 集成必须作为独立外部 Adapter，不得在本 Git Adapter 中加入网络写操作。
- Codex、MCP 或 Agent 不得获得 Git command passthrough，也不能把 Evidence 创建解释为 Git 操作授权。
- 若未来确需 Git 写能力，必须另立产品边界、恢复策略和 ADR，不能扩展当前 RepositoryKernel。
- AI 可以在后续阶段分析已确认的 Git Evidence，但 D2 不自动调用 AI，也不让 AI 触发 Git 读取或写入。
