//! 主菜单栏（Windows 规格）。
//!
//! 规格真源（一手源码，逐条核对过行号）：`gpui/research/windows/10-menu-bar.md`。
//!
//! ## 真源是什么
//!
//! `windows/tauri/src/features/window/components/window-menu-bar.tsx`（579 行，单一文件定义
//! 全部菜单）：**9 个顶级菜单**（文件 / 编辑 / 视图 / 转到 / 终端 / 运行 / 工具 / 窗口 / 帮助，
//! `:133,200,277,365,411,427,440,462,504`），**89 条项**（88 动作 + 1 个二级子菜单容器）
//! 与 21 条分隔线。真源用 `@base-ui/react` 的 `Menubar` **纯自绘**，`src-tauri` 全部
//! `.rs` 里 `menu` 零命中 —— 所以这一件本来就没有原生分支。
//!
//! ⚠️ 需求里给的那 12 项英文菜单（Navigate / Code / Refactor / Build / VCS …）是 **IntelliJ IDEA
//! 的菜单**，在 Lithe 真源里没有任何对应物（`locale.ts:7724-7732` 只有 9 条顶级键）。
//! 维护者已拍板：按 Lithe 真源的 9 项做，IDEA 那张只当视觉参考。
//!
//! ## 两种形态共用一份静态表
//!
//! | | 形态 A：左上角图标下拉 | 形态 B：常驻 |
//! | --- | --- | --- |
//! | 真源开关 | `compactMenuBar === true`（**真源默认值**） | `compactMenuBar === false` |
//! | 真源证据 | `title-bar.tsx:207-229`；`default-settings.ts:105` | `title-bar.tsx:230-232` |
//! | 本侧开关 | [`MenuBarMode::Compact`]（`--compact-menu-bar`） | [`MenuBarMode::Pinned`]，**本侧默认** |
//!
//! ⚠️ **本侧默认取"常驻"（维护者明确要求"固定在界面上"），与真源默认值相反**。
//! 两形态的**菜单数据是同一份**（[`MENUS`]），差别只有容器样式与定位：
//! 常驻是标题栏同一行内的 24px 胶囊（占其中左侧一段），图标形态是 `ListIcon` 按钮 +
//! 定位在标题栏正下方的浮动胶囊（真源 `absolute top-full left-0 mt-1`，
//! `window-menu-bar.tsx:539,548-549`）。
//!
//! ## 为什么自绘而不是 gpui-kit 的 `menu::AppMenuBar`
//!
//! 三条都有源码证据（`10-menu-bar.md` §4.2）：
//!
//! 1. `AppMenuBar` 的顶级项是 `Button::small()` = **24px** 硬编码、栏内 `gap_x_1()` = 4
//!    （`gpui-component-0.6.6/src/menu/app_menu_bar.rs:129,265-271`），而真源是
//!    **项 20 / gap 2**（`ui/menubar.tsx:36,86`）—— 那两个值都在它的 `render` 里写死，
//!    **没有任何覆盖点**；
//! 2. 它喂下拉项的路径 `with_menu_items` **没有 icon 参数**（`popup_menu.rs:802-807`）；
//! 3. `menu::NativeMenu` 在 Windows 上走 `TrackPopupMenuEx`，全文件**没有 `SetMenu`**
//!    （`native_menu/windows.rs:25-27,114-120,204`）→ 不可能做常驻栏。
//!
//! 所以这里只复用 [`gpui_kit::component::menu::PopupMenu`]（下拉面板 / 子菜单 / 分隔线 /
//! 键盘导航 / 快捷键提示全部现成），容器与顶级项自绘。
//!
//! ## v1 只列**真能执行**的项（维护者口径）
//!
//! 真源 89 条里，按 gpui 侧能力分档：**5 条已有实现**、**约 20 条有底层能力缺动作**、
//! **约 64 条无能力**（`10-menu-bar.md` §2.11）。本侧**只把真的改到状态的那些放进来**
//! —— 不摆"点了一片灰"的菜单，理由有两条：本仓库一贯"不放死控件"，以及《设计指南》的
//! "还能做得更少吗"。没能力的那 ~64 条**逐条登记**在 `.artifacts/p7/NOTES.md`，
//! 随能力落地再加。
//!
//! ⚠️ 这条**有意偏离真源**：真源把那 89 条全画出来，只按 4 处后端能力开关禁用
//! （`window-menu-bar.tsx:558-567`），其余项一律可点。本侧反着做（少画、不画灰），
//! 所以**菜单结构与真源不可逐条比对** —— 这是维护者明确接受的口径，不是遗漏。
//!
//! ## B1 修的两条"收不掉"根因（各自独立，只修一条都还会坏）
//!
//! 1. **没接上游的"点外部关闭"出口**：`PopupMenu` 自己挂了
//!    `on_mouse_down_out` → `dismiss()` → `cx.emit(DismissEvent)`
//!    （`gpui-component-0.6.6/src/menu/popup_menu.rs:1475 → 1084-1108 → 1055-1057`，
//!    `EventEmitter<DismissEvent>` 在 `:1407`），而本模块**从头到尾没有订阅过**它 ——
//!    于是 `selected` 永不被清，面板下一帧照旧被画出来。现在订阅落在面板**真的被建出来**
//!    的那一帧（[`menu_item`]），处理函数是 [`MenuBar::handle_dismiss`]。
//!    顺带把焦点交给面板（`Esc` / `↑↓` / `Enter` 的 `key_context` 在它身上），
//!    收起时再安排"下一帧把焦点还给外壳根"（[`MenuBar::restore_focus`]）。
//! 2. **摆位写错：面板落在标题栏那一行、把自己的触发器盖住**。原来用
//!    `deferred(anchored().anchor(Anchor::BottomLeft)…)` —— `anchored()` 取的是锚点元素
//!    **自己在布局里的静态位置**，配合 `BottomLeft` 会把面板放到触发器**上方**；向上溢出
//!    窗口上沿后又被 `snap_to_window_with_margin(8)` 夹回 `y=8`，于是面板压回标题栏那一行。
//!    面板 `.occlude()` = `HitboxBehavior::BlockMouse`，被盖住的标题既拿不到 hover 也拿不到
//!    点击 —— 这正是"再点同一个标题收不掉"（那一下点击根本没进回调）的原因。
//!    现在统一走 [`popup_for`] 的 `Positioner::side(触发器 bounds)`（紧凑形态的浮动层
//!    同理，见 `compact_bar`）。
//!
//! 五条关闭路径全部汇合到 [`MenuBar::close`] 这一个出口，各自打一行
//! `S1_MENU_CLOSE reason=…`（取值域见 [`CloseReason`]）。
//!
//! ⚠️ **顶级的开合因此从 `on_click` 挪到了 `on_mouse_down`**：面板的
//! `on_mouse_down_out` 在**捕获阶段**就会把"按在触发器上"的这一下看成"点到了面板外"
//! （触发器对面板而言确实是外部），若开合还挂在 mouse-up 的 `on_click` 上，
//! 就会"先被面板收掉、又被 click 开回来"。现在 [`MenuBar::toggle`] 独占这层语义，
//! [`MenuBar::handle_dismiss`] 对"按在打开项自己的触发器上"的 dismiss 故意放过。
//!
//! ## 度量（真源 `windows/tauri/src/ui/menubar.tsx`）
//!
//! | 规格 | 值 | 出处 | 本侧写法 |
//! | --- | --- | --- | --- |
//! | 菜单栏本体高 | 24 | `ui/menubar.tsx:35-38` 的 `h-6` | `h_6()` |
//! | 本体圆角 / 边框 / 底色 | 全圆角、1px `border-border/70`、`bg-background/65` | 同上 | `rounded_full()` + `border_1()` + `background` |
//! | 本体内边距 / 项间距 | 2 / 2 | 同上 `px-0.5 py-0.5` / `gap-0.5` | `p_0p5()` + `gap_0p5()` |
//! | 顶级项高 | **20** | `ui/menubar.tsx:85-88` 的 `h-5` | `h_5()` |
//! | 顶级项内边距 / 圆角 | 6 / 6.4 | 同上 `px-1.5` / `rounded-md` | `px_1p5()` + [`ITEM_RADIUS`] |
//! | 下拉面板最小宽 / 圆角 | 240 / 11.2 | `ui/menubar.tsx:125-127` 的 `min-w-60` / `rounded-xl` | `rems(240./16.)` + [`PANEL_RADIUS`] |
//! | 浮动胶囊圆角 / 内边距 / 上间距 | 14.4 / 4 / 4 | `window-menu-bar.tsx:539,548-549` 的 `rounded-2xl px-1 py-1` / `mt-1` | [`FLOAT_RADIUS`] + `p_1()` + `mt_1()` |
//! | 字号 | 13（`--ui-text-sm`） | `ui/menubar.tsx:85` | `text_sm()`（14；13→14 是经维护者确认的有意改动，见 `crate::workspace` 模块头） |
//!
//! 圆角一律走应用层具名常量（Lithe 的圆角阶梯是 `--radius × k` = 4.8 / 6.4 / 11.2，
//! 不在 4px 网格上，也不能从 `ThemeConfig.radius` 读 —— 那是 `usize`，理由见
//! `crate::workspace` 的 `ISLAND_RADIUS`）。颜色一律 `cx.theme()`，不写裸色值。
//!
//! ## 键盘可达性（**如实说明，未做假**）
//!
//! | 能力 | 本侧 | 依据 |
//! | --- | --- | --- |
//! | `↑` / `↓` 选项、`←` / `→` 进子菜单与切换顶级、`Enter` 激活、`Esc` 关闭 | ✅ **现成** | `PopupMenu` 自己绑好这 6 个键（`popup_menu.rs:21-30`），本模块不用写 |
//! | 关闭后焦点归还 | ✅ **现成** | `PopupMenu::dismiss` 把焦点还给 `action_context`（`popup_menu.rs:1052-1072`），本模块把外壳的焦点句柄设进去 |
//! | **Alt 助记键直接开菜单** | ❌ **做不到** | gpui 的键盘模型是 `action + key_context`，**没有 mnemonic / 下划线助记符**的概念；`gpui_pre` 的 `Menu` / `MenuItem` 也没有 mnemonic 字段（`10-menu-bar.md` §4.6）。真机 Windows 前端的菜单项文案里同样没有 `&` 助记符 |
//! | **Tab 聚焦顶级项 → Enter 打开** | ❌ **未实现**（本轮明确不做，见下） | 需要顶级项是 focusable + 有 `Enter` 处理器；详见下面两段 |
//!
//! 最后一条的具体情况，写清楚免得被当成 bug：gpui 确实有 `tab_index(..)` 能让 `div` 进
//! Tab 序（`gpui-pre-0.3.6/src/elements/div.rs:809-814` 把 `focusable` 与 `tab_stop` 一起置位），
//! 也能用 `on_action` 接 `gpui_kit::component::actions::Confirm`（`Enter` 在 gpui-base 里就是它）。
//! **但** `Confirm` 不是全局绑定：`gpui-component` 只在 `PopupMenu` 自己的 `key_context`
//! 里登记它（`popup_menu.rs:22-23`），菜单栏这一层要用就得自己
//! `cx.bind_keys(KeyBinding::new("enter", Confirm { secondary: false }, Some("lithe-menu-bar")))`
//! —— 那是**应用级**注册，必须放在组合点（`ShellWorkspace::new`），而且要先确认
//! "focusable 的顶级项 + Enter"在开菜单那一刻不会被 `PopupMenu` 抢走焦点。
//! 本轮**没有做这一步**，所以"Tab 聚焦 + Enter 打开"这条链路**未实现、未验证**，
//! 如实登记在这里与 `.artifacts/p7/NOTES.md`。
//!
//! （`Ctrl+M` → 切换两种形态那条**已实现**：[`crate::workspace`] 的 `ToggleMenuBar`
//! action；它只需要一个 action，不需要 focusable 的顶级项。）

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui_kit::assets::IconName;
// `Align` / `Placement` / `Positioner` 住在 `gpui_kit::base`（与 `project_menu` 同一取法，
// 它们是布局基元而不是组件）；`ElementExt` 提供 `on_prepaint`（量触发器盒子要用它）。
use gpui_kit::base::{Align, ElementExt as _, Placement, Positioner, h_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};
use gpui_kit::{
    App, AppContext as _, Bounds, ClickEvent, Context, DismissEvent, Entity, FocusHandle,
    Focusable as _, InteractiveElement as _, IntoElement, Keystroke, MouseButton,
    ParentElement as _, Pixels, SharedString, StatefulInteractiveElement as _, Styled as _,
    Subscription, Window, deferred, div, px,
};

