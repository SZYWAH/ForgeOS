# ForgeOS 当前状态

更新时间：2026-07-28

当前分支：`codex/ai-provider-workbench-polish`

发布基线：`v0.2.0`

## 当前阶段

D9.4.1 已完成本地实现和验证，正在进行功能分支交付收尾。下一阶段仅准备 D10.0 架构计划，不开始 D10 实现。

## 已完成

- `PASS`：Rust AI 协议测试改为 Header 名大小写无关解析，并独立校验 Bearer、`api-key`、`x-api-key` 与 Anthropic Version 的值；生产协议网关无差异。
- `PASS`：AI 服务页采用列表与详情/创建区工作台，空状态不再渲染无意义详情面。
- `PASS`：共享表单组件统一必填标记、Select、Checkbox、说明、错误和可访问性关联。
- `PASS`：Custom、DeepSeek、Ollama 和已配置服务状态已纳入真实页面无头测试夹具。
- `PASS`：Playwright 使用本机 Edge `msedge` Channel、`headless: true` 和 Tauri `mockIPC`；不连接真实 SQLite、Credential Manager 或 AI Provider。
- `PASS`：根目录生成的 `forgeos.exe`、`forgeos-mcp.exe` 和 `uninstall.exe` 通过精确规则忽略，文件本身未删除。

## 验证证据

- `PASS`：`npm run typecheck`。
- `PASS`：Vitest 13 个文件、68 项测试。
- `PASS`：Playwright 9 个视口/缩放项目全部完成几何验收；另有 900px 以下单列检查和 2 张截图基线，共 12 项执行通过、24 项按项目矩阵设计跳过。
- `PASS`：`npm run build`；仅保留既有的单 Chunk 体积提示。
- `PASS`：Desktop `cargo fmt --check`、`cargo check` 和完整 `cargo test`；库测试 106 项通过、1 项本机 Codex Smoke Test按设计忽略，集成测试通过，3 项真实 GitHub 网络 Smoke Test按设计忽略。
- `PASS`：MCP `cargo fmt --check` 与完整 `cargo test`；3 项只读测试通过。
- `PASS`：视觉基线人工检查确认表单分区、按钮边界、18px Checkbox、标签首行对齐和统一控件高度。
- `UNVERIFIED`：本轮按要求不启动可见 Tauri 窗口，因此原生窗口标题栏和 WebView2 宿主层视觉未验收；页面内容使用同 Chromium/WebView2 渲染内核的无头 Edge 验收。

## 风险与注意事项

- `npm install` 当前报告依赖图存在 3 个 high severity advisory；本轮没有运行会改变依赖版本的 `npm audit fix`，应在独立依赖审计任务中处理。
- Playwright 截图基线是 Windows + Edge 基线；不同操作系统或字体环境不应直接复用。
- 当前分支只允许普通快进推送到同名远程功能分支，不合并 `master`、不移动 `v0.2.0` Tag、不创建 PR 或 Release。

## 下一步

1. 完成本阶段文档提交和功能分支普通推送。
2. 输出 D10.0 GitHub Clone & Reproduction Architecture Freeze 的决策完整计划。
3. 等待用户确认后再决定是否开始 D10.0 文档冻结；不得直接实现 Clone 或执行能力。
