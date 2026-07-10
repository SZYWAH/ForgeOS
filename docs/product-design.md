# ForgeOS 产品设计基线 v0.1

- 状态：已冻结
- 当前工程阶段：D0
- 目标用户：同时维护多个软件项目并使用 Git、GitHub 和 Codex 的独立开发者

## 产品定位

ForgeOS 是 **Personal AI-Assisted Engineering Operating System**。

它位于代码仓库、GitHub 和 AI 编程助手之上，负责保存项目意图、当前状态、研发过程、决策理由和验证证据。

## 不可替代价值

ForgeOS 的核心价值是：

> 建立项目隔离、人工确认、证据可追溯的研发记忆，并把它转换为开发者和 Codex 都能可靠使用的上下文。

它重点回答：

- 项目现在是什么状态？
- 上一次工作完成了什么？
- 下一次应从哪里继续？
- 某项技术决策为什么被采用？
- 某个问题经过了哪些排查？
- 某项成果是否已经验证？
- Codex 本次工作需要哪些上下文？

## 产品边界

ForgeOS 不是：

- Todo 或 GTD 工具
- Jira 或 GitHub Issues 替代品
- 普通笔记软件
- AI 聊天工具
- IDE 或代码托管平台
- 无人监督的自动开发 Agent

## MVP 核心对象

### Project

保存项目身份、目标、范围、技术栈和约束。

### Current Snapshot

表达项目当前的可信状态，包括当前目标、已验证成果、阻塞和最多三个下一步。

### Development Session

记录一次研发工作的目标、结果、验证和接手信息。

### Decision

记录技术决策的背景、候选方案、权衡、选择和长期影响。

### Problem

记录问题现象、环境、排查尝试、根因、解决方案和验证。

### Evidence

保存支持成果、决策和问题结论的文件、链接、测试、Commit 或其他证据。

### Context Export

从单一项目的已确认记录中生成可供 Codex 使用的 Markdown Context Pack。

## MVP 必须具备

- 单用户
- 本地优先
- Windows 桌面应用
- 项目级数据隔离
- 人工确认正式事实
- 记录版本和来源
- 基础搜索
- 一致性备份与恢复
- 确定性 Context Export

## 推迟实现

- 通用任务、看板、Sprint、工时和 Backlog
- Git 自动采集
- GitHub 同步
- AI 主动分析
- 自动维护 `AGENTS.md`
- ForgeOS MCP Server
- Codex 自动任务执行
- 多用户和多设备同步
- 向量数据库
- 移动端、云服务和插件系统

## 产品原则

1. Project 是所有研发记录的归属边界。
2. Current Snapshot 与历史事件分离。
3. 原始证据、人工确认事实和 AI 候选必须分层。
4. AI 不得静默修改正式事实。
5. Context Pack 默认只能包含一个项目。
6. 一次记录应能服务开发交接、复盘和职业材料。
7. 产品首页围绕状态、阻塞、成果和新鲜度，而不是任务数量。
8. 所有核心能力在无 AI、无网络时仍可使用。

## 防止退化为任务管理软件

- 不建立通用 Task 数据模型。
- 不提供 Kanban、燃尽图或任务完成百分比。
- Project 首页默认展示 Current Snapshot。
- Snapshot 的下一步最多三项，且不形成独立待办系统。
- Development Session 固定围绕“目标、行动、结果、验证、接手”。
- 项目进展使用已验证成果、阻塞和状态新鲜度表达。

## MVP 成功标准

- 60 秒内恢复一个项目的上下文
- 两分钟内完成一次 Development Session 收尾
- 已确认事实能够追溯到来源和版本
- Context Pack 不出现跨项目上下文污染
- 离线状态下核心流程完整可用

## D0 边界

D0 只验证工程基础和 Windows 构建链路，不实现上述业务对象的完整功能。任何超出 D0 的能力都必须等待当前阶段验收后再开始。
