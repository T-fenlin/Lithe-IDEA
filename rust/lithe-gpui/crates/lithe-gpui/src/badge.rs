//! 徽章：语义状态的小型胶囊标签。样式走 Ely 的安静路线——彩色变体用
//! subtle 底 + 语义色文字，Primary 用墨色实底。

use gpui_kit::TestSupportExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, SharedString, Window, div, px};

use crate::{ActiveTheme, Sizable, Size};

/// 徽章语义变体。语义保持稀缺：普通元数据用 Neutral，不要给每个枚举值配色。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeVariant {
    #[default]
    Neutral,
    Primary,
    Success,
    Warning,
    Danger,
    Info,
}

/// 徽章。
#[derive(IntoElement)]
pub struct Badge {
    id: ElementId,
    label: SharedString,
    variant: BadgeVariant,
    size: Size,
}

impl Badge {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: BadgeVariant::Neutral,
            size: Size::Medium,
        }
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Sizable for Badge {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

// 高度/内边距走 Ely 的控件刻度（18/20/24px @ 16px rem），统一在像素域表达：
// 命名档位随窗口 rem 缩放，自定义档位（Size::Size）按调用方给的原值。
fn badge_geometry(size: Size, rem_scale: f32) -> (gpui_kit::Pixels, gpui_kit::Pixels) {
    match size {
        Size::Size(height) => (height, px(6.0 * rem_scale)),
        Size::XSmall | Size::Small => (px(18.0 * rem_scale), px(6.0 * rem_scale)),
        Size::Medium => (px(20.0 * rem_scale), px(8.0 * rem_scale)),
        Size::Large => (px(24.0 * rem_scale), px(10.0 * rem_scale)),
    }
}

impl RenderOnce for Badge {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let rem_scale = f32::from(window.rem_size()) / 16.0;
        let (height, padding) = badge_geometry(self.size, rem_scale);
        let (bg, fg) = match self.variant {
            BadgeVariant::Neutral => (theme.muted, theme.muted_foreground),
            // Ely 的单色 accent 设计：primary 徽章是墨色实底，不引入品牌彩。
            BadgeVariant::Primary => (theme.primary, theme.primary_foreground),
            BadgeVariant::Success => (theme.success_subtle, theme.success),
            BadgeVariant::Warning => (theme.warning_subtle, theme.warning),
            BadgeVariant::Danger => (theme.danger_subtle, theme.danger),
            BadgeVariant::Info => (theme.info_subtle, theme.info),
        };
        div()
            .id(self.id)
            .test_support()
            .flex_shrink_0()
            .h(height)
            .px(padding)
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(bg)
            .text_color(fg)
            .text_xs()
            .child(self.label)
    }
}
