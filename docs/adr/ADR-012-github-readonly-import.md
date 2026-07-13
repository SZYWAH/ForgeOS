# ADR-012: GitHub 第一阶段采用只读导入而非持续同步

## 背景

ForgeOS 已能管理 Project、Snapshot、Session、Evidence 与本地 Git 事实，但既有软件项目通常已在 GitHub 积累 Repository 描述、README、Commit 和 Release。直接要求用户重新手工创建 Project 会丢失已有研发资产；持续同步又会引入冲突处理、后台任务、Webhook、凭据权限和“外部数据是否覆盖人工事实”等复杂问题。

## 候选方案

1. 手工复制 GitHub 信息，不实现 Connector。
2. 建立 GitHub 双向或持续自动同步。
3. 建立 GitHub 只读 Import Draft，由用户审核后一次性导入。

## 决策

选择方案 3。GitHub Connector 第一阶段仅执行只读 API 请求。外部数据先保存为 Import Draft；用户显式审核后，ForgeOS 才在单个事务中创建 Project 与 GitHub Evidence。Token 只存于 Windows Credential Manager。

## 优点

- 能接管已有项目，同时保留 ForgeOS 作为研发语义事实层的控制权。
- Draft 隔离外部数据，避免 API 结果直接覆盖人工维护的 Project。
- 没有后台同步、Webhook 和写权限，攻击面与故障面较小。
- 导入事务可保证 Project、Draft 与 Evidence 不出现半成品。
- GitHub Evidence 保留来源 URL 与对象身份，便于追溯。

## 缺点

- 导入后的 GitHub 信息不会自动刷新。
- 同一 Repository 当前只保留一个 Import Draft，重新导入需要未来独立设计。
- README 摘要是确定性截取，不具备 AI 语义总结能力。
- 私有仓库仍需要用户自行提供最小权限 Token。

## 长期影响

未来若增加同步，必须作为独立阶段设计来源版本、冲突策略、用户确认和撤销机制；不得把当前 Import 自动升级为后台覆盖流程。GitHub 写操作、Issue/PR 管理、Webhook、Codex 执行与 Agent 均不由本 ADR 授权。
