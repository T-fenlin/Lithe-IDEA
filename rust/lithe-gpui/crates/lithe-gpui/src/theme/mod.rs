//! 主题层：全局 [`Theme`] + 语义 token + light/dark 切换。
//!
//! 色值逐项移植自 Ely GPUI Components（见 `palette.rs` 头注）；语义角色命名对齐
//! gpui-component 的 `ThemeColor`，调用侧写法与 gpui-component 一致
//! （`cx.theme().background` 等），宿主迁移时调用点不需要改主题读取代码。
//! Ely 的跨模式渐变动画未移植：[`Theme::set_mode`] 直接切换。

mod palette;

use gpui_kit::{App, Global, Pixels, SharedString, WindowAppearance, px};
use std::ops::{Deref, DerefMut};

pub use palette::{Syntax, ThemeColors};

/// 外观模式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub fn is_dark(self) -> bool {
        self == Self::Dark
    }
}

impl From<WindowAppearance> for ThemeMode {
    fn from(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::Light,
        }
    }
}

/// 全局主题。字体不移植 Ely（Inter/JetBrains Mono）：默认系统 UI 字体，
/// 等宽字体按平台给默认值，调用方可直接覆盖这两个字段。
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub mode: ThemeMode,
    pub high_contrast: bool,
    pub font_family: SharedString,
    pub font_size: Pixels,
    pub mono_font_family: SharedString,
    pub mono_font_size: Pixels,
    /// 常规控件圆角（Ely Radius::Md，6px）。
    pub radius: Pixels,
    pub radius_sm: Pixels,
    pub radius_lg: Pixels,
    pub radius_xl: Pixels,
    /// 聚焦控件是否在边框外画 focus ring。ring 画在元素外，布局大量裁剪的
    /// 面板（overflow_hidden）可以关掉，只保留着色边框。
    pub focus_ring: bool,
    pub colors: ThemeColors,
}

impl Global for Theme {}

impl Deref for Theme {
    type Target = ThemeColors;

    fn deref(&self) -> &Self::Target {
        &self.colors
    }
}

impl DerefMut for Theme {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.colors
    }
}

impl Theme {
    #[inline(always)]
    pub fn global(cx: &App) -> &Theme {
        cx.global::<Theme>()
    }

    /// 直接可变访问只改到被触碰的字段，日常写路径请用 [`Theme::update`]。
    #[inline(always)]
    pub fn global_mut(cx: &mut App) -> &mut Theme {
        cx.global_mut::<Theme>()
    }

    /// 修改主题并刷新所有窗口。这是唯一允许的写路径。
    pub fn update(cx: &mut App, edit: impl FnOnce(&mut Theme)) {
        edit(cx.global_mut::<Theme>());
        cx.refresh_windows();
    }

    pub fn is_dark(&self) -> bool {
        self.mode.is_dark()
    }

    /// 切换 light/dark。Ely 的跨模式渐变（palette fade）未移植，直接切换。
    pub fn set_mode(mode: ThemeMode, cx: &mut App) {
        Theme::update(cx, |theme| {
            theme.mode = mode;
            theme.colors = ThemeColors::for_mode(mode, theme.high_contrast);
        });
    }

    pub fn set_high_contrast(on: bool, cx: &mut App) {
        Theme::update(cx, |theme| {
            theme.high_contrast = on;
            theme.colors = ThemeColors::for_mode(theme.mode, on);
        });
    }

    pub fn light(high_contrast: bool) -> Theme {
        Self::build(ThemeMode::Light, high_contrast)
    }

    pub fn dark(high_contrast: bool) -> Theme {
        Self::build(ThemeMode::Dark, high_contrast)
    }

    fn build(mode: ThemeMode, high_contrast: bool) -> Theme {
        Theme {
            mode,
            high_contrast,
            font_family: ".SystemUIFont".into(),
            font_size: px(16.),
            mono_font_family: default_mono_font_family().into(),
            mono_font_size: px(13.),
            radius: px(6.),
            radius_sm: px(4.),
            radius_lg: px(8.),
            radius_xl: px(12.),
            focus_ring: true,
            colors: ThemeColors::for_mode(mode, high_contrast),
        }
    }
}

/// 注册全局主题（默认 light）。必须在打开窗口前调用一次。
pub fn init(cx: &mut App) {
    gpui_kit::base::init(cx);
    crate::overlay::init(cx);
    cx.set_global(Theme::light(false));
}

/// 语义主题读取入口。与 gpui-component 的 `ActiveTheme` 同名同义。
pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    #[inline(always)]
    fn theme(&self) -> &Theme {
        Theme::global(self)
    }
}

fn default_mono_font_family() -> &'static str {
    // 与 gpui-component 的默认口径一致；缺字体时的兜底交给系统字体解析。
    if cfg!(target_os = "macos") {
        "Menlo"
    } else if cfg!(target_os = "windows") {
        "Consolas"
    } else {
        "DejaVu Sans Mono"
    }
}
