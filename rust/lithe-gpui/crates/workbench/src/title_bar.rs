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
//!   ⚠️ **两处有意偏离真源**（2026-09-30 维护者口径）：
//!   1. **挂第一行、贴窗口右边缘**：真源只有一行标题栏，三键就在那一行的右端；本侧第一行才是
//!      窗口最上面那一行，三键放第一行且右端与窗口边缘齐平，才与 Windows / IDEA 的手感一致；
//!   2. 宽度按新行高重算：真源那条 56 配的是 40 高（比例 1.4），第一行是 30 高 → 同比例得 42，
//!      取最近的 4px 档位 **44**（[`WINDOW_CONTROL_WIDTH_SPEC`]）。
//!   图标尺寸 `windows/tauri/src/ui/button.tsx:9` 的 `[&_svg:not([class*='size-'])]:size-3.5` = **14×14**。
//! - 高度 / 间距 / 内边距 / 字号令牌：`windows/tauri/src/styles/theme.css:115,118,126-133`。
//! - 中文文案：`windows/tauri/src/i18n/locale.ts:6191`（`titleProject.trigger` = `项目：{project}`，
//!   本区域按可见规格只显示项目名本身，见 `title_bar` 的文档）。
//!
//! ## 两行的排布（2026-09-27 效果图口径）
//!
//! ```text
//! 第一行 30：菜单栏（文件 编辑 视图 转到 终端 Git 工具 窗口 帮助）｜……拖拽区……｜ 窗口三键
//! 第二行 38：项目下拉 ▾ ｜ 分支项 ｜…………拖拽区…………｜ 运行配置 ▾ ▶ 🐞 ｜ 账号槽位
//! ```
//!
//! **第一行是菜单栏 + 右端的窗口三键**（效果图 §1.3 的定稿表格只写菜单栏；三键上移到这一行、
//! 并贴住窗口右边缘是 2026-09-30 维护者口径，理由见上面的窗口三键条目）。真源把菜单栏画在
//! **标题栏同一行里**：
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
//! （[`account_slot`]），顺序照效果图 §1.3；窗口三键（[`window_controls`]）**不在这一行**，
//! 它在第一行的右端（见上面的两行排布）。
//!
//! ⚠️ **两行里除拖拽区自己以外的可点元素都必须是 `drag_region` 的兄弟节点**：Windows 的
//! 命中测试取 `window_control_hitboxes` 里第一个命中项（`gpui-pre-0.3.6/src/window.rs:1952-1956`，
//! 按绘制顺序 = 祖先在前），祖先的 `Drag` 会赢 —— 把它们放进 `drag_region` 里面，
//! 点它们只会拖窗口。这条与下面窗口三键的坑同源：**`Drag` 不能做任何可点元素的祖先**。
//! 两行结构下的落法：**每一行各有一个 `drag_region()`**（各占"这一行剩下的宽度"），
//! 菜单栏与窗口三键挂在第一行容器上，项目下拉 / 分支项 / 运行控件 / 账号槽位挂在第二行
//! 容器上 —— 它们都是**行容器的儿子**，`drag_region` 只是它们的兄弟。
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
// rem base = 主题字号 16px（`lithe-gpui/themes/*.json` 的 `font.size` + `root.rs:582` 的
// `set_rem_size`），所以 helper 后缀 `N` = `N × 4px`。**内边距 / 间距 / 字号**仍与 Windows
// 规格逐像素相等；**行高**是 2026-09-27 效果图口径（30 + 38），不在 4px 档位上，见常量：
//
// | 规格（Windows 真源 / 效果图） | 值 | 用到的 helper |
// | --- | --- | --- |
// | 第一行高（效果图 §1.3「标题栏：两行」，第一行是菜单 + 右端窗口三键） | 30 | `rems(TITLE_BAR_MENU_ROW_HEIGHT_SPEC / 16.)` |
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

