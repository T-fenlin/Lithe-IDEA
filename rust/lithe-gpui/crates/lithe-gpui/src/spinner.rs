//! 加载指示：四点环旋转。Ely 的动效口径（reduced motion 联动）在动效章统一补齐。

use std::time::Duration;

use gpui_kit::TestSupportExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{
    Animation, AnimationExt as _, App, ElementId, IntoElement, RenderOnce, Window, div, px,
};

use crate::{ActiveTheme, Sizable, Size};

/// 加载指示器。纯展示元素：是否处于加载中由调用方的状态决定。
#[derive(IntoElement)]
pub struct Spinner {
    id: ElementId,
    size: Size,
}

impl Spinner {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            size: Size::Medium,
        }
    }
}

impl Sizable for Spinner {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

// 本 gpui 的 Transformation/.transform() 是 SVG 专属能力，Div 没有 transform；
// 旋转用"动画闭包内逐帧重排圆点极角"实现。动画闭包每帧拿到元素的全新克隆，
// 在闭包内追加 children 不会跨帧累积。观测点在静态外壳上，动画跑在内层，
// 这样闭包元素是普通 Div，不必受 Observed 包装的转发面限制。
const DOTS: [(f32, f32); 4] = [(0.0, 1.0), (90.0, 0.75), (180.0, 0.5), (270.0, 0.3)];

impl RenderOnce for Spinner {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let fg = cx.theme().foreground;
        let ring = f32::from(self.size.icon_px(window.rem_size()));
        let dot = (ring / 4.0).max(2.0);
        let radius = (ring - dot) / 2.0;
        let center = (ring - dot) / 2.0;

        div()
            .id(self.id)
            .test_support()
            .relative()
            .size(px(ring))
            .flex_shrink_0()
            .child(
                div().size_full().with_animation(
                    "spin",
                    Animation::new(Duration::from_millis(900))
                        .repeat()
                        .with_easing(|delta| delta),
                    move |el, delta| {
                        let sweep = delta * 360.0;
                        el.children(DOTS.into_iter().map(|(angle, opacity)| {
                            let a = (angle + sweep).to_radians();
                            let x = center + radius * a.sin();
                            let y = center - radius * a.cos();
                            div()
                                .absolute()
                                .size(px(dot))
                                .left(px(x))
                                .top(px(y))
                                .rounded_full()
                                .bg(fg)
                                .opacity(opacity)
                        }))
                    },
                ),
            )
    }
}
