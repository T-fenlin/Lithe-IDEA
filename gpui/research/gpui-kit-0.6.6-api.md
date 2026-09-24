# gpui-kit 0.6.6 可编译级 API 事实清单

> 目的：为重写 `gpui/` 桌面界面提供**只依据已发布源码**的 API 事实。
> 在线文档、`gpui-kit-design-guides` / `gpui-kit` skill 描述的是**未发布的 0.7.0**，与本清单冲突时**以本清单为准**。

## 0. 证据基准

本文件所有结论都来自本机 cargo registry 里的**已发布源码**，行号即这些文件里的行号：

| 简写 | 绝对路径 |
| --- | --- |
| `KIT` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-kit-0.6.6\` |
| `CMP` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-component-0.6.6\` |
| `BASE` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-base-0.6.6\` |
| `ASSETS` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-kit-assets-0.6.6\` |
| `GPUI` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-pre-0.3.6\` |
| `GPMAC` | `D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\gpui-pre-macros-0.3.6\` |

下文写作 `CMP\src\button\button.rs:44` 这种形式，路径省略前缀。

交叉验证应用：`D:\developmentProjects\rust\Dodona\crates\dodona`（依赖 `gpui-kit = "0.6"`），
只用于印证「这样写能编译」，**不作为事实来源**。

---

## 0.1 三条必须先记住的硬事实

### (A) `gpui_kit` 根只有 GPUI + 三个层，没有组件符号

`KIT\src\lib.rs`：

```rust
:95   pub use ::gpui::*;                       // gpui_kit::* 就是 gpui
:98   pub use ::gpui;                          // #[doc(hidden)] gpui_kit::gpui
:106  pub use ::gpui_base as base;             // gpui_kit::base
:143  pub use ::gpui_component as component;   // gpui_kit::component
:145  pub use ::gpui_kit_assets as assets;     // gpui_kit::assets
:149  pub use ::gpui_platform::application;
:156  pub fn init(cx: &mut App)
```

推论：

* **`gpui_kit::Sizable` / `gpui_kit::Disableable` / `gpui_kit::Selectable` 都不存在。**
  正确路径是 `gpui_kit::component::Sizable` 等（`CMP\src\lib.rs:118 pub use styled::*;`
  → `CMP\src\styled.rs:1-2`）。
* `gpui_kit::base` 里**也没有** `Sizable`：`BASE\src\lib.rs:183-186` 的 `theme_tokens` 导出里没有它，
  `Sizable` 定义在 `CMP\src\sizing.rs:178`。
* `gpui_kit::component::Size`（尺寸档枚举，`CMP\src\sizing.rs:6`）与
  `gpui_kit::Size`（**GPUI 的几何结构体**，来自 `pub use ::gpui::*`）**同名不同物**。
  同时 `use gpui_kit::*; use gpui_kit::component::*;` 会让 `Size` 产生歧义。

### (B) `open_window` 在 `App` / `AsyncApp` 上，不在 `gpui_kit` 上

`KIT\src\lib.rs` 全文（含 doc 注释）只有两处 `open_window`，
且都在 `//!`/`///` 注释里（`:37`、`:132`）指 `cx.open_window(...)`。
真实定义：

* `GPUI\src\app.rs:1347` `pub fn open_window<V: 'static + Render>(&mut self, options: WindowOptions, build_root_view: impl FnOnce(&mut Window, &mut App) -> Entity<V>) -> anyhow::Result<WindowHandle<V>>`
* `GPUI\src\app\async_context.rs:193` `AsyncApp::open_window` 同上（闭包签名一致）

`gpui_kit::open_window(...)` 这种写法**不存在**。

### (C) 大量「看起来像方法」的 API 在 trait 上，必须 `use ... as _`

0.6.6 里 trait 默认方法不会自动解析。已确认必须显式导入的 trait：

| trait | 定义处 | 提供的方法 | 导入路径 |
| --- | --- | --- | --- |
| `Sizable` | `CMP\src\sizing.rs:178` | `with_size` / `xsmall` / `small` / `large` | `gpui_kit::component::Sizable as _` |
| `Disableable` | `BASE\src\component_traits.rs:14` | `disabled` | `gpui_kit::component::Disableable as _` |
| `Selectable` | `BASE\src\component_traits.rs:3` | `selected` / `is_selected` / `secondary_selected` | `gpui_kit::component::Selectable as _` |
| `ButtonVariants` | `CMP\src\button\button.rs:44` | `primary/secondary/danger/warning/success/info/ghost/link/text/custom/with_variant` | `gpui_kit::component::button::ButtonVariants as _` |
| `ToggleVariants` | `CMP\src\button\toggle.rs:20` | `ghost` / `outline` / `with_variant` | `gpui_kit::component::button::ToggleVariants as _` |
| `DropdownMenu` | `CMP\src\menu\dropdown_menu.rs:12` | `dropdown_menu` / `dropdown_menu_with_anchor` | `gpui_kit::component::menu::DropdownMenu as _` |
| `ContextMenuExt` | `CMP\src\menu\context_menu.rs:13` | `context_menu` | `gpui_kit::component::menu::ContextMenuExt as _` |
| `ScrollableElement` | `CMP\src\scroll\scrollable.rs:16` | `scrollbar` / `vertical_scrollbar` / `horizontal_scrollbar` / `overflow_scrollbar` / `overflow_x_scrollbar` / `overflow_y_scrollbar` | `gpui_kit::component::scroll::ScrollableElement as _` |
| `WindowExt` | `CMP\src\window_ext.rs:12` | `open_dialog` / `open_alert_dialog` / `open_sheet` / `push_notification` … | `gpui_kit::component::WindowExt as _` |
| `ActiveTheme` | `CMP\src\theme\mod.rs:44` | `theme()` | `gpui_kit::component::ActiveTheme as _` |
| `StyledExt` | `BASE\src\styled.rs:78` | `h_flex` / `v_flex` / `paddings` / `margins` / `refine_style` | `gpui_kit::component::StyledExt as _` |
| `FocusTrapElement` | `BASE\src\focus_trap.rs:14` | `focus_trap` | `gpui_kit::component::FocusTrapElement as _` |
| `IconNameExt` | `CMP\src\icon.rs:67` | `view`（**只对 `assets::IconName`**） | `gpui_kit::component::IconNameExt as _` |
| `PanelSource` / `PanelBuilder` | `BASE\src\dock\state_convert.rs:15` / `:72` | `to_state` / `from_state` | `gpui_kit::component::dock::{PanelSource, PanelBuilder}` |
| `InteractiveElementExt` | `BASE\src\event.rs:26` | 见 `BASE\src\event.rs` | `gpui_kit::component::InteractiveElementExt as _` |
| `AxisExt` / `LengthExt` | `BASE\src\geometry.rs:88` / `:106` | `is_horizontal`/`is_vertical`；`to_pixels` | `gpui_kit::component::{AxisExt, LengthExt}` |

官方测试里就有这条证据：`KIT\tests\components.rs:98`
`use gpui_component::scroll::ScrollableElement as _;` —— 不导入就调不出 `.overflow_y_scrollbar()`。

反过来，以下组件的同名方法是**固有方法**，**不需要** trait 导入（固有方法优先于 trait 方法）：

* `ListItem::selected` `CMP\src\list\list_item.rs:76`、`ListItem::disabled` `:87`
* `Tab::disabled` `CMP\src\tab\tab.rs:547`
* `Input::disabled` `CMP\src\input\input.rs:310`
* `Switch` 没有固有 `disabled`（只有 `Sizable`/`Disableable` 的 trait 实现：`CMP\src\switch.rs:113,120`）

---

## 1. 外壳与布局

### 1.1 `component::Root`（`Root` = `gpui_component::Root`）

* 导入：`gpui_kit::component::Root`（`CMP\src\lib.rs:117 pub use root::Root;`）
* 类型：`CMP\src\root.rs:37 pub struct Root`，**不是 `RenderOnce`**，而是
  `Entity<Root>` + `impl Render for Root`（`CMP\src\root.rs:580`）
* 构造函数：

```rust
// CMP\src\root.rs:100
pub fn new(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self
```

* builder（返回 `Self`，都是固有方法）：

| 方法 | 证据 |
| --- | --- |
| `bordered(mut self, bordered: bool) -> Self` | `CMP\src\root.rs:143` |
| `window_shadow_size(mut self, size: impl Into<Pixels>) -> Self` | `CMP\src\root.rs:151` |

* 静态 helper（**注意都是关联函数，不是 `&self`**）：

| 方法 | 证据 |
| --- | --- |
| `Root::update(window: &mut Window, cx: &mut App, f) -> R` | `CMP\src\root.rs:156` |
| `Root::read<'a>(window: &'a Window, cx: &'a App) -> &'a Self` | `CMP\src\root.rs:176` |
| `Root::render_notification_layer(window, cx) -> Option<impl IntoElement>` | `CMP\src\root.rs:185` |
| `Root::render_sheet_layer(window, cx) -> Option<impl IntoElement>` | `CMP\src\root.rs:215` |
| `Root::render_dialog_layer(window, cx) -> Option<impl IntoElement>` | `CMP\src\root.rs:242` |

* `Root` 的 4 个必需/常用能力：`open_dialog` `:297`、`open_sheet_at` `:379`、
  `push_notification` `:425`、`close_dialog` `:333`、`close_all_dialogs` `:365`。
* `Root` 实现了 `Styled`（`:574`）。

最小片段（**Root 必须是窗口第一个 view**，否则 `Root::update` 会 panic：`CMP\src\root.rs:163`）：

```rust
use gpui_kit::component::Root;
use gpui_kit::*;

cx.open_window(WindowOptions::default(), |window, cx| {
    let view = cx.new(|_| MyApp::default());
    cx.new(|cx| Root::new(view, window, cx))   // Root::new(view, window, cx)
}).expect("open window");
```

### 1.2 `component::TitleBar`

* 导入：`gpui_kit::component::TitleBar`（`CMP\src\lib.rs:121 pub use title_bar::*;`）
* 类型：`CMP\src\title_bar.rs:42 pub struct TitleBar`，`#[derive(IntoElement)]` +
  `impl RenderOnce`（`:319`）+ `Styled`（`:296`）+ `ParentElement`（`:302`）

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `() -> Self` | `CMP\src\title_bar.rs:50` |
| `title_bar_options` | `() -> TitlebarOptions` | `:59` |
| `window_options` | `() -> WindowOptions` | `:81` |
| `on_close_window` | `(f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self` | `:95`（**只在 Linux 生效**，`:99`） |

```rust
use gpui_kit::component::{Root, TitleBar};
use gpui_kit::*;

TitleBar::window_options(); // 作为 WindowOptions 的 base
// render 里：
TitleBar::new()
    .child("Lithe")                     // ParentElement
    .child(div().flex_1())
    .child(Button::new("theme").label("Theme"))
```

### 1.3 `component::StatusBar` —— **不在 component 根**

* 导入：**`gpui_kit::component::status_bar::StatusBar`**（`CMP\src\lib.rs:85` 只有 `pub mod status_bar;`，
  **没有** `pub use status_bar::*`）。`gpui_kit::component::StatusBar` 不存在。
* 类型：`CMP\src\status_bar.rs:32 pub struct StatusBar`，`#[derive(IntoElement)]` +
  `impl RenderOnce`（`:77`）+ `Styled`（`:71`）+ `ParentElement`（`:65`）

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `() -> Self` | `:41` |
| `left` | `(child: impl IntoElement) -> Self` | `:51` |
| `right` | `(child: impl IntoElement) -> Self` | `:57` |
| `child`/`children` | 来自 `ParentElement`，进**中间区** | `:65-69` |

```rust
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::StyledExt as _;   // h_flex/v_flex 等；`use gpui_kit::*` 不含它

StatusBar::new()
    .left("main")                       // StatusBar::left
    .left("Ln 1, Col 1")
    .right("UTF-8")
```

### 1.4 `component::dock` —— 0.6.6 与 0.7 差异最大的一块

导入：`gpui_kit::component::dock::{...}`（`CMP\src\dock\mod.rs`）。

**先说两个「不存在」：**

* **`TabPanel` 在 0.6.6 已删除。**`CMP\src\dock\` 全目录搜索只有两处注释提到旧名
  （`CMP\src\dock\tab_panel.rs:892`、`:1419`），没有任何 `pub struct TabPanel`。
  替代物是 `DockSkin` + `TabGroupSkin`（后者 `pub(crate)`，见 `CMP\src\dock\tab_panel.rs:185`，**应用不可用**）。
* **`DockItem` 在 0.6.6 已删除。**`CMP` 全 crate 无匹配；`BASE` 只在注释里出现
  （`BASE\src\dock\dock_area.rs:207`、`:2052`，`BASE\src\dock\layout\builder.rs:19`、`:249`）。
  替代物是 `DockLayout`。

#### 1.4.1 `DockArea`（= `gpui_base::dock::DockArea`，`CMP\src\dock\mod.rs:49` 再导出）

* 类型：`BASE\src\dock\dock_area.rs:99 pub struct DockArea`，
  `Entity<DockArea>` + `impl Render`（`:1385`）+ `EventEmitter<DockEvent>`（`:1377`）+ `Focusable`（`:1379`）
* 构造：`BASE\src\dock\dock_area.rs:126`

```rust
pub fn new(id: impl Into<SharedString>, version: Option<usize>,
           _window: &mut Window, cx: &mut Context<Self>) -> Self
```

* `with_renderer(mut self, renderer: Rc<dyn DockAreaRenderer>) -> Self` `:154`
* 布局/停靠：`set_center(&mut self, DockLayout, &mut Window, &mut Context<Self>)` `:232`、
  `set_dock(&mut self, DockPlacement, DockLayout, …)` `:246`、
  `remove_dock(&mut self, DockPlacement, …)` `:271`、
  `toggle_dock` `:301`、`is_dock_open` `:293`、`set_dock_size` `:348`
* 面板：`add_panel<P: Panel>` `:371`、`add_panel_view` `:389`、`remove_panel<P: Panel>` `:469`、
  `move_panel` `:480`、`select_panel` `:558`
* 缩放：`set_zoomed_in` `:626`、`set_zoomed_out` `:635`、`is_zoomed` `:639`
* 持久化：`dump(&self, cx: &App) -> DockAreaState` `:794`、`load(...)` `:716`

#### 1.4.2 `DockSkin`

* 类型：`CMP\src\dock\mod.rs:122 pub struct DockSkin`（`Rc<SkinShared>` 的封装，**不是 Entity**）
* 构造（两个，签名不同，别搞混）：

```rust
// CMP\src\dock\mod.rs:132  —— 推荐：在构造 area 的同时拿到 skin
pub fn dock_area(id: impl Into<SharedString>, version: Option<usize>,
                 window: &mut Window, cx: &mut App) -> (Entity<DockArea>, Rc<Self>)

// CMP\src\dock\mod.rs:151  —— 只能在 cx.new(...) 的闭包里调用
pub fn new(cx: &mut Context<DockArea>) -> Rc<Self>
```

* 设置：`panel_style(&self) -> PanelStyle` `:167`、`set_panel_style(&self, PanelStyle, &mut App)` `:171`

#### 1.4.3 `DockLayout`（描述式布局，非 Entity）

* 类型：`BASE\src\dock\layout\builder.rs:24 pub struct DockLayout`
* 构造：`h_split()` `:41`、`v_split()` `:46`、`tabs()` `:51`
* builder：

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `child` | `(child: DockLayout, size: Option<Pixels>) -> Self` | `:71`（**只能用在 `h_split`/`v_split`**，误用触发 `debug_assert!`） |
| `panel` | `<P: Panel>(panel: Entity<P>) -> Self` | `:83` |
| `panel_view` | `(panel: Arc<dyn PanelView>, cx: &App) -> Self` | `:101` |
| `active_index` | `(ix: usize) -> Self` | `:114` |

#### 1.4.4 `DockPlacement`

`BASE\src\dock\state.rs:161 pub enum DockPlacement { Center, Left, Bottom, Right }`
（`#[serde(rename)]` 见 `:162-169`）。方法：`axis()` `:173`、`is_left()` `:180`、`is_bottom()` `:184`、`is_right()` `:188`。

#### 1.4.5 `Panel` / `PanelEvent` —— **两层 trait，必须同时实现**

这是最容易踩的坑：`component::dock::Panel`（表现层）**extends** `gpui_base::dock::Panel`（行为层），
而后者在本模块里被改名导出为 `BasePanel`（`CMP\src\dock\mod.rs:32`）。

```rust
// BASE\src\dock\panel.rs:21  —— 行为层（= component::dock::BasePanel）
pub trait Panel: EventEmitter<PanelEvent> + Render + Focusable {
    fn panel_name(&self) -> &'static str;                     // :23 唯一必需
    fn visible(&self, cx: &App) -> bool { true }              // :28
    fn closable(&self, cx: &App) -> bool { true }             // :34
    fn zoomable(&self, cx: &App) -> bool { true }             // :40
    fn set_active(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {}  // :53
    fn set_zoomed(&mut self, zoomed: bool, window: &mut Window, cx: &mut Context<Self>) {}  // :61
    fn on_added_to(&mut self, group: WeakEntity<TabGroup>, window: &mut Window, cx: &mut Context<Self>) {} // :68
    fn on_removed(&mut self, window: &mut Window, cx: &mut Context<Self>) {}                 // :82
    fn dump(&self, cx: &App) -> PanelState { PanelState::new(self.panel_name()) }            // :90
}

// CMP\src\dock\panel.rs:71  —— 表现层（component::dock::Panel）
pub trait Panel: gpui_base::dock::Panel {
    fn tab_name(&self, cx: &App) -> Option<SharedString> { None }                            // :76
    fn title(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement    // :82（有默认值 t!("Dock.Unnamed")）
    fn title_style(&self, cx: &App) -> Option<TitleStyle> { None }                           // :87
    fn title_suffix(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<impl IntoElement> { None::<gpui::Div> } // :92
    fn toolbar_buttons(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<Vec<Button>> { None } // :101
    fn dropdown_menu(&mut self, menu: PopupMenu, window: &mut Window, cx: &mut Context<Self>) -> PopupMenu { menu } // :110
    fn zoom_control(&self, cx: &App) -> Option<PanelControl> { Some(PanelControl::Menu) }      // :131
    fn inner_padding(&self, cx: &App) -> bool { true }                                        // :137
    fn title_bar(&self, cx: &App) -> bool { true }                                            // :146
}
```

* `PanelEvent`：`BASE\src\dock\panel.rs:12 pub enum PanelEvent { ZoomIn, ZoomOut, LayoutChanged }`
  （**没有** `Clone`/`Debug` derive）
* `PanelStyle`：`CMP\src\dock\panel.rs:31 pub enum PanelStyle { Auto, TabBar }`
* `PanelControl`：`CMP\src\dock\panel.rs:46 pub enum PanelControl { Both, Menu, Toolbar }`
  （`#[default]` 是 `Menu`，`:48`）
* `TitleStyle`：`CMP\src\dock\panel.rs:40 pub struct TitleStyle { pub background: Hsla, pub foreground: Hsla }`
* `panel_handle<P: Panel>(panel: Entity<P>) -> Arc<dyn gpui_base::dock::PanelView>` `CMP\src\dock\panel.rs:325`

最小可用片段（拼自 `KIT\tests\dock.rs:18-72`，方法名全部在源码中存在）：

