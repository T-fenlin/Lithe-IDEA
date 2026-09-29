//! 标题栏区域（Windows 规格）。
//!
//! 规格真源（一手源码，逐条核对过行号）：
//!
//! - 容器：`windows/tauri/src/features/window/components/title-bar/title-bar.tsx:332-360`
//!   —— `h-(--lithe-title-bar-height)` = **40**、`items-center justify-between`、
//!   `gap-(--lithe-chrome-gap)` = **4**、`px-(--lithe-chrome-padding-inline)` = **8**、
//!   `bg-surface`、`text-muted-foreground`、`ui-text-chrome` = **13px**。
//!   ⚠️ **高度已不照这一条**（2026-09-27）：按效果图定稿口径
//!   （`lithe-gpui/docs/ui-mockup-idea.md` §1.3「标题栏：**两行**」）把单行 40 拆成
//!   **两行 30 + 38 = 68**，取值与理由写在 [`TITLE_BAR_MENU_ROW_HEIGHT_SPEC`] 与
//!   [`TITLE_BAR_MAIN_ROW_HEIGHT_SPEC`] 上；内边距 / 间距 / 底色 / 字色仍照上面这条真源。
//! - 窗口三键：`.../title-bar/window-controls.tsx:51-96` —— 顺序「最小化 → 最大化/还原 → 关闭」，
//!   每个 `h-full w-14 min-w-14 rounded-none` = **56×40、直角**；组内 `gap="none"` = 0；
//!   最小化/最大化 `variant=ghost`、关闭 `variant=danger` + `hover:bg-destructive hover:text-white`。
//!   ⚠️ 本侧三键跟着**第二行**走，所以高度是 38（**56×38**），不是真源的 40 —— 见
//!   [`TITLE_BAR_MAIN_ROW_HEIGHT_SPEC`]。
//!   图标尺寸 `windows/tauri/src/ui/button.tsx:9` 的 `[&_svg:not([class*='size-'])]:size-3.5` = **14×14**。
//! - 高度 / 间距 / 内边距 / 字号令牌：`windows/tauri/src/styles/theme.css:115,118,126-133`。
//! - 中文文案：`windows/tauri/src/i18n/locale.ts:6191`（`titleProject.trigger` = `项目：{project}`，
//!   本区域按可见规格只显示项目名本身，见 `title_bar` 的文档）。
//!
//! ## 两行的排布（2026-09-27 效果图口径）
//!
//! ```text
//! 第一行 30：菜单栏（文件 编辑 视图 转到 终端 Git 工具 窗口 帮助）｜…………拖拽区…………
//! 第二行 38：项目下拉 ▾ ｜ 分支项 ｜…………拖拽区…………｜ 运行配置 ▾ ▶ 🐞 ｜ 账号槽位 ｜ 窗口三键
//! ```
//!
//! **第一行只有菜单栏**（效果图 §1.3 的定稿表格）。真源把菜单栏画在**标题栏同一行里**：
//! 常驻形态 `title-bar.tsx:230-232`、紧凑形态是左上角的 `ListIcon` 按钮 + 一行浮动胶囊
//! （`:207-229`）。两种形态都走 [`crate::menu_bar::menu_bar`]，本文件只负责给它一个位置 ——
//! 2026-09-27 起这个位置变成**独占第一行**。
//!
//! **第二行的左侧组**是真源 `ChromeGroup grow min-w-0 { menuItem, projectControls }`
//! （`title-bar.tsx:339-343`）里去掉菜单栏之后剩下的两项：项目名下拉在前、分支项紧跟其后
//! （真源同一个 `ChromeGroup gap="tight"`，gap = 2px，`title-bar.tsx:236-238`），
//! 所以参数顺序是「项目下拉 → 分支项」。项目名本身**画在触发器里**
//! （`title-project-menu.tsx:157`），所以本文件的拖拽区不再重复画一遍。
//! ⚠️ 分支触发器上**没有** `▾`：真源的子节点只有分支图标 / 分支名 / ahead-behind 箭头
//! （`git-branch-manager.tsx:646-661`），维护者截图里的那个 caret 是左邻项目下拉的
//! `ChevronDownIcon`（`title-project-menu.tsx:157-163`），两者只隔 2px
//! （研究 `lithe-gpui/research/windows/12-branch-manager.md` §1.2 与 §7.2 第 1 条）。
//!
//! **第二行的右侧组**是本轮新增的运行控件（[`run_controls`]）＋「账号 / 设置」槽位
//! （[`account_slot`]）＋窗口三键（[`window_controls`]），顺序照效果图 §1.3。
//!
//! ⚠️ **两行里除拖拽区自己以外的可点元素都必须是 `drag_region` 的兄弟节点**：Windows 的
//! 命中测试取 `window_control_hitboxes` 里第一个命中项（`gpui-pre-0.3.6/src/window.rs:1952-1956`，
//! 按绘制顺序 = 祖先在前），祖先的 `Drag` 会赢 —— 把它们放进 `drag_region` 里面，
//! 点它们只会拖窗口。这条与下面窗口三键的坑同源：**`Drag` 不能做任何可点元素的祖先**。
//! 两行结构下的落法：**每一行各有一个 `drag_region()`**（各占"这一行剩下的宽度"），
//! 菜单栏挂在第一行容器上，项目下拉 / 分支项 / 运行控件 / 账号槽位 / 窗口三键挂在第二行
//! 容器上 —— 它们都是**行容器的儿子**，`drag_region` 只是它们的兄弟。
//!
//! 为什么**不用** gpui-kit 的 `component::TitleBar`：
//!
//! 1. 它内部**无条件**再画一组自带窗口三键（`gpui-component-0.6.6/src/title_bar.rs:247-294`
//!    的 `WindowControls`，:400-402 挂进渲染树），每个控件硬编码宽 `TITLE_BAR_HEIGHT` = 34px
//!    （:15、:211）且不可定制 —— 会和本文件自绘的 56×38 三键叠成两组按钮；
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
    AnyElement, App, Hsla, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, Window, WindowControlArea, div, rems,
};

