//! 标题栏区域。**高度与排布的真源是 macOS 前端**（不是本文早先引的 `windows/tauri/**`
//! —— 那份前端已被删除、其调研笔记作废，见 `develop-lithe` Skill 的「Read the relevant
//! source of truth」）。
//!
//! ## 单行 **40**（2026-09-30 对齐 macOS）
//!
//! ```text
//! 唯一一行 40：菜单栏 ｜ 项目下拉 ▾ ｜ 分支项 ｜ ……拖拽区…… ｜ 运行配置 ▾ ▶ 🐞 ｜ 账号槽位 ｜ 窗口三键
//! ```
//!
//! - 高度 **40**：`LitheTheme.Metrics.toolbarHeight = 40`
//!   （`macos/Sources/Lithe/Theme/LitheTheme.swift:478`），应用点是 topBar 的
//!   `.frame(height: LitheTheme.Metrics.toolbarHeight)`
//!   （`macos/Sources/Lithe/Views/Workbench/WorkbenchView.swift:829`）。
//!   ⚠️ **本轮改的是"两行 30 + 38 = 68"**（那是 2026-09-27 按效果图 `ui-mockup-idea.md` §1.3
//!   拍板的临时口径，`docs/tasks/gpui-core-first.md` §4.2 的 U25 行也把它写作缺口）。
//! - 左右内边距 **8**、组内间距 **4**：本文件早先按 Windows 令牌
//!   `--lithe-chrome-padding-inline: 8px` / `--lithe-chrome-gap: 4px` 取值，与 macOS 的
//!   `topBar` 同量级（macOS 是 `padding(.trailing, 10)` + `HStack(spacing: 9)`，
//!   `WorkbenchView.swift:736,828`，那是**含右侧账号槽位在内**的整体间距；本侧右端收尾是
//!   自绘窗口三键，`px_2` 的 8 更稳）。这一条**保持不改**，理由见下面的"平台差异"。
//! - 左侧**不预留** macOS 的 83pt 红绿灯内衬（`WorkbenchTopBarMetrics.leadingInset = 83`，
//!   `WorkbenchView.swift:30`，原注释自陈是照 IDEA 的 78pt + 一点余量）：
//!   本侧窗口键在**右上角**且由本文件自绘（`WindowControlArea`），左上角没有要避让的系统控件，
//!   照抄 83 只会在左侧留一段死白。按 `UI-MAP.md` §1.1 第 5 条「不写平台业务分支」——
//!   平台差异交给 gpui 平台层与自绘的窗口键，**业务代码里没有 `#[cfg]`**。
//!
//! ## 菜单栏为什么在同一行
//!
//! macOS 把菜单栏画在**系统菜单栏**上（窗口内只有 topBar），而 Windows 没有可用的等价物，
//! 菜单必须留在窗口内 —— 所以它和 topBar 的内容共用这**唯一一行**，顺序照真源从左到右：
//! 菜单栏（[`crate::menu_bar`]，形态由它自己决定）→ 项目下拉 → 分支项 → 拖拽区 →
//! 运行配置 ▾ ▶ 🐞 → 账号槽位。窗口三键在右端（见下）。
//! ⚠️ 代价是真的：一行里放得下的东西比两行少，窗口很窄时菜单栏 / 运行配置槽位会先被挤到
//! 边界。本文件**不**为它写宽度分支（那会变成第二套排布），窄窗表现属未经真机验证的已知风险。
//!
//! ## 窗口三键（唯一一处"平台自有量"）
//!
//! 顺序「最小化 → 最大化/还原 → 关闭」、直角、无点击回调（由平台命中测试交给系统）。
//! 真源（macOS）**没有**这个界面点：它的红绿灯由 AppKit 画、在窗口左上。所以三键的宽度
//! [`WINDOW_CONTROL_WIDTH_SPEC`] 是**平台层的量**，本文件只保证它贴住窗口右边缘。
//!
//! ⚠️ **除拖拽区自己以外的可点元素都必须是 `drag_region` 的兄弟节点**：Windows 的命中测试取
//! `window_control_hitboxes` 里第一个命中项（`gpui-pre-0.3.7/src/window.rs`，按绘制顺序 =
//! 祖先在前），祖先的 `Drag` 会赢 —— 把它们放进 `drag_region` 里面，点它们只会拖窗口。
//! 这条与窗口三键的坑同源：**`Drag` 不能做任何可点元素的祖先**。单行结构下它们全是
//! **行容器的儿子**，`drag_region` 只是它们的兄弟。
//!
//! 为什么**不用** gpui-kit 的 `component::TitleBar`：
//!
//! 1. 它内部**无条件**再画一组自带窗口三键（`gpui-component-0.6.6/src/title_bar.rs:247-294`
//!    的 `WindowControls`，:400-402 挂进渲染树），每个控件硬编码宽 `TITLE_BAR_HEIGHT` = 34px
//!    （:15、:211）且不可定制 —— 会和本文件自绘的 44×30 三键叠成两组按钮；
//! 2. 它给可定制子元素套的是 `h_flex().flex_1()`（:371-378），外层自带 `pl(12)`（:19、:336）
//!    与 1px 下边框 `border_b_1()`（:337），与 Windows 标题栏（左右内边距 8、无边框）不一致。
//!
//! 因此这里用普通 `div()` 自建容器与三键，拖拽区用 gpui 的 `WindowControlArea`。
//! 窗口三键**不挂任何点击回调**：Windows 平台靠 `WindowControlArea` 命中测试把这三块区域
//! 交给系统（`gpui-pre-windows-0.3.6/src/events.rs:976-984`），而 gpui 侧一旦有监听器消费掉
//! 按下事件就会 return `Some(0)` 并让系统收不到该次点击（同文件 `:1078-1095`）——
//! gpui-kit 自己的三键也是这个结论（`gpui-component-0.6.6/src/title_bar.rs:106-109`、`:220-242`）。

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};
use gpui_kit::{
    AnyElement, App, Entity, Hsla, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window,
    WindowControlArea, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image};