```rust
use gpui_kit::component::dock::{
    BasePanel, DockLayout, DockPlacement, DockSkin, Panel, PanelControl, PanelEvent, panel_handle,
};
use gpui_kit::*;

struct Document { name: &'static str, focus: FocusHandle }

impl BasePanel for Document {                                  // gpui_base::dock::Panel
    fn panel_name(&self) -> &'static str { self.name }
}
impl Panel for Document {                                      // component::dock::Panel
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement { self.name }
    fn zoom_control(&self, _: &App) -> Option<PanelControl> { Some(PanelControl::Toolbar) }
}
impl EventEmitter<PanelEvent> for Document {}
impl Focusable for Document { fn focus_handle(&self, _: &App) -> FocusHandle { self.focus.clone() } }
impl Render for Document { /* ... */ }

// 构造：
let (area, skin) = DockSkin::dock_area("main", None, window, cx);   // CMP dock/mod.rs:132
skin.set_panel_style(PanelStyle::TabBar, cx);                       // :171
let a = cx.new(|cx| Document { name: "alpha", focus: cx.focus_handle() });
let layout = DockLayout::tabs().panel_view(panel_handle(a), cx);     // builder.rs:51 / :101 / panel.rs:325
area.update(cx, |area, cx| area.set_center(layout, window, cx));     // dock_area.rs:232
area.update(cx, |area, cx| area.set_dock(DockPlacement::Left,
    DockLayout::tabs().panel_view(panel_handle(b), cx), window, cx)); // dock_area.rs:246
```

### 1.5 `component::resizable`

导入：`gpui_kit::component::resizable::{h_resizable, v_resizable, resizable_panel, ResizableState, ResizablePanelGroup, ResizablePanel, ResizablePanelEvent}`
（`CMP\src\lib.rs:68-73` 的 `pub mod resizable { pub use super::{...} }`，
同时 `CMP\src\lib.rs:107-110` 也把它们放在 `gpui_kit::component` 根上）。

| 项 | 签名 | 证据 |
| --- | --- | --- |
| `h_resizable` | `(id: impl Into<ElementId>) -> ResizablePanelGroup` | `BASE\src\resizable\mod.rs:17` |
| `v_resizable` | `(id: impl Into<ElementId>) -> ResizablePanelGroup` | `:22` |
| `resizable_panel` | `() -> ResizablePanel` | `:27` |
| `ResizableState` | `BASE\src\resizable\mod.rs:33`（`Entity<ResizableState>`） |  |
| `ResizablePanelGroup` | `BASE\src\resizable\panel.rs:31`，`#[derive(IntoElement)]` + `RenderOnce`（`:137`） |  |
| `ResizablePanel` | `BASE\src\resizable\panel.rs:238`，`#[derive(IntoElement)]` + `RenderOnce`（`:301`） |  |

* `ResizablePanelGroup` builder：`new(id)` `:43`、`with_handle_appearance(ResizeHandleRenderer)` `:59`、
  `with_state(&Entity<ResizableState>)` `:67`、`axis(Axis)` `:73`、`child(impl Into<ResizablePanel>)` `:83`、
  `children(...)` `:89`、`size(Pixels)` `:101`、`on_resize(...)` `:111`
* `ResizablePanel` builder：`visible(bool)` `:269`、`size(impl Into<Pixels>)` `:275`、
  `size_range(impl Into<Range<Pixels>>)` `:283`、`Styled` `:289`、`ParentElement` `:295`
* `ResizableState` 方法：`sizes() -> &Vec<Pixels>` `:56`、`resize_panel(ix, size, window, cx)` `:69`、
  `insert_panel(size: Option<Pixels>, ix: Option<usize>, cx)` `:95`、`remove_panel` `:221`、
  `reset_panel` `:233`、`clear` `:242`、`container_size` `:249`
* `ResizableState` 是 `EventEmitter<ResizablePanelEvent>`（`BASE\src\resizable\mod.rs:388`），
  事件枚举在 `BASE\src\resizable\panel.rs:17`
* `PANEL_MIN_SIZE: Pixels = px(100.)` `BASE\src\resizable\mod.rs:14`

```rust
use gpui_kit::component::resizable::{h_resizable, resizable_panel};

h_resizable("main")                      // -> ResizablePanelGroup
    .child(resizable_panel().size(px(280.)).child(sidebar))   // .size(impl Into<Pixels>)
    .child(resizable_panel().child(editor));
```

### 1.6 `component::scroll`

导入：`gpui_kit::component::scroll::{Scrollbar, ScrollbarMode}`（`CMP\src\scroll\mod.rs:5-8` 从 `gpui_base` 再导出）。
注意 `ScrollbarMode` **也在** `gpui_kit::component::scroll` 里 —— Dodona 就是
`use gpui_kit::component::{Theme, ThemeMode, ThemeTokens, scroll::ScrollbarMode};`。

* `ScrollbarMode`：`BASE\src\scrollbar.rs:48 pub enum ScrollbarMode { Scrolling, Hover, Always }`
  （`#[default]` = `Scrolling`，`:50`）
* `Scrollbar`：`BASE\src\scrollbar.rs:815 pub struct Scrollbar`，实现的是
  **`IntoElement`（`:1155`）+ `Element`（`:1252`）**，**不是 `RenderOnce`**

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `<H: ScrollbarHandle + Clone>(scroll_handle: &H) -> Self` | `:836` |
| `horizontal` | `<H: ScrollbarHandle + Clone>(&H) -> Self` | `:853` |
| `vertical` | `<H: ScrollbarHandle + Clone>(&H) -> Self` | `:859` |
| `id` | `(impl Into<ElementId>) -> Self` | `:866` |
| `mode` | `(mode: ScrollbarMode) -> Self` | `:874` |
| `scroll_size` | `(Size<Pixels>) -> Self` | `:882` |
| `viewport_bounds` | `(Bounds<Pixels>) -> Self` | `:893` |
| `viewport_from_layout` | `() -> Self` | `:903` |
| `axis` | `(impl Into<ScrollbarAxis>) -> Self` | `:919` |
| `styles` | `(impl FnOnce(ScrollbarStyles) -> ScrollbarStyles) -> Self` | `:924` |
| `max_fps` | `(usize) -> Self` | `:935` |

* `ScrollableElement`（`CMP\src\scroll\scrollable.rs:16`）—— **只有 `Div` 和 `Stateful<E>` 实现了它**
  （`:189`、`:190`），自定义 Element 用不了：

```rust
fn overflow_scrollbar(self) -> Scrollable<Self>          // :46
fn overflow_x_scrollbar(self) -> Scrollable<Self>        // :53
fn overflow_y_scrollbar(self) -> Scrollable<Self>        // :60
fn scrollbar<H: ScrollbarHandle + Clone>(self, &H, impl Into<ScrollbarAxis>) -> Self // :19
fn vertical_scrollbar<H>(self, &H) -> Self               // :33
fn horizontal_scrollbar<H>(self, &H) -> Self             // :39
```

⚠️ 返回值是 `Scrollable<Self>`（`CMP\src\scroll\scrollable.rs:67`），**不是 `Div`**。
需要继续 `.id(...)`/`.track_focus(...)` 时按官方测试的写法（`KIT\tests\components.rs:99-105`）串下去。

### 1.7 `component::sidebar`

导入：`gpui_kit::component::sidebar::{Sidebar, SidebarMenu, SidebarMenuItem, SidebarGroup, SidebarItem, SidebarCollapsible, SidebarHeader, SidebarFooter, SidebarToggleButton}`
（`CMP\src\sidebar\mod.rs:22-25` + 同文件 `:222` 的 `Sidebar`）。

* `SidebarItem` trait：`CMP\src\sidebar\mod.rs:211 pub trait SidebarItem: Collapsible + Clone`
* `Sidebar<E: SidebarItem + 'static>`：`CMP\src\sidebar\mod.rs:222`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `(id: impl Into<ElementId>) -> Self` | `:238` |
| `side` | `(side: Side) -> Self` | `:254` |
| `collapsible` | `(impl Into<SidebarCollapsible>) -> Self` | `:264` |
| `collapsed` | `(bool) -> Self` | `:270` |
| `header` | `(impl IntoElement) -> Self` | `:276` |
| `footer` | `(impl IntoElement) -> Self` | `:282` |
| `child` / `children` | `(E) -> Self` / `(impl IntoIterator<Item = E>) -> Self` | `:288` / `:294` |

* `SidebarMenu`：`CMP\src\sidebar\menu.rs:20`

| 方法 | 证据 |
| --- | --- |
| `new() -> Self` | `:28` |
| `child(child: impl Into<SidebarMenuItem>) -> Self` | `:39` |
| `children(...)` | `:45` |
| `Collapsible` 实现（`collapsed` / `is_collapsed`） | `:54` |

* `SidebarMenuItem`：`CMP\src\sidebar\menu.rs:94`

| 方法 | 证据 |
| --- | --- |
| `new(label: impl Into<SharedString>) -> Self` | `:113` |
| `icon(impl Into<Icon>)` | `:133` |
| `label_style(StyleRefinement)` | `:139` |
| `active(bool)` | `:145` |
| `on_click(...)` | `:151` |
| `collapsed(bool)` | `:160` |
| `default_open(bool)` | `:168` |
| `click_to_open(bool)` | `:176` |
| `click_to_toggle(bool)` | `:188` |
| `children(impl IntoIterator<Item = impl Into<Self>>)` | `:193` |
| `suffix<F, E>(builder: F)` | `:199` |
| `disable(bool)` | `:211`（**注意是 `disable`，不是 `disabled`**） |
| `context_menu(...)` | `:225` |

* `SidebarGroup<E: SidebarItem + 'static>`：`CMP\src\sidebar\group.rs:9`
  `new(label: impl Into<SharedString>)` `:17`、`child(E)` `:26`、`children(...)` `:34`

```rust
use gpui_kit::component::sidebar::{Sidebar, SidebarGroup, SidebarMenu, SidebarMenuItem};
use gpui_kit::component::{Icon, IconName, Side};

Sidebar::new("explorer")                              // Sidebar::new(id)
    .side(Side::Left)                                 // Side::Left，来自 gpui_kit::component::Side
    .collapsible(true)                                // impl Into<SidebarCollapsible>，From<bool> 在 mod.rs:48
    .child(
        SidebarGroup::new("FILES")                    // SidebarGroup::new(label)
            .child(
                SidebarMenu::new()
                    .child(SidebarMenuItem::new("src")   // SidebarMenuItem::new(label)
                        .icon(Icon::new(IconName::Folder))
                        .active(true))
                    .child(SidebarMenuItem::new("main.rs")
                        .icon(Icon::new(IconName::FileText))
                        .on_click(|_, _, _| {})),
            ),
    )
```

### 1.8 `component::breadcrumb::Breadcrumb`

导入：`gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem}`（`CMP\src\breadcrumb.rs`）。

* `Breadcrumb`：`:13`，`RenderOnce` `:156`，`Styled` `:150`
  `new()` `:119`、`child(item: impl Into<BreadcrumbItem>)` `:127`、`children(...)` `:133`
* `BreadcrumbItem`：`:20`，`RenderOnce` `:91`，`Styled` `:67`
  `new(label: impl Into<SharedString>)` `:31`、`disabled(bool)` `:42`、`on_click(...)` `:47`
  `From<&'static str>` `:73`、`From<String>` `:79`、`From<SharedString>` `:85`

```rust
use gpui_kit::component::breadcrumb::{Breadcrumb, BreadcrumbItem};

Breadcrumb::new()
    .child(BreadcrumbItem::new("lithe").on_click(|_, _, _| {}))
    .child("src")               // &'static str -> BreadcrumbItem
    .child(BreadcrumbItem::new("main.rs"))
```

---

## 2. 列表与数据

### 2.1 `component::list::{List, ListItem, ListState, ListDelegate}`

导入：`gpui_kit::component::list::{List, ListItem, ListState, ListDelegate, ListEvent, ListSeparatorItem}`
（`CMP\src\list\mod.rs:8-12`）。

* `ListDelegate` trait：`CMP\src\list\delegate.rs:10`

```rust
pub trait ListDelegate: Sized + 'static {
    type Item: Selectable + IntoElement;                                         // :11 必需
    fn perform_search(&mut self, query: &str, window, cx) -> Task<()> { Task::ready(()) } // :15
    fn sections_count(&self, cx: &App) -> usize { 1 }                            // :27
    fn items_count(&self, section: usize, cx: &App) -> usize;                    // :35 必需
    fn render_item(&mut self, ix: IndexPath, window, cx) -> Option<Self::Item>;  // :42 必需
    fn render_section_header(&mut self, section, window, cx) -> Option<impl IntoElement> // :52
    fn render_section_footer(&mut self, section, window, cx) -> Option<impl IntoElement> // :64
    fn render_empty(&mut self, window, cx) -> impl IntoElement                   // :74（默认 Inbox 骨架）
    fn render_initial(&mut self, window, cx) -> Option<AnyElement>               // :95
    fn loading(&self, cx: &App) -> bool { false }                                // :104
    fn render_loading(&mut self, window, cx) -> impl IntoElement { Loading }     // :110
    fn set_selected_index(&mut self, ix: Option<IndexPath>, window, cx);         // :119 必需
    fn set_right_clicked_index(&mut self, ix: Option<IndexPath>, window, cx) {}  // :127
    fn confirm(&mut self, secondary: bool, window, cx) {}                        // :139
    fn cancel(&mut self, window, cx) {}                                          // :143
    fn has_more(&self, cx: &App) -> bool { false }                               // :148
    fn load_more_threshold(&self) -> usize { 20 }                                // :159
    fn load_more(&mut self, window, cx) {}                                       // :170
}
```

* `ListState<D: ListDelegate>`：`CMP\src\list\list.rs:70`，
  `impl Render for ListState<D>` `:621`，`EventEmitter<ListEvent>` `:620`，`Focusable` `:608`
  `new(delegate: D, window: &mut Window, cx: &mut Context<Self>) -> Self` `:94`
  `searchable(bool)` `:125`、`selectable(bool)` `:136`、`delegate()` `:147`、`delegate_mut()` `:151`、
  `focus(window, cx)` `:156`、`set_selected_index(...)` `:184`、`selected_index()` `:194`、
  `set_query(...)` `:215`、`scroll_to_item(...)` `:239`、`scroll_handle()` `:259`
* `List<D: ListDelegate + 'static>`：`CMP\src\list\list.rs:721`，`#[derive(IntoElement)]`，
  `RenderOnce` `:772`、`Styled` `:753`、`Sizable` `:762`
  `new(state: &Entity<ListState<D>>)` `:732`、`scrollbar_visible(bool)` `:741`、`search_placeholder(...)` `:747`
* `ListItem`：`CMP\src\list\list_item.rs:26`，`#[derive(IntoElement)]`，`RenderOnce` `:176`、
  `Styled` `:153`、`ParentElement` `:159`、`InteractiveElement` `:168`、`Selectable` `:137`、`Disableable` `:130`

**关键事实（坑）**：`ListItem::new(id)` 内部是 `base: h_flex().id(id)`（`CMP\src\list\list_item.rs:48`）
—— **children 是横排**。要竖排必须在里面自己放 `v_flex()`。

**`IndexPath`**（`ListDelegate`/`TableDelegate`/`SelectState` 到处都用）：
`CMP\src\index_path.rs:1 pub use gpui_base::IndexPath;` —— 就是 `BASE\src\index_path.rs:9` 的结构体：

```rust
pub struct IndexPath { pub section: usize, pub row: usize, pub column: usize }  // :9 / :11 / :13 / :15
pub fn new(row: usize) -> Self          // :39
pub fn section(mut self, usize) -> Self // :48
pub fn row(mut self, usize) -> Self     // :54   ← 与字段 `row` 同名，但字段/方法各在一个命名空间
pub fn column(mut self, usize) -> Self  // :60
pub fn eq_row(&self, index: IndexPath) -> bool  // :66
```

导入路径：`gpui_kit::component::IndexPath`（`CMP\src\lib.rs:113`）或 `gpui_kit::base::IndexPath`。
⚠️ `section`/`row`/`column` 既是**公开字段**又是**builder 方法**；读 `ix.row`，构造用 `IndexPath::new(0).section(1)`。

```rust
use gpui_kit::component::list::{List, ListDelegate, ListItem, ListState};
use gpui_kit::component::{IndexPath, Selectable, h_flex, v_flex, Icon, IconName};
use gpui_kit::*;

struct FilesDelegate { items: Vec<String>, selected: Option<IndexPath> }

impl ListDelegate for FilesDelegate {
    type Item = ListItem;                                                  // :11
    fn items_count(&self, _section: usize, _cx: &App) -> usize { self.items.len() }  // :35
    fn render_item(&mut self, ix: IndexPath, _: &mut Window, _: &mut Context<ListState<Self>>)
        -> Option<Self::Item> {                                            // :42
        let selected = self.selected == Some(ix);
        // 横排要自己补：把图标和文字放进 h_flex（ListItem 本来就是 h_flex），
        // 如果要做两行文本，必须显式 v_flex()，否则两段文字会并排。
        Some(
            ListItem::new(ix.row)                                          // :44
                .selected(selected)                                        // 固有方法 :76
                .child(Icon::new(IconName::FileText))
                .child(v_flex().child(self.items[ix.row].clone()).child("2 KB")),
        )
    }
    fn set_selected_index(&mut self, ix: Option<IndexPath>, _: &mut Window,
                          _: &mut Context<ListState<Self>>) { self.selected = ix; }  // :119
}

// 使用：
let state = cx.new(|cx| ListState::new(FilesDelegate { items, selected: None }, window, cx)); // list.rs:94
List::new(&state).scrollbar_visible(true)                                   // list.rs:732 / :741
```

### 2.2 `component::tree::{Tree, TreeState, TreeItem, TreeEntry}`

导入：`gpui_kit::component::tree::{Tree, tree, TreeState, TreeItem, TreeEntry, TreeEvent, TreeEntryState}`。
`CMP\src\tree.rs:15 pub use gpui_base::{TreeEntry, TreeEvent, TreeItem, TreeState};`
—— 这四个是 **`gpui_base` 的类型**，`Tree` 才是 `CMP` 的样式包装。

* `TreeItem`：`BASE\src\tree.rs:41`
  `new(id: impl Into<SharedString>, label: impl Into<SharedString>)` `:99`、
  `child(TreeItem)` `:111`、`children(...)` `:116`、`expanded(bool)` `:121`、`disabled(bool)` `:126`、
  `is_folder()` `:132`、`is_disabled()` `:136`、`is_expanded()` `:141`、`ancestors(&SharedString)` `:146`
* `TreeEntry`：`BASE\src\tree.rs:50`
  `item() -> &TreeItem` `:61`、`depth() -> usize` `:66`、`is_root()` `:71`、`is_folder()` `:76`、
  `is_expanded()` `:81`、`is_disabled()` `:86`
* `TreeEvent`：`BASE\src\tree.rs:93 pub enum TreeEvent { Expanded(SharedString), Collapsed(SharedString) }`
  （`#[derive(Clone, Debug, PartialEq, Eq)]`，`:92`）
* `TreeEntryState`：`BASE\src\tree.rs:164`，`is_selected(self) -> bool` `:171`、`is_right_clicked(self) -> bool` `:176`
* `TreeState`：`BASE\src\tree.rs:184`，`impl Render` `:420`，`EventEmitter<TreeEvent>` `:194`
  `new(cx: &mut App)` `:197`、`items(impl Into<Vec<TreeItem>>)` `:209`、`set_items(...)` `:214`、
  `selected_index()` `:221`、`set_selected_index(...)` `:225`、`set_selected_item(...)` `:230`、
  `selected_item()` `:243`、`selected_entry()` `:248`、`entry(ix)` `:252`、`scroll_handle()` `:256`、
  `scroll_to_item(ix, ScrollStrategy)` `:260`、`index_of(&SharedString)` `:264`、`reveal_item(...)` `:268`、
  `focus(window, cx)` `:280`
