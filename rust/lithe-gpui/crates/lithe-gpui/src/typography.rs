//! 排版：标题、正文、行内代码。字号走 rem 刻度，随窗口缩放。

use gpui_kit::TestSupportExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, SharedString, Window, div, rems};

use crate::ActiveTheme;

/// 标题层级。Display 用于页面主标题，H1–H4 逐级递减。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeadingLevel {
    Display,
    #[default]
    H1,
    H2,
    H3,
    H4,
}

impl HeadingLevel {
    fn size_rems(self) -> f32 {
        match self {
            Self::Display => 2.25,
            Self::H1 => 1.875,
            Self::H2 => 1.5,
            Self::H3 => 1.25,
            Self::H4 => 1.0,
        }
    }
}

/// 标题。语义层级由 [`HeadingLevel`] 表达，视觉由主题统一。
#[derive(IntoElement)]
pub struct Heading {
    id: ElementId,
    label: SharedString,
    level: HeadingLevel,
}

impl Heading {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            level: HeadingLevel::H1,
        }
    }

    pub fn level(mut self, level: HeadingLevel) -> Self {
        self.level = level;
        self
    }

    pub fn display(mut self) -> Self {
        self.level = HeadingLevel::Display;
        self
    }

    pub fn h2(mut self) -> Self {
        self.level = HeadingLevel::H2;
        self
    }

    pub fn h3(mut self) -> Self {
        self.level = HeadingLevel::H3;
        self
    }

    pub fn h4(mut self) -> Self {
        self.level = HeadingLevel::H4;
        self
    }
}

impl RenderOnce for Heading {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .test_support()
            .text_size(rems(self.level.size_rems()))
            .font_weight(gpui_kit::FontWeight::MEDIUM)
            .text_color(cx.theme().foreground)
            .child(self.label)
    }
}

/// 正文段落。变体只表达"信息权重"：普通、次要、弱化。
#[derive(IntoElement)]
pub struct Text {
    id: ElementId,
    content: SharedString,
    muted: bool,
    subtle: bool,
    small: bool,
}

impl Text {
    pub fn new(id: impl Into<ElementId>, content: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            muted: false,
            subtle: false,
            small: false,
        }
    }

    pub fn muted(mut self) -> Self {
        self.muted = true;
        self
    }

    pub fn subtle(mut self) -> Self {
        self.subtle = true;
        self
    }

    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }
}

impl RenderOnce for Text {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let color = if self.subtle {
            theme.subtle_foreground
        } else if self.muted {
            theme.muted_foreground
        } else {
            theme.foreground
        };
        div()
            .id(self.id)
            .test_support()
            .when(self.small, |el| el.text_sm())
            .text_color(color)
            .child(self.content)
    }
}

/// 行内代码。
#[derive(IntoElement)]
pub struct Code {
    id: ElementId,
    content: SharedString,
}

impl Code {
    pub fn new(id: impl Into<ElementId>, content: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
        }
    }
}

impl RenderOnce for Code {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .px(rems(0.375))
            .rounded(theme.radius_sm)
            .bg(theme.sunken)
            .text_xs()
            .text_color(theme.foreground)
            .font_family(theme.mono_font_family.clone())
            .child(self.content)
    }
}