use lithe_gpui_shared::tr;

use crate::menu_bar::{MenuAction, push_show_run_pane};

// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px（`lithe-gpui/themes/*.json` 的 `font.size` + `root.rs` 的
// `set_rem_size`），所以 helper 后缀 `N` = `N × 4px`。
//
// | 规格（macOS 真源） | 值 | 出处 | 用到的 helper |
// | --- | --- | --- | --- |
// | 标题栏高 | **40** | `LitheTheme.swift:478`（`Metrics.toolbarHeight`）→ `WorkbenchView.swift:829` | `h_10()` |
// | 左右内边距 8 / 组内间距 4 | 8 / 4 | Windows 令牌（本侧保留，见模块头"平台差异"） | `px_2()` / `gap_1()` |
// | 运行控件按钮（▶ / 🐞） | **28×28** | `WorkbenchView.swift:1079,1101`（`.frame(width: 28, height: 28)`） | `size_7()` |
// | 运行控件按钮的图标 | **16** | `WorkbenchView.swift:1073,1097`（`LitheIDEAIcon(size: 16)`） | `rems(16. / 16.)` |
// | 运行配置槽位：高 / 最小宽 / 最大宽 | **30 / 160 / 190** | `WorkbenchView.swift:1163,1164` | `rems(30. / 16.)` / `rems(160. / 16.)` / `rems(190. / 16.)` |
// | 运行配置槽位内边距 / 组内间距 | 4 / 8 | `WorkbenchView.swift:1147,1162`（`spacing: 8`、`padding(.horizontal, 4)`） | `px_1()` / `gap_2()` |
// | 账号槽位宽 | 28 | 与运行控件同档（真源这一段是 `UpdateControl` + 背景选择器） | `w_7()` |
//
// ⚠️ 编辑区文字档位：真源那几格是 11.5 / 12.5 / 13，gpui 的档位只有 `text_xs()`(12) /
// `text_sm()`(14)。按本仓既有口径用 **`text_sm()`（14px）**（13 → 14 是经维护者确认的**有意**
// 视觉改动，应用里所有 chrome 文字同步 +1px），槽位里的小字走 `text_xs()`（12）。
//
// ⚠️ gpui-kit 的 `TitleBar` 默认高度是硬编码的 34px，**用不了**（它内部无条件再画一组自带
// 窗口三键，理由见下）；本文件自建容器，高度用下面的常量显式表达。

/// 单个窗口控件（最小化 / 最大化 / 关闭）的宽度 **44**。
///
/// ## 这是**平台层的量**，不与 macOS 对齐
///
/// 真源（macOS）没有这个界面点：红绿灯由 AppKit 画在窗口**左上角**，规格里根本没有
/// 窗口内的窗口键。按 `UI-MAP.md` §1.1 第 5 条「不写平台业务分支」，本侧的三键是
/// 平台差异的承载者（`WindowControlArea`，命中后交给系统），**宽度由平台手感定**：
/// 维持上一版的 **44**，本轮只把它从"两行里的第一行"挪到"唯一一行的右端"。
///
/// 44 在档位上（`w_11()`），但仍写 `rems(C / 16.)`：与同文件既有写法一致，且不随 gpui
/// 档位表变化（默认 16px 基准下 `rems(44. / 16.)` = 44）。
const WINDOW_CONTROL_WIDTH_SPEC: f32 = 44.;

