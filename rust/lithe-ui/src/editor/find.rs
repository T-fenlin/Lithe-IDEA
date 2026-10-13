use std::{ops::Range, rc::Rc};

use gpui::{
    App, ElementId, Entity, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
    div, prelude::*,
};

use crate::{
    buttons::{Button, ButtonVariant, ToggleButton, ToggleItem},
    forms::{Input, InputState},
    i18n,
    primitives::{IconName, Tooltip},
    theme::{ActiveTheme, Elevation, Radius, Size, TextSize},
};

/// How a search matches: case exactly, whole words only, the query as a regular expression.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindOptions {
    pub case: bool,
    pub word: bool,
    pub regex: bool,
}

/// Where `query` appears in `text` under `options`; a pattern that does not compile is an error to show.
pub fn find_all(
    text: &str,
    query: &str,
    options: FindOptions,
) -> Result<Vec<Range<usize>>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let pattern = if options.regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    let pattern = if options.word {
        format!(r"\b(?:{pattern})\b")
    } else {
        pattern
    };
    let found = regex::RegexBuilder::new(&pattern)
        .case_insensitive(!options.case)
        .multi_line(true)
        .build()
        .map_err(|error| error.to_string())?;
    Ok(found
        .find_iter(text)
        .filter(|hit| !hit.is_empty())
        .map(|hit| hit.range())
        .collect())
}

type OnOptions = Rc<dyn Fn(FindOptions, &mut Window, &mut App)>;
type OnBool = Rc<dyn Fn(bool, &mut Window, &mut App)>;
type OnClose = Rc<dyn Fn(&mut Window, &mut App)>;

/// A find box: the query, its options when the owner takes them, where the current match stands among all, steps between them, and a replace row that folds away when the owner offers one.
#[derive(IntoElement)]
pub struct FindWidget {
    id: ElementId,
    find: Entity<InputState>,
    replace: Option<Entity<InputState>>,
    options: FindOptions,
    matches: Result<(Option<usize>, usize), SharedString>,
    on_options: Option<OnOptions>,
    on_step: Option<OnBool>,
    on_replace: Option<OnBool>,
    on_toggle_replace: Option<OnBool>,
    on_close: Option<OnClose>,
}

impl FindWidget {
    /// `find` holds the query; `current` is the chosen match among `total`.
    pub fn new(
        id: impl Into<ElementId>,
        find: &Entity<InputState>,
        current: Option<usize>,
        total: usize,
    ) -> Self {
        Self {
            id: id.into(),
            find: find.clone(),
            replace: None,
            options: FindOptions::default(),
            matches: Ok((current, total)),
            on_options: None,
            on_step: None,
            on_replace: None,
            on_toggle_replace: None,
            on_close: None,
        }
    }

    /// The query failed to compile. The widget shows a short word in the count cell and carries
    /// the raw `regex::Error` message in its tooltip, so long messages are readable but the cell
    /// stays a fixed width.
    pub fn error(mut self, message: impl Into<SharedString>) -> Self {
        self.matches = Err(message.into());
        self
    }

    pub fn options(mut self, options: FindOptions) -> Self {
        self.options = options;
        self
    }

    /// Shows the replace row, its text in `field`.
    pub fn replace(mut self, field: &Entity<InputState>) -> Self {
        self.replace = Some(field.clone());
        self
    }

