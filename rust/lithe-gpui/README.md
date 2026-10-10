# `rust/lithe-gpui/` — 界面层（GPUI 宿主）

这个目录是**统一 GPUI 前端的界面层**：编辑器、主界面（外壳/工作台）、项目树面板、Git、
运行与调试、终端、设置等宿主 crate 都在这里。

## 成员与依赖方向

- `crates/*`：12 个宿主 crate（`app` / `workbench` / `editor` / `explorer` / `git` / `java` /
  `terminal` / `settings` / `run` / `notify` / `shared` / `extensions`）。
- 组件库在**平级的** `rust/lithe-ui/`（Ely GPUI Components 的 fork + Lithe 自己的组件），
  是独立 workspace，通过 path 依赖引入：`lithe-ui = { path = "../../lithe-ui" }`。
- `assets/` 由 `crates/app` 用 `rust-embed` 编译期内嵌（`$CARGO_MANIFEST_DIR/../../assets`）。

## 当前状态（2026-10-10）

宿主 crate 是**从既有分支 `feat/lithe-gpui-components` 照搬过来的**，不是从零重写。
搬过来时只做了必要适配：组件库的依赖路径（`../lithe-gpui` → `../../lithe-ui`）与 14 处
`lithe_gpui::` 引用。

✅ **过渡依赖已经摘干净**：原先按切片迁往 `lithe-ui` 的那一批（窗口根 `Root`、
对话框/抽屉/通知的浮层挂载、菜单、按钮、输入框、图标、主题桥）全部落地，
`gpui-kit` 从所有 crate 的 `Cargo.toml` 与 workspace 的 profile 覆盖里删除 ——
代码、注释、构建声明里都不再有它。UI 测试也随之改从 gpui 自己的 debug selector
（`.debug_selector` + `VisualTestContext::debug_bounds`）断言，不再依赖任何"观察包装"。

## 工具

`tools/check-ui-px.mjs` 等脚本是从既有分支一起搬来的，**其中的路径与 crate 名仍是旧布局**
（它排除的 `VENDORED_CRATE = "lithe-gpui"` 现在已改名叫 `lithe-ui`），尚未适配。

分层与理由见
`.agents/notes/proposed/architecture/2026-10-08-unified-gpui-frontend-selection-and-layer-boundaries.md`。
