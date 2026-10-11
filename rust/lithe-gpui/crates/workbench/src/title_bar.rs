use gpui::WindowControlArea;
use gpui::{
    AnyElement, App, Entity, Hsla, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image};
use lithe_gpui_shared::tr;
use lithe_ui::layout::{h_stack, v_stack};
use lithe_ui::theme::ActiveTheme as _;

use crate::menu_bar::{MenuAction, push_show_run_pane};

const WINDOW_CONTROL_WIDTH_SPEC: f32 = 44.;

const WINDOW_CONTROL_ICON_SPEC: f32 = 14.;

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
    let colors = &cx.theme().colors;
    v_stack()
        .flex_shrink_0()
        .w_full()
        .bg(colors.title_bar)
        // 下边框画在这一层（整条 header 的下沿），不在 `merged_bar` 自己身上：
        // `Merged` 模式下菜单栏是这一行里的子块，边框落在它下面会穿到 header 中间去；
        // IDEA 的 header 边界也是沿着整条 header 的下沿走的。
        //
        // `Above` / `AutoHide` 那两档菜单是**独立的一行**、在这一层之外，分隔线由
        // `above_bar()` 自己画（见 `menu_bar.rs`）。
        .border_b_1()
        .border_color(colors.border)
        .text_sm()
        .text_color(colors.fg_muted)
        .child(
            h_stack()
                .w_full()
                .h(rems(TITLE_BAR_HEIGHT_SPEC / 16.))
                .child(
                    h_stack()
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
    h_stack()
        .flex_1()
        .min_w_0()
        .h_full()
        .window_control_area(WindowControlArea::Drag)
        .on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
}

fn window_controls(window: &Window, cx: &App) -> impl IntoElement {
    h_stack()
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
            theme.colors.fg
        } else {
            theme.colors.fg_muted
        })
        .window_control_area(control.area())
        .hover(move |style| style.bg(hover_background).text_color(hover_foreground))
        .child(control.glyph(cx))
}

/// 标题栏按钮的图形：本仓图标集（`assets/icons/idea/**`）里的那一张。
///
/// **颜色刻意不写死**：外层 `window_control` 的 `.text_color(..)`（含 hover 里的那一档）
/// 由继承落到这张图上，与迁移前 `Icon::new(..)` 继承文字色的行为一致。
fn control_glyph(icon: &'static idea::IdeaIcon, cx: &App) -> AnyElement {
    gpui::svg()
        .path(icon.path(cx.theme().is_dark()))
        .flex_shrink_0()
        .w(rems(WINDOW_CONTROL_ICON_SPEC / 16.))
        .h(rems(WINDOW_CONTROL_ICON_SPEC / 16.))
        .into_any_element()
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

    /// 这个按钮画哪张图。
    ///
    /// **本仓图标集里没有「最大化」那张图形** ⇒ 同尺寸**空槽**（留位、不画），
    /// 不拿组件库的图标顶替 —— 维护者口径见
    /// `.agents/notes/proposed/architecture/2026-10-08-unified-gpui-frontend-selection-and-layer-boundaries.md`。
    /// 想让它有图形，正路是往 `assets/icons/idea/**` 补一张我们自己的图并登记 `idea::*` 常量。
    fn glyph(self, cx: &App) -> AnyElement {
        match self {
            Self::Minimize => control_glyph(&idea::MINUS_ICON, cx),
            Self::Maximize => div()
                .flex_none()
                .w(rems(WINDOW_CONTROL_ICON_SPEC / 16.))
                .h(rems(WINDOW_CONTROL_ICON_SPEC / 16.))
                .into_any_element(),
            Self::Restore => control_glyph(&idea::COPY_ICON, cx),
            Self::Close => control_glyph(&idea::X_ICON, cx),
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
            theme.colors.danger
        } else {
            theme.colors.accent
        }
    }

    fn hover_foreground(self, cx: &App) -> Hsla {
        let theme = cx.theme();
        if self.is_close() {
            // 危险底上的前景：与库自身的危险按钮同槽（`tone(Danger).fg`）。
            theme.colors.on_accent
        } else {
            theme.colors.fg
        }
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
