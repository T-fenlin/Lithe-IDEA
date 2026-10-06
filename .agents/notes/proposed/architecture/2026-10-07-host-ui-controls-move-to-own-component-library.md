# Agent 笔记：宿主界面控件迁往自己的组件库，Dock 先收回花活

状态：提议中

## 先说结论

宿主（`rust/lithe-gpui/crates/*`）的界面控件以**仓库内的组件库**
`crates/lithe-gpui`（Ely GPUI Components 的 vendored fork，由我们维护）为归宿；
外部发布的 `gpui-kit` 会逐步被换掉，最终不再依赖。这是长期方向，**本轮不排期
全量迁移**。

本轮只做第一个切片：把组件库 `layout/dock` 里**外壳没有使用的花活**（同侧两组、
最大化、浮动入口、视图注册表、已关闭面板条、图标栏、可配外观）撤掉，只留外壳
真正依赖的契约，并顺手把现在整片空白的宿主布局修回来。

开发者记住两件事：**新写的宿主界面控件优先用 `lithe_gpui::`**；但**不要顺手去改
别的 crate**——`gpui-kit` 的退场按切片走，不搞一次全仓替换。

## 问题

现在有三处矛盾，不写下来下一个人每碰一次界面就要重新决定一次。

1. **归宿没有写下来。** 宿主 12 个 crate 里 11 个仍全部依赖 `gpui-kit`
   （`app` / `editor` / `explorer` / `git` / `settings` / `terminal` / `notify` /
   `run` / `shared`），只有 `workbench` 用了自己的组件库，而且只用在一处
   （Dock）。"以后到底用哪个库"没有结论，于是每个新控件都是一次临时裁决。

2. **Dock 有一批没有消费方的能力。** 换装前，组件库的 `layout/dock` 被加了
   7 项"优化"和几项附加能力，但宿主（唯一真实消费方）**一项都没用**：
   同侧两组、最大化/还原、面板头的浮动入口、面板视图注册表、"已关闭面板"
   条、图标栏、可配边框/圆角/背景。它们唯一的消费者是组件库自己的画廊页
   （`examples/gallery/pages/layout/systems.rs`）与 `dock/mod.rs` 里的单测。
   能力先于需求存在，维护面全在组件库这一侧。

3. **宿主界面现在是坏的。** 外壳改用组件库 Dock 的那次换装，把三块工具窗塞进
   一个 `h_flex()`；而 `h_flex()` 是 `align-items: center`，子元素不会被撑到行高，
   Dock 的 `size_full()` 根于是按不确定高度求解，**整个中央列连编辑器一起塌成
   零**（`.artifacts/lithe-screen.png` 整片空白）。

## 提案

分两步：**先定归宿，再做第一个切片**。

### 归宿：宿主界面控件用组件库，`gpui-kit` 逐步退场

- 组件库是**我们维护的代码**（vendored fork，见
  `implemented/architecture/2026-10-05-lithe-gpui-vendored-from-ely.md`），不是
  外部依赖；宿主界面控件从它取。
- `gpui-kit` 的退场**按切片**：每个切片只处理一处真实用到的能力，做完可独立
  验证、可独立回退。需要时先登记"这个能力在组件库里还没有对等物"，而不是
  为了凑迁移去手写一份。
- **主题桥是过渡措施，也要退场。** 宿主现在把 gpui-kit 的主题颜色"映射"进组件库
  自己的主题全局（`crates/workbench/src/component_theme.rs`：组合根启动时装一次，
  并 `observe_global` 宿主的 gpui-kit 主题，宿主换主题就重映射一次）。方向是
  **组件库自己的主题成为真源**、宿主直接读它，而不是两边各存一份再手工对齐。
  本轮不动它，登记为后续切片——**控件迁完不等于主题对齐完**，这是两件事。
- 本文只**登记方向**，不给全量迁移的排期与顺序。真要启动时需要单独一篇 Note
  说明：哪些 crate 先动、`gpui-kit` 的只读能力（编辑引擎 `EditorState`、`Tree`、
  `DataTable`、窗口根 `Root` 浮层、主题与主题桥、i18n、通知）各自的对等物从哪来。

### 本轮切片：Dock 收回花活

（本切片已落地；结果与验收证据记在
`implemented/architecture/2026-10-05-shell-tool-windows-on-own-dock.md` 的「2026-10-07 修订」。）

外壳对 Dock 的依赖**只有**下面这些；组件库的 Dock 缩到"刚好够用"。