/// 单个窗口控件的宽度 **44**。
///
/// ## 为什么不是真源的 56
///
/// 真源是 `w-14` = 56（`windows/tauri/src/features/window/components/title-bar/window-controls.tsx:59,71,85`），
/// 但它那里配的是**单行 40 高**的标题栏，比例 56 : 40 = **1.4**。本侧三键 2026-09-30 起挂在
/// **30 高**的第一行（见模块头的窗口三键条目），按同一比例应得 42 —— 42 不在 4px 档位上
/// （`_10()`=40、`_11()`=44），取最近的 **44**，比例 1.47 与真源的 1.4 基本持平。
///
/// ## 为什么仍然写成 `rems(C / 16.)`
///
/// 44 虽然在档位上（`w_11()`），但这里与同组其它度量（行高 30 / 38）保持同一种写法：
/// 档位外的值只能这样写，档位内的值这样写也不会错，且不随 gpui 档位表变化 —— 见《编码指南》
/// 「档位外的值用 helper 底层的 `rems(P / 16.)`」。默认 16px 基准下 `rems(44. / 16.)` = 44。
const WINDOW_CONTROL_WIDTH_SPEC: f32 = 44.;

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

/// 标题栏**第二行**（项目下拉 / 分支项 / 拖拽区 / 运行控件 / 账号槽位）的高度 38。
///
/// ## 为什么是 38（以及两行为什么必须成对看）
///
/// 出处同 [`TITLE_BAR_MENU_ROW_HEIGHT_SPEC`]（效果图 §1.3 的第二行）。取 38 的直接后果是
/// **两行合计 68**，比上一版单行 40 **净增 28px** —— 这 28px 是这一轮唯一新增的纵向占用，
/// 所以底部工具窗同步从 320 收到 240（`crate::workspace` 里的 `BOTTOM_PANE_HEIGHT_SPEC`），
/// 保证"窗口总高不变、项目树底部不被挤出窗口"。
/// 两处是**一对**：只改这里不改底部工具窗，中央列就会把内容顶出窗口。
///
/// ⚠️ 窗口三键**不在这一行**：2026-09-30 起它们挂第一行（30 高），宽度随之重算为 44 ——
/// 见 [`WINDOW_CONTROL_WIDTH_SPEC`] 与模块头的窗口三键条目。这一行的高度只影响
/// 项目下拉 / 分支项 / 运行控件 / 账号槽位的可用高度。
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

/// 运行配置槽位的最大宽度 180。
///
/// 真源那一段按内容宽度铺开、只受标题栏余量限制；本侧给它一个上界，免得一个超长配置名
/// （Maven 的 `artifactId` 可以很长）把第二行的右组顶出窗口。180 是"看得见一截、截断处有省略号"
/// 的量级（一行约 20 个中文字符），`truncate()` 负责省略号。
const TITLE_BAR_RUN_SLOT_MAX_WIDTH_SPEC: f32 = 180.;

/// 标题栏（无状态）：**两行**的高度与排布、第二行右侧的运行控件、第一行右端的自绘窗口三键
/// 都在这里，不持有 `Entity`。只依赖传入的**已渲染好的**菜单栏、项目下拉与分支项、当前主题与窗口状态。
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
///   右边接该行的拖拽区，该行右端是 [`window_controls`]（窗口三键，贴窗口右边缘）；
/// - `project_menu` → `branch_item` → **第二行**（38，[`TITLE_BAR_MAIN_ROW_HEIGHT_SPEC`]）
///   的左侧，顺序不可调换（真源 `ChromeGroup { menuItem, projectControls }` + 同组的分支项，
///   `title-bar.tsx:236-238,339-343`），之后是该行的拖拽区；
/// - 第二行拖拽区的右边依次是 [`run_controls`]（本轮新增的运行控件）与 [`account_slot`]
///   （账号 / 设置槽位，只留空隙）。
///
/// ⚠️ **上述可点元素全都是 `drag_region` 的兄弟节点，不能放进它内部**：Windows 的命中测试取
/// `window_control_hitboxes` 里**第一个**命中项（`gpui-pre-0.3.6/src/window.rs:1952-1956`，
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
        // ① 第一行 30：菜单栏 + 拖拽区 + 右端窗口三键（效果图 §1.3 只写菜单栏；
        //    三键上移是 2026-09-30 维护者口径，理由见模块头的窗口三键条目）。
        //    ⚠️ 菜单栏是 `drag_region` 的**兄弟**：它挂在行容器上，不在 `drag_region` 里面。
        //    常驻形态占这一行左侧一段（真源 `title-bar.tsx:230-232`），
        //    图标形态只占一个 24×24 按钮 + 一层浮动胶囊。
        //    ⚠️ 三键是**行容器的儿子**、不在左段里面：本行的 8px 水平内衬只加在左段上，
        //    三键才能像 Windows / IDEA 那样贴住窗口右上角（hover 底色一路铺到边缘）。
        .child(
            h_flex()
                .w_full()
                .h(rems(TITLE_BAR_MENU_ROW_HEIGHT_SPEC / 16.))
                .child(
                    h_flex()
                        // 左段独占剩余宽度：`flex_1` 撑开，三键因此被推到最右端。
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .gap_1()
                        .px_2()
                        .child(menu_bar)
                        // 这一行剩下的宽度全是拖拽区（每行一个，见 [`drag_region`]）。
                        .child(drag_region()),
                )
                .child(window_controls(window, cx)),
        )
        // ② 第二行 38：左组（项目下拉 → 分支项）｜拖拽区｜右组（运行控件 → 账号槽位）。
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
                .child(run_controls(run, cx))
                // 「账号 / 设置」槽位：只留空隙，见 [`account_slot`]。
                .child(account_slot()),
        )
}