use lithe_gpui_shared::tr;

use crate::command_palette::CommandId;

// ---------------------------------------------------------------------------
// 圆角常量（档位外的值，逐条给理由）
// ---------------------------------------------------------------------------/// 顶级项圆角 6.4：`ui/menubar.tsx:86` 的 `rounded-md` = `--radius × 0.8`（`theme.css:7`）。
///
/// ⚠️ **保留 `px(...)`**：6.4 不是 gpui 的 rem 档位（`rounded_md()` 是 6），且 Lithe 的圆角阶梯
/// 一律走应用层具名常量 —— 与 `crate::right_tool_window` 的 `CLOSE_BUTTON_RADIUS` 同一条理由。
const ITEM_RADIUS: Pixels = px(6.4);

/// 浮动胶囊圆角 14.4：`window-menu-bar.tsx:549` 的 `rounded-2xl` = `--radius × 1.8`（`theme.css:10`）。
///
/// ⚠️ 同上：14.4 不是 gpui 档位。
const FLOAT_RADIUS: Pixels = px(14.4);

// ⚠️ 下拉面板自己的圆角（真源 `rounded-xl` = 11.2）**不在本文件里**：`PopupMenu` 的圆角是
// 组件内部的 `cx.theme().radius.min(px(8.))`（`popup_menu.rs:1460`，`RenderOptions` 的
// `radius`），没有任何公开 setter 能覆盖它。所以面板圆角由 gpui-kit 主题决定（默认 6），
// 这是本轮**已知且未解**的偏差 —— 硬要改只能改全局 `ThemeConfig.radius`，那会连带改掉
// 所有 gpui-kit 组件的圆角（`crate::workspace` 的 `ISLAND_RADIUS` 记过同一条）。

/// 下拉面板最小宽 240：`ui/menubar.tsx:126` 的 `min-w-60`（60 × 4 = 240）。
///
/// ⚠️ 240 不在 gpui 的固定档位（档位里 224 → `w_56()`、256 → `w_64()`），按《编码指南》
/// 这类值应写成 helper 底层的 `rems(P / 16.)`。**但这里写 `px(240.)`**：
/// `PopupMenu::min_w` 收的是 `impl Into<Pixels>`（`popup_menu.rs:429-432`），
/// 而 gpui **没有 `impl From<Rems> for Pixels`**（`geometry.rs:2909` 起的那一批
/// `impl From<_> for Pixels` 里没有它）—— 本轮实测 `rems(..)` 报 E0277。
/// 要写 rem 得走 `AbsoluteLength::to_pixels`（`crate::command_palette` 的 `rem_px` 就是那么做的），
/// 为一个菜单面板最小宽引入那层换算不划算；240 与 rem 基准 16px 的换算关系写在这里备查。
const PANEL_MIN_WIDTH: Pixels = px(240.);

/// 面板与触发器之间的空隙 4：真源 `Menu.Positioner` 的 `sideOffset={4}`
/// （`windows/tauri/src/ui/menubar.tsx:107`），与 `project_menu.rs` 的 `PANEL_OFFSET_SPEC`
/// 同一个数、同一条理由（`ui/dropdown.tsx:735-737` 的默认值）。
///
/// 4 在 gpui 档位上（`top_1()` = 4），但 `Positioner::offset` 收的是 `Pixels`，所以写 `px`。
const PANEL_OFFSET_SPEC: f32 = 4.;

/// 面板贴窗口边时留的边距 8：真源 `collisionPadding={8}`（`ui/menubar.tsx:108`）。
///
/// ⚠️ 它是**碰撞留白**，不是"面板离触发器 8" —— B1 之前这里把它当成后者的近似
/// （`snap_to_window_with_margin(px(8.))`），那正是面板被顶回标题栏那一行的最后一环。
const PANEL_WINDOW_MARGIN_SPEC: f32 = 8.;

/// 一个按键是不是"裸 `Esc`"（`Esc` 拦截器的判据，见 [`MenuBar::escape_interceptor`]）。
///
/// `modifiers` 必须为空：`Ctrl+Esc` / `Alt+Esc` 是系统级组合，不该被菜单吃掉。
fn is_escape(keystroke: &Keystroke) -> bool {
    keystroke.key == "escape" && keystroke.modifiers.number_of_modifiers() == 0
}

// ---------------------------------------------------------------------------
// 菜单数据：静态表（唯一真源）
// ---------------------------------------------------------------------------

/// 一条顶级菜单。
pub struct TopMenu {
    /// 诊断行 `S1_MENU_OPEN id=…` 的取值（真源顶级键去掉 `menu.` 前缀，可 grep）。
    pub id: &'static str,
    /// 文案键（真源 `windows/tauri/src/i18n/locale.ts` 的 `menu.<id>`，本侧前缀 `lithe.`）。
    pub label_key: &'static str,
    /// 菜单里的项与分隔线。
    pub items: &'static [MenuItem],
}

impl TopMenu {
    /// 顶级项的可见文案。
    pub fn label(&self) -> SharedString {
        tr(self.label_key)
    }
}

/// 菜单里的一项。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuItem {
    /// 可执行的项。
    Action(MenuAction),
    /// 分隔线（真源的 `<MenubarSeparator />`）。
    Separator,
    /// 二级子菜单：**主题列表**（真源唯一的动态项，见 `10-menu-bar.md` §3.1）。
    Theme,
}

/// v1 里**真的能执行**的菜单动作。
///
/// 每个变体都对应一条已存在的实现；走 [`CommandId`] 的那几条与命令面板**共用同一个执行点**
/// （`ShellWorkspace::run_command`），其余的执行分支与接线点写在
/// `crate::workspace::ShellWorkspace::run_menu_action` 的文档上。**没有一个占位分支**：
/// 「无能力的项」根本不在 [`MENUS`] 里（理由见模块文档）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuAction {
    /// 文件 → 打开文件…（**不绑键位**：`Ctrl+O` 留给「打开文件夹」，Q18）。
    OpenFile,
    /// 文件 → 保存（`mod+s`）。
    Save,
    /// 文件 → 关闭标签页（`Ctrl+W`，见 `lithe_gpui_editor::install_actions`）。
    CloseTab,
    /// 文件 → 关闭其他标签页。
    CloseOtherTabs,
    /// 文件 → 关闭所有标签页。
    CloseAllTabs,
    /// 文件 → 关闭已保存标签页。
    CloseSavedTabs,
    /// 文件 → 关闭左侧标签页。
    CloseTabsToLeft,
    /// 文件 → 关闭右侧标签页。
    CloseTabsToRight,
    /// 文件 → 重新打开已关闭标签页。
    ReopenClosedTab,
    /// 编辑 → 命令面板（`mod+shift+p`）。
    CommandPalette,
    /// 视图 → 显示/隐藏终端（真源 `mod+j`，本侧没有绑这个键）。
    ToggleTerminal,
    /// 视图 → 显示/隐藏 Maven（本侧既有能力，真源的视图菜单里没有这一项）。
    ToggleMaven,
    /// 视图 → 显示/隐藏状态栏（本侧既有能力，真源的视图菜单里没有这一项）。
    ToggleStatusBar,
    /// 视图 → 外观设置（打开设置对话框并停在外观页）。
    OpenAppearanceSettings,
    /// 转到 → 转到定义（`f12`）。
    GoToDefinition,
    /// 终端 → 新建终端。
    NewTerminalTab,
    /// 终端 → 关闭终端（关掉当前页签）。
    CloseTerminalTab,
    /// 工具 → 首选项（`Ctrl+,`）。
    Preferences,
    /// 窗口 → 最小化。
    Minimize,
    /// 窗口 → 最大化 / 还原。
    Maximize,
    /// 窗口 → 切换全屏。
    ToggleFullscreen,
}

impl MenuAction {
    /// 文案键（`lithe.menu.*`，全部是真源既有键，见 `10-menu-bar.md` §4.7）。
    ///
    /// 写成 `match` 而**不是**表里的字符串：拼错的键在这里能在 review 时一眼看出，
    /// 而进 `WIRED` 表的那些键由 `lithe_gpui_shared` 的
    /// `every_wired_key_resolves_in_both_locales` 逐条断言。
    pub const fn label_key(self) -> &'static str {
        match self {
            // ⚠️ 「打开文件…」**不是** `lithe.menu.*` 的键：真源的 89 条菜单里没有这一项
            // （Windows 靠快速打开 / 文件树取文件），`menu.*` 段没有 `openFile`。
            // 这里复用真源 `outline.openFile`（zh「打开文件」/ en "Open a File"）——
            // 同一个词、同一件事，且**零新增键**（不碰 `GPUI_ONLY_KEYS` 与两份 yml）。
            // 与「显示状态栏」复用 `settings.appearance.showStatusBar` 是同一条口径。
            Self::OpenFile => "lithe.outline.openFile",
            Self::Save => "lithe.menu.save",
            Self::CloseTab => "lithe.menu.closeTab",
            Self::CloseOtherTabs => "lithe.menu.closeOtherTabs",
            Self::CloseAllTabs => "lithe.menu.closeAllTabs",
            Self::CloseSavedTabs => "lithe.menu.closeSavedTabs",
            Self::CloseTabsToLeft => "lithe.menu.closeTabsToLeft",
            Self::CloseTabsToRight => "lithe.menu.closeTabsToRight",
            Self::ReopenClosedTab => "lithe.menu.reopenClosedTab",
            Self::CommandPalette => "lithe.menu.commandPalette",
            Self::ToggleTerminal => "lithe.menu.toggleTerminal",
            Self::ToggleMaven => "lithe.workbench.maven",
            // ⚠️ 状态栏这条**不是**真源键：真源的视图菜单里没有状态栏开关
            // （它是设置项 `settings.appearance.showStatusBar`），所以文案复用真源设置项的
            // 那一句「显示状态栏」（`locale.ts:6644`）—— 与设置界面、命令面板里的那两条
            // 同一个词，用户看到的是同一件事。**不开新键**。
            Self::ToggleStatusBar => "lithe.settings.appearance.showStatusBar",
            Self::OpenAppearanceSettings => "lithe.settings.tabs.appearance",
            Self::GoToDefinition => "lithe.menu.goToDefinition",
            Self::NewTerminalTab => "lithe.menu.newTerminal",
            Self::CloseTerminalTab => "lithe.menu.closeTerminal",
            Self::Preferences => "lithe.menu.preferences",
            Self::Minimize => "lithe.menu.minimize",
            Self::Maximize => "lithe.menu.maximize",
            Self::ToggleFullscreen => "lithe.menu.toggleFullscreen",
        }
    }

    /// 可见文案。
    pub fn label(self) -> SharedString {
        tr(self.label_key())
    }

    /// 左侧图标。
    ///
    /// 真源的菜单项**没有图标**（`ui/menubar.tsx:142-160` 的 `MenubarItem` 没有 icon 参数），
    /// 但 `PopupMenu` 支持，而且本侧命令面板已经给同一批动作配了图标
    /// （`crate::workspace::command_action` 的 `icon`）。**用同一批字形**既让"两处入口视觉同源"，
    /// 也让"这条到底能做什么"一眼可认 —— 这是**有意偏离真源**，
    /// 与"只列可执行项"同一性质，登记在 `.artifacts/p7/NOTES.md`。
    pub fn icon(self) -> IconName {
        match self {
            // 「打开文件」用「打开的文件夹」字形：真源菜单项没有图标，字形只是本侧
            // "两处入口视觉同源"的补充（见本方法的文档），所以取语义最近的那一个。
            Self::OpenFile => IconName::FolderOpen,
            Self::Save => IconName::Save,
            // 关闭系一律用「×」系字形：真机标签上的关闭就是 lucide `x`
            // （`windows/tauri/src/ui/tab-bar-item.tsx:150-180` 的关闭按钮）。
            Self::CloseTab => IconName::X,
            Self::CloseOtherTabs => IconName::Files,
            Self::CloseAllTabs => IconName::SquareX,
            // 「关闭已保存」用 `save-check`：这条的判据正是"这份 buffer 已落盘"。
            Self::CloseSavedTabs => IconName::SaveCheck,
            Self::CloseTabsToLeft => IconName::PanelLeft,
            Self::CloseTabsToRight => IconName::PanelRight,
            // 「重新打开已关闭标签页」= 回退一步，用与「重新加载」同一个回头箭头。
            Self::ReopenClosedTab => IconName::RotateCcw,
            Self::CommandPalette => IconName::Search,
            Self::ToggleTerminal | Self::NewTerminalTab | Self::CloseTerminalTab => {
                IconName::SquareTerminal
            }
            Self::ToggleStatusBar => IconName::PanelBottom,
            Self::ToggleMaven => IconName::Package,
            Self::OpenAppearanceSettings | Self::Preferences => IconName::Settings,
            // 跳转语义用「向右的箭头」：真源 `f12` 是"跳到定义处"。
            Self::GoToDefinition => IconName::ArrowRight,
            Self::Minimize => IconName::WindowMinimize,
            Self::Maximize => IconName::WindowMaximize,
            Self::ToggleFullscreen => IconName::Maximize,
        }
    }

    /// 能映射到 [`CommandId`] 的那几条：**执行逻辑复用命令面板那一条**，不在菜单里重写一遍。
    ///
    /// 这是"同一份能力只有一个执行点"的落点：菜单项与命令面板项都走
    /// `ShellWorkspace::run_command(id, window, cx)`。
    pub fn command_id(self) -> Option<CommandId> {
        match self {
            Self::OpenFile => Some(CommandId::OpenFile),
            Self::Save => Some(CommandId::SaveBuffer),
            Self::CloseTab => Some(CommandId::CloseTab),
            Self::CloseOtherTabs => Some(CommandId::CloseOtherTabs),
            Self::CloseAllTabs => Some(CommandId::CloseAllTabs),
            Self::CloseSavedTabs => Some(CommandId::CloseSavedTabs),
            Self::CloseTabsToLeft => Some(CommandId::CloseTabsToLeft),
            Self::CloseTabsToRight => Some(CommandId::CloseTabsToRight),
            Self::ReopenClosedTab => Some(CommandId::ReopenClosedTab),
            Self::CommandPalette => Some(CommandId::OpenCommandPalette),
            Self::ToggleTerminal => Some(CommandId::ToggleTerminal),
            Self::ToggleMaven => Some(CommandId::ToggleMaven),
            Self::ToggleStatusBar => Some(CommandId::ToggleStatusBar),
            Self::OpenAppearanceSettings => Some(CommandId::OpenAppearanceSettings),
            Self::GoToDefinition => Some(CommandId::NavigateToDefinition),
            Self::NewTerminalTab
            | Self::CloseTerminalTab
            | Self::Preferences
            | Self::Minimize
            | Self::Maximize
            | Self::ToggleFullscreen => None,
        }
    }
}