**撤（花活，出处见括号）**：

- 同侧两组：`GROUPS`、`add_to`、`move_to_group`、`set_split`、`is_active_in`
  （优化 4）；
- 最大化/还原：`zoom`、`unzoom`、`toggle_zoom`、`is_zoomed`（优化 5）；
- 面板头操作（关闭 / 浮动 / 最大化三个按钮）与 `toggle_float`（优化 6）；
- 面板视图注册表：`PanelRegistry`、`from_registry`（优化 7）；
- "已关闭面板"条：`closed_strip`；
- 图标栏：`rail`；
- 可配外观：`DockLook`、`PanelLook`、`PaintedLook`、`look`、`painted`、`border`、
  `radius`、`background`、`border_color`、`handle`、`gap`（含对外暴露的 `header()`；
  `plain` 内部仍关标签头）。

**留（外壳真正在用）**：

- 开合：`is_hidden`、`show`、`hide`、`toggle`（优化 1）；
- 面板标题 i18n：`PanelTitle` 与 `resolve`（优化 2）；
- 每边尺寸上下限：`SideLimits`、`set_side_limits`、`set_side_limits_with`、
  `side_limits`、`clamp`、`clamp_in`、`cap`（优化 3）；
- 无标签头面板：`DockPanel::plain`；
- 尺寸持久化要用到的 `size`、`set_size`、`DockEvent::Resized`；
- Ely 原版：`new`、`add`、`activate`、`move_panel`、`layout`、`restore`。

**两个边界项的结论**：

- `Spot::Floating` **保留**：它是 Ely 原版就有的枚举值，且参与 `DockLayout` 的
  序列化；要撤的是"给它加的面板头入口"，不是这个类型本身。
- `DockPanel::plain` **保留**：外壳用它的"不画标签头"。撤的是**对外可配外观**，
  不是"能不能不画头"。

**连带改动**：撤掉的 API 的唯一消费者是组件库自己，所以同一个切片里要
同步删掉画廊页 `examples/gallery/pages/layout/systems.rs` 里那几段演示、
`i18n.rs` 里对应的文案键、`dock/mod.rs` 里对应的断言，以及 `layout/mod.rs`
的再导出。宿主 `workbench` 不受影响——它只用"留"那一批。

### 正确做法

- 宿主需要"开合工具窗 / 拖分隔条 / 记住尺寸"时，调 Dock 的 `show` / `hide` /
  `SideLimits` / `DockEvent::Resized`，不要在外壳里再写一份钳制或拖动。
- 要给 Dock 添一种新外观或新交互前，先问"宿主有没有消费方"；没有就不加，
  避免再攒一批只有画廊在用的能力。
- 往 Dock 里放内容时，包裹层必须是 **flex 容器**（`…flex().flex_col().flex_1()`），
  不能是普通 `div()`：gpui 的 `div()` 默认 `display: block`，里面的 `flex_1` 不生效，
  内容（编辑器、面板体）会塌成 0。真机验证时踩过这一条。

### 不要这样做

- 不要把"撤花活"做成"回到 `c6da4f26`"：那是 Ely 原版，`Dock` 只有
  `new`/`add`/`activate`/`move_panel`/`layout`/`restore`，**没有开合、没有每边
  上下限、没有尺寸事件、没有 `plain`**。撤到那里等于连带砍掉外壳行为，还要
  让 Dock 重新画标签头。
- 不要为了"不再用 gpui-kit"去改别的 crate：本轮范围只有 Dock；`gpui-kit`
  的退场按切片推进。
- 不要只让"外壳不用"而把 API 留在库里：那等于把死代码留成公共面，下一个人
  会以为它承重。

## 考虑过的备选方案

### 备选方案一：全撤到 `c6da4f26`（Ely 原版）

最"干净"，一次性回到上游形态。但那时 Dock 的公开面只有
`new`/`add`/`activate`/`move_panel`/`layout`/`restore`，外壳赖以工作的
开合（优化 1）、每边上下限（优化 3）、尺寸事件全都消失，工具窗打不开也
关不掉，还要重写外壳集成并接受 Dock 自画标签头。收益是"少几项能力"，代价是
"再换一次框架"，不划算。

### 备选方案二：连外壳换装一起撤，回到 gpui-kit 的 `resize_handle`

