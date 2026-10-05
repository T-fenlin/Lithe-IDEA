//! 图标按钮：方形按钮承载一个图标。变体语义与 [`crate::Button`] 一致。

use std::rc::Rc;

use gpui_kit::base::Button as BaseButton;
use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{
    App, ClickEvent, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, px,
};

use crate::{ActiveTheme, Icon, Sizable, Size, button::ButtonVariant};

/// 图标按钮。图标路径由调用方注入（库不内嵌图标集）。
#[derive(IntoElement)]
pub struct IconButton {
    id: ElementId,
    path: gpui_kit::SharedString,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    variant: ButtonVariant,
    size: Size,
    disabled: bool,
    style: gpui_kit::StyleRefinement,
}

impl IconButton {
    pub fn new(id: impl Into<ElementId>, path: impl Into<gpui_kit::SharedString>) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            on_click: None,
            variant: ButtonVariant::Ghost,
            size: Size::Medium,
            disabled: false,
            style: Default::default(),
        }
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    pub fn danger(mut self) -> Self {
        self.variant = ButtonVariant::Danger;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for IconButton {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl Sizable for IconButton {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for IconButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let rem_scale = f32::from(window.rem_size()) / 16.0;
        let edge = match self.size {
            Size::Size(height) => f32::from(height),
            Size::XSmall => 20.0,
            Size::Small => 24.0,
            Size::Medium => 28.0,
            Size::Large => 32.0,
        };
        let (bg, fg, hover_bg, bordered) = match self.variant {
            ButtonVariant::Primary => (
                theme.primary,
                theme.primary_foreground,
                theme.primary_hover,
                false,
            ),
            ButtonVariant::Secondary => (
                theme.secondary,
                theme.secondary_foreground,
                theme.secondary_hover,
                false,
            ),
            ButtonVariant::Outline => (theme.surface, theme.foreground, theme.hover, true),
            ButtonVariant::Ghost => (theme.transparent, theme.foreground, theme.hover, false),
            ButtonVariant::Danger => (
                theme.danger,
                theme.danger_foreground,
                theme.danger_hover,
                false,
            ),
        };
        let disabled_fg = theme.disabled_foreground;
        let focus_color = theme.focus;

        let mut button = BaseButton::new(self.id)
            .disabled(self.disabled)
            .size(px(edge * rem_scale))
            .rounded(theme.radius)
            .bg(bg)
            .text_color(fg)
            .when(bordered, |el| {
                el.border_1().border_color(theme.border_strong)
            })
            .when(!self.disabled, |el| {
                el.hover(move |style| style.bg(hover_bg))
            })
            .when(theme.focus_ring, |el| {
                el.focus_visible(move |style| style.border_color(focus_color))
            })
            .when(self.disabled, |style| style.text_color(disabled_fg))
            .refine_style(&self.style)
            .child(Icon::new("icon", self.path).with_size(self.size));

        if let Some(handler) = self.on_click {
            button = button.on_click(move |event, window, cx| handler(event, window, cx));
        }
        button
    }
}