* `Tree`（CMP 包装）：`CMP\src\tree.rs:32`，`#[derive(IntoElement)]`，`RenderOnce` `:71`，`Styled` `:65`

```rust
// CMP\src\tree.rs:18  —— 自由函数
pub fn tree<R>(state: &Entity<TreeState>, render_item: R) -> Tree
where R: Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem + 'static;

// CMP\src\tree.rs:41  —— 同样的东西
pub fn new<R>(state: &Entity<TreeState>, render_item: R) -> Self
// CMP\src\tree.rs:55
pub fn context_menu<F>(mut self, f: F) -> Self
```

⚠️ **`Tree` 的行渲染闭包必须返回 `ListItem`**（`CMP\src\tree.rs:20`），不是任意 Element。

```rust
use gpui_kit::component::tree::{tree, TreeEntry, TreeItem, TreeState};
use gpui_kit::component::list::ListItem;

let state = cx.new(|cx| TreeState::new(cx).items(vec![          // BASE tree.rs:197 / :209
    TreeItem::new("src", "src").expanded(true)                  // :99 / :121
        .child(TreeItem::new("src/main.rs", "main.rs")),
]));
tree(&state, |_ix, entry, selected, _window, _cx| {             // CMP tree.rs:18
    ListItem::new(entry.item().id.clone())
        .selected(selected)
        .child(entry.item().label.clone())
})
```

### 2.3 `component::table::{DataTable, TableState, TableDelegate, Column}`

导入：`gpui_kit::component::table::{DataTable, Table, TableState, TableDelegate, TableEvent, Column, ColumnGroup, ColumnSort, ColumnFixed, SelectionMode, TableVisibleRange}`
（`CMP\src\table\mod.rs:10-14`）。

* `Column`：`CMP\src\table\column.rs:10`（**全部字段 pub**）

```rust
pub struct Column {
    pub key: SharedString,          // :16
    pub name: SharedString,         // :18
    pub align: TextAlign,           // :20
    pub sort: Option<ColumnSort>,   // :24
    pub paddings: Option<Edges<Pixels>>, // :26
    pub width: Pixels,              // :28
    pub fixed: Option<ColumnFixed>, // :30
    pub resizable: bool,            // :32
    pub movable: bool,              // :34
    pub selectable: bool,           // :44
    pub min_width: Pixels,          // :46
    pub max_width: Pixels,          // :48
}
```

  builder：`new(key, name)` `:88`、`sort(ColumnSort)` `:99`、`sortable()` `:107`、`ascending()` `:113`、
  `descending()` `:119`、`text_center()` `:125`、`text_right()` `:133`、`paddings(impl Into<Edges<Pixels>>)` `:139`、
  `p_0()` `:145`、`width(impl Into<Pixels>)` `:151`、`fixed(impl Into<ColumnFixed>)` `:157`、
  `fixed_left()` `:163`、`resizable(bool)` `:169`、`movable(bool)` `:175`、`selectable(bool)` `:196`、
  `min_width(...)` `:202`、`max_width(...)` `:215`；`FluentBuilder` `:228`

* `TableDelegate`：`CMP\src\table\delegate.rs:16`

```rust
pub trait TableDelegate: Sized + 'static {
    fn columns_count(&self, cx: &App) -> usize;                              // :18 必需
    fn rows_count(&self, cx: &App) -> usize;                                 // :21 必需
    fn column(&self, col_ix: usize, cx: &App) -> Column;                     // :26 必需
    fn perform_sort(&mut self, col_ix, sort: ColumnSort, window, cx) {}      // :29
    fn render_header(&mut self, window, cx) -> Stateful<Div> { div().id("header") } // :39
    fn group_headers(&self, cx: &App) -> Option<Vec<Vec<ColumnGroup>>> { None }     // :50
    fn render_group_th(...)                                                  // :56
    fn render_th(...)                                                        // :77
    fn render_tr(&mut self, row_ix, window, cx) -> impl IntoElement          // :91
    fn render_td(&mut self, row_ix, col_ix, window, cx) -> impl IntoElement; // :112 必需
    fn context_menu(...) -> PopupMenu                                        // :101
    fn move_column(&mut self, col_ix, to_ix, window, cx) {}                  // :123
    fn render_empty(&mut self, window, cx) -> impl IntoElement               // :133
    fn loading(&self, cx: &App) -> bool { false }                            // :147
    fn render_loading(...)                                                   // :154
    fn has_more(&self, cx: &App) -> bool { false }                           // :166
    fn load_more_threshold(&self) -> usize                                   // :175
    fn load_more(&mut self, window, cx) {}                                   // :185
    fn render_last_empty_col(...)                                            // :188
    fn visible_rows_changed(...)                                             // :202
    fn visible_columns_changed(...)                                          // :216
    fn cell_text(&self, row_ix, col_ix, cx: &App) -> String                  // :228
}
```

* `TableState<D: TableDelegate>`：`CMP\src\table\state.rs:190`
  `new(delegate: D, _: &mut Window, cx: &mut Context<Self>)` `:267`、
  `loop_selection(bool)` `:315`、`col_movable(bool)` `:321`、`col_resizable(bool)` `:327`、
  `sortable(bool)` `:333`、`row_selectable(bool)` `:339`、`col_selectable(bool)` `:345`、
  `cell_selectable(bool)` `:367`、`row_header(bool)` `:382`、`refresh(cx)` `:388`、
  `scroll_to_row(row_ix, cx)` `:393`、`scroll_to_col(col_ix, cx)` `:400`、
  `selected_row()` `:458`、`set_selected_row(row_ix, cx)` `:463`、`right_clicked_row()` `:489`、
  `selected_col()` `:503`、`selected_cell()` `:529`、`clear_selection(cx)` `:562`、
  `visible_range()` `:574`、`headers(cx)` `:582`、`dump(cx)` `:599`、`dump_range(range, cx)` `:610`
* `TableEvent`：`CMP\src\table\state.rs:61`，变体
  `SelectRow(usize)` `:63`、`DoubleClickedRow(usize)` `:65`、`SelectColumn(usize)` `:67`、
  `SelectCell(usize, usize)` `:74`、`DoubleClickedCell(usize, usize)` `:81`、
  `ColumnWidthsChanged(Vec<Pixels>)` `:85`、`MoveColumn(usize, usize)` `:90`、
  `RightClickedRow(Option<usize>)` `:95`、`RightClickedCell(usize, usize)` `:103`、`ClearSelection` `:107`
* `DataTable<D: TableDelegate>`：`CMP\src\table\data_table.rs:91`，`#[derive(IntoElement)]`，
  `RenderOnce` `:141`，**只额外实现了 `Sizable`（`:131`）—— 没有 `Styled`**

**关键事实（坑）**：`DataTable` 不是 `Styled`，`.bg(...)` / `.p_4()` / `.flex_1()` 都不解析。
要加样式必须包一层 `div()`。

```rust
use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_kit::component::Sizable as _;      // 想用 .small()/.with_size() 才需要

struct FilesTable { rows: Vec<(String, String)> }

impl TableDelegate for FilesTable {
    fn columns_count(&self, _cx: &App) -> usize { 2 }                       // :18
    fn rows_count(&self, _cx: &App) -> usize { self.rows.len() }            // :21
    fn column(&self, ix: usize, _cx: &App) -> Column {                      // :26
        match ix { 0 => Column::new("name", "Name"), _ => Column::new("size", "Size") }
    }
    fn render_td(&mut self, row_ix: usize, col_ix: usize, _: &mut Window,
                 _: &mut Context<TableState<Self>>) -> impl IntoElement {   // :112
        let row = &self.rows[row_ix];
        if col_ix == 0 { row.0.clone() } else { row.1.clone() }
    }
}

let state = cx.new(|cx| TableState::new(FilesTable { rows }, window, cx));   // state.rs:267
// DataTable 没有 Styled，需要包裹：
div().size_full().child(DataTable::new(&state).stripe(true).bordered(true)); // :101 / :109 / :115
```

### 2.4 `base::v_virtual_list` / `base::h_virtual_list`

两者都能用，因为 `CMP\src\virtual_list.rs:2` 就是原样转发 `gpui_base`：

```rust
// CMP\src\virtual_list.rs:2
pub use gpui_base::{VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list};
// CMP\src\lib.rs:122 又把它放到 gpui_kit::component 根
pub use virtual_list::{VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list};
```

所以 `gpui_kit::base::v_virtual_list`、`gpui_kit::component::v_virtual_list`、
`gpui_kit::component::virtual_list::v_virtual_list` 都指向同一个函数。

```rust
// BASE\src\virtual_list.rs:139
pub fn v_virtual_list<R, V>(
    view: Entity<V>,
    id: impl Into<ElementId>,
    item_sizes: Rc<Vec<Size<Pixels>>>,          // ← 每个 item 的 Size，不是行高
    f: impl 'static + Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>,
) -> VirtualList where R: IntoElement, V: Render;

// BASE\src\virtual_list.rs:160
pub fn h_virtual_list<R, V>(...)  // 同上，Axis::Horizontal
```

⚠️ 三点与「直觉/0.7 文档」不同：

1. 参数是 **`Rc<Vec<Size<Pixels>>>`**（完整尺寸），不是「行高 `Pixels`」。
   文档注释明确：垂直列表只用 `height`，`width` 由 `with_item_to_measure_index` 指定项测量
   （`BASE\src\virtual_list.rs:133-135`）。
2. 闭包拿的是 **`Range<usize>`**，要自己按 range 返回 `Vec<R>`。
3. `VirtualList` 是 `IntoElement` + `Element`（`BASE\src\virtual_list.rs:414`、`:422`），
   `Styled`（`:229`），方法：`track_scroll` `:236`、`with_sizing_behavior` `:243`、
   `with_item_to_measure_index` `:249`、`with_scroll_handle` `:258`。
4. `GPUI\src\elements\uniform_list.rs:22` 的 `uniform_list(id, item_count, f)` 是 **GPUI 原生**的另一个函数
   （`f: Fn(Range<usize>, &mut Window, &mut App) -> Vec<R>`）；它**不是** gpui-kit 的虚拟列表，
   没有 gpui-kit 的滚动条/尺寸投影。

```rust
use gpui_kit::component::v_virtual_list;      // 或 gpui_kit::base::v_virtual_list
use gpui_kit::{Entity, Size, px};
use std::rc::Rc;

let row_h = px(26.);
let sizes: Rc<Vec<Size<Pixels>>> =
    Rc::new((0..self.rows.len()).map(|_| gpui::size(px(0.), row_h)).collect());

v_virtual_list(
    cx.entity(),                              // Entity<V>
    "file-list",                              // impl Into<ElementId>
    sizes,
    |this, range, _window, _cx| {             // Range<usize>
        range.map(|ix| div().h(row_h).child(this.rows[ix].clone())).collect()
    },
)
```

### 2.5 `component::description_list::DescriptionList`

导入：`gpui_kit::component::description_list::{DescriptionList, DescriptionItem, DescriptionText}`。
（`DescriptionText` 是**枚举**，不是 trait：`CMP\src\description_list.rs:31`）

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `() -> Self` | `:126` |
| `vertical` | `() -> Self` | `:138` |
| `horizontal` | `() -> Self` | `:143` |
| `label_width` | `(impl Into<DefiniteLength>) -> Self` | `:150` |
| `layout` | `(Axis) -> Self` | `:156` |
| `bordered` | `(bool) -> Self` | `:164` |
| `columns` | `(usize) -> Self` | `:172` |
| `item` | `(...)` | `:178` |
| `child` | `(impl Into<DescriptionItem>) -> Self` | `:193` |
| `children` | `(...)` | `:199` |
| `separator` | `() -> Self` | `:209` |

* `DescriptionItem::new(label: impl Into<DescriptionText>)` `:81`、`value(...)` `:90`、`span(usize)` `:101`
* `DescriptionList` 实现 `Sizable` `:243`、`RenderOnce` `:250`

```rust
use gpui_kit::component::description_list::{DescriptionItem, DescriptionList};

DescriptionList::vertical()                                     // :138
    .columns(1)                                                 // :172
    .child(DescriptionItem::new("Branch").value("main"))         // :81 / :90
    .child(DescriptionItem::new("Status").value("clean"))
    .separator()                                                // :209
```

### 2.6 `component::tag::Tag`

导入：`gpui_kit::component::tag::{Tag, TagVariant}`。`Tag`：`CMP\src\tag.rs:125`；
`TagVariant`：`CMP\src\tag.rs:10`（`Primary` `:11`、`Secondary`(default) `:13`、`Danger` `:14`、
`Success` `:15`、`Warning` `:16`、`Info` `:17`、`Color(ColorName)` `:18`、`Custom{..}` `:19`）。

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `() -> Self` | `:135` |
| `primary` / `secondary` / `danger` / `success` / `warning` / `info` | `() -> Self` | `:147`/`:152`/`:157`/`:162`/`:167`/`:172` |
| `custom` | `(color: Hsla, foreground: Hsla, border: Hsla) -> Self` | `:177` |
| `color` | `(impl Into<ColorName>) -> Self` | `:186` |
| `with_variant` | `(TagVariant) -> Self` | `:191` |
| `outline` | `() -> Self` | `:197` |
| `rounded` | `(impl Into<AbsoluteLength>) -> Self` | `:203` |
| `rounded_full` | `() -> Self` | `:209` |

`Sizable` `:215`、`ParentElement` `:222`、`Styled` `:228`、`RenderOnce` `:234`

### 2.7 `component::badge::Badge`

导入：`gpui_kit::component::badge::Badge`。`CMP\src\badge.rs:31`。
`BadgeVariant` 是 **`enum` 但非 `pub`**（`:9 enum BadgeVariant`）—— 应用无法直接命名它。

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `() -> Self` | `:43` |
| `dot` | `() -> Self` | `:56` |
| `count` | `(usize) -> Self` | `:64` |
| `icon` | `(impl Into<Icon>) -> Self` | `:70` |
| `max` | `(usize) -> Self` | `:76` |
| `color` | `(impl Into<Hsla>) -> Self` | `:82` |

`ParentElement` `:88`、`Sizable` `:94`、`RenderOnce` `:101`

### 2.8 `component::kbd::Kbd`

导入：`gpui_kit::component::kbd::Kbd`（`CMP\src\kbd.rs`）。

⚠️ **`Kbd::new` 收的是 `Keystroke`，不是 `&str`**：

```rust
pub fn new(stroke: Keystroke) -> Self                                   // CMP\src\kbd.rs:31
impl From<Keystroke> for Kbd { ... }                                    // :18
pub fn appearance(mut self, appearance: bool) -> Self                    // :41
pub fn outline(mut self) -> Self                                        // :47
pub fn binding_for_action(action: &dyn Action, window: &Window) -> Option<Self>      // :53
pub fn binding_for_action_in(action: &dyn Action, context: &str, window: &Window) -> Option<Self> // :74
pub fn global_binding_for_action(action: &dyn Action, window: &Window) -> Option<Self> // :85
pub fn format(key: &Keystroke) -> String                                // :100
```

`Styled` `:221`、`RenderOnce` `:227`

```rust
use gpui_kit::component::kbd::Kbd;
use gpui_kit::Keystroke;

Kbd::new(Keystroke::parse("cmd-s").unwrap())        // kbd.rs:31（Keystroke 来自 gpui_kit::*）
```

### 2.9 `component::empty`

导入：`gpui_kit::component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle, EmptyDescription, EmptyContent}`。

| 类型 | 位置 | 构造 | 关键 builder |
| --- | --- | --- | --- |
| `Empty` | `CMP\src\empty.rs:14`，`RenderOnce` `:63`，`Styled` `:57`，`ParentElement` `:51` | `new()` `:23` | `header(EmptyHeader)` `:33`、`content(EmptyContent)` `:39` |
| `EmptyHeader` | `:89`，`RenderOnce` `:138`，`Styled` `:132` | `new()` `:98` | `media(EmptyMedia)` `:108`、`title(EmptyTitle)` `:114`、`description(EmptyDescription)` `:120` |
| `EmptyMedia` | `:167`，`RenderOnce` `:208`，`Styled` `:202`，`ParentElement` `:196` | `new()` `:175` | `with_variant(EmptyMediaVariant)` `:184` |
| `EmptyMediaVariant` | `:157` | `enum { Default, Icon }` | — |
| `EmptyTitle` | `:231`，`RenderOnce` `:264`，`Styled` `:258`，`ParentElement` `:252` | `new()` `:238` | — |
| `EmptyDescription` | `:279`，`RenderOnce` `:312`，`Styled` `:306`，`ParentElement` `:300` | `new()` `:286` | — |
| `EmptyContent` | `:328`，`RenderOnce` `:361`，`Styled` `:355`，`ParentElement` `:349` | `new()` `:335` | — |

⚠️ `EmptyHeader::media/title/description` 收的是**具体类型**（`EmptyMedia`/`EmptyTitle`/`EmptyDescription`），
不是 `impl IntoElement`；内容要通过这些包装类型的 `ParentElement::child`/`children` 放进**去**。

```rust
use gpui_kit::component::empty::{
    Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant, EmptyTitle,
};
use gpui_kit::component::{h_flex, Icon, IconName, StyledExt as _};

Empty::new()                                                      // empty.rs:23
    .header(
        EmptyHeader::new()                                        // :98
            .media(EmptyMedia::new().with_variant(EmptyMediaVariant::Icon)   // :175 / :184
                .child(Icon::new(IconName::Inbox)))
            .title(EmptyTitle::new().child("No files"))            // :238
            .description(EmptyDescription::new().child("Open a folder to start")), // :286
    )
```

### 2.10 `component::message_scroller`

导入：`gpui_kit::component::message_scroller::{MessageScroller, MessageScrollerState}`。

* `MessageScrollerState`：`CMP\src\message_scroller.rs:26`
  `new(item_count: usize, cx: &mut Context<Self>)` `:36`、`item_count()` `:53`、
  `is_scrolled_up()` `:58`、`is_following_tail()` `:65`、`reset(item_count, cx)` `:70`、
  `splice(...)` `:80`、`append(count, cx) -> bool` `:109`、`prepend(count, cx) -> bool` `:115`、
  `remeasure(cx)` `:120`、`remeasure_items(range, cx) -> bool` `:128`、
  `scroll_to_item(index, cx) -> bool` `:139`、`scroll_to_end(cx)` `:153`
* `MessageScroller`：`CMP\src\message_scroller.rs:166`，`#[derive(IntoElement)]`，`RenderOnce` `:296`，`Styled` `:290`

```rust
// CMP\src\message_scroller.rs:185
pub fn new<E>(id: impl Into<ElementId>, state: Entity<MessageScrollerState>,
              renderer: impl FnMut(usize, &mut Window, &mut App) -> E + 'static) -> Self
where E: IntoElement;

pub fn scrollbar(bool) -> Self                     // :215
pub fn jump_button(bool) -> Self                    // :221
pub fn with_jump_button_label(impl Into<SharedString>) -> Self   // :227
pub fn with_content_style(StyleRefinement) -> Self  // :233
pub fn with_list_style(StyleRefinement) -> Self     // :239
pub fn with_row_style(StyleRefinement) -> Self      // :245
pub fn with_jump_button_style(StyleRefinement) -> Self // :251
pub fn with_jump_button_renderer(...) -> Self       // :260
pub fn with_jump_button_transition(Duration) -> Self // :272
pub fn with_bottom_fade(impl Into<Hsla>) -> Self    // :284
```

```rust
use gpui_kit::component::message_scroller::{MessageScroller, MessageScrollerState};

let state = cx.new(|cx| MessageScrollerState::new(0, cx));         // :36
MessageScroller::new("chat", state.clone(), |ix, _window, _cx| {   // :185
    div().child(format!("message {ix}"))
})
.jump_button(true)                                                 // :221
```