/// 9 个顶级菜单的静态表（**唯一真源**，两形态共用）。v1 只列真能执行的项。
///
/// 空菜单（运行 / 帮助）也**照画**：真源有这 9 个顶级项，少一个肉眼就能看出来；
/// 点开是一张空面板，而不是一片灰 —— 这是本侧口径下"没有能力"的最诚实表达。
/// 项数与"没能力因此未列入"的条数逐条记在 `.artifacts/p7/NOTES.md`。
///
/// ⚠️ "视图 → 外观设置" 与 "工具 → 首选项" 指向**同一个对话框**，但页面不同
/// （外观页 vs 常规页）：真源 `menu.preferences` 开的是常规页
/// （`settings-actions.tsx:155-163`），而外观页在真源里只有命令面板入口
/// （`:127-138` 的 `settingsTabCommands`）。这里把外观页放进视图菜单是**本侧的补齐**
/// （真源视图菜单里没有它）。
pub static MENUS: &[TopMenu] = &[
    TopMenu {
        id: "file",
        label_key: "lithe.menu.file",
        // 顺序照真源 19 条里本侧已实现的那些（`window-menu-bar.tsx:133-199`）：
        // 打开文件（本侧新增，占真源第 4 项「打开文件夹」的位置 —— 那一条要项目生命周期，
        // 属 B4）→ 保存（真源第 6）→ ── → 关闭系（真源第 11/13/14/15/16/17/18 项，
        // 顺序不变）。
        //
        // ⚠️ **「打开文件」有意不绑键位**（Q18）：真源把 `Ctrl+O` 给「打开文件夹」，
        // 本侧照抄，所以"打开文件"只有菜单入口。关闭系里只有「关闭标签页」有真键位
        // （`Ctrl+W`），但 B1 **不显示**它 —— 键位显示走 `.action(..)` 让上游解析，
        // 那是 B2 的事（Q13）。
        items: &[
            MenuItem::Action(MenuAction::OpenFile),
            MenuItem::Action(MenuAction::Save),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::CloseTab),
            MenuItem::Action(MenuAction::CloseOtherTabs),
            MenuItem::Action(MenuAction::CloseAllTabs),
            MenuItem::Action(MenuAction::CloseSavedTabs),
            MenuItem::Action(MenuAction::CloseTabsToLeft),
            MenuItem::Action(MenuAction::CloseTabsToRight),
            MenuItem::Separator,
            MenuItem::Action(MenuAction::ReopenClosedTab),
        ],
    },
    TopMenu {
        id: "edit",
        label_key: "lithe.menu.edit",
        items: &[MenuItem::Action(MenuAction::CommandPalette)],
    },
    TopMenu {
        id: "view",
        label_key: "lithe.menu.view",
        items: &[
            MenuItem::Action(MenuAction::ToggleTerminal),
            MenuItem::Action(MenuAction::ToggleStatusBar),
            MenuItem::Separator,
            // 真源的「主题」是二级子菜单，列出**注册表里全部主题**（动态项）。
            MenuItem::Theme,
        ],
    },
    TopMenu {
        id: "go",
        label_key: "lithe.menu.go",
        items: &[MenuItem::Action(MenuAction::GoToDefinition)],
    },
    TopMenu {
        id: "terminal",
        label_key: "lithe.menu.terminal",
        items: &[
            MenuItem::Action(MenuAction::NewTerminalTab),
            MenuItem::Action(MenuAction::CloseTerminalTab),
        ],
    },
    TopMenu {
        id: "run",
        label_key: "lithe.menu.run",
        items: &[],
    },
    TopMenu {
        id: "tools",
        label_key: "lithe.menu.tools",
        items: &[
            MenuItem::Action(MenuAction::Preferences),
            // ⚠️ 「外观设置」与「首选项」是**同一个对话框的两页**（常规 / 外观），不是两条死项：
            // 真源 `menu.preferences` 开常规页（`settings-actions.tsx:155-163`），外观页在真源里
            // 只有命令面板入口（`:127-138` 的 `settingsTabCommands`）。本侧把它放进工具菜单
            // 是**补齐**（真源视图菜单里没有它，见 `10-menu-bar.md` §5.1 B 档）。
            MenuItem::Action(MenuAction::OpenAppearanceSettings),
            MenuItem::Separator,
            // ⚠️ 「Maven」放在这里（**不在**真源的**视图**菜单里）：真源把 Maven 工具窗开在
            // **右侧栏**（`maven-tool-window-actions.ts:58` 的 `toggleMavenToolWindow`），
            // 而 Windows 的工具菜单本来就只有 4 项、其中「数据库」默认还是灰的
            // （`backend-capabilities.ts:5`）。本侧把"Maven 工具窗"归到「工具」下更贴语义。
            // 真源视图菜单里的第 9 项是「运行和调试」，本侧没有该能力。
            MenuItem::Action(MenuAction::ToggleMaven),
        ],
    },
    TopMenu {
        id: "window",
        label_key: "lithe.menu.window",
        items: &[
            MenuItem::Action(MenuAction::Minimize),
            MenuItem::Action(MenuAction::Maximize),
            MenuItem::Action(MenuAction::ToggleFullscreen),
        ],
    },
    TopMenu {
        id: "help",
        label_key: "lithe.menu.help",
        items: &[],
    },
];

/// v1 实际画出来的**可执行项**条数（诊断与文档都用它，避免两处各写一份数字）。
pub fn action_count() -> usize {
    MENUS
        .iter()
        .flat_map(|menu| menu.items.iter())
        .filter(|item| matches!(item, MenuItem::Action(_)))
        .count()
}

// ---------------------------------------------------------------------------
// 诊断（`S1_MENU_*`）
// ---------------------------------------------------------------------------

/// 菜单栏的两种形态：**画在哪里**。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuBarMode {
    /// 形态 B：与标题栏同一行的常驻胶囊。**本侧默认**（维护者要求"固定在界面上"）。
    Pinned,
    /// 形态 A：左上角 `ListIcon` 按钮 + 标题栏下方的浮动胶囊（真源默认值）。
    Compact,
}

impl MenuBarMode {
    /// 诊断行 `S1_MENU_BAR mode=…` 的取值（可 grep 的契约，改它就是改验证脚本）。
    pub const fn id(self) -> &'static str {
        match self {
            Self::Pinned => "pinned",
            Self::Compact => "compact",
        }
    }

    /// 构造期与切换形态时那一行诊断：形态 + 顶级项数 + 可执行项数。
    ///
    /// 为什么打这三个数：它们在界面上读不出来（要数像素），而"9 个顶级菜单 / n 条可执行项"
    /// 正是本任务最要紧的两个事实。
    pub fn diagnose(self) {
        eprintln!(
            "S1_MENU_BAR mode={} items={} actions={}",
            self.id(),
            MENUS.len(),
            action_count()
        );
    }
}

/// 打开某一项菜单时的诊断（`id` 取 [`TopMenu::id`]）。
pub fn diagnose_open(id: &str) {
    eprintln!("S1_MENU_OPEN id={id}");
}

/// 收起的**起因**（Q17 把下拉的收起统一到一个出口，`reason` 就是它的参数）。
///
/// 取值域**只有这五个**，改它就是改可 grep 的验证契约：
///
/// | 值 | 什么时候 |
/// | --- | --- |
/// | `dismiss` | 点面板外（编辑器 / 文件树 / 面板下方……） |
/// | `toggle` | 再点**同一个**标题；或点左上角图标收起整层浮动菜单栏 |
/// | `escape` | `Esc` |
/// | `select` | 点了面板里的一项 |
/// | `switch` | 悬停切到**另一个**顶级项（真源 `openOnHover`，`ui/menubar.tsx:84`） |
///
/// ⚠️ 为什么不是一个字符串：这五个值要在四处调用点各写一次，写成字符串就一定会有人
/// 拼出 `escape` 之外的写法（`esc` / `cancel`）而验证脚本 grep 不到。枚举 + 单测钉住
/// 取值域，比注释可靠。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    /// 点面板外。
    Dismiss,
    /// 再点同一标题（或左上角图标收起整层）。
    Toggle,
    /// `Esc`。
    Escape,
    /// 选了面板里的一项。
    Select,
    /// 悬停切到另一个顶级项。
    Switch,
}

impl CloseReason {
    /// 诊断行 `S1_MENU_CLOSE reason=…` 的取值（可 grep 的契约，改它就是改验证脚本）。
    pub const fn id(self) -> &'static str {
        match self {
            Self::Dismiss => "dismiss",
            Self::Toggle => "toggle",
            Self::Escape => "escape",
            Self::Select => "select",
            Self::Switch => "switch",
        }
    }
}

/// 收起时的诊断。`reason` 是**唯一**的收起出口 [`MenuBar::close`] 的入参。
pub fn diagnose_close(reason: CloseReason) {
    eprintln!("S1_MENU_CLOSE reason={}", reason.id());
}

/// 执行一条菜单动作后的诊断。`id` 用**文案键去掉 `lithe.` 前缀**，
/// 这样它既短又能与 `lithe.menu.*` / `lithe.workbench.*` 的键对上。
pub fn diagnose_run(action: MenuAction, state: &str) {
    eprintln!(
        "S1_MENU_RUN id={} state={state}",
        action.label_key().trim_start_matches("lithe.")
    );
}

/// 菜单栏的**可见性状态**（形态无关）：要么整条栏收起，要么某一项打开。
///
/// 抽成纯值是为了把"点第二个顶级项，上一个要收起"这条迁移做成**可单测的纯函数**
/// （[`resolve_menu_toggle`]）—— 本 crate 拿不到 `gpui::TestAppContext`，而
/// `MenuBar::toggle` 需要 `&mut App`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavState {
    /// 都收起（常驻形态下菜单栏本身仍可见，只是没有下拉面板）。
    Closed,
    /// 第 `N` 个顶级菜单打开（[`MENUS`] 的下标）。
    Open(usize),
}

/// 点某个顶级项之后的新状态：**已经开着它就收起，否则换成它**。
///
/// 真源是 `activeMenu` 单一受控值（`window-menu-bar.tsx:105-112`），所以这里也是"同时只有
/// 一个能开"。越界下标返回 `prev`（不动状态），不静默落到某一项上。
pub fn resolve_menu_toggle(prev: NavState, index: usize) -> NavState {
    if index >= MENUS.len() {
        return prev;
    }
    match prev {
        NavState::Open(current) if current == index => NavState::Closed,
        _ => NavState::Open(index),
    }
}

