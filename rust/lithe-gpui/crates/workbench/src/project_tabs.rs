use std::rc::Rc;

use gpui::ScrollHandle;
use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image, idea_icon_mono};
use lithe_gpui_shared::tr_args;
use lithe_ui::buttons::{Button, ButtonVariant};
use lithe_ui::layout::h_stack;
use lithe_ui::theme::{ActiveTheme as _, Size};

const STRIP_HEIGHT_SPEC: f32 = 38.;

const TAB_HEIGHT_SPEC: f32 = 30.;

const TAB_MIN_WIDTH_SPEC: f32 = 180.;

const TAB_PADDING_SPEC: f32 = 38.;

const CLOSE_BUTTON_TRAILING_SPEC: f32 = 3.;

const FOLDER_ICON_SIZE_SPEC: f32 = 11.;

const TAB_UNDERLINE_WIDTH_SPEC: f32 = 28.;
const TAB_UNDERLINE_BOTTOM_SPEC: f32 = 1.;

const CLOSE_BUTTON_SIZE_SPEC: f32 = 28.;

const CLOSE_GROUP: &str = "project-tab-group";

#[non_exhaustive]
pub struct ProjectTab {
    name: SharedString,
}

impl ProjectTab {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self { name: name.into() }
    }

    pub fn name(&self) -> &SharedString {
        &self.name
    }
}

pub fn project_tabs(
    tabs: &[ProjectTab],
    active: Option<usize>,
    scroll: &ScrollHandle,
    on_activate: impl Fn(usize, &mut Window, &mut App) + 'static,
    on_close: impl Fn(usize, &mut Window, &mut App) + 'static,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    let rem = window.rem_size();
    let on_activate: Rc<dyn Fn(usize, &mut Window, &mut App)> = Rc::new(on_activate);
    let on_close: Rc<dyn Fn(usize, &mut Window, &mut App)> = Rc::new(on_close);

    let colors = &cx.theme().colors;
    let strip_bg = colors.tab_bar;
    let border = colors.border;
    let accent = colors.accent;
    let inactive_bg = colors.surface;
    let foreground = colors.fg;
    let subtle_foreground = colors.fg_muted;
    // 选中标签底部那条下划线画在 `accent` 底上 ⇒ 取 `on_accent`。
    let primary = colors.on_accent;
    let focus_border = colors.focus;

    h_stack()
        .w_full()
        .h(rems(STRIP_HEIGHT_SPEC / 16.))
        .flex_shrink_0()
        .min_w_0()
        .items_center()
        .bg(strip_bg)
        .border_b_1()
        .border_color(border)
        .child(
            h_stack()
                .id("project-tabs-scroll")
                .flex_1()
                .min_w_0()
                .h_full()
                .items_center()
                .overflow_x_scroll()
                .track_scroll(scroll)
                .child(
                    h_stack()
                        .h_full()
                        .items_center()
                        .gap_1p5()
                        .px_1p5()
                        .children(tabs.iter().enumerate().map(|(index, tab)| {
                            let is_active = active == Some(index);
                            let name = tab.name().clone();
                            let tooltip = tab.name().clone();

                            let activate = on_activate.clone();
                            let close = on_close.clone();

                            div()
                                .relative()
                                .flex_1()
                                .flex_shrink_0()
                                .min_w(rems(TAB_MIN_WIDTH_SPEC / 16.))
                                .h(rems(TAB_HEIGHT_SPEC / 16.))
                                .child({
                                    let mut label = h_stack()
                                        .id(SharedString::from(format!("project-tab-{name}")))
                                        .relative()
                                        .size_full()
                                        .items_center()
                                        .gap_1p5()
                                        .px(rems(TAB_PADDING_SPEC / 16.))
                                        .rounded_md()
                                        .text_sm()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .cursor_pointer()
                                        .group(CLOSE_GROUP)
                                        .on_click(move |_, window, cx| {
                                            if !is_active {
                                                activate(index, window, cx);
                                            }
                                        })
                                        .child(idea_icon_image(
                                            &idea::FOLDER_ICON,
                                            cx,
                                            rems(FOLDER_ICON_SIZE_SPEC / 16.),
                                        ))
                                        .child(
                                            div()
                                                .min_w_0()
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .text_ellipsis()
                                                .text_color(if is_active {
                                                    foreground
                                                } else {
                                                    subtle_foreground
                                                })
                                                .child(name),
                                        );

                                    if is_active {
                                        label = label.bg(accent);
                                    } else {
                                        label = label
                                            .bg(inactive_bg)
                                            .hover(move |this| this.bg(accent));
                                    }

                                    if is_active {
                                        label = label.child(
                                            div()
                                                .absolute()
                                                .bottom(rems(TAB_UNDERLINE_BOTTOM_SPEC / 16.))
                                                .left_0()
                                                .right_0()
                                                .flex()
                                                .justify_center()
                                                .child(
                                                    div()
                                                        .w(rems(TAB_UNDERLINE_WIDTH_SPEC / 16.))
                                                        .h_0p5()
                                                        .rounded_full()
                                                        .bg(primary),
                                                ),
                                        );
                                    }

                                    label
                                })
                                .when(is_active, |this| {
                                    this.child(
                                        div()
                                            .absolute()
                                            .inset_0()
                                            .rounded_md()
                                            .border_1()
                                            .border_color(focus_border.opacity(0.7)),
                                    )
                                })
                                .child(
                                    h_stack()
                                        .absolute()
                                        .top_0()
                                        .bottom_0()
                                        .right(rems(CLOSE_BUTTON_TRAILING_SPEC / 16.))
                                        .items_center()
                                        .when(!is_active, |this| {
                                            this.opacity(0.)
                                                .group_hover(CLOSE_GROUP, |this| this.opacity(1.))
                                        })
                                        .child(
                                            Button::new(SharedString::from(format!(
                                                "project-tab-close-{tooltip}"
                                            )))
                                            // 关闭用的叉来自**本仓**图标集（`icons/idea/**`）。
                                            .icon_element(idea_icon_mono(
                                                &idea::X_ICON,
                                                cx.theme().is_dark(),
                                            ))
                                            .variant(ButtonVariant::Ghost)
                                            .size(Size::Size(crate::rem_px(
                                                rem,
                                                CLOSE_BUTTON_SIZE_SPEC,
                                            )))
                                            .tooltip(tr_args(
                                                "lithe.titleProject.closeProject",
                                                &[("name", tooltip.as_ref())],
                                            ))
                                            .on_click(move |_, window, cx| {
                                                close(index, window, cx);
                                            }),
                                        ),
                                )
                        })),
                ),
        )
}
