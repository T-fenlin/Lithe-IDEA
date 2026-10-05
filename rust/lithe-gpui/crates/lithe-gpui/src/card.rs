//! 卡片与排版容器（第 3 章 Layout）。堆叠直接用 base 的 `h_flex`/`v_flex`。

use gpui_kit::TestSupportExt as _;
use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div, rems,
};

use crate::ActiveTheme;

/// 卡片：surface 底 + 边框 + 大圆角的内容容器。
#[derive(IntoElement)]
pub struct Card {
    id: ElementId,
    children: Vec<AnyElement>,
    style: gpui_kit::StyleRefinement,
}

impl Card {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            children: Vec::new(),
            style: Default::default(),
        }
    }
}

impl Styled for Card {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Card {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .rounded(theme.radius_lg)
            .border_1()
            .border_color(theme.border)
            .bg(theme.surface)
            .p(rems(1.0))
            .flex()
            .flex_col()
            .gap(rems(0.75))
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// 卡片标题行。
#[derive(IntoElement)]
pub struct CardTitle {
    id: ElementId,
    label: gpui_kit::SharedString,
}

impl CardTitle {
    pub fn new(id: impl Into<ElementId>, label: impl Into<gpui_kit::SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

impl RenderOnce for CardTitle {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .test_support()
            .text_base()
            .font_weight(gpui_kit::FontWeight::MEDIUM)
            .text_color(cx.theme().foreground)
            .child(self.label)
    }
}

/// 固定尺寸的弹性间隔。方向由使用处的 flex 主轴决定。
#[derive(IntoElement)]
pub struct Spacer {
    id: ElementId,
    length: gpui_kit::DefiniteLength,
    vertical: bool,
}

impl Spacer {
    pub fn new(id: impl Into<ElementId>, length: gpui_kit::DefiniteLength) -> Self {
        Self {
            id: id.into(),
            length,
            vertical: false,
        }
    }

    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }
}

impl RenderOnce for Spacer {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .test_support()
            .flex_shrink_0()
            .when(self.vertical, |el| el.h(self.length).w_full())
            .when(!self.vertical, |el| el.w(self.length).h_full())
    }
}

/// 内容区标题组：标题 + 可选说明文字。
#[derive(IntoElement)]
pub struct SectionHeader {
    id: ElementId,
    title: gpui_kit::SharedString,
    description: Option<gpui_kit::SharedString>,
}

impl SectionHeader {
    pub fn new(id: impl Into<ElementId>, title: impl Into<gpui_kit::SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
        }
    }

    pub fn description(mut self, description: impl Into<gpui_kit::SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl RenderOnce for SectionHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .flex()
            .flex_col()
            .gap(rems(0.25))
            .child(
                div()
                    .text_lg()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(theme.foreground)
                    .child(self.title),
            )
            .children(
                self.description
                    .map(|d| div().text_sm().text_color(theme.muted_foreground).child(d)),
            )
    }
}
