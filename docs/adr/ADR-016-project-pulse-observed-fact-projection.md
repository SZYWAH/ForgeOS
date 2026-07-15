# ADR-016：Project Pulse 作为待确认事实投影

- 状态：已接受
- 日期：2026-07-13
- 范围：D6 Project Pulse、Snapshot/Session 确认闭环、MCP 与 Context Pack

## 背景

早期 ForgeOS 要求用户手工填写 Current Snapshot 和 Development Session。这样能够保护正式事实，却把“人工确认”错误地实现成“人工录入”，导致项目首页无法快速回答当前在做什么、最近完成了什么、是否验证、Git 是否待整理以及下一步是什么。

Codex 与 Git 可以提供外部观察，但自动摘要存在遗漏和误判。自动结果不能直接覆盖 Snapshot、Session、Evidence 或 Project 状态。

## 候选方案

1. 自动更新 Snapshot/Session：体验直接，但错误观察会污染正式事实和版本历史。
2. 复用 AI Candidate：Candidate 绑定用户主动保存的 ContextExport，缺少同步新鲜度、Codex/Git 来源链和项目首页生命周期。
3. 新建 Project Pulse，作为由外部观察生成、必须人工审核的非正式事实投影。

## 决策

选择方案三。

- Snapshot 与 Session 继续作为正式事实；Pulse 默认状态为 `pending_review`。
- 同一 Project 最多一个待审核 Pulse。来源变化时生成新版本，并把旧待审核版本标记为 `superseded`。
- 相同来源指纹返回已有 Pulse，不重新调用 AI。
- 无 AI 时使用确定性规则；AI 仅增强结构化草稿，失败自动回退规则摘要。
- 用户可以编辑 Pulse，但只有 `confirm_project_pulse` 可以把选定内容应用到正式领域。
- 确认时重新检查 Snapshot/Session basis version，在一个 SQLite 事务中创建新版 Snapshot、开始或更新 Session，并把 Pulse 标记为 `confirmed`；冲突或任一步失败全部回滚。
- 完成 Session 默认不选中，且必须同时存在 Outcome、Validation 和 Handoff。
- MCP 可以读取最近保存的 Pulse，但必须返回 `fact_status`；Context Pack 把未确认观察放在独立章节，不能与 confirmed Snapshot 混写。

## 优点

- 用户从“填写记录”转为“审核事实”，显著降低研发记忆维护成本。
- 自动化错误不会污染正式领域。
- 来源链、内容哈希、新鲜度和历史状态使系统观察可解释、可审计。
- 规则与 AI 共用同一治理层，Provider 故障不会阻断同步。

## 缺点

- 产品中同时存在 observed 与 confirmed 两层，界面和 MCP 必须持续明确标识。
- 确认事务需要处理 Snapshot/Session 并发版本冲突。
- Pulse 不保存完整对话，某些判断仍需用户回到 Codex 查看原任务。

## 长期影响

- 新的自动分析能力必须先进入 Pulse 或其他候选层，不得直接写正式事实。
- Project Overview 以 Pulse 为唯一主入口；手工 Snapshot/Session 页面保留为高级编辑入口。
- Candidate 与 Pulse 保持不同语义和生命周期，不合并数据表。
- Codex Execution Record 保留兼容读取，但不进入 Project Pulse 主流程，也不继续扩展 Runner。
