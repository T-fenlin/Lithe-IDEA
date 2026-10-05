# Agent 笔记：lithe-gpui 自有组件库与 Ely 视觉移植

状态：已实现

## 先说结论

Lithe 有了自己的 GPUI 组件库 `lithe-gpui`（位于 `rust/lithe-gpui/crates/lithe-gpui`），
视觉值移植自 Ely GPUI Components，行为层建在 gpui-kit 的 `gpui-base` 上，不再引入
gpui-component。最终目标是全量替换宿主里的 gpui-component；迁移在那之前不动现有
调用点。日常开发中，新界面一律优先用 `lithe_gpgi::*` 的组件；颜色一律读
`cx.theme()`，不许写死色值。

## 问题

- 宿主 48 个文件直接消费 gpui-component 的默认观感，面板、git 等界面样式不可控，
  且功能视图（git 面板）写死在 feature crate 里，别处无法复用。
- 需要一个大规模、可对账的组件目录（对齐 Ely 44 章框架、千级变体），而不是零散的
  页面级代码。
- gpui-component 无法满足"Ely 风格"：它的样式与其组件形态耦合，只能整体接受。

## 决策

- **分层**：`lithe-gpui` 只依赖 `gpui-kit`（`default-features = false`），只用其
  `gpui-base` 行为层；gpui 本体经 `lithe_gpgi::*` 再导出。库是纯库：不依赖任何
  lithe crate，图标（调用方注入 SVG 资产路径）与字体都不内嵌。
- **视觉**：色值逐项移植 Ely `palette.rs`（MIT OR Apache-2.0，出处写在
  `src/theme/palette.rs` 头注）；圆角刻度 Sm 4 / Md 6 / Lg 8 / Xl 12；Ely 未提供的
  hover/active 步进色按"light 朝暗、dark 朝亮"明度派生（非逐字移植，已在代码注明）。
  语义角色命名对齐 gpui-component 的 `ThemeColor`，宿主迁移时调用侧主题读取代码不变。
  字体不移植（保持系统 UI 字体）。Ely 的跨模式渐变动画未移植，`set_mode` 直接切换。
- **目录**：对齐 Ely 44 章框架但按范围取舍——通用章（1–15、35、38–40、43、44）与
  IDE 章（17–21、27、33–34）全做，纯演示章（Finance/Mail/Maps 等）跳过；章号沿用
  Ely 编号留洞。进度对账真源是 `CATALOG.md`，演示真源是 gallery 示例窗口。
- **测试**：UI 集成测试用 `#[gpui_kit::test]`（只需 dev 侧 `test-support`）。
  组件的可观测元素必须在 `.id()` 之后跟 `.test_support()`——gpui-base 的元素观测
  只登记显式包裹的元素，漏包会让测试报 "Registered paths: <none>"。
- **迁移**：目录推进期间宿主不动；首次迁移前把 Ely 语义色同步映射进现有
  gpui-component 主题以缓解观感分裂（延后决策，未实现）。

## 考虑过的备选方案

- **继续用 gpui-component + 定制主题**：被否。观感受组件形态限制（Ely 的单色
  accent、安静徽章等表达不出来），且组件目录广度无法自主扩展。
- **fork/移植 Ely 源码**：被否。Ely 绑定 zed 上游 gpui 的 git rev，与本仓库钉住的
  gpui-pre 0.3.7 不兼容；只有设计值可移植。
- **脱离 gpui-base 从零自研行为层**：被否。gpui-base 0.7.0 已含 Dialog/Toast/
  Tooltip/Tree/Table/Dock/motion 的 headless 全套，重造违背"复用成熟上游"原则，
  只在表现层（本 crate）做产品级工作。

## 后果

- 收益：组件目录自主可控、可对账；视觉统一到 Ely 语言；行为层复用上游，新组件
  成本主要是样式与测试；`ActiveTheme` 与 gpui-component 同名同义，迁移时调用侧
  改动集中在 import 行。
- 代价：共存期双栈（gpui-component + lithe-gpui）同时编译，dev 构建变慢（已在
  workspace profile 给 gpui-base 加 opt-level 3 缓解）；Ely 渐变动画缺失，模式
  切换是硬切；派生 hover/active 色不是 Ely 原值；Spinner 的 reduced-motion 联动
  推迟到动效章。

## 验证

- `cargo test -p lithe-gpui`：10 项全绿（5 单元 + 2 Primitives UI + 3 主题 UI）。
- `powershell .agents/skills/write-stable-tests/scripts/verify-test-stability.ps1`：通过。
- timing 报告：`.artifacts/test-stability/lithe-gpui.json`（最慢 173ms）。
- gallery 冒烟：`cargo run -p lithe-gpui --example gallery` 正常开启、干净退出。

## 适用范围

- `rust/lithe-gpui/crates/lithe-gpui/**`（库本体、CATALOG.md、gallery 示例）
- `rust/Cargo.toml`（workspace 成员经 glob 纳管 + `[profile.dev.package.gpui-base]`）
- 组件 API 形状遵循 gpui-kit 编码指南（RenderOnce builder、`XState` 命名、
  `Sizable`/`Disableable` 等 trait 语义）；与本 Note 冲突时以代码 + CATALOG 为准。
