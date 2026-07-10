# ADR-003：Current Snapshot 采用只追加版本

- 状态：已接受
- 日期：2026-07-10
- 决策范围：ForgeOS D1.2 的项目当前状态表达、版本保留与并发更新语义
- 前置决策：ADR-002（Project Identity Kernel）

## 背景

Project Identity Kernel 回答“这个项目是什么”，Current Snapshot 必须回答“这个项目现在真实处于什么状态”。用户暂停项目 30 天后再次打开时，应能通过当前目标、整体状态、最近成果、阻塞、下一步和最后确认时间，在 60 秒内恢复研发上下文。

Snapshot 是用户人工确认的研发事实，不是 Todo 列表、Task 系统、进度百分比或看板状态。它也不是可以被 AI 或外部同步静默改写的临时摘要。ForgeOS 的长期价值依赖可信研发记忆：当前内容要易于读取，曾经确认过的内容要能够追溯，陈旧页面或多个窗口不能覆盖较新的事实。

D1.2 需要在本地 SQLite 中同时支持：

- 每个 Project 在首次创建后只有一个明确的 Current Snapshot。
- 每次修改保留之前的确认版本。
- 并发或陈旧提交不会静默覆盖新事实。
- Snapshot 必须严格归属于 Project。
- 当前读取简单、确定，不引入 Task、Timeline、AI 或外部同步机制。

## 候选方案

### 方案一：原地覆盖单条 Snapshot

每个 Project 保存一行 Snapshot，修改时直接执行 `UPDATE`。实现和 Current 查询最简单，但旧事实会丢失；错误编辑、陈旧提交和未来自动化修改都难以审计，也无法解释项目状态如何演变。

### 方案二：版本表加可变 Current 指针

每次修改插入新版本，同时在 Project 或独立表中维护 `current_snapshot_id`。历史能够保留，但版本插入与指针更新必须始终原子提交；指针可能缺失、指向错误 Project 或落后于最新版本，还会形成两份“哪一条是 Current”的事实源。

### 方案三：只追加版本，以最高版本作为 Current

每个 Project 的 Snapshot 从版本 `1` 开始，每次修改只插入 `N + 1`。`UNIQUE(project_id, version)` 保护版本唯一，按 Project 读取最高版本即可确定 Current。客户端提交 `expected_version`，Application 检查后再追加；若两个写入竞争同一下一版本，唯一约束只允许一个成功，另一个回读最新版本并返回冲突。

## 决策

ForgeOS 选择方案三：Snapshot 使用不可变、只追加的 Project 内版本序列，最高版本是 Current Snapshot。

- `project_snapshots` 中每一行代表一次人工确认的完整状态，而不是局部补丁。
- 首次创建使用版本 `1`；更新读取当前版本 `N` 后插入完整版本 `N + 1`。
- 既有版本不原地覆盖，也不通过 `is_current` 标志切换 Current。
- 不在 Project 或其他表中维护可变 Current 指针；Current 是 `project_id` 范围内最高版本的确定性投影。
- 数据库通过 Project 外键禁止孤立 Snapshot，通过 `UNIQUE(project_id, version)` 防止同一 Project 出现重复版本。
- Application 读取当前版本并检查 `expected_version`，随后以单次 `INSERT` 写入下一版本。若版本已陈旧，或并发写入触发 `(project_id, version)` 唯一冲突，则回读最新版本并返回并发冲突，不覆盖较新的事实。
- Domain 校验 Snapshot 内容；`next_steps` 最多三项，成果、阻塞和下一步使用结构化字符串列表。
- SQLite 将三个列表编码为 JSON 文本；该存储格式不泄漏到 React，前端只使用类型化 DTO。
- `confirmed_at` 记录用户对该版本事实的确认时间。D1.2 不允许 AI、Git/GitHub 或其他自动化代替人工确认。
- 已归档 Project 及其 Snapshot 历史可读取，但不能继续创建新版本。
- 测试所需的项目和 Snapshot 由测试或人工验收流程创建，生产启动不注入示例数据。

## 优点

- 已确认事实不会因后续编辑而消失，能够支撑可靠复盘和状态演变追溯。
- Current 的定义只有一个，不存在版本记录与可变指针不一致的问题。
- 乐观并发检查可阻止陈旧页面、多窗口或未来自动化静默覆盖新事实。
- 每个版本保存完整状态，读取 Current 和任意历史版本时不需要重放事件或合并补丁。
- Project 外键继续保持跨项目隔离，避免 Snapshot 上下文污染。
- “当前目标、成果、阻塞、三个下一步、确认时间”的结构能直接服务 60 秒上下文恢复。
- 不引入任务字段和看板语义，避免 ForgeOS 退化为普通任务管理工具。
- 未来 Context Pack、复盘和风险分析可以明确引用某个已确认版本，而不是依赖易变文本。

## 缺点

- 每次小修改也会复制完整 Snapshot 内容，数据库行数和文本存储会持续增长。
- 更新流程比单行 `UPDATE` 更复杂，需要版本读取、唯一约束和冲突处理。
- Current 查询需要按 Project 查找最高版本；必须使用合适索引保持读取稳定。
- `updated_at` 对不可变行表示该版本写入时间，不表示之后还会发生原地更新，命名语义需要在文档中保持清楚。
- JSON 文本列表无法直接获得关系表级的项目约束；合法 JSON、字符串元素与最多三个下一步主要依赖 Domain 校验。
- 完整版本记录描述“确认后的状态”，不是详细操作日志，不能替代未来的 Development Session 或 Timeline。
- 用户需要主动确认 Snapshot；D1.2 不自动判断内容是否过期或是否与代码仓库一致。

## 长期影响

- Snapshot 历史是正式研发记忆，不得由普通更新、Project 归档或未来清理流程静默删除。
- 所有 Snapshot 写入来源，包括未来 AI 建议或外部适配器，都必须经过相同 Domain、Application、版本检查与人工确认边界；AI 不能直接修改正式事实。
- Current Snapshot 始终由最高版本确定。若未来确实需要回滚，应该通过“基于历史内容创建一个新版本”表达，而不是移动指针或删除较新版本。
- Context Pack 应引用具体 Snapshot ID 与版本，并优先使用最新的人工确认版本；不能把未确认的 AI 推断冒充事实。
- Development Session、Decision、Problem、Evidence 与 Timeline 仍需在各自阶段建立独立领域模型，不能塞进 Snapshot 列表。
- 若未来需要对成果、阻塞或下一步做关系查询，应通过新 Migration 和 ADR 演进存储模型；不得回写已发布 Migration 或破坏既有 JSON 历史。
- Snapshot 新鲜度可以由 `confirmed_at` 推导并在未来用于风险提示，但 D1.2 不自动更新、不生成任务，也不把时间陈旧等同于事实错误。
- 版本增长与查询性能需要持续观察；在个人本地规模下优先保持模型可追溯和实现清晰，不提前引入事件存储或归档服务。
