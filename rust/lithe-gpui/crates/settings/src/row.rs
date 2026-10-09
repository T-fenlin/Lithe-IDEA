use gpui::{
    AnyElement, App, ClickEvent, ElementId, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    rems,
};
use gpui::{Div, prelude::FluentBuilder as _};
use lithe_ui::layout::{h_stack, v_stack};
use lithe_ui::theme::{ActiveTheme as _, Radius};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlWidth {
    Default,
    Number,
}

impl ControlWidth {
    pub fn rems(self) -> gpui::Rems {
        match self {
            Self::Default => rems(144. / 16.),
            Self::Number => rems(112. / 16.),
        }
    }

    pub fn length(self) -> gpui::Length {
        self.rems().into()
    }
}

pub type RowActivation = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

pub fn settings_group(title: SharedString, rows: Vec<AnyElement>, cx: &App) -> impl IntoElement {
    v_stack()
        .w_full()
        .rounded(cx.theme().radius(Radius::Md))
        .border_1()
        .border_color(cx.theme().colors.border)
        .bg(cx.theme().colors.overlay.opacity(0.35))
        .overflow_hidden()
        .child(
            h_stack()
                .w_full()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(cx.theme().colors.border)
                .text_sm()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(cx.theme().colors.fg_muted)
                .child(title),
        )
        .child(v_stack().w_full().gap_3().p_3().children(rows))
}

pub fn settings_row(
    id: impl Into<ElementId>,
    label: SharedString,
    description: Option<SharedString>,
    control: AnyElement,
    on_activate: Option<RowActivation>,
    cx: &App,
) -> AnyElement {
    settings_row_with_note(id, label, description, None, control, on_activate, cx)
}

pub fn settings_row_with_note(
    id: impl Into<ElementId>,
    label: SharedString,
    description: Option<SharedString>,
    note: Option<AnyElement>,
    control: AnyElement,
    on_activate: Option<RowActivation>,
    cx: &App,
) -> AnyElement {
    h_stack()
        .id(id)
        .w_full()
        .min_h_8()
        .items_center()
        .gap_4()
        .when_some(on_activate, |this, on_activate| {
            this.on_click(move |event, window, cx| on_activate(event, window, cx))
        })
        .child(
            v_stack()
                .min_w_0()
                .flex_1()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().colors.fg)
                        .child(label),
                )
                .children(description.map(|description| {
                    div()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(description)
                }))
                .children(note),
        )
        .child(div().flex_shrink_0().child(control))
        .into_any_element()
}

pub fn appearance_source_note(text: SharedString, revert: AnyElement, cx: &App) -> AnyElement {
    h_stack()
        .w_full()
        .min_w_0()
        .items_center()
        .gap_2()
        .child(
            div()
                .min_w_0()
                .text_xs()
                .text_color(cx.theme().colors.fg_muted)
                .child(text),
        )
        .child(div().flex_shrink_0().child(revert))
        .into_any_element()
}

pub fn page_title(title: SharedString) -> impl IntoElement {
    div()
        .w_full()
        .mb_5()
        .text_xl()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(title)
}

pub fn page_stack() -> Div {
    v_stack().w_full().gap_4()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_widths_are_the_windows_values() {
        let to_px = |width: ControlWidth, rem: f32| {
            gpui::AbsoluteLength::from(width.rems()).to_pixels(gpui::px(rem))
        };

        assert_eq!(to_px(ControlWidth::Default, 16.), gpui::px(144.));
        assert_eq!(to_px(ControlWidth::Number, 16.), gpui::px(112.));
        assert_eq!(to_px(ControlWidth::Default, 32.), gpui::px(288.));
    }
}
