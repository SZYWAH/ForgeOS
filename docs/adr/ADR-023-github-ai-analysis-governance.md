# ADR-023：GitHub AI 增强必须建立在来源治理的规则报告之上

- 状态：Accepted
- 日期：2026-07-22
- 决策范围：D9.2 GitHub Project Intelligence 的用户主动 AI 增强

## 背景

`github-rules-v2` 已能为公开 GitHub Repository 生成带来源的确定性报告。AI 可以改善简介、架构职责、风险和阅读顺序的表达，但 Repository 内容本身不可信，模型输出也不能成为新的事实来源。如果 AI 绕过规则报告直接阅读仓库，ForgeOS 将无法保证来源归属、确定性字段、Secret 边界和失败回退。

## 决策

1. 每次 AI 增强都先生成完整 `github-rules-v2` 基线；安全阻断的基线不得调用 AI。
2. AI 仅接收规则报告与最多 60,000 个 Unicode 字符的脱敏来源摘要。它不获得 Tool Calling、Shell、Credential、本地文件、GitHub Token 或任何写权限。
3. Repository 身份、类型、技术栈、入口、命令、依赖、License、Commit、Release、章节可用性和采集统计保持规则结果，AI 不能改写。
4. AI 只能增强合同列出的解释性章节；每个非空结论必须引用当前 Run、已进入 AI 输入且适用于该章节的 Source ID。
5. 首次响应不合法时只允许一次格式修复请求。网络、Provider、结构或来源校验失败时保存完整规则报告，并标记 `rules_fallback`，不让 Run 整体失败。
6. AI Run 与规则 Run 分开保存。相同 HEAD、重点和问题可以复用安全规则来源，但必须为新 Run 重建来源身份，旧报告、Markdown 和 Hash 不改写。
7. Provider 必须由用户显式选择且处于启用状态；本地 OpenAI-compatible 服务可以不配置 API Key。Request Hash 纳入 Provider ID、Provider Version、Model 和 AI Prompt Revision。
8. 不保存 API Key、完整 Prompt、原始 AI Response 或推理过程。

## 结果

- 无 AI、AI 断开或 AI 输出不可信时，规则报告仍完整可用。
- AI 报告保持可追溯，并与规则报告形成独立历史。
- D9.2 不获得克隆、运行、导入 Project 或 GitHub 写入能力。

## 后续

“导入为 ForgeOS 项目”继续留给 D9.3，并必须经过 GitHub Import Draft 和用户确认，不能由 AI 报告直接创建正式 Project。
