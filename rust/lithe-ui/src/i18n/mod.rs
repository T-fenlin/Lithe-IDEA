//! Component copy and locale data.
//!
//! Messages live in `locales/ui.yml` and are read through [`rust_i18n`], the same
//! mechanism the Lithe host uses, so one `set_locale` moves the whole
//! application. [`text`] stays as the seam components call: it looks the message
//! up, fills its arguments, and applies the locale's paragraph direction.
//!
//! [`Locale`] is separate on purpose: it carries the date, number and direction
//! data rust-i18n does not model.

mod locale;
mod skip;
#[cfg(test)]
mod tests;

use gpui::{App, SharedString};

pub use locale::{Direction, LOCALES, Locale};
pub use skip::SkipLink;

/// Serializes the tests that read or set the process-wide locale. `rust_i18n::set_locale` is global
/// state and the tests run in parallel, so a test that changes it would otherwise move another
/// test's lookups under it.
#[cfg(test)]
pub(crate) static LOCALE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The shown locale's message for `key`, each `%{name}` filled from `args`.
///
/// gpui sets no paragraph direction, so a right-to-left message leads with a
/// right-to-left mark: CoreText would lay out one that starts with Latin letters
/// or a signed number left to right.
pub fn text(_cx: &App, key: &str, args: &[(&str, &str)]) -> SharedString {
    render(&message(key), args, direction()).unwrap_or_else(|error| panic!("{key:?}: {error}"))
}

/// A message with its arguments filled and its reading direction applied.
///
/// Pure on purpose: the direction arrives as an argument rather than being read
/// from the global locale, so this can be tested without touching process state.
fn render(
    message: &str,
    args: &[(&str, &str)],
    direction: Direction,
) -> Result<SharedString, String> {
    let text = fill(message, args)?;
    Ok(match direction {
        Direction::Ltr => text.into(),
        Direction::Rtl => format!("\u{200f}{text}").into(),
    })
}

/// The shown locale's raw message, `%{name}` placeholders still in place.
fn message(key: &str) -> String {
    rust_i18n::t!(key).to_string()
}

/// The shown locale's paragraph direction, left to right when it is not one of
/// ours (an application may show a locale this crate does not list).
fn direction() -> Direction {
    Locale::find(&rust_i18n::locale())
        .map(|locale| locale.direction)
        .unwrap_or(Direction::Ltr)
}

/// The locale every window shows.
pub fn locale() -> SharedString {
    rust_i18n::locale().to_string().into()
}

/// Shows another locale in every window.
pub fn set_locale(tag: &str, cx: &mut App) {
    rust_i18n::set_locale(tag);
    cx.refresh_windows();
}

/// `message` with each `%{name}` replaced by its argument.
fn fill(message: &str, args: &[(&str, &str)]) -> Result<String, String> {
    let mut out = String::with_capacity(message.len());
    let mut rest = message;
    while let Some(open) = rest.find("%{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let close = after.find('}').ok_or("a %{ without }")?;
        let name = &after[..close];
        let (_, value) = args
            .iter()
            .find(|(arg, _)| *arg == name)
            .ok_or_else(|| format!("no argument {name:?}"))?;
        out.push_str(value);
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}
