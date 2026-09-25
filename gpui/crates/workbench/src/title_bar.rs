//! 标题栏区域（Windows 规格）。
//!
//! 规格真源（一手源码，逐条核对过行号）：
//!
//! - 容器：`windows/tauri/src/features/window/components/title-bar/title-bar.tsx:332-360`
//!   —— `h-(--lithe-title-bar-height)` = **40**、`items-center justify-between`、
//!   `gap-(--lithe-chrome-gap)` = **4**、`px-(--lithe-chrome-padding-inline)` = **8**、
//!   `bg-surface`、`text-muted-foreground`、`ui-text-chrome` = **13px**。
//! - 窗口三键：`.../title-bar/window-controls.tsx:51-96` —— 顺序「最小化 → 最大化/还原 → 关闭」，
//!   每个 `h-full w-14 min-w-14 rounded-none` = **56×40、直角**；组内 `gap="none"` = 0；
//!   最小化/最大化 `variant=ghost`、关闭 `variant=danger` + `hover:bg-destructive hover:text-white`。
//!   图标尺寸 `windows/tauri/src/ui/button.tsx:9` 的 `[&_svg:not([class*='size-'])]:size-3.5` = **14×14**。
//! - 高度 / 间距 / 内边距 / 字号令牌：`windows/tauri/src/styles/theme.css:115,118,126-133`。
//! - 中文文案：`windows/tauri/src/i18n/locale.ts:6191`（`titleProject.trigger` = `项目：{project}`，
//!   本区域按可见规格只显示项目名本身，见 `title_bar` 的文档）。
//!
//! ## 左侧的菜单栏（见模块 [`crate::menu_bar`]）与项目下拉（见 [`crate::project_menu`]）
//!
//! 真源把菜单栏画在**标题栏这一行里**（常驻形态，`title-bar.tsx:230-232`）或标题栏左上角的
//! `ListIcon` 按钮 + 一行浮动胶囊（紧凑形态，`:207-229`）。两种形态都走
//! [`crate::menu_bar::menu_bar`]，本文件只负责给它一个位置。
//!
//! 项目名下拉是左侧组的第二项：真源 `ChromeGroup grow min-w-0 { menuItem, projectControls }`
//! （`title-bar.tsx:339-343`）—— **菜单栏在前、项目下拉在后**，两者都在拖拽区左边。
//! 项目名本身**画在触发器里**（`title-project-menu.tsx:157`），所以本文件的拖拽区不再重复画一遍。
//!
//! ⚠️ **菜单栏与项目下拉都必须是 `drag_region` 的兄弟节点**：Windows 的命中测试取
//! `window_control_hitboxes` 里第一个命中项（`gpui-pre-0.3.6/src/window.rs:1952-1956`，
//! 按绘制顺序 = 祖先在前），祖先的 `Drag` 会赢 —— 把它们放进 `drag_region` 里面，
//! 点它们只会拖窗口。这条与下面窗口三键的坑同源：**`Drag` 不能做任何可点元素的祖先**。
//!
//! 为什么**不用** gpui-kit 的 `component::TitleBar`：
//!
//! 1. 它内部**无条件**再画一组自带窗口三键（`gpui-component-0.6.6/src/title_bar.rs:247-294`
//!    的 `WindowControls`，:400-402 挂进渲染树），每个控件硬编码宽 `TITLE_BAR_HEIGHT` = 34px
//!    （:15、:211）且不可定制 —— 会和本文件自绘的 56×40 三键叠成两组按钮；
//! 2. 它给可定制子元素套的是 `h_flex().flex_1()`（:371-378），外层自带 `pl(12)`（:19、:336）
//!    与 1px 下边框 `border_b_1()`（:337），与 Windows 标题栏（左右内边距 8、无边框）不一致。
//!
//! 因此这里用普通 `div()` 自建容器与三键，拖拽区用 gpui 的 `WindowControlArea`。
//! 窗口三键**不挂任何点击回调**：Windows 平台靠 `WindowControlArea` 命中测试把这三块区域
//! 交给系统（`gpui-pre-windows-0.3.6/src/events.rs:976-984`），而 gpui 侧一旦有监听器消费掉
//! 按下事件就会 return `Some(0)` 并让系统收不到该次点击（同文件 `:1078-1095`）——
//! gpui-kit 自己的三键也是这个结论（`gpui-component-0.6.6/src/title_bar.rs:106-109`、`:220-242`）。

use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};
use gpui_kit::{
    AnyElement, App, Hsla, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    Pixels, Styled as _, Window, WindowControlArea, div, px,
};

// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px（`gpui/themes/*.json` 的 `font.size` + `root.rs:582` 的
// `set_rem_size`），所以 helper 后缀 `N` = `N × 4px`，与 Windows 规格逐像素相等：
//
// | 规格（Windows 真源） | 值 | 用到的 helper |
// | --- | --- | --- |
// | `--lithe-title-bar-height: 2.5rem`（`styles/theme.css:118`） | 40 | `h_10()` |
// | `--lithe-chrome-padding-inline: 8px`（`styles/theme.css:132`） | 8 | `px_2()` |
// | `--lithe-chrome-gap: 4px`（`styles/theme.css:130`） | 4 | `gap_1()` |
// | `--ui-text-chrome: 13px`（`styles/theme.css:115`） | 13 | `text_sm()`（14px，见下） |
//
// ⚠️ 不在此表的 `w-14` = **56** 见下方常量：56 不在 gpui 的 rem 档位上。
//
// 字号：`--ui-text-chrome` 是 **13px**，gpui 的档位只有 `text_xs()`(12) 与 `text_sm()`(14)，
// 13 不在档位上。按《编码指南》这里用 **`text_sm()`（14px）**——Lithe 原基准是 13px，
// 13 → 14 是经维护者确认的**有意**视觉改动（应用里所有 chrome 文字同步 +1px），不是等价换算。
//
// ⚠️ gpui-kit 的 `TitleBar` 默认高度是硬编码的 34px
// （`gpui-component-0.6.6/src/title_bar.rs:15` `TITLE_BAR_HEIGHT`），所以这里必须显式
// `h_10()` 覆盖掉它。

