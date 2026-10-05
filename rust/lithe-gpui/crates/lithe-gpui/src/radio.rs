//! 单选（第 6 章）。交互根复用 headless `Button`；圆框与选中点自绘。
//! 分组语义由调用方持有（同一 `selected` 值的一组 Radio）。

use std::rc::Rc;

use gpui_kit::base::Button as BaseButton;
use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, Styled, Window, div, rems};

use crate::ActiveTheme;

/// 单选项。
#[derive(IntoElement)]
pub struct Radio {
    id: ElementId,
    label: Option<gpui_kit::SharedString>,
    selected: bool,
    disabled: bool,
    on_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    style: gpui_kit::StyleRefinement,
}

impl Radio {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            selected: false,
            disabled: false,
            on_change: None,
            style: Default::default(),
        }
    }

    pub fn label(mut self, label: impl Into<gpui_kit::SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
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

impl Styled for Radio {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Radio {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let selected = self.selected;
        let (ring, dot) = if selected {
            (theme.primary, theme.primary)
        } else {
            (theme.border_strong, theme.transparent)
        };
        let focus_color = theme.focus;

        let mut button = BaseButton::new(self.id)
            .disabled(self.disabled)
            .rounded(theme.radius_sm)
            .bg(theme.transparent)
            .px(rems(0.0))
            .gap(rems(0.5))
            .text_sm()
            .text_color(theme.foreground)
            .when(theme.focus_ring, |el| {
                el.focus_visible(move |style| style.border_color(focus_color))
            })
            .refine_style(&self.style)
            .child(
                div()
                    .size(rems(1.0))
                    .flex_shrink_0()
                    .rounded_full()
                    .border_1()
                    .border_color(ring)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(div().size(rems(0.5)).rounded_full().bg(dot)),
            );

        if let Some(label) = self.label {
            button = button.child(label);
        }
        if let Some(handler) = self.on_change {
            button = button.on_click(move |_, window, cx| handler(&!selected, window, cx));
        }
        button
    }
}
