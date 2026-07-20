# ADR-022: GitHub 项目分析架构

## 背景

ForgeOS 当前能够把用户自己的 GitHub Repository 导入为 Project，但“导入一个项目”和“理解一个尚未接管的外部项目”是两个不同的产品行为。

D9 需要让用户输入一个公开 GitHub Repository URL，在不克隆、不执行代码、不创建 ForgeOS Project 的前提下，得到可追溯、可复查的项目分析报告。该能力仍需遵守 ForgeOS 的核心边界：外部数据是只读来源，AI 只能协助分析，用户决定哪些内容进入正式研发记忆。

## 候选方案

### 方案 A：先导入 Project，再复用现有项目上下文

- 优点：可以直接复用 Project、Evidence 和 Context Pack。
- 缺点：用户仅想了解一个开源项目时也会污染个人项目列表；外部分析结果容易被误认为已确认的项目事实。

### 方案 B：先克隆到本地，再使用 Git Adapter 分析

- 优点：可以读取完整代码树，后续运行和二开路径直接。
- 缺点：扩大磁盘、进程、Git 写操作和不可信代码边界；与本阶段的只读、低风险目标冲突。

### 方案 C：Workspace 级只读分析，人工决定是否导入

- 优点：分析与接管生命周期分离；无需本地仓库；可以按固定 HEAD 保存可复查报告；安全边界清晰。
- 缺点：GitHub API 有速率、Tree 截断和文件预算限制；无法证明项目能够在本机运行。

## 决策

采用方案 C。

1. 分析对象是 Workspace 下的外部公开 GitHub Repository，不要求预先存在 Project。
2. D9 只支持 Public Repository。现有 Token 仅用于提高只读 API 预算，不扩大到私有仓库。
3. 默认不克隆、不执行代码、不持续同步；一次分析由用户主动发起。
4. 规则分析始终可用；AI 增强必须由用户主动开启，并使用经过筛选和脱敏的结构化来源。
5. 每次分析绑定默认分支的具体 HEAD Commit SHA。HEAD 变化时旧报告保留并标记过期，不原地重算。
6. Analysis Report 是外部项目理解结果，不是 ForgeOS 正式项目事实。
7. 只有用户点击“导入为 ForgeOS 项目”后，才进入现有 GitHub Import Draft 和 Project 创建流程。
8. 每个非空报告结论必须包含置信度和真实来源 ID。AI 无法提供有效来源的结论不得进入正式报告。
9. ADR-012 的 GitHub 只读和人工审核原则继续有效；ADR-018 的研发记忆主线不变。
10. MCP、Codex Execution、Git Adapter 不获得任何写入、Shell、克隆或执行能力。

计划中的 Migration 为 `0019_github_project_intelligence.sql`，但 D9.0 只冻结设计，不创建 Migration。

## 优点

- 用户可以先理解项目，再决定是否接管，项目列表不会被试读对象污染。
- 报告绑定 HEAD SHA，结论和来源可复查，不会把不同版本混成一个“最新事实”。
- 规则模式不依赖 AI Provider，失败时仍能给出基础报告。
- 来源预算、脱敏和引用约束使外部 Repository 无法直接控制 AI 或 ForgeOS。
- 后续导入复用现有 GitHub Import Draft，避免建立第二套 Project 创建流程。

## 缺点

- 大型 Monorepo、二进制仓库或缺少文档的项目只能得到部分报告。
- GitHub API 限流、Tree 截断和 SBOM 可用性会影响完整度。
- 不执行构建或测试，因此“运行方式”和“复现难度”只能基于静态来源判断，不能宣称已验证。
- 需要新增独立的 Analysis Run 与 Source 存储，不能简单复用 Evidence。

## 长期影响

- D9.1 的实现必须遵守 `docs/github-analysis-contract.md`，不得在实现阶段重新扩张权限。
- 后续若增加私有仓库、克隆、复现或二开执行，必须分别新增 ADR、安全审计和明确授权，不得借用本 ADR 隐式放行。
- Analysis Report 可以成为 Project Import 的参考来源，但不会自动成为 Snapshot、Session、Evidence、Achievement 或 MCP 正式上下文。
- 新分析器版本通过 `analyzer_revision` 和新的 Analysis Run 演进；历史报告永久绑定原 HEAD 和原版本。
