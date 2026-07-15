# ADR-019：统一研发历史与结构化成果记忆

- 状态：Accepted
- 日期：2026-07-15

## 背景

Codex Activity Observation 以轮次保存，Git Commit 独立存在，Development Session 又代表人工确认过程。逐条展示会产生大量噪声，也无法把“做了什么、为什么、结果、验证、交接”放在一个可复盘单位中。Evidence 适合保存资料来源，但不足以表达可用于复盘、简历和面试的项目成果。

## 候选方案

1. 建立新的统一事件表，把所有历史重新写入一套模型。
2. 继续分别展示 Observation、Commit、Session 和 Evidence。
3. 保留不可变来源，以运行时 Activity Episode 形成统一历史读模型，并新增结构化 Achievement 领域。

## 决策

采用方案 3。`ActivityEpisodeBuilder` 是 Pulse 与研发历史共享的唯一分段实现。Codex Episode 为主要历史单位；时间窗内 Commit 附加到 Episode，未匹配 Commit 按仓库和自然日聚合，纯人工 Session 独立展示。历史使用稳定游标分页，不持久化重复事件。

新增 `project_achievements` 与来源链。系统只能从已确认 Pulse 或历史活动生成 Draft，用户确认后才成为正式成果。GitHub Evidence 不自动转换为成果。

## 优点

- 保留来源真实性和项目隔离，不重写既有 Observation。
- 数百条 Codex Turn 被压缩为少量可理解的研发活动。
- 成果可结构化保存问题、方案、结果、验证、影响和简历表述。
- Pulse、历史与成果之间具有可追溯来源链。

## 缺点

- 运行时聚合需要稳定排序、分页和去重测试。
- Episode 规则升级可能改变读模型呈现，需要通过版本化规则控制。
- 成果审核增加一次人工确认步骤。

## 长期影响

研发历史继续保持读模型，不建立第二套事实事件库。Achievement 是唯一新增的用户核心领域；其他自动分析结果必须先以草稿形式进入，不能自动成为正式项目记忆。