---

## 3. 标签与动作

### 3.1 `component::tab::{Tab, TabBar, TabVariant}`

导入：`gpui_kit::component::tab::{Tab, TabBar, TabVariant}`（`CMP\src\tab\mod.rs:4-5`）。

* `TabVariant`：`CMP\src\tab\tab.rs:14 pub enum TabVariant { Tab(default), Outline, Pill, Segmented, Underline }`
* `Tab`：`CMP\src\tab\tab.rs:397`，`#[derive(IntoElement)]`，`RenderOnce` `:619`，`Styled` `:606`，
  `ParentElement` `:581`，`Selectable` `:587`，`Sizable` `:612`，`InteractiveElement` `:598`

  ⚠️ **`Tab::new()` 不带 id**（`:478 pub fn new() -> Self`）。id 由 `TabBar` 决定。
  `label(impl Into<SharedString>)` `:483`、`aria_label(...)` `:489`、`icon(impl Into<Icon>)` `:499`、
  `with_variant(TabVariant)` `:505`、`pill()` `:511`、`outline()` `:517`、`segmented()` `:523`、
  `underline()` `:529`、`prefix(impl IntoElement)` `:535`、`suffix(impl IntoElement)` `:541`、
  `disabled(bool)` `:547`（**固有方法**）、`on_click(...)` `:553`
  以及 `From<&'static str>` `:422`、`From<String>` `:428`、`From<SharedString>` `:434`、
  `From<Icon>` `:440`、`From<IconName>` `:446`
* `TabBar`：`CMP\src\tab\tab_bar.rs:40`，`RenderOnce` `:359`，`Styled` `:346`，`Sizable` `:352`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `(id: impl Into<ElementId>) -> Self` | `:59` |
| `with_variant` | `(TabVariant) -> Self` | `:80` |
| `pill`/`outline`/`segmented`/`underline` | `() -> Self` | `:86`/`:92`/`:98`/`:104` |
| `menu` | `(bool) -> Self` | `:110` |
| `max_width` | `(impl Into<Pixels>) -> Self` | `:118` |
| `track_scroll` | `(&ScrollHandle) -> Self` | `:127` |
| `prefix` / `suffix` | `(impl IntoElement) -> Self` | `:133` / `:139` |
| `children` | `(impl IntoIterator<Item = impl Into<Tab>>) -> Self` | `:145` |
| `child` | `(impl Into<Tab>) -> Self` | `:151` |
| `selected_index` | `(usize) -> Self` | `:157` |
| `last_empty_space` | `(impl IntoElement) -> Self` | `:163` |
| `on_click` | `(F) -> Self` | `:171` |

```rust
use gpui_kit::component::tab::{Tab, TabBar};

TabBar::new("editor-tabs")                                  // TabBar::new(id)
    .pill()                                                 // :86
    .selected_index(0)                                      // :157
    .child(Tab::new().label("main.rs").icon(IconName::FileText))   // Tab::new() 无参
    .child(Tab::new().label("lib.rs").on_click(cx.listener(|this, _, _, cx| { /* ... */ })))
```

### 3.2 `component::button`

导入：`gpui_kit::component::button::{Button, ButtonGroup, ButtonVariants, Toggle, ToggleVariants, DropdownButton, ButtonVariant, ButtonRounded, ButtonIcon, ButtonCustomVariant, ToggleVariant, ToggleGroup}`
（`CMP\src\button\mod.rs:7-11`）。

#### `ButtonVariants`（trait，**必须 `as _`**）

```rust
// CMP\src\button\button.rs:44
pub trait ButtonVariants: Sized {
    fn with_variant(self, variant: ButtonVariant) -> Self;   // :45 唯一必需
    fn primary(self) -> Self;    // :48 默认实现
    fn secondary(self) -> Self;  // :53
    fn danger(self) -> Self;     // :58
    fn warning(self) -> Self;    // :63
    fn success(self) -> Self;    // :68
    fn info(self) -> Self;       // :73
    fn ghost(self) -> Self;      // :78
    fn link(self) -> Self;       // :83
    fn text(self) -> Self;       // :88
    fn custom(self, style: ButtonCustomVariant) -> Self; // :93
}
```

`impl ButtonVariants for`：`Button` `:539`、`ButtonGroup` `:145`、`DropdownButton` `:130`
（`Toggle`/`ToggleGroup` 用的是 `ToggleVariants`：`CMP\src\button\toggle.rs:20`，
提供 `ghost` `:24`、`outline` `:28`、`with_variant` `:22`）。

#### `Button`

* 类型：`CMP\src\button\button.rs:186 pub struct Button`，`RenderOnce` `:564`、
  `Styled` `:546`、`ParentElement` `:552`、`InteractiveElement` `:558`、`From<Button> for AnyElement` `:226`
* **固有**方法（不需要 trait 导入）：`new(id: impl Into<ElementId>)` `:233`、`role(impl Into<RoleOverride>)` `:312`、
  `outline()` `:318`、`rounded(impl Into<ButtonRounded>)` `:324`、`label(impl Into<SharedString>)` `:357`、
  `accessibility_id` `:363`、`accessibility_label` `:377`、`icon(impl Into<ButtonIcon>)` `:383`、
  `tooltip(impl Into<SharedString>)` `:389`、`tooltip_placement(Placement)` `:398`、`tooltip_with_action` `:404`、
  `loading(bool)` `:421`、`compact()` `:427`、`on_click(...)` `:433`、`on_hover(...)` `:442`、
  `loading_icon(impl Into<Icon>)` `:450`、`tab_index(isize)` `:458`、`tab_stop(bool)` `:466`、
  `dropdown_caret(bool)` `:472`、`toggled(bool)` `:483`
* **trait**方法（必须导入）：`disabled`（`Disableable` `:503`）、`selected`/`is_selected`（`Selectable` `:521`）、
  `focus_ring`/`is_focus_ring_enabled`（`FocusableExt` `:510`）、
  `with_size`/`xsmall`/`small`/`large`（`Sizable` `:532`）、
  `primary`/`ghost`/…（`ButtonVariants` `:539`）
* `ButtonVariant`：`CMP\src\button\button.rs:142`；`ButtonRounded`：`:20`；
  `ButtonCustomVariant`：`:36`（`new(cx: &App)` `:99`）
* `ButtonGroup`：`CMP\src\button\button_group.rs:18`，`RenderOnce` `:152`、`Styled` `:139`、`Sizable` `:132`、`Disableable` `:35`、`ButtonVariants` `:145`
  `new(id: impl Into<ElementId>)` `:44`、`child(Button)` `:61`、`children(impl IntoIterator<Item = Button>)` `:67`、
  `multiple(bool)` `:73`、`layout(Axis)` `:79`、`compact()` `:87`、`outline()` `:95`、`on_click(...)` `:123`
  —— ⚠️ **`child` 只收 `Button`**（`:61`），其它元素塞不进去
* `Toggle`：`CMP\src\button\toggle.rs:34`，`RenderOnce` `:149`、`Styled` `:143`、`ParentElement` `:123`、`Sizable` `:136`、`Disableable` `:129`、`ToggleVariants` `:116`
  `new(id)` `:50`、`tooltip(...)` `:72`、`label(...)` `:78`、`icon(...)` `:85`、`checked(bool)` `:92`、
  `on_click(impl Fn(&bool, &mut Window, &mut App) + 'static)` `:100`
* `DropdownButton`：`CMP\src\button\dropdown_button.rs:25`，`RenderOnce` `:148`、`Styled` `:117`、`Sizable` `:123`、`Disableable` `:110`、`Selectable` `:137`、`ButtonVariants` `:130`
  `new(id)` `:43`、`button(Button)` `:76`、`dropdown_menu(...)` `:82`、`dropdown_menu_with_anchor(...)` `:91`、`outline()` `:104`

```rust
use gpui_kit::component::button::{Button, ButtonVariants as _, ButtonGroup};
use gpui_kit::component::{Disableable as _, Selectable as _, Sizable as _};
use gpui_kit::*;

Button::new("save")                 // 固有 :233
    .label("Save")                  // 固有 :357
    .icon(IconName::Check)
    .primary()                      // ButtonVariants :48 —— 必须 use ... as _
    .small()                        // Sizable :193（默认实现）—— 必须 use ... as _
    .selected(false)                // Selectable :4 —— 必须 use ... as _
    .disabled(false)                // Disableable :15 —— 必须 use ... as _
    .on_click(cx.listener(|this, _, _, cx| { /* ... */ }))   // 固有 :433
```

### 3.3 `component::Icon` / `IconName` —— **有两个 `IconName`，这是最大的坑之一**

| 名称 | 实际类型 | 变体数 | 导入路径 | 证据 |
| --- | --- | --- | --- | --- |
| 组件 `IconName` | 独立 enum（**兼容用窄枚举**） | 101 | `gpui_kit::component::IconName` | `CMP\src\icon.rs:18`（由 `ASSETS` 的宏 `__component_icon_names!` 生成，`CMP\src\icon.rs:38`） |
| 资源 `IconName` | 完整 Lucide 目录 enum | 1830 | `gpui_kit::assets::IconName` | `ASSETS\src\lib.rs:8`；`ASSETS\src\icon.rs`；由 `ASSETS\build.rs:54-67` 生成 |

* 两者之间只有 **组件 → 资源** 的 `From`（`CMP\src\icon.rs:22`），反向没有。
* 只有 `assets::IconName` 有 **`ALL: &'static [Self]`**（`ASSETS\build.rs:59-62`）和**固有** `path(self) -> SharedString`
  （`ASSETS\build.rs:63-67`）。组件 `IconName` 的 `path()` 只能通过 `IconNamed` trait 拿到
  （`CMP\src\icon.rs:30`），它**没有** `ALL`。
* 两者都实现 `IconNamed`（`CMP\src\icon.rs:30`、`ASSETS\src\icon.rs`），
  所以 `Icon::new(...)`（`CMP\src\icon.rs:105`，泛型 `impl Into<Icon>` + 全实现
  `impl<T: IconNamed> From<T> for Icon`，`CMP\src\icon.rs:59`）两者都能吃。
* `.view(cx) -> Entity<Icon>`：组件 `IconName` 有**固有** `view`（`CMP\src\icon.rs:42`）；
  资源 `IconName` 只能通过 **`IconNameExt` trait**（`CMP\src\icon.rs:67`，impl 在 `:71`），
  需要 `use gpui_kit::component::IconNameExt as _;`
* 想在代码里用 `IconName::GitBranch`，必须用 `gpui_kit::assets::IconName`，
  并且窗口的 `AssetSource` 必须是 `AllAssets`（见 §7）。

`Icon`：`CMP\src\icon.rs:84`，`#[derive(Clone, IntoElement)]`，`RenderOnce` `:216`、`Render` `:229`、`Styled` `:198`、`Sizable` `:209`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `(icon: impl Into<Icon>) -> Self` | `:105` |
| `path` | `(impl Into<SharedString>) -> Self` | `:117` |
| `data` | `(&[u8]) -> Self` | `:136` |
| `view` | `(cx: &mut App) -> Entity<Icon>` | `:147` |
| `transform` | `(gpui::Transformation) -> Self` | `:152` |
| `empty` | `() -> Self` | `:157` |
| `rotate` | `(impl Into<Radians>) -> Self` | `:164` |
| `text_color` | `(impl Into<Hsla>) -> Self` | `:203` |

**`Icon` 的尺寸方法全部在 trait 上**：

* `small()` / `xsmall()` / `large()` / `with_size(impl Into<Size>)` → **`Sizable`**（`CMP\src\icon.rs:209` + `CMP\src\sizing.rs:178-202`）
* `size_3()` / `size_3p5()` / `size_4()` / `size_6()` / `size_12()` → **`gpui::Styled`**
  （由 `GPMAC\src\styles.rs:40 style_helpers!()` 在 `gpui::Styled` 上生成；
  `size` 前缀在 `GPMAC\src\styles.rs:858`，档位表在 `:926-1021`）。
  `Icon` 自己在 `CMP\src\icon.rs:181-187` 用它们，映射关系写在 `CMP\src\icon.rs:183-186`：
  `XSmall→size_3`、`Small→size_3p5`、`Medium→size_4`、`Large→size_6`。

```rust
use gpui_kit::component::{Icon, IconName, Sizable as _};   // 组件窄枚举（101 个）
use gpui_kit::assets::IconName as CatalogIcon;             // 完整目录（1830 个）
use gpui_kit::Styled;                                      // size_3/size_4 来自 gpui::Styled

Icon::new(IconName::FileText).small()      // Sizable::small
Icon::new(IconName::Search).size_4()       // Styled::size_4（= 1rem = 16px @ rem=16）
Icon::new(CatalogIcon::GitBranch)          // 需要 AllAssets，否则加载失败
```

### 3.4 `Sizable` / `Disableable` / `Selectable` 的确切方法集

```rust
// CMP\src\sizing.rs:178
pub trait Sizable: Sized {
    fn with_size(mut self, size: impl Into<Size>) -> Self;   // :183 必需
    fn xsmall(self) -> Self { self.with_size(Size::XSmall) }  // :187
    fn small(self) -> Self { self.with_size(Size::Small) }    // :193
    fn large(self) -> Self { self.with_size(Size::Large) }    // :199
}
// 注意：没有 `medium()`，因为 Medium 是默认；没有 `size_3()`（那是 Styled 的）。

// BASE\src\component_traits.rs:3
pub trait Selectable: Sized {
    fn selected(mut self, selected: bool) -> Self;   // :4 必需
    fn is_selected(&self) -> bool;                   // :5 必需
    fn secondary_selected(self, _: bool) -> Self { self }  // :7 默认 no-op
}

// BASE\src\component_traits.rs:14
pub trait Disableable {
    fn disabled(mut self, disabled: bool) -> Self;   // :15 必需
}
```

可用的 `Size` 档位（`CMP\src\sizing.rs:6-13`）：`Size(Pixels)`、`XSmall`、`Small`、`Medium`(default)、`Large`。
`Size::from_str`（`:45`）接受 `"xs"|"xsmall"|"sm"|"small"|"md"|"medium"|"lg"|"large"`。

---

## 4. 输入

### 4.1 `component::input`

导入：`gpui_kit::component::input::{Input, InputState, Textarea, TextareaState, Editor, EditorState, NumberInput, NumberInputEvent, AnyInputState, InputEvent, InputContentType, Rope, RopeExt}`。
（`CMP\src\input\mod.rs:19-51`）

`InputState` / `TextareaState` / `EditorState` 是 **`gpui_base` 的类型别名**：

```rust
pub type InputState    = InputBaseState<InputMode>;      // BASE\src\input\input\mod.rs:10
pub type TextareaState = InputBaseState<TextareaMode>;   // BASE\src\input\textarea\mod.rs:10
pub type EditorState   = InputBaseState<EditorMode>;     // BASE\src\input\editor\mod.rs:11
```

构造（**都是 `window, cx`，不是 `Entity` 工厂**）：

```rust
InputState::new(window: &mut Window, cx: &mut Context<Self>) -> Self      // BASE\src\input\base\state.rs:8958
TextareaState::new(window: &mut Window, cx: &mut Context<Self>) -> Self   // :9163
EditorState::new(window: &mut Window, cx: &mut Context<Self>) -> Self     // :9230
```

常用 `InputBaseState` 方法：`placeholder()` `:518`、`placeholder(mut self, impl Into<SharedString>)` `:766`、
`set_value(...)` `:897`、`value() -> SharedString` `:1197`

`Input`（组件外观层）：`CMP\src\input\input.rs:111`，`RenderOnce` `:487`、`Styled` `:481`、`Sizable` `:143`、`Selectable` `:150`、`FocusableExt` `:161`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `(state: &Entity<InputState>) -> Self` | `:180` |
| `id` | `(impl Into<ElementId>) -> Self` | `:174` |
| `accessibility_id` / `aria_label` | `(impl Into<SharedString>) -> Self` | `:230` / `:235` |
| `prefix` / `suffix` | `(impl IntoElement) -> Self` | `:240` / `:245` |
| `h_full` | `() -> Self` | `:251` |
| `h` | `(impl Into<DefiniteLength>) -> Self` | `:257` |
| `appearance` | `(bool) -> Self` | `:263` |
| `bordered` | `(bool) -> Self` | `:269` |
| `focus_bordered` | `(bool) -> Self` | `:275` |
| `cleanable` | `(bool) -> Self` | `:281` |
| `mask_toggle` | `() -> Self` | `:287` |
| `content_type` | `(InputContentType) -> Self` | `:296` |
| `role` | `(impl Into<RoleOverride>) -> Self` | `:304` |
| `disabled` | `(bool) -> Self` | `:310`（**固有**） |
| `readonly` | `(bool) -> Self` | `:320` |
| `tab_index` | `(isize) -> Self` | `:326` |
| `context_menu` / `on_paste` | — | `:334` / `:355` |

`Textarea`：`CMP\src\input\textarea.rs:14`，`RenderOnce` `:162`、`Styled` `:132`
`new(state: &Entity<TextareaState>)` `:36`、`h(impl Into<DefiniteLength>)` `:54`、`appearance(bool)` `:59`、
`bordered(bool)` `:64`、`disabled(bool)` `:69`、`readonly(bool)` `:79`、`tab_index(isize)` `:84`、
`role(...)` `:89`、`accessibility_id(...)` `:95`、`aria_label(...)` `:100`、`context_menu` `:109`、`on_paste` `:123`

`Editor`：`CMP\src\input\editor.rs:18`，`RenderOnce` `:134`、`Styled` `:128`
`new(state: &Entity<EditorState>)` `:39`、`h(impl Into<DefiniteLength>)` `:56`、`appearance(bool)` `:61`、
`bordered(bool)` `:66`、`disabled(bool)` `:71`、`readonly(bool)` `:81`、`tab_index(isize)` `:86`、
`role(...)` `:91`、`aria_label(...)` `:96`、`context_menu` `:105`、`on_paste` `:119`

⚠️ **`Textarea`/`Editor` 没有 `Sizable` 实现**（只有 `Input` 有 `Sizable` `:143`）。

`NumberInput`：`CMP\src\input\number_input.rs:21`，`RenderOnce` `:111`、`Styled` `:105`、`Sizable` `:98`、
`Disableable` `:74`、`FocusableExt` `:81`、`Focusable` `:92`

```rust
pub fn new(state: &Entity<InputState>) -> Self   // CMP\src\input\number_input.rs:35
// 注意：它吃的是普通 InputState，没有 NumberInputState
pub fn placeholder(impl Into<SharedString>) -> Self  // :50
pub fn prefix(impl IntoElement) -> Self             // :56
pub fn suffix(impl IntoElement) -> Self             // :62
pub fn appearance(bool) -> Self                     // :68
```

`NumberInputEvent`：`BASE\src\number_input.rs:57`，`impl EventEmitter<NumberInputEvent> for InputState`
（`BASE\src\number_input.rs:60`）—— 事件挂在 **`InputState`** 上，订阅时用 `InputState` 的实体。
`NumberStep` `:32`、`StepAction` `:25`、`step_value` `:331`。

`AnyInputState`：`CMP\src\input\state.rs:17`，`as_input()` `:240`、`as_textarea()` `:248`、
`as_editor()` `:256`、`as_otp()` `:264`、`value(cx)` `:274`、`focus_handle(cx)` `:284`

最小片段：