/// 标题栏的高度 **40**。
///
/// 真源（macOS）：`LitheTheme.Metrics.toolbarHeight = 40`
/// （`macos/Sources/Lithe/Theme/LitheTheme.swift:478`），应用点是 topBar 的
/// `.frame(height: LitheTheme.Metrics.toolbarHeight)`
/// （`macos/Sources/Lithe/Views/Workbench/WorkbenchView.swift:829`）。
///
/// 40 在 gpui 的 4px 档位上（`h_10()`），但仍走 `rems(TITLE_BAR_HEIGHT_SPEC / 16.)`：
/// 本文件的度量一律以具名常量表达（值 + 出处 + 理由放在一起），免得"档位内用 helper、
/// 档位外用 rems"两套写法在同一处漂移。
const TITLE_BAR_HEIGHT_SPEC: f32 = 40.;

/// 运行配置槽位里的图标 / 文字 / 计数用的高度 **30**。
///
/// 真源 `WorkbenchView.swift:1164`（`.frame(height: 30)`，`runConfigurationPicker`）。
/// 30 不在 4px 档位上（`h_7()` = 28、`h_8()` = 32），所以调用点写 `rems(30. / 16.)` ——
/// 见《编码指南》「档位外的值用 helper 底层的 `rems(P / 16.)`」。
const RUN_SLOT_HEIGHT_SPEC: f32 = 30.;

/// 运行配置槽位的最小宽度 **160**（真源 `WorkbenchView.swift:1163` 的 `minWidth: 160`）。
const RUN_SLOT_MIN_WIDTH_SPEC: f32 = 160.;

/// 运行配置槽位的最大宽度 **190**（真源 `WorkbenchView.swift:1163` 的 `maxWidth: 190`）。
///
/// 同时是"超长配置名截断"的上界（Maven 的 `artifactId` 可以很长），`truncate()` 负责省略号。
const RUN_SLOT_MAX_WIDTH_SPEC: f32 = 190.;

/// 运行配置槽位 / 运行控件按钮的圆角 **6.4**。
///
/// 与 [`crate::project_menu`] 的 `TRIGGER_RADIUS_SPEC`、[`crate::branch_panel`] 的
/// `TRIGGER_RADIUS_SPEC` 同一个值 —— 标题栏里的这一组小控件共用一个圆角，避免同一行里
/// 出现两种圆角。值的**唯一 owner** 是 [`lithe_gpui_shared::design::CONTROL_RADIUS`]
/// （`rounded-md` = `--radius × 0.8` = 6.4），本常量只是把它锚定到本文件的调用点。
const TITLE_BAR_CONTROL_RADIUS_SPEC: f32 = lithe_gpui_shared::design::CONTROL_RADIUS;