// ---------------------------------------------------------------------------
// 菜单栏状态
// ---------------------------------------------------------------------------

/// 菜单栏状态：`ShellWorkspace` 持有它，两形态共用。
///
/// ⚠️ 为什么是 `Entity` 而不是像 `activity_bar` / `status_bar` 那样一个无状态渲染函数：
/// 菜单的开关状态要**跨越帧**存在（点击 → 下一帧渲染出下拉面板 → 再点收起），
/// 而渲染函数没有地方放它；放进 `ShellWorkspace` 又会把"菜单内部状态"混进布局状态里。
///
/// 下拉面板（`PopupMenu` 的 `Entity`）**每次打开现建、收起就丢**：
/// 真机的菜单项是静态表 + 一处动态主题列，重建一张 5 行的面板代价极小（`AppMenuBar` 也是
/// 这一套：`popup_menu` + `Subscription` + `handle_dismiss`，见 `app_menu_bar.rs:142-218`）。
/// **打开时重置高亮/滚动**也正好是真机行为（菜单重新打开时从第一项开始）。
///
/// ⚠️ 但"现建"不等于"不持有"：B1 起面板实体与它的 `DismissEvent` 订阅**必须被本结构体
/// 持有到收起为止**（`Subscription` 一 drop 就取消，见 [`MenuBar::dismiss_subscription`]），
/// 收起时必须一起置 `None`（见 [`MenuBar::close`]），否则旧订阅会吊住已被替换的面板。
pub struct MenuBar {
    mode: MenuBarMode,
    /// 当前打开的顶级菜单下标（[`MENUS`] 的下标）；`None` = 都收起。
    ///
    /// 真源是 `activeMenu` 单一受控值（`window-menu-bar.tsx:105-112`）：**同时只有一个**菜单
    /// 能开着，所以这里也是一个 `Option<usize>` 而不是每项一个 bool。
    selected: Option<usize>,
    /// 形态 A 的浮动层是否展开（真源 `isCompactMenuVisible`，`title-bar.tsx:165-173`）。
    ///
    /// 与 `selected` 分开：图标按钮控制的是"菜单栏这一层在不在"，`selected` 控制的是
    /// "里面哪一项开着"。点图标收起整层时两个一起清。
    compact_visible: bool,
    /// 打开的那张下拉面板（`None` = 没有面板被建出来）。
    ///
    /// 为什么必须持有它（而不是像以前那样每次渲染时现建、画完就不管）：
    /// 面板的"点外部 / `Esc` / 选中项"三条收起路径都会 `cx.emit(DismissEvent)`
    /// （`popup_menu.rs:1055-1057` 的 `dismiss`，触发点 `:1474-1475`、`:873`），
    /// 而我们**必须订阅**它才能把 `selected` 清掉 —— 否则面板下一帧照旧被画出来。
    /// 订阅要一个活着的实体，实体又要活到订阅被取消为止。
    popup: Option<Entity<PopupMenu>>,
    /// 面板 `DismissEvent` 的订阅。**必须被持有**：`Subscription` 一 drop 就取消
    /// （gpui 的 RAII 语义），丢掉它等于"点外部 / `Esc` 再也收不掉面板"
    /// —— 那正是 B1 要修的第一个根因。
    dismiss_subscription: Option<Subscription>,
    /// 9 个顶级项各自的**窗口内边界**（上一帧 prepaint 量到的）。
    ///
    /// 面板摆位要用它：`Positioner::side(trigger_bounds)` 收的是**触发器的盒子**，
    /// 而不是"锚点元素在布局里的静态位置"（用后者实测会把面板顶到窗口左上角、
    /// 反过来盖住触发器，见 `popup_for` 的文档与 `project_menu.rs:633-639` 的同一条实测）。
    /// 用 `Rc<RefCell<Vec<..>>>` 而不是普通字段：量它的 `on_prepaint` 回调要 `'static`，
    /// 而渲染读它发生在同一个实体的渲染链里（与 `project_menu.rs:726-732` 同一招；
    /// 那里只有一个触发器所以是 `Cell`，这里有 9 个所以按下标存）。
    trigger_bounds: Rc<RefCell<Vec<Option<Bounds<Pixels>>>>>,
    /// 形态 A 的图标按钮边界（浮动胶囊的摆位锚点，同上）。
    compact_trigger_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    /// 「把焦点还给外壳根」这件事**排在下一帧做**（`true` = 还没做）。
    ///
    /// 为什么需要它：本结构体在收起时会**丢掉面板实体**，而面板当时正持有焦点
    /// （`Esc` / `↑↓` / `Enter` 的 `key_context` 在它身上，所以打开时我们显式把焦点交给它）。
    /// 焦点节点一消失，gpui 的按键派发路径就空了 —— **全部全局快捷键都会失效**，
    /// 直到用户再点一下界面（这条死角在 `crate::command_palette` 的 `SHELL_FOCUS` 上
    /// 已经踩过一次）。`Window::focus` 要 `&mut Window`，而收起可能发生在
    /// `--menu-probe` 那种"只有 `&mut App`"的路径上，所以统一延到下一帧的渲染里做
    /// （渲染链手里正好有 `&mut Window`）。
    restore_focus: bool,
    /// 下拉项的点击回执，由 [`MenuBar::drain_runs`] 取走。
    ///
    /// ⚠️ 为什么用队列而不是在 item 的 `on_click` 里直接执行：`PopupMenu` 的 item 处理器拿到的是
    /// `&mut App`，够不到 `&mut Window`（保存要 `window`、新建终端页签要 `window`），
    /// 也拿不到 `ShellWorkspace` 的 `&mut self`。用队列把"用户点了什么"带回
    /// `ShellWorkspace::render`（那里两样都有），是本 crate 既有的做法
    /// （`command_palette` 用 `on_confirm` + `SHELL` 也是同一个问题的另一种解法）。
    pending: Vec<MenuAction>,
    /// 外壳根元素的焦点句柄：`PopupMenu` 收起时把焦点还给它
    /// （`popup_menu.rs:1052-1072` 的 `dismiss`），所以关闭菜单之后焦点不会凭空消失。
    action_context: FocusHandle,
    /// 外壳句柄：菜单状态一变就要让外壳重绘。
    ///
    /// **为什么需要它**：菜单栏**不是** `Render`（它由外壳的 `render` 画），所以菜单栏
    /// 这一侧没有 `cx.notify()` 可调（`App::notify` 要一个 `EntityId`，而
    /// `Context<MenuBar>::notify` 只在 `Render` 的上下文里存在）。菜单栏的状态变化
    /// 因此统一通过外壳的句柄通知。
    ///
    /// 取值来自 `MENU_BAR_SHELL`（[`set_shell`]，`ShellWorkspace::new` 登记），
    /// 所以单元/集成测试可以先把某个真实外壳登记进来再用菜单栏。
    shell: Option<gpui_kit::WeakEntity<crate::workspace::ShellWorkspace>>,
    /// `Esc` 的**应用级拦截器**（`App::intercept_keystrokes`）。
    ///
    /// 为什么非它不可：`Esc` 在上游是 `PopupMenu` 自己绑的 `escape → Cancel`
    /// （`popup_menu.rs:24`）→ `dismiss()` → `DismissEvent`，而且**动作派发早于**任何
    /// `on_key_down` 监听器（`window.rs:5869` 的 interceptor → `:5880` 的 binding →
    /// `:6060` 才轮到 key listener）。所以从 `DismissEvent` 那一侧根本分不出
    /// "`Esc`" 与 "点面板外"（两者是同一个事件、同一个 `dismiss`）。拦截器是唯一
    /// **在动作之前**跑到的钩子（`app.rs:2317-2321` 明写"fires _before_ all other action
    /// and event mechanisms"），因此在它里面就能拿到确切的 `escape` 起因，
    /// 并且 `stop_propagation()` 掉，让上游那条 `Cancel` 不再把面板自己收一遍
    /// （否则会多出一行 `reason=dismiss`）。
    escape_interceptor: Option<Subscription>,
}

impl MenuBar {
    /// 建菜单栏：**这里**打构造期诊断（形态 / 顶级项数 / 可执行项数）。
    ///
    /// 外壳句柄从 [`MENU_BAR_SHELL`] 取（`ShellWorkspace::new` 先调 [`set_shell`]）；
    /// 没有登记时是 `None`，菜单照常工作，只是状态变化不会让外壳重绘 ——
    /// 这只在"菜单栏根本不在某个外壳里"的测试宿主上发生。
    pub fn new(mode: MenuBarMode, action_context: FocusHandle, cx: &mut App) -> Entity<Self> {
        mode.diagnose();
        let shell = MENU_BAR_SHELL.with(|slot| slot.borrow().clone());
        let bar = cx.new(|_| Self {
            mode,
            selected: None,
            compact_visible: false,
            popup: None,
            dismiss_subscription: None,
            trigger_bounds: Rc::new(RefCell::new(vec![None; MENUS.len()])),
            compact_trigger_bounds: Rc::new(Cell::new(None)),
            restore_focus: false,
            pending: Vec::new(),
            action_context,
            shell,
            escape_interceptor: None,
        });
        // `Esc` 的拦截器（理由见 [`MenuBar::escape_interceptor`]）。
        // ⚠️ 注册在**构造之后**：闭包要一个本实体的弱引用，而 `cx.new` 的闭包里还拿不到它。
        // `WeakEntity` 而不是强引用：窗口关掉之后这个应用级订阅不能吊住整条菜单栏。
        let handle = bar.downgrade();
        bar.update(cx, |bar, cx| {
            bar.escape_interceptor = Some(cx.intercept_keystrokes(move |event, _window, cx| {
                if !is_escape(&event.keystroke) {
                    return;
                }
                let closed = handle
                    .update(cx, |bar, cx| bar.close(CloseReason::Escape, cx))
                    .unwrap_or(false);
                if closed {
                    // 上游那条 `escape → Cancel` 不能再跑：不然面板会自己 `dismiss()` 一次，
                    // 我们又会收到一个 `DismissEvent`（那一侧的起因只能是 `dismiss`），
                    // 日志里就会多出一行原因不对的 `S1_MENU_CLOSE`。
                    cx.stop_propagation();
                }
            }));
        });
        bar
    }

    /// 让外壳重绘（菜单状态变了就调它；句柄丢了 —— 窗口正在关 —— 就什么都不做）。
    fn notify_shell(&self, cx: &mut App) {
        let Some(shell) = self.shell.as_ref() else {
            return;
        };
        let _ = shell.update(cx, |_, cx| cx.notify());
    }

    /// 当前形态。
    pub fn mode(&self) -> MenuBarMode {
        self.mode
    }

    /// 当前打开的顶级菜单下标。
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// 形态 A 的浮动层是否展开。
    pub fn compact_visible(&self) -> bool {
        self.compact_visible
    }

    /// 切换形态（`Ctrl+M` → `crate::workspace` 的 `ToggleMenuBar`）。
    ///
    /// 换形态等于重画整条栏：两个可见性状态一起清、面板实体与订阅一起丢，免得切回常驻时
    /// 还留着浮动层或一个没人看的订阅。
    ///
    /// ⚠️ 这里**不打** `S1_MENU_CLOSE`：Q17 把那个诊断的取值域钉死为五条关闭路径，
    /// 而"换形态"不在其中（它有自己的 `S1_MENU_BAR mode=…`）。状态清理照做，
    /// 只是不给它编一个假的起因。
    pub fn toggle_mode(&mut self, cx: &mut App) {
        self.mode = match self.mode {
            MenuBarMode::Pinned => MenuBarMode::Compact,
            MenuBarMode::Compact => MenuBarMode::Pinned,
        };
        let had_popup = self.popup.take().is_some();
        self.dismiss_subscription = None;
        self.selected = None;
        self.compact_visible = false;
        if had_popup {
            // 面板实体被丢掉 → 它持有的焦点也要在下一帧还给外壳（理由见字段文档）。
            self.restore_focus = true;
        }
        // 诊断行与构造期同格式，所以验证脚本一个 grep 就能同时看到两种形态。
        self.mode.diagnose();
        self.notify_shell(cx);
    }

