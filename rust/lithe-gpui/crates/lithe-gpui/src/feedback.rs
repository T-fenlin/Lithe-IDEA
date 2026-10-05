//! 提示与空态（第 10 章 Feedback）。

use gpui_kit::TestSupportExt as _;
use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, rems,
};

use crate::{ActiveTheme, Severity};

/// 提示条：severity 语义色 + 可选标题 + 说明/操作内容。
#[derive(IntoElement)]
pub struct Alert {
    id: ElementId,
    severity: Severity,
    title: SharedString,
    children: Vec<AnyElement>,
    style: gpui_kit::StyleRefinement,
}

impl Alert {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            severity: Severity::Info,
            title: title.into(),
            children: Vec::new(),
            style: Default::default(),
        }
    }

    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn info(mut self) -> Self {
        self.severity = Severity::Info;
        self
    }

    pub fn success(mut self) -> Self {
        self.severity = Severity::Success;
        self
    }

    pub fn warning(mut self) -> Self {
        self.severity = Severity::Warning;
        self
    }

    pub fn danger(mut self) -> Self {
        self.severity = Severity::Danger;
        self
    }
}

impl Styled for Alert {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Alert {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Alert {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let color = theme.severity(self.severity);
        div()
            .id(self.id)
            .test_support()
            .rounded(theme.radius)
            .border_l_2()
            .bg(theme.severity_subtle(self.severity))
            .p(rems(0.75))
            .flex()
            .flex_col()
            .gap(rems(0.25))
            .refine_style(&self.style)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(color)
                    .child(self.title),
            )
            .when(!self.children.is_empty(), |el| {
                el.child(
                    div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .flex()
                        .flex_col()
                        .gap(rems(0.25))
                        .children(self.children),
                )
            })
    }
}

/// 空态：标题 + 可选说明 + 操作区。
#[derive(IntoElement)]
pub struct Empty {
    id: ElementId,
    title: SharedString,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    style: gpui_kit::StyleRefinement,
}

impl Empty {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            children: Vec::new(),
            style: Default::default(),
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl Styled for Empty {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Empty {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Empty {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(rems(0.75))
            .p(rems(2.0))
            .refine_style(&self.style)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(theme.foreground)
                    .child(self.title),
            )
            .children(
                self.description
                    .map(|d| div().text_sm().text_color(theme.muted_foreground).child(d)),
            )
            .when(!self.children.is_empty(), |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(rems(0.5))
                        .children(self.children),
                )
            })
    }
}
