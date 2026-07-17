# ForgeOS Visual System

ForgeOS 使用面向 Windows 桌面工程工具的原生 CSS 视觉系统。它参考 Fluent 2 的控件纪律与 Primer 的开发者工具密度，但不声明为官方 Fluent 或 Primer 实现。

## Design Read

- 产品类型：个人研发记忆工作台。
- 使用者：同时维护多个项目的独立开发者。
- 视觉语言：冷静、紧凑、可信、文档优先。
- 参数：`DESIGN_VARIANCE 3`、`MOTION_INTENSITY 1`、`VISUAL_DENSITY 7`。

## Contract

- 界面只使用 `tokens.css` 中声明的颜色、字体、间距、圆角、阴影和动效变量。
- 品牌蓝是唯一操作强调色；成功、警告和失败色只表达状态。
- 界面字体为 Segoe UI Variable / Segoe UI；代码、Hash 和 Commit 标识使用 Cascadia Code / Consolas。
- 控件圆角为 6px，内容面圆角为 8px，胶囊圆角只用于状态标签。
- 每页首屏最多一个主操作；卡片只用于真实信息层级，不用于填补空白。
- 空列表必须显示单一完整空状态，不保留无意义详情面板。
- 页面级滚动只发生在 `.content-scroll`；长正文、记录列表和代码预览可以局部滚动。
- 新页面优先复用 `AppButton`、`Surface`、`StatusBadge`、`Notice`、`EmptyState`、`ListDetailLayout` 与既有内容治理组件。
- 新样式必须进入明确的 Cascade Layer，禁止继续向单体 CSS 追加阶段性覆盖。
- 动效只用于 Hover、Focus、Active、Loading 与浮层反馈，并遵守 `prefers-reduced-motion`。

## Page Patterns

- Index：页头、状态摘要、连续记录列表、加载更多。
- Workbench：紧凑页头、一个核心内容面、工程状态带、辅助记录。
- List Detail：有数据时使用 280–312px 列表加自适应详情；无数据时退化为单一空状态。
- Form：字段按语义分组，正文不超过 760px，危险操作独立呈现。

## Verification

- 自动检查 Token 外硬编码颜色、共享组件状态与空列表详情行为。
- 实机检查 960×720、1280×820、1920×1080，以及 Windows 100%、125%、150% 缩放。
- 中文和英文均需覆盖空、加载、错误、有数据和超长内容状态。