    pub fn on_options(
        mut self,
        handler: impl Fn(FindOptions, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_options = Some(Rc::new(handler));
        self
    }

    /// Gets `true` for the next match, `false` for the previous.
    pub fn on_step(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_step = Some(Rc::new(handler));
        self
    }

    /// Gets `true` to replace every match, `false` for the current one.
    pub fn on_replace(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_replace = Some(Rc::new(handler));
        self
    }

    /// Gets whether the replace row should show.
    pub fn on_toggle_replace(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_replace = Some(Rc::new(handler));
        self
    }

    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for FindWidget {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let options = self.options;
        // Every visible word comes from the catalogue, so one `set_locale` moves the whole bar.
        let toggle_replace = i18n::text(cx, "editor.find.toggleReplace", &[]);
        let match_case = i18n::text(cx, "editor.find.matchCase", &[]);
        let whole_word = i18n::text(cx, "editor.find.wholeWord", &[]);
        let regular_expression = i18n::text(cx, "editor.find.regularExpression", &[]);
        let no_results = i18n::text(cx, "editor.find.noResults", &[]);
        let invalid_pattern = i18n::text(cx, "editor.find.invalidPattern", &[]);
        let previous_match = i18n::text(cx, "editor.find.previousMatch", &[]);
        let next_match = i18n::text(cx, "editor.find.nextMatch", &[]);
        let close = i18n::text(cx, "editor.find.close", &[]);
        let replace_word = i18n::text(cx, "editor.find.replace", &[]);
        let replace_all_word = i18n::text(cx, "editor.find.replaceAll", &[]);
        let query_field = i18n::text(cx, "editor.find.queryField", &[]);
        let replace_field = i18n::text(cx, "editor.find.replaceField", &[]);
        let toggle = |key: &'static str,
                      icon: IconName,
                      words: SharedString,
                      on: bool,
                      set: fn(&mut FindOptions, bool)| {
            let on_options = self.on_options.clone();
            ToggleButton::new(
                (self.id.clone(), key),
                ToggleItem::new(key).icon(icon).tooltip(words),
                on,
            )
            .size(Size::Small)
            .on_toggle(move |on, window, cx| {
                let mut next = options;
                set(&mut next, on);
                if let Some(on_options) = &on_options {
                    on_options(next, window, cx);
                }
            })
        };
        // The count cell shows the short word for a broken pattern; the compiler's own message
        // (English, possibly several lines) hangs off it as a tooltip instead of bursting the
        // fixed-width cell.
        // An empty query is idle, not "no results": the cell stays blank (its width is fixed, so
        // the row does not jump) rather than telling the user nothing matched.
        let idle = self.find.read(cx).text().is_empty();
        let (count, missing, refusal) = match &self.matches {
            Ok((_, 0)) if idle => (SharedString::from(""), false, None),
            Ok((_, 0)) => (no_results.clone(), true, None),
            Ok((Some(current), total)) => (
                i18n::text(
                    cx,
                    "editor.find.matchCount",
                    &[
                        ("current", &(current + 1).to_string()),
                        ("total", &total.to_string()),
                    ],
                ),
                false,
                None,
            ),
            Ok((None, total)) => (
                i18n::text(cx, "editor.find.found", &[("total", &total.to_string())]),
                false,
                None,
            ),
            Err(message) => (invalid_pattern.clone(), true, Some(message.clone())),
        };
        let found = matches!(self.matches, Ok((_, total)) if total > 0);
        let icon_button = |key: &'static str,
                           icon: IconName,
                           words: SharedString,
                           enabled: bool,
                           action: Option<OnBool>,
                           value: bool| {
            Button::new((self.id.clone(), key))
                .icon(icon)
                .variant(ButtonVariant::Ghost)
                .size(Size::Small)
                .tooltip(words)
                .disabled(!enabled)
                .when_some(action, move |button, action| {
                    button.on_click(move |_, window, cx| action(value, window, cx))
                })
        };
        let showing = self.replace.is_some();
        let row = || div().flex().items_center().gap_1();
        div()
            .w(theme.label_width() * 2.75)
            .flex()
            .flex_col()
            .gap_1()
            .p_1()
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(colors.border)
            .bg(colors.overlay)
            .shadow(theme.elevation(Elevation::Floating))
            .text_size(theme.text_size(TextSize::Xs))
            .child(
                row()
                    .children(self.on_toggle_replace.clone().map(|fold| {
                        icon_button(
                            "fold",
                            if showing {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            },
                            toggle_replace.clone(),
                            true,
                            Some(fold),
                            !showing,
                        )
                    }))
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(&self.find)
                                .size(Size::Small)
                                .aria_label(query_field.clone()),
                        ),
                    )
                    .when(self.on_options.is_some(), |row| {
                        row.child(toggle(
                            "case",
                            IconName::CaseSensitive,
                            match_case.clone(),
                            options.case,
                            |next, on| next.case = on,
                        ))
                        .child(toggle(
                            "word",
                            IconName::WholeWord,
                            whole_word.clone(),
                            options.word,
                            |next, on| next.word = on,
                        ))
                        .child(toggle(
                            "regex",
                            IconName::Regex,
                            regular_expression.clone(),
                            options.regex,
                            |next, on| next.regex = on,
                        ))
                    })
                    .child(
                        div()
                            .id((self.id.clone(), "count"))
                            .w(theme.label_width() * 0.5)
                            .text_center()
                            .truncate()
                            .whitespace_nowrap()
                            .overflow_hidden()
                            .text_color(if missing {
                                colors.danger
                            } else {
                                colors.fg_muted
                            })
                            .when_some(refusal, |cell, reason| cell.tooltip(Tooltip::text(reason)))
                            .child(count),
                    )
                    .child(icon_button(
                        "previous",
                        IconName::ArrowUp,
                        previous_match.clone(),
                        found,
                        self.on_step.clone(),
                        false,
                    ))
                    .child(icon_button(
                        "next",
                        IconName::ArrowDown,
                        next_match.clone(),
                        found,
                        self.on_step.clone(),
                        true,
                    ))
                    .children(self.on_close.clone().map(|handler| {
                        Button::new((self.id.clone(), "close"))
                            .icon(IconName::X)
                            .variant(ButtonVariant::Ghost)
                            .size(Size::Small)
                            .tooltip(close.clone())
                            .on_click(move |_, window, cx| handler(window, cx))
                    })),
            )
            .children(self.replace.as_ref().map(|field| {
                let (one, all) = (self.on_replace.clone(), self.on_replace.clone());
                row()
                    .when(self.on_toggle_replace.is_some(), |row| {
                        // A slot as wide as the fold button, so the replace field lines up with the
                        // query field; the row's own `gap_1` supplies the rest of the inset.
                        row.child(div().flex_none().w(theme.control_height(Size::Small)))
                    })
                    .child(
                        div().flex_1().min_w_0().child(
                            Input::new(field)
                                .size(Size::Small)
                                .aria_label(replace_field.clone()),
                        ),
                    )
                    .child(
                        Button::new((self.id.clone(), "replace"))
                            .label(replace_word.clone())
                            .variant(ButtonVariant::Ghost)
                            .size(Size::Small)
                            .disabled(!found)
                            .when_some(one, |button, one| {
                                button.on_click(move |_, window, cx| one(false, window, cx))
                            }),
                    )
                    .child(
                        Button::new((self.id.clone(), "all"))
                            .label(replace_all_word.clone())
                            .variant(ButtonVariant::Ghost)
                            .size(Size::Small)
                            .disabled(!found)
                            .when_some(all, |button, all| {
                                button.on_click(move |_, window, cx| all(true, window, cx))
                            }),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_by_case_word_and_pattern() {
        let text = "Let total = total_cost + TOTAL;";
        let plain = FindOptions::default();
        assert_eq!(
            find_all(text, "total", plain).map(|found| found.len()),
            Ok(3)
        );
        let case = FindOptions {
            case: true,
            ..plain
        };
        assert_eq!(
            find_all(text, "total", case).map(|found| found.len()),
            Ok(2)
        );
        let word = FindOptions {
            word: true,
            ..plain
        };
        assert_eq!(find_all(text, "total", word), Ok(vec![4..9, 25..30]));
        let pattern = FindOptions {
            regex: true,
            ..plain
        };
        let word = 12..22;
        assert_eq!(find_all(text, r"total_\w+", pattern), Ok(vec![word]));
        assert!(
            find_all(text, "(", pattern).is_err(),
            "a broken pattern says so"
        );
        assert_eq!(
            find_all(text, "(", plain),
            Ok(Vec::new()),
            "and plain text is escaped"
        );
    }

    /// The catalogue is where the bar's words come from: English is what it hardcoded, so the
    /// default words do not change, and another locale moves them. The process-wide locale is
    /// shared, so this test takes the same lock the catalogue's own locale tests hold.
    #[cfg(feature = "test-support")]
    #[gpui::test]
    fn its_words_come_from_the_catalogue_and_follow_the_locale(cx: &mut gpui::TestAppContext) {
        let _guard = crate::i18n::LOCALE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = rust_i18n::locale().to_string();
        struct Restore(String);
        impl Drop for Restore {
            fn drop(&mut self) {
                rust_i18n::set_locale(&self.0);
            }
        }
        let _restore = Restore(previous);

        let word = |cx: &mut gpui::TestAppContext, locale: &str, key: &str| {
            rust_i18n::set_locale(locale);
            cx.read(|cx| i18n::text(cx, key, &[]).to_string())
        };
        let filled =
            |cx: &mut gpui::TestAppContext, locale: &str, key: &str, args: &[(&str, &str)]| {
                rust_i18n::set_locale(locale);
                cx.read(|cx| i18n::text(cx, key, args).to_string())
            };

        // English is the copy the widget used to hardcode, key by key.
        assert_eq!(word(cx, "en-US", "editor.find.queryField"), "Find");
        assert_eq!(
            word(cx, "en-US", "editor.find.replaceField"),
            "Replace with"
        );
        assert_eq!(
            word(cx, "en-US", "editor.find.toggleReplace"),
            "Toggle replace"
        );
        assert_eq!(word(cx, "en-US", "editor.find.matchCase"), "Match case");
        assert_eq!(word(cx, "en-US", "editor.find.wholeWord"), "Whole word");
        assert_eq!(
            word(cx, "en-US", "editor.find.regularExpression"),
            "Regular expression"
        );
        assert_eq!(word(cx, "en-US", "editor.find.noResults"), "No results");
        assert_eq!(
            word(cx, "en-US", "editor.find.invalidPattern"),
            "Invalid regex"
        );
        assert_eq!(
            word(cx, "en-US", "editor.find.previousMatch"),
            "Previous match"
        );
        assert_eq!(word(cx, "en-US", "editor.find.nextMatch"), "Next match");
        assert_eq!(word(cx, "en-US", "editor.find.close"), "Close");
        assert_eq!(word(cx, "en-US", "editor.find.replace"), "Replace");
        assert_eq!(word(cx, "en-US", "editor.find.replaceAll"), "Replace all");
        assert_eq!(
            filled(
                cx,
                "en-US",
                "editor.find.matchCount",
                &[("current", "2"), ("total", "5")]
            ),
            "2 of 5"
        );
        assert_eq!(
            filled(cx, "en-US", "editor.find.found", &[("total", "5")]),
            "5 found"
        );

        // Another locale reads differently — both a bare word and a filled count.
        assert_eq!(word(cx, "zh-CN", "editor.find.noResults"), "无结果");
        assert_eq!(
            filled(
                cx,
                "zh-CN",
                "editor.find.matchCount",
                &[("current", "2"), ("total", "5")]
            ),
            "第 2 / 共 5"
        );
        assert_ne!(
            word(cx, "zh-CN", "editor.find.close"),
            word(cx, "en-US", "editor.find.close")
        );
    }
}
