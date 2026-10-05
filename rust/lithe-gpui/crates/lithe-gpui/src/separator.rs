//! 分隔线：细分隔线，方向可选。

use gpui_kit::TestSupportExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, Window, div, px};

use crate::ActiveTheme;

/// 分隔线。默认水平（1px 高、占满宽），[`Separator::vertical`] 切为竖直。
#[derive(IntoElement)]
pub struct Separator {
    id: ElementId,
    vertical: bool,
}

impl Separator {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            vertical: false,
        }
    }

    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.vertical = false;
        self
    }
}

impl RenderOnce for Separator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = cx.theme().border;
        div()
            .id(self.id)
            .test_support()
            .flex_shrink_0()
            .when(!self.vertical, |el| el.h(px(1.)).w_full())
            .when(self.vertical, |el| el.w(px(1.)).h_full())
            .bg(color)
    }
}
