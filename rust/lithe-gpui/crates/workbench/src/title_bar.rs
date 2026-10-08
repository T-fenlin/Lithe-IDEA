use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};
use gpui::{AnyElement, App, Entity, Hsla, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, rems};
use gpui::{WindowControlArea};
use lithe_gpui_shared::icons::{idea, idea_icon_image};

use lithe_gpui_shared::tr;

use crate::menu_bar::{MenuAction, push_show_run_pane};

const WINDOW_CONTROL_WIDTH_SPEC: f32 = 44.;

const TITLE_BAR_HEIGHT_SPEC: f32 = 40.;

const RUN_SLOT_HEIGHT_SPEC: f32 = 30.;

const RUN_SLOT_MIN_WIDTH_SPEC: f32 = 160.;

const RUN_SLOT_MAX_WIDTH_SPEC: f32 = 190.;

const TITLE_BAR_CONTROL_RADIUS_SPEC: f32 = lithe_gpui_shared::design::CONTROL_RADIUS;

pub fn title_bar(
    menu_bar: AnyElement,
    project_menu: AnyElement,
    branch_item: AnyElement,
    run: Entity<lithe_gpui_run::RunPane>,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    v_flex()
        .flex_shrink_0()
        .w_full()
        .bg(cx.theme().title_bar)
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(
            h_flex()
                .w_full()
                .h(rems(TITLE_BAR_HEIGHT_SPEC / 16.))
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .gap_1()
                        .px_2()
                        .child(menu_bar)
                        .child(project_menu)
                        .child(branch_item)
                        .child(drag_region())
                        .child(run_controls(run, cx))
                        .child(account_slot()),
                )
                .child(window_controls(window, cx)),
        )
}

fn drag_region() -> impl IntoElement {
    h_flex()
        .flex_1()
        .min_w_0()
        .h_full()
        .window_control_area(WindowControlArea::Drag)
        .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
}

fn window_controls(window: &Window, cx: &App) -> impl IntoElement {
    h_flex()
        .flex_shrink_0()
        .h_full()
        .gap_0()
        .child(window_control(WindowControl::Minimize, cx))
        .child(window_control(
            if window.is_maximized() {
                WindowControl::Restore
            } else {
                WindowControl::Maximize
            },
            cx,
        ))
        .child(window_control(WindowControl::Close, cx))
}

fn window_control(control: WindowControl, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let hover_background = control.hover_background(cx);
    let hover_foreground = control.hover_foreground(cx);

    div()
        .id(control.id())
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .w(rems(WINDOW_CONTROL_WIDTH_SPEC / 16.))
        .h_full()
        .rounded_none()
        .text_color(if control.is_close() {
            theme.foreground
        } else {
            theme.muted_foreground
        })
        .window_control_area(control.area())
        .hover(move |style| style.bg(hover_background).text_color(hover_foreground))
        .child(Icon::new(control.icon()).small())
}

#[derive(Clone, Copy)]
enum WindowControl {
    Minimize,
    Maximize,
    Restore,
    Close,
}

impl WindowControl {
    fn id(self) -> &'static str {
        match self {
            Self::Minimize => "title-bar-window-minimize",
            Self::Maximize => "title-bar-window-maximize",
            Self::Restore => "title-bar-window-restore",
            Self::Close => "title-bar-window-close",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Self::Minimize => IconName::Minus,
            Self::Maximize => IconName::Square,
            Self::Restore => IconName::Copy,
            Self::Close => IconName::X,
        }
    }

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

    fn hover_background(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        if self.is_close() {
            theme.danger
        } else {
            theme.accent
        }
    }

    fn hover_foreground(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        if self.is_close() {
            theme.danger_foreground
        } else {
            theme.foreground
        }
    }
}

fn run_controls(run: Entity<lithe_gpui_run::RunPane>, cx: &App) -> impl IntoElement {
    h_flex()
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
    let idle = theme.muted_foreground;
    let hover_bg = theme.accent;
    let hover_fg = theme.foreground;
    let label = run
        .read(cx)
        .selected()
        .map(|configuration| SharedString::from(configuration.name.clone()))
        .unwrap_or_else(|| tr("lithe.run.configurations"));

    h_flex()
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
        .child(Icon::new(IconName::ChevronDown).size_2p5())
}

fn run_control(
    control: RunControl,
    run: Entity<lithe_gpui_run::RunPane>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let hover_bg = theme.accent;

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
            Self::Run => idea_icon_image(&idea::PLAY_ICON, cx, rems(16. / 16.)),
            Self::Debug => idea_icon_image(&idea::BUG_ICON, cx, rems(16. / 16.)),
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Self::Run => MenuAction::ShowRunAndDebug.label_key(),
            Self::Debug => "lithe.menu.startDebugging",
        }
    }
}
