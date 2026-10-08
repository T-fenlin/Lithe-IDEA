//! One locale for the window: component copy, numbers and dates all follow it.
//!
//! Messages live in the crate's `locales/ui.yml` and are read through
//! `rust_i18n`, the same mechanism gpui-kit and the Lithe host use, so
//! `set_locale` moves everything at once.

use gpui::{
    App, AppContext, Context, FocusHandle, IntoElement, ParentElement, Render, Styled, Window,
    WindowOptions, div,
};
use lithe_ui::{
    Assets,
    buttons::Button,
    i18n::{Direction, Locale, locale, set_locale, text},
    primitives::FocusScope,
    theme::ActiveTheme,
};

struct Words {
    focus: FocusHandle,
}

impl Render for Words {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let locale = Locale::find(&locale()).unwrap_or(Locale::of("en-US"));
        let greeting = text(cx, "demo.welcome", &[("name", "Ada")]);
        let files = text(
            cx,
            "demo.files",
            &[("count", &locale.number(1234.0, 0)), ("folder", "Docs")],
        );
        let next = if locale.direction == Direction::Rtl {
            "en-US"
        } else {
            "he-IL"
        };
        let switcher = if locale.direction == Direction::Rtl {
            "English"
        } else {
            "עברית"
        };
        let row = locale
            .direction
            .row(div())
            .gap_4()
            .child(greeting)
            .child(files);
        FocusScope::new(&self.focus)
            .root()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_8()
            .bg(theme.colors.bg)
            .text_color(theme.colors.fg)
            .child(row)
            .child(
                Button::new("locale")
                    .label(switcher)
                    .on_click(move |_, _, cx| set_locale(next, cx)),
            )
    }
}

fn main() {
    gpui_platform::application()
        .with_assets(Assets)
        .run(|cx: &mut App| {
            lithe_ui::init(cx);
            cx.open_window(WindowOptions::default(), |window, cx| {
                cx.new(|cx| {
                    let focus = cx.focus_handle();
                    window.focus(&focus, cx);
                    Words { focus }
                })
            })
            .expect("window failed to open");
            cx.activate(true);
        });
}