use lithe_gpui_shared::tr;

use crate::menu_bar::{MISSING_DEBUGGER_HOST, MenuAction, Missing};

// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px（`lithe-gpui/themes/*.json` 的 `font.size` + `root.rs:582` 的
// `set_rem_size`），所以 helper 后缀 `N` = `N × 4px`。**内边距 / 间距 / 字号**仍与 Windows
// 规格逐像素相等；**行高**是 2026-09-27 效果图口径（30 + 38），不在 4px 档位上，见常量：
//
// | 规格（Windows 真源 / 效果图） | 值 | 用到的 helper |
// | --- | --- | --- |
// | 第一行高（效果图 §1.3「标题栏：两行」，第一行只有菜单） | 30 | `rems(TITLE_BAR_MENU_ROW_HEIGHT_SPEC / 16.)` |
// | 第二行高（效果图 §1.3 的第二行） | 38 | `rems(TITLE_BAR_MAIN_ROW_HEIGHT_SPEC / 16.)` |
// | `--lithe-chrome-padding-inline: 8px`（`styles/theme.css:132`） | 8 | `px_2()` |
// | `--lithe-chrome-gap: 4px`（`styles/theme.css:130`） | 4 | `gap_1()` |
// | `--ui-text-chrome: 13px`（`styles/theme.css:115`） | 13 | `text_sm()`（14px，见下） |
// | 运行控件 / 账号槽位尺寸（`icon-xs` = `size_6()`，`ui/button.tsx:27`） | 24 | `size_6()` / `h_6()` / `w_6()` |
//
// ⚠️ 不在此表的 `w-14` = **56** 见下方常量：56 不在 gpui 的 rem 档位上。
//
// 字号：`--ui-text-chrome` 是 **13px**，gpui 的档位只有 `text_xs()`(12) 与 `text_sm()`(14)，
// 13 不在档位上。按《编码指南》这里用 **`text_sm()`（14px）**——Lithe 原基准是 13px，
// 13 → 14 是经维护者确认的**有意**视觉改动（应用里所有 chrome 文字同步 +1px），不是等价换算。
//
// ⚠️ gpui-kit 的 `TitleBar` 默认高度是硬编码的 34px
// （`gpui-component-0.6.6/src/title_bar.rs:15` `TITLE_BAR_HEIGHT`），所以这里必须给两行各自
// 显式的高度（下面两个常量），不能靠它的默认值。

/// 单个窗口控件的宽度：`w-14` = 56px
/// （`windows/tauri/src/features/window/components/title-bar/window-controls.tsx:59,71,85`）。
///
/// 56 不在 gpui 的固定 rem 档位上（档位后缀是 `…_11()`=44、`_12()`=48、`_16()`=64，
/// `gpui-pre-macros-0.3.6/src/styles.rs:1003-1017`；没有 `_14()`）。Tailwind 的 `w-14` 是任意档位，
/// gpui 的档位表里没有对应项，不能自己发明 helper —— 档位外就写 helper 底层的
/// `rems(WINDOW_CONTROL_WIDTH_SPEC / 16.)`（默认 16px 基准下与 56 逐像素相等，且随字号缩放）。
const WINDOW_CONTROL_WIDTH_SPEC: f32 = 56.;