/// 单个窗口控件的宽度：`w-14` = 56px
/// （`windows/tauri/src/features/window/components/title-bar/window-controls.tsx:59,71,85`）。
///
/// ⚠️ **保留 `px(...)`**：56 不在 gpui 的固定 rem 档位上（档位后缀是 `…_11()`=44、`_12()`=48、
/// `_16()`=64，`gpui-pre-macros-0.3.6/src/styles.rs:1003-1017`；没有 `_14()`）。
/// Tailwind 的 `w-14` 是任意档位，gpui 的档位表里没有对应项，不能自己发明 helper。
const WINDOW_CONTROL_WIDTH: Pixels = px(56.);

/// 标题栏（无状态）：只依赖传入的**已渲染好的**菜单栏与项目下拉、当前主题与窗口状态，
/// 不持有 `Entity`。
///
/// `menu_bar` / `project_menu` 是调用方（[`crate::workspace::ShellWorkspace::render`]）先渲染好的
/// 元素，收 `AnyElement` 而不是收 `Entity<..>` + `&mut App` 有三个原因：
///
/// 1. 本 crate 是 edition 2024，`-> impl IntoElement` 会捕获签名里所有在作用域的生命周期
///    （见 `lib.rs` 模块头的"区域渲染函数必须收 `&Window` / `&App`"），多一个 `&mut App`
///    参数会把可变借用带进返回值；
/// 2. 菜单栏的形态（常驻 / 图标）由它自己决定要画哪一种，标题栏不需要知道这件事；
/// 3. 项目下拉要读 `Entity<ProjectMenu>` 并新建 `PopupMenu`（要 `&mut App`），
///    那一步必须发生在调用方，而不是这个只读的区域函数里。
///
/// ⚠️ **项目名不再由本函数画**：Windows 的项目名就在项目下拉的触发器里
/// （`title-project-menu.tsx:157`），所以本函数收的是**已经带名字的触发器元素**，
/// 拖拽区因此只是一块空的剩余宽度（真源 `ChromeGroup grow min-w-0`，`title-bar.tsx:339-343`）。
///
/// ⚠️ **参数顺序不可调换**：`menu_bar` 在左、`project_menu` 在右
/// （真源 `ChromeGroup { menuItem, projectControls }`，`title-bar.tsx:339-343`），
/// 两者都排在拖拽区之前。
///
/// ⚠️ **两者都是 `drag_region` 的兄弟节点，不能放进它内部**：Windows 的命中测试取
/// `window_control_hitboxes` 里**第一个**命中项（`gpui-pre-0.3.6/src/window.rs:1952-1956`，
/// 按绘制顺序 = 祖先在前），祖先的 `Drag` 会赢，那时点它们只会拖窗口。
/// 本条与本文件窗口三键的 `Drag` 祖先坑同源（见下）。
pub fn title_bar(
    menu_bar: AnyElement,
    project_menu: AnyElement,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    h_flex()
        // ChromeBar 基线带 `shrink-0`（`windows/tauri/src/ui/chrome.tsx:6`），标题栏不参与压缩。
        .flex_shrink_0()
        .w_full()
        .h_10()
        .justify_between()
        .gap_1()
        .px_2()
        // `bg-surface`（`title-bar.tsx:337`）：gpui-kit 的 `ThemeColor` **没有** `surface`
        // 这一项，语义上最贴近「标题栏专用底色」的是主题自己的 `title_bar`
        // （`gpui-component-0.6.6/src/theme/theme_color.rs:293`）。
        .bg(cx.theme().title_bar)
        .text_sm()
        // `text-muted-foreground`（`title-bar.tsx:337`）→ `theme.muted_foreground`
        // （`gpui-component-0.6.6/src/theme/theme_color.rs:195`）。
        .text_color(cx.theme().muted_foreground)
        // ① 菜单栏：`drag_region` 的**兄弟**（理由见函数文档）。
        //    常驻形态占标题栏左侧一段（真源 `title-bar.tsx:230-232`），
        //    图标形态只占一个 24×24 按钮 + 一层浮动胶囊。
        .child(menu_bar)
        // ② 项目下拉触发器：同样是 `drag_region` 的**兄弟**（见 [`crate::project_menu`] 模块头）。
        .child(project_menu)
        // ③ 剩下的宽度全部是拖拽区。
        .child(drag_region())
        .child(window_controls(window, cx))
}

/// 拖拽区：标题栏里除菜单栏 / 项目下拉 / 窗口三键之外的**剩余宽度**。
///
/// 由左组独占剩余宽度（`ChromeGroup grow min-w-0`，`title-bar.tsx:339`），三键是它的
/// **兄弟节点**而不是子节点 —— 这一点必须保持：平台命中测试取
/// `window_control_hitboxes` 里**第一个**命中项（`gpui-pre-0.3.6/src/window.rs:1952-1956`，
/// 按绘制顺序 = 祖先在前），若把 `Drag` 挂在整条标题栏的祖先上，
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
/// （`window-controls.tsx:52-55`）—— 三键之间 0 间距、整组满高 40。
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

/// 单个窗口控件：56×40、直角、无点击回调（见文件头说明），只有悬停态。
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
        .w(WINDOW_CONTROL_WIDTH)
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
