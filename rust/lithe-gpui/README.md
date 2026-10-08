# `rust/lithe-gpui/` — 界面层（GPUI 宿主）

这个目录是**统一 GPUI 前端的界面层**：编辑器、主界面（外壳/工作台）、项目树面板、Git、
运行与调试、终端、设置等宿主 crate 都放这里。它按"从零重建"推进——不复用既有分支上
那批围绕 `gpui-kit` 长出来的宿主代码，只继承它的结论与教训。

**当前状态（2026-10-08）**：宿主还未开始写，因此这里**故意没有 Cargo.toml**。
一个没有成员的 workspace 不合法（`cargo metadata` 直接报错），所以不在第一个宿主 crate
落地前建空壳。第一个宿主 crate 落地时，在此建 workspace 根（`members = ["crates/*"]`、
`resolver = "3"`，`edition 2024` 成员要求）。

## 与组件库的关系

组件库在**平级的** `rust/lithe-ui/`（Ely GPUI Components 的 fork + Lithe 自己的组件），
是独立 workspace。界面层通过 path 依赖使用它：

```toml
lithe-ui = { path = "../../lithe-ui" }
```

分层与理由见
`.agents/notes/proposed/architecture/2026-10-08-unified-gpui-frontend-selection-and-layer-boundaries.md`。