/// 标题栏（无状态）：**单行 40** 的高度与排布、右端的运行控件与自绘窗口三键都在这里，
/// 不持有 `Entity`。只依赖传入的**已渲染好的**菜单栏、项目下拉与分支项、当前主题与窗口状态。
///
/// `menu_bar` / `project_menu` / `branch_item` 是调用方（[`crate::workspace::ShellWorkspace::render`]）
/// 先渲染好的元素，收 `AnyElement` 而不是收 `Entity<..>` + `&mut App` 有四个原因：
///
/// 1. 本 crate 是 edition 2024，`-> impl IntoElement` 会捕获签名里所有在作用域的生命周期
///    （见 `lib.rs` 模块头的"区域渲染函数必须收 `&Window` / `&App`"），多一个 `&mut App`
///    参数会把可变借用带进返回值；
/// 2. 菜单栏的形态（常驻 / 图标）由它自己决定要画哪一种，标题栏不需要知道这件事；
/// 3. 项目下拉要读 `Entity<ProjectMenu>` 并新建 `PopupMenu`（要 `&mut App`），
///    那一步必须发生在调用方，而不是这个只读的区域函数里；
/// 4. 分支项同样要读 `Entity<BranchPanel>`（开合态决定 hover / 展开外观），
///    而且它**可以不存在**（没有仓库 / 没有分支名时真源整项不渲染），
///    所以调用方给 `AnyElement`（缺省时是一块空 `div`）。
///
/// ⚠️ **项目名不由本函数画**：它画在项目下拉的触发器里，所以本函数收的是**已经带名字的
/// 触发器元素**，拖拽区因此只是一块空的剩余宽度。
///
/// ⚠️ **落位（单行，2026-09-30 对齐 macOS）**，从左到右：
///
/// 1. `menu_bar`（菜单栏——macOS 在系统菜单栏上，Windows 必须画在窗内，所以共用这一行）；
/// 2. `project_menu`（项目下拉触发器）；
/// 3. `branch_item`（分支项，可以不存在）；
/// 4. [`drag_region`]（该行剩下的宽度全是它）；
/// 5. [`run_controls`]（`运行配置 ▾ ▶ 🐞`）；
/// 6. [`account_slot`]（账号 / 设置槽位，只留空隙）；
/// 7. [`window_controls`]（窗口三键，贴窗口右边缘）。
///
/// ⚠️ **上述可点元素全都是 `drag_region` 的兄弟节点，不能放进它内部**：Windows 的命中测试取
/// `window_control_hitboxes` 里**第一个**命中项（`gpui-pre-0.3.7/src/window.rs`，
/// 按绘制顺序 = 祖先在前），祖先的 `Drag` 会赢，那时点它们只会拖窗口
/// （"点菜单只拖窗口"那个真 bug 的成因）。本条与本文件窗口三键的 `Drag` 祖先坑同源（见下）。
pub fn title_bar(
    menu_bar: AnyElement,
    project_menu: AnyElement,
    branch_item: AnyElement,
    run: Entity<lithe_gpui_run::RunPane>,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    v_flex()
        // 标题栏不参与压缩。
        .flex_shrink_0()
        .w_full()
        // 底色走主题自己的 `title_bar`（gpui-kit 的 `ThemeColor` 没有 `surface` 这一项，
        // 语义上最贴近「标题栏专用底色」的就是它）。
        .bg(cx.theme().title_bar)
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        // 唯一一行 **40**（[`TITLE_BAR_HEIGHT_SPEC`]，macOS `LitheTheme.swift:478`）。
        .child(
            h_flex()
                .w_full()
                .h(rems(TITLE_BAR_HEIGHT_SPEC / 16.))
                .child(
                    // 左段：菜单栏 → 项目下拉 → 分支项 → 拖拽区 → 运行控件 → 账号槽位。
                    // `flex_1` 撑开，右端的三键因此被推到窗口最右；**8px 水平内衬只加在这一段**，
                    // 三键才能贴住窗口右上角（hover 底色一路铺到边缘）。
                    h_flex()
                        // 左段独占剩余宽度：`flex_1` 撑开，三键因此被推到最右端。
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .gap_1()
                        .px_2()
                        // ① 菜单栏：`drag_region` 的**兄弟**（挂在左段上，不在 `drag_region` 里）。
                        // 常驻形态占左侧一段，图标形态只占一个按钮 + 一层浮动胶囊 —— 由它自己决定。
                        .child(menu_bar)
                        // ② 项目下拉触发器：`drag_region` 的**兄弟**（见 [`crate::project_menu`] 模块头）。
                        .child(project_menu)
                        // ③ 分支项：仍是 `drag_region` 的**兄弟**（见 [`crate::branch_panel`] 模块头）；
                        // 没有仓库 / 没有分支名时调用方传一块空 `div`，与真源"整项不渲染"一致。
                        .child(branch_item)
                        // ④ 拖拽区独占剩余宽度（`flex_1`）—— 所以这一行**不需要** `justify_between()`：
                        // 左右两组各自按内容宽度贴在两端，中间全归拖拽区。
                        .child(drag_region())
                        // ⑤ 运行控件（`运行配置 ▾ ▶ 🐞`）：见 [`run_controls`]。
                        .child(run_controls(run, cx))
                        // ⑥ 「账号 / 设置」槽位：只留空隙，见 [`account_slot`]。
                        .child(account_slot()),
                )
                // ⑦ 窗口三键：**行容器的儿子**、不在左段里面，所以那一行的 8px 内衬不会把它
                // 推离窗口边缘（见模块头的"窗口三键"条目）。
                .child(window_controls(window, cx)),
        )
}

/// 拖拽区：**唯一一行**里除其它子元素之外的**剩余宽度**（每帧调一次）。
///
/// 该行其它元素（菜单栏 / 项目下拉 / 分支项 / 运行控件 / 账号槽位 / 窗口三键）都是它的
/// **兄弟节点**而不是子节点 —— 这一点必须保持：平台命中测试取
/// `window_control_hitboxes` 里**第一个**命中项（`gpui-pre-0.3.7/src/window.rs`，
/// 按绘制顺序 = 祖先在前），若把 `Drag` 挂在整条标题栏（或整个行容器）的祖先上，
/// 它会盖住子节点的 `Min` / `Max` / `Close`，三键就永远点不动了
/// （gpui-kit 自己也是把 `Drag` 挂在三键的兄弟 `h_flex` 上）。
///
/// ⚠️ **不画项目名**：项目名归项目下拉的触发器，本区域只提供可拖动的空白。
fn drag_region() -> impl IntoElement {
    h_flex()
        .flex_1()
        .min_w_0()
        .h_full()
        // 拖拽区：`WindowControlArea::Drag` 在 Windows 上命中测试为 `HTCAPTION`
        // （`gpui-pre-windows-0.3.6/src/events.rs:976-977`），拖动由系统完成。
        .window_control_area(WindowControlArea::Drag)
        // 非 Windows 平台 `WindowControlArea` 不一定由平台层实现，而 `mod.rs` 给窗口用的是
        // `TitleBar::window_options()`（`app_owns_titlebar_drag: true`，
        // `gpui-component-0.6.6/src/title_bar.rs:81-91,88`），要求应用自己发起移动。
        // `Window::start_window_move`（`gpui-pre-0.3.6/src/window.rs:2848`）在 Windows 平台是
        // 空实现（`gpui-pre-windows-0.3.6` 内无该方法，用 `gpui-pre-0.3.6/src/platform.rs:1008`
        // 的默认空实现），所以这里是「Windows 由 HTCAPTION 拖、其它平台由本回调拖」，
        // 不需要 `#[cfg]`，也不消费事件（`handle_nc_mouse_down_msg` 只有被消费时才拦截，
        // `gpui-pre-windows-0.3.6/src/events.rs:1078-1083`）。
        .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
}

