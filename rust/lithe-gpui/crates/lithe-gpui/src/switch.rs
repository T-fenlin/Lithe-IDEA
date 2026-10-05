//! 开关（第 6 章）。交互根复用 headless `Button`；轨道与滑块自绘，
//! checked 为受控值，`on_change` 报告意图。

use std::rc::Rc;

use gpui_kit::base::Button as BaseButton;
use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, Styled, Window, div, px, rems};

use crate::ActiveTheme;

/// 开关。
#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    checked: bool,
    disabled: bool,
    on_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    style: gpui_kit::StyleRefinement,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            disabled: false,
            on_change: None,
            style: Default::default(),
        }
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

impl Styled for Switch {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Switch {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let checked = self.checked;
        let track_bg = if checked { theme.primary } else { theme.active };
        let thumb_bg = if checked {
            theme.primary_foreground
        } else {
            theme.muted_foreground
        };
        let focus_color = theme.focus;

        let mut button = BaseButton::new(self.id)
            .disabled(self.disabled)
            .rounded_full()
            .bg(theme.transparent)
            .px(rems(0.0))
            .when(theme.focus_ring, |el| {
                el.focus_visible(move |style| style.border_color(focus_color))
            })
            .refine_style(&self.style)
            .child(
                div()
                    .w(rems(2.25))
                    .h(rems(1.25))
                    .flex_shrink_0()
                    .rounded_full()
                    .bg(track_bg)
                    .relative()
                    .child(
                        div()
                            .absolute()
                            .size(rems(1.0))
                            .top(px(2.0))
                            .left(rems(if checked { 1.125 } else { 0.125 }))
                            .rounded_full()
                            .bg(thumb_bg),
                    ),
            );

        if let Some(handler) = self.on_change {
            button = button.on_click(move |_, window, cx| handler(&!checked, window, cx));
        }
        button
    }
}
