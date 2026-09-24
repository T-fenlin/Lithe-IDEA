# 09 · gpui-kit 0.6.6「在窗口上弹浮层」实现说明

> 范围：只依据**已发布源码**（不看在线文档，线上文档描述的是未发布的 0.7.0）。
> 证据规则：所有结论带 `文件:行号`。凡未在源码中找到证据的，写「未找到」，不推测。
> 源码根：`D:\ProgramData\rust\cargo\registry\src\rsproxy.cn-e3de039b2554c837\`，下文相对路径均以该目录为基准：
> - `gpui-kit-0.6.6/`（facade，只有 `src/lib.rs`、`src/test.rs`、`tests/`）
> - `gpui-component-0.6.6/src/`（`gpui_kit::component` 的真身）
> - `gpui-base-0.6.6/src/`（`gpui_kit::base`，无样式行为层）
> - `gpui-pre-0.3.6/src/`（`gpui_kit::*` 即 GPUI 本体）
>
> 本项目现状：`gpui/shell/src/bin/shell_probe/`（`mod.rs` 启动顺序、`workspace.rs` 组装、`panels.rs` 面板）。`gpui/shell/Cargo.toml` 钉的是 `gpui-kit = "=0.6.6"`。

---

## 1. `gpui_kit::component::Root` 是什么，位置约束是什么

### 结论

`Root` 是 gpui-kit 0.6.6 里**唯一**的「窗口级浮层宿主」视图。它不是浮层本身，而是**窗口根视图必须包的一层壳**：把业务视图包进去，它负责 Dialog、Sheet、Notification 三类浮层，另外还内联渲染 tooltip / 原生菜单兜底 / 触摸选择三个窗口级 overlay。

三条硬约束：

1. **必须是窗口的第一个视图**（即 `open_window` 的回调里 `cx.new(|cx| Root::new(view, window, cx))`）。
   违反时 `Root::update` / `Root::read` 直接 **panic**，不是静默降级。
2. **它自己 render 里只有内联 overlay，没有 Dialog / Sheet / Notification 三层**。这三层要用 `Root::render_dialog_layer` / `render_sheet_layer` / `render_notification_layer` 由**应用自己**在某个视图的 render 里挂上去；不挂就等于「浮层不显示」。
3. **0.6.6 没有 `Root::open_window`，也没有 `Window::open_window`。** `open_window` 只存在于 `App` / `AsyncApp` / 测试上下文上。已验证依据见下表最后一行。

### 可照抄的调用序列

```rust
// ① 窗口根：Root 包住业务视图（唯一正确形态）
//    gpui/shell/src/bin/shell_probe/mod.rs:86-92 就是这一形态
cx.open_window(window_options, move |window, cx| {
    let workspace = cx.new(|cx| ShellWorkspace::new(root, window, cx));
    cx.new(|cx| Root::new(workspace, window, cx))   // Root::new 返回 Self，外面套 cx.new
})
.expect("failed to open window");

// ② 在业务视图的 render 里挂浮层宿主层（关键：Root 自己不挂）
impl Render for ShellWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialogs       = Root::render_dialog_layer(window, cx);
        let sheets        = Root::render_sheet_layer(window, cx);
        let notifications = Root::render_notification_layer(window, cx);

        div()
            .relative().size_full()
            /* …工作台内容… */
            .children(dialogs)          // Option<impl IntoElement>，直接 children
            .children(sheets)
            .children(notifications)
    }
}