    /// 点顶级项：已经开着就收起，否则打开（真源 `activeMenu` 的受控语义）。
    ///
    /// 状态迁移本身是纯函数 [`resolve_menu_toggle`]（单测直接覆盖它），这里只负责
    /// 把它落到字段、打诊断、通知外壳。
    ///
    /// ⚠️ **由鼠标按下触发**（不是 `on_click`）：真机上"再点同一个标题"这一下会先被下游面板
    /// 的 `on_mouse_down_out` 看见（触发器对面板而言就是"外部"），面板于是自己
    /// `dismiss()` 并发 `DismissEvent`；若本函数挂在 mouse-up 的 `on_click` 上，
    /// 那次收起之后又会立刻把菜单开回来（净效果 = 点了没反应）。挂在按下那一刻，
    /// 并且 [`MenuBar::handle_dismiss`] 对"按在打开项自己的触发器上"的 dismiss **故意不处理**
    /// （留给这里），两条路径就不打架了。
    pub fn toggle(&mut self, index: usize, cx: &mut App) {
        // 越界下标什么也不做（纯函数里已经挡住，这里再挡一次是为了不白打日志）。
        let Some(menu) = MENUS.get(index) else {
            return;
        };
        let next = resolve_menu_toggle(self.nav_state(), index);
        match next {
            NavState::Closed => {
                self.close(CloseReason::Toggle, cx);
            }
            NavState::Open(opened) => {
                // 换到另一项：先把旧面板实体与订阅丢掉，新面板由下一帧的渲染现建
                // （不丢的话旧面板的 `DismissEvent` 订阅还挂着，会把新面板一起收掉）。
                self.popup = None;
                self.dismiss_subscription = None;
                self.compact_visible = true;
                self.selected = Some(opened);
                diagnose_open(menu.id);
                self.notify_shell(cx);
            }
        }
    }

    /// 当前可见性状态（纯值形式，[`resolve_menu_toggle`] 的输入）。
    fn nav_state(&self) -> NavState {
        match self.selected {
            Some(index) => NavState::Open(index),
            None => NavState::Closed,
        }
    }

    /// **收起的唯一出口**（Q17）：五条路径全部走这里，`reason` 说明起因。
    ///
    /// 五条路径与各自的调用点：
    ///
    /// | 路径 | 调到这里的地方 |
    /// | --- | --- |
    /// | 点面板外 | [`MenuBar::handle_dismiss`]（上游 `on_mouse_down_out` → `DismissEvent`） |
    /// | 再点同一标题 | [`MenuBar::toggle`]（`NavState::Closed` 那一支） |
    /// | `Esc` | [`MenuBar::new`] 里登记的 `App::intercept_keystrokes` 拦截器 |
    /// | 选中某项 | [`MenuBar::push_run`] |
    /// | 悬停切换 | [`MenuBar::hover_switch`]（收起旧的，再打开新的） |
    ///
    /// 做三件事：清可见性状态、**丢掉面板实体与它的订阅**、按需安排"下一帧把焦点还给外壳"。
    ///
    /// 返回 `true` = 这次真的收起了（本来是开着的）；`false` = 本来就是收起的（不白打日志、
    /// 也不抢焦点）。`Esc` 拦截器靠这个返回值决定要不要 `stop_propagation`
    /// （没有菜单开着时绝不能吞掉 `Esc` —— 它可能是别的浮层要用的键）。
    pub fn close(&mut self, reason: CloseReason, cx: &mut App) -> bool {
        if self.selected.is_none() && !self.compact_visible {
            return false;
        }
        let had_popup = self.popup.take().is_some();
        self.dismiss_subscription = None;
        self.selected = None;
        self.compact_visible = false;
        // 焦点是交给面板的（`Esc` / `↑↓` / `Enter` 的 `key_context` 在它身上），面板一丢
        // 焦点节点就没了 —— 而"没有焦点节点"会让**全部全局快捷键**失效，所以要还回去。
        // 还的时机是下一帧的渲染（理由见 [`MenuBar::restore_focus`]）。
        //
        // ⚠️ 只有"我们自己丢掉了面板"才需要还：`PopupMenu::dismiss()` 那三条路径
        // （点外部 / `Esc` / 选中项）它自己会把焦点还给 `action_context`
        // （`popup_menu.rs:1059-1072`），但那时我们**已经**把实体丢了 ——
        // 上游是在 `DismissEvent` 之后才还的，不冲突，只是这里多还一次同样安全的焦点。
        if had_popup {
            self.restore_focus = true;
        }
        diagnose_close(reason);
        self.notify_shell(cx);
        true
    }

    /// 面板自己收起来了（`Esc` / 点面板外 / 选中某一项都会发 `DismissEvent`）。
    ///
    /// ⚠️ **"按在打开项自己的触发器上"要放过**：那次按下的位置对面板来说是"外部"，
    /// 于是面板先 `dismiss()` 并把这个事件发过来；但用户的意图是"再点一下这个标题 → 收起"
    /// （[`MenuBar::toggle`] 的 Closed 分支），如果在这里就收掉，紧接着那次按下的
    /// `on_mouse_down` 又会把菜单重新打开 —— 净效果是"点标题关不掉"，正是 B1 要修的第二个根因
    /// 的另一种表现。判据用**当前鼠标位置是否落在打开项自己的触发器盒里**：
    /// 鼠标位置在事件派发前刚被置成这次按下的坐标（`gpui-pre-0.3.6/src/window.rs:5447`），
    /// 所以它就是这一下的真实落点。
    fn handle_dismiss(
        &mut self,
        _popup: &Entity<PopupMenu>,
        _event: &DismissEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(index) = self.selected {
            let on_own_trigger = self
                .trigger_bounds_of(index)
                .is_some_and(|bounds| bounds.contains(&window.mouse_position()));
            if on_own_trigger {
                return;
            }
        }
        self.close(CloseReason::Dismiss, cx);
    }

    /// 第 `index` 个触发器上一帧量到的边界（还没量到就是 `None`）。
    fn trigger_bounds_of(&self, index: usize) -> Option<Bounds<Pixels>> {
        self.trigger_bounds.borrow().get(index).copied().flatten()
    }

    /// 悬停到另一个顶级项：**已经有菜单开着时**直接换过去（真源 `openOnHover`，
    /// `ui/menubar.tsx:84`；上游 `AppMenuBar::handle_hover` 同口径，`app_menu_bar.rs:241-254`）。
    ///
    /// 都收起时**不打开** —— 真源的 `openOnHover` 只在 menubar 已有打开项时才生效，
    /// 否则鼠标扫过菜单栏就会一路弹出面板。
    fn hover_switch(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(current) = self.selected else {
            return;
        };
        if current == index || MENUS.get(index).is_none() {
            return;
        }
        // 旧的以 `switch` 为由收起（面板实体与订阅一起丢），然后打开新的 ——
        // 这两步与"点另一个标题"走的是同一条受控单值迁移，只是起因不同。
        self.close(CloseReason::Switch, cx);
        self.toggle(index, cx);
    }

    /// 点左上角图标：整层浮动菜单栏的显示 / 收起（真源 `handleCompactMenuToggle`）。
    ///
    /// 收起整层与"再点同一标题"是**同一个意图**（把开着的那一层关掉），所以共用
    /// [`CloseReason::Toggle`]；`id=compact_bar` 那一行 `S1_MENU_OPEN` 负责区分是"整层"
    /// 还是"某一项"（`S1_MENU_CLOSE` 的取值域由 Q17 钉死为那五个，不为形态另开一个值）。
    pub fn toggle_compact(&mut self, cx: &mut App) {
        if self.compact_visible {
            self.close(CloseReason::Toggle, cx);
        } else {
            self.compact_visible = true;
            self.selected = None;
            diagnose_open("compact_bar");
            self.notify_shell(cx);
        }
    }

    /// 记下一次菜单项点击（渲染期由 item 的 `on_click` 写入，见 [`MenuBar::pending`]）。
    fn push_run(&mut self, action: MenuAction, cx: &mut App) {
        self.pending.push(action);
        // 点完就收起（真源 `closeMenu()`，`window-menu-bar.tsx:105-112`）。
        // ⚠️ 这里是 `reason=select`：**不是**上游面板 `dismiss` 的那条路径 —— 面板的
        // `confirm()` 在跑完我们的 item 处理器之后**才会** `dismiss()`（`popup_menu.rs:859-873`），
        // 而那时订阅已经被 `close()` 一起丢掉了，所以不会多出第二行日志。
        self.close(CloseReason::Select, cx);
    }

    /// 取走待执行的动作。
    ///
    /// 返回值带 `window` / `cx` 的执行留给外壳 —— 理由见 [`MenuBar::pending`]。
    pub fn drain_runs(&mut self) -> Vec<MenuAction> {
        std::mem::take(&mut self.pending)
    }

    /// **验证/诊断入口**：按名字打开一个顶级菜单（`--menu-probe <id>`）。
    ///
    /// 这不是产品能力：它与真机上"点一下那个顶级项"调的是**同一个** [`MenuBar::toggle`]，
    /// 唯一被绕开的是"操作系统把这次点击送进窗口"那一段。存在的理由与
    /// `crate::command_palette::open_command_palette` 的 `--open-palette` 完全相同：
    /// 本机的工作站**锁屏**，鼠标/键盘注入到不了应用（`PostMessage` 与
    /// `SetCursorPos+mouse_event` 两条路实测都无效），没有这个入口就无法在无人值守环境里
    /// 取证"菜单到底长什么样、点下去真的改了什么状态"。
    ///
    /// 返回 `false` = 没有这个 id（调用方据此报错退出，不静默）。
    pub fn open_by_id(&mut self, id: &str, cx: &mut App) -> bool {
        let Some(index) = MENUS.iter().position(|menu| menu.id == id) else {
            return false;
        };
        // `--menu-probe` 的形态复位：不带着启动时可能残留的状态打开（面板实体与订阅也一起
        // 丢掉 —— 与 [`MenuBar::close`] 同一套清理，只是这里不产生"关闭"日志）。
        self.popup = None;
        self.dismiss_subscription = None;
        self.selected = None;
        self.compact_visible = false;
        self.toggle(index, cx);
        true
    }

    /// **验证/诊断入口**：执行主题子菜单的一项（`--theme-probe <下标|主题名>`）。
    ///
    /// 主题项是**动态生成**的（注册表里有什么就画什么），没有固定的动作名，
    /// 所以 [`lookup_action`] 覆盖不到它 —— 这是菜单里唯一一对"没有名字的动作"。
    /// 这个入口走的仍是 `on_click` 里那段完全相同的代码（[`theme_menu`] 的处理器），
    /// 不是另一条实现。
    ///
    /// `selector` 是数字时按下标取（注册表序：`ThemeRegistry::sorted_themes`），
    /// 否则按名字取 —— **按名字更稳**：注册表里除了 Lithe 自己的两个主题，还有
    /// gpui-kit 的内置主题，下标会随上游变化。
    ///
    /// 返回 `false` = 既不是合法下标、也没有这个名字（调用方据此报错，不静默）。
    pub fn run_theme_by(&mut self, selector: &str, cx: &mut App) -> bool {
        let names = lithe_gpui_settings::theme::theme_names(cx);
        let picked = match selector.parse::<usize>() {
            Ok(index) => names.get(index).cloned(),
            Err(_) => names.iter().find(|name| name.as_str() == selector).cloned(),
        };
        let Some(name) = picked else {
            return false;
        };
        apply_theme_choice(name, cx);
        true
    }

    /// **验证/诊断入口**：按名字执行一条菜单动作（`--menu-probe <id> <动作>`）。
    ///
    /// ⚠️ 它走的是**菜单项点击那条完全相同的路**：先 [`MenuBar::push_run`]（`on_click` 里
    /// 干的唯一一件事），再由外壳在下一帧 `drain_runs` + `apply_menu_action` 真正执行。
    /// 所以它证明的是"菜单项被点之后会发生什么"，而不是绕开菜单直接调动作。
    pub fn run_action(&mut self, action: MenuAction, cx: &mut App) {
        self.push_run(action, cx);
    }
}

/// 主题子菜单里点一项时干的事（`theme_menu` 的 `on_click` 与
/// [`MenuBar::run_theme_by_index`] **共用**这一段，不写两份）。
///
/// 与真源 `handleThemeChange`（`use-menu-events-wrapper.ts:287-295`）同一条语义：用户从菜单里
/// 点名要一个主题，意思就是**别跟着系统了**，所以走 `set_theme_explicit`
/// （它把 `syncSystemTheme` 一起关掉，`settings/src/store.rs:269`）。
pub fn apply_theme_choice(name: String, cx: &mut App) {
    match lithe_gpui_settings::try_store(cx) {
        Some(store) => {
            let value: SharedString = name.clone().into();
            store.update(cx, |store, cx| store.set_theme_explicit(value, cx));
            eprintln!("S1_MENU_RUN id=lithe.menu.theme state=applied name={name}");
        }
        None => eprintln!("S1_MENU_RUN id=lithe.menu.theme state=unavailable"),
    }
}

