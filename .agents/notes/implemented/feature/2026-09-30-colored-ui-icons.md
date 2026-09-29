# Agent 笔记：UI 图标走彩色渲染（`img()` 而不是单色 `svg()`）

状态：已实现

## 先说结论

IntelliJ `expui` 那套 UI 图标**本身是彩色的**，而界面里一直画成灰的 —— 因为 UI 图标走的是
`svg()` 管线（渲成 alpha 遮罩 + 用 `text_color` 上**单色**）。

现在改成：**expui 的 UI 图标走 `img()`**（与文件图标同一条管线，原色保留），
Lucide 那些单色字形继续走 `.icon(..)`（它们本来就没有颜色可保留）。

开发者要记住的三条：

1. **彩色图标不跟随 `text_color`**。悬停 / 选中的反馈靠**底色**（`ButtonCustomVariant::hover`、
   `Button::selected`），与 IDEA New UI 一致；不要再写"图标变色表示选中"的样式 —— 对 expui 不生效。
2. **要用哪个 helper**：`shared::icons::idea_icon_image(icon, cx, side)`（彩色）；
   只有确实想要单色时才用 `idea_icon_svg` / `idea_icon_svg_px`。
3. `idea_icon_image` **自己兜底**：资产取不到时回落到单色版本，所以调用点直接
   `.child(idea_icon_image(..))`，不必判空。

## 问题

维护者口径（2026-09-30）："界面图标怎么还是单色的？" 查证结果：

- `lithe-gpui/assets/ui-icons/idea/**` 的 **157 个 SVG，0 个**用 `currentColor`，
  全是硬编码色值（`#3574F0` 蓝、`#E66D17` 橙、`#FFAF0F` 琥珀、`#55A76A` 绿……），
  而且每个图标都有 `light` / `dark` 两套（深色下另一组色值，由 `IdeaIcon::path(is_dark)` 选）。
- 而 `svg()` 这条管线把 SVG 当**形状遮罩**用：`SvgRenderer::render_alpha_mask` + 单一
  `style.text.color`（源码依据见 `shared/src/icons/file_icon.rs` 的 `theme_file_icon` 文档）。
  于是所有色值被抹掉，界面上就是"图标全是灰的"。
- **文件图标没有这个问题**：它们 2026-09-29 已经改走 `img()`（`FileIcon::render`），
  所以 `.java` 橙、`pom.xml` 蓝是彩色的。UI 图标那一批没跟着改。

## 决策

### 1. 新 helper `idea_icon_image`（`shared/src/icons/mod.rs`）

```rust
pub fn idea_icon_image(icon: &idea::IdeaIcon, cx: &App, side: impl Into<Length>) -> AnyElement
```

- 明暗选择沿用 `IdeaIcon::path(is_dark)`（与单色版本同一份判据）；
- 渲染**复用** `file_icon::theme_file_icon`（同一个 `img()` 实现、同一份"资源在不在"的检查），
  不另写一条 —— 两条彩色路径各写一份迟早漂移；
- 取不到资产时回落到 `idea_icon_svg_px`：**不画空白**，"找不到图"不该变成"少一块界面"。

### 2. 换掉的地方（本批）

| 位置 | 图标 | 尺寸 |
| --- | --- | --- |
| 左/右活动栏轨道项（`workbench/src/activity_bar.rs`） | 项目 / 更改 / 搜索 / 诊断 / 设置 / 通知 | `rems(1.)` = 16 |
| 项目标签头的文件夹（`workbench/src/project_tabs.rs`） | `FOLDER_ICON` | `rems(14. / 16.)` |
| 设置对话框头部（`settings/src/dialog.rs`） | `GEAR_ICON` | `rems(1.)` = 16 |

活动栏的 `ActivityIcon` 因此从 `button_icon(&self) -> Icon` 改成
`apply(&self, button: Button, cx) -> Button`：expui 走 `.child(idea_icon_image(..))`、
Lucide 走 `.icon(..)`（两条路各自用对的管线）。**几何与状态样式一行没动** ——
悬停与选中本来就是底色（`ButtonCustomVariant::hover` / `Button::selected`），
所以这次只换图标颜色，交互外观不变。

### 3. 状态栏**没换**（有意，附原因）

`workbench/src/status_bar.rs` 的 `entry_icon` 现在仍是 `svg()` + `text_color(muted_foreground)`。
那里的注释记着一次**真实失败**：早期用 `img()` 时状态栏那一格整块空白
（`statuszoom-before/after.png` 对比），当时归因于"`Img::request_layout` 只有拿到
`GlobalElementId` 才 `use_data`"。