```rust
use gpui_kit::component::input::{Input, InputState, Editor, EditorState};
use gpui_kit::component::Sizable as _;         // Input::small()
use gpui_kit::*;

struct Demo { name: Entity<InputState>, code: Entity<EditorState> }

// 构造（在 cx.new 里）：
let name = cx.new(|cx| InputState::new(window, cx).placeholder("file name"));
let code = cx.new(|cx| EditorState::new(window, cx));

// render：
Input::new(&self.name).id("name").cleanable(true).small()   // :180 / :174 / :281 / Sizable
Editor::new(&self.code).h(px(400.)).bordered(true)          // :39 / :56 / :66
```

### 4.2 `component::checkbox::Checkbox`

`CMP\src\checkbox.rs:17`，`RenderOnce` `:215`、`Styled` `:127`、`InteractiveElement` `:120`、
`StatefulInteractiveElement` `:125`、`Disableable` `:133`、`FocusableExt` `:140`、`Selectable` `:151`、
`ParentElement` `:161`、`Sizable` `:167`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `(id: impl Into<ElementId>) -> Self` | `:39` |
| `role` | `(impl Into<RoleOverride>) -> Self` | `:60` |
| `tooltip` | `(impl Into<SharedString>) -> Self` | `:66` |
| `label` | `(impl Into<Text>) -> Self` | `:72` |
| `accessibility_label` | `(impl Into<SharedString>) -> Self` | `:80` |
| `checked` | `(bool) -> Self` | `:86` |
| `on_click` | `(impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self` | `:92` |
| `on_change` | `(impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self` | `:102` |
| `tab_stop` / `tab_index` | `(bool)` / `(isize)` | `:108` / `:114` |

⚠️ `label` 收 **`impl Into<Text>`**（不是 `SharedString`）。

### 4.3 `component::switch::Switch`

`CMP\src\switch.rs:14`，`RenderOnce` `:127`、`Styled` `:107`、`Sizable` `:113`、`Disableable` `:120`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `(id: impl Into<ElementId>) -> Self` | `:31` |
| `checked` | `(bool) -> Self` | `:49` |
| `label` | `(impl Into<Text>) -> Self` | `:55` |
| `accessibility_label` | `(impl Into<SharedString>) -> Self` | `:66` |
| `on_click` | `<F>(F) -> Self` | `:72` |
| `on_change` | `<F>(F) -> Self` | `:85` |
| `color` | `(impl Into<Hsla>) -> Self` | `:95` |
| `tooltip` | `(impl Into<SharedString>) -> Self` | `:101` |

`Switch` **没有** `InteractiveElement`/`ParentElement` 实现，也没有固有 `disabled`。

### 4.4 `component::select::{Select, SelectState}`

`Select` 和 `SelectState` 都是**泛型**：`SelectState<D: SearchableListDelegate + 'static>`
（`CMP\src\select.rs:118`）、`Select<D: SearchableListDelegate + 'static>`（`:133`）。

```rust
// CMP\src\select.rs:149  —— 注意多一个 selected_index
pub fn new(delegate: D, selected_index: Option<IndexPath>,
           window: &mut Window, cx: &mut Context<Self>) -> Self
where <D::Item as SearchableListItem>::Value: PartialEq + Clone;   // :146

// CMP\src\select.rs:642
pub fn new(state: &Entity<SelectState<D>>) -> Self
```

`SelectState` 方法：`searchable(bool)` `:300`、`set_selected_index(...)` `:306`、`set_selected_value(...)` `:334`、
`set_items(D, window, cx)` `:354`、`selected_index(cx)` `:364`、`selected_value()` `:369`、`focus(window, cx)` `:374`

`Select` 方法：`id` `:652`、`menu_width(impl Into<Length>)` `:658`、`menu_max_h` `:664`、
`placeholder` `:670`、`accessibility_label` `:679`、`icon(impl Into<Icon>)` `:685`、
`title_prefix` `:693`、`cleanable(bool)` `:699`、`search_placeholder` `:704`、
`disabled(bool)` `:711`（**固有**）、`empty<...>` `:717`、`appearance(bool)` `:728`

别名与配套类型（`CMP\src\select.rs:28-36`）：

```rust
pub use crate::searchable_list::SearchableGroup     as SelectGroup;
pub use crate::searchable_list::SearchableListDelegate as SelectDelegate;
pub use crate::searchable_list::SearchableListItem  as SelectItem;
pub use crate::searchable_list::SearchableListItemElement as SelectListItem;
pub use crate::searchable_list::SearchableVec;      // :36 —— 现成 delegate
```

`SelectEvent`：`CMP\src\select.rs:70`，只有 `Confirm(Option<<D::Item as SearchableListItem>::Value>)`（`:74`）。
订阅：`impl<D> EventEmitter<SelectEvent<D>> for SelectState<D>`（`CMP\src\select.rs:760`）；
另实现 `EventEmitter<DismissEvent>`（`:767`）。用 `cx.subscribe_in(&state, window, ...)`（`cx` 是 `Context<D>`）。

`SearchableVec<T>`：`CMP\src\searchable_list\vec.rs:76`，`new(items: impl Into<Vec<T>>)` `:83`、`push(T)` `:93`
（`SearchableVec` 要求 `T: SearchableListItem`，trait 在 `CMP\src\searchable_list\delegate.rs:8`：
`fn title(&self) -> SharedString` `:16`、`fn value(&self) -> &Self::Value` `:38` 等）

```rust
use gpui_kit::component::select::{Select, SelectState, SelectEvent, SearchableVec};
use gpui_kit::component::searchable_list::SearchableListItem;

#[derive(Clone)]
struct Choice(String);
impl SearchableListItem for Choice {                     // delegate.rs:8
    type Value = String;                                // trait 关联类型
    fn title(&self) -> SharedString { self.0.clone().into() }   // :16
    fn value(&self) -> &String { &self.0 }                      // :38
}

let items = SearchableVec::new(vec![Choice("a".into()), Choice("b".into())]);  // vec.rs:83
let state = cx.new(|cx| SelectState::new(items, None, window, cx));            // select.rs:149
Select::new(&state).placeholder("Choose…").cleanable(true)                     // :642 / :670 / :699
```

### 4.5 `component::combobox::{Combobox, ComboboxState}`

`ComboboxState<D: SearchableListDelegate + 'static>`：`CMP\src\combobox.rs:109`；
`Combobox<D: SearchableListDelegate + 'static>`：`:749`。

```rust
// CMP\src\combobox.rs:144  —— 注意收的是 Vec<IndexPath>
pub fn new(delegate: D, selected_indices: Vec<IndexPath>,
           window: &mut Window, cx: &mut Context<Self>) -> Self

// CMP\src\combobox.rs:768
pub fn new(state: &Entity<ComboboxState<D>>) -> Self
```

`ComboboxState` 方法：`multiple(bool)` `:286`、`searchable(bool)` `:292`、`selected_values()` `:298`、
`selected_value()` `:305`、`selection()` `:310`、`set_selected_values(...)` `:319`、
`set_selected_indices(...)` `:341`、`add_selected_index(...)` `:353`、`remove_selected_index(...)` `:365`、
`clear_selection(...)` `:376`、`set_items(...)` `:384`、`focus(...)` `:391`、`query(cx)` `:396`、`set_query(...)` `:401`

`Combobox` 方法：`menu_width` `:780`、`menu_max_h` `:786`、`placeholder` `:792`、`icon` `:798`、
`check_icon` `:804`、`search_placeholder` `:810`、`cleanable` `:816`、`disabled` `:822`（固有）、
`empty` `:828`、`appearance` `:839`、`render_trigger` `:845`、`footer` `:856`

`ComboboxEvent`：`CMP\src\combobox.rs:128`，`Change(Vec<Value>)` `:133`、`Confirm(Vec<Value>)` `:135`。
订阅：`impl<D> EventEmitter<ComboboxEvent<D>> for ComboboxState<D>`（`CMP\src\combobox.rs:715`）；
另实现 `EventEmitter<DismissEvent>`（`:721`）。

---

## 5. 浮层

### 5.1 `WindowExt`（trait，**必须 `as _`**）

导入：`gpui_kit::component::WindowExt as _`（`CMP\src\window_ext.rs:12`，`CMP\src\lib.rs:124` 放根上）。
实现对象：**只有 `Window`**（`CMP\src\window_ext.rs:109`）。

确切签名（`CMP\src\window_ext.rs`）：

```rust
fn open_sheet<F>(&mut self, cx: &mut App, build: F)                       // :14
    where F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;
fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F) // :19
    where F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;
fn has_active_sheet(&mut self, cx: &mut App) -> bool                       // :24
fn close_sheet(&mut self, cx: &mut App)                                    // :27

fn open_dialog<F>(&mut self, cx: &mut App, build: F)                       // :30
    where F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static;
fn open_alert_dialog<F>(&mut self, cx: &mut App, build: F)                 // :51
    where F: Fn(AlertDialog, &mut Window, &mut App) -> AlertDialog + 'static;
fn has_active_dialog(&mut self, cx: &mut App) -> bool                      // :56
fn close_dialog(&mut self, cx: &mut App)                                   // :59
fn close_all_dialogs(&mut self, cx: &mut App)                              // :62

fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App) // :65
fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App)         // :69
fn remove_notification1<T: Sized + 'static>(&mut self, key: impl Into<ElementId>, cx: &mut App) // :72
fn clear_notifications(&mut self, cx: &mut App)                            // :75
fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>> // :78

fn focused_input(&mut self, cx: &mut App) -> Option<AnyInputState>         // :86
fn has_focused_input(&mut self, cx: &mut App) -> bool                      // :88
// 已 deprecated（:92/:97/:101/:105）：selected_text / has_text_selection /
// clear_text_selection / end_text_selection —— 改用 gpui_base::TextSelection
```

### 5.2 `component::dialog`

导入：`gpui_kit::component::dialog::{Dialog, AlertDialog, DialogButtonProps, DialogFooter, DialogClose, DialogAction, DialogHeader, DialogTitle, DialogDescription, DialogContent}`
+ `gpui_kit::component::AlertDialog`（`CMP\src\lib.rs:100` 的 `pub use dialog::{...}` 也放出来了）。

`Dialog`：`CMP\src\dialog\dialog.rs:258`，`ParentElement` `:460`、`Styled` `:466`、`RenderOnce` `:506`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `(cx: &mut App) -> Self` | `:287` |
| `trigger` | `(impl IntoElement) -> Self` | `:308` |
| `content` | `<F>(builder: F) -> Self` | `:314` |
| `title` | `(impl IntoElement) -> Self` | `:323` |
| `footer` | `(impl IntoElement) -> Self` | `:339` |
| `button_props` | `(DialogButtonProps) -> Self` | `:345` |
| `on_close` / `on_ok` / `on_cancel` | `...` | `:358` / `:369` / `:380` |
| `close_button` | `(bool) -> Self` | `:389` |
| `margin_top` | `(impl Into<Pixels>) -> Self` | `:395` |
| `w` / `width` / `max_w` | `(impl Into<Pixels>) -> Self` | `:405` / `:413` / `:419` |
| `overlay` / `overlay_closable` | `(bool) -> Self` | `:425` / `:433` |
| `keyboard` | `(bool) -> Self` | `:439` |

`DialogButtonProps`：`CMP\src\dialog\dialog.rs:27` —— `ok_text` `:55`、`ok_variant(ButtonVariant)` `:61`、
`cancel_text` `:67`、`cancel_variant(ButtonVariant)` `:73`、`show_cancel(bool)` `:79`、`on_ok` `:87`、`on_cancel` `:98`

`AlertDialog`：`CMP\src\dialog\alert_dialog.rs:64`，`Styled` `:313`、`ParentElement` `:319`、`RenderOnce` `:356`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `(cx: &mut App) -> Self` | `:79` |
| `confirm` | `() -> Self` | `:96` |
| `trigger` | `(impl IntoElement) -> Self` | `:109` |
| `content` | `<F>(builder: F) -> Self` | `:132` |
| `footer` | `(impl IntoElement) -> Self` | `:146` |
| `icon` / `title` / `description` | `(impl IntoElement) -> Self` | `:161` / `:169` / `:177` |
| `button_props` | `(DialogButtonProps) -> Self` | `:199` |
| `width` | `(impl Into<Pixels>) -> Self` | `:206` |
| `show_cancel` | `(bool) -> Self` | `:212` |
| `overlay_closable` | `(bool) -> Self`（**no-op，参数被忽略**） | `:219` |
| `close_button` / `keyboard` | `(bool) -> Self` | `:224` / `:230` |
| `on_close` / `on_ok` / `on_cancel` | — | `:238` / `:249` / `:260` |

⚠️ **`AlertDialog` 没有 `warning()` / `success()` / `error()` / `info()`。**
`CMP\src\dialog\alert_dialog.rs` 全文没有这四个词。
`CMP\src\window_ext.rs:45` 的 doc 示例 `alert.warning()` 是**过期文档**，照抄编译不过。
变色要用 `DialogButtonProps::ok_variant(ButtonVariant::Danger)`（`CMP\src\dialog\dialog.rs:61`）
+ `AlertDialog::icon(...)`。带 variant 的 `*_::warning(id, msg)` 属于**另一个类型**
`component::alert::Alert`（`CMP\src\alert.rs:104`），那是页内提示条，不是 dialog。

### 5.3 `component::sheet::Sheet`

`CMP\src\sheet.rs:42`，`RenderOnce` `:134`、`ParentElement` `:123`、`Styled` `:128`、`EventEmitter<DismissEvent>` `:122`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `(_: &mut Window, cx: &mut App) -> Self` | `:59` |
| `title` / `footer` | `(impl IntoElement) -> Self` | `:77` / `:83` |
| `size` | `(impl Into<DefiniteLength>) -> Self` | `:89` |
| `resizable` | `(bool) -> Self` | `:95` |
| `overlay` / `overlay_closable` | `(bool) -> Self` | `:101` / `:107` |
| `on_close` | — | `:113` |

⚠️ `Sheet::new` **不是** `new()`；但正常路径是通过 `window.open_sheet(cx, |sheet, window, cx| ...)`
拿到的现成实例（`CMP\src\window_ext.rs:14`、`CMP\src\root.rs:222`）。

### 5.4 `component::notification::Notification`

`CMP\src\notification.rs:107`，`Render` `:395`（是 `Entity<Notification>`），
`EventEmitter<DismissEvent>` `:386`、`EventEmitter<DismissRequest>` `:387`、`FluentBuilder` `:388`、`Styled` `:389`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `() -> Self` | `:167` |
| `message` | `(impl Into<SharedString>) -> Self` | `:190` |
| `info` / `success` / `warning` / `error` | `(impl Into<SharedString>) -> Self` | `:196`–`:217` |
| `id` | `<T: Sized + 'static>() -> Self` | `:229` |
| `id1` | `<T: Sized + 'static>(key: impl Into<ElementId>) -> Self` | `:235` |
| `title` | `(impl Into<SharedString>) -> Self` | `:243` |
| `icon` | `(impl Into<Icon>) -> Self` | `:251` |
| `with_type` | `(NotificationType) -> Self` | `:257` |
| `placement` | `(Anchor) -> Self` | `:266` |
| `delivery` | `(NotificationDelivery) -> Self` | `:290` |
| `system` / `in_app_and_system` | `() -> Self` | `:299` / `:309` |
| `autohide` | `(bool) -> Self` | `:314` |
| `on_click` / `on_close` / `action` | — | `:320` / `:332` / `:340` |
| `content` | — | `:377` |

转换：`From<String>` `:128`、`From<SharedString>` `:134`、`From<&str>` `:140` —— 所以
`window.push_notification("saved", cx)`（`CMP\src\window_ext.rs:180`，`impl Into<Notification>`）也能编译。

`NotificationType` `:31`、`NotificationDelivery` `:53`、`NotificationId` `:85`、
`NotificationList` `:693`（`push` `:798`、`clear` `:952`、`notifications` `:965`）

### 5.5 `component::popover::Popover`

`CMP\src\popover.rs:106`，`RenderOnce` `:293`、`Styled` `:267`、`ParentElement` `:261`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `(id: impl Into<ElementId>) -> Self` | `:132` |
| `anchor` | `(impl Into<Anchor>) -> Self` | `:157` |
| `mouse_button` | `(MouseButton) -> Self` | `:163` |
| `trigger` | `<T>(trigger: T) -> Self` | `:169` |
| `default_open` | `(bool) -> Self` | `:185` |
| `open` | `(bool) -> Self` | `:195` |
| `on_open_change` | `<F>(callback: F) -> Self` | `:205` |
| `trigger_style` | `(StyleRefinement) -> Self` | `:214` |
| `overlay_closable` | `(bool) -> Self` | `:220` |
| `content` | `<F, E>(content: F) -> Self` | `:229` |
| `appearance` | `(bool) -> Self` | `:246` |
| `track_focus` | `(&FocusHandle) -> Self` | `:255` |

`PopoverState` 从 `gpui_base` 再导出（`CMP\src\popover.rs:16`）。

### 5.6 `component::menu::{PopupMenu, DropdownMenu, PopupMenuItem, ContextMenuExt, ContextMenu}`

导入：`gpui_kit::component::menu::{PopupMenu, PopupMenuItem, DropdownMenu, DropdownMenuPopover, ContextMenu, ContextMenuExt, ContextMenuState, AppMenuBar}`
（`CMP\src\menu\mod.rs:9-12`）。

**`DropdownMenu` 是 trait**（`CMP\src\menu\dropdown_menu.rs:12`），**只对 `Button` 有实现**（`:34`）：

```rust
pub trait DropdownMenu: Styled + Selectable + InteractiveElement + IntoElement + 'static {
    fn dropdown_menu(self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static)
        -> DropdownMenuPopover<Self>;                       // :14
    fn dropdown_menu_with_anchor(mut self, anchor: impl Into<Anchor>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static)
        -> DropdownMenuPopover<Self>;                       // :22
}
impl DropdownMenu for Button {}                             // :34
```

⚠️ 注意闭包的 `cx` 是 **`&mut Context<PopupMenu>`**（菜单自己的 context），不是宿主 view 的。

**`PopupMenu` 是 `Entity<PopupMenu>`**：`CMP\src\menu\popup_menu.rs:282`，
`Render` `:1421`、`Focusable` `:1408`、`EventEmitter<DismissEvent>` `:1407`、`FluentBuilder` `:1406`

```rust
// CMP\src\menu\popup_menu.rs:358  —— 注意：没有 id 参数，第一个是 window
pub fn build(
    window: &mut Window,
    cx: &mut App,
    f: impl FnOnce(Self, &mut Window, &mut Context<PopupMenu>) -> Self,
) -> Entity<Self>
```

* `PopupMenu` 其余：`action_context(FocusHandle)` `:371`
* 常用 builder：`menu(label, Box<dyn Action>)` `:465`、`menu_with_enable` `:470`、`menu_with_disabled` `:481`、
  `label(...)` `:492`、`link(...)` `:498`、`link_with_disabled` `:503`、`link_with_icon` `:516`、
  `menu_with_icon` `:543`、`menu_with_icon_and_disabled` `:553`、`menu_with_check` `:565`、
  `menu_with_check_and_disabled` `:575`、`menu_element` `:587`、`menu_element_with_disabled` `:596`、
  `menu_element_with_icon` `:610`、`menu_element_with_check` `:624`、`separator()` `:680`、
  `submenu(...)` `:694`、`submenu_with_icon` `:705`、`item(impl Into<PopupMenuItem>)` `:728`、
  `min_w/max_w/max_h` `:429`/`:435`/`:441`、`scrollable(bool)` `:447`、`check_side(Side)` `:453`、
  `external_link_icon(bool)` `:459`、`rebuild(...)` `:753`、`is_empty()` `:841`
