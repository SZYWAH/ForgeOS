# ADR-015：Codex Activity Intake 使用 App Server 只读协议

- 状态：已接受
- 日期：2026-07-13
- 范围：D6 Codex Activity Intake

## 背景

ForgeOS 需要观察与本地仓库相关的 Codex 研发活动，提取任务目标、轮次状态、最终结果、验证命令摘要和未完成计划。直接读取 `.codex` 目录下的 SQLite、JSONL 或 rollout 文件会绑定 Codex 内部存储实现，也容易越过隐私边界保存完整对话与 reasoning。

Codex App Server 提供稳定的任务查询协议，可以按工作目录列出任务，并在不恢复任务的情况下读取历史轮次。ForgeOS 只需要观察，不需要启动任务、执行命令或修改 Codex 元数据。

## 候选方案

1. 读取 `.codex` 内部数据库和 JSONL：实现直接，但依赖私有格式，升级风险和隐私风险高。
2. 解析 Codex CLI 文本输出：缺少稳定结构，难以可靠区分任务、轮次、命令和文件变化。
3. 通过 `codex app-server --listen stdio://` 使用只读方法白名单。

## 决策

选择方案三。

- 新增 `CodexActivitySource` Port，基础实现为 `CodexAppServerAdapter`。
- 每次同步按需启动 STDIO App Server，同步后关闭进程，不监听 HTTP/TCP 端口。
- 方法白名单固定为 `initialize`、`thread/list`、`thread/read`、`thread/goal/get`。
- Adapter 不暴露通用 request 方法给 Application，不允许 `turn/start`、`command/exec`、`thread/archive` 等写入或执行方法。
- Project 归属只通过已注册 Repository 的规范化绝对路径和精确 `cwd` 过滤确定，不根据标题或仓库名称猜测。
- 首次回溯限制为近 30 天、50 个根任务、500 个轮次、2MB 脱敏后结构化文本。
- 只保存结构化 Observation、来源 ID、状态、时间和 SHA-256，不保存原始响应、完整对话、reasoning、完整命令输出或完整 Diff。

## 优点

- 使用 Codex 官方协议边界，不依赖内部存储格式。
- 白名单、按需子进程和 STDIO 降低攻击面。
- 结构化摘要支持幂等、增量同步、项目隔离和可测试的 Fake Server。
- Codex 升级造成的协议差异集中在一个 Infrastructure Adapter。

## 缺点

- 需要处理子进程生命周期、JSONL 协议、超时和版本兼容。
- Codex App Server 不可用时，自动观察只能退化为 Git 和已有正式事实。
- 摘要化会主动舍弃部分对话细节，无法作为完整聊天档案。

## 长期影响

- 任何新增 App Server 方法必须经过单独安全审查；默认拒绝不在白名单中的方法。
- ForgeOS 不建立 `.codex` 私有文件兼容层。
- 若未来需要实时事件，只能作为新的只读增量能力设计，不能改变当前无执行、无写入边界。
