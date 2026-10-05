//! 窗口外壳（第 4 章 Window & Shell）：标题栏、状态栏、侧栏容器。

use gpui_kit::TestSupportExt as _;
use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px,
    rems,
};

use crate::ActiveTheme;

/// 标题栏。高度取 Ely `titlebar_height`（38px @ 16px rem）。
/// macOS 交通灯内嵌属于 app shell 的职责，不在库层处理。
#[derive(IntoElement)]
pub struct TitleBar {
    id: ElementId,
    children: Vec<AnyElement>,
    style: gpui_kit::StyleRefinement,
}

impl TitleBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            children: Vec::new(),
            style: Default::default(),
        }
    }
}

impl Styled for TitleBar {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for TitleBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for TitleBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .h(rems(2.375))
            .px(rems(0.75))
            .flex()
            .items_center()
            .gap(rems(0.5))
            .bg(theme.title_bar)
            .text_color(theme.title_bar_foreground)
            .border_b_1()
            .border_color(theme.border)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// 状态栏。安静底色 + 小字，内容按调用方排布。
#[derive(IntoElement)]
pub struct StatusBar {
    id: ElementId,
    children: Vec<AnyElement>,
    style: gpui_kit::StyleRefinement,
}

impl StatusBar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            children: Vec::new(),
            style: Default::default(),
        }
    }
}

impl Styled for StatusBar {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for StatusBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .h(px(24.))
            .px(rems(0.75))
            .flex()
            .items_center()
            .gap(rems(0.75))
            .bg(theme.sunken)
            .text_xs()
            .text_color(theme.muted_foreground)
            .border_t_1()
            .border_color(theme.border)
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// 侧栏容器。默认宽 240px，collapsed 时 56px；内容按调用方排布。
#[derive(IntoElement)]
pub struct Sidebar {
    id: ElementId,
    children: Vec<AnyElement>,
    collapsed: bool,
    style: gpui_kit::StyleRefinement,
}

impl Sidebar {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            children: Vec::new(),
            collapsed: false,
            style: Default::default(),
        }
    }

    pub fn collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }
}

impl Styled for Sidebar {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Sidebar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Sidebar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .h_full()
            .w(rems(if self.collapsed { 3.5 } else { 15.0 }))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(theme.sidebar)
            .text_color(theme.sidebar_foreground)
            .border_r_1()
            .border_color(theme.sidebar_border)
            .refine_style(&self.style)
            .children(self.children)
    }
}
