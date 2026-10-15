use std::ops::Range;

use gpui::{
    AnyElement, Context, FontWeight, HighlightStyle, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, ParentElement, Pixels, Stateful, Styled, StyledText,
    UnderlineStyle, Window, div, prelude::*,
};

use super::{
    cursor::Selection,
    gutter::Cell,
    layout::{Shown, stack},
    state::{CodeEditor, CursorShape, EditorEvent},
    syntax,
    wrap::Wrap,
};
use crate::{
    charts::tint,
    theme::{ActiveTheme, Radius},
};

/// Whether a byte range touches one row's byte range.
///
/// `styles` and `line_row` must agree on this: the underline and the hand cursor are decided in two
/// different functions, and a mismatch shows up as an underlined symbol with no hand cursor (or the
/// reverse). The `max(row.start + 1)` keeps an empty row range from swallowing the symbol that ends
/// exactly at it.
fn range_touches_row(span: &Range<usize>, row: &Range<usize>) -> bool {
    span.start < row.end.max(row.start + 1) && row.start < span.end
}

impl CodeEditor {
    /// A line's own styles: syntax colors, rainbow brackets, problems, washes, matches and the matched pair.
    fn styles(&self, line: usize, cx: &Context<Self>) -> Vec<(Range<usize>, HighlightStyle)> {
        let colors = &cx.theme().colors;
        let range = self.row_range(line);
        let local = |span: &Range<usize>| {
            span.start.max(range.start) - range.start..span.end.min(range.end) - range.start
        };
        let touches = |span: &Range<usize>| range_touches_row(span, &range);
        let mut spans = syntax::colors_in(&self.buffer, range.clone(), cx);
        let brackets = self
            .frame
            .brackets
            .iter()
            .filter(|bracket| range.contains(&bracket.offset));
        for bracket in brackets {
            let at = bracket.offset - range.start;
            let color = if self.options.rainbow {
                tint(colors, bracket.depth % 3)
            } else {
                colors.syntax.punctuation
            };
            spans.retain(|(span, _)| *span != (at..at + 1));
            spans.push((at..at + 1, color.into()));
        }
        for span in self.frame.occurrences.iter().filter(|span| touches(span)) {
            spans.push((
                local(span),
                HighlightStyle {
                    background_color: Some(colors.selection.opacity(0.45)),
                    ..HighlightStyle::default()
                },
            ));
        }
        for (span, color) in self
            .marks
            .backgrounds
            .iter()
            .filter(|(span, _)| touches(span))
        {
            spans.push((
                local(span),
                HighlightStyle {
                    background_color: Some(*color),
                    ..HighlightStyle::default()
                },
            ));
        }
        for diagnostic in self
            .marks
            .diagnostics
            .iter()
            .filter(|diagnostic| touches(&diagnostic.range))
        {
            spans.push((
                local(&diagnostic.range),
                HighlightStyle {
                    underline: Some(UnderlineStyle {
                        thickness: cx.theme().underline_thickness(),
                        color: Some(diagnostic.severity.color(colors)),
                        wavy: true,
                    }),
                    ..HighlightStyle::default()
                },
            ));
        }
        // 按住 Ctrl/Cmd：指针下那个符号画成可跳转的样子（跳转本身由宿主处理，见 `EditorPane::on_editor_click`）。
        // 放在最后压过其它下划线：problem 的波浪线是自己的告警语义，这里只补「能跳」这一条。
        if self.secondary_held {
            if let Some(symbol) = self.hover_symbol.as_ref().filter(|symbol| touches(symbol)) {
                spans.push((
                    local(symbol),
                    HighlightStyle {
                        underline: Some(UnderlineStyle {
                            thickness: cx.theme().underline_thickness(),
                            color: Some(colors.link),
                            wavy: false,
                        }),
                        ..HighlightStyle::default()
                    },
                ));
            }
        }
        if let Some((open, close)) = self.frame.matched {
            for at in [open, close].into_iter().filter(|at| range.contains(at)) {
                let at = at - range.start;
                spans.push((
                    at..at + 1,
                    HighlightStyle {
                        background_color: Some(colors.active),
                        font_weight: Some(FontWeight::BOLD),
                        ..HighlightStyle::default()
                    },
                ));
            }
        }
        stack(spans)
    }

    /// The columns guides stand at on a line: every tab stop inside its indent, or the next line's when it is blank.
    fn guides(&self, line: usize) -> Vec<usize> {
        let indent = (line..self.buffer.lines())
            .find(|at| !self.buffer.line(*at).trim().is_empty())
            .map_or(0, |at| self.buffer.indent(at));
        (0..indent).step_by(self.options.tab).collect()
    }

