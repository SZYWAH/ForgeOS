# ADR-014：MCP Query Intelligence 采用只读确定性查询架构

- 状态：已接受
- 日期：2026-07-13
- 决策范围：ForgeOS MCP Intelligence Tools、规则型建议、Context Pack 增强与敏感信息保护
- 前置决策：ADR-007（Context Pack 与 AI 隔离）、ADR-009（Git 只读集成）、ADR-010（MCP STDIO 只读架构）

## 背景

D3 已建立独立的 ForgeOS MCP STDIO Server，通过专用 `McpQueryKernel` 和 SQLite read-only 连接向 Codex 等客户端公开 Project、Snapshot、Session、Evidence 与 Context Pack。随着 ForgeOS 积累 Git Evidence、AI Candidate 和更多研发记忆，客户端需要的不只是单表读取，还需要项目总览、研发历史、安全的 Git Activity 摘要、Evidence 关系以及可解释的下一步建议。

这些能力不能破坏 ForgeOS 的事实边界。MCP 是读取层，Codex 是代码工作执行者；MCP 不应获得修改正式事实、启动进程、执行 Shell/Git 命令或调用 AI 的能力。下一步建议也不能伪装成已经确认的项目事实。

## 候选方案

1. 在 MCP Tool 内直接查询 SQLite 并拼装结果：实现较快，但会把 SQL、Project 隔离、安全脱敏和排序规则复制到协议层，难以审计和复用。
2. 调用 AI 分析 Context Pack 并生成下一步建议：表达能力更强，但相同数据可能产生不同结果，也会混合确认事实与模型推断，并引入 Provider、Credential 和自动调用风险。
3. 扩展 `McpQueryKernel` 与 Query Repository Port：Application Query 负责归属验证、确定性排序、安全投影和固定规则，MCP 层只做协议转换；底层继续使用 SQLite read-only 连接。

## 决策

选择方案三。

- 数据访问链固定为 `forgeos-mcp → McpQueryKernel → Application Query → Repository Port → SQLite read-only connection`。
- 所有 Tool 必须接收 `project_id`，Kernel 在读取任何子域数据前验证 Project 属于当前 Workspace。
- 新增 Project Overview、Current State、Development History、Recent Git Activity、Evidence Summary 与 Recommended Next Steps 查询；保留 D3 只读工具兼容面。
- Recommended Next Steps 只使用 Current Snapshot、Open Session、Pending Candidate 和尚未确认的 Evidence，按固定优先级、稳定排序和去重规则生成。
- 规则输出必须标识为建议，不接受 Candidate，不修改 Snapshot、Session、Evidence 或任何其他正式事实。
- `build_context_pack` 在内存中按固定章节顺序渲染 Project Overview、Snapshot、Session、Evidence Summary 和 Git Activity，并计算稳定 SHA-256；不持久化 ContextExport。
- Git Activity 与 Evidence Summary 使用专用安全 DTO。不得返回 Evidence `source_uri`、Repository path、原始路径字段、API Key、Credential、Token 或 AI Provider Secret。
- `forgeos-mcp` 不持有 Mutation Kernel，不调用 AI，不启动 Agent、Codex 或 Shell，不执行任何 Git 命令，也不提供 HTTP/TCP 或写 Tool。
- 既有 Codex Execution Foundation 保留为历史记录能力，但本决策不扩展 Runner，不建立执行链路。

## 优点

- Project 隔离、排序、脱敏和推荐规则集中在 Application Query 层，便于测试和审计。
- 相同 ForgeOS 数据产生相同 Tool 结果、Context Pack 和哈希，适合 Codex 上下文恢复。
- 规则型建议可解释且不依赖网络、模型、Provider 或 Credential。
- MCP 协议层不包含 SQL 或领域规则，未来其他只读消费者可以复用 Query Kernel。
- Query-only API 与 SQLite read-only 连接继续形成双重写保护。

## 缺点

- 固定规则只能整理已有事实，不能替代人工技术判断或 AI 深度分析。
- 为避免泄露本地路径，Git Evidence 需要转换为有限的结构化摘要，部分原始细节不会通过 MCP 返回。
- 保留旧 Tool 兼容面会增加工具数量和回归测试范围。
- Query Port 需要覆盖 Evidence 关系、Open Session、Pending Candidate 和 Git Activity，接口范围比 D3 更大。

## 长期影响

- 新增 MCP Intelligence 能力必须继续使用只读 Query Use Case；不得向现有 Tool 偷渡 create、update、delete、execute 或 accept 行为。
- 若未来需要 AI 推荐，必须建立独立的显式调用、Candidate 治理与新 ADR，不能替换当前确定性规则后仍声称结果稳定。
- 若未来需要 MCP 写入，必须单独设计权限、人工确认、审计和事实版本控制，不得扩展当前 `McpQueryKernel`。
- 若未来需要返回本地路径或更详细来源，必须使用显式授权和默认关闭策略。
- Codex Execution、Agent、Shell、Git 自动化和后台任务继续属于独立方向，不由 MCP Intelligence Layer 引入。