那份归因后来被 `file_icon.rs` 的复核**推翻**了（那个 `global_id.is_some()` 只门控多帧动画续帧，
图像数据在 paint 阶段取，不需要全局 id），但**本批没有像素级验证手段**
（这台机器抓不到 gpui 的 GPU 帧，见 `.artifacts/probe/NOTES.md`），
所以在"能亲眼确认状态栏图标没变空白"之前不动它 —— 换它是一条 5 行的改动，
风险却是"状态栏图标整块消失"。状态栏那几个图标也因此仍是单色（含本应是绿色的
`successDialog` 对勾）。接手照 `S1_*` / 截图复核后可直接改成 `idea_icon_image`。

## 考虑过的备选方案

- **给每个状态各备一套彩色资产**（默认 / 悬停 / 选中）：能同时保住"彩色 + 图标随状态变色"，
  但 `assets/ui-icons/idea/**` 只有明暗两套，另外两套要么自己调色重导、要么改资产来源；
  而 IDEA 本身也不是这么做的（它的轨道图标不随悬停变色）—— 被否。
- **把彩色图标渲成图像后再 `text_color` 染色**：那又回到单色，等于没做。
- **自己预渲染 `RenderImage` 并缓存**（`cx.svg_renderer().render_single_frame(bytes, scale)`）：
  能同时解决"原色 + 任意 DPI 锐利"，但要自建缓存与失效逻辑；`file_icon.rs` 已把它登记为
  后续优化，本批不改（见下条代价）。
- **只改活动栏、别处不动**：会得到"轨道彩色、状态栏与标签头灰"的割裂界面 ——
  所以把项目标签与设置头部一起换了（它们没有历史失败记录），只有状态栏按上面那条留着。

## 后果

收益：

- expui UI 图标恢复真源颜色（活动栏的项目 / 更改 / 搜索 / 诊断 / 设置 / 通知、
  项目标签的文件夹、设置头部的齿轮）。
- 彩色与单色两条路各有**一个**入口 helper，调用点选错的代价是"颜色不对"，不是"画不出来"
  （兜底在 helper 内部）。

代价与例外：

- **`img()` 固定按 `scale_factor = 1.0` 栅格化**（`gpui-pre/elements/img.rs` 里上游留了
  `TODO: Can we make SVGs always rescale?`），所以 125% / 150% DPI 下是把 16×16 位图放大，
  边缘比矢量遮罩**略软**。文件图标从 2026-09-29 起就是这个取舍（原色优先）。
- **彩色图标不再参与主题前景色**：深色主题下靠 `_dark` 变体，浅色下靠 `_light` 变体；
  如果将来加"高对比度"主题，需要额外准备资产（当前主题集没有这一档）。
- 状态栏仍是单色（原因见上）。

## 验证

- `cargo test -p lithe-gpui-shared --lib`（64）、`-p lithe-gpui-workbench --lib`（83）、
  `-p lithe-gpui-settings --lib`（169）、`cargo fmt --check`、
  `node rust/lithe-gpui/tools/check-ui-px.mjs`（仅剩 terminal 的 2 处既有违规）、
  `cargo build --bin Lithe`。
- **未做像素级验证**：本机抓不到 gpui 的 GPU 帧（`PrintWindow` 纯黑、`CopyFromScreen` 只有底色，
  见 `.artifacts/probe/NOTES.md`）。所以"图标真的画出彩色"这一条要靠维护者看一眼：
  活动栏「项目 / 更改 / 搜索 / 诊断 / 设置」应恢复成 IDEA 的彩色图标
  （诊断那一个是彩色警告三角，通知是彩色铃铛）。
- 已知可回退点：`idea_icon_image` 内部就是 `theme_file_icon` + 单色兜底，
  要退回全单色只需把三处调用点换回 `idea_icon_svg*`。

## 适用范围

- `rust/lithe-gpui/crates/shared/src/icons/mod.rs`（`idea_icon_image`）
- `rust/lithe-gpui/crates/shared/src/icons/file_icon.rs`（`theme_file_icon` 的管线依据）
- `rust/lithe-gpui/crates/workbench/src/activity_bar.rs`、`project_tabs.rs`
- `rust/lithe-gpui/crates/settings/src/dialog.rs`
- 待办：`rust/lithe-gpui/crates/workbench/src/status_bar.rs`（`entry_icon`）
