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
    App, Hsla, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Pixels,
    SharedString, Styled as _, Window, WindowControlArea, div, px,
};

/// 标题栏高度：`--lithe-title-bar-height: 2.5rem` = 40px
/// （`windows/tauri/src/styles/theme.css:118`）。
///
/// ⚠️ gpui-kit 的 `TitleBar` 默认高度是硬编码的 34px
/// （`gpui-component-0.6.6/src/title_bar.rs:15` `TITLE_BAR_HEIGHT`），必须显式覆盖。
const TITLE_BAR_HEIGHT: Pixels = px(40.);

/// 标题栏左右内边距：`--lithe-chrome-padding-inline: 8px`
/// （`windows/tauri/src/styles/theme.css:132`）。
const TITLE_BAR_PADDING_INLINE: Pixels = px(8.);

/// 标题栏内元素间距：`--lithe-chrome-gap: 4px`
/// （`windows/tauri/src/styles/theme.css:130`）。
const CHROME_GAP: Pixels = px(4.);

/// 标题栏字号：`--ui-text-chrome: 13px`
/// （`windows/tauri/src/styles/theme.css:115`）。
const CHROME_TEXT_SIZE: Pixels = px(13.);

/// 单个窗口控件的宽度：`w-14` = 56px
/// （`windows/tauri/src/features/window/components/title-bar/window-controls.tsx:59,71,85`）。
const WINDOW_CONTROL_WIDTH: Pixels = px(56.);

/// 标题栏（无状态）：只依赖传入的项目名与当前主题，不持有 `Entity`。
///
/// `project_name` 是**可见标签**：Windows 侧显示的是项目显示名本身
/// （`.../title-bar/title-project-menu.tsx:157`），`项目：{project}` 只是该项的
/// `aria-label`（同文件 `:147`，`windows/tauri/src/i18n/locale.ts:6191`）。
pub fn title_bar(project_name: &str, window: &Window, cx: &App) -> impl IntoElement {
    let project_name: SharedString = project_name.to_owned().into();

    h_flex()
        // ChromeBar 基线带 `shrink-0`（`windows/tauri/src/ui/chrome.tsx:6`），标题栏不参与压缩。
        .flex_shrink_0()
        .w_full()
        .h(TITLE_BAR_HEIGHT)
        .justify_between()
        .gap(CHROME_GAP)
        .px(TITLE_BAR_PADDING_INLINE)
        // `bg-surface`（`title-bar.tsx:337`）：gpui-kit 的 `ThemeColor` **没有** `surface`
        // 这一项，语义上最贴近「标题栏专用底色」的是主题自己的 `title_bar`
        // （`gpui-component-0.6.6/src/theme/theme_color.rs:293`）。
        .bg(cx.theme().title_bar)
        .text_size(CHROME_TEXT_SIZE)
        // `text-muted-foreground`（`title-bar.tsx:337`）→ `theme.muted_foreground`
        // （`gpui-component-0.6.6/src/theme/theme_color.rs:195`）。
        .text_color(cx.theme().muted_foreground)
        .child(drag_region(project_name))
        .child(window_controls(window, cx))
}

/// 左侧「项目名 + 拖拽区」。
///
/// 由左组独占标题栏剩余宽度（`ChromeGroup grow min-w-0`，`title-bar.tsx:339`），
/// 三键是它的**兄弟节点**而不是子节点 —— 这一点必须保持：平台命中测试取
/// `window_control_hitboxes` 里**第一个**命中项（`gpui-pre-0.3.6/src/window.rs:1952-1956`，
/// 按绘制顺序 = 祖先在前），若把 `Drag` 挂在整条标题栏的祖先上，
/// 它会盖住子节点的 `Min` / `Max` / `Close`，三键就永远点不动了
/// （gpui-kit 自己也是把 `Drag` 挂在三键的兄弟 `h_flex` 上：
/// `gpui-component-0.6.6/src/title_bar.rs:371-379`）。
fn drag_region(project_name: SharedString) -> impl IntoElement {
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
        // Windows 侧标签是 `min-w-0 truncate`（`title-project-menu.tsx:157`），
        // 不新增 `max-w-56`（224px 上限属于带 logo 与箭头的下拉触发器整体，
        // `title-project-menu.tsx:146`）。
        .child(
            div()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .child(project_name),
        )
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
