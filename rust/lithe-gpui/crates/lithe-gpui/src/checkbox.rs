//! 复选框（第 6 章）。交互根复用 gpui-base 的 headless `Button`
//! （焦点/键盘/禁用），勾选框与标签自绘；受控语义：`checked` 进、
//! `on_change` 报告意图，是否真的翻转由调用方决定。

use std::rc::Rc;

use gpui_kit::base::Button as BaseButton;
use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window, div, rems};

use crate::ActiveTheme;

/// 复选框。
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    label: Option<SharedString>,
    checked: bool,
    disabled: bool,
    on_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    style: gpui_kit::StyleRefinement,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            checked: false,
            disabled: false,
            on_change: None,
            style: Default::default(),
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let checked = self.checked;
        let (box_bg, box_border, box_fg) = if checked {
            (theme.primary, theme.primary, theme.primary_foreground)
        } else {
            (theme.surface, theme.border_strong, theme.transparent)
        };
        let hover_border = theme.muted_foreground;
        let disabled_fg = theme.disabled_foreground;
        let label_color = if self.disabled {
            disabled_fg
        } else {
            theme.foreground
        };
        let focus_color = theme.focus;

        let mut button = BaseButton::new(self.id)
            .disabled(self.disabled)
            .rounded(theme.radius_sm)
            .bg(theme.transparent)
            .px(rems(0.0))
            .gap(rems(0.5))
            .text_sm()
            .text_color(label_color)
            .when(!self.disabled, |el| {
                el.hover(move |style| style.bg(theme.transparent).border_color(hover_border))
            })
            .when(theme.focus_ring, |el| {
                el.focus_visible(move |style| style.border_color(focus_color))
            })
            .refine_style(&self.style)
            .child(
                div()
                    .size(rems(1.0))
                    .flex_shrink_0()
                    .rounded(theme.radius_sm)
                    .border_1()
                    .border_color(box_border)
                    .bg(box_bg)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .text_color(box_fg)
                    .when(checked, |el| el.child("✓")),
            );

        if let Some(label) = self.label {
            button = button.child(label);
        }
        if let Some(handler) = self.on_change {
            button = button.on_click(move |_, window, cx| handler(&!checked, window, cx));
        }
        button
    }
}