/// **验证/诊断入口**：把命令行给的动作名解析成一条菜单动作（`--menu-probe` 的第二段）。
///
/// 取值 = [`MenuAction::label_key`]（就是诊断行 `S1_MENU_RUN id=…` 里那个 id），
/// 所以验证脚本手里已经有一个稳定、可 grep 的名字，不需要再维护第二张对照表。
pub fn lookup_action(id: &str) -> Option<MenuAction> {
    MENUS
        .iter()
        .flat_map(|menu| menu.items.iter())
        .find_map(|item| match item {
            MenuItem::Action(action) if action.label_key() == id => Some(*action),
            _ => None,
        })
}

// ---------------------------------------------------------------------------
// 渲染
// ---------------------------------------------------------------------------

/// 当前应该画哪一种形态。集中一处，免得"形态"这件事在两个地方各判一次。
pub fn mode_for(compact: bool) -> MenuBarMode {
    if compact {
        MenuBarMode::Compact
    } else {
        MenuBarMode::Pinned
    }
}

thread_local! {
    /// 当前窗口的菜单栏句柄。
    ///
    /// 为什么需要它（两件事都要它）：
    ///
    /// 1. [`crate::command_palette::ToggleMenuBar`] 是**应用级** action
    ///    （`cx.bind_keys` 的处理器拿不到任何视图），而形态归菜单栏自己所有；
    /// 2. [`menu_bar`] 在 `ShellWorkspace::render` 里被调用，那里
    ///    `cx.entity()` 是**外壳**的句柄而不是菜单栏的 —— 点击回调要回来改菜单状态，
    ///    只能靠这个句柄。
    ///
    /// 用 `WeakEntity` 而不是强引用：窗口关掉之后这里不能吊住整个视图。
    /// 与 `crate::command_palette` 的 `SHELL` 同一套做法与同一条理由。
    static MENU_BAR: std::cell::RefCell<Option<gpui_kit::WeakEntity<MenuBar>>> =
        const { std::cell::RefCell::new(None) };

    /// 外壳句柄（菜单状态一变就通知它重绘，见 [`MenuBar::shell`]）。
    ///
    /// 与 [`MENU_BAR`] 分开存：菜单栏是**每个窗口一个**，而"谁是外壳"这件事要能在
    /// 菜单栏建出来**之前**登记好（`ShellWorkspace::new` 里 `MenuBar::new` 的调用点
    /// 还没法把自己传进去）。
    static MENU_BAR_SHELL: std::cell::RefCell<Option<gpui_kit::WeakEntity<crate::workspace::ShellWorkspace>>> =
        const { std::cell::RefCell::new(None) };
}

/// 登记外壳句柄（[`crate::workspace::ShellWorkspace::new`] 调用一次）。
pub(crate) fn set_shell(shell: gpui_kit::WeakEntity<crate::workspace::ShellWorkspace>) {
    MENU_BAR_SHELL.with(|slot| *slot.borrow_mut() = Some(shell));
}

/// 登记菜单栏句柄（[`crate::workspace::ShellWorkspace::new`] 调用一次）。
pub(crate) fn set_menu_bar(bar: gpui_kit::WeakEntity<MenuBar>) {
    MENU_BAR.with(|slot| *slot.borrow_mut() = Some(bar));
}

/// 取当前窗口的菜单栏句柄（没有就报错，不静默）。
///
/// `pub` 是给 `--menu-probe` 用的（诊断入口，见 [`MenuBar::open_by_id`] 的说明）。
pub fn handle() -> gpui_kit::WeakEntity<MenuBar> {
    MENU_BAR.with(|slot| slot.borrow().clone()).expect(
        "菜单栏句柄没有登记：`ShellWorkspace::new` 必须在 `render` 之前调 `set_menu_bar`",
    )
}

/// 取走待执行的菜单动作（`ShellWorkspace::render` 每帧开头调一次）。
///
/// 句柄丢了（窗口正在关）返回空表，不 panic —— 该帧不会执行任何菜单动作。
pub(crate) fn take_pending_runs(cx: &mut App) -> Vec<MenuAction> {
    let Some(bar) = MENU_BAR.with(|slot| slot.borrow().clone()) else {
        return Vec::new();
    };
    bar.update(cx, |bar, _| bar.drain_runs()).unwrap_or_default()
}

/// 「切换菜单栏」的实现（`Ctrl+M`）。句柄丢了（窗口正在关）就什么都不做。
pub fn toggle_menu_bar(cx: &mut App) {
    let Some(bar) = MENU_BAR.with(|slot| slot.borrow().clone()) else {
        eprintln!("S1_MENU_BAR mode=unavailable");
        return;
    };
    let _ = bar.update(cx, |bar, cx| bar.toggle_mode(cx));
}

/// 画菜单栏。**必须是 `drag_region` 的兄弟节点**（理由见 [`crate::title_bar`] 的 `title_bar`
/// 文档：Windows 命中测试取第一个 `window_control_hitboxes` 命中项，祖先的 `Drag` 会赢，
/// 点菜单只会拖窗口）。
///
/// 两形态共用同一份 [`MENUS`]，差别只有容器样式与定位 —— 见模块文档的对照表。
///
/// ⚠️ 收 `&mut MenuBar` / `&mut Context<MenuBar>`（B1 改的签名）：打开的那一帧要**建面板实体
/// 并订阅它的 `DismissEvent`**，两件事都要 `&mut`（订阅是 `Context::subscribe_in`）。
/// 以前这里一路是 `&App`，所以当年"订阅 DismissEvent"这件事根本没有落点。
pub fn menu_bar(
    bar: &mut MenuBar,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> impl IntoElement {
    // 「把焦点还给外壳根」延到这一帧做（理由见 [`MenuBar::restore_focus`]）：渲染链手里
    // 正好有 `&mut Window`，而收起可能发生在只有 `&mut App` 的路径上。
    if bar.restore_focus {
        bar.restore_focus = false;
        window.focus(&bar.action_context, cx);
    }
    match bar.mode {
        MenuBarMode::Pinned => pinned_bar(bar, window, cx).into_any_element(),
        MenuBarMode::Compact => compact_bar(bar, window, cx).into_any_element(),
    }
}

/// 画菜单栏并返回 `AnyElement`（`ShellWorkspace::render` 的入口）。
///
/// 为什么要有这一层：菜单栏是 `Entity`，读它要 `cx` 的不可变借用，而画下拉面板要
/// `&mut Context<MenuBar>`（`PopupMenu::build` 会新建实体、`subscribe_in` 要可变上下文）
/// —— 两者不能在同一段代码里同时成立。把"读 + 画 + 转 `AnyElement`"收在这一个函数里，
/// 外壳那一行就只剩一次调用。
///
/// ⚠️ 收的是 `&Entity<MenuBar>` 而**不是**从 [`MENU_BAR`] 里现取：外壳持有这个 `Entity`
/// 才算"菜单栏活着"（见 `ShellWorkspace::menu_bar` 字段的说明），句柄丢了就返回 `None`。
pub(crate) fn render_bar(
    bar: &Entity<MenuBar>,
    window: &mut Window,
    cx: &mut App,
) -> Option<gpui_kit::AnyElement> {
    bar.downgrade()
        .update(cx, |bar, cx| menu_bar(bar, window, cx).into_any_element())
        .ok()
}

/// 形态 B：与标题栏同一行的 24px 胶囊。
fn pinned_bar(bar: &mut MenuBar, window: &mut Window, cx: &mut Context<MenuBar>) -> impl IntoElement {
    // ⚠️ 顶级项**逐个 `.child(..)`**，不要写成
    // `.children((0..MENUS.len()).map(|index| trigger(.., window, cx)))`：
    // `window` / `cx` 是可变借用，那个 `map` 闭包一构造就要求独占访问，而外层
    // 也还在用它们，编译器直接报 E0500「closure requires unique access to `*window`」
    // （本轮实测）。逐个 `.child()` 没有闭包，借用按语句顺序进出。
    let mut bar_element = h_flex()
        // 必须有 `id`：`hover` / `active` 态存在元素状态里，无 id 的 `Div` 拿不到
        // （`gpui-pre-0.3.6/src/elements/div.rs:2844-2849`），与 `title_bar.rs` 的三键同一条理由。
        .id("lithe-menu-bar-pinned")
        .flex_shrink_0()
        .items_center()
        .h_6()
        .gap_0p5()
        .p_0p5()
        .rounded_full()
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background);
    for index in 0..MENUS.len() {
        bar_element = bar_element.child(menu_item(bar, index, window, cx));
    }
    bar_element
}

/// 形态 A：左上角 `ListIcon` 按钮 + 标题栏下方的浮动胶囊。
///
/// 定位照真源 `absolute top-full left-0 mt-1`（`window-menu-bar.tsx:539`）：
/// 浮动层的**上沿**贴着按钮的下沿、左端与按钮对齐。
///
/// ⚠️ B1 前这里用的是 `anchored().anchor(Anchor::BottomLeft)`（"锚点元素自己在布局里的
/// origin + 面板左下角"），实测把整层顶到了标题栏那一行、压住按钮自己 —— 与下拉面板
/// 是同一条根因。现在与下拉面板统一走 [`Positioner::side`]（理由见 [`popup_for`]）。
fn compact_bar(
    bar: &mut MenuBar,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> impl IntoElement {
    // 回调都是 `'static`、够不到 `bar` 的引用，所以用句柄回来改状态
    // （与 `crate::workspace` 里项目标签条 / 活动栏的回调用法一致）。
    let handle = handle();
    let toggle_handle = handle.clone();
    let button = Button::new("lithe-menu-bar-trigger")
        // 真源是 `variant=ghost size=icon-xs`（`title-bar.tsx:209-219`），图标 14×14
        // （`ui/button.tsx:9` 的 `size-3.5`）；gpui-kit 的 `Sizable::small()` 给的正是 14。
        .ghost()
        .small()
        .icon(IconName::List)
        .tooltip(tr("lithe.window.menu"))
        .accessibility_label(tr("lithe.window.menu"))
        .on_click(move |_: &ClickEvent, _window: &mut Window, cx: &mut App| {
            let _ = toggle_handle.update(cx, |bar, cx| bar.toggle_compact(cx));
        });

    // 浮动层里的 9 个顶级项**先建好**（理由同 `pinned_bar`：`.children(map(..))` 会让
    // `window` / `cx` 的可变借用撞在一起，E0500）。只在真的展开时才建，收起时不做工。
    let mut floating = None;
    if bar.compact_visible {
        let mut capsule = h_flex()
            .id("lithe-menu-bar-floating")
            .items_center()
            // 胶囊里也按住 24px 高（真源 `rounded-2xl px-1 py-1` 包着同样的 20px 项，
            // 见 `window-menu-bar.tsx:548-549`），这样两种形态的顶级项行高完全一致。
            .h_6()
            .gap_0p5()
            .p_1()
            .mt_1()
            .rounded(FLOAT_RADIUS)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background);
        for index in 0..MENUS.len() {
            capsule = capsule.child(menu_item(bar, index, window, cx));
        }
        // 摆位与下拉面板同一条：`Positioner::side(按钮边界)`，`offset` 4 就是真源
        // `mt-1`（`window-menu-bar.tsx:539`）。还没量到按钮边界时（第 1 帧）**不画** ——
        // 画在错误的位置上比晚一帧更糟（`on_prepaint` 里已经补要了一帧）。
        if let Some(bounds) = bar.compact_trigger_bounds.get() {
            floating = Some(
                deferred(
                    Positioner::side(bounds)
                        .placement(Placement::Bottom)
                        .align(Align::Start)
                        .offset(px(PANEL_OFFSET_SPEC))
                        .margin(px(PANEL_WINDOW_MARGIN_SPEC))
                        // `occlude()` 不可省：不遮挡的话它下面的标题栏拖拽区会继续吃鼠标事件
                        // （`AppMenuBar` 的同一处：`app_menu_bar.rs:290-294`）。
                        .occlude()
                        .child(capsule),
                )
                .with_priority(1)
                .into_any_element(),
            );
        }
    }

    // 量按钮盒子的包装层：`on_prepaint` 打在 `Button` 自己身上给的是它的**内容盒**
    // （`.artifacts/p8` 在项目下拉上实测过：带 `px_2` 的触发器会整体右移 8 逻辑 px），
    // 所以量这一层无内边距的包装（与 `project_menu.rs:971-989` 同一条）。
    let bounds_cell = bar.compact_trigger_bounds.clone();
    div()
        .id("lithe-menu-bar-compact")
        .relative()
        .flex_shrink_0()
        .on_prepaint(move |bounds, window, _cx| {
            let first = bounds_cell.get().is_none();
            bounds_cell.set(Some(bounds));
            // 第一次量到时主动要一帧：浮动层的位置读的是**上一帧**的 bounds。
            if first {
                window.request_animation_frame();
            }
        })
        .child(button)
        .children(floating)
}