/// 标题栏**第一行**（菜单栏独占）的高度 30。
///
/// ## 为什么是 30
///
/// 出处是效果图定稿口径 `lithe-gpui/docs/ui-mockup-idea.md` §1.3 的表格（「标题栏：**两行**。
/// 第一行菜单（中文）：文件 / 编辑 / 视图 / 转到 / 终端 / Git / 工具 / 窗口 / 帮助」），
/// 同一份文档 §4 记着 IDEA New UI 的工具栏目测值 ≈**30**（那一行标了"待量"）——
/// 本轮就按它落值。
///
/// ⚠️ **这不是 Windows 真源的值**：真源只有**一条** 40 高的标题栏
/// （`--lithe-title-bar-height`，`windows/tauri/src/features/window/components/title-bar/title-bar.tsx:337`
/// 与 `windows/tauri/src/styles/theme.css:118`），两行是 2026-09-27 维护者拍板的结构改动
/// （`ui-mockup-idea.md` §7 的 D-1「尺寸真源以谁为准」**仍未决**，本轮只在标题栏这一处按效果图）。
///
/// ⚠️ 30 不在 gpui 的 4px 档位上（`h_7()` = 28、`h_8()` = 32；档位表见
/// `gpui-pre-macros-0.3.6/src/styles.rs:983-992`），也不能自己发明 helper —— 按《编码指南》
/// 写档位外的 `rems(TITLE_BAR_MENU_ROW_HEIGHT_SPEC / 16.)`（16px 基准下 = 30，且随字号缩放）。
const TITLE_BAR_MENU_ROW_HEIGHT_SPEC: f32 = 30.;

/// 标题栏**第二行**（项目下拉 / 分支项 / 拖拽区 / 运行控件 / 账号槽位 / 窗口三键）的高度 38。
///
/// ## 为什么是 38（以及两行为什么必须成对看）
///
/// 出处同 [`TITLE_BAR_MENU_ROW_HEIGHT_SPEC`]（效果图 §1.3 的第二行）。取 38 的直接后果是
/// **两行合计 68**，比上一版单行 40 **净增 28px** —— 这 28px 是这一轮唯一新增的纵向占用，
/// 所以底部工具窗同步从 320 收到 240（`crate::workspace` 里的 `BOTTOM_PANE_HEIGHT_SPEC`），
/// 保证"窗口总高不变、项目树底部不被挤出窗口"。
/// 两处是**一对**：只改这里不改底部工具窗，中央列就会把内容顶出窗口。
///
/// ⚠️ 连带的下场：窗口三键的高度从 40 变成 38（它们用 `h_full()` 撑满第二行），
/// 宽度仍是 [`WINDOW_CONTROL_WIDTH_SPEC`] = 56 —— 这是本结构改动的**有意**结果，
/// 效果图第二行给三键留的就是这一行的高度。
///
/// ⚠️ 38 同样不在 4px 档位上（`h_9()` = 36、`h_10()` = 40，
/// `gpui-pre-macros-0.3.6/src/styles.rs:993-1002`），所以也写
/// `rems(TITLE_BAR_MAIN_ROW_HEIGHT_SPEC / 16.)`。
const TITLE_BAR_MAIN_ROW_HEIGHT_SPEC: f32 = 38.;

/// 第二行右侧小控件（运行控件按钮 / 运行配置槽位）的圆角 6.4。
///
/// 真源 `rounded-md` = `--radius × 0.8`（`windows/tauri/src/styles/theme.css:7,134`），
/// 与 [`crate::project_menu`] 的 `ROW_RADIUS_SPEC`、[`crate::branch_panel`] 的
/// `TRIGGER_RADIUS_SPEC` 同一个值 —— 标题栏第二行里的小控件共用一个圆角，避免同一行里
/// 出现两种圆角。`Styled::rounded` 收 `impl Into<AbsoluteLength>`，所以调用点写
/// `rems(TITLE_BAR_CONTROL_RADIUS_SPEC / 16.)`。
const TITLE_BAR_CONTROL_RADIUS_SPEC: f32 = 6.4;

