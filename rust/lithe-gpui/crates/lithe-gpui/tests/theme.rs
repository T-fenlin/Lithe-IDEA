//! 主题语义契约：默认值锚定 Ely 移植值，模式切换必须整体换装。

use gpui_kit::TestAppContext;
use lithe_gpui::{ActiveTheme as _, Theme, ThemeMode, init};

#[gpui_kit::test]
fn init_registers_the_light_ely_theme(cx: &mut TestAppContext) {
    cx.update(init);
    cx.update(|cx| {
        let theme = cx.theme();
        assert!(!theme.is_dark());
        assert!(!theme.high_contrast);
        assert_eq!(theme.background, gpui_kit::rgb(0xfcfaf7).into());
        assert_eq!(theme.primary, gpui_kit::rgb(0x181613).into());
        assert_eq!(theme.radius, gpui_kit::px(6.));
        assert!(theme.focus_ring);
    });
}

#[gpui_kit::test]
fn set_mode_switches_the_whole_semantic_surface(cx: &mut TestAppContext) {
    cx.update(init);
    let light_background = cx.update(|cx| cx.theme().background);
    cx.update(|cx| Theme::set_mode(ThemeMode::Dark, cx));
    cx.update(|cx| {
        let theme = cx.theme();
        assert!(theme.is_dark());
        assert_ne!(theme.background, light_background);
        assert_eq!(theme.background, gpui_kit::rgb(0x131110).into());
        assert_eq!(theme.primary, gpui_kit::rgb(0xf3f1f0).into());
    });
}

#[gpui_kit::test]
fn high_contrast_swaps_in_the_strengthened_palette(cx: &mut TestAppContext) {
    cx.update(init);
    cx.update(|cx| Theme::set_high_contrast(true, cx));
    cx.update(|cx| {
        let theme = cx.theme();
        assert!(theme.high_contrast);
        assert_eq!(theme.border, gpui_kit::rgb(0xa6a4a2).into());
    });
}