* `PopupMenuItem`：`CMP\src\menu\popup_menu.rs:33`，`new(label)` `:71`、`element(builder)` `:85`、
  `submenu(label, Entity<PopupMenu>)` `:102`、`separator()` `:113`、`label(...)` `:119`、
  `icon(impl Into<Icon>)` `:126`、`action(Box<dyn Action>)` `:145`、`disabled(bool)` `:161`、
  `checked(bool)` `:180`、`on_click(...)` `:196`、`link(label, href)` `:214`；`FluentBuilder` `:67`
* `ContextMenuExt`：`CMP\src\menu\context_menu.rs:13 pub trait ContextMenuExt: InteractiveElement + ParentElement + Styled`
  → `context_menu(...)`；`ContextMenu` `:42`，`new(id, element)` `:53`；`ContextMenuState` `:129`

```rust
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};

// 宿主 render 里（Dodona 就是这个写法）：
Button::new("more")
    .icon(IconName::Ellipsis)
    .dropdown_menu(|menu, _window, _cx| {                // DropdownMenu::dropdown_menu :14
        menu.item(PopupMenuItem::new("Rename"))          // popup_menu.rs:728 / :71
            .item(PopupMenuItem::new("Delete").disabled(true))  // :161
            .separator()                                 // :680
    })
```

### 5.7 `component::command::{Command, CommandState, CommandGroup, CommandItem}`

导入：`gpui_kit::component::command::{Command, CommandState, CommandGroup, CommandItem, CommandEntry}`
（`CMP\src\command\mod.rs:11-13`）。

* `CommandState`：`CMP\src\command\state.rs:117`，`Render` `:801`、`Focusable` `:791`
  `new(window: &mut Window, cx: &mut Context<Self>)` `:142`、`query(cx) -> SharedString` `:201`、
  `set_query(...)` `:209`、`selected_index()` `:231`、`set_selected_index(...)` `:243`、
  `matched_count()` `:276`、`focus(window, cx)` `:281`、`set_loading(bool, window, cx)` `:293`、`is_loading()` `:301`
* `Command`：`CMP\src\command\command.rs:58`，`Styled` `:234`、`RenderOnce` `:240`

| 方法 | 签名 | 证据 |
| --- | --- | --- |
| `new` | `(state: &Entity<CommandState>) -> Self` | `:72` |
| `item` | `(CommandItem) -> Self` | `:87` |
| `items` | `(impl IntoIterator<Item = CommandItem>) -> Self` | `:93` |
| `group` | `(CommandGroup) -> Self` | `:100` |
| `separator` | `() -> Self` | `:106` |
| `searchable` / `filterable` | `(bool) -> Self` | `:112` / `:123` |
| `on_query` / `on_select` / `on_confirm` / `on_cancel` | `<F>(F) -> Self` | `:130`/`:145`/`:157`/`:168` |
| `placeholder` | `(impl Into<SharedString>) -> Self` | `:177` |
| `empty` | `<F, E>(F) -> Self` | `:183` |
| `max_h` | `(impl Into<DefiniteLength>) -> Self` | `:195` |
| `bordered` | `(bool) -> Self` | `:204` |
| `header` / `footer` | `<F, E>(F) -> Self` | `:210` / `:222` |

* `CommandItem`：`CMP\src\command\item.rs:9`，`Disableable` `:143`
  `new()` `:37`、`label(...)` `:50`、`icon(impl Into<Icon>)` `:56`、`action(Box<dyn Action>)` `:64`、
  `checked(bool)` `:74`、`keywords<I, S>(...)` `:80`、`child<F, E>(builder: F)` `:95`
* `CommandGroup`：`CMP\src\command\item.rs:153`
  `new()` `:169`、`label(...)` `:177`、`item(CommandItem)` `:183`、`items(...)` `:189`、`heading()` `:195`
* `CommandEntry`：`CMP\src\command\item.rs:201`（`From<CommandItem>` `:223`、`From<CommandGroup>` `:229`）

```rust
use gpui_kit::component::command::{Command, CommandItem, CommandState};

let state = cx.new(|cx| CommandState::new(window, cx));       // state.rs:142
Command::new(&state)                                          // command.rs:72
    .placeholder("Type a command…")                           // :177
    .searchable(true)                                         // :112
    .item(CommandItem::new().label("Open File").action(Box::new(OpenFile)))  // :37 / :50 / :64
```

### 5.8 `FocusTrapElement`

* trait：`BASE\src\focus_trap.rs:14 pub trait FocusTrapElement: InteractiveElement + Sized`
* 唯一方法：**`BASE\src\focus_trap.rs:39`**

```rust
fn focus_trap(self, id: impl Into<ElementId>, focus_handle: &FocusHandle) -> FocusTrapContainer<Self>
where Self: ParentElement + Styled + Element + 'static;
```

* 空白实现，任何 `InteractiveElement + Sized` 都能用：`BASE\src\focus_trap.rs:50`
* 导入：`gpui_kit::component::FocusTrapElement as _`（`CMP\src\lib.rs:104`）
  或 `gpui_kit::base::FocusTrapElement as _`（`BASE\src\lib.rs:106`）
* 它要求 `gpui_base::init` 已跑（`BASE\src\focus_trap.rs:9` `pub fn init(cx: &mut App)`）——
  `gpui_component::init` 会调用（`CMP\src\lib.rs:137`）；`gpui_kit::init` 又会调 `gpui_component::init`（`KIT\src\lib.rs:158`）

```rust
use gpui_kit::component::FocusTrapElement as _;

v_flex()                                        // Div: InteractiveElement + ParentElement + Styled + Element
    .child(Button::new("ok").label("OK"))
    .child(Button::new("cancel").label("Cancel"))
    .focus_trap("confirm-trap", &self.trap_focus)   // focus_trap.rs:39
```

---

## 6. 主题

导入：`gpui_kit::component::{Theme, ThemeMode, ThemeColor, ThemeTokens, ThemeToken, ThemeConfig, ActiveTheme, SemanticThemeTokens, ColorTokens, RadiusTokens, SpacingTokens, TypographyTokens, TextStyleToken, ShadowTokens}`
（`CMP\src\lib.rs:119 pub use theme::*;` + `CMP\src\theme\mod.rs:9-12`、
`:30-34`、`CMP\src\theme\mod.rs`）。

### 6.1 `ActiveTheme`

```rust
// CMP\src\theme\mod.rs:44
pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

// CMP\src\theme\mod.rs:48  —— 只有 App
impl ActiveTheme for App {
    fn theme(&self) -> &Theme { Theme::global(self) }
}
```

⚠️ **`ActiveTheme` 在 0.6.6 里只对 `App` 实现**（`CMP` 全 crate 只有 `CMP\src\theme\mod.rs:48`
一处 `impl ActiveTheme for`）。所以 `Window::theme()` / `Context<T>::theme()` **不存在**；
只能 `cx.theme()`（`cx: &App`）。在 `&mut Window` 里拿主题要先有 `cx: &App`。

⚠️ `gpui_base` 里另有一个**同名但 `pub(crate)`** 的 `ActiveTheme`（`BASE\src\theme.rs:40`），
不是公开 API。应用要用的永远是 `gpui_kit::component::ActiveTheme`。

`Theme` 通过 `Deref`/`DerefMut` 暴露 `ThemeColor`（`CMP\src\theme\mod.rs:179` / `:187`），
所以 `cx.theme().background` == `cx.theme().colors.background`。

`tokens` 是**字段**而不是方法：`CMP\src\theme\mod.rs:114 pub tokens: ThemeTokens`
—— `cx.theme().tokens.background` 类型是 `ThemeToken`（`CMP\src\theme\theme_color.rs:11`），
`Deref<Target = Hsla>`（`:22`），所以能直接当 `Hsla` 用。

### 6.2 `Theme::change` / `set_scrollbar_mode` / `global_mut` 确切签名

```rust
// CMP\src\theme\mod.rs:198
pub fn global(cx: &App) -> &Theme

// CMP\src\theme\mod.rs:208
pub fn global_mut(cx: &mut App) -> &mut Theme

// CMP\src\theme\mod.rs:214
pub fn is_dark(&self) -> bool

// CMP\src\theme\mod.rs:219
pub fn theme_name(&self) -> &SharedString

// CMP\src\theme\mod.rs:228
pub fn sync_system_appearance(window: Option<&mut Window>, cx: &mut App)

// CMP\src\theme\mod.rs:240
pub fn sync_scrollbar_appearance(cx: &mut App)

// CMP\src\theme\mod.rs:250
pub fn set_scrollbar_mode(mode: ScrollbarMode, cx: &mut App)

// CMP\src\theme\mod.rs:261
pub fn change(mode: impl Into<ThemeMode>, window: Option<&mut Window>, cx: &mut App)
```

⚠️ 三点：

1. `Theme::change` 的第二个参数是 **`Option<&mut Window>`**，传 `None` 就不刷新窗口；
   要刷新必须 `Theme::change(mode, Some(window), cx)`（`:287-289` 有 `window.refresh()`）。
2. `set_scrollbar_mode` **没有** `window` 参数（`CMP\src\theme\mod.rs:250`），且它会同时投影到
   `gpui_base::Theme`（`:252-257`）。
3. `Theme::change` 会**重置** `light_theme`/`dark_theme` 配置里的颜色
   （`BTreeMap` 风格的 `apply_config`，`:274-277`），所以自定义 palette 要在 `change` **之后**写
   （Dodona 的 `theme\mod.rs:231-239` 就是这个顺序：`Theme::change(...)` → `Theme::global_mut(cx)` → 写颜色）。

`ThemeMode`：`CMP\src\theme\mod.rs:701 enum { Light(default), Dark }`；
`is_dark()` `:709`、`name()` `:714`；`From<WindowAppearance>` `:722`。

### 6.3 `ThemeColor` 全部 token 字段（逐个列出）

`CMP\src\theme\theme_color.rs:59 pub struct ThemeColor`，**全部 `pub`、类型都是 `Hsla`**：

| # | 字段 | 行 | # | 字段 | 行 |
| --- | --- | --- | --- | --- | --- |
| 1 | `accent` | 61 | 61 | `popover_foreground` | 199 |
| 2 | `accent_foreground` | 63 | 62 | `primary` | 201 |
| 3 | `accordion` | 65 | 63 | `primary_active` | 203 |
| 4 | `background` | 67 | 64 | `primary_foreground` | 205 |
| 5 | `border` | 69 | 65 | `primary_hover` | 207 |
| 6 | `button` | 71 | 66 | `progress_bar` | 209 |
| 7 | `button_active` | 73 | 67 | `ring` | 211 |
| 8 | `button_foreground` | 75 | 68 | `scrollbar` | 213 |
| 9 | `button_hover` | 77 | 69 | `scrollbar_thumb` | 215 |
| 10 | `button_danger` | 79 | 70 | `scrollbar_thumb_hover` | 217 |
| 11 | `button_danger_active` | 81 | 71 | `secondary` | 219 |
| 12 | `button_danger_foreground` | 83 | 72 | `secondary_active` | 221 |
| 13 | `button_danger_hover` | 85 | 73 | `secondary_foreground` | 223 |
| 14 | `button_info` | 87 | 74 | `secondary_hover` | 225 |
| 15 | `button_info_active` | 89 | 75 | `selection` | 227 |
| 16 | `button_info_foreground` | 91 | 76 | `sidebar` | 229 |
| 17 | `button_info_hover` | 93 | 77 | `sidebar_accent` | 231 |
| 18 | `button_primary` | 95 | 78 | `sidebar_accent_foreground` | 233 |
| 19 | `button_primary_active` | 97 | 79 | `sidebar_border` | 235 |
| 20 | `button_primary_foreground` | 99 | 80 | `sidebar_foreground` | 237 |
| 21 | `button_primary_hover` | 101 | 81 | `sidebar_primary` | 239 |
| 22 | `button_secondary` | 103 | 82 | `sidebar_primary_foreground` | 241 |
| 23 | `button_secondary_active` | 105 | 83 | `skeleton` | 243 |
| 24 | `button_secondary_foreground` | 107 | 84 | `slider_bar` | 245 |
| 25 | `button_secondary_hover` | 109 | 85 | `slider_thumb` | 247 |
| 26 | `button_success` | 111 | 86 | `success` | 249 |
| 27 | `button_success_active` | 113 | 87 | `success_foreground` | 251 |
| 28 | `button_success_foreground` | 115 | 88 | `success_hover` | 253 |
| 29 | `button_success_hover` | 117 | 89 | `success_active` | 255 |
| 30 | `button_warning` | 119 | 90 | `switch` | 257 |
| 31 | `button_warning_active` | 121 | 91 | `switch_thumb` | 259 |
| 32 | `button_warning_foreground` | 123 | 92 | `tab` | 261 |
| 33 | `button_warning_hover` | 125 | 93 | `tab_active` | 263 |
| 34 | `group_box` | 127 | 94 | `tab_active_foreground` | 265 |
| 35 | `group_box_foreground` | 129 | 95 | `tab_bar` | 267 |
| 36 | `caret` | 131 | 96 | `tab_bar_segmented` | 269 |
| 37 | `chart_1` | 133 | 97 | `tab_foreground` | 271 |
| 38 | `chart_2` | 135 | 98 | `table` | 273 |
| 39 | `chart_3` | 137 | 99 | `table_active` | 275 |
| 40 | `chart_4` | 139 | 100 | `table_active_border` | 277 |
| 41 | `chart_5` | 141 | 101 | `table_even` | 279 |
| 42 | `chart_bullish` | 143 | 102 | `table_head` | 281 |
| 43 | `chart_bearish` | 145 | 103 | `table_head_foreground` | 283 |
| 44 | `danger` | 147 | 104 | `table_foot` | 285 |
| 45 | `danger_active` | 149 | 105 | `table_foot_foreground` | 287 |
| 46 | `danger_foreground` | 151 | 106 | `table_hover` | 289 |
| 47 | `danger_hover` | 153 | 107 | `table_row_border` | 291 |
| 48 | `description_list_label` | 155 | 108 | `title_bar` | 293 |
| 49 | `description_list_label_foreground` | 157 | 109 | `title_bar_border` | 295 |
| 50 | `drag_border` | 159 | 110 | `status_bar` | 297 |
| 51 | `drop_target` | 161 | 111 | `status_bar_border` | 299 |
| 52 | `foreground` | 163 | 112 | `warning` | 301 |
| 53 | `info` | 165 | 113 | `warning_active` | 303 |
| 54 | `info_active` | 167 | 114 | `warning_hover` | 305 |
| 55 | `info_foreground` | 169 | 115 | `warning_foreground` | 307 |
| 56 | `info_hover` | 171 | 116 | `overlay` | 309 |
| 57 | `input` | 173 | 117 | `window_border` | 315 |
| 58 | `link` | 175 | 118 | `red` / `red_light` | 318 / 320 |
| 59 | `link_active` | 177 | 119 | `green` / `green_light` | 322 / 324 |
| 60 | `link_hover` | 179 | 120 | `blue` / `blue_light` | 326 / 328 |
| 61 | `list` | 181 | 121 | `yellow` / `yellow_light` | 330 / 332 |
| 62 | `list_active` | 183 | 122 | `magenta` / `magenta_light` | 334 / 336 |
| 63 | `list_active_border` | 185 | 123 | `cyan` / `cyan_light` | 338 / 340 |
| 64 | `list_even` | 187 |  |  |  |
| 65 | `list_head` | 189 |  |  |  |
| 66 | `list_hover` | 191 |  |  |  |
| 67 | `muted` | 193 |  |  |  |
| 68 | `muted_foreground` | 195 |  |  |  |
| 69 | `popover` | 197 |  |  |  |

（上表行号对应 `CMP\src\theme\theme_color.rs` 的字段声明行。）

* `ThemeColor::light() -> Arc<Self>` `:517`、`dark() -> Arc<Self>` `:522`
* `ThemeToken`：`CMP\src\theme\theme_color.rs:11 pub struct ThemeToken { pub color: Hsla, pub background: Background }`，
  `new(Hsla, Background)` `:17`，`Deref<Target = Hsla>` `:22`，
  `From<Hsla>` `:30` / `From<ThemeToken> for Hsla` `:39` / `for Background` `:45` / `for Fill` `:51`
* `ThemeTokens`：字段名与 `ThemeColor` **完全一致**（`CMP\src\theme\theme_color.rs:374-513` 的
  `define_theme_tokens!` 展开），但类型是 `ThemeToken`（`:355`）

`ThemeConfig`（**JSON/TOML schema 类型，不是运行时 token**）：
`CMP\src\theme\schema.rs:38`，全部是 `Option<_>` 的字符串颜色引用；
字段：`is_default` `:40`、`name` `:42`、`mode` `:44`、`font_size` `:48`、`font_family` `:51`、
`mono_font_family` `:61`、`mono_font_size` `:64`、`radius` `:68`、`radius_lg` `:71`、`shadow` `:74`、
`colors: ThemeConfigColors` `:77`、`highlight` `:81`、`tokens: SemanticThemeConfig` `:101`。
`ThemeConfigColors`：`CMP\src\theme\schema.rs:253`。
⚠️ `ThemeConfigColors` 有 `group_box_title_foreground`（`:361`），**`ThemeColor` 里没有**这个字段。

### 6.4 `Theme` 的其余关键字段与方法

```rust
// CMP\src\theme\mod.rs:107
pub struct Theme {
    pub colors: ThemeColor,                 // :108
    pub tokens: ThemeTokens,                // :114
    pub highlight_theme: Arc<HighlightTheme>, // :115
    pub light_theme: Rc<ThemeConfig>,       // :116
    pub dark_theme: Rc<ThemeConfig>,        // :117
    pub mode: ThemeMode,                    // :119
    pub font_family: SharedString,          // :126（默认 .SystemUIFont）
    pub font_size: Pixels,                  // :128（默认 16px）
    pub mono_font_family: SharedString,     // :141（macOS Menlo / Windows Consolas / Linux DejaVu Sans Mono）
    pub mono_font_size: Pixels,             // :143（默认 13px）
    pub radius: Pixels,                     // :145
    pub radius_lg: Pixels,                  // :147
    pub shadow: bool,                       // :148
    pub focus_ring: bool,                   // :156（默认 true）
    pub transparent: Hsla,                  // :157
    pub scrollbar_mode: ScrollbarMode,      // :160（默认 Scrolling）
    pub notification: NotificationSettings, // :163
    pub list: ListSettings,                 // :165
    pub sheet: SheetSettings,               // :167
    pub motion: MotionTokens,               // :170
}
```

| 方法 | 返回 | 证据 |
| --- | --- | --- |
| `semantic_tokens()` | `SemanticThemeTokens` | `CMP\src\theme\mod.rs:399` |
| `motion_tokens()` | `&MotionTokens` | `:410` |
| `color_tokens()` | `ColorTokens` | `:414` |
| `radius_full()` | `Pixels` | `:445` |
| `radius_2xl()` / `radius_3xl()` / `radius_4xl()` | `Pixels` | `:457` / `:462` / `:467` |
| `radius_tokens()` | `RadiusTokens` | `:471` |
| `spacing_tokens()` | `SpacingTokens` | `:482` |
| `typography_tokens()` | `TypographyTokens` | `:486` |
| `shadow_tokens()` | `ShadowTokens` | `:495` |

⚠️ `Theme::tokens` 是**字段**，`Theme::typography_tokens()` 是**方法** —— 别写成 `theme.tokens.typography`。

### 6.5 token 类型（来自 `gpui_base::theme_tokens`，`CMP\src\theme\mod.rs:9-12` 再导出）