/// 标题栏（无状态）：**两行**的高度与排布、第二行右侧的运行控件与自绘窗口三键都在这里，
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
///    而且它**可以不存在**（没有仓库 / 没有分支名时真源整项不渲染，研究 §5），
///    所以调用方给 `AnyElement`（缺省时是一块空 `div`）。
///
/// ⚠️ **项目名不再由本函数画**：Windows 的项目名就在项目下拉的触发器里
/// （`title-project-menu.tsx:157`），所以本函数收的是**已经带名字的触发器元素**，
/// 拖拽区因此只是一块空的剩余宽度（真源 `ChromeGroup grow min-w-0`，`title-bar.tsx:339-343`）。
///
/// ⚠️ **落位（两行，2026-09-27 起）**：
///
/// - `menu_bar` → **第一行**（30，[`TITLE_BAR_MENU_ROW_HEIGHT_SPEC`]）的左侧，
///   右边接该行的拖拽区；
/// - `project_menu` → `branch_item` → **第二行**（38，[`TITLE_BAR_MAIN_ROW_HEIGHT_SPEC`]）
///   的左侧，顺序不可调换（真源 `ChromeGroup { menuItem, projectControls }` + 同组的分支项，
///   `title-bar.tsx:236-238,339-343`），之后是该行的拖拽区；
/// - 第二行拖拽区的右边依次是 [`run_controls`]（本轮新增的运行控件）、[`account_slot`]
///   （账号 / 设置槽位，只留空隙）与 [`window_controls`]（窗口三键）。
///
/// ⚠️ **上述可点元素全都是 `drag_region` 的兄弟节点，不能放进它内部**：Windows 的命中测试取
/// `window_control_hitboxes` 里**第一个**命中项（`gpui-pre-0.3.6/src/window.rs:1952-1956`，
/// 按绘制顺序 = 祖先在前），祖先的 `Drag` 会赢，那时点它们只会拖窗口
/// （"点菜单只拖窗口"那个真 bug 的成因）。本条与本文件窗口三键的 `Drag` 祖先坑同源（见下）。
pub fn title_bar(
    menu_bar: AnyElement,
    project_menu: AnyElement,
    branch_item: AnyElement,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    v_flex()
        // ChromeBar 基线带 `shrink-0`（`windows/tauri/src/ui/chrome.tsx:6`），标题栏不参与压缩。
        .flex_shrink_0()
        .w_full()
        // `bg-surface`（`title-bar.tsx:337`）：gpui-kit 的 `ThemeColor` **没有** `surface`
        // 这一项，语义上最贴近「标题栏专用底色」的是主题自己的 `title_bar`
        // （`gpui-component-0.6.6/src/theme/theme_color.rs:293`）。
        // 底色 / 字色 / 字号写在**两行的公共父节点**上而不是每行各写一遍：两行同底同色是效果图
        // 的定稿外观，写两处迟早漂移。
        .bg(cx.theme().title_bar)
        .text_sm()
        // `text-muted-foreground`（`title-bar.tsx:337`）→ `theme.muted_foreground`
        // （`gpui-component-0.6.6/src/theme/theme_color.rs:195`）。
        .text_color(cx.theme().muted_foreground)
        // ① 第一行 30：菜单栏 + 拖拽区（效果图 §1.3：第一行只有菜单）。
        //    ⚠️ 菜单栏是 `drag_region` 的**兄弟**：它挂在行容器上，不在 `drag_region` 里面。
        //    常驻形态占这一行左侧一段（真源 `title-bar.tsx:230-232`），
        //    图标形态只占一个 24×24 按钮 + 一层浮动胶囊。
        .child(
            h_flex()
                .w_full()
                .h(rems(TITLE_BAR_MENU_ROW_HEIGHT_SPEC / 16.))
                .gap_1()
                .px_2()
                .child(menu_bar)
                // 这一行剩下的宽度全是拖拽区（每行一个，见 [`drag_region`]）。
                .child(drag_region()),
        )
        // ② 第二行 38：左组（项目下拉 → 分支项）｜拖拽区｜右组（运行控件 → 账号槽位 → 三键）。
        .child(
            h_flex()
                .w_full()
                .h(rems(TITLE_BAR_MAIN_ROW_HEIGHT_SPEC / 16.))
                .gap_1()
                .px_2()
                // 项目下拉触发器：`drag_region` 的**兄弟**（见 [`crate::project_menu`] 模块头）。
                .child(project_menu)
                // 分支项：仍是 `drag_region` 的**兄弟**（见 [`crate::branch_panel`] 模块头）。
                // 真源里它与项目下拉同在 `ChromeGroup gap="tight"`（2px）内、紧跟其后
                // （`title-bar.tsx:236-238`）；没有仓库时调用方传一块空 `div`，真源同样整项不渲染。
                .child(branch_item)
                // 拖拽区独占剩余宽度（`flex_1`）—— 所以两行都**不需要** `justify_between()`：
                // 左右两组各自按内容宽度贴在两端，中间全归拖拽区（真源 `ChromeGroup grow`）。
                .child(drag_region())
                // 运行控件（`运行配置 ▾ ▶ 🐞`）：本轮新增，见 [`run_controls`]。
                .child(run_controls(cx))
                // 「账号 / 设置」槽位：只留空隙，见 [`account_slot`]。
                .child(account_slot())
                .child(window_controls(window, cx)),
        )
}

