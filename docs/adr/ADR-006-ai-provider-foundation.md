# ADR-006：AI Provider Foundation 与正式研发事实隔离

- 状态：已接受
- 日期：2026-07-11
- 决策范围：OpenAI-compatible Provider 配置、API Key 存储与显式调用边界
- 前置决策：ADR-001（Tauri + Rust + SQLite）、ADR-005（Evidence 独立模型）

## 背景

ForgeOS 长期需要 AI 辅助记录、分析与研发参谋能力，但当前可信研发记忆仍必须由用户确认。
在进入 AI 总结、Context Pack 或主动分析前，需要先建立稳定的 Provider 抽象，使本地桌面应用能够连接 OpenAI、DeepSeek、Azure OpenAI、Ollama、LM Studio 或其他兼容接口。

如果 Provider 配置直接散落在前端或业务用例中，后续会产生供应商锁定、API Key 泄漏、自动调用和 AI 结果越权写入正式事实的风险。Provider Foundation 因此必须先冻结连接与安全边界。

## 候选方案

### 方案一：为每个厂商实现独立 SDK

可以使用厂商特有能力，但会引入多套依赖、认证和错误模型，并使当前阶段过早承担模型能力差异。Ollama、LM Studio 和其他兼容服务也需要额外适配。

### 方案二：前端直接发送 HTTP 请求并保存 API Key

实现快速，但密钥会进入 WebView 状态或前端持久化，绕过 Rust Application/Ports，也无法统一执行启停、超时和错误映射。这违反现有架构依赖方向。

### 方案三：Rust OpenAI-compatible Port/Adapter 与操作系统凭据存储

SQLite 保存非敏感 Provider 元数据；API Key 通过独立 Port 进入 Windows Credential Manager；用户显式调用由 Application 检查 Enabled 状态后交给最小 Chat Completions Adapter。结果只返回调用者，不写业务表。

## 决策

选择方案三。

### Provider 配置

- Provider 只包含稳定 ID、名称、Base URL、Model、Enabled、时间和 version。
- 不在 Domain 中硬编码 OpenAI、DeepSeek、Ollama 等厂商枚举；兼容性由用户提供的 Base URL 和 Model 决定。
- Base URL 可以是 API 根路径，也可以是完整 `/chat/completions` 地址。
- 新 Provider 默认 Disabled，启停变更和配置更新受乐观并发保护。

### API Key

- API Key 不写 SQLite、日志、DTO 或前端持久化。
- Windows 使用 Credential Manager，服务名固定为 `ForgeOS AI Provider`，Provider ID 作为账户标识。
- 前端只能知道密钥是否已配置，不能读取已有明文；替换密钥必须提交新的值。
- 本地无认证服务允许不配置 API Key。

### 调用接口

- 当前只支持非流式 Chat Completions 文本请求，输入为 `system/user/assistant` 文本消息。
- 请求只包含 Model、Messages 与 `stream = false`，不提供 tools、function calling、Agent 或自动任务能力。
- HTTP Client 使用统一连接池、连接超时、总超时、系统代理和响应体大小上限。
- Disabled Provider 在进入 HTTP Adapter 前被拒绝。
- Prompt 与 Response 只存在于调用过程和当前页面内存，不创建调用历史表。

### 事实写入边界

AiProviderKernel 不持有 ProjectKernel，也不拥有 Project、Snapshot、Session 或 Evidence Repository Port。
调用成功不产生任何数据库 mutation。未来若 AI 输出需要成为建议或正式事实，必须新增独立、可审计且要求用户确认的用例。

## 优点

- 任意兼容端点使用同一配置和调用接口，避免当前阶段绑定特定 SDK。
- API Key 不进入 SQLite 或前端回读路径。
- Provider 启停和默认 Disabled 防止配置创建后被意外调用。
- AI 返回内容与可信研发事实保持硬隔离。
- Port/Adapter 结构允许未来更换凭据后端或增加专用 Provider Adapter。
- 本地 Ollama/LM Studio 可以在无 API Key 时使用。

## 缺点

- 只保证最小 Chat Completions 兼容，不覆盖 Responses API、流式输出、多模态或厂商扩展参数。
- 不同“兼容”服务仍可能返回非标准错误或响应结构。
- Windows Credential Manager 是当前平台实现，若扩展到其他桌面平台需要增加对应 Secret Store Adapter。
- Provider 删除、调用审计、速率限制和模型发现不在当前范围。
- 手动测试调用结果不会保留，关闭或切换页面后即丢失。

## 长期影响

- 自动调用、AI 总结、事实建议和 Agent 执行必须另立阶段，不得通过本 Foundation 偷渡。
- 新 Provider 能力应优先扩展 Port/Adapter，不能把 SDK 类型暴露给 Domain 或 IPC。
- 未来调用日志若确有需要，必须明确隐私、脱敏、保留期限和用户控制，不能默认记录 Prompt/API Key/Response。
- Context Pack 或研发分析只能读取经用户授权的 Project 上下文，并将 AI 输出保持为候选，不能直接写正式事实。
- MCP 不属于 Provider Foundation；若未来实现，必须继续复用 Application/Domain 边界。
