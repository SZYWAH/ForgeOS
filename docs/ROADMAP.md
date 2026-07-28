# ForgeOS 路线图

## 已完成主线

| 阶段 | 状态 | 结果 |
|---|---|---|
| D0–D0.5 | 完成 | 架构基础、工程冻结与模块边界 |
| D1.1–D1.6 | 完成 | Project、Snapshot、Session、Evidence、Context Pack、AI Provider 与 AI Candidate |
| D2–D5 | 完成 | Git Evidence、MCP Read-only、Codex Task Proposal、GitHub Import 与 MCP Intelligence |
| D6 | 完成 | Codex Activity Intake、Project Pulse、内容治理与全局本地化 |
| D7 | 完成 | 研发记忆助手聚焦、统一研发历史与结构化成果 |
| D8 | 完成 | 全应用视觉系统与共享控件基础 |
| D9.0 | 完成 | GitHub Project Intelligence 架构冻结 |
| D9.1–D9.1.1 | 完成 | 公开仓库规则分析与报告质量冻结 |
| D9.2–D9.2.1 | 完成 | 来源治理的 AI 增强与主流 AI Provider 兼容 |
| D9.3 | 完成 | GitHub 分析报告人工导入 Project |
| D9.4 | 完成 | `v0.2.0` Windows Preview 发布 |
| D9.4.1 | 完成 | AI 服务工作台、共享表单契约与无头 Edge 视觉验收 |

## 当前状态

D9 功能主线已经闭环。D9.4.1 只修复 AI 服务页视觉、可访问性和回归验收，没有新增 Domain、Migration、IPC 或外部权限。

## 下一阶段

### D10.0 — GitHub Clone & Reproduction Architecture Freeze

状态：`待规划 / 未开始实现`

只允许完成：

- Clone、依赖安装、构建运行和二开之间的授权边界。
- 用户选择目录、仓库归属、目录冲突、磁盘预算、取消与失败清理合同。
- 第三方代码安全、项目隔离、进度恢复与页面状态设计。
- Git 与 GitHub 只读边界变化的 ADR 评审。

D10.0 不创建实现代码或 Migration；没有获得新的实施确认前，不进入 D10.1。

## 持续非目标

- 自动执行第三方代码。
- 自动安装依赖或运行构建。
- 自动 Commit、Push、Fork、PR 或 Merge。
- Agent、MCP 写入和 Codex 自动执行。
- 云端同步与后台 Windows Service。