    /// Dots where the line has spaces and arrows for tabs, spaces elsewhere, so they fall on the text's columns.
    fn whitespace(&self, line: usize, shown: &Shown, cx: &Context<Self>) -> String {
        let text = self.row_text(line);
        let notes = self.notes(line, cx);
        let mut out = String::new();
        let mut note = notes.iter().peekable();
        for (at, ch) in text.char_indices() {
            while let Some((_, words, _)) = note.next_if(|(place, ..)| *place <= at) {
                out.extend(std::iter::repeat_n(' ', words.chars().count()));
            }
            out.push(match ch {
                ' ' => '·',
                '\t' => '→',
                _ => ' ',
            });
        }
        debug_assert!(out.chars().count() <= shown.text.chars().count());
        out
    }

    pub(crate) fn line_row(
        &self,
        base: Stateful<gpui::Div>,
        row: usize,
        line: usize,
        sub: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let (advance, height) = (self.frame.advance, self.frame.line);
        let text = self.row_text(line);
        let range = self.row_range(line);
        let shown = Shown::new(text, self.notes(line, cx), self.styles(line, cx));
        // Which visual sub-row of the line this row draws: its own slice of the drawn text, and where that
        // slice starts among the line's drawn columns. On a line that does not wrap this is the whole line,
        // as it was before soft wrap existed.
        let wraps = self.line_wraps(line, self.frame_hidden());
        let wrap = wraps.get(sub).cloned().unwrap_or(Wrap {
            range: 0..text.len(),
            column: 0,
        });
        let drawn = shown.drawn_range(&wrap.range);
        let (drawn, drawn_text) = (drawn.clone(), shown.text[drawn].to_string());
        let left = shown.text[..drawn.start].chars().count();
        let (first, len) = (wrap.column, drawn_text.chars().count());
        let x = |column: usize| advance * (column.saturating_sub(left) + 1) as f32;
        let focused = self.focus.is_focused(window);
        let primary = self.primary();
        let current = primary.is_empty() && self.buffer.line_of(primary.head) == line;
        let column_of =
            |offset: usize| shown.column(text, self.buffer.point(offset).1.min(text.len()));
        let selections = self.selections.iter().filter_map(|selection| {
            let span = selection.range();
            if span.is_empty() || span.end < range.start || span.start > range.end {
                return None;
            }
            let start = if span.start <= range.start {
                0
            } else {
                column_of(span.start)
            };
            let past = span.end > range.end;
            let end = if past {
                column_of(range.end) + 1
            } else {
                column_of(span.end)
            };
            let clipped = end.min(first + len);
            (clipped > start.max(first)).then(|| {
                div()
                    .absolute()
                    .top_0()
                    .left(x(start.max(first)))
                    .w(advance * (clipped - start.max(first)) as f32)
                    .h(height)
                    .bg(if focused {
                        colors.selection
                    } else {
                        colors.selection.opacity(0.5)
                    })
            })
        });
        let active = self
            .frame
            .folds
            .iter()
            .filter(|(header, end)| {
                let at = self.buffer.line_of(primary.head);
                *header < at && at <= *end
            })
            .map(|(header, _)| self.buffer.indent(*header))
            .next_back();
        let guides = if self.options.guides {
            self.guides(line)
        } else {
            Default::default()
        };
        let rulers = self
            .options
            .rulers
            .iter()
            .filter(|column| (first..first + len).contains(*column))
            .map(|column| {
                div()
                    .absolute()
                    .top_0()
                    .left(x(*column))
                    .h(height)
                    .w_0()
                    .border_l_1()
                    .border_color(colors.border.opacity(0.6))
            });
        let guides = guides
            .into_iter()
            .filter(|column| (first..first + len).contains(column))
            .map(|column| {
                let strong = active == Some(column);
                div()
                    .absolute()
                    .top_0()
                    .left(x(column))
                    .h(height)
                    .w_0()
                    .border_l_1()
                    .border_color(if strong {
                        colors.border_strong
                    } else {
                        colors.border.opacity(0.6)
                    })
            });
        let carets = self
            .selections
            .iter()
            .filter(|selection| self.buffer.line_of(selection.head) == line)
            // A caret draws on the row of its own visual sub-row, so a cursor inside a wrapped line stands
            // under the character it is at rather than at the wrap column of the row that holds the line.
            .filter(|selection| {
                let column = column_of(selection.head);
                (first..=first + len).contains(&column)
            })
            .filter(|_| focused && self.caret_on)
            .map(|selection| {
                let left = x(column_of(selection.head));
                let caret = theme.caret_width().to_pixels(window.rem_size());
                let mark = div().absolute().left(left);
                match self.options.cursor {
                    CursorShape::Line => mark.top_0().w(caret).h(height).bg(colors.focus),
                    CursorShape::Block => mark
                        .top_0()
                        .w(advance)
                        .h(height)
                        .bg(colors.focus.opacity(0.35)),
                    CursorShape::Underline => mark
                        .top(height - caret * 1.5)
                        .w(advance)
                        .h(caret * 1.5)
                        .bg(colors.focus),
                }
            });
        // Whitespace marks are the line's own, so they draw in the first sub-row and nowhere else.
        let dots = (self.options.whitespace && sub == 0).then(|| {
            div()
                .absolute()
                .top_0()
                .left(x(0))
                .h(height)
                .line_height(height)
                .whitespace_nowrap()
                .text_color(colors.fg_subtle.opacity(0.6))
                .child(self.whitespace(line, &shown, cx))
        });
        // The fold marker stands at the end of the line's text, which is only drawn on its last sub-row.
        let folded = (self.folded.contains(&line) && sub + 1 == wraps.len()).then(|| {
            let entity = cx.entity();
            div()
                .id(("unfold", line))
                .absolute()
                .top_0()
                .left(x(first + len + 1))
                .h(height)
                .flex()
                .items_center()
                .child(
                    div()
                        .px_1()
                        .rounded(theme.radius(Radius::Sm))
                        .bg(colors.hover)
                        .text_color(colors.fg_muted)
                        .line_height(height * 0.8)
                        .child("⋯"),
                )
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    entity.update(cx, |editor, cx| editor.toggle_fold(line, cx));
                })
        });
        let added = self
            .marks
            .hunks
            .iter()
            .any(|hunk| (hunk.line..hunk.line + hunk.added).contains(&line));
        // 按住 Ctrl/Cmd 且指针正停在这一行的符号上：整行给手型光标（行的命中区就是这一行，
        // 所以只能整行给——按字符给光标要另做一层 hitbox，收益不抵复杂度）。
        let jump_target = self.secondary_held
            && self
                .hover_symbol
                .as_ref()
                .is_some_and(|symbol| range_touches_row(symbol, &range));
        let code = div()
            .id(("code", row))
            .relative()
            .flex_1()
            .h_full()
            .overflow_hidden()
            .when(jump_target, |code| code.cursor_pointer())
            .when(current && focused, |code| code.bg(colors.hover))
            .when(added, |code| code.bg(colors.success.opacity(0.08)))
            .children(selections)
            .children(guides)
            .children(rulers)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left(x(0))
                    .h(height)
                    .line_height(height)
                    .text_color(colors.syntax.variable)
                    .child(
                        StyledText::new(drawn_text.clone()).with_highlights(drawn_styles(
                            &shown,
                            &drawn,
                            &drawn_text,
                        )),
                    ),
            )
            .children(dots)
            .children(folded)
            .children(carets)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |editor, event: &MouseDownEvent, window, cx| {
                    editor.press(line, sub, event, window, cx)
                }),
            )
            .on_mouse_move(cx.listener(move |editor, event: &MouseMoveEvent, _, cx| {
                editor.drag(line, sub, event, cx);
                editor.track_hover(line, sub, event, cx);
            }));
        let cell = if sub == 0 {
            Cell::Line(line)
        } else {
            Cell::Blank
        };
        base.child(self.gutter(cell, row, cx))
            .child(code)
            .into_any_element()
    }

    /// A press on a line: one caret, a word on a double press, the line on a triple; Shift extends, Alt adds a caret.
    ///
    /// `sub` names the visual sub-row the press landed on, so the caret goes where the pointer points on a
    /// wrapped line rather than to the line's first columns.
    pub(crate) fn press(
        &mut self,
        line: usize,
        sub: usize,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        let at = self.pressed_offset(line, sub, event.position.x, cx);
        let chosen = match event.click_count {
            2 => {
                let word = self.buffer.word_at(at);
                vec![Selection {
                    anchor: word.start,
                    head: word.end,
                    goal: None,
                }]
            }
            3.. => {
                let range = self.buffer.line_range(line);
                vec![Selection {
                    anchor: range.start,
                    head: self.buffer.next(range.end),
                    goal: None,
                }]
            }
            _ if event.modifiers.shift => {
                let mut all = self.selections.clone();
                let primary = all.last_mut().expect("an editor keeps a cursor");
                primary.head = at;
                primary.goal = None;
                all
            }
            _ if event.modifiers.alt => {
                let mut all = self.selections.clone();
                all.push(Selection::caret(at));
                all
            }
            _ => {
                self.dragging = true;
                vec![Selection::caret(at)]
            }
        };
        self.set_selections(chosen, cx);
    }

    /// A drag over a line stretches the primary selection to where the pointer is.
    pub(crate) fn drag(
        &mut self,
        line: usize,
        sub: usize,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if !self.dragging || !event.dragging() {
            return;
        }
        let at = self.pressed_offset(line, sub, event.position.x, cx);
        let mut all = self.selections.clone();
        let primary = all.last_mut().expect("an editor keeps a cursor");
        if primary.head == at {
            return;
        }
        primary.head = at;
        primary.goal = None;
        self.set_selections(all, cx);
    }

    /// Asks the owner for a hover ([`EditorEvent::HoverRequested`]) while the pointer rests on a symbol: one
    /// symbol is asked for once, whitespace and punctuation ask for nothing, and a selection being dragged asks
    /// for nothing.
    ///
    /// The library only reports where the pointer stopped; fetching the content and taking a hover down are the
    /// owner's. Leaving a symbol raises nothing (see [`EditorEvent::HoverRequested`]) — the owner times its own
    /// dismissal.
    pub(crate) fn track_hover(
        &mut self,
        line: usize,
        sub: usize,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if self.dragging {
            return;
        }
        let at = self.pressed_offset(line, sub, event.position.x, cx);
        let word = self.buffer.word_at(at);
        let symbol = (!word.is_empty()
            && self.buffer.text()[word.clone()]
                .chars()
                .all(|ch| ch.is_alphanumeric() || ch == '_'))
        .then_some(word);
        if symbol == self.hover_symbol {
            return;
        }
        if let Some(word) = symbol.as_ref() {
            log::debug!("code editor: hover asked at {}", word.start);
            cx.emit(EditorEvent::HoverRequested(word.start));
        }
        self.hover_symbol = symbol;
    }

    /// The offset a pointer at window x stands over, read on the visual sub-row it landed on.
    pub(crate) fn pressed_offset(
        &self,
        line: usize,
        sub: usize,
        x: Pixels,
        cx: &Context<Self>,
    ) -> usize {
        let wraps = self.line_wraps(line, self.frame_hidden());
        let Some(wrap) = wraps.get(sub) else {
            return self.buffer.line_range(line).start;
        };
        self.line_offset(line, wrap, x, cx).0
    }
}

