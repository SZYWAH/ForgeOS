# ADR-025：GitHub 分析必须经过人工项目导入

- 状态：Accepted
- 日期：2026-07-23
- 决策范围：D9.3 GitHub Analysis Project Import

## 背景

GitHub Analysis Report 只能说明一个公开仓库在特定 HEAD 上“是什么”。它不等于用户正在开发的项目，也不能证明用户当前做到哪里。若分析完成后直接创建 Project、Snapshot 或 Session，会把外部静态结论混入个人研发事实，并使不完整、过期或 AI 回退报告获得过高权限。

## 决策

1. Analysis Run、GitHub Import Draft、完整 Project Proposal 和 Project 使用独立生命周期。
2. 用户必须在专门审核页查看并修改所有项目字段，保存草稿、确认内容后才能创建 Project。
3. Proposal 永久绑定选定 Analysis Run 的 HEAD 和 Content Hash。更新的报告不得静默覆盖人工修改。
4. `partial` 与过期报告可以进入审核，但最终确认前必须分别明确知情；Blocked、Failed、运行中和 Archived 报告禁止导入。
5. 同一 GitHub Repository 只复用一个 Import Draft 和一个 Project。重复请求返回已有草稿或项目。
6. 最终导入使用单一数据库事务，创建 Project、标记 Draft、关联 Analysis，并创建仓库资料和可选的最新 Release 资料。
7. 导入不创建 Snapshot、Session、Achievement、本地 Repository 或 Commit Evidence。
8. 导入只读取已保存报告和来源，不重新调用 GitHub 或 AI。

## 结果

- 外部项目理解与个人研发事实保持清晰边界。
- 用户修改、来源版本、过期确认和不完整确认都可追溯。
- 事务失败不会留下半创建项目、孤立资料或错误关联。
- 导入后用户仍需主动连接本地仓库并同步 Codex/Git，才能形成当前研发状态。

## 不采用的方案

- 不在分析完成后自动创建 Project。
- 不用随机后缀静默解决 Project Slug 冲突。
- 不批量把 Commit 转成 Evidence。
- 不根据 GitHub 静态资料推断 Snapshot、Session 或 Achievement。