/// 拖拽区：**某一行**里除该行其它子元素之外的**剩余宽度**（两行各调一次）。
///
/// 由左组独占剩余宽度（`ChromeGroup grow min-w-0`，`title-bar.tsx:339`），该行其它元素
/// （第一行的菜单栏与窗口三键、第二行的项目下拉 / 分支项 / 运行控件 / 账号槽位）都是它的
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

/// 窗口三键组：`ChromeGroup gap="none" h-(--lithe-title-bar-height)`
/// （`window-controls.tsx:52-55`）—— 三键之间 0 间距、整组 `h_full()` 撑满**第一行**。
///
/// ⚠️ 高度因此是**第一行的 30**（[`TITLE_BAR_MENU_ROW_HEIGHT_SPEC`]），不是真源的 40：
/// 2026-09-30 起三键挂第一行（维护者口径），见模块头的窗口三键条目。
/// 宽度 [`WINDOW_CONTROL_WIDTH_SPEC`] 随行高重算（44），整组贴住窗口右边缘 ——
/// 它挂在行容器上、不在那一行的左段里，所以行内衬 8 不会把它推离边缘。
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

/// 单个窗口控件：宽 [`WINDOW_CONTROL_WIDTH_SPEC`]（44）× **第一行高 30**、直角、无点击回调
/// （见文件头说明），只有悬停态。
///
/// ⚠️ 真源是 56×40（`window-controls.tsx:51-96` 的 `h-full` 撑标题栏那唯一一行）；
/// 这里 `h_full()` 撑的是**第一行**，所以高 30、宽按同一比例收到 44。宽度为什么不是 56
/// 写在那个常量的文档里。
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
fn run_controls(run: Entity<lithe_gpui_run::RunPane>, cx: &App) -> impl IntoElement {
    h_flex()
        .flex_shrink_0()
        .h_full()
        // 组内 4（`--lithe-chrome-gap`，`styles/theme.css:130`）：与第二行其它子元素同一个间距。
        .gap_1()
        .child(run_configuration_slot(run.clone(), cx))
        .child(run_control(RunControl::Run, run.clone(), cx))
        .child(run_control(RunControl::Debug, run, cx))
}

/// 第二行右端的「账号 / 设置」槽位：**只留空隙、不画控件**。
///
/// 效果图 §1.3 的第二行右端还有「账号 + 设置」（IDEA New UI 的
/// 账号头像与齿轮），本轮**不接**这两个入口：
///
/// - 「设置」在 Lithe 已有两个入口（左活动栏底部组的「设置」项，`crate::workspace` 的
///   `SETTINGS_ACTIVITY_IX`，以及 `Ctrl+,` 的 `MenuAction::Preferences`），再画一个第三入口
///   只会多一处状态；
/// - 「账号」在本侧没有任何数据源（没有登录 / 同步子系统）。
///
/// 所以这里只保留那一段**固定宽度的空隙**（24 = `w_6()`，与运行控件按钮同宽），
/// 让第二行右端的留白与效果图一致（窗口三键已上移到第一行，这一段现在是第二行右组的收尾）
/// —— **不画一个点了没反应的按钮**（《编码指南》的占位项纪律：不许假装能用）。
fn account_slot() -> impl IntoElement {
    div().flex_shrink_0().w_6()
}

