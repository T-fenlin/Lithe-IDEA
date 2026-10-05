//! 表单标签：字段名 + 可选必填标记。

use gpui_kit::TestSupportExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, SharedString, Window, div, rems};

use crate::ActiveTheme;

/// 字段标签。必填标记是视觉约定，真正的校验归调用方。
#[derive(IntoElement)]
pub struct Label {
    id: ElementId,
    label: SharedString,
    required: bool,
}

impl Label {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            required: false,
        }
    }

    /// 显示必填星标。
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
}

impl RenderOnce for Label {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .flex()
            .items_center()
            .gap(rems(0.125))
            .text_sm()
            .text_color(theme.foreground)
            .child(self.label)
            .when(self.required, |el| {
                el.child(div().text_color(theme.danger).text_sm().child("*"))
            })
    }
}