```rust
// BASE\src\theme_tokens.rs:11
pub struct SemanticThemeTokens {
    pub colors: ColorTokens,        // :12
    pub radius: RadiusTokens,       // :13
    pub spacing: SpacingTokens,     // :14
    pub typography: TypographyTokens, // :15
    pub shadow: ShadowTokens,       // :16
}

// BASE\src\theme_tokens.rs:20  —— 语义色（注意 destructive == ThemeColor::danger，见 CMP\src\theme\mod.rs:428）
pub struct ColorTokens {
    pub background, foreground, surface, surface_foreground,
    primary, primary_foreground, secondary, secondary_foreground,
    muted, muted_foreground, accent, accent_foreground,
    destructive, destructive_foreground, border, input, ring, selection: Hsla,
}   // :21-44

// BASE\src\theme_tokens.rs:109  —— RadiusTokens 默认 (:118)
none=0, sm=3px, md=6px, lg=8px, xl=12px, full=9999px
// 但 Theme::radius_tokens() 覆盖为 : CMP\src\theme\mod.rs:471-480
//   none=0, sm=theme.radius/2, md=theme.radius, lg=theme.radius_lg, xl=theme.radius*2, full=theme.radius_full()

// BASE\src\theme_tokens.rs:132  —— SpacingTokens 默认 (:142)
xxs=2px, xs=4px, sm=8px, md=12px, lg=16px, xl=24px, xxl=32px
// Theme::spacing_tokens() 永远返回 default（CMP\src\theme\mod.rs:482-484），主题不可改。

// BASE\src\theme_tokens.rs:157
pub struct TextStyleToken { pub size: Pixels, pub line_height: Pixels, pub weight: FontWeight }

// BASE\src\theme_tokens.rs:164  —— TypographyTokens 的档位
pub struct TypographyTokens {
    pub sans: SharedString, pub mono: SharedString,
    pub xs, sm, md, lg, xl, mono_md: TextStyleToken,
}
```

**`TypographyTokens` 档位与像素**（`BASE\src\theme_tokens.rs:175-188` 的 `Default`）：

| 档 | size | line_height | 行号 |
| --- | --- | --- | --- |
| `xs` | 12px | 16px | `:180` |
| `sm` | 14px | 20px | `:181` |
| `md` | 16px | 24px | `:182` |
| `lg` | 18px | 28px | `:183` |
| `xl` | 20px | 28px | `:184` |
| `mono_md` | 13px | 20px | `:185` |
| `sans` | `".SystemUIFont"` | | `:178` |

⚠️ `Theme::typography_tokens()`（`CMP\src\theme\mod.rs:486`）会覆盖 `sans`/`mono` 为当前字体、
`md.size` 为 `theme.font_size`、`mono_md.size` 为 `theme.mono_font_size`。
所以 `typography_tokens().md.size` 是**实际生效的正文大小**（默认 16px）。

### 6.6 `Sizable` / `Size` 档位值与像素对应关系

`Size`（`CMP\src\sizing.rs:6`）与 `Size::Size(Pixels)` 自定义档：

| 档 | 含义 | 证据 |
| --- | --- | --- |
| `Size(Pixels)` | 自定义像素 | `:7` |
| `XSmall` | 默认最小档（`as_f32 = 0.`） | `:8` / `:19` |
| `Small` | | `:9` |
| `Medium` | **`#[default]`** | `:11` |
| `Large` | | `:12` |

各组件里 `Size` → 像素（`CMP\src\sizing.rs`）：

| 用途 | XSmall | Small | Medium | Large | 证据 |
| --- | --- | --- | --- | --- | --- |
| 表格行高 `table_row_height()` | 26px | 30px | 32px | 40px | `:57-64` |
| 输入框左右 padding `input_px()` | 4px | 8px | 10px | 12px | `:147-153` |
| 输入框上下 padding `input_py()` | 0px | 2px | 8px | 10px | `:158-165` |
| 输入框高度 `input_h()` | `h_5()` | `h_6()` | `h_8()` | `h_11()` | `:261-268` |
| 表格单元格 padding `table_cell_padding()` | 2/4 | 3/6 | 4/8 | 8/12 | `:69-95` |
| 输入框字号 `input_text_size()` | `text_xs` | `text_sm` | `text_sm` | `text_base` | `:225-232` |

`StyleSized` trait（`CMP\src\sizing.rs:205`）把这些暴露成方法：
`input_text_size` `:206`、`input_size` `:207`、`input_pl/pr/px/py/h` `:208-212`、
`list_size/list_px/list_py` `:213-215`、`size_with` `:217`、`table_cell_size` `:219`、
`button_text_size` `:220`；**全实现**在 `:223 impl<T: Styled> StyleSized<T> for T`。

### 6.7 `gpui::Styled` 生成的尺寸/间距档位（rem 基准）

`size_3()` 这类方法由 `GPMAC` 在 `gpui::Styled` 上生成（`GPUI\src\styled.rs:26 gpui_macros::style_helpers!();`
→ `GPMAC\src\styles.rs:40`）。前缀含 `w`、`h`、`size`、`min_w/min_h/min_size`、`max_w…`、`p/px/py/pt/pb/pl/pr`、
`m/mx/my/mt/mb/ml/mr`、`gap` 等（`GPMAC\src\styles.rs:830-925` 的 prefix 表）。

档位表在 `GPMAC\src\styles.rs:926-1021`，值是 **rem**：

| 后缀 | rem | @rem=16px（gpui-kit 默认，`CMP\src\root.rs:582` 设 `set_rem_size(cx.theme().font_size)`） |
| --- | --- | --- |
| `0` | 0 | 0px |
| `0p5` | 0.125 | 2px |
| `1` | 0.25 | 4px |
| `1p5` | 0.375 | 6px |
| `2` | 0.5 | 8px |
| `2p5` | 0.625 | 10px |
| `3` | 0.75 | 12px |
| `3p5` | 0.875 | 14px |
| `4` | 1.0 | 16px |
| `5` | 1.25 | 20px |
| `6` | 1.5 | 24px |
| `7` | 1.75 | 28px |
| `8` | 2.0 | 32px |
| `9` | 2.25 | 36px |
| `10` | 2.5 | 40px |
| `11` | 2.75 | 44px |
| `12` | 3.0 | 48px |
| `16` | 4.0 | 64px |
| `20` | 5.0 | 80px |

（行号：`0`:929、`0p5`:934、`1`:939、`1p5`:944、`2`:949、`2p5`:954、`3`:959、`3p5`:964、`4`:969、`5`:974、
`6`:979、`7`:984、`8`:989、`9`:994、`10`:999、`11`:1004、`12`:1009、`16`:1014、`20`:1019。）

⚠️ 因此 `Icon::small()`（`Sizable`，渲染成 `size_3p5` = 14px，`CMP\src\icon.rs:184`）与
`Icon::size_3()`（`Styled`，12px）是**两套独立体系**，混用容易对不齐。

### 6.8 主题最小片段（与 Dodona 的 `theme\mod.rs:231-269` 同序）

```rust
use gpui_kit::component::{ActiveTheme as _, Theme, ThemeMode, ThemeTokens};
use gpui_kit::component::scroll::ScrollbarMode;
use gpui_kit::*;

// 启动时（cx: &mut App）：
Theme::change(ThemeMode::Dark, None, cx);            // theme/mod.rs:261
{
    let theme = Theme::global_mut(cx);               // :208
    theme.background = hsla(0.6, 0.2, 0.05, 1.0);    // ThemeColor 字段，theme_color.rs:67
    theme.primary    = hsla(0.6, 0.8, 0.55, 1.0);    // :201
    theme.radius     = px(6.);                       // :145
}
Theme::set_scrollbar_mode(ScrollbarMode::Hover, cx); // :250

// 在 view 里读：
fn bg(cx: &App) -> Hsla { cx.theme().background }    // ActiveTheme::theme() :45
fn muted(cx: &App) -> Hsla { cx.theme().tokens.muted_foreground }  // ThemeToken: Deref<Hsla>
```

---

## 7. 默认图标集全量清单（`gpui-kit-assets-0.6.6/default-icons.txt`）

### 7.1 这份清单怎么生效

`ASSETS\build.rs` 在编译期做三件事：

1. 扫描 `assets/icons/*.svg`（`:20-51`），把文件名转成 PascalCase 变体名（`:26-38`），
   写进 `OUT_DIR/icon_name.rs` 的 `pub enum IconName`（`:54-67`）—— **这份 enum 包含全部 1830 个图标**，
   与注册哪个 asset source 无关。
2. 同一个文件里生成 `IconName::ALL`（`:59-62`）、`IconName::path(self)`（`:63-67`）
   和 `pub mod embedded`（`:68-72`）。
3. 读 `default-icons.txt`（`:73-78`），筛出 101 个默认图标，生成
   `OUT_DIR/default_assets.rs`（`:110-120`）：

```rust
// ASSETS\build.rs:111
#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "icons/a-large-small.svg"]
#[include = "icons/arrow-down.svg"]
...   // 101 行
pub struct Assets;
```

`ASSETS\src\native_assets.rs:5 include!(concat!(env!("OUT_DIR"), "/default_assets.rs"));`

推断出的两条重要结论：

* `gpui_kit::assets::IconName` 里能**命名**的图标有 1830 个，但 `gpui_kit::assets::Assets`
  只**嵌入了** 101 个。用 `IconName::GitBranch` + `Assets` → `Assets::load` 返回 `Err`
  （`ASSETS\src\native_assets.rs:27-30`，`anyhow!("could not find asset at path ...")`）。
* `gpui_kit::component::IconName`（`CMP\src\icon.rs:18`）是**只有这 101 个变体**的窄枚举，
  专门为兼容保留。`IconName::GitBranch` 在它上面**不存在**。

### 7.2 全部 101 个字形名（逐字，来自 `ASSETS\default-icons.txt` 第 1–101 行）

原始行（左）与 `IconName` 变体名（右）。这是应用实际注册的 `gpui_kit::assets::Assets` 的内容。

| # | default-icons.txt 原文 | 变体名 | # | 原文 | 变体名 |
| --- | --- | --- | --- | --- | --- |
| 1 | `icons/a-large-small.svg` | `ALargeSmall` | 52 | `icons/info.svg` | `Info` |
| 2 | `icons/arrow-down.svg` | `ArrowDown` | 53 | `icons/inspector.svg` | `Inspector` |
| 3 | `icons/arrow-left.svg` | `ArrowLeft` | 54 | `icons/layout-dashboard.svg` | `LayoutDashboard` |
| 4 | `icons/arrow-right.svg` | `ArrowRight` | 55 | `icons/loader-circle.svg` | `LoaderCircle` |
| 5 | `icons/arrow-up.svg` | `ArrowUp` | 56 | `icons/loader.svg` | `Loader` |
| 6 | `icons/asterisk.svg` | `Asterisk` | 57 | `icons/map.svg` | `Map` |
| 7 | `icons/battery-charging.svg` | `BatteryCharging` | 58 | `icons/maximize.svg` | `Maximize` |
| 8 | `icons/battery-full.svg` | `BatteryFull` | 59 | `icons/memory-stick.svg` | `MemoryStick` |
| 9 | `icons/battery-low.svg` | `BatteryLow` | 60 | `icons/menu.svg` | `Menu` |
| 10 | `icons/battery-medium.svg` | `BatteryMedium` | 61 | `icons/minimize.svg` | `Minimize` |
| 11 | `icons/battery-warning.svg` | `BatteryWarning` | 62 | `icons/minus.svg` | `Minus` |
| 12 | `icons/battery.svg` | `Battery` | 63 | `icons/moon.svg` | `Moon` |
| 13 | `icons/bell.svg` | `Bell` | 64 | `icons/network.svg` | `Network` |
| 14 | `icons/book-open.svg` | `BookOpen` | 65 | `icons/palette.svg` | `Palette` |
| 15 | `icons/bot.svg` | `Bot` | 66 | `icons/panel-bottom-open.svg` | `PanelBottomOpen` |
| 16 | `icons/building-2.svg` | `Building2` | 67 | `icons/panel-bottom.svg` | `PanelBottom` |
| 17 | `icons/calendar.svg` | `Calendar` | 68 | `icons/panel-left-close.svg` | `PanelLeftClose` |
| 18 | `icons/case-sensitive.svg` | `CaseSensitive` | 69 | `icons/panel-left-open.svg` | `PanelLeftOpen` |
| 19 | `icons/chart-pie.svg` | `ChartPie` | 70 | `icons/panel-left.svg` | `PanelLeft` |
| 20 | `icons/check.svg` | `Check` | 71 | `icons/panel-right-close.svg` | `PanelRightClose` |
| 21 | `icons/chevron-down.svg` | `ChevronDown` | 72 | `icons/panel-right-open.svg` | `PanelRightOpen` |
| 22 | `icons/chevron-left.svg` | `ChevronLeft` | 73 | `icons/panel-right.svg` | `PanelRight` |
| 23 | `icons/chevron-right.svg` | `ChevronRight` | 74 | `icons/pause.svg` | `Pause` |
| 24 | `icons/chevron-up.svg` | `ChevronUp` | 75 | `icons/play.svg` | `Play` |
| 25 | `icons/chevrons-up-down.svg` | `ChevronsUpDown` | 76 | `icons/plus.svg` | `Plus` |
| 26 | `icons/circle-check.svg` | `CircleCheck` | 77 | `icons/redo-2.svg` | `Redo2` |
| 27 | `icons/circle-user.svg` | `CircleUser` | 78 | `icons/redo.svg` | `Redo` |
| 28 | `icons/circle-x.svg` | `CircleX` | 79 | `icons/replace.svg` | `Replace` |
| 29 | `icons/close.svg` | `Close` | 80 | `icons/resize-corner.svg` | `ResizeCorner` |
| 30 | `icons/copy.svg` | `Copy` | 81 | `icons/rotate-cw.svg` | `RotateCw` |
| 31 | `icons/cpu.svg` | `Cpu` | 82 | `icons/search.svg` | `Search` |
| 32 | `icons/dash.svg` | `Dash` | 83 | `icons/settings-2.svg` | `Settings2` |
| 33 | `icons/delete.svg` | `Delete` | 84 | `icons/settings.svg` | `Settings` |
| 34 | `icons/ellipsis-vertical.svg` | `EllipsisVertical` | 85 | `icons/sort-ascending.svg` | `SortAscending` |
| 35 | `icons/ellipsis.svg` | `Ellipsis` | 86 | `icons/sort-descending.svg` | `SortDescending` |
| 36 | `icons/external-link.svg` | `ExternalLink` | 87 | `icons/square-terminal.svg` | `SquareTerminal` |
| 37 | `icons/eye-off.svg` | `EyeOff` | 88 | `icons/star-fill.svg` | `StarFill` |
| 38 | `icons/eye.svg` | `Eye` | 89 | `icons/star-off.svg` | `StarOff` |
| 39 | `icons/file-text.svg` | `FileText` | 90 | `icons/star.svg` | `Star` |
| 40 | `icons/file.svg` | `File` | 91 | `icons/sun.svg` | `Sun` |
| 41 | `icons/folder-closed.svg` | `FolderClosed` | 92 | `icons/thumbs-down.svg` | `ThumbsDown` |
| 42 | `icons/folder-open.svg` | `FolderOpen` | 93 | `icons/thumbs-up.svg` | `ThumbsUp` |
| 43 | `icons/folder.svg` | `Folder` | 94 | `icons/triangle-alert.svg` | `TriangleAlert` |
| 44 | `icons/frame.svg` | `Frame` | 95 | `icons/undo-2.svg` | `Undo2` |
| 45 | `icons/gallery-vertical-end.svg` | `GalleryVerticalEnd` | 96 | `icons/undo.svg` | `Undo` |
| 46 | `icons/github.svg` | `Github` | 97 | `icons/user.svg` | `User` |
| 47 | `icons/globe.svg` | `Globe` | 98 | `icons/window-close.svg` | `WindowClose` |
| 48 | `icons/hard-drive.svg` | `HardDrive` | 99 | `icons/window-maximize.svg` | `WindowMaximize` |
| 49 | `icons/heart-off.svg` | `HeartOff` | 100 | `icons/window-minimize.svg` | `WindowMinimize` |
| 50 | `icons/heart.svg` | `Heart` | 101 | `icons/window-restore.svg` | `WindowRestore` |
| 51 | `icons/inbox.svg` | `Inbox` |  |  |  |

### 7.3 ★ Git / GitHub 相关字形：**只有 1 个，且只有 GitHub 图标**

在 `default-icons.txt` 里做 `git` 子串匹配，**唯一命中**是第 46 行 `icons/github.svg`
（即 `IconName::Github`）。

**默认集里完全没有**这些字形（它们只存在于完整 Lucide 目录，即 `assets/icons/` 里）：

* `git-branch` / `git-branch-plus` / `git-branch-minus`
* `git-commit-horizontal` / `git-commit-vertical`
* `git-compare` / `git-compare-arrows`
* `git-fork` / `git-graph`
* `git-merge` / `git-merge-conflict`
* `git-pull-request` 及全部 `git-pull-request-*`
* `folder-git` / `folder-git-2`
* `x`（只有 `close`）

（以上文件名逐一存在于 `ASSETS\assets\icons\`，但都不在 `default-icons.txt` 中。）

**对 Lithe 的直接后果**：Git 面板要画分支 / 提交 / PR 图标，只有两条路：

1. `Assets` + `Icon::new(IconName::Github)`（默认集里唯一的 Git 相关字形）；
2. 注册 `AllAssets`（或用 `icon_assets!` 自带 SVG），才能 `use gpui_kit::assets::IconName as CatalogIcon;`
   然后 `Icon::new(CatalogIcon::GitBranch)`。

### 7.4 `AllAssets`（全 Lucide 目录）怎么注册、代价是什么

注册方式（`ASSETS\src\native_assets.rs:8-11` + `:42`）：

```rust
#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "icons/**/*.svg"]
pub struct AllAssets;

impl_asset_source!(AllAssets);   // :42 -> ASSETS\src\native_assets.rs:13-39
```

应用侧（`ASSETS\src\lib.rs:19` 的文档 + `KIT\src\lib.rs:145`）：

```rust
use gpui_kit::assets::AllAssets;
gpui_kit::application().with_assets(AllAssets).run(|cx| { gpui_kit::init(cx); ... });
```

代价：

| 项 | 数值 | 证据 |
| --- | --- | --- |
| 嵌入的 SVG 数量 | **1830 个**（`ASSETS\assets\icons\*.svg`） | 目录计数 |
| 磁盘总字节 | **约 749 008 字节（≈731 KiB）** | 目录计数 |
| 默认 `Assets` 嵌入 | 101 个 SVG | `ASSETS\build.rs:113-115` 生成 |
| 编译期 | `rust_embed` 对每个匹配文件生成 `include_bytes!`；1830 个文件会显著增加 `gpui-kit-assets` 的编译时间与 rlib 体积 | `ASSETS\src\native_assets.rs:8-11` |
| 运行期 | 与 `Assets` 相同：`Self::get(path)`，命中即返回 `Cow::Borrowed` | `ASSETS\src\native_assets.rs:27` |
| WASM | `AllAssets` 只是 `Assets` 的别名（都走 CDN 按需下载） | `ASSETS\src\lib.rs:42-43` |

**第三个选项（推荐给「只要几个额外图标」的场景）**：`icon_assets!` 宏（`ASSETS\src\lib.rs:66`）
只嵌入点名的那几个 SVG，并且可以和 `Assets` 组合：

```rust
use gpui_kit::assets::{icon_assets, IconName};

