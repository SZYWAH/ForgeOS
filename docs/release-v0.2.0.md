# ForgeOS v0.2.0 Preview

ForgeOS v0.2.0 是面向 Windows x64 的公开预览版，继续提供本地优先的个人研发记忆，并新增公开 GitHub 项目的可信分析与人工导入闭环。

## 本版新增

- 对公开 GitHub Repository 进行带来源引用的本地规则分析。
- 用户主动选择 AI 服务后，对规则报告进行来源约束的 AI 增强。
- 支持 OpenAI、DeepSeek、Anthropic、Azure OpenAI、Gemini、Mistral、Ollama、LM Studio 和自定义兼容接口。
- 将分析报告转换为完整、可编辑的项目草稿；只有人工审核后才创建 ForgeOS Project。
- 导入只沉淀 Repository 和可用的最新 Release 资料，不自动创建当前状态、开发记录或成果。
- 安装包继续内置只读 `forgeos-mcp.exe`，可在“应用设置 → Codex 连接”中一键配置。

## 安装与连接 Codex

1. 下载 `ForgeOS-0.2.0-windows-x64-setup.exe`。
2. 对照 `SHA256SUMS.txt` 验证文件。
3. 运行安装器并启动 ForgeOS。
4. 打开“应用设置 → Codex 连接”，点击连接。
5. 重启 Codex，使 `forgeos` MCP 生效。

ForgeOS MCP 使用 STDIO 按需启动，只读取 ForgeOS 保存的项目上下文。它不会执行代码、修改 Git、写入 GitHub 或修改 ForgeOS 数据。

## GitHub 分析边界

- 当前只支持公开仓库。
- 不克隆、不构建、不运行仓库代码。
- 不执行 Fork、Push、PR 或其他 GitHub 写操作。
- 静态报告不会声称已经运行或验证项目。
- AI 只能增强已通过规则分析与来源校验的解释性章节。
- 导入项目必须经过完整草稿审核，不会自动改变正式研发状态。

## 已知限制

- 这是未签名的预览版，Windows SmartScreen 可能显示“未知发布者”。
- 仅支持 Windows x64 当前用户安装。
- 目标电脑缺少 WebView2 时，安装过程需要联网下载 Microsoft Bootstrapper。
- 不提供自动更新器。
- 不单独发布 MCP 二进制。
- 卸载默认保留 ForgeOS SQLite 数据，方便以后重新安装。

## 发布产物

- `ForgeOS-0.2.0-windows-x64-setup.exe`
- `SHA256SUMS.txt`

安装包目标大小不超过 30 MiB。

## 维护者构建

```powershell
npm run typecheck
npm run test -- --run
npm run build

cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml

cargo fmt --check --manifest-path forgeos-mcp/Cargo.toml
cargo test --manifest-path forgeos-mcp/Cargo.toml

npm run release:windows
```

构建入口会：

1. 校验前端、Tauri、Desktop Rust、MCP Rust 及 Lockfile 版本均为 `0.2.0`。
2. 构建并校验 x64 MCP Sidecar。
3. 构建 NSIS 当前用户安装包。
4. 验证安装器中的 Sidecar 与新构建二进制 Hash 一致。
5. 输出规范文件名和 SHA-256。
6. 拒绝超过 30 MiB 的安装包。

## 安装与升级验收

全新安装、同版本重装和普通卸载：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-windows-installer.ps1
```

真实 `v0.1.0 → v0.2.0` 升级：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-windows-upgrade.ps1
```

两项测试都使用隔离的安装目录和 `CODEX_HOME`，不会修改用户真实 Codex 配置。升级测试只复制真实数据库及其 WAL/SHM 到隔离目录，并通过安装版验证 Migration 21、数据库完整性和 MCP 连接保留；它不会修改真实用户数据库。

## 本次发布验收

- 前端类型检查、63 项 Vitest 测试和生产构建通过。
- Desktop Rust 格式、检查和全量测试通过：106 项库测试通过，1 项本机 Codex 历史 Smoke Test 按设计忽略；数据库与领域集成测试全部通过。
- MCP 格式和三项只读测试通过，默认 STDIO 工具面仍为五个只读项目记忆工具。
- 全新安装、同版本重装、普通卸载和自有 Codex MCP 配置清理通过。
- 使用已发布的 v0.1.0 安装包完成真实 v0.1.0→v0.2.0 升级，升级期间 MCP 连接保持。
- 真实数据库隔离副本保持 Migration 21，`integrity_check = ok`，`foreign_key_check = 0`。
- 最终安装包大小为 8.52 MiB，未签名。
- 安装包 SHA-256：

```text
f1f67150981c21682fb18d862fc50da7a794c3b17f9402e5640277f0d02b95e6
```