/// 窗口三键组：三键之间 0 间距、整组 `h_full()` 撑满**唯一一行**（40）。
///
/// 整组贴住窗口右边缘 —— 它挂在行容器上、不在那一行的左段里，所以左段的 8px 内衬
/// 不会把它推离边缘。宽度 [`WINDOW_CONTROL_WIDTH_SPEC`]（平台自有的量，见该常量文档）。
fn window_controls(window: &Window, cx: &App) -> impl IntoElement {
    h_flex()
        .flex_shrink_0()
        .h_full()
        .gap_0()
        .child(window_control(WindowControl::Minimize, cx))
        .child(window_control(
            // 最大化/还原二选一。
            if window.is_maximized() {
                WindowControl::Restore
            } else {
                WindowControl::Maximize
            },
            cx,
        ))
        .child(window_control(WindowControl::Close, cx))
}

/// 单个窗口控件：宽 [`WINDOW_CONTROL_WIDTH_SPEC`]（44）× **整行高 40**、直角、无点击回调
/// （见文件头说明），只有悬停态。
///
/// ⚠️ 尺寸是**平台自有的量**（真源 macOS 没有这个界面点，见 [`WINDOW_CONTROL_WIDTH_SPEC`]）：
/// 这里只有"撑满整行、贴右边缘、直角"三条是外观约束。
fn window_control(control: WindowControl, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let hover_background = control.hover_background(cx);
    let hover_foreground = control.hover_foreground(cx);

    div()
        // 必须有 `id`：gpui 的 `hover` / `active` 态存在元素状态里
        // （`gpui-pre-0.3.6/src/elements/div.rs:2844-2849` 读 `element_state.hover_state`，
        // `:3503-3518` 读 `clicked_state`），无 id 的 `Div` 拿不到元素状态。
        .id(control.id())
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .w(rems(WINDOW_CONTROL_WIDTH_SPEC / 16.))
        .h_full()
        // `rounded-none`（`window-controls.tsx:59,71,85`）：直角。
        .rounded_none()
        // 未悬停前景：最小化/最大化是 ghost 的 `text-subtle-foreground`、关闭是 danger 的
        // `text-foreground`（`windows/tauri/src/ui/button.tsx:17,19`）。
        // `--subtle-foreground` 在 gpui-kit 里没有独立 token，与 `--muted-foreground` 合并。
        .text_color(if control.is_close() {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        // 命中测试区域：Windows 上分别映射为 `HTMINBUTTON` / `HTMAXBUTTON` / `HTCLOSE`
        // （`gpui-pre-windows-0.3.6/src/events.rs:978-984`），按下/抬起由系统完成动作。
        .window_control_area(control.area())
        .hover(move |style| style.bg(hover_background).text_color(hover_foreground))
        // 图标 14×14：`Sizable::small()` → `Size::Small` → `size_3p5()` = 14px
        // （`gpui-component-0.6.6/src/sizing.rs:193`、`.../src/icon.rs:181-187`）。
        .child(Icon::new(control.icon()).small())
}

/// 窗口三键。顺序照 `window-controls.tsx:56-94`：最小化 → 最大化/还原 → 关闭。
#[derive(Clone, Copy)]
enum WindowControl {
    Minimize,
    Maximize,
    Restore,
    Close,
}

impl WindowControl {
    /// 稳定的元素 id（各自唯一）。
    fn id(self) -> &'static str {
        match self {
            Self::Minimize => "title-bar-window-minimize",
            Self::Maximize => "title-bar-window-maximize",
            Self::Restore => "title-bar-window-restore",
            Self::Close => "title-bar-window-close",
        }
    }

    /// 字形取自全量 Lucide 目录 `gpui_kit::assets::IconName`：`Minus` / `Square` / `Copy` / `X`
    /// —— 与真源**逐字对齐**（`window-controls.tsx:2-7,66,78,93` 用的就是 lucide 的
    /// `Minus` / `Square` / `CopyIcon as Restore` / `X`；四个 svg 都在
    /// `gpui-kit-assets-0.7.0/assets/icons/` 里）。
    ///
    /// ⚠️ **2026-09-30 换过一次**（维护者口径「这三个图标可以优化一下」）。此前用的是
    /// `WindowMinimize` / `WindowMaximize` / `WindowRestore` / `WindowClose`，当时的理由是
    /// "`window-*` 这一组正是 lucide 为窗口三键提供的等价字形"—— **那条理由是错的**：
    /// 这四个 `window-*.svg` 不是 Lucide，而是 **MDI 精灵图**（`fill="#000000"` +
    /// `fill-rule="nonzero"` + `translate(-40/80/120, 0)` 的排版变换，实心路径），
    /// 描边只有 1.6 / 24 个单位宽 —— 14px 下不到 1px，渲染出来是发虚的灰边，与本应用其余
    /// 单色字形（Lucide，`stroke-width="2"`）不是同一套语言。换回真源那四个之后，
    /// 三键与 `project_tabs` / `right_tool_window` 的关闭按钮用的是同一套字形。
    fn icon(self) -> IconName {
        match self {
            Self::Minimize => IconName::Minus,
            Self::Maximize => IconName::Square,
            Self::Restore => IconName::Copy,
            Self::Close => IconName::X,
        }
    }

    /// 平台命中区域：`Minimize → Min`、`Maximize/Restore → Max`、`Close → Close`
    /// （`window-controls.tsx:27-49` 的动作与 gpui-kit 的映射一致：
    /// `gpui-component-0.6.6/src/title_bar.rs:155-161`）。
    fn area(self) -> WindowControlArea {
        match self {
            Self::Minimize => WindowControlArea::Min,
            Self::Maximize | Self::Restore => WindowControlArea::Max,
            Self::Close => WindowControlArea::Close,
        }
    }

    fn is_close(self) -> bool {
        matches!(self, Self::Close)
    }

    /// 悬停底色：最小化/最大化用 ghost 的 `hover:bg-accent`；关闭用 Windows 覆盖过的
    /// `hover:bg-destructive`（`window-controls.tsx:84-85`、`ui/button.tsx:17,19`）。
    /// `--accent` → `theme.accent`、`--destructive` → `theme.danger`
    /// （gpui-kit 的 `ThemeColor` 里没有 `destructive`，对应项叫 `danger`：
    /// `gpui-component-0.6.6/src/theme/theme_color.rs:61,147`）。
    fn hover_background(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        if self.is_close() {
            theme.danger
        } else {
            theme.accent
        }
    }

    /// 悬停前景：最小化/最大化 ghost 的 `hover:text-foreground` → `theme.foreground`
    /// （`gpui-component-0.6.6/src/theme/theme_color.rs:163`）；关闭的 `hover:text-white`
    /// 在 gpui-kit 里没有白色 token，改用主题的「危险底上的前景」
    /// `theme.danger_foreground` —— 这也是 gpui-kit 自己三键的做法
    /// （`gpui-component-0.6.6/src/title_bar.rs:168-174`、`theme_color.rs:151`）。
    fn hover_foreground(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        if self.is_close() {
            theme.danger_foreground
        } else {
            theme.foreground
        }
    }
}

// ---------------------------------------------------------------------------
// 右侧的运行控件（`运行配置 ▾ ▶ 🐞`）
// ---------------------------------------------------------------------------
//
// ⚠️ **本节三个元素都是 `drag_region` 的兄弟**（[`title_bar`] 把它们挂在行容器上），
// 不是它的后代 —— 原因见本文件模块头：`Drag` 做了可点元素的祖先，Windows 命中测试里
// 祖先先命中，点按钮只会拖窗口。

/// 运行控件组：`运行配置 ▾` ＋ `▶` ＋ `🐞`。
///
/// 形状与顺序照 macOS topBar 的右组：`runConfigurationPicker` → `runLaunchButton` →
/// `debugLaunchButton`（`macos/Sources/Lithe/Views/Workbench/WorkbenchView.swift:814-821`
/// 的 `HStack(spacing: 8)`），尺寸各自见那两个函数的文档。
fn run_controls(run: Entity<lithe_gpui_run::RunPane>, cx: &App) -> impl IntoElement {
    h_flex()
        .flex_shrink_0()
        .h_full()
        // 组内 8：真源 `WorkbenchView.swift:814`（`HStack(spacing: 8)`）。
        .gap_2()
        .child(run_configuration_slot(run.clone(), cx))
        .child(run_control(RunControl::Run, run.clone(), cx))
        .child(run_control(RunControl::Debug, run, cx))
}

/// 右端的「账号 / 设置」槽位：**只留空隙、不画控件**。
///
/// macOS topBar 的右端是 `UpdateControl(compact: true)` + `backgroundPickerButton`
/// （`WorkbenchView.swift:823-824`）。本侧**不接**这两个入口：
///
/// - 「设置」在 Lithe 已有两个入口（左活动栏底部组的「设置」项，`crate::workspace` 的
///   `SETTINGS_ACTIVITY_IX`，以及 `Ctrl+,` 的 `MenuAction::Preferences`），再画一个第三入口
///   只会多一处状态；
/// - 「账号」在本侧没有任何数据源（没有登录 / 同步子系统）。
///
/// 所以这里只保留那一段**固定宽度的空隙**（28 = `w_7()`，与运行控件按钮同宽），
/// 让右端的留白与真源一致 —— **不画一个点了没反应的按钮**（《编码指南》的占位项纪律：
/// 不许假装能用）。
fn account_slot() -> impl IntoElement {
    div().flex_shrink_0().w_7()
}

/// `运行配置 ▾`：右端的文字槽位。
///
/// ## 度量（真源 `WorkbenchView.swift:1143-1170` 的 `runConfigurationPicker`）
///
/// 高 30、最小宽 160、最大宽 190、左右内边距 4、组内间距 8、图标 14、文字档位见下。
///
/// ## 显示的是**真实**的当前配置名
///
/// 从运行面板实体读（`lithe_gpui_run::RunPane::selected`）：有配置时显示它的名字，
/// 一条都没有（未打开 Java 工程 / 还没识别出来）时退回工具窗自己的名字
/// `lithe.run.configurations`（zh「运行配置」）。**不编假配置名**。
///
/// ## 点它 = 打开运行工具窗（那里才是配置列表）
///
/// 真源这一格点开是一个下拉清单（`isRunConfigurationPickerPresented` → popover）。
/// 本侧**不画假下拉**：列表在底部「运行」工具窗里（`lithe_gpui_run::RunPane`，点一行即选中），
/// 所以这里的落点是把那个工具窗切过来并显示 —— 与「视图 → 运行和调试」走同一条
/// `MenuAction::ShowRunAndDebug`（同一个待执行队列）。等这一格真的有了弹出菜单组件
/// （`Entity<ProjectMenu>` 那种），再把它换成内联下拉。
fn run_configuration_slot(run: Entity<lithe_gpui_run::RunPane>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let idle = theme.muted_foreground;
    let hover_bg = theme.accent;
    let hover_fg = theme.foreground;
    // 配置名来自**运行面板**（同一份 `RunConfigurationView` 模型，不另解析一次）。
    let label = run
        .read(cx)
        .selected()
        .map(|configuration| SharedString::from(configuration.name.clone()))
        .unwrap_or_else(|| tr("lithe.run.configurations"));

    h_flex()
        .id("title-bar-run-configuration")
        .flex_shrink_0()
        // 高 30（不在 4px 档位上：`h_7()`=28、`h_8()`=32）→ `rems(30. / 16.)`。
        .h(rems(RUN_SLOT_HEIGHT_SPEC / 16.))
        // `padding(.horizontal, 4)`（`WorkbenchView.swift:1162`）→ `px_1()`。
        .px_1()
        // `HStack(spacing: 8)`（`WorkbenchView.swift:1147`）→ `gap_2()`。
        .gap_2()
        .min_w(rems(RUN_SLOT_MIN_WIDTH_SPEC / 16.))
        .max_w(rems(RUN_SLOT_MAX_WIDTH_SPEC / 16.))
        .rounded(rems(TITLE_BAR_CONTROL_RADIUS_SPEC / 16.))
        .text_color(idle)
        .cursor_pointer()
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        // 与两个图标按钮同样的第二道保险（防"将来被挪进拖拽区"）。
        .on_mouse_down(MouseButton::Left, |_event, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_click(|_event, _window, cx| {
            push_show_run_pane(cx);
        })
        .aria_label(tr("lithe.run.configurations"))
        // 文字档位：真源 11.5（`WorkbenchView.swift:1153`）→ gpui 最近的档位 `text_xs()`（12）。
        .child(div().min_w_0().truncate().text_xs().child(label))
        // chevron 9（`WorkbenchView.swift:1157-1159` 的 `size: 9, weight: .semibold`）→
        // `size_2p5()`（10px，gpui 档位里离 9 最近的一档；9 不在档位上，不能发明 helper）。
        // 不用 `small()`（=14）：那会让 chevron 比旁边的 12px 文字还高。
        .child(Icon::new(IconName::ChevronDown).size_2p5())
}

/// 一个运行控件按钮（`▶` / `🐞`）：**28×28**、圆角 6.4、悬停底色。
///
/// 尺寸照真源 `WorkbenchView.swift:1079,1101,1125`（`.frame(width: 28, height: 28)`），
/// 图标照 `:1073,1097`（`LitheIDEAIcon(size: 16)`）。
/// ⚠️ **前景色不吃 `text_color`**：图标是彩色 `img()`（见 [`RunControl::icon`]），
/// 所以这里没有"未悬停 muted、悬停 foreground"那对颜色 —— 反馈只有底色。
fn run_control(
    control: RunControl,
    run: Entity<lithe_gpui_run::RunPane>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    // 先按值取出来再进闭包：`cx.theme()` 借 `cx`，而 `.hover(..)` 的闭包要 `'static`。
    let hover_bg = theme.accent;

    div()
        // 必须有 `id`：`hover` 态要元素状态。
        .id(control.id())
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        // 28×28（`size_7()`）：真源 `WorkbenchView.swift:1079,1101`。
        .size_7()
        .rounded(rems(TITLE_BAR_CONTROL_RADIUS_SPEC / 16.))
        .cursor_pointer()
        // 悬停反馈**只有底色**：图标是彩色 `img()` 管线，不吃 `text_color`。
        .hover(move |style| style.bg(hover_bg))
        // 与项目下拉 / 分支项触发器逐字相同的第二道保险：本元素此刻**不在** `Drag` 命中区里
        // （它是 `drag_region` 的兄弟），这两行挡的是"将来有人把它挪进拖拽区"这类回归。
        // 它不是兄弟关系的替代品 —— 真正的约束是布局，见本文件模块头。
        .on_mouse_down(MouseButton::Left, |_event, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        // 点击**直接**让运行面板起一次（不再走菜单队列）：`▶` = 普通运行，`🐞` = 调试方式
        // （JDWP 等调试器接入，见 `lithe_gpui_run` 的模块文档）。面板自己会：
        // 起进程 → 通过 `RunPaneEvent::Started` 让外壳把底部工具窗切到「运行」。
        .on_click(move |_event, _window, cx| {
            let debug = matches!(control, RunControl::Debug);
            let _ = run.update(cx, |pane, cx| pane.launch(debug, cx));
        })
        // 无障碍名 = 它复用的那一条菜单文案（图标按钮没有可见文字，必须给名字）。
        .aria_label(tr(control.label_key()))
        // 图标 **16×16**：真源 `WorkbenchView.swift:1073,1097`（`LitheIDEAIcon(size: 16)`）。
        .child(control.icon(cx))
}

/// 右侧的两个运行控件。
///
/// 只 derive `Clone, Copy`（与 [`WindowControl`] 同一口径）：本枚举只用来**选一行**分支，
/// 不参与比较，多 derive 一个 `PartialEq` 只会让"将来有人拿它当状态比较"看起来可行。
#[derive(Clone, Copy)]
enum RunControl {
    /// `▶` 运行：按当前选中的运行配置起一次进程。
    Run,
    /// `🐞` 调试：同一条计划 + `debugPort`（JVM 以 JDWP 起、挂着等调试器接入）。
    Debug,
}

impl RunControl {
    /// 稳定的元素 id（各自唯一，给 `hover` 态用）。
    fn id(self) -> &'static str {
        match self {
            Self::Run => "title-bar-run-control-run",
            Self::Debug => "title-bar-run-control-debug",
        }
    }

    /// 字形：`▶` = `idea::PLAY_ICON`、`🐞` = `idea::BUG_ICON`，两条都走**彩色** `img()` 管线
    /// （[`idea_icon_image`]）。
    ///
    /// ⚠️ **彩色 → 不吃 `text_color`**：图标保留资产自带的颜色，悬停反馈由底色承担
    /// （见 `.agents/notes/implemented/feature/2026-09-30-colored-ui-icons.md`）。
    ///
    /// 尺寸 **16**：真源 `WorkbenchView.swift:1073,1097`（`LitheIDEAIcon(size: 16)`）——
    /// 原来是 14（照 Windows 的 `size-3.5`），本轮随按钮 24 → 28 一起放大到真源的 16。
    /// 彩色那条**自己带尺寸**，`.child(..)` 不会替它设。
    fn icon(self, cx: &App) -> AnyElement {
        match self {
            Self::Run => idea_icon_image(&idea::PLAY_ICON, cx, rems(16. / 16.)),
            Self::Debug => idea_icon_image(&idea::BUG_ICON, cx, rems(16. / 16.)),
        }
    }

    /// 无障碍名 / 悬停提示复用的是**菜单里的哪一条**（图标按钮没有可见文字，必须给名字）。
    fn label_key(self) -> &'static str {
        match self {
            // 「视图 → 运行和调试」：键从 `MenuAction` 现取，不在这里抄第二份字符串。
            Self::Run => MenuAction::ShowRunAndDebug.label_key(),
            // 「运行 → 开始调试」：与菜单里那条**同一个键**（`crate::menu_bar` 的 `MENUS`）。
            Self::Debug => "lithe.menu.startDebugging",
        }
    }
}
