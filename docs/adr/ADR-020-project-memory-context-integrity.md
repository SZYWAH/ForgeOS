# ADR-020：项目研发记忆与 AI 上下文完整性

- 状态：Accepted
- 日期：2026-07-17

## 背景

ForgeOS 已经能够从 Codex、Git、Snapshot、Session 和 Pulse 聚合研发记忆，但旧数据中可能保留 Codex 请求包装、附件元数据、整段 README、绝对路径或疑似凭据。历史读模型还可能把同一条 Agent 最终回答同时显示为“活动”和“结果”。如果这些内容未经治理便进入项目概览、Context Export 或 MCP，会降低项目状态的可信度，并扩大敏感信息暴露范围。

## 候选方案

1. 批量重写旧 Snapshot、Observation 和 ContextExport。
2. 调用 AI 自动清洗并覆盖正式事实。
3. 保留不可变历史，在 Application 层增加确定性内容质量策略；读时生成安全投影，正式状态通过人工审核的修复草稿创建新版本。

## 决策

采用方案 3。新增统一 `MemoryContentPolicy`，负责移除提示包装、语义去重、README 范围摘录、路径与凭据脱敏、Unicode 安全截断和稳定质量代码。研发历史、Snapshot 修复草稿、AI 项目上下文、Context Export 与 MCP 必须复用该策略，不得各自维护清洗规则。

旧 Snapshot、Observation 和 ContextExport 永不回写。Snapshot 污染通过只读 Repair Draft 暴露；用户编辑并确认后仍调用现有 Snapshot 版本化写入，产生 v2、v3 等新版本。疑似真实 Secret 或无法清除的提示包装属于阻断问题，不得复制、导出或通过 MCP 返回 Markdown；身份缺失、状态过期和成功移除的包装只形成警告。

AI 上下文的结构化预览和 Markdown 使用同一份安全数据，Hash 对最终安全 Markdown 计算。MCP `get_project_context` 与 `build_ai_context` 使用相同投影，但工具列表、只读边界和项目定位协议保持不变。

## 优点

- 保留历史可追溯性，不伪造或覆盖用户确认事实。
- 桌面端、Context Export 与 MCP 不会出现相互矛盾的安全投影。
- 无需依赖 AI 即可稳定测试、复现和解释清洗结果。
- 用户能看到质量问题及来源，并决定是否创建新版正式状态。

## 缺点

- 旧数据仍然保存在本地数据库中，只是在对外投影时被治理。
- 确定性规则需要持续维护误报与漏报测试。
- 阻断问题会暂时降低复制和导出便利性，必须先修正来源内容。

## 长期影响

任何新增的研发记忆出口都必须先通过统一质量策略。自动分析只能生成观察或草稿；正式事实继续由用户确认。若未来升级清洗规则，应版本化投影行为并保持旧 ContextExport 的内容与 Hash 不变。
