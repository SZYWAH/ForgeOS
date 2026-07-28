# ForgeOS 决策索引

## 事实来源顺序

1. `docs/PROJECT.md`：长期目标、范围和非目标。
2. `docs/ROADMAP.md`：阶段状态与下一阶段边界。
3. `docs/STATUS.md`：当前实现、验证、风险和下一步。
4. `docs/architecture.md`：当前系统架构事实。
5. `docs/adr/`：不可随意改变的架构决策。

发生冲突时，先更新对应 ADR，再同步 Architecture、Roadmap 和 Status，不能只改页面或代码。

## 当前关键决策

- ADR-012：GitHub 第一阶段采用只读导入与人工确认，不进行写操作或后台同步。
- ADR-018：ForgeOS 聚焦个人多项目研发记忆，Candidate、Task Proposal 与 Execution 退出主流程。
- ADR-020：研发记忆与 AI Context 必须经过统一内容治理，不重写历史事实。
- ADR-022：GitHub Project Intelligence 的 Analysis、Import Draft 与 Project 生命周期分离。
- ADR-025：GitHub 分析报告只能通过完整草稿和人工审核进入 Project。
- MCP 始终只读；Codex、AI Provider 和外部仓库内容不能获得 Shell、Git 写入或正式事实写入能力。
- D9.4.1 是纯前端视觉与测试基础设施调整，不需要新 ADR，也不改变数据库、Rust Domain 或 IPC。

## 待决策

D10.0 必须先形成新的 Clone & Reproduction ADR，至少冻结：

- Clone、依赖安装、构建运行和二开的逐级授权。
- 本地目录所有权、冲突、磁盘预算、取消和失败清理。
- 第三方代码默认不执行及 Sandbox 边界。
- 与现有 Project、Repository、GitHub Analysis 和研发记忆的归属关系。

在该 ADR 获得确认前，不创建 D10 实现代码或 Migration。
