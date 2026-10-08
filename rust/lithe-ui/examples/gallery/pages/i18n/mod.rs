use gpui::{AnyElement, App, IntoElement, ParentElement, Styled, Window, div, px};
use jiff::{Timestamp, tz::TimeZone};
use lithe_ui::{
    forms::{Choice, Select},
    i18n::{Direction, LOCALES, Locale, locale, set_locale, text},
    primitives::{Icon, IconName},
    theme::{ActiveTheme, Radius},
    typography::Ellipsis,
};

mod a11y;

use super::Page;
use crate::probe::probe;
use crate::step::Step;
use crate::ui::{section, specimen, specimens};

pub const PAGE: Page = Page {
    number: 40,
    slug: "i18n",
    title: "i18n & a11y",
    summary: "Messages, numbers and dates in the reader's locale. A right-to-left locale mirrors its stacks.",
    render,
    script: &[
        Step::Click("locale"),
        Step::Key("end"),
        Step::Key("enter"),
        Step::Wait(300),
        Step::Shot("right-to-left"),
        Step::Click("locale"),
        Step::Key("home"),
        Step::Key("enter"),
        Step::Wait(300),
        Step::Key("tab"),
        Step::HoverAt("skip-demo", 300.0, 70.0),
        Step::Wait(300),
        Step::Shot("skip-link"),
        Step::Key("enter"),
        Step::Key("tab"),
        Step::Wait(300),
        Step::Shot("skipped"),
        Step::Click("color-blind"),
        Step::Wait(300),
        Step::Shot("color-blind"),
        Step::Click("color-blind"),
        Step::Wait(300),
    ],
};

/// 1 September 2026, when the demo's folder was saved.
const SAVED: i64 = 1_788_220_800;

fn render(window: &mut Window, cx: &mut App) -> AnyElement {
    div()
        .child(messages(cx))
        .child(formats(cx))
        .child(mirrored(cx))
        .child(a11y::skip_link(window, cx))
        .child(a11y::color_blind(cx))
        .into_any_element()
}

fn messages(cx: &App) -> impl IntoElement + use<> {
    let locale = Locale::find(&locale()).unwrap_or(Locale::of("en-US"));
    let saved = Timestamp::from_second(SAVED).expect("a time");
    let lines = [
        // The catalogue the library ships: plain messages, one with an argument,
        // and the locale's own number and date spelling beside them.
        text(cx, "palette.recent", &[]),
        text(
            cx,
            "palette.search.empty",
            &[("query", &locale.number(1204.0, 0))],
        ),
        text(
            cx,
            "palette.branch.create",
            &[("query", &locale.date(saved, &TimeZone::UTC))],
        ),
        text(cx, "state.unavailable", &[]),
    ];
    section(
        "rust_i18n · messages read from locales/ui.yml",
        "One global locale, one catalogue per language, read through the same rust_i18n the host uses. A message with an unfilled argument fails; an unknown key falls back to English.",
        cx,
    )
    .child(
        specimens()
            .child(specimen(
                "locale",
                probe(
                    "locale",
                    div().w(px(240.0)).child(
                        Select::new(
                            "locale",
                            LOCALES.map(|locale| Choice::new(locale.tag, locale.name)),
                        )
                        .selected(locale.tag)
                        .on_change(|tag, _, cx| set_locale(tag, cx)),
                    ),
                ),
                cx,
            ))
            .child(specimen(
                "messages",
                locale.direction.align(
                    div()
                        .w(px(280.0))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(lines.map(Ellipsis::new)),
                ),
                cx,
            )),
    )
}

fn formats(cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let saved = Timestamp::from_second(SAVED).expect("a time");
    let rows = LOCALES.map(|locale| {
        div()
            .flex()
            .flex_wrap()
            .gap_x_4()
            .child(
                div()
                    .w(px(96.0))
                    .text_color(theme.colors.fg_muted)
                    .child(locale.name),
            )
            .child(div().w(px(120.0)).child(locale.number(-1234567.891, 2)))
            .child(div().child(locale.date(saved, &TimeZone::UTC)))
    });
    section(
        "LocaleNumber / LocaleDate",
        "Each locale's separators and its short numeric date, from CLDR.",
        cx,
    )
    .child(div().flex().flex_col().gap_1().children(rows))
}

fn mirrored(cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let direction = Locale::find(&locale())
        .map(|locale| locale.direction)
        .unwrap_or(Direction::Ltr);
    let file = |name: &'static str, size: &'static str| {
        direction
            .row(div())
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .child(Icon::new(IconName::File).color(theme.colors.fg_muted))
            .child(direction.align(div().flex_1().min_w_0().child(Ellipsis::new(name))))
            .child(
                div()
                    .flex_none()
                    .text_color(theme.colors.fg_muted)
                    .child(size),
            )
    };
    section(
        "RTL layout",
        "A right-to-left locale starts its stacks and sets its text at the right; pick עברית above. Ely's other components stay left to right.",
        cx,
    )
    .child(
        div()
            .max_w(px(320.0))
            .flex()
            .flex_col()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(theme.colors.border)
            .child(file("Docs", "12 KB"))
            .child(file("Photos", "3.1 MB")),
    )
}
