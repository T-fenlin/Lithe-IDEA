//! 键位提示：等宽字体的按键帽。

use gpui_kit::TestSupportExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, SharedString, Window, div, rems};

use crate::ActiveTheme;

/// 按键帽。内容是调用方格式化好的键位文本（如 "⌘K"、"Ctrl+Shift+P"）。
#[derive(IntoElement)]
pub struct Kbd {
    id: ElementId,
    keys: SharedString,
}

impl Kbd {
    pub fn new(id: impl Into<ElementId>, keys: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            keys: keys.into(),
        }
    }
}

impl RenderOnce for Kbd {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .h(rems(1.25))
            .px(rems(0.375))
            .flex()
            .items_center()
            .rounded(theme.radius_sm)
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.surface)
            .text_xs()
            .text_color(theme.subtle_foreground)
            .font_family(theme.mono_font_family.clone())
            .child(self.keys)
    }
}
