# `rust/lithe-gpui/` — 界面层（GPUI 宿主）

这个目录是**统一 GPUI 前端的界面层**：编辑器、主界面（外壳/工作台）、项目树面板、Git、
运行与调试、终端、设置等宿主 crate 都在这里。

## 成员与依赖方向

- `crates/*`：12 个宿主 crate（`app` / `workbench` / `editor` / `explorer` / `git` / `java` /
  `terminal` / `settings` / `run` / `notify` / `shared` / `extensions`）。
- 组件库在**平级的** `rust/lithe-ui/`（Ely GPUI Components 的 fork + Lithe 自己的组件），
  是独立 workspace，通过 path 依赖引入：`lithe-ui = { path = "../../lithe-ui" }`。
- `assets/` 由 `crates/app` 用 `rust-embed` 编译期内嵌（`$CARGO_MANIFEST_DIR/../../assets`）。

## 当前状态（2026-10-08）

宿主 crate 是**从既有分支 `feat/lithe-gpui-components` 照搬过来的**，不是从零重写。
搬过来时只做了必要适配：组件库的依赖路径（`../lithe-gpui` → `../../lithe-ui`）与 14 处
`lithe_gpui::` 引用。

⚠️ **`gpui-kit` 是过渡依赖**：搬过来的宿主里仍有约 **429 行**引用 `gpui-kit`
（`workbench` 196、`settings` 76、`editor` 48、`git` 29、`terminal` 22、`shared` 19、
`explorer` 17、`app` 15、`run` 5、`notify` 2；`java` 与 `extensions` 为 0），
其中包含窗口根 `Root`（对话框/抽屉/通知的统一浮层挂载）。它**正在按切片迁往 `lithe-ui`**，
退场后 `Cargo.toml` 里的 `gpui-kit` 与 workspace 里的对应 profile 覆盖一并删除。

## 工具

`tools/check-ui-px.mjs` 等脚本是从既有分支一起搬来的，**其中的路径与 crate 名仍是旧布局**
（它排除的 `VENDORED_CRATE = "lithe-gpui"` 现在已改名叫 `lithe-ui`），尚未适配。

分层与理由见
`.agents/notes/proposed/architecture/2026-10-08-unified-gpui-frontend-selection-and-layer-boundaries.md`。
