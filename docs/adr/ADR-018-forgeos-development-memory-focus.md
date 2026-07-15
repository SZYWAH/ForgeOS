# ADR-018：ForgeOS 聚焦个人研发记忆

- 状态：Accepted
- 日期：2026-07-15

## 背景

ForgeOS 已经具备 Project、Snapshot、Session、Evidence、Context Pack、Candidate、Codex Task Proposal、Execution、GitHub Import、Git 观察和 MCP。真实数据表明，Codex Observation 与 Project Pulse 已产生持续价值，而 Candidate、Task Proposal、Execution 几乎没有被使用。过多同级页面让产品更像研发管理后台，偏离“快速恢复多项目上下文”的核心目标。

## 候选方案

1. 继续扩展所有既有领域，使其成为完整研发管理平台。
2. 删除低使用率领域与历史 Migration，重新建立精简数据库。
3. 保留底层数据和兼容查询，重构产品表面为项目身份、当前状态、研发历史、成果与 AI 上下文四条主线。

## 决策

采用方案 3。ForgeOS 的项目一级页面收敛为概览、研发历史、成果与资料、AI 项目上下文。Snapshot、Repository 与 Pulse 审核保留为高级入口。Candidate、Task Proposal、Execution 从导航移除并冻结为只读兼容页面；对应 Domain、Repository、Migration 与历史数据继续保留。

应用内 AI 降级为用户主动使用的可选工具。Codex 的主要集成面是只读 MCP 和确定性项目上下文。ForgeOS 不成为任务管理器、Codex 执行器或 Git 控制器。

## 优点

- 用户进入项目后可以直接回答“项目是什么、现在做到哪里、最近做了什么、下一步是什么”。
- 降低领域术语和页面切换成本，避免多项目上下文混淆。
- 不删除历史数据，升级可回滚且兼容已有数据库。
- AI 与正式事实仍保持人工确认边界。

## 缺点

- 旧功能代码与表仍然存在，需要承担兼容维护成本。
- 研发历史是读模型，查询和聚合逻辑比直接列 Session 更复杂。
- 部分高级功能需要从设置或兼容 URL 进入。

## 长期影响

后续新增能力必须证明其直接改善项目上下文恢复、研发历史可信度、成果沉淀或 AI 理解效率。不得以通用任务、看板、Agent 执行或写入型 MCP 重新扩张主流程。