// ③ 之后才可以用浮层 API
window.open_dialog(cx, |dialog, window, cx| dialog.title("…"));
```

### `Root::new` 签名与构造

```rust
// gpui-component-0.6.6/src/root.rs:100
pub fn new(view: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self
```

`Root::new` 返回的是 `Self`（不是 `Entity<Root>`），所以标准写法是 `cx.new(|cx| Root::new(view, window, cx))`（`gpui-kit-0.6.6/src/lib.rs:132-135`、`gpui/shell/src/bin/shell_probe/mod.rs:90`）。
`view` 参数是 `impl Into<AnyView>`，`Entity<业务视图>` 可以直接传。

### Root 负责的浮层清单

| 类别 | 是否 Root 自己 render | 证据 |
| --- | --- | --- |
| Dialog（模态对话框，可叠多层） | **否**，要应用调 `Root::render_dialog_layer` | `gpui-component-0.6.6/src/root.rs:242-295`、`root.rs:586-602`（render 里没有 dialog） |
| Sheet（侧边抽屉，单层） | **否**，要应用调 `Root::render_sheet_layer` | `root.rs:215-239` |
| Notification（Toast 列表） | **否**，要应用调 `Root::render_notification_layer` | `root.rs:185-212` |
| Tooltip overlay | **是**，内联 | `root.rs:44`、`root.rs:601` |
| 原生菜单兜底（非 macOS 的 FallbackMenuOverlay） | **是**，内联 | `root.rs:45`、`root.rs:602` |
| 文本选择层 / 触摸选择层 | **是**，内联 | `root.rs:46`、`root.rs:598-600` |

Root 公开的状态操作入口（都要求 Root 已在窗口里，否则 panic）：

```rust
// gpui-component-0.6.6/src/root.rs
pub fn open_dialog<F>(&mut self, build: F, window: &mut Window, cx: &mut Context<'_, Root>)   // :297
pub fn close_dialog(&mut self, window: &mut Window, cx: &mut Context<'_, Root>)               // :333
pub fn close_all_dialogs(&mut self, window: &mut Window, cx: &mut Context<'_, Root>)          // :365
pub fn open_sheet_at<F>(&mut self, placement: Placement, build: F, …)                          // :379
pub fn close_sheet(&mut self, window: &mut Window, cx: &mut Context<'_, Root>)                // :410
pub fn push_notification(&mut self, note: impl Into<Notification>, …)                          // :425
pub fn view(&self) -> &AnyView                                                                 // :488
pub fn bordered(self, bool) -> Self          // Linux CSD 边框开关                          // :143
pub fn window_shadow_size(self, …) -> Self                                                     // :151
// 静态入口：把 &mut Window 里的 Root 捞出来做一次更新
pub fn update<F, R>(window: &mut Window, cx: &mut App, f: F) -> R                              // :156
pub fn read<'a>(window: &'a Window, cx: &'a App) -> &'a Self                                   // :176
```

### 「0.6.6 有没有 `open_window`」的结论依据

**结论：Root 上没有，Window 上没有。** 0.6.6 的窗口创建仍是 `AppContext::open_window`。

| 事实 | 证据 |
| --- | --- |
| `gpui-kit 0.6.6` 的 `src/` 只有 `lib.rs` 与 `test.rs`，没有任何 `open_window` 定义 | `gpui-kit-0.6.6/src/lib.rs`（全文 164 行，无此定义） |
| `open_window` 定义在 `App` 上 | `gpui-pre-0.3.6/src/app.rs:1347` `pub fn open_window<V: 'static + Render>(…)` |
| 也定义在 `AsyncApp` 上 | `gpui-pre-0.3.6/src/app/async_context.rs:193` |
| `Window` 上不存在 `open_window`（`gpui-pre-0.3.6/src/window.rs` 全文 9274 行，grep `open_window` 无匹配） | `gpui-pre-0.3.6/src/window.rs`（grep 未找到） |
| gpui-component 内部要开窗口时用的是 `AppContext::open_window` | `gpui-component-0.6.6/src/input/input.rs:1105`、`input.rs:1191`、`src/spinner.rs:93` |
| 项目注释已记录这是 0.6.6 → 0.7.0 的变更点（`Root::new -> open_window`），但 0.6.6 仍用 `Root::new` | `gpui/shell/Cargo.toml`（`gpui-kit = "=0.6.6"` 上方注释） |

### 位置约束的证据

| 结论 | 证据 |
| --- | --- |
| 文档明写「Must be the first view in the window」，用途是管理 Sheet / Dialog / Notification | `gpui-component-0.6.6/src/root.rs:34-36` |
| 不是 Root 时 `Root::update` 直接 panic：`expect("BUG: window first layer should be a gpui_component::Root.")` | `gpui-component-0.6.6/src/root.rs:160-163` |
| `Root::read` 同样 expect | `root.rs:176-182` |
| `try_update` 是唯一的非 panic 变体（返回 `Option`） | `root.rs:168-174` |
| `window.root::<E>()` 是「取窗口根视图」的 GPUI 原语，返回 `Option<Option<Entity<E>>>` | `gpui-pre-0.3.6/src/window.rs:2257-2264` |
| 替换窗口根视图的 API 在 `WindowContext`/`App` 上是 `replace_root`，不在 `Window` 上 | `gpui-pre-0.3.6/src/window.rs:2242` |
| 本项目当前**没有**任何地方调用 `render_*_layer`（全仓库 grep 无匹配） | `D:\developmentProjects\rust\Lithe-IDEA` grep `render_dialog_layer|render_sheet_layer|render_notification_layer` → 未找到 |
| 也就是说：今天的 `shell_probe` 里 `open_dialog` / `open_sheet` / `push_notification` **不会显示任何东西**，必须先补上层的挂载 | 上两条 + `gpui/shell/src/bin/shell_probe/workspace.rs:161-287`（无 layers） vs `gpui-kit-0.6.6/tests/overlays.rs:20-22,84-86`（官方测试必须挂） |

---

## 2. 在窗口上叠一个「居中浮层」（命令面板：输入框 + 可过滤列表 + Esc 关 + 点外关 + 焦点归还）有哪几条路

0.6.6 可用路径共 6 条，按「适不适合做命令面板」排序。

### 路径 A：`Dialog`（模态居中浮层）— **唯一开箱即用的正解**

**组件与构造**
```rust
use gpui_kit::component::{Root, WindowExt, dialog::Dialog};
use gpui_kit::component::command::{Command, CommandState};
```

**状态归属**：`Entity<CommandState>` 由**你自己的视图**持有；Dialog 本体是 `RenderOnce` 的临时元素，**状态不放在 Dialog 里**（Dialog 每次打开都由 `Root` 里的 `Rc<dyn Fn(...)>` 构造器重新构造：`gpui-component-0.6.6/src/root.rs:64,73,262`）。焦点句柄由 `Root` 持有（`ActiveDialog.focus_handle`，`root.rs:69`）。

**完整调用序列**
```rust
// 0) Root 在窗口根 + 业务视图 render 里挂 render_dialog_layer（见第 1 节）

// 1) 打开（on_click / on_action 里）
let palette = self.palette.clone();
window.open_dialog(cx, move |dialog, _window, _cx| {
    dialog
        .w(px(560.))                  // 宽度；默认 448px → dialog/dialog.rs:405
        .margin_top(px(120.))         // 距顶；默认 viewport 高度 / 10 → dialog.rs:395 / dialog.rs:529
        .max_w(px(640.))              // 可选 → dialog.rs:419
        .overlay_closable(true)       // 点外部关闭；默认就是 true → dialog.rs:433
        .keyboard(true)               // Esc 关闭；默认 true → dialog.rs:439
        .close_button(true)           // 默认 true → dialog.rs:389
        .child(
            Command::new(&palette)
                .bordered(false)      // 已在 Dialog 的框里，去掉自己的框 → command.rs:204
                .max_h(rems(20.))     // 列表最大高；默认 18.75rem → command.rs:195
                .placeholder("输入命令…")            // command.rs:177
                .on_confirm(|index, window, cx| { /* 用户确认了哪一项 */ })
                .on_cancel(|window, cx| { /* Esc 传播到 Dialog 之前的机会 */ })
        )
});

// 2) 让键盘进到搜索框（Root::open_dialog 自己只 focus 它分配的句柄，见 root.rs:309-310）
window.defer(cx, move |window, cx| {
    palette.update(cx, |state, cx| state.focus(window, cx));   // command/state.rs:281
});
```

**已知限制**

| 项 | 结论 | 证据 |
| --- | --- | --- |
| 能不能带输入框 | 能。官方测试就在 Dialog 里放 `Input::new(&draft).id("name")` 并输入中文 | `gpui-kit-0.6.6/tests/overlays.rs:38-63,125-131` |
| 能不能自定义尺寸 | 能改宽/最大宽/顶部偏移；**高度不直接可设**，由内容撑开且被 `max_h`（视口减顶部减 margin）夹住 | `dialog/dialog.rs:405-422,395-398`；高度夹取 `dialog.rs:530-535` |
| 是否水平居中 | 是，组件自己算 `x = (view_width - width) / 2` | `dialog/dialog.rs:534` |
| 是否垂直居中 | **不是**。默认 `y = viewport_height / 10`，叠层每多一层再 +16px | `dialog/dialog.rs:529`；测试断言 16px 步进：`dialog/dialog.rs:843` |
| 是否必须 Root 在最外层 | **是**。`window.open_dialog` 内部走 `Root::update`，非 Root 根直接 panic | `window_ext.rs:141-148` + `root.rs:160-163` |
| 是否必须挂 layer | **是**，否则「看起来和没打开一样」 | `dialog/dialog.rs:287-289`（源码原话）、`root.rs:586-602` |
| 点外部关闭 | 自带：backdrop 的 `on_any_mouse_down` → `on_cancel` → `request_close(false)` | `gpui-base-0.6.6/src/dialog.rs:590-625` |
| Esc 关闭 | 自带：`key_context("Dialog")` + `escape → Cancel` 绑定 | `gpui-base-0.6.6/src/dialog.rs:91-92,554,559-573` |
| 焦点归还 | 自带：打开时记 `window.focused(cx)`，关闭时 `window.focus(&handle, cx)` 还原。**Esc / 点外关走立即路径**（`close_dialog`，`request_close(false)`）；**Enter 确认走延迟路径**（`defer_close_dialog`，等 `ANIMATION_DURATION` = 0.25s 再还原，且期间又开了新 dialog 就不还原） | 打开 `root.rs:301-310`；立即还原 `root.rs:333-339` + `gpui-base/dialog.rs:570`；延迟还原 `root.rs:341-363` + `gpui-base/dialog.rs:585` + `dialog/dialog.rs:22,601-607` |
| 焦点陷阱 | 自带（`focus_trap` + `Root` 拦 Tab） | `gpui-base-0.6.6/src/dialog.rs:553`；`root.rs:492-560` |
| 「Dialog + Command」官方用例 | **未找到**（crate 内 `Command::new` 只出现在 `command/state.rs` 的测试里，均无 Dialog 宿主） | grep `Command::new` 命中全在 `command/state.rs` |

### 路径 B：`Sheet`（侧边抽屉）

```rust
// 需要 Root::render_sheet_layer 挂在视图里
window.open_sheet(cx, |sheet, _, _| {           // = open_sheet_at(Placement::Right, …)
    sheet.title("命令").size(px(420.)).child(Command::new(&palette))
});
```
- `open_sheet` 就是 `open_sheet_at(Placement::Right, …)`：`window_ext.rs:111-116`。
- 尺寸/位置/关闭回调：`sheet.rs:89`（`size`，默认 350px，`sheet.rs:63`）、`sheet.rs:101-110`（overlay / overlay_closable）、`sheet.rs:113-119`（`on_close`）。
- 位置由 `Placement` 决定（Top/Right/Bottom/Left），`root.rs:194-200` 据此把通知层让开。
- **限制**：是**贴边**抽屉，不是居中浮层；同样必须 Root 在最外层（`window_ext.rs:119-126` → `Root::update`）。

### 路径 C：`Popover`（挂到触发元素上的浮层，非模态）

```rust
// 直接在视图树里 render，不需要 Root 在最外层
Popover::new("palette-popover")
    .trigger(Button::new("open").label("命令"))     // 必需，见下
    .anchor(Anchor::TopLeft)                       // 默认 TopLeft → popover.rs:136,157
    .overlay_closable(true)                        // 点外关，默认 true → popover.rs:220
    .track_focus(&input_focus_handle)              // 打开时把焦点交给它 → popover.rs:255
    .on_open_change(|open, window, cx| { … })      // popover.rs:205
    .content(|_state, window, cx| Command::new(&palette).bordered(false))
```
- 状态：`gpui_base::PopoverState` 存在**元素状态**里（`window.use_keyed_state`），不是你自己的 `Entity`：`gpui-base-0.6.6/src/popover.rs:271-273`、`popover.rs:32-45`。公开操控入口 `PopoverState::show/dismiss/toggle_open/set_open/is_open`：`popover.rs:57-95`。
- 焦点：打开时记 `previous_focus_handle` 并 focus 目标句柄，关闭时还给上一个：`popover.rs:98-125`。
- 点外部关闭：`on_mouse_down_out` → `dismiss`：`popover.rs:328-336`。
- **限制**：`trigger` 是必需的——没有 trigger 时 `RenderOnce` 直接返回一个 `div().id("empty")`，什么都不显示（`gpui-base-0.6.6/src/popover.rs:284-286`）。所以它天然是「某个控件旁边弹一个小面板」，不是「全局居中的命令面板」；位置由 trigger 的 bounds + Anchor/Positioner 决定，**不能居中**。
- 不需要 Root：官方 `Popover` 测试用的是普通 `add_window_view`，没有 `Root`（`gpui-component-0.6.6/src/popover.rs:408-433`）。

### 路径 D：`PopupMenu` / `ContextMenu`

```rust
// 唯一入口是给元素挂右键菜单
some_div.context_menu(|menu, window, cx| menu.menu("打开文件…", Box::new(SomeAction)))
         .into_any_element();
```
- `ContextMenuExt::context_menu`：`gpui-component-0.6.6/src/menu/context_menu.rs:13-39`。
- 菜单项构造：`PopupMenuItem::new/element/submenu/separator/label/icon/action/disabled/checked/on_click/link`：`menu/popup_menu.rs:71-214`。
- **限制**：菜单项是行式条目，**不承载输入框**，也没有「居中显示」的入口；它是右键/触发式菜单，做不了「输入框 + 可过滤列表」。不适合命令面板。

### 路径 E：`Notification`（Toast）

```rust
window.push_notification(Notification::new().message("已保存").autohide(true), cx);
// 需要 Root::render_notification_layer 挂在视图里
```
- `Notification::new/message/success/error/placement/autohide/on_click/action`：`notification.rs:167,190,196-217,266,314,320,340`。
- `Root::push_notification`：`root.rs:425-434`；`WindowExt::push_notification`：`window_ext.rs:180-185`。
- **限制**：角落 HUD，只显示信息，没有输入与条目选择。

### 路径 F：自己画一个 absolute 浮层（最自由，成本最高）

```rust
// 在任意视图里，用一个铺满父级的层自己实现遮罩 + 居中 + Esc + 点外关
div()
    .relative().size_full()
    .child(/* 业务内容 */)
    .when(self.palette_open, |this| {
        this.child(
            div()
                .absolute().inset_0()
                .occlude()                              // 挡住下层点击
                .on_mouse_down_out({ … dismiss … })     // div.rs:263
                .on_key_down / key_context + on_action  // 自己绑 Esc
                .child(
                    v_flex().absolute().top(px(120.)).left_relative()  // 自己居中
                        .child(Command::new(&self.palette))
                )
        )
    })
```
- 若要让浮层真的画在**所有**内容之上（不被祖先 `overflow` 裁切、不被后绘制的兄弟盖住），需要用延迟绘制：
  - 元素级：`deferred(child).with_priority(n)`：`gpui-pre-0.3.6/src/elements/deferred.rs:7,25`。`gpui_base::Dialog` 就是这么画的（`gpui-base-0.6.6/src/dialog.rs:541`、`dialog.rs:631`）。
  - 窗口级：`Window::defer_draw(element, absolute_offset, priority, content_mask)`：`gpui-pre-0.3.6/src/window.rs:4330-4352`。注意注释写明「**只能在 prepaint 阶段调用**」（`window.rs:4329`，且内部有 `debug_assert_prepaint()`，`window.rs:4337`），不能从事件回调里调用。
- 定位用 `anchored().position(..).snap_to_window()`：`gpui-pre-0.3.6/src/elements/anchored.rs:27,68`。
- **限制**：遮罩、焦点陷阱、Esc 绑定、点外关、打开/关闭动画、焦点记录与归还、叠层优先级，**全部要自己写**——这些正是 `Dialog` 已经做好的东西（对照第 5 节）。只有在「必须突破 Dialog 的固定宽度/居中/层叠策略」时才值得。

### 路径总览

| 路径 | 居中 | 带输入框 | 自定义尺寸 | Root 必须在最外层 | 挂 layer | 点外关 | Esc | 焦点归还 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A `Dialog` | 水平是，垂直靠 `margin_top` | 能 | 宽/最大宽/顶偏移 | **是** | **要** | 自带 | 自带 | 自带 |
| B `Sheet` | 否（贴边） | 能 | `size()` | **是** | **要** | 自带 | 自带 | 自带 |
| C `Popover` | 否（贴 trigger） | 能 | Styled | 否 | 否 | 自带 | 自带 | 自带 |
| D `PopupMenu` | 否 | **不能** | 有 min/max | 否 | 否 | 自带 | 自带 | 自带 |
| E `Notification` | 否 | **不能** | 有 anchor | **是** | **要** | 手动关 | 无 | 无 |
| F 自绘 | 自己算 | 能 | 全自由 | 否 | 否（可选 deferred） | 自己写 | 自己写 | 自己写 |

### `Dialog` 各 capability 的源码证据

| 能力 | 证据 |
| --- | --- |
| `Dialog::new(cx: &mut App) -> Self` | `gpui-component-0.6.6/src/dialog/dialog.rs:287` |
| `.content(builder: Fn(DialogContent, &mut Window, &mut App) -> DialogContent)` | `dialog/dialog.rs:314-320`；`DialogContent` 是 `v_flex().w_full().flex_1()`：`dialog/content.rs:35-43` |
| `.title(impl IntoElement)` | `dialog/dialog.rs:323-326` |
| `.footer(impl IntoElement)` / `.button_props(DialogButtonProps)` | `dialog/dialog.rs:339-342` / `:345-348` |
| `.on_ok(...) -> bool` / `.on_cancel(...) -> bool` / `.on_close(...)` | `dialog/dialog.rs:369-375` / `:380-386` / `:358-364`（返回 `false` 则不关） |
| `.trigger(impl IntoElement)`（把 Dialog 变成「一个点击打开的按钮」） | `dialog/dialog.rs:308-311`、`:472-503` |
| 默认值：宽 448、overlay 开、overlay_closable 开、keyboard 开、close_button 开 | `dialog/dialog.rs:173-186` |
| 几何：`margin` = `theme.spacing_tokens().lg`；`max_height = viewport_h - y - margin` | `dialog/dialog.rs:528,535` |
| 动画：0.25s cubic-bezier，`slide-down` + `fade-in` | `dialog/dialog.rs:22,554-563,701-728` |
| 层叠：每层 +16px 下移，最上层负责画 overlay | `dialog/dialog.rs:529`；`root.rs:254-285` |
| 点外部关闭的具体链路 | `gpui-base-0.6.6/src/dialog.rs:600-622` |
| Esc / Enter 绑定 | `gpui-base-0.6.6/src/dialog.rs:89-94` |
| 焦点陷阱 + 只在最上层允许 backdrop 关闭 | `gpui-base-0.6.6/src/dialog.rs:531,553` |
| `Window::open_dialog` 的完整签名 | `window_ext.rs:30-32,141-148` |
| `Window::open_alert_dialog`（带图标、居中按钮的便捷版） | `window_ext.rs:51-53,151-158` |

---

## 3. `component::command::{Command, CommandState, CommandGroup, CommandItem}` 完整公开 API

模块导出：`pub use command::Command; pub use item::{CommandEntry, CommandGroup, CommandItem}; pub use state::CommandState;`、`pub(crate) use state::init;` —— `gpui-component-0.6.6/src/command/mod.rs:11-15`。

### 3.1 `Command`（`RenderOnce`，无状态外观）

```rust
// gpui-component-0.6.6/src/command/command.rs
pub struct Command { state: Entity<CommandState>, entries: Vec<CommandEntry>, searchable: bool,
                     filterable: bool, on_query/on_select/on_confirm/on_cancel, options: CommandOptions }  // :57-68

pub fn new(state: &Entity<CommandState>) -> Self                     // :72
pub fn item(self, item: CommandItem) -> Self                         // :87
pub fn items(self, impl IntoIterator<Item = CommandItem>) -> Self    // :93
pub fn group(self, group: CommandGroup) -> Self                      // :100
pub fn separator(self) -> Self                                       // :106
pub fn searchable(self, bool) -> Self        // 显示/隐藏搜索框 + 本地过滤，默认 true   // :112
pub fn filterable(self, bool) -> Self        // 保留搜索框但关掉本地过滤，默认 true     // :123
pub fn on_query<F: Fn(&str, &mut Window, &mut App) + 'static>(self, F) -> Self       // :130
pub fn on_select<F: Fn(IndexPath, &mut Window, &mut App) + 'static>(self, F) -> Self // :145
pub fn on_confirm<F: Fn(IndexPath, &mut Window, &mut App) + 'static>(self, F) -> Self// :157
pub fn on_cancel<F: Fn(&mut Window, &mut App) + 'static>(self, F) -> Self            // :168
pub fn placeholder(self, impl Into<SharedString>) -> Self            // 默认走 i18n "Command.placeholder" // :177
pub fn empty<F, E: IntoElement>(self, F: Fn(&CommandState, &mut Window, &mut App) -> E + 'static) -> Self // :183
pub fn max_h(self, impl Into<DefiniteLength>) -> Self   // 列表最大高，默认 rems(18.75)=300px  // :195
pub fn bordered(self, bool) -> Self                     // 边框+圆角，默认 true            // :204
pub fn header<F, E: IntoElement>(self, F) -> Self       // 搜索框上方                    // :210
pub fn footer<F, E: IntoElement>(self, F) -> Self       // 列表下方                      // :222
impl Styled for Command                                   // 可 refine_style              // :234
impl RenderOnce for Command                               // :240（render 时把 model 推进 state）
```

`CommandOptions::default()`：`max_h: rems(18.75)`、`bordered: true`、`placeholder/empty/header/footer: None` —— `command.rs:28-40`。

### 3.2 `CommandState`（`Entity`，真正的交互状态）

```rust
// gpui-component-0.6.6/src/command/state.rs
pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self          // :142
pub fn query(&self, cx: &App) -> SharedString                            // :201
pub fn set_query(&mut self, impl Into<SharedString>, window, cx)         // :209（程序化写入，会重新过滤并触发 on_query）
pub fn selected_index(&self) -> Option<IndexPath>                        // :231（高亮项在**未过滤模型**里的坐标）
pub fn set_selected_index(&mut self, Option<IndexPath>, window, cx)      // :243（被过滤掉/disabled 的路径会清空高亮）
pub fn matched_count(&self) -> usize                                     // :276
pub fn focus(&self, window: &mut Window, cx: &mut App)                   // :281（searchable 时 focus 搜索框，否则 focus 自身）
pub fn set_loading(&mut self, bool, window, cx)                          // :293（转圈，同时抑制「无结果」文案）
pub fn is_loading(&self) -> bool                                         // :301
impl Focusable for CommandState { fn focus_handle(&self, cx) -> FocusHandle }  // :791-799
impl Render for CommandState                                             // :801-937
```

键盘绑定（`key_context = "Command"`）：`escape → Cancel`、`enter → Confirm`、`up → SelectUp`、`down → SelectDown` —— `state.rs:62-70`。

### 3.3 `CommandGroup` / `CommandItem` / `CommandEntry`

```rust
// gpui-component-0.6.6/src/command/item.rs
CommandGroup::new()                                   // :169
    .label(impl Into<SharedString>)                   // :177（组标题；组内条目全被过滤掉时标题不画）
    .item(CommandItem)                                // :183
    .items(impl IntoIterator<Item = CommandItem>)     // :189
    .heading() -> Option<&SharedString>               // :195

CommandItem::new()                                    // :37
    .label(impl Into<SharedString>)                   // :50（同时是搜索文本）
    .icon(impl Into<Icon>)                            // :56
    .action(Box<dyn Action>)                          // :64（点击/回车时 dispatch，右侧显示其快捷键提示）
    .checked(bool)                                    // :74（右侧打勾；有 action 快捷键时该槽被快捷键占用）
    .keywords(impl IntoIterator<Item = impl Into<SharedString>>)  // :80（额外搜索词，含中文也能匹配）
    .child(Fn(&mut Window, &mut App) -> E)            // :95（替换整行内容；可能被多次调用，必须无副作用）
    .disabled(bool)                                   // :143-148（Disableable）
```
`CommandEntry::{Item, Group, Separator}` 与 `From<CommandItem>` / `From<CommandGroup>`：`item.rs:201-233`。分隔符在过滤后若处于首/尾/相邻位置则不渲染：`state.rs:340-342,384-387`。

### 3.4 「用户确认了哪一项」怎么拿

两条路：

1. **`on_confirm(|index: IndexPath, window, cx| ...)`** —— `command.rs:157-163`。回调在**该条目的 Action 被 dispatch 之后**、且在 `CommandState` 更新释放租约之后运行（`state.rs:592-603`）。`IndexPath` 结构：`{ section: usize, row: usize, column: usize }`（`gpui-base-0.6.6/src/index_path.rs:9-16`）。坐标语义：未分组条目 section=0、row=输入顺序；显式分组用组序号与组内序号；**本地过滤不改变这些坐标**（`command.rs:140-144`、`state.rs:359,394`）。
2. **`CommandItem::action(Box<dyn Action>)`** —— `item.rs:64-67`，确认时 `window.dispatch_action(action, cx)`（`state.rs:596-598`）。

另可用 `CommandState::selected_index()` 读当前高亮项（`state.rs:231`），或在 `on_select` 里跟踪（`command.rs:145`）。

### 3.5 空状态

- 入口：`Command::empty(Fn(&CommandState, &mut Window, &mut App) -> E)`（`command.rs:183-192`）。
- 渲染条件：`rows_count == 0 && !loading`（`state.rs:907-909`）；默认文案是 i18n `"Command.empty"`（`state.rs:773-788`）。

### 3.6 尺寸 / 最大高度

- **最大高度**：`Command::max_h(...)`（`command.rs:195`），作用在列表容器上（`state.rs:903`），默认 `rems(18.75)`（300px）。
- **宽度**：Command 自己只写 `w_full()`（`state.rs:828`），**没有宽度 API**；宽度由外层容器决定（Dialog 的 `w()`、或包一层 `div().w(px(...))`）。
- 其余样式走 `impl Styled for Command`（`command.rs:234-238`）→ `.refine_style`（`state.rs:837`）。
- 列表容器有 `overflow_hidden` + 垂直 `Scrollbar`（`state.rs:904,930`）；行高按实际内容测量（`state.rs:610-637`）。

### 3.7 它能不能直接当浮层用？

**不能。** 依据：

- `Command` 是 `RenderOnce`，`render` 只返回 `self.state`（`command.rs:240-258`）；`CommandState::render` 产出的是一个**普通流式** `v_flex()`：`.id("command")`、`w_full`、`overflow_hidden`、`bg(theme.popover)`、可选边框圆角（`state.rs:819-837`）。
- 全程**没有** `absolute` / `inset_0` / 遮罩 / `deferred` / `occlude` / `focus_trap` / Esc 关闭自带逻辑（grep `state.rs` 无 `absolute`、无 `deferred`）。
- 唯一自带的是 `key_context("Command")` + `track_focus(&self.focus_handle)`（`state.rs:822-823`），以及「Esc 先清空查询、再 `cx.propagate()`」（`state.rs:567-582`）。源码注释直说：「Escape clears a non-empty query first, and only then leaves the palette — **the dialog that hosts it closes on the second press**」（`state.rs:567-568`），即它**假设自己被一个 Dialog 宿主**。
- `Command::bordered` 的文档也直说：「Turn it off when the palette already sits inside a frame of its own, **such as a `crate::Dialog`**」（`command.rs:200-207`）。

**结论：`Command` 必须放进别的容器（首选 Dialog）里用。**

---

## 4. `Window` 这一层有没有「插入浮层 / 模态」的接口

### 结论

**没有** `open_modal` / `show_overlay` / `push_overlay` / `add_overlay` 之类的**语义化**接口。
`gpui-pre-0.3.6/src/window.rs` 全文 9274 行，grep `open_modal|modal|show_overlay|push_overlay|add_overlay` → **未找到**。

`Window` 这一层只有**底层**能力（都是「画 / 调度」而非「弹出模态」）：

| API | 签名 / 用途 | 证据 |
| --- | --- | --- |
| `Window::defer_draw` | `pub fn defer_draw(&mut self, element: AnyElement, absolute_offset: Point<Pixels>, priority: usize, content_mask: Option<ContentMask<Pixels>>)` —— 把元素推迟到「当前树之上」的稍后时间绘制，`priority` 越大越靠上 | `gpui-pre-0.3.6/src/window.rs:4330-4352`（文档 `:4322-4329`，内部 `debug_assert_prepaint()` `:4337`） |
| `Window::defer` | `pub fn defer(&self, cx: &mut App, f: impl FnOnce(&mut Window, &mut App) + 'static)` —— 当前效果周期末尾执行一次 | `gpui-pre-0.3.6/src/window.rs:2507-2512` |
| `Window::root::<E>()` | `pub fn root<E>(&self) -> Option<Option<Entity<E>>>` —— 取窗口根视图（`Root` 就是靠它定位自己） | `gpui-pre-0.3.6/src/window.rs:2257-2264` |
| `Window::insert_hitbox` | 手动插入命中盒 | `gpui-pre-0.3.6/src/window.rs:5117` |
| `Window::replace_root` | 在 `WindowContext` 上替换窗口根视图（不在 `Window` 上） | `gpui-pre-0.3.6/src/window.rs:2242` |
| `Window::show_window_menu` / `show_character_palette` | 系统级菜单/字符面板，跟浮层无关 | `gpui-pre-0.3.6/src/window.rs:2840,2926` |

配套的**元素级**原语（真正被组件用来实现浮层的东西）：

| API | 用途 | 证据 |
| --- | --- | --- |
| `deferred(child).with_priority(n)` | 把子树推迟到窗口层绘制（不受祖先 `overflow` 裁切），`Dialog` 用它 | `gpui-pre-0.3.6/src/elements/deferred.rs:7,25`；`gpui-base-0.6.6/src/dialog.rs:541,631` |
| `anchored().position(..).snap_to_window()` | 相对窗口定位 | `gpui-pre-0.3.6/src/elements/anchored.rs:27,68` |
| `div().occlude()` | 阻断下层命中（模态语义的一半） | `gpui-base-0.6.6/src/dialog.rs:572`；`gpui-component-0.6.6/src/popover.rs:282` |
| `focus_trap(id, &handle)` | 焦点陷阱 | `gpui-base-0.6.6/src/focus_trap.rs:14,39-48` |

**所以「插入浮层」在 0.6.6 里只有两条正路**：
(a) `WindowExt`（`open_dialog` / `open_sheet` / `push_notification`）→ 状态存进 `Root`，由应用在 render 里重放成元素；
(b) 自己在视图树里 render 一个 `Popover` / 自绘 absolute 层（必要时 `deferred`）。

---

## 5. 焦点处理：抢焦点与归还

### 5.1 基础 API

| 需求 | API | 证据 |
| --- | --- | --- |
| 造一个焦点句柄 | `cx.focus_handle()`（`AppContext`/`Context`） | `gpui-pre-0.3.6/src/app.rs:2784-2787` |
| 抢焦点 | `window.focus(&handle, cx)` | `gpui-pre-0.3.6/src/window.rs:2302-2312` |
| 等价写法 | `handle.focus(window, cx)`（`FocusHandle::focus`） | `gpui-pre-0.3.6/src/window.rs:601` |
| 读当前焦点 | `window.focused(cx) -> Option<FocusHandle>` | `gpui-pre-0.3.6/src/window.rs:2285-2288` |
| 丢焦点 | `window.blur(cx)` | `gpui-pre-0.3.6/src/window.rs:2315-2327` |
| Tab 导航 | `window.focus_next(cx)` / `focus_prev(cx)` | `gpui-pre-0.3.6/src/window.rs:2336,2347` |
| 元素加入焦点链 | `.track_focus(&handle)` | `gpui-base-0.6.6/src/dialog.rs:552`；`gpui-component-0.6.6/src/command/state.rs:823` |
| 判断是否仍在自己内部 | `handle.contains_focused(window, cx)` | `root.rs:502`；`gpui-base-0.6.6/src/popover.rs:121` |

### 5.2 焦点归还（0.6.6 已经内建，不用自己写）

```rust
// 打开：记录「之前谁有焦点」，并 focus 新分配的句柄
// gpui-component-0.6.6/src/root.rs:301-310
let mut previous_focused_handle = window.focused(cx).map(|h| h.downgrade());
if let Some(pending) = self.pending_focus_restore.take() { previous_focused_handle = Some(pending); }
let focus_handle = cx.focus_handle();
focus_handle.focus(window, cx);

// 关闭：直接还原
// root.rs:333-339
pub fn close_dialog(&mut self, window: &mut Window, cx: &mut Context<'_, Root>) {
    if let Some(handle) = self.close_dialog_internal() { window.focus(&handle, cx); }  // close_dialog_internal: :325-331
}

// 带关闭动画的路径：延迟 ANIMATION_DURATION(0.25s) 再还原，且若期间又开了新 dialog 就不还原
// root.rs:341-363（ANIMATION_DURATION = dialog/dialog.rs:22）
```
- Sheet 同理：打开时记 `previous_focused_handle`（`root.rs:388-392`），关闭时还原（`root.rs:410-423`）。
- Popover 自己在 `PopoverState` 里做同样的事：`previous_focus_handle`（`gpui-base-0.6.6/src/popover.rs:100-125`）。
- 端到端验证用例：`gpui-kit-0.6.6/tests/overlays.rs:154-211`（Esc 关掉 Dialog 后 `window.find("name").focused() == Some(true)`）。

### 5.3 焦点陷阱

- 入口：`FocusTrapElement::focus_trap(id, &focus_handle)`（`gpui-base-0.6.6/src/focus_trap.rs:14,39-48`）。
- `Dialog` 与 `Sheet` **自带**：`gpui-base-0.6.6/src/dialog.rs:553`。
- Tab 在本 trap 内循环：由 `Root` 的 `on_action_tab` / `on_action_tab_prev` 实现（`root.rs:492-560`），查 `gpui_base::active_focus_trap(window, cx)`（`root.rs:494`；`focus_trap.rs:226`）。
- `Root` 用 `key_context("Root")` 绑 `tab` / `shift-tab`（`root.rs:20-31,588`）。

### 5.4 「点外面关闭」

有两套，按你用哪条路径选：

| 场景 | 写法 | 证据 |
| --- | --- | --- |
| 用 `Dialog` | **不用写**。backdrop 的 `on_any_mouse_down` → `on_cancel`（返回 true 才关）→ `request_close(false, …)`；`overlay_closable` 决定是否生效 | `gpui-base-0.6.6/src/dialog.rs:590-625`；开关 `gpui-component-0.6.6/src/dialog/dialog.rs:433-436,585` |
| 用 `Popover` | **不用写**。`on_mouse_down_out` → `PopoverState::dismiss` | `gpui-base-0.6.6/src/popover.rs:328-336` |
| 自绘浮层 | 自己写：`div().on_mouse_down_out(\|_, window, cx\| …)`（捕获阶段、鼠标在这元素 bounds 之外、且没有 active prompt 时触发） | `gpui-pre-0.3.6/src/elements/div.rs:258-276` |

注意：`Dialog` 的 `dismiss_below_y(TITLE_BAR_HEIGHT)`（`gpui-component-0.6.6/src/dialog/dialog.rs:586`）表示**标题栏区域内的点击不会用来关闭对话框**（`gpui-base-0.6.6/src/dialog.rs:600-603`），拖窗口不会误关。

---

## 6. 推荐做法（本项目实现「命令面板浮层」应当采用的写法）

### 6.1 选型：`window.open_dialog` + `Command`，层由 `ShellWorkspace` 挂

这是唯一同时满足「居中 / 带输入框 / Esc 关 / 点外关 / 焦点自动归还 / 官方测试覆盖」的路径。

### 6.2 为什么其它路径不选

| 路径 | 不选的理由（一句） | 证据 |
| --- | --- | --- |
| `Sheet` | 它是贴边抽屉（`Placement::Right` + 350px），位置就错了；命令面板要的是屏幕中上部 | `window_ext.rs:111-116`、`sheet.rs:63` |
| `Popover` | 必须有 `trigger` 元素并且锚在 trigger 上，做不了全局居中；它适合「某个按钮旁边的小面板」 | `gpui-base-0.6.6/src/popover.rs:284-286` |
| `PopupMenu` / `ContextMenu` | 菜单项不承载输入框，也没有命令式「居中弹出」入口 | `menu/context_menu.rs:19-36`、`menu/items` 全为行式条目 |
| `Notification` | 角落 Toast，无输入、无条目选择，语义不同 | `notification.rs:107-190` |
| 自绘 absolute 浮层 | 遮罩/陷阱/Esc/点外关/焦点记录归还/动画全要自己写，等于重造 `Dialog` | 对照第 2 节路径 A 与第 5 节 |
| 把 `Command` 直接铺满窗口 | 它是普通流式 `v_flex`，没有遮罩/定位/关闭语义 | `command/state.rs:819-837` |
| `window.defer_draw` 手插浮层 | 只能在 prepaint 阶段调用，事件回调里调用会 `debug_assert_prepaint` 失败 | `gpui-pre-0.3.6/src/window.rs:4329,4337` |

### 6.3 具体写法（可直接照抄）

**Step 0（必须先做，否则今天弹不出来）**：在 `ShellWorkspace::render` 的根 `div()` 末尾挂上宿主层。

```rust
// gpui/shell/src/bin/shell_probe/workspace.rs 的 impl Render for ShellWorkspace
impl Render for ShellWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialogs       = Root::render_dialog_layer(window, cx);   // 命令面板走这条
        let sheets        = Root::render_sheet_layer(window, cx);
        let notifications = Root::render_notification_layer(window, cx);

        div()
            .relative().w(logical_width).h(logical_height)
            /* ……现有内容保持不变…… */
            .children(dialogs)
            .children(sheets)
            .children(notifications)
    }
}
```
（形态照 `gpui-kit-0.6.6/tests/overlays.rs:20-22,84-86`；`Root` 已在窗口根，`mod.rs:90`。）

**Step 1**：把 `Entity<CommandState>` 从 `ShellPanel`（`panels.rs:103,130,154,184`）挪到 `ShellWorkspace` 持有 —— 浮层属于工作台，不属于 Dock 标签页。`ShellPanel` 里那份可以直接删掉。

```rust
pub(super) struct ShellWorkspace {
    dock_area: Entity<DockArea>,
    palette: Entity<CommandState>,          // 新增：命令面板状态
    // …
}

// ShellWorkspace::new 里
let palette = cx.new(|cx| CommandState::new(window, cx));
```

**Step 2**：定义动作与快捷键（0.6.6 的 `actions!` 用 `gpui_kit::actions!`）。

```rust
gpui_kit::actions!(lithe_shell, [ToggleCommandPalette]);

// 注册（window 语境）
cx.bind_keys([KeyBinding::new("secondary-shift-p", ToggleCommandPalette, None)]);

// 在工作台根元素上接住
div()
    .key_context("ShellWorkspace")
    .on_action(cx.listener(Self::on_toggle_command_palette))
```

**Step 3**：打开浮层。

```rust
fn on_toggle_command_palette(
    &mut self, _: &ToggleCommandPalette, window: &mut Window, cx: &mut Context<Self>,
) {
    let palette = self.palette.clone();
    window.open_dialog(cx, move |dialog, _window, _cx| {
        dialog
            .w(px(560.))                    // 默认 448 对命令面板偏窄
            .margin_top(px(120.))           // 默认 viewport_h/10；这是唯一的「纵向位置」旋钮
            .overlay_closable(true)         // 点外部关闭（默认已 true）
            .keyboard(true)                 // Esc 关闭（默认已 true）
            .close_button(false)            // 命令面板不需要右上角 X
            .child(
                Command::new(&palette)
                    .bordered(false)        // 已在 Dialog 框内
                    .max_h(rems(22.))
                    .placeholder("输入命令或搜索…")
                    .empty(|_, _, _| div().p_6().child("没有匹配的命令"))
                    .on_confirm(|index, window, cx| {
                        // index: gpui_kit::component::IndexPath { section, row, column }
                        // 用 (section,row) 映射到稳定的命令 id，再调自己的分发函数。
                    })
                    .on_cancel(|window, cx| { /* 可选：埋点 */ })
            )
    });

    // 关键：Root::open_dialog 只 focus 它自己分配的句柄（root.rs:309-310），
    // 要真正把键盘交给搜索框必须自己再 focus 一次。
    // 注意：这一句是「按 Root 自己的做法推断」的写法，源码里没有等价的测试用例，
    //       落地时请自测「浮层一出现就能直接打字」。
    window.defer(cx, move |window, cx| {
        palette.update(cx, |state, cx| state.focus(window, cx));
    });
}
```

**Step 4**：关闭与确认。

- Esc：**不用写**。Command 收到第一次 Esc 会先清空查询并停下（`command/state.rs:569-573`），第二次 Esc 调 `on_cancel` 后 `cx.propagate()`（`state.rs:577-581`），Dialog 的 `escape → Cancel` 处理器接住并 `request_cancel(false, …)`（`gpui-base/dialog.rs:89-94,559-573`），最终走 `Dialog::request_close` 的 `deferred == false` 分支 → `window.close_dialog(cx)` → `Root::close_dialog` **立即**把焦点还回去（`dialog/dialog.rs:601-607`；`root.rs:333-339`）。只有 Enter 确认走的那条路才是 `deferred == true` → 等 0.25s 再还焦点（`gpui-base/dialog.rs:585`；`root.rs:341-363`）。
- 点外部：**不用写**（`gpui-base/dialog.rs:590-625`）。
- 焦点归还：**不用写**（`root.rs:333-339` / `root.rs:341-363`）。
- 确认：尽量用 `on_confirm(IndexPath)` 而不是 `CommandItem::action(...)`。理由：Action 是 `window.dispatch_action` 派发的（`state.rs:596-598`），要通过 `on_action` 冒泡接住，路径依赖焦点位置；`IndexPath` 是稳定的模型坐标（`command.rs:140-144`），映射到命令表更直接、更好测。

**Step 5（可选，双击 Shift 唤出）**：`CommandState` 有现成的查询框焦点语义，双击 Shift 只需再绑一个 action 到同一个 `on_toggle_command_palette`；本项目源码里**未找到**双击 Shift 的现成实现，需要自己判定两次 shift keydown 的时间差。

### 6.4 需要注意的已知风险（源码未覆盖的部分，落地时要自测）

1. **`Command` 放进 `Dialog` 没有官方用例**。crate 内 `Command::new` 只出现在 `command/state.rs` 的测试里，宿主都是普通视图或用例自建的 `Harness`，没有 Dialog 宿主（grep 全 crate 确认）。`Dialog` 会把 children 包进 `v_flex().size_full().overflow_y_scrollbar()`（`dialog/dialog.rs:663-675`），`.content()` 路径包进 `v_flex().w_full().flex_1()`（`dialog/content.rs:35-43`）——两条路都会让 Command 拿到「撑满」的父容器，配合它自己的 `max_h` 应当正常，但**没有测试背书**，首次落地请用一个 `debug_selector` 断言实际 bounds。
2. **Enter 由谁吃掉**：Command 的 `Confirm` 绑定在 `"Command"` 上下文（`state.rs:66`），Dialog 的 `Confirm` 绑定在 `"Dialog"` 上下文（`gpui-base/dialog.rs:92`）。两者都在焦点路径上，靠「内层优先 + Command 不 propagate」分离（`state.rs:561-565` 不 propagate）。已发布测试只验证了「无 Dialog 宿主时 Enter 能确认」（`gpui-kit-0.6.6/tests/search.rs:101-107`），**Dialog 宿主下的键序没有测试覆盖**，需要自测「输入框有焦点时按 Enter 不会误关对话框」。
3. **窗口根不能换成别的**。任何 `open_dialog` 都要求 `window.root::<Root>()` 是 `Root`（`root.rs:160-163`）。如果以后要在 `Root` 和业务视图之间插别的容器视图，`Root::render` 里的 `self.view.clone()`（`root.rs:599`）就是唯一入口，插不了。
4. **`Root::render` 里没有 layers 这件事是「静默失败」**：忘挂 layer 时 `open_dialog` 不报错、不 panic，只是屏幕上什么都没有（源码注释原话：`dialog/dialog.rs:287-289`）。本项目当前正处于这个状态。
5. **「一打开就能打字」的写法没有源码先例可抄**。已发布测试要么先 `render_frame` 再 `state.focus(...)`（`gpui-kit-0.6.6/tests/search.rs:80-83`），要么直接点击输入框（`tests/overlays.rs:125`）；`Root::open_dialog` 本身是「先 focus 句柄、句柄所在元素下一帧才入树」（`root.rs:309-310`）也能生效，所以第 6.3 节 Step 3 的 `window.defer` 写法**属于按此推断**，需要实测确认「浮层出现后立刻输入」不会丢字。
