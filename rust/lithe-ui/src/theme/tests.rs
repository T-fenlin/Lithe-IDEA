use std::time::Duration;

use gpui::{TestAppContext, hsla};

use super::{ActiveTheme, Mode, Palette, Syntax, Theme};

/// An owner's palette serves its own mode; the other keeps Ely's, and none brings Ely's back.
#[gpui::test]
fn an_owners_palette_serves_its_mode_alone(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let mut own = Palette::light(false);
    own.accent = hsla(0.6, 0.5, 0.5, 1.0);
    cx.update(|cx| Theme::set_palette(Mode::Light, Some(own.clone()), cx));
    assert_eq!(cx.read(|cx| cx.theme().palette()), own);
    cx.update(|cx| Theme::set_mode(Mode::Dark, cx));
    assert_eq!(cx.read(|cx| cx.theme().palette()), Palette::dark(false));
    cx.update(|cx| {
        Theme::set_mode(Mode::Light, cx);
        Theme::set_palette(Mode::Light, None, cx);
    });
    assert_eq!(cx.read(|cx| cx.theme().palette()), Palette::light(false));
}

/// A mode set at once keeps its palette through fades.
#[gpui::test]
fn a_mode_set_at_once_skips_the_fade(cx: &mut TestAppContext) {
    cx.update(|cx| {
        Theme::init(cx);
        Theme::update(cx, |theme| theme.reduced_motion = true);
        Theme::set_mode(Mode::Dark, cx);
        Theme::set_mode_now(Mode::Light, cx);
    });
    assert_eq!(
        cx.read(|cx| cx.theme().colors.clone()),
        Palette::light(false)
    );
    std::thread::sleep(Duration::from_millis(2));
    cx.executor().advance_clock(Duration::from_millis(100));
    cx.run_until_parked();
    assert!(!cx.read(|cx| cx.theme().is_dark()));
    assert_eq!(
        cx.read(|cx| cx.theme().colors.clone()),
        Palette::light(false)
    );
}

/// Every color of the palette and of code answers to its field's name, the quiet fills too.
#[test]
fn every_color_answers_to_its_name() {
    let mut palette = Palette::light(false);
    for (ix, name) in Palette::NAMES.iter().enumerate() {
        *palette.token_mut(name) = hsla(ix as f32 / 64.0, 0.5, 0.5, 1.0);
    }
    // 41 = 36 + chrome 表面槽 `tab_bar` / `title_bar` / `status_bar`
    // （前两个 2026-10-08 加，`status_bar` 2026-10-09 加，见 palette.rs 的说明）
    // + 品牌色槽 `primary` / `on_primary`（2026-10-10 加，同文件说明）。
    assert_eq!(Palette::NAMES.len(), 41, "every color of the palette");
    let at = |name: &str| {
        Palette::NAMES
            .iter()
            .position(|each| *each == name)
            .expect("a name")
    };
    assert_eq!(
        palette.success_subtle,
        hsla(at("success_subtle") as f32 / 64.0, 0.5, 0.5, 1.0)
    );
    assert_eq!(palette.ink, hsla(at("ink") as f32 / 64.0, 0.5, 0.5, 1.0));
    for name in Syntax::NAMES {
        *palette.syntax.token_mut(name) = hsla(0.4, 0.2, 0.3, 1.0);
    }
    assert_eq!(Syntax::NAMES.len(), 13, "every color of code");
    assert_eq!(palette.syntax.comment, hsla(0.4, 0.2, 0.3, 1.0));
}

#[test]
#[should_panic(expected = "no palette color glow")]
fn an_unknown_color_fails() {
    Palette::light(false).token_mut("glow");
}

/// The status bar is its own chrome surface slot: it starts on the chrome color, never the page
/// background, and a host that has a separate status-bar color moves it without touching `bg`.
///
/// Regression risk: mapping a host's status-bar color onto `bg` or `title_bar` would repaint the
/// page (or the window title bar) whenever a theme gives the status bar its own color —
/// `themes/jetbrains.json` gives it `#f2f2f2` against a `#ffffff` page in the light theme.
#[test]
fn the_status_bar_surface_moves_alone() {
    for mut palette in [Palette::light(false), Palette::dark(false)] {
        // 2026-10-11 起 chrome 槽不再与页面底色同值：一条与页面同色的状态栏没有可见边界，
        // 正是 chrome 槽要避免的那个问题。
        assert_ne!(
            palette.status_bar, palette.bg,
            "a chrome bar must be tellable from the page it sits on"
        );
        palette.status_bar = hsla(0.2, 0.5, 0.5, 1.0);
        assert_ne!(
            palette.status_bar, palette.title_bar,
            "and it has its own slot"
        );
        assert_eq!(palette.token("status_bar"), palette.status_bar);
    }
}

/// The chrome slots answer to different colors than the page.
///
/// Regression risk: `title_bar` once held the page `bg`. A `MenuBar` at the top of a page then
/// painted a background nobody could see — the reported "transparent menu bar".
#[test]
fn every_chrome_slot_differs_from_the_page() {
    for palette in [Palette::light(false), Palette::dark(false)] {
        for (slot, name) in [
            (palette.title_bar, "title_bar"),
            (palette.status_bar, "status_bar"),
        ] {
            assert_ne!(slot, palette.bg, "{name} must differ from bg: {slot:?}");
        }
    }
}

#[test]
fn glass_lets_the_blur_through_until_high_contrast() {
    for (plain, strong) in [
        (Palette::light(false), Palette::light(true)),
        (Palette::dark(false), Palette::dark(true)),
    ] {
        assert!(
            plain.glass.a > 0.0 && plain.glass.a < 1.0,
            "{:?}",
            plain.glass
        );
        assert_eq!(strong.glass, strong.bg);
    }
}