/// `运行配置 ▾`：第二行右端的文字槽位（效果图 §1.3）。
///
/// ## 显示的是**真实**的当前配置名
///
/// 从运行面板实体读（`lithe_gpui_run::RunPane::selected`）：有配置时显示它的名字，
/// 一条都没有（未打开 Java 工程 / 还没识别出来）时退回工具窗自己的名字
/// `lithe.run.configurations`（zh「运行配置」）。**不编假配置名**。
///
/// ## 点它 = 打开运行工具窗（那里才是配置列表）
///
/// 真源这一格点开是一个下拉清单。本侧**不画假下拉**：列表在底部「运行」工具窗里
/// （`lithe_gpui_run::RunPane`，点一行即选中），所以这里的落点是把那个工具窗切过来并显示
/// —— 与「视图 → 运行和调试」走同一条 `MenuAction::ShowRunAndDebug`（同一个待执行队列）。
/// 等这一格真的有了弹出菜单组件（`Entity<ProjectMenu>` 那种），再把它换成内联下拉。
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
        .h_6()
        .px_2()
        .gap_1()
        .max_w(rems(TITLE_BAR_RUN_SLOT_MAX_WIDTH_SPEC / 16.))
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
        .child(div().min_w_0().truncate().child(label))
        .child(Icon::new(IconName::ChevronDown).small())
}

/// 一个运行控件按钮（`▶` / `🐞`）：24×24 的 ghost 按钮。
///
/// 尺寸与底色照标题栏里既有的两个触发器（`crate::project_menu::trigger` /
/// `crate::branch_panel::trigger`）：悬停 `bg-accent`
/// （真源 ghost 按钮的 `hover:bg-accent`，`windows/tauri/src/ui/button.tsx:17,19`）。
/// ⚠️ **前景色那一半不照抄**：图标是彩色 `img()`（见 [`RunControl::icon`]），不吃 `text_color`，
/// 所以这里没有"未悬停 muted、悬停 foreground"那对颜色 —— 反馈只有底色。
/// 尺寸 24 = `icon-xs`（`windows/tauri/src/ui/button.tsx:27`），与右工具窗的关闭按钮、
/// 终端页签的关闭按钮同档（`right_tool_window.rs` / `terminal/src/constants.rs` 的同一处对照）。
fn run_control(
    control: RunControl,
    run: Entity<lithe_gpui_run::RunPane>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    // 先按值取出来再进闭包：`cx.theme()` 借 `cx`，而 `.hover(..)` 的闭包要 `'static`。
    let hover_bg = theme.accent;

    div()
        // 必须有 `id`：`hover` 态要元素状态（`gpui-pre-0.3.6/src/elements/div.rs:2844-2849`）。
        .id(control.id())
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size_6()
        .rounded(rems(TITLE_BAR_CONTROL_RADIUS_SPEC / 16.))
        .cursor_pointer()
        // 悬停反馈**只有底色**：图标是彩色 `img()` 管线，不吃 `text_color`
        // （原来那两行 `text_color(idle)` / `text_color(hover_fg)` 因此删掉 —— 留着是死样式，
        // 还会让下一个人以为"图标会跟着变亮"；见 [`RunControl::icon`]）
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
        // 图标 14×14：彩色那条自带尺寸（见 [`RunControl::icon`]）。
        .child(control.icon(cx))
}

/// 第二行右侧的两个运行控件。
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
    /// ⚠️ **2026-09-30 维护者口径**：此前走的是 `idea_icon_mono`（单色，跟随 `text_color`）。
    /// 换成彩色之后：图标保留资产自带的颜色，**不再跟随 `text_color`** —— 这一项本来就有
    /// 悬停底色（`.hover(.. bg(hover_bg))`），反馈由底色承担，与活动栏轨道项同一条口径
    /// （见 `.agents/notes/implemented/feature/2026-09-30-colored-ui-icons.md`）。
    ///
    /// 尺寸 `rems(14. / 16.)` = 14：与原来 `.small()`（`size_3p5()`）逐像素相同，
    /// 也是本仓库对 `.small()` 的统一换算——彩色那条**自己带尺寸**，`.child(..)` 不会替它设。
    fn icon(self, cx: &App) -> AnyElement {
        match self {
            Self::Run => idea_icon_image(&idea::PLAY_ICON, cx, rems(14. / 16.)),
            Self::Debug => idea_icon_image(&idea::BUG_ICON, cx, rems(14. / 16.)),
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