能立刻把界面救回来（外壳自管拖动与钳制的那套还在历史里）。但方向和"界面控件
用组件库"相反，等于把刚统一到组件库的拖动/钳制机制又搬回外壳，两个库的
分隔条行为会重新分叉。只在"本轮只想先要一个能跑的界面"时才是合理取舍。

### 备选方案三：不撤，只让外壳不用

改动最小、零风险。但那些能力会永久留在库里当公共面，画廊与单测持续为它们
背书，下一个人无法从 API 判断"哪些是真需求"。与"撤销没有需求的能力"相悖。

### 备选方案四：撤 API 但保留画廊演示

看起来"文档不缩水"。但演示要用被删的 API，等于 API 不能删；真要保留文档，
正确做法是**改写**演示段只演示剩下的能力（开合 / 每边上下限 / 无标签头），
而不是把死 API 留成演示的脚手架。

## 验收标准

行为级（不追求逐像素对上换装前的外观）：

- 宿主启动后界面不再空白：中央列（编辑器）有高度，三块工具窗按会话状态显示。
- 从活动栏与菜单能开合工具窗；再点同一项能收起（`select_top_activity` 的
  `by_click` 语义）。
- 三块工具窗的分隔条都能拖，拖动结果写回会话文件（`leftPaneWidth` /
  `rightPaneWidth` / `bottomPaneHeight`），重启后恢复。
- `cargo test -p lithe-gpui --features test-support`（1529 通过）与
  `cargo test -p lithe-gpui-workbench`（238 单元 + 集成测试全绿）全绿。
  ⚠️ 本机跑这两条要把 `TEMP`/`TMP` 指到工作区内可写目录，否则 49 条夹具报
  `PermissionDenied`（见 `HANDOFF.md` §2），与本次改动无关。
- `cargo fmt --manifest-path rust/Cargo.toml -- --check` 通过。
- `node rust/lithe-gpui/tools/check-ui-px.mjs` **exit 0 / 违规 0**：守卫已把 vendored
  组件库 `crates/lithe-gpui` 排除出 Application UI 范围（`tools/check-ui-px.mjs` 的
  `VENDORED_CRATE`，并配了一条"被排除的目录必须带 `UPSTREAM.md`"的自检，防止排除被
  顺手扩大）。要新写的第一方组件请放**独立 crate**，不要挂进被排除的目录。
- 工作区里的 `S1_D_RENDER` / `S1_H_SLOT` 诊断打印已删除。

## 风险

- **撤错边界**：把外壳真正在用的能力（`show`/`hide`、`SideLimits`、`plain`、
  `Resized`）一并撤掉，会让工具窗无法开合或尺寸无法持久化。回退方式：这些
  能力都在本 Note 的"留"清单里，按清单逐项核对。
- **画廊缩水引来异议**：删演示段会缩小组件库的文档面。若维护者认为画廊必须
  保留这些主题，退路是备选方案四（改写演示段），而不是保留死 API。
- **方向被误读成"现在就全量迁移"**：本文只登记归宿、不排期。真实迁移启动时
  另开 Note 说明范围与顺序。
- **主题桥被当成"已经对齐"**：主题桥是**保守映射**（只覆盖宿主确实有的语义色，
  渐变/图表/ANSI/语法色仍用组件库自己的调色板），不等于两边主题已经是同一份。
  控件迁移与本轮 Dock 切片都不改它。
- **宿主空白是否只由 Dock 引起**：当前证据是 `h_flex()` 的 `align-items`
  语义加整片空白的截图；动手前仍要以一次真机启动确认"撤完 + 修回高度"后
  编辑器与三块窗格都在。

## 适用范围

- `rust/lithe-gpui/crates/lithe-gpui/src/layout/dock/`（Dock 本体与单测）
- `rust/lithe-gpui/crates/lithe-gpui/src/layout/mod.rs`（再导出）
- `rust/lithe-gpui/crates/lithe-gpui/examples/gallery/pages/layout/systems.rs`
  与 `examples/gallery/i18n.rs`（演示与文案）
- `rust/lithe-gpui/crates/workbench/src/workspace.rs`（`dock_with_panels` /
  `sync_dock` / `panel_content` / `on_dock_event`）
- `rust/lithe-gpui/crates/workbench/src/component_theme.rs`（主题桥，后续切片）
- `rust/lithe-gpui/UI-MAP.md`（三块工具窗一节）
- 本切片落地后，更新
  `implemented/architecture/2026-10-05-shell-tool-windows-on-own-dock.md`