/// 拖拽区：**某一行**里除该行其它子元素之外的**剩余宽度**（两行各调一次）。
///
/// 由左组独占剩余宽度（`ChromeGroup grow min-w-0`，`title-bar.tsx:339`），该行其它元素
/// （第一行的菜单栏，第二行的项目下拉 / 分支项 / 运行控件 / 账号槽位 / 三键）都是它的
/// **兄弟节点**而不是子节点 —— 这一点必须保持：平台命中测试取
/// `window_control_hitboxes` 里**第一个**命中项（`gpui-pre-0.3.6/src/window.rs:1952-1956`，
/// 按绘制顺序 = 祖先在前），若把 `Drag` 挂在整条标题栏（或整个行容器）的祖先上，
/// 它会盖住子节点的 `Min` / `Max` / `Close`，三键就永远点不动了
/// （gpui-kit 自己也是把 `Drag` 挂在三键的兄弟 `h_flex` 上：
/// `gpui-component-0.6.6/src/title_bar.rs:371-379`）。
///
/// ⚠️ **不再画项目名**：项目名归项目下拉的触发器（`title-project-menu.tsx:157`），
/// 本区域只提供可拖动的空白（真源同一层的 `grow` 部分本来就是空白）。
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

/// 右侧窗口三键组：`ChromeGroup gap="none" h-(--lithe-title-bar-height)`
/// （`window-controls.tsx:52-55`）—— 三键之间 0 间距、整组 `h_full()` 撑满第二行。
///
/// ⚠️ 高度因此是**第二行的 38**（[`TITLE_BAR_MAIN_ROW_HEIGHT_SPEC`]），不再是真源的 40：
/// 标题栏改两行后三键跟着第二行走，见那个常量的文档。
fn window_controls(window: &Window, cx: &App) -> impl IntoElement {
    h_flex()
        .flex_shrink_0()
        .h_full()
        .gap_0()
        .child(window_control(WindowControl::Minimize, cx))
        .child(window_control(
            // 最大化/还原二选一：`window-controls.tsx:73-78`。
            if window.is_maximized() {
                WindowControl::Restore
            } else {
                WindowControl::Maximize
            },
            cx,
        ))
        .child(window_control(WindowControl::Close, cx))
}

