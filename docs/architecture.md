# ForgeOS 总体架构

## 架构目标

ForgeOS 采用本地优先的模块化单体架构。

D0 的目标不是实现完整业务，而是建立可以长期扩展且边界清晰的基础：

- React 只负责界面和交互
- Rust 负责应用用例、领域规则和数据访问
- SQLite 保存结构化事实
- 文件系统负责未来的附件和导出文件
- 所有外部能力通过明确的端口接入

## 总体结构

```text
React + TypeScript UI
        │
        ▼
Typed Tauri IPC
        │
        ▼
Rust IPC Layer
        │
        ▼
Application Use Cases
        │
        ├── Domain
        └── Ports
              ▲
              │
SQLite / File System / Future Adapters
```

## 前端职责

前端负责：

- 页面呈现
- 用户输入
- 即时表单校验
- 调用经过定义的 Tauri Command
- 展示 Rust 返回的只读数据和错误

前端不得：

- 直接访问 SQLite
- 绕过 Rust 修改正式数据
- 在 `localStorage` 保存正式领域事实
- 实现与 Rust 重复的核心业务状态机

## Rust 模块职责

### `bootstrap`

负责应用启动和依赖组装：

- 解析数据目录
- 初始化日志
- 建立 SQLite 连接
- 执行 Migration
- 注册 Tauri Commands
- 组装基础设施实现

### `ipc`

负责 Tauri 边界：

- Commands
- 输入与输出 DTO
- IPC 参数校验
- 错误转换

IPC 不直接执行 SQL，也不保存业务规则。

### `application`

负责应用用例编排：

- 调用领域规则
- 控制事务边界
- 调用端口
- 返回用例结果

D0 只保留模块边界，不实现完整业务用例。

### `domain`

负责领域对象和不变量。

未来承载：

- Project
- Current Snapshot
- Development Session
- Decision
- Problem
- Evidence

Domain 不依赖 Tauri、SQLx、文件系统或具体 AI SDK。

### `ports`

定义应用依赖的抽象能力，例如：

- Repository
- Clock
- ID Generator
- File Store
- Context Renderer
- AI Provider

D0 只增加当前确实需要的端口，不提前建立空泛抽象。

### `infrastructure`

实现技术细节：

- SQLite 连接
- Migration
- Repository 实现
- 文件系统
- 日志和配置
- 后续 Git、AI 等适配器

### `read_models`

负责面向界面的查询模型。

未来可承载 Dashboard、Project Overview 和 Search 投影。D0 只建立边界。

### `jobs`

负责未来可取消、可审计的后台任务，例如 Git 刷新和 AI 分析。D0 不运行后台业务任务。

## 依赖规则

固定依赖方向：

```text
IPC → Application → Domain
Application → Ports
Infrastructure → Ports
Bootstrap → 各模块组装
```

禁止：

- Domain 引用 Tauri 或 SQLx
- IPC 直接执行 SQL
- React 直接读写数据库
- Infrastructure 反向依赖 UI
- AI Provider 直接修改领域表
- 模块之间通过全局可变状态交换业务数据

## 启动数据流

```text
启动 Tauri
→ bootstrap 确定数据目录
→ 建立 SQLite 连接
→ 执行 Migration
→ 进行数据库健康检查
→ 注册 Commands
→ 创建桌面窗口
→ React 显示 Hello ForgeOS
```

如果迁移失败，应用必须返回清晰错误，不得假装正常启动。

## 普通请求数据流

```text
React
→ Typed IPC
→ IPC Command
→ Application Use Case
→ Domain Validation
→ Port
→ Infrastructure
→ SQLite Transaction
→ DTO
→ React
```

## D0 实施边界

D0 实现：

- Hello ForgeOS 窗口
- 基础布局
- Rust 模块目录
- SQLite 连接
- Migration 执行
- Rust 数据库访问验证

D0 不实现：

- 业务页面
- 完整领域模型
- AI Provider
- Git/GitHub
- Codex 集成
- 后台任务
- 完整搜索与附件系统

## 后续扩展原则

新增能力必须从 Application Use Case 进入，不允许从 UI、AI 或外部集成直接写数据库。

若未来拆出 MCP 进程，应复用领域与应用核心，而不是复制一套业务规则。
