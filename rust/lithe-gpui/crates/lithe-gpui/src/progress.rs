//! 进度条：受控值展示。Ely 的细线口径（2px 轨道）。

use gpui_kit::TestSupportExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, Window, div, px, relative};

use crate::ActiveTheme;

/// 进度条。value 是 0.0–1.0 的受控值，越界部分被钳制；是否推进由调用方驱动。
#[derive(IntoElement)]
pub struct Progress {
    id: ElementId,
    value: f32,
}

impl Progress {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: 0.0,
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = clamp_progress(value);
        self
    }
}

fn clamp_progress(value: f32) -> f32 {
    // f32::clamp 对 NaN 原样返回，会画出非法宽度；NaN 按未推进处理。
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

impl RenderOnce for Progress {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .w_full()
            .h(px(2.))
            .rounded_full()
            .overflow_hidden()
            .bg(theme.muted)
            .child(
                div()
                    .id("fill")
                    .h_full()
                    .w(relative(self.value))
                    .rounded_full()
                    .bg(theme.primary),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::clamp_progress;

    #[test]
    fn progress_value_is_clamped_to_unit_interval() {
        assert_eq!(clamp_progress(-0.5), 0.0);
        assert_eq!(clamp_progress(0.42), 0.42);
        assert_eq!(clamp_progress(1.5), 1.0);
        assert_eq!(clamp_progress(f32::NAN), 0.0);
    }
}
