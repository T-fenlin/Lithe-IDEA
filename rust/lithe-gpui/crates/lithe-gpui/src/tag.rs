//! 标签：可移除的条目标签（文件类型、过滤器、收件人之类）。

use std::rc::Rc;

use gpui_kit::TestSupportExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, SharedString, Window, div, rems};

use crate::ActiveTheme;

/// 标签。设置 [`Tag::on_remove`] 后渲染关闭钮；回调只报告意图，
/// 是否真的移除由调用方的状态决定。
#[derive(IntoElement)]
pub struct Tag {
    id: ElementId,
    label: SharedString,
    on_remove: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

impl Tag {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            on_remove: None,
        }
    }

    pub fn on_remove(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_remove = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Tag {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .flex_shrink_0()
            .h(rems(1.5))
            .px(rems(0.5))
            .gap(rems(0.25))
            .rounded(theme.radius_sm)
            .border_1()
            .border_color(theme.border)
            .bg(theme.sunken)
            .flex()
            .items_center()
            .text_sm()
            .text_color(theme.foreground)
            .child(self.label)
            .when_some(self.on_remove, |el, on_remove| {
                el.child(
                    div()
                        .id("remove")
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(rems(1.0))
                        .rounded(theme.radius_sm)
                        .text_color(theme.subtle_foreground)
                        .text_sm()
                        .hover(|style| style.bg(theme.hover).text_color(theme.foreground))
                        .cursor_pointer()
                        .child("×")
                        .on_click(move |_, window, cx| on_remove(window, cx))
                        .test_support(),
                )
            })
    }
}