/// 单个窗口控件：宽 56 × **第二行高 38**、直角、无点击回调（见文件头说明），只有悬停态。
///
/// ⚠️ 真源是 56×40（`window-controls.tsx:51-96` 的 `h-full` 撑标题栏那唯一一行）；
/// 这里 `h_full()` 撑的是**第二行**，所以高 38 —— 见 [`TITLE_BAR_MAIN_ROW_HEIGHT_SPEC`]。
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

    /// 字形取自全量 Lucide 目录 `gpui_kit::assets::IconName`：`window-close` / `window-maximize`
    /// / `window-minimize` / `window-restore` 四个 svg **都在**
    /// `gpui-kit-assets-0.6.6/assets/icons/` 里，所以这四个变体在全量枚举中确实存在、无需替代。
    ///
    /// Windows 源码用的是 lucide 的 `Minus` / `Square` / `CopyIcon as Restore` / `X`
    /// （`window-controls.tsx:2-7,66,78,93`）。那四个在目录里也都存在（`icons/minus.svg`、
    /// `icons/square.svg`、`icons/copy.svg`、`icons/x.svg`），但 `window-*` 这一组正是 lucide
    /// 为窗口三键提供的等价字形，与 gpui-kit 自带三键的选择一致
    /// （`gpui-component-0.6.6/src/title_bar.rs:148-151`），所以保持现状。
    fn icon(self) -> IconName {
        match self {
            Self::Minimize => IconName::WindowMinimize,
            Self::Maximize => IconName::WindowMaximize,
            Self::Restore => IconName::WindowRestore,
            Self::Close => IconName::WindowClose,
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
// 第二行右侧的运行控件（2026-09-27 新增：`运行配置 ▾ ▶ 🐞`）
// ---------------------------------------------------------------------------
//
// ⚠️ **本节三个元素都是 `drag_region` 的兄弟**（[`title_bar`] 把它们挂在第二行容器上），
// 不是它的后代 —— 原因见本文件模块头：`Drag` 做了可点元素的祖先，Windows 命中测试里
// 祖先先命中，点按钮只会拖窗口。

/// 标题栏第二行右侧的运行控件组：`运行配置 ▾` ＋ `▶` ＋ `🐞`（效果图 §1.3 的定稿形状）。
///
/// 真源 Windows 标题栏**没有**这一组（它把运行 / 调试挂在编辑区标签栏右侧，
/// `lithe-gpui/docs/ui-mockup-idea.md` §1.2 的图注 A1「后续把启动、debug 放这里」正是这一轮要做的搬迁），
/// 所以这里没有 `title-bar.tsx` 的行号可引；形状与顺序取自效果图 §1.3。
fn run_controls(cx: &App) -> impl IntoElement {
    h_flex()
        .flex_shrink_0()
        .h_full()
        // 组内 4（`--lithe-chrome-gap`，`styles/theme.css:130`）：与第二行其它子元素同一个间距。
        .gap_1()
        .child(run_configuration_slot(cx))
        .child(run_control(RunControl::Run, cx))
        .child(run_control(RunControl::Debug, cx))
}

/// 第二行右端的「账号 / 设置」槽位：**只留空隙、不画控件**。
///
/// 效果图 §1.3 的第二行右端在运行控件与窗口三键之间还有「账号 + 设置」（IDEA New UI 的
/// 账号头像与齿轮），本轮**不接**这两个入口：
///
/// - 「设置」在 Lithe 已有两个入口（左活动栏底部组的「设置」项，`crate::workspace` 的
///   `SETTINGS_ACTIVITY_IX`，以及 `Ctrl+,` 的 `MenuAction::Preferences`），再画一个第三入口
///   只会多一处状态；
/// - 「账号」在本侧没有任何数据源（没有登录 / 同步子系统）。
///
/// 所以这里只保留那一段**固定宽度的空隙**（24 = `w_6()`，与运行控件按钮同宽），
/// 让运行控件与窗口三键之间的留白与效果图一致 —— **不画一个点了没反应的按钮**
/// （《编码指南》的占位项纪律：不许假装能用）。
fn account_slot() -> impl IntoElement {
    div().flex_shrink_0().w_6()
}

/// `运行配置 ▾`：第二行右端的文字槽位（效果图 §1.3）。
///
/// ## 文案
///
/// 用 `lithe.run.configurations`（zh「运行配置」/ en "Run configurations"，
/// `lithe-gpui/crates/shared/locales/lithe.zh-CN.yml:3078` 与 `lithe.en.yml:3078` 同一行号）。
/// 真源那一段显示的是**当前运行配置的名字**（`run.run` 的键面就是 "Run configuration"），
/// 但本侧**没有运行配置子系统** —— 运行菜单三条（开始调试 / 停止 / 切换断点）全是占位项
/// （`crate::menu_bar` 的 `MENUS`，`menu_bar.rs:1343-1365`），拿不到任何配置名，
/// 所以这里显示这件东西的**类名**本身，不编一个假配置名。
///
/// ## ⚠️ 为什么它**不挂点击**
///
/// 没有可下拉的东西：本侧没有任何"运行配置"清单（对比项目下拉走 `Entity<ProjectMenu>`、
/// 菜单栏走 `Entity<MenuBar>`，两者都有真实数据与浮层实现）。画成可点却什么都不发生，
/// 就是《编码指南》禁止的"假装能用"；`▾` 只是效果图给的外形，等运行配置接上再补行为。
/// 它也因此**不需要** `id()`（`hover` / `active` 态要元素状态，`gpui-pre-0.3.6/src/elements/div.rs:2844-2849`，
/// 而这里两者都没有）。
fn run_configuration_slot(cx: &App) -> impl IntoElement {
    h_flex()
        .flex_shrink_0()
        .h_6()
        .px_2()
        .gap_1()
        .rounded(rems(TITLE_BAR_CONTROL_RADIUS_SPEC / 16.))
        // 与项目下拉 / 分支项触发器同一个未悬停前景（ghost 的 `text-subtle-foreground`
        // 在 gpui-kit 里与 `muted_foreground` 合并，见 [`window_control`] 的同一处说明）。
        .text_color(cx.theme().muted_foreground)
        .child(tr("lithe.run.configurations"))
        // caret 14×14：与项目下拉触发器同一个字形与尺寸
        // （`ui-mockup-idea.md` §1.3 的 `运行配置 ▾`；`title-project-menu.tsx:157-163` 的 `ChevronDownIcon`）。
        .child(Icon::new(IconName::ChevronDown).small())
}

/// 一个运行控件按钮（`▶` / `🐞`）：24×24 的 ghost 按钮。
///
/// 视觉照标题栏里既有的两个触发器（`crate::project_menu::trigger` /
/// `crate::branch_panel::trigger`）：未悬停 `text-muted-foreground`、悬停 `bg-accent` +
/// `text-foreground`（真源 ghost 按钮的 `hover:bg-accent`，`windows/tauri/src/ui/button.tsx:17,19`）。
/// 尺寸 24 = `icon-xs`（`windows/tauri/src/ui/button.tsx:27`），与右工具窗的关闭按钮、
/// 终端页签的关闭按钮同档（`right_tool_window.rs` / `terminal/src/constants.rs` 的同一处对照）。
fn run_control(control: RunControl, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    // 先按值取出来再进闭包：`cx.theme()` 借 `cx`，而 `.hover(..)` 的闭包要 `'static`。
    let idle = theme.muted_foreground;
    let hover_bg = theme.accent;
    let hover_fg = theme.foreground;

    div()
        // 必须有 `id`：`hover` 态要元素状态（`gpui-pre-0.3.6/src/elements/div.rs:2844-2849`）。
        .id(control.id())
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size_6()
        .rounded(rems(TITLE_BAR_CONTROL_RADIUS_SPEC / 16.))
        .text_color(idle)
        .cursor_pointer()
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        // 与项目下拉 / 分支项触发器逐字相同的第二道保险：本元素此刻**不在** `Drag` 命中区里
        // （它是 `drag_region` 的兄弟），这两行挡的是"将来有人把它挪进拖拽区"这类回归。
        // 它不是兄弟关系的替代品 —— 真正的约束是布局，见本文件模块头。
        .on_mouse_down(MouseButton::Left, |_event, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        // 点击只**入队**，真正的执行在下一帧的 `ShellWorkspace::render` 里
        // （见 [`RunControl::dispatch`]）。
        .on_click(move |_event, _window, cx| control.dispatch(cx))
        // 无障碍名 = 它复用的那一条菜单文案（图标按钮没有可见文字，必须给名字）。
        .aria_label(tr(control.label_key()))
        // 图标 14×14：`Sizable::small()` → `size_3p5()`（与窗口三键同一处换算）。
        .child(Icon::new(control.icon()).small())
}

/// 第二行右侧的两个运行控件。
///
/// 只 derive `Clone, Copy`（与 [`WindowControl`] 同一口径）：本枚举只用来**选一行**分支，
/// 不参与比较，多 derive 一个 `PartialEq` 只会让"将来有人拿它当状态比较"看起来可行。
#[derive(Clone, Copy)]
enum RunControl {
    /// `▶` 运行：复用「视图 → 运行和调试」。
    Run,
    /// `🐞` 调试：复用「运行 → 开始调试」那条**占位项**。
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

    /// 字形：**只用 `gpui_kit::assets::IconName` 里已有的变体，不新增图标资源**
    /// （`▶` = `IconName::Play`，`🐞` = `IconName::Bug`；两个字形在本仓库里都已用过：
    /// `menu_bar.rs:879` 的「运行和调试」与 `menu_bar.rs:1444` 的帮助菜单项）。
    ///
    /// ⚠️ 如实说明一处差别：菜单里「运行 → 开始调试」那一项的图标是 `IconName::Play`
    /// （`menu_bar.rs:1352`），而效果图把调试按钮画成 `🐞` —— 这里按效果图取 `IconName::Bug`。
    /// 图标不同、**行为完全相同**（同一条 `Missing` 声明，见 [`RunControl::dispatch`]）。
    fn icon(self) -> IconName {
        match self {
            Self::Run => IconName::Play,
            Self::Debug => IconName::Bug,
        }
    }

    /// 这一项复用的是**菜单里的哪一条**（诊断行与无障碍名都用它）。
    fn label_key(self) -> &'static str {
        match self {
            // 「视图 → 运行和调试」：键从 `MenuAction` 现取，不在这里抄第二份字符串。
            Self::Run => MenuAction::ShowRunAndDebug.label_key(),
            // 「运行 → 开始调试」在菜单里是 `MenuItem::NotWired`，**没有** `MenuAction` 变体，
            // 所以只能写它的文案键（`crate::menu_bar` 的 `MENUS`，`menu_bar.rs:1350`）。
            // ⚠️ 与菜单里那条是**同一个字符串**、不是同一个来源：那边改了键，这里会画出键名
            // （rust-i18n 缺键时回显键名），是**可见**的失败而不是静默。
            // 这个键本身有测试守着解析得出（`menu_bar.rs:3093` 的 `assert_ne!(tr(label_key), label_key)`）。
            Self::Debug => "lithe.menu.startDebugging",
        }
    }

    /// 已接线的动作；`None` = 这一项在菜单里是**占位项**（那 [`RunControl::missing`] 才有值）。
    ///
    /// 两个方法故意写成互补的一对：`dispatch` 用 `match (action(), missing())` 分流，
    /// 于是"接线"与"缺能力"不可能各写一份判据而互相漂移。
    fn action(self) -> Option<MenuAction> {
        match self {
            Self::Run => Some(MenuAction::ShowRunAndDebug),
            Self::Debug => None,
        }
    }

    /// 占位项缺什么；已接线的是 `None`。
    fn missing(self) -> Option<Missing> {
        match self {
            Self::Run => None,
            // 与菜单里「开始调试」同一条能力声明：本侧没有调试进程宿主
            // （`menu_bar.rs:349-355` 的 `MISSING_DEBUGGER_HOST`），
            // 「Run 工具窗」目前也只是占位内容（`crate::workspace` 的 `BottomPaneKind::Run`）。
            Self::Debug => Some(MISSING_DEBUGGER_HOST),
        }
    }

    /// 点击落点：**与菜单项点击同一条队列、同一段执行代码**。
    ///
    /// 走 `crate::menu_bar` 的待执行队列（`MenuBar::push_run` / `MenuBar::push_not_wired`），
    /// 下一帧由 `ShellWorkspace::render` 的 `for request in take_pending_runs(cx)` 分流：
    ///
    /// - 已接线的（`▶`）→ `ShellWorkspace::apply_menu_action` 的
    ///   `MenuAction::ShowRunAndDebug` 分支：把底部工具窗切到「运行」页签并显示
    ///   （`workspace.rs:1928-1943`；⚠️ 该页签**内容**目前仍是"未实现"占位）；
    /// - 占位项（`🐞`）→ `ShellWorkspace::report_menu_not_wired`：状态栏"尚未接入：缺 X" ＋
    ///   `S1_MENU notWired id=… missing=debugger_host` 诊断行（`workspace.rs:2747-2757`），
    ///   与点菜单里「运行 → 开始调试」**逐字相同**。
    ///
    /// `Ctrl+O` 的 `install_open_project_action` 与菜单键位（`crate::menu_bar::push_menu_key`）
    /// 走的也是这条链，所以"点标题栏的 ▶"与"点菜单里的运行和调试"不可能表现出两种行为。
    fn dispatch(self, cx: &mut App) {
        let label_key = self.label_key();
        // 菜单栏句柄没登记（窗口还没建出来 / 正在关）就什么都不做，只留一行诊断：
        // `try_handle` 不 panic（与 `crate::menu_bar::push_menu_key` 同一条理由 —— `handle()`
        // 在没有窗口时会 expect 失败，而标题栏按钮可能跑在窗口拆除的过程中）。
        let Some(bar) = crate::menu_bar::try_handle() else {
            eprintln!("S1_TITLE_BAR_RUN id={label_key} state=no_menu_bar");
            return;
        };
        let _ = bar.update(cx, |bar, cx| match (self.action(), self.missing()) {
            (Some(action), None) => bar.push_run(action, cx),
            (None, Some(missing)) => bar.push_not_wired(label_key, missing, cx),
            // 上面两个方法是一对互补取值，这一支不可达；写出来只是把匹配穷尽 ——
            // 真出现了（有人只改了一边）就**哪个都不做**，而不是猜一个方向执行。
            (Some(_), Some(_)) | (None, None) => {}
        });
        eprintln!("S1_TITLE_BAR_RUN id={label_key} state=pushed");
    }
}