/// One sub-row's highlights: the line's own styles, cut to the sub-row's slice of the drawn text.
///
/// A style that reaches across a break colors each sub-row it touches, which is what an underline or a search
/// wash over a wrapped match should do.
fn drawn_styles(
    shown: &Shown,
    drawn: &std::ops::Range<usize>,
    drawn_text: &str,
) -> Vec<(std::ops::Range<usize>, gpui::HighlightStyle)> {
    shown
        .styles
        .iter()
        .filter_map(|(span, style)| {
            let start = span.start.max(drawn.start) - drawn.start;
            let end = span.end.min(drawn.end).saturating_sub(drawn.start);
            (start < end && drawn_text.get(start..end).is_some()).then(|| (start..end, *style))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::range_touches_row;
    use std::ops::Range;

    fn touches(span: Range<usize>, row: Range<usize>) -> bool {
        range_touches_row(&span, &row)
    }

    #[test]
    fn a_symbol_inside_the_row_touches_it() {
        assert!(touches(10..14, 0..40));
    }

    #[test]
    fn a_symbol_ending_exactly_where_the_row_starts_does_not_touch_it() {
        // 上一行末尾的符号不该把下一行也标成可跳转。
        assert!(!touches(0..10, 10..40));
        assert!(!touches(0..10, 10..10), "空行的行范围不该被上一个符号吃掉");
    }

    #[test]
    fn a_symbol_spanning_several_rows_touches_each_of_them() {
        assert!(touches(5..100, 0..40));
        assert!(touches(5..100, 40..80));
        assert!(touches(5..100, 80..90));
        assert!(!touches(5..100, 100..140));
    }

    #[test]
    fn a_symbol_that_starts_on_a_later_row_does_not_touch_this_one() {
        assert!(!touches(50..60, 0..40));
    }
}

