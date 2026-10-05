use jiff::{Timestamp, tz::TimeZone};

use super::{Direction, LOCALES, Locale, fill, message, render};

#[cfg(feature = "test-support")]
mod a11y;

/// Each `%{name}` takes its argument, in the order the message puts them.
#[test]
fn a_message_fills_its_arguments() {
    let args = [("folder", "Docs"), ("count", "1,204")];
    assert_eq!(
        render("%{count} files in %{folder}", &args, Direction::Ltr),
        Ok("1,204 files in Docs".into())
    );
    assert_eq!(render("Hello", &[], Direction::Ltr), Ok("Hello".into()));
    assert_eq!(fill("%{a}%{a}", &[("a", "x")]), Ok("xx".into()));
    assert_eq!(fill("%{a", &[("a", "x")]), Err("a %{ without }".into()));
    assert_eq!(fill("%{b}", &[("a", "x")]), Err("no argument \"b\"".into()));
    // A stray brace that opens nothing is just text.
    assert_eq!(fill("100% sure", &[]), Ok("100% sure".into()));
}

/// A right-to-left message leads with its mark, so one that starts with Latin letters or a signed number still reads right to left.
#[test]
fn a_right_to_left_message_leads_with_its_mark() {
    let change = Locale::of("he-IL").number(-3.5, 1);
    assert_eq!(
        render(
            "%{change}% מאז אתמול",
            &[("change", &change)],
            Direction::Rtl
        ),
        Ok("\u{200f}\u{200e}−3.5% מאז אתמול".into())
    );
    assert_eq!(
        render("%{name} שלח", &[("name", "Ada")], Direction::Rtl),
        Ok("\u{200f}Ada שלח".into())
    );
    assert_eq!(
        render("Hello", &[], Direction::Ltr),
        Ok("Hello".into()),
        "a left-to-right message takes no mark"
    );
}

/// A message whose key the catalogue holds reads as this crate's English copy.
#[test]
fn a_key_reads_its_message() {
    assert_eq!(message("palette.recent"), "Recent");
    assert_eq!(message("state.unavailable"), "Unavailable");
}

/// Every leaf in the catalogue carries every locale, so a message cannot ship
/// translated for one language and missing for another. This replaces the
/// parity assertion the hand-rolled catalogue used to run at load time.
#[test]
fn every_locale_holds_the_same_keys() {
    let catalogue = include_str!("../../../locales/ui.yml");
    assert!(
        catalogue.starts_with("# Component copy."),
        "the catalogue's header changed"
    );
    let mut leaves = 0;
    for line in catalogue.lines().filter(|line| line.contains(": {")) {
        leaves += 1;
        // Every message carries every locale this crate lists, so one cannot ship
        // translated for a language and missing for another.
        for locale in LOCALES {
            assert!(
                line.contains(&format!("{}: ", locale.tag)),
                "no {} in {line:?}",
                locale.tag
            );
        }
    }
    assert!(leaves >= 30, "the catalogue shrank to {leaves} messages");
}

/// The catalogue declares the rust-i18n schema version it is written in.
#[test]
fn the_catalogue_names_the_rust_i18n_schema_version() {
    let catalogue = include_str!("../../../locales/ui.yml");
    assert!(catalogue.contains("\n_version: 2\n"), "rust_i18n v2 format");
}

#[test]
fn a_missing_argument_fails() {
    assert_eq!(
        render(
            "%{count} files in %{folder}",
            &[("count", "3")],
            Direction::Ltr
        ),
        Err("no argument \"folder\"".into())
    );
}

/// Separators and the date's order follow each locale.
#[test]
fn locales_write_their_own_numbers_and_dates() {
    let at = Timestamp::from_second(1_788_220_800).unwrap();
    let cases: Vec<_> = LOCALES
        .iter()
        .map(|locale| {
            (
                locale.tag,
                locale.number(-1234567.891, 2),
                locale.date(at, &TimeZone::UTC),
            )
        })
        .collect();
    assert_eq!(
        cases,
        [
            ("en-US", "−1,234,567.89".into(), "9/1/26".into()),
            ("en-GB", "−1,234,567.89".into(), "01/09/2026".into()),
            ("de-DE", "−1.234.567,89".into(), "01.09.26".into()),
            (
                "fr-FR",
                "−1\u{202f}234\u{202f}567,89".into(),
                "01/09/2026".into()
            ),
            ("ja-JP", "−1,234,567.89".into(), "2026/09/01".into()),
            ("zh-CN", "−1,234,567.89".into(), "2026/9/1".into()),
            ("he-IL", "\u{200e}−1,234,567.89".into(), "1.9.2026".into()),
        ]
    );
    let rtl: Vec<_> = LOCALES
        .iter()
        .filter(|locale| locale.direction == Direction::Rtl)
        .collect();
    assert_eq!(rtl, [&Locale::of("he-IL")]);
    assert_eq!(
        Locale::of("he-IL").number(3.5, 1),
        "3.5",
        "a gain takes no mark"
    );
}

/// An application may show a locale this crate does not list: lookup says so
/// rather than panicking, and the direction falls back to left to right.
#[test]
fn find_reports_a_locale_this_crate_does_not_list() {
    assert_eq!(
        Locale::find("en-US").map(|locale| locale.tag),
        Some("en-US")
    );
    assert_eq!(Locale::find("xx-XX"), None);
}

#[test]
#[should_panic(expected = "no locale \"xx-XX\"")]
fn an_unlisted_locale_fails_when_it_must_be_known() {
    Locale::of("xx-XX");
}

#[cfg(feature = "test-support")]
mod shown {
    use gpui::{IntoElement, ParentElement, Styled, TextAlign, div};

    use super::super::Direction;

    /// Text sits against the side the locale reads from.
    #[test]
    fn text_sits_against_the_reading_side() {
        let side = |direction: Direction| direction.align(div()).style().text.text_align;
        assert_eq!(side(Direction::Ltr), Some(TextAlign::Left));
        assert_eq!(side(Direction::Rtl), Some(TextAlign::Right));
    }
}
