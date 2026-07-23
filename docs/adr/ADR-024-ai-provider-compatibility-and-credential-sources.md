# ADR-024：AI Provider 协议与凭据来源必须显式配置

- 状态：Accepted
- 日期：2026-07-23
- 决策范围：D9.2.1 主流 AI Provider 兼容层

## 背景

ForgeOS 最初只支持 OpenAI Chat Completions、Bearer Token 和 Windows Credential Manager。D9.2 已让 GitHub 分析可以由用户主动选择 AI 服务，但单一协议无法可靠连接 OpenAI Responses、Anthropic、Azure OpenAI、本地 Ollama/LM Studio 及不同鉴权方式的兼容接口。若由客户端猜测协议或在多个密钥来源间自动回退，会使失败原因、凭据边界和请求 Hash 失去确定性。

## 决策

1. Provider 显式保存协议、鉴权方式、凭据来源和是否需要密钥；调用时禁止自动猜测或跨来源回退。
2. 支持 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages、Azure OpenAI v1 和 OpenAI-compatible Chat Completions 五种固定协议。禁止任意请求模板和通用 HTTP 逃生口。
3. 凭据只能来自 Windows Credential Manager、指定环境变量或明确的无密钥模式。SQLite 只保存 Provider ID 或环境变量名称，不保存密钥值。
4. 远程 Endpoint 必须使用 HTTPS；HTTP 只允许 localhost、127.0.0.1 和 `[::1]`。HTTP 客户端禁止跟随重定向。
5. Provider 创建后继续默认停用。连接测试和业务调用都必须由用户主动触发；测试结果不保存 Prompt、响应正文或密钥。
6. 环境变量按 ForgeOS 当前进程读取，修改后需要重启应用。IPC 只返回存在或缺失状态。
7. D9.2 GitHub AI 增强继续只依赖 `AiProviderKernel`，不接触具体协议，也不获得 Tool Calling、Shell、GitHub 写入或 Credential 读取能力。
8. 错误响应限制为 2 MiB，截断并脱敏；原始请求、响应和 Prompt 不写入 SQLite 或日志。

## 结果

- 主流云端和本地接口可以通过同一治理边界接入。
- Provider、Model、版本和凭据配置变化继续形成独立 Request Hash，旧报告不重算。
- 密钥来源和失败原因对用户可见，但密钥值永远不会通过 IPC 回读。

## 不采用的方案

- 不支持任意 JSON 模板、任意 Header 集合或脚本适配器，因为它们会扩大凭据泄露和请求执行面。
- 不在凭据管理器和环境变量之间自动回退，因为隐式行为会掩盖错误配置。
- 不加入流式聊天、Agent、Tool Calling、后台定时调用或模型发现。
