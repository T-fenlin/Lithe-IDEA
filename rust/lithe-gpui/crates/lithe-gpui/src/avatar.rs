//! 头像：圆形底 + 首字母兜底。图片源（网络/资产加载）在数据展示章扩展。

use gpui_kit::TestSupportExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, SharedString, Window, div, px};

use crate::{ActiveTheme, Sizable, Size};

/// 头像。当前只支持首字母/短文本兜底；边长取 Ely `avatar_size` 的四档
/// （20/24/32/40px），自定义尺寸用 `Size::Size(px(...))`。
#[derive(IntoElement)]
pub struct Avatar {
    id: ElementId,
    initials: SharedString,
    size: Size,
}

impl Avatar {
    pub fn new(id: impl Into<ElementId>, initials: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            initials: initials.into(),
            size: Size::Medium,
        }
    }
}

impl Sizable for Avatar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Avatar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let length = self.size.icon_px(window.rem_size());
        div()
            .id(self.id)
            .test_support()
            .size(length)
            .flex_shrink_0()
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.primary)
            .text_color(theme.primary_foreground)
            .text_size(px(length / px(2.4)))
            .child(self.initials)
    }
}