/// 一个顶级菜单项（两形态共用）：20px 高、`px-1.5`、圆角 6.4、悬停 / 打开态各有底色。
///
/// ⚠️ 这里**不画**下拉面板：面板由 [`menu_item`] 放在本项的外层里（理由见那里的文档）。
fn trigger(
    bar: &MenuBar,
    index: usize,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> impl IntoElement {
    let _ = window;
    let menu = &MENUS[index];
    let is_open = bar.selected == Some(index);
    let theme = cx.theme();
    let (idle, active_text, hover_bg) = (theme.muted_foreground, theme.foreground, theme.accent);
    let handle = handle();

    div()
        .id(("lithe-menu-trigger", index))
        .flex()
        .items_center()
        // `h-5` = 20（`ui/menubar.tsx:85-88`）。整条栏 24 = 20 + 上下各 2 的内边距。
        .h_5()
        .px_1p5()
        .rounded(ITEM_RADIUS)
        .text_sm()
        // 未悬停是 `text-subtle-foreground`（`ui/menubar.tsx:87`）；gpui-kit 没有 `subtle`
        // 这一项，语义最近的是 `muted_foreground`（与标题栏同一取法）。
        .text_color(if is_open { active_text } else { idle })
        .hover(move |style| style.bg(hover_bg).text_color(active_text))
        // 悬停切换顶级项（真源 `openOnHover`，`ui/menubar.tsx:84`）：只在**已经有菜单开着**
        // 时生效，判据在 `MenuBar::hover_switch` 里（都收起时悬停不打开，否则鼠标扫过菜单栏
        // 会把面板一路弹出来）。
        .on_hover(cx.listener(move |bar, hovered: &bool, _window, cx| {
            if *hovered {
                bar.hover_switch(index, cx);
            }
        }))
        // 点菜单**不能**拖窗口：菜单栏是 `drag_region` 的兄弟节点，本身不在 `Drag` 命中区里，
        // 所以这里不需要 `prevent_default`（`10-menu-bar.md` §4.5 第 4 条）；
        // `stop_propagation` 只为"别把这次点击继续冒泡给外层"。
        //
        // ⚠️ **顶级的开合挂在"按下"而不是"点击"上**（理由见 `MenuBar::toggle` 的文档）：
        // 面板的 `on_mouse_down_out` 会在按下的**捕获阶段**把落在触发器上的这一下当成
        // "点到了面板外"。改成按下就切换之后，那一下的意图由 `toggle` 独自表达，
        // 不会再出现"先被面板收掉、又被 click 开回来"的抖动。
        .on_mouse_down(MouseButton::Left, {
            let handle = handle.clone();
            move |_, _, cx| {
                cx.stop_propagation();
                let _ = handle.update(cx, |bar, cx| bar.toggle(index, cx));
            }
        })
        .child(menu.label())
}

/// 一个顶级菜单的**外层**：`relative` 的定位上下文 + 顶级项 + （打开时的）下拉面板。
///
/// 为什么必须有这一层：下拉面板是 `deferred(..)` 的浮层，如果把它直接当 `trigger` 的
/// 子节点，它（高度 = 面板高）会参与**这一行**的布局测量，把整条菜单栏的其余项顶偏
/// （实测：打开「视图」时那一项被抬高约 3px，与相邻项基线不齐）。这里让外层
/// （高度 = 顶级项的 20px）留在行内，面板用 `position: absolute` 摘出布局流。
///
/// 这一层**还是量触发器盒子的地方**（B1）：`on_prepaint` 打在无内边距的包装层上，
/// 量到的 bounds 与触发器的视觉盒子逐像素相等，面板因此能与触发器左沿严格对齐
/// （真源 `align="start"`）。打在 `trigger` 自己身上会拿到**内容盒**（带 `px_1.5`），
/// 面板会整体右移 6 逻辑 px —— 这条在项目下拉上已经实测过一次
/// （`project_menu.rs:971-975`）。
///
/// 面板**打开的那一帧**在这里建出来并订阅（见 [`MenuBar::popup`] /
/// [`MenuBar::dismiss_subscription`]），而不是每帧现建 —— 每帧现建的话订阅也会每帧重建，
/// 而"点外部 / `Esc`"的事件正好落在两次渲染之间，会被丢掉。
fn menu_item(
    bar: &mut MenuBar,
    index: usize,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> impl IntoElement {
    let is_open = bar.selected == Some(index);

    // 量本项外层的边界（面板摆位要用）。
    let bounds_cell = bar.trigger_bounds.clone();
    let mut wrapper = div()
        // 必须有 `id`：`on_prepaint` 与元素状态都挂在有 id 的元素上
        // （`project_menu.rs:978` 是同一个写法）。
        .id(("lithe-menu-item", index))
        .relative()
        .flex_shrink_0()
        .on_prepaint(move |bounds, window, _cx| {
            let first = bounds_cell.borrow()[index].is_none();
            bounds_cell.borrow_mut()[index] = Some(bounds);
            // 第一次量到时主动要一帧：面板的位置读的是**上一帧**的 bounds，
            // 不补一帧的话第一帧会画在错误的位置上（`Popup` 用同一招，
            // `gpui-base-0.6.6/src/popup.rs:110-125`）。
            if first {
                window.request_animation_frame();
            }
        })
        .child(trigger(bar, index, window, cx));

    if is_open {
        // 打开的第一帧才建面板：同时**订阅**它的 `DismissEvent`、把焦点交给它。
        // ⚠️ 顺序有讲究（照 `project_menu.rs:942-961`）：`subscribe_in` 与 `focus` 都必须在
        // 面板真的被建出来的那一次做，之后它由 `MenuBar::popup` 缓存，直到收起。
        if bar.popup.is_none() {
            let popup = build_popup(bar, index, window, cx);
            bar.dismiss_subscription =
                Some(cx.subscribe_in(&popup, window, MenuBar::handle_dismiss));
            // 焦点交给面板：`Esc` / `↑↓` / `Enter` 的 `key_context` 在它身上
            // （`popup_menu.rs:21-30,1467-1475`），焦点不在它身上这些键就不响 ——
            // 而"焦点不在"正是 B1 之前 `Esc` 收不掉的一半原因。
            let focus = popup.read(cx).focus_handle(cx);
            if !focus.contains_focused(window, cx) {
                focus.focus(window, cx);
            }
            // 位置证据：bounds 只可能来自 `on_prepaint`，所以这一行同时证明"量到了"。
            // `state=pending` 表示面板实体已经建出来、但**这一帧还没量到**触发器边界
            // （只有 `--menu-probe` 那种"第一帧就打开"的路径会这样，下一帧就量到了）——
            // 那时 `popup_for` 返回 `None`，面板还不会画出来，所以不能在这里编造坐标。
            match bar.trigger_bounds_of(index) {
                Some(bounds) => eprintln!(
                    "S1_MENU_BOUNDS id={} x={} y={} w={} h={}",
                    MENUS[index].id,
                    f32::from(bounds.origin.x) as i32,
                    f32::from(bounds.origin.y) as i32,
                    f32::from(bounds.size.width) as i32,
                    f32::from(bounds.size.height) as i32,
                ),
                None => eprintln!("S1_MENU_BOUNDS id={} state=pending", MENUS[index].id),
            }
            bar.popup = Some(popup);
        }
        if let Some(popup) = bar.popup.clone() {
            // `.children(Option<..>)`：`Option<T: IntoElement>` 本身是迭代器（0 或 1 个孩子），
            // 所以"还没量到触发器边界"那一帧自然就是"不画面板"。
            wrapper = wrapper.children(popup_for(popup, bar.trigger_bounds_of(index)));
        }
    }
    wrapper
}

/// 打开的下拉面板：延迟绘制 + 摆在触发器的**正下方**（左对齐、间隔 4、贴边留 8）。
///
/// ⚠️ **为什么不用 `anchored()` + `Anchor::BottomLeft`**（B1 之前用的那一种，也是"点标题
/// 收不掉"的第二个根因）：那条路取的是**锚点元素自己在布局里的静态位置**
/// （`gpui-pre-0.3.6/src/elements/anchored.rs:27-36,273-277`），配合 `Anchor::BottomLeft`
/// 会把面板的**左下角**放在触发器左上角 —— 面板整体跑到触发器**上方**，向上溢出窗口上沿后
/// 又被 `snap_to_window_with_margin(8)` 夹回 `y=8`，于是面板压回标题栏那一行、
/// **把触发器自己盖住**（面板 `.occlude()` = `HitboxBehavior::BlockMouse`，被盖住的标题
/// 既拿不到 hover 也拿不到点击）。`project_menu.rs:633-639` 在本仓的另一个下拉上
/// 实测并记过同一条（截图里只露出徽标的 1px）。
///
/// `Positioner::side(触发器 bounds)` 直接把"在触发器下方"这件事说出来，与真源的
/// `side="bottom" align="start" sideOffset=4 collisionPadding=8`
/// （`ui/menubar.tsx:106-110`）一一对应，也正是 gpui-kit 自己给 Select / Combobox /
/// DatePicker 用的那一支（`gpui-component-0.6.6/src/popover.rs:33-39` 的
/// `dropdown_positioner`）。
///
/// `bounds` 是 `None` 时**不画**（还没量到触发器，画在哪儿都是错的；`on_prepaint` 里已经
/// 补要了一帧，所以只影响打开后的第一帧）。
fn popup_for(popup: Entity<PopupMenu>, bounds: Option<Bounds<Pixels>>) -> Option<impl IntoElement> {
    let bounds = bounds?;
    Some(
        deferred(
            Positioner::side(bounds)
                .placement(Placement::Bottom)
                .align(Align::Start)
                .offset(px(PANEL_OFFSET_SPEC))
                .margin(px(PANEL_WINDOW_MARGIN_SPEC))
                // `occlude()` 不可省：不遮挡的话面板下面的标题栏拖拽区会继续吃鼠标事件
                // （`AppMenuBar` 的同一处：`app_menu_bar.rs:290-294`）。
                .occlude()
                .child(popup),
        )
        .with_priority(1),
    )
}

/// 造一个顶级菜单的下拉面板（**展开的第一帧建一次**，之后由 `MenuBar::popup` 持有到收起，
/// 理由见 [`MenuBar::popup`] 的字段文档）。
fn build_popup(
    bar: &MenuBar,
    index: usize,
    window: &mut Window,
    cx: &mut Context<MenuBar>,
) -> Entity<PopupMenu> {
    let items = MENUS[index].items;
    let action_context = bar.action_context.clone();
    let handle = handle();

    PopupMenu::build(window, cx, move |popup, window, cx| {
        // 动作上下文：`PopupMenu` 收起时把焦点还给它（`popup_menu.rs:1052-1072`），
        // 所以关闭菜单之后焦点不会凭空消失。
        let popup = popup
            .action_context(action_context.clone())
            // 面板最小宽 240（`ui/menubar.tsx:126` 的 `min-w-60`）。
            .min_w(PANEL_MIN_WIDTH);

        // 主题子菜单是**动态项**，内容每次打开现取（注册表里加了主题文件就能立刻看到）。
        // 主题名来自设置 crate 的注册表（`lithe_gpui_settings::theme::theme_names`），
        // 不写死。
        let names = lithe_gpui_settings::theme::theme_names(cx);
        let theme_menu = (!names.is_empty()).then(|| theme_menu(names, window, cx));

        let mut popup = popup;
        for item in items {
            popup = match item {
                MenuItem::Separator => popup.separator(),
                MenuItem::Action(action) => {
                    let action = *action;
                    // `on_click` 的处理器只做一件事：把"点了什么"记进队列，
                    // 真正的执行在 `ShellWorkspace::render`（理由见 `MenuBar::pending`）。
                    let item_handle = handle.clone();
                    popup.item(
                        PopupMenuItem::new(action.label())
                            .icon(Icon::new(action.icon()))
                            .on_click(move |_: &ClickEvent, _window: &mut Window, cx: &mut App| {
                                let _ = item_handle
                                    .update(cx, |bar, cx| bar.push_run(action, cx));
                            }),
                    )
                }
                MenuItem::Theme => match &theme_menu {
                    Some(submenu) => {
                        popup.item(PopupMenuItem::submenu(tr("lithe.menu.theme"), submenu.clone()))
                    }
                    // 注册表里一个主题都没有（宿主没装主题目录）：这一条**不画**，
                    // 而不是画一个点开是空的子菜单。
                    None => popup,
                },
            };
        }
        popup
    })
}

/// 主题子菜单：每个主题一条，点击写 `SettingsStore::set_theme_explicit`。
///
/// ⚠️ 与真源一致的两点：**不打勾**当前主题（真源没有勾选态，`10-menu-bar.md` §2.3），
/// 也没有"跟随系统"那一项（真源同样没有）。
fn theme_menu(
    names: Vec<String>,
    window: &mut Window,
    cx: &mut App,
) -> Entity<PopupMenu> {
    PopupMenu::build(window, cx, move |popup, _window, _cx| {
        let mut popup = popup.min_w(PANEL_MIN_WIDTH);
        for name in names {
            let label: SharedString = name.clone().into();
            let value: SharedString = name.into();
            popup = popup.item(
                PopupMenuItem::new(label)
                    .icon(Icon::new(IconName::Palette))
                    .on_click(move |_: &ClickEvent, _window: &mut Window, cx: &mut App| {
                        // 与真源 `handleThemeChange` 同一条语义：用户点名要一个主题 =
                        // 别跟着系统了，所以走 `set_theme_explicit`
                        // （它把 `syncSystemTheme` 一起关掉，`settings/src/store.rs:269`）。
                        // 这一段与 `MenuBar::run_theme_by_index`（`--theme-probe`）
                        // **共用** `apply_theme_choice`，不写两份。
                        apply_theme_choice(value.to_string(), cx);
                    }),
            );
        }
        popup
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CloseReason, MENUS, MenuAction, MenuBarMode, MenuItem, NavState, action_count, mode_for,
        resolve_menu_toggle,
    };

    /// `S1_MENU_CLOSE reason=…` 的取值域**正好是 Q17 那五个**（顺序也照那份规格写），
    /// 而且互不相同。
    ///
    /// 这条守的是"五条关闭路径能被日志区分开"：验证脚本按这五个字面量 grep，
    /// 谁把 `escape` 写成 `esc`、或者给形态切换另编一个 `compact_toggle`，这里就红。
    #[test]
    fn close_reasons_match_the_spec_vocabulary() {
        let reasons = [
            (CloseReason::Dismiss, "dismiss"),
            (CloseReason::Toggle, "toggle"),
            (CloseReason::Escape, "escape"),
            (CloseReason::Select, "select"),
            (CloseReason::Switch, "switch"),
        ];
        let ids: Vec<&str> = reasons.iter().map(|(reason, _)| reason.id()).collect();
        assert_eq!(
            ids,
            vec!["dismiss", "toggle", "escape", "select", "switch"]
        );
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "五个起因必须互不相同");
    }

    /// B1 的三个根因各有一条**结构判据**（不需要窗口就能跑）：
    ///
    /// 1. 关闭系菜单项**必须**能映射到 `CommandId` —— 它们就是 `run_command_id` 的分支，
    ///    映射不上就会变成"画出来但点了不动"的死项；
    /// 2. 「打开文件」的文案键是**真源既有**的 `lithe.outline.openFile`（真源 `menu.*` 段
    ///    没有 `openFile`，本侧不新增键）；
    /// 3. File 菜单里的关闭项**一个不少**（真源 19 条里属于 B1 的那 8 条）。
    #[test]
    fn file_menu_close_entries_are_wired() {
        use crate::command_palette::CommandId;

        let expected = [
            (MenuAction::OpenFile, "lithe.outline.openFile", CommandId::OpenFile),
            (MenuAction::CloseTab, "lithe.menu.closeTab", CommandId::CloseTab),
            (
                MenuAction::CloseOtherTabs,
                "lithe.menu.closeOtherTabs",
                CommandId::CloseOtherTabs,
            ),
            (
                MenuAction::CloseAllTabs,
                "lithe.menu.closeAllTabs",
                CommandId::CloseAllTabs,
            ),
            (
                MenuAction::CloseSavedTabs,
                "lithe.menu.closeSavedTabs",
                CommandId::CloseSavedTabs,
            ),
            (
                MenuAction::CloseTabsToLeft,
                "lithe.menu.closeTabsToLeft",
                CommandId::CloseTabsToLeft,
            ),
            (
                MenuAction::CloseTabsToRight,
                "lithe.menu.closeTabsToRight",
                CommandId::CloseTabsToRight,
            ),
            (
                MenuAction::ReopenClosedTab,
                "lithe.menu.reopenClosedTab",
                CommandId::ReopenClosedTab,
            ),
        ];
        for (action, key, id) in expected {
            assert_eq!(action.label_key(), key, "{action:?} 的文案键变了");
            assert_eq!(action.command_id(), Some(id), "{action:?} 没有执行分支");
        }

        // File 菜单里**逐条**都在（顺序也照真源：打开文件 → 保存 → ── → 关闭系 → ── →
        // 重新打开）。
        let file = MENUS
            .iter()
            .find(|menu| menu.id == "file")
            .expect("顶级菜单里必须有 file");
        let actions: Vec<MenuAction> = file
            .items
            .iter()
            .filter_map(|item| match item {
                MenuItem::Action(action) => Some(*action),
                _ => None,
            })
            .collect();
        assert_eq!(
            actions,
            vec![
                MenuAction::OpenFile,
                MenuAction::Save,
                MenuAction::CloseTab,
                MenuAction::CloseOtherTabs,
                MenuAction::CloseAllTabs,
                MenuAction::CloseSavedTabs,
                MenuAction::CloseTabsToLeft,
                MenuAction::CloseTabsToRight,
                MenuAction::ReopenClosedTab,
            ]
        );
    }

    /// 顶级菜单必须**正好 9 个**，id 互不相同且顺序照真源
    /// （`window-menu-bar.tsx:133,200,277,365,411,427,440,462,504`；`locale.ts:7724-7732`）。
    ///
    /// 这条是"菜单结构与真源可比对"的唯一机器判据 —— 少了或多了一个顶级项，肉眼很难发现。
    #[test]
    fn nine_top_level_menus_in_source_order() {
        let ids: Vec<&str> = MENUS.iter().map(|menu| menu.id).collect();
        assert_eq!(
            ids,
            vec![
                "file", "edit", "view", "go", "terminal", "run", "tools", "window", "help"
            ]
        );
        let mut unique = ids.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "顶级菜单 id 必须互不相同");
    }

    /// 顶级项的文案键一律是 `lithe.menu.<id>`（真源由英文键名小写推导，`window-menu-bar.tsx:569`）。
    #[test]
    fn top_level_labels_use_the_menu_namespace() {
        for menu in MENUS {
            assert_eq!(
                menu.label_key,
                format!("lithe.menu.{}", menu.id).as_str(),
                "顶级菜单 {} 的文案键不是 lithe.menu.{}",
                menu.id,
                menu.id
            );
        }
    }

    /// **v1 不允许出现"没有行为"的项**：每个 `Action` 都必须有图标与文案键，
    /// 且要么能映射到一条 `CommandId`、要么在 `ShellWorkspace::run_menu_action` 里有分支。
    ///
    /// 这条测试守的是维护者的硬口径（"不要摆一片灰"）：一旦有人往表里塞一个占位项，这里就红。
    #[test]
    fn every_listed_action_is_executable() {
        // 6 条不走 `CommandId` 的动作 —— 它们的执行分支在
        // `ShellWorkspace::run_menu_action` 里，逐条接线点写在那里。
        // 这是**清单**而不是白名单：新增一条就得在这里显式登记一次。
        const NON_COMMAND: [MenuAction; 6] = [
            MenuAction::NewTerminalTab,
            MenuAction::CloseTerminalTab,
            MenuAction::Preferences,
            MenuAction::Minimize,
            MenuAction::Maximize,
            MenuAction::ToggleFullscreen,
        ];

        let mut seen: Vec<MenuAction> = Vec::new();
        for menu in MENUS {
            for item in menu.items {
                let MenuItem::Action(action) = item else {
                    continue;
                };
                assert!(
                    action.command_id().is_some() || NON_COMMAND.contains(action),
                    "{action:?} 既没有 CommandId，也没登记在 NON_COMMAND 里 —— 它就是一条死项"
                );
                assert!(
                    action.label_key().starts_with("lithe."),
                    "{action:?} 的文案键不在 lithe.* 命名空间里"
                );
                assert!(
                    !seen.contains(action),
                    "{action:?} 在菜单里出现了两次（同一个能力不该两个入口）"
                );
                seen.push(*action);
            }
        }
        // 反向：`NON_COMMAND` 里登记过的动作必须真的在表里出现过，否则这张表会越攒越假。
        for action in NON_COMMAND {
            assert!(
                seen.contains(&action),
                "{action:?} 登记在 NON_COMMAND 里，但菜单里没有它"
            );
        }
        assert_eq!(action_count(), seen.len());
    }

    /// 分隔线不能出现在菜单的**首尾**（`PopupMenu` 会把末尾那条丢掉，
    /// `popup_menu.rs:1497-1498`），也不允许连着两条。
    #[test]
    fn separators_are_between_items() {
        for menu in MENUS {
            let items = menu.items;
            if items.is_empty() {
                continue;
            }
            assert!(
                !matches!(items[0], MenuItem::Separator),
                "{} 以分隔线开头",
                menu.id
            );
            assert!(
                !matches!(items[items.len() - 1], MenuItem::Separator),
                "{} 以分隔线结尾",
                menu.id
            );
            for pair in items.windows(2) {
                assert!(
                    !(matches!(pair[0], MenuItem::Separator)
                        && matches!(pair[1], MenuItem::Separator)),
                    "{} 里有连着两条分隔线",
                    menu.id
                );
            }
        }
    }

    /// 形态映射：默认常驻（维护者口径），`--compact-menu-bar` 才走浮动。
    #[test]
    fn mode_defaults_to_pinned() {
        assert_eq!(mode_for(false), MenuBarMode::Pinned);
        assert_eq!(mode_for(true), MenuBarMode::Compact);
        assert_eq!(MenuBarMode::Pinned.id(), "pinned");
        assert_eq!(MenuBarMode::Compact.id(), "compact");
    }

    /// 主题子菜单必须**只出现一次**（真源唯一的动态项，在「视图」里）。
    #[test]
    fn theme_submenu_appears_exactly_once() {
        let count = MENUS
            .iter()
            .flat_map(|menu| menu.items.iter())
            .filter(|item| matches!(item, MenuItem::Theme))
            .count();
        assert_eq!(count, 1, "主题子菜单只应出现在「视图」里一次");
    }

    /// **顶级菜单的受控单值迁移**：点第二个 → 上一个收起、这个展开；再点自己 → 收起。
    ///
    /// ⚠️ 这一条只用**纯状态**验证（不建 gpui 窗口 / 不要 `App`）：本 crate 的直接依赖是
    /// `gpui-kit`，拿不到 `gpui::TestAppContext` / `#[gpui::test]`，而 `Application::new()`
    /// 在这个平台上也拿不到（本轮实测只有 `new_inaccessible` / `with_platform`）。
    ///
    /// 所以状态迁移被抽成不碰 `App` 的纯函数 [`resolve_menu_toggle`]，这条测试覆盖它；
    /// 渲染路径（下拉面板真的能被 `PopupMenu::build` 出来）与"点击真的改了状态"由
    /// `.artifacts/p7/verify.ps1` 的实机截图 + `S1_MENU_OPEN/CLOSE/RUN` 覆盖。
    #[test]
    fn opening_another_menu_closes_the_previous_one() {
        assert_eq!(MENUS[2].id, "view");
        assert_eq!(MENUS[5].id, "run");

        // 一开始都收起 → 点「视图」。
        let mut state = NavState::Closed;
        state = resolve_menu_toggle(state, 2);
        assert_eq!(state, NavState::Open(2));

        // 点「运行」→ 上一个收起、这个展开（受控单值语义）。
        let switched = resolve_menu_toggle(state, 5);
        assert_eq!(switched, NavState::Open(5));
        assert_ne!(switched, state, "切到别的顶级菜单必须换打开项，不是两个都开");

        // 再点自己 → 收起。
        assert_eq!(resolve_menu_toggle(switched, 5), NavState::Closed);

        // 收起之后再点「文件」→ 打开；越界下标不动状态。
        let opened = resolve_menu_toggle(NavState::Closed, 0);
        assert_eq!(opened, NavState::Open(0));
        assert_eq!(resolve_menu_toggle(opened, MENUS.len()), opened);

        // 未知 id 不静默（`--menu-probe` 的入口靠它报错）。
        assert!(MENUS.iter().any(|menu| menu.id == "terminal"));
        assert!(!MENUS.iter().any(|menu| menu.id == "nope"));
    }
}
