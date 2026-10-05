//! 链接（第 5 章）。仅用于外部 URL/邮件（产品约定：应用内跳转走导航组件，
//! 命令走 Button）。打开方式保留 base 的 `open_with` 缝，默认交给系统。

use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window};

use crate::ActiveTheme;

/// 外部链接。
#[derive(IntoElement)]
pub struct Link {
    id: ElementId,
    label: SharedString,
    href: SharedString,
    disabled: bool,
    style: gpui_kit::StyleRefinement,
}

impl Link {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        href: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            href: href.into(),
            disabled: false,
            style: Default::default(),
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for Link {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Link {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (fg, hover_fg) = if self.disabled {
            (theme.disabled_foreground, theme.disabled_foreground)
        } else {
            (theme.link, theme.link_hover)
        };
        gpui_kit::base::Link::new(self.id)
            .href(self.href)
            .when(self.disabled, |el| el.disabled(true))
            .text_sm()
            .text_color(fg)
            .hover(move |style| style.text_color(hover_fg).underline())
            .refine_style(&self.style)
            .child(self.label)
    }
}
