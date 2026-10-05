//! 控件尺寸刻度：对齐 gpui-component 的 `Sizable`/`Size`，调用侧写法一致。

use gpui_kit::{Pixels, px};

/// 控件尺寸。`Size::Size` 携带自定义像素值（`px(30.)`），命名档位用于常规刻度。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Size {
    Size(Pixels),
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

impl From<Pixels> for Size {
    fn from(value: Pixels) -> Self {
        Self::Size(value)
    }
}

impl Size {
    /// 图标类元素（Icon/Spinner）的像素边长，基准取 Ely `icon_size` 的前四档
    /// （12/14/16/20px @ 16px rem），随窗口 rem 缩放；更大档位用
    /// `Size::Size(px(...))` 表达。
    pub(crate) fn icon_px(self, rem_size: Pixels) -> Pixels {
        // Pixels 的字段在本 gpui 里是 pub(crate)，取值走 From<Pixels> for f32。
        let scale = f32::from(rem_size) / 16.0;
        let base = match self {
            Self::Size(length) => return length,
            Self::XSmall => 12.0,
            Self::Small => 14.0,
            Self::Medium => 16.0,
            Self::Large => 20.0,
        };
        px(base * scale)
    }
}

/// 流式尺寸设置。与 gpui-component 的 `Sizable` 同名同义，迁移时调用点不改。
pub trait Sizable: Sized {
    fn with_size(self, size: impl Into<Size>) -> Self;

    #[inline(always)]
    fn xsmall(self) -> Self {
        self.with_size(Size::XSmall)
    }

    #[inline(always)]
    fn small(self) -> Self {
        self.with_size(Size::Small)
    }

    #[inline(always)]
    fn medium(self) -> Self {
        self.with_size(Size::Medium)
    }

    #[inline(always)]
    fn large(self) -> Self {
        self.with_size(Size::Large)
    }
}