icon_assets!(ExtraAssets, [GitBranch, GitCommitHorizontal, X]);   // ASSETS\src\lib.rs:66-85
// unlisted 路径返回 Ok(None)（:76-77），所以要和 Assets 组合使用
```

⚠️ `icon_assets!` 生成的 `AssetSource` 对未列出的路径返回 `Ok(None)`（不是 `Err`），
所以必须有一个「兜底」source 一起注册，否则默认 101 个图标全部失效。

---

## 8. 「常见错误写法 → 正确写法」对照表

| # | ❌ 0.7.0 文档 / 凭直觉 | ✅ 0.6.6 真实写法 | 证据 |
| --- | --- | --- | --- |
| 1 | `gpui_kit::open_window(opts, \|_, cx\| ...)` | `cx.open_window(opts, \|window, cx\| cx.new(\|cx\| Root::new(view, window, cx)))` —— `open_window` 在 `App`/`AsyncApp` 上。返回 `anyhow::Result<WindowHandle<V>>`，要 `.expect(...)` | `KIT\src\lib.rs:37`/`:132`（只有注释）；`GPUI\src\app.rs:1347`；`GPUI\src\app\async_context.rs:193` |
| 2 | `Button::new("ok").selected(true)` 直接调 | 先 `use gpui_kit::component::Selectable as _;` —— `selected` 在 `Selectable` trait 上（`Button` 无固有 `selected`） | `BASE\src\component_traits.rs:4`；`CMP\src\button\button.rs:521`（trait impl）；`CMP\src\button\button.rs` 的 `impl Button` 块（`:232-490`）里没有 `selected` |
| 3 | `Button::new("ok").ghost()` / `.xsmall()` / `.disabled(true)` | 分别 `use gpui_kit::component::button::ButtonVariants as _;`、`use gpui_kit::component::Sizable as _;`、`use gpui_kit::component::Disableable as _;` | `CMP\src\button\button.rs:78`（`ghost` 默认方法）；`CMP\src\sizing.rs:187`；`BASE\src\component_traits.rs:15` |
| 4 | `Icon::new(IconName::Search).small()` / `.size_4()`，只 `use gpui_kit::component::Icon` | `small()` 要 `use gpui_kit::component::Sizable as _;`；`size_4()` 要 `use gpui_kit::Styled;`（那是 `gpui::Styled` 生成的方法，不是 `Sizable`） | `CMP\src\icon.rs:209`；`CMP\src\sizing.rs:193`；`GPUI\src\styled.rs:26`；`GPMAC\src\styles.rs:40`+`:858` |
| 5 | `task.then(\|result, cx\| ...)` | 0.6.6 的 `Task` **没有 `then`**。只有 `ready` / `from_async_task` / `is_ready` / `detach` / `fallible` / `downcast` | `gpui-pre-scheduler-0.3.6\src\executor.rs:527-583`（`impl<T> Task<T>` 的全部方法）；对照 `BASE` 用 `Task::ready(())`（`CMP\src\list\delegate.rs:21`） |
| 6 | `div().overflow_y_scrollbar()`，没导入任何东西 | 必须 `use gpui_kit::component::scroll::ScrollableElement as _;`，且**只有 `Div` / `Stateful<E>` 实现了它**；返回类型是 `Scrollable<Self>` 不是 `Div` | `CMP\src\scroll\scrollable.rs:60`（方法）、`:189`/`:190`（唯二 impl）；官方测试 `KIT\tests\components.rs:98-103` 就先写了这行 `use ... as _` |
| 7 | 把 `Button`/`DataTable`/`StatusBar`/`Empty` 当 `Entity` 用（`cx.new(\|_\| Button::new(...))`、`Entity<Button>`） | 这些是 `#[derive(IntoElement)] + impl RenderOnce` 的**一次性元素**，直接 `.child(...)`。只有 `Root`/`ListState`/`TableState`/`TreeState`/`DockArea`/`InputState`/`CommandState`/`Popover` 的 state 才是 `Entity<...>` | `CMP\src\button\button.rs:564`；`CMP\src\table\data_table.rs:141`；`CMP\src\status_bar.rs:77`；`CMP\src\empty.rs:63`；对照 `CMP\src\root.rs:580`（`impl Render`）、`CMP\src\list\list.rs:621` |
| 8 | `DataTable::new(&state).bg(cx.theme().background).p_4()` | `DataTable` **没有 `Styled`**。包一层：`div().bg(...).p_4().child(DataTable::new(&state))` | `CMP\src\table\data_table.rs:131` 只有 `impl Sizable`，没有 `impl Styled` |
| 9 | 假设 `ListItem::new("id").child("a").child("b")` 会竖排两行 | `ListItem` 内部是 `h_flex()`（横排）。要竖排必须自己放 `v_flex()` | `CMP\src\list\list_item.rs:48 base: h_flex().id(id)` |
| 10 | `v_virtual_list(view, id, item_count, row_height, f)` 或直接抄 `uniform_list` | `v_virtual_list(view, id, item_sizes: Rc<Vec<Size<Pixels>>>, f)`，`f` 收 `Range<usize>`。`uniform_list` 是 GPUI 原生的另一个函数（`(id, item_count, f)`），不是 gpui-kit 的虚拟列表 | `BASE\src\virtual_list.rs:139`／`:160`；`GPUI\src\elements\uniform_list.rs:22` |
| 11 | `gpui_kit::component::dock::TabPanel` / `DockItem` | 两者在 0.6.6 **都已删除**。用 `DockLayout`（`h_split`/`v_split`/`tabs`/`panel`/`panel_view`）+ `DockSkin` | `CMP\src\dock\tab_panel.rs:892`/`:1419` 只是「旧 `TabPanel`」的注释；`BASE\src\dock\layout\builder.rs:19` 提到「旧 `DockItem`」；`CMP\src\dock\mod.rs:122` 的 `DockSkin`；`BASE\src\dock\layout\builder.rs:41`/`:46`/`:51` |
| 12 | 只 `impl gpui_kit::component::dock::Panel for MyPanel` | 必须**同时** `impl BasePanel`（= `gpui_base::dock::Panel`，提供 `panel_name`）+ `impl Panel`（表现层）+ `impl EventEmitter<PanelEvent>` + `impl Focusable` + `impl Render` | `CMP\src\dock\mod.rs:32`（`pub use gpui_base::dock::Panel as BasePanel`）；`BASE\src\dock\panel.rs:21`；`CMP\src\dock\panel.rs:71`；官方样例 `KIT\tests\dock.rs:18-46` |
| 13 | `DockSkin::new(cx)` 在 `cx.new(...)` 外面调 | `DockSkin::new` 签名是 `(&mut Context<DockArea>) -> Rc<Self>`，只能在构造 `DockArea` 的闭包里调用；外面请用 `DockSkin::dock_area(id, version, window, cx)` 同时拿到 `(Entity<DockArea>, Rc<DockSkin>)` | `CMP\src\dock\mod.rs:151`（`new`）、`:132`（`dock_area`）；官方样例 `KIT\tests\dock.rs:59` |
| 14 | `window.open_alert_dialog(cx, \|alert, _, _\| alert.warning()...)` | `AlertDialog` **没有 `warning()`**。用 `AlertDialog::confirm()` + `DialogButtonProps::ok_variant(ButtonVariant::Danger)`；`warning()` 属于另一个类型 `component::alert::Alert::warning(id, msg)`（页内提示条） | `CMP\src\window_ext.rs:45` 是**过期 doc**；`CMP\src\dialog\alert_dialog.rs:96` 只有 `confirm`；`CMP\src\dialog\dialog.rs:61` `ok_variant`；`CMP\src\alert.rs:104` `Alert::warning` |
| 15 | `use gpui_kit::component::IconName;` 然后 `IconName::GitBranch` | 组件 `IconName` 只有 101 个变体，没有 `GitBranch`。要 `use gpui_kit::assets::IconName as CatalogIcon;` + 注册 `AllAssets`（否则 `Assets::load` 返回 `Err`） | `CMP\src\icon.rs:18`（窄枚举）+ `:38`；`ASSETS\build.rs:73-82`（只取 default-icons.txt）；`ASSETS\src\native_assets.rs:27-30` |
| 16 | `use gpui_kit::Sizable;` / `gpui_kit::Disableable` / `gpui_kit::base::Sizable` | 都在 `gpui_kit::component::{Sizable, Disableable, Selectable, Size, StyleSized}`；`gpui_kit` 根只有 `gpui::*` + `base`/`component`/`assets` | `KIT\src\lib.rs:95`/`:106`/`:143`；`CMP\src\lib.rs:118` → `CMP\src\styled.rs:1-2`；`BASE\src\lib.rs` 无 `Sizable` |
| 17 | `use gpui_kit::component::StatusBar;` | `pub mod status_bar;` 没有根再导出，只能 `gpui_kit::component::status_bar::StatusBar` | `CMP\src\lib.rs:85`（`pub mod status_bar;`，对比 `:121 pub use title_bar::*;`） |
| 18 | `Kbd::new("cmd-s")` | `Kbd::new(stroke: Keystroke)`，字符串要先 `Keystroke::parse(..)` | `CMP\src\kbd.rs:31`；`From<Keystroke>` `:18` |
| 19 | `Tab::new("id")` | `Tab::new()` **无参**；id 由宿主 `TabBar` 决定 | `CMP\src\tab\tab.rs:478` |
| 20 | `Switch::new("id").disabled(true)` 当作固有方法 | `Switch` 没有固有 `disabled`，要走 `Disableable`（`use gpui_kit::component::Disableable as _;`） | `CMP\src\switch.rs:120` 是唯一的 `disabled`；对照 `CMP\src\input\input.rs:310`（`Input` 有固有 `disabled`） |
| 21 | `ButtonGroup::new("g").child(div())` | `ButtonGroup::child` 只收 `Button` | `CMP\src\button\button_group.rs:61` |
| 22 | `cx.theme()` 在 `&mut Window` / `&mut Context<T>` 上 | `ActiveTheme` 只对 `App` 实现；需要 `cx: &App` 才能 `cx.theme()` | `CMP\src\theme\mod.rs:48`（全 crate 唯一的 `impl ActiveTheme for`） |
| 23 | `let t = cx.theme().tokens.typography;` | `tokens` 是 `ThemeTokens` 字段（全是 `ThemeToken`）；排版是**方法** `cx.theme().typography_tokens()` → `TypographyTokens` | `CMP\src\theme\mod.rs:114`（字段）、`:486`（方法）；`CMP\src\theme\theme_color.rs:354`（`ThemeTokens` 定义） |
| 24 | `Theme::change(mode, window, cx)` 里传 `&mut Window` 之外的东西 / 期望自动重绘 | 第二参数是 `Option<&mut Window>`；传 `None` 不会刷新窗口。且 `change` 会重载 stock config，自定义颜色必须在 `change` **之后**写 | `CMP\src\theme\mod.rs:261`、`:287-289`（`window.refresh()`）、`:274-277`（`apply_config`） |
| 25 | `Theme::set_scrollbar_mode(mode, window, cx)` | 只有 `(mode: ScrollbarMode, cx: &mut App)`，**没有 window 参数** | `CMP\src\theme\mod.rs:250` |
| 26 | 用 `TabPanel` 之外，把 `DockSkin` 当 `Entity` 用 | `DockSkin` 是 `Rc<DockSkin>`（无 EntityId），只有 `DockArea` 是 `Entity<DockArea>` | `CMP\src\dock\mod.rs:122`（`pub struct DockSkin`）；`:137` 返回 `Rc<Self>` |
| 27 | `.child(EmptyTitle::new().child("No data"))` 直接放进 `Empty` | `Empty::header(EmptyHeader)` 收**具体类型**；`EmptyTitle` 要放进 `EmptyHeader::title(...)` | `CMP\src\empty.rs:33`（`header(EmptyHeader)`）、`:114`（`title(EmptyTitle)`） |
| 28 | `use gpui_kit::component::IconNameExt; IconName::Search.view(cx)` | 组件 `IconName` 的 `view` 是**固有**方法，不需要 trait；`IconNameExt` 只对 `gpui_kit::assets::IconName` 生效 | `CMP\src\icon.rs:42`（固有）、`:67`/`:71`（trait 只给 assets 的 `IconName`） |

---

## 9. 我**没能确认** / 明确判定为「不存在」的 API 清单

这一节是本次调研里最重要的「坑位表」。凡是没有源码证据的，一律标为未确认，不做猜测。

### 9.1 明确「在 0.6.6 中不存在」（已用全 crate 搜索确认）

| 名称 | 结论 | 搜索范围与证据 |
| --- | --- | --- |
| `gpui_kit::open_window` | **不存在** | `KIT\src\*.rs` 只匹配到 `lib.rs:37`/`:132` 的 doc 注释 |
| `component::dock::TabPanel` | **不存在**（仅注释提及旧名） | `CMP` 全 crate 搜索 `TabPanel`：仅 `dock/tab_panel.rs:892`、`:1419` 两处注释 |
| `component::dock::DockItem` | **不存在**（仅注释提及旧名） | `CMP` 全 crate 无匹配；`BASE` 4 处全在注释（`dock/dock_area.rs:207`、`:2052`，`dock/layout/builder.rs:19`、`:249`） |
| `TabGroupSkin` | **存在但 `pub(crate)`，应用不可用** | `CMP\src\dock\tab_panel.rs:185 pub(crate) struct TabGroupSkin` |
| `AlertDialog::warning/success/error/info` | **不存在** | `CMP\src\dialog\alert_dialog.rs` 全文无这四个标识符 |
| `Task::then` | **不存在** | `gpui-pre-scheduler-0.3.6\src\executor.rs:527-583`（`impl<T> Task<T>` 全部方法）无 `then` |
| `Sizable::medium()` / `Sizable::size_3()` | **不存在**（`Medium` 是默认值；`size_*` 属 `gpui::Styled`） | `CMP\src\sizing.rs:178-202` 只有 `with_size`/`xsmall`/`small`/`large` |
| `component::dock::Dock` | **被故意不导出** | `CMP\src\dock\mod.rs:43-47` 的注释明确说明「base 的 `Dock` 不在此再导出」，读 dock 请用 `DockContext`（`BASE\src\dock\dock_area.rs:1664`） |
| 默认图标集里的 `git-branch*` / `git-commit*` / `git-pull-request*` / `git-fork` / `git-merge*` / `x` | **不在默认 101 个里** | `ASSETS\default-icons.txt` 的 `git` 子串只命中第 46 行 `github.svg` |

### 9.2 我**没有**在本次调研中确认到源码证据的项（不要照抄，需要补查）

| 项 | 情况 | 建议 |
| --- | --- | --- |
| `component::scroll::Scrollbar` 的**全部** builder 组合（`styles` + `mode` + `viewport_from_layout` 的交互） | 只读了签名（`BASE\src\scrollbar.rs:831-935`），没读 2885 行文件的 `Element` 实现细节 | 需要时读 `BASE\src\scrollbar.rs:1252+` 的 `impl Element for Scrollbar` |
| `component::table` 的 `render_th` / `render_group_th` / `render_last_empty_col` 的完整签名与返回类型 | 只拿到行号与首行（`CMP\src\table\delegate.rs:56`/`:77`/`:188`），没有逐个读完整 `fn` 块 | 读 `CMP\src\table\delegate.rs:50-230` |
| `TableState::selection_mode(...)` | **不存在公开 API**。`SelectionMode` 是**私有** enum（`CMP\src\table\state.rs:36`），`selection_mode` 是私有字段（`:241`）；模式由你调用哪个 setter 隐式决定：`set_selected_row` → `Row`（`:470`）、`set_selected_col` → `Column`（`:509`）、`set_selected_cell` → `Cell`（`:549`）、`clear_selection` → `Row`（`:563`） | 用 `set_selected_row/col/cell` 决定模式，别找 setter |
| `component::tree::TreeEvent` 的变体列表 | ✅ 已确认 | `Expanded(SharedString)` / `Collapsed(SharedString)`，`BASE\src\tree.rs:93-96` |
| `PopupMenu::build` 的完整签名 | ✅ 已确认 | `build(window: &mut Window, cx: &mut App, f: impl FnOnce(Self, &mut Window, &mut Context<PopupMenu>) -> Self) -> Entity<Self>`，`CMP\src\menu\popup_menu.rs:358-364` |
| `DropdownMenuPopover::on_open_change` 精确签名 | ✅ 已确认 | `fn on_open_change(mut self, callback: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self`，`CMP\src\menu\dropdown_menu.rs:81-87` |
| `component::dock::{DockEvent, DockAreaRenderer, TabGroupRenderer}` 的完整要求 | 只拿到定义位置（`BASE\src\dock\dock_area.rs:39`、`:182` 再导出；`BASE\src\dock\tab_group.rs:894`） | 若要做自定义 dock 皮肤，必须逐个读完这两个 trait |
| `TableVisibleRange` 的字段可见性 | `CMP\src\table\state.rs:112-126` 有私有字段 + `rows()`/`cols()` accessor | 用 accessor，别碰字段 |
| `Theme::apply_config` 是否公开 | `CMP\src\theme\schema.rs:1060 pub fn apply_config`，但它是 `impl` 在哪一层没确认 | 本清单只写已经用到的 `Theme::change` 路径 |
| `Icon` 的 `Sizable`/`Styled` 尺寸叠加优先级 | `CMP\src\icon.rs:170-187` 的逻辑是「显式 style 尺寸优先，其次 `Icon.size`」，但没有验证具体渲染结果 | 需要视觉验证 |
| `DockAreaRenderer` 的自定义实现（替代 `DockSkin`） | 只看到 `CMP\src\dock\dock.rs:35 impl DockAreaRenderer for DockSkin` 与 `:261` 测试里的 `ChromelessDockSkin` | 需要时读 `BASE\src\dock\dock_area.rs:182` 之后 trait 定义 |
| `Select` / `Combobox` 的事件订阅注册方式 | ✅ 已确认：`impl EventEmitter<SelectEvent<D>> for SelectState<D>`（`CMP\src\select.rs:760`）、`impl EventEmitter<ComboboxEvent<D>> for ComboboxState<D>`（`CMP\src\combobox.rs:715`），两者也都实现 `EventEmitter<DismissEvent>`（`select.rs:767` / `combobox.rs:721`） | 用 `cx.subscribe_in(&state, window, \|this, state, event, window, cx\| ...)` |
| `component::setting` 模块（`gpui_kit::component::setting`，Dodona 在用） | 不在本次清单范围内，未调研 | 单独补查 `CMP\src\setting\` |
| `component::chart` / `plot` / `highlighter` / `markdown` 等 | 不在本次清单范围内，未调研 | 单独补查 |

### 9.3 已知的「文档/源码不一致」点（照抄文档会编译失败）

1. **`CMP\src\window_ext.rs:41-50`** 的 `open_alert_dialog` doc 示例用了 `alert.warning()` —— 该方法不存在。
2. **`CMP\src\lib.rs:9-13`** 的模块 doc 指向 `gpui_component::component` 之外的东西，但示例中
   `cx.open_window(...)` 是 `App` 的方法（不是 `gpui_kit` 的）。
3. **`CMP\src\list\delegate.rs:83`** 的默认 `render_empty` 用 `Icon::new(IconName::Inbox).size_12()`
   —— 这是**组件内部**代码，它同时导入了 `crate::ActiveTheme as _`（`:4`）。
   应用侧在 `impl ListDelegate` 里写默认实现之外的代码时，必须自己补
   `use gpui_kit::component::{Sizable as _, ActiveTheme as _};` 才能编译同样的调用。
4. `CMP\src\tab\tab_bar.rs:86`/`:92`/`:98`/`:104` 的 `pill()`/`outline()`/`segmented()`/`underline()`
   与 `Tab` 上同名方法（`CMP\src\tab\tab.rs:511`/`:517`/`:523`/`:529`）是**两套独立 API**，
   `TabBar` 上设置不会自动传给 `Tab`；反之亦然。
