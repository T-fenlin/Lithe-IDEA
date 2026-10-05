//! 图标：渲染调用方注入的 SVG 资源路径。本 crate 不内嵌图标集——宿主经
//! `AssetSource` 提供 SVG 资产（app crate 走 `LitheAssets`），gallery 示例
//! 内嵌了最小集合演示注入方式。

use gpui_kit::TestSupportExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, Hsla, IntoElement, RenderOnce, SharedString, Window, svg};

use crate::{Sizable, Size};

/// 图标元素。填充色默认继承父级文本色（currentColor 语义）。
#[derive(IntoElement)]
pub struct Icon {
    id: ElementId,
    path: SharedString,
    size: Size,
    color: Option<Hsla>,
}

impl Icon {
    pub fn new(id: impl Into<ElementId>, path: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            size: Size::Medium,
            color: None,
        }
    }

    /// 覆盖填充色；不设置则继承文本色。
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

impl Sizable for Icon {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        // path 是 Svg 的固有方法，必须在 .id() 把元素变为 Stateful 之前调用。
        svg()
            .path(self.path)
            .id(self.id)
            .test_support()
            .size(self.size.icon_px(window.rem_size()))
            .when_some(self.color, |el, color| el.text_color(color))
    }
}
