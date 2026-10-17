use gpui::{
    AnyElement, App, Entity, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image};
use lithe_gpui_shared::tr;
use lithe_ui::layout::{h_stack, v_stack};
// 标题栏整块是组件库的：窗口键、拖拽、双击最大化都要按平台分（macOS 系统红黄绿 /
// Windows 方角 / Linux 圆钮），本侧不再自己画一份。
use lithe_ui::shell::TitleBar;
use lithe_ui::theme::ActiveTheme as _;

use crate::menu_bar::{MenuAction, MenuBarMode, push_show_run_pane};

const RUN_SLOT_HEIGHT_SPEC: f32 = 30.;

const RUN_SLOT_MIN_WIDTH_SPEC: f32 = 160.;

const RUN_SLOT_MAX_WIDTH_SPEC: f32 = 190.;

const TITLE_BAR_CONTROL_RADIUS_SPEC: f32 = lithe_gpui_shared::design::CONTROL_RADIUS;

/// 标题栏。窗口键、拖拽、双击最大化都是**平台相关**的，所以整块交给组件库的
/// [`TitleBar`]：macOS 走系统红黄绿（`TitleBar::window_options` 已经把
/// `traffic_light_position` 设好，库负责在左侧让出 `traffic_light_inset()`），
/// Windows 画方角按钮（最小化 / 最大化·还原 / 关闭，图标取库的 `IconName`），
/// Linux 画圆钮且按窗口自己报的能力决定显示哪几个。本侧只决定两边放什么。
pub fn title_bar(
    menu_bar: AnyElement,
    project_menu: AnyElement,
    branch_item: AnyElement,
    run: Entity<lithe_gpui_run::RunPane>,
    mode: MenuBarMode,
    cx: &App,
) -> AnyElement {
    // 菜单 / 工程 / 分支这一簇保留原来的字号与前景色：库只画标题栏的底色与边界，
    // 不替子元素定文字样式。
    let muted = cx.theme().colors.fg_muted;
    let leading = || {
        div()
            .flex()
            .items_center()
            .gap_1()
            .text_sm()
            .text_color(muted)
    };

    let bar = |leading: gpui::Div| {
        TitleBar::new("lithe-title-bar")
            .child(leading)
            .action(run_controls(run.clone(), cx))
            .action(account_slot())
    };

    // Above / AutoHide 两档的菜单**不是标题栏这一行里的东西**（见 menu_bar.rs：前者是
    // 独立一行、自带下边框；后者是一条只在鼠标压到窗口顶边才浮出的热区）。把它们塞进
    // 标题栏的 flex 行里，AutoHide 的 w_full 热区会把同行兄弟挤走。所以这两档压在上面。
    match mode {
        MenuBarMode::Merged | MenuBarMode::Compact => bar(leading()
            .child(menu_bar)
            .child(project_menu)
            .child(branch_item))
        .into_any_element(),
        MenuBarMode::Above | MenuBarMode::AutoHide => v_stack()
            .w_full()
            .flex_shrink_0()
            .child(menu_bar)
            .child(bar(leading().child(project_menu).child(branch_item)))
            .into_any_element(),
    }
}

fn run_controls(run: Entity<lithe_gpui_run::RunPane>, cx: &App) -> impl IntoElement {
    h_stack()
        .flex_shrink_0()
        .h_full()
        .gap_2()
        .child(run_configuration_slot(run.clone(), cx))
        .child(run_control(RunControl::Run, run.clone(), cx))
        .child(run_control(RunControl::Debug, run, cx))
}

fn account_slot() -> impl IntoElement {
    div().flex_shrink_0().w_7()
}

fn run_configuration_slot(run: Entity<lithe_gpui_run::RunPane>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let idle = theme.colors.fg_muted;
    let hover_bg = theme.colors.accent;
    let hover_fg = theme.colors.fg;
    let label = run
        .read(cx)
        .selected()
        .map(|configuration| SharedString::from(configuration.name.clone()))
        .unwrap_or_else(|| tr("lithe.run.configurations"));

    h_stack()
        .id("title-bar-run-configuration")
        .flex_shrink_0()
        .h(rems(RUN_SLOT_HEIGHT_SPEC / 16.))
        .px_1()
        .gap_2()
        .min_w(rems(RUN_SLOT_MIN_WIDTH_SPEC / 16.))
        .max_w(rems(RUN_SLOT_MAX_WIDTH_SPEC / 16.))
        .rounded(rems(TITLE_BAR_CONTROL_RADIUS_SPEC / 16.))
        .text_color(idle)
        .cursor_pointer()
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        .on_mouse_down(MouseButton::Left, |_event, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_click(|_event, _window, cx| {
            push_show_run_pane(cx);
        })
        .aria_label(tr("lithe.run.configurations"))
        .child(div().min_w_0().truncate().text_xs().child(label))
        // 展开箭头取**本仓**图标集；颜色照旧继承外层 `.text_color(..)`（含 hover 那一档）。
        .child(
            gpui::svg()
                .path(idea::CARET_DOWN_ICON.path(cx.theme().is_dark()))
                .flex_shrink_0()
                .size_2p5(),
        )
}

fn run_control(
    control: RunControl,
    run: Entity<lithe_gpui_run::RunPane>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let hover_bg = theme.colors.accent;

    div()
        .id(control.id())
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size_7()
        .rounded(rems(TITLE_BAR_CONTROL_RADIUS_SPEC / 16.))
        .cursor_pointer()
        .hover(move |style| style.bg(hover_bg))
        .on_mouse_down(MouseButton::Left, |_event, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_click(move |_event, _window, cx| {
            let debug = matches!(control, RunControl::Debug);
            let _ = run.update(cx, |pane, cx| pane.launch(debug, cx));
        })
        .aria_label(tr(control.label_key()))
        .child(control.icon(cx))
}

#[derive(Clone, Copy)]
enum RunControl {
    Run,
    Debug,
}

impl RunControl {
    fn id(self) -> &'static str {
        match self {
            Self::Run => "title-bar-run-control-run",
            Self::Debug => "title-bar-run-control-debug",
        }
    }

    fn icon(self, cx: &App) -> AnyElement {
        match self {
            Self::Run => idea_icon_image(&idea::PLAY_ICON, cx, rems(1.)),
            Self::Debug => idea_icon_image(&idea::BUG_ICON, cx, rems(1.)),
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Self::Run => MenuAction::ShowRunAndDebug.label_key(),
            Self::Debug => "lithe.menu.startDebugging",
        }
    }
}
