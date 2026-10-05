//! 按钮（第 5 章）。行为完全复用 gpui-base 的 headless `Button`
//! （焦点、Tab、键盘激活、禁用拦截），本层只提供 Ely 视觉与变体 API。

use std::rc::Rc;

use gpui_kit::base::Button as BaseButton;
use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, IntoElement, ParentElement, RenderOnce, SharedString,
    Styled, Window, px,
};

use crate::{ActiveTheme, Sizable, Size};

/// 按钮变体。语义保持稀缺：主操作 Primary，危险 Danger，其余按权重选。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    #[default]
    Secondary,
    Outline,
    Ghost,
    Danger,
}

/// 按钮。`on_click` 只报告意图；loading/进行中状态由调用方表达。
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    variant: ButtonVariant,
    size: Size,
    selected: bool,
    disabled: bool,
    style: gpui_kit::StyleRefinement,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            children: Vec::new(),
            on_click: None,
            variant: ButtonVariant::Secondary,
            size: Size::Medium,
            selected: false,
            disabled: false,
            style: Default::default(),
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
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

    pub fn outline(mut self) -> Self {
        self.variant = ButtonVariant::Outline;
        self
    }

    pub fn ghost(mut self) -> Self {
        self.variant = ButtonVariant::Ghost;
        self
    }

    pub fn danger(mut self) -> Self {
        self.variant = ButtonVariant::Danger;
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
}

impl Styled for Button {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for Button {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

/// 控件几何：高度与水平内边距随 rem 缩放（Ely 控件刻度 20/24/28/32px）。
fn control_geometry(size: Size, rem_scale: f32) -> (gpui_kit::Pixels, gpui_kit::Pixels) {
    let (height, padding_x) = match size {
        Size::Size(height) => (f32::from(height), 12.0),
        Size::XSmall => (20.0, 8.0),
        Size::Small => (24.0, 10.0),
        Size::Medium => (28.0, 12.0),
        Size::Large => (32.0, 14.0),
    };
    (px(height * rem_scale), px(padding_x * rem_scale))
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let rem_scale = f32::from(window.rem_size()) / 16.0;
        let (height, padding_x) = control_geometry(self.size, rem_scale);
        let (bg, fg, hover_bg, active_bg, bordered) = match self.variant {
            ButtonVariant::Primary => (
                theme.primary,
                theme.primary_foreground,
                theme.primary_hover,
                theme.primary_active,
                false,
            ),
            ButtonVariant::Secondary => (
                theme.secondary,
                theme.secondary_foreground,
                theme.secondary_hover,
                theme.secondary_active,
                false,
            ),
            ButtonVariant::Outline => (
                theme.surface,
                theme.foreground,
                theme.hover,
                theme.active,
                true,
            ),
            // Ghost 的静止态不占色，靠 hover 表达可点。
            ButtonVariant::Ghost => (
                theme.transparent,
                theme.foreground,
                theme.hover,
                theme.active,
                false,
            ),
            ButtonVariant::Danger => (
                theme.danger,
                theme.danger_foreground,
                theme.danger_hover,
                theme.danger_active,
                false,
            ),
        };
        let selected_bg = theme.accent;
        let selected_fg = theme.accent_foreground;
        let disabled_fg = theme.disabled_foreground;
        let hover_bg = hover_bg;
        let active_bg = active_bg;
        let focus_color = theme.focus;
        let focus_ring = theme.focus_ring;

        let mut button = BaseButton::new(self.id)
            .disabled(self.disabled)
            .selected(self.selected)
            .h(height)
            .px(padding_x)
            .rounded(theme.radius)
            .bg(bg)
            .text_color(fg)
            .text_sm()
            .when(bordered, |el| {
                el.border_1().border_color(theme.border_strong)
            })
            .when(!self.disabled, |el| {
                el.hover(move |style| style.bg(hover_bg))
                    .active(move |style| style.bg(active_bg))
            })
            .when(focus_ring, |el| {
                el.focus_visible(move |style| style.border_color(focus_color))
            })
            .styles(|styles| {
                styles
                    .selected(|style| style.bg(selected_bg).text_color(selected_fg))
                    .disabled(|style| style.text_color(disabled_fg))
            })
            .refine_style(&self.style);

        if let Some(handler) = self.on_click {
            button = button.on_click(move |event, window, cx| handler(event, window, cx));
        }
        if let Some(label) = self.label {
            button = button.child(label);
        }
        button.children(self.children)
    }
}
