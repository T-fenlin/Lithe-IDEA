use std::ops::Range;

use gpui::{ClipboardItem, Context};

use super::{
    buffer::{Buffer, clamped_range},
    cursor::{Motion, Selection, merged, replace_each},
    state::CodeEditor,
};

/// Pairs typing an opener closes at once.
const PAIRS: [(char, char); 5] = [('(', ')'), ('[', ']'), ('{', '}'), ('"', '"'), ('\'', '\'')];

impl CodeEditor {
    /// Replaces each selection with `text`; typing in one burst makes one undo step.
    pub(crate) fn insert(&mut self, text: &str, typing: bool, cx: &mut Context<Self>) {
        let edits = self
            .selections
            .iter()
            .map(|selection| (selection.range(), text.to_string()))
            .collect();
        self.apply(edits, typing, cx);
    }

    /// Replaces ranges all at once and leaves a caret after each insert.
    pub(crate) fn apply(
        &mut self,
        edits: Vec<(Range<usize>, String)>,
        typing: bool,
        cx: &mut Context<Self>,
    ) {
        if !self.editable() {
            log::debug!("code editor: read only, edit dropped");
            return;
        }
        if edits.is_empty() {
            return;
        }
        let before = self.snapshot();
        let primary = self.primary().range().start;
        let edits = joined(edits);
        let lead = edits
            .iter()
            .rposition(|(range, _)| range.start <= primary)
            .unwrap_or(0);
        let mut carets = replace_each(&mut self.buffer, edits);
        let lead = carets.remove(lead);
        carets.push(lead);
        self.selections = merged(carets);
        self.marked = None;
        self.commit(before, typing, cx);
        self.reveal(true, cx);
    }

    /// Applies a batch of the owner's text edits (a language server's `TextEdit`: byte-range replacements) as
    /// one undo step; read-only text refuses.
    ///
    /// Ranges are applied **bottom-up**, so an earlier edit cannot move a later one's offsets, and every range
    /// is clamped onto whole characters of the text as it stands **at that moment** — an out-of-bounds,
    /// character-splitting or even overlapping batch lands safely instead of panicking (where ranges overlap,
    /// the one applied later, i.e. the earlier one, wins). Afterwards the primary cursor sits where the edits
    /// left it: a cursor inside an edit stops at the end of that edit's replacement (a completion, a rename),
    /// and edits above it shift it (a new import line). Only a real text change counts as a step and raises
    /// [`super::EditorEvent::Changed`].
    pub fn apply_edits(&mut self, edits: &[(Range<usize>, String)], cx: &mut Context<Self>) {
        if !self.editable() {
            log::debug!("code editor: read only, LSP edits dropped");
            return;
        }
        if edits.is_empty() {
            return;
        }
        let mut applied: Vec<(Range<usize>, String)> = edits
            .iter()
            .map(|(range, text)| {
                (
                    clamped_range(self.buffer.text(), range.clone()),
                    text.clone(),
                )
            })
            .collect();
        applied.sort_by(|a, b| b.0.start.cmp(&a.0.start).then(b.0.end.cmp(&a.0.end)));
        let caret = caret_after(self.cursor_offset(), &applied);
        let before = self.snapshot();
        for (range, replacement) in &applied {
            let range = clamped_range(self.buffer.text(), range.clone());
            self.buffer.replace(range, replacement);
        }
        self.marked = None;
        let caret = clamped_range(self.buffer.text(), caret..caret).start;
        self.commit(before, false, cx);
        self.set_selections(vec![Selection::caret(caret)], cx);
        self.reveal(true, cx);
    }

    /// Types text at each cursor; an opener brings its closer, and a closer already there is stepped over.
    pub(crate) fn type_text(&mut self, text: &str, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let mut chars = text.chars();
        let (Some(ch), None) = (chars.next(), chars.next()) else {
            self.insert(text, true, cx);
            return self.request_after_typing(text, cx);
        };
        let next = |caret: usize| self.buffer.text()[caret..].chars().next();
        let all_empty = self.selections.iter().all(Selection::is_empty);
        if all_empty
            && PAIRS.iter().any(|(_, close)| *close == ch)
            && self
                .selections
                .iter()
                .all(|selection| next(selection.head) == Some(ch))
        {
            return self.motion(Motion::Right, false, cx);
        }
        let Some((open, close)) = PAIRS.iter().find(|(open, _)| *open == ch) else {
            self.insert(text, true, cx);
            return self.request_after_typing(text, cx);
        };
        let free = |caret: usize| {
            next(caret).is_none_or(|after| after.is_whitespace() || ")]}".contains(after))
        };
        if !all_empty || !self.selections.iter().all(|selection| free(selection.head)) {
            self.insert(text, true, cx);
            return self.request_after_typing(text, cx);
        }
        let edits = self
            .selections
            .iter()
            .map(|selection| (selection.range(), format!("{open}{close}")))
            .collect();
        self.apply(edits, true, cx);
        let back: Vec<Selection> = self
            .selections
            .iter()
            .map(|caret| Selection::caret(caret.head - close.len_utf8()))
            .collect();
        self.set_selections(back, cx);
        self.request_after_typing(text, cx);
    }

    /// Deletes each selection, or what `reach` finds from each bare caret.
    pub(crate) fn delete_by(
        &mut self,
        reach: fn(&Buffer, usize) -> Range<usize>,
        cx: &mut Context<Self>,
    ) {
        if !self.editable() {
            return;
        }
        let edits = self
            .selections
            .iter()
            .map(|selection| {
                let range = if selection.is_empty() {
                    reach(&self.buffer, selection.head)
                } else {
                    selection.range()
                };
                (range, String::new())
            })
            .collect();
        self.apply(edits, false, cx);
    }

    /// A new line at each cursor, indented like its own; one step deeper after an opener, with the closer on a line of its own.
    pub(crate) fn newline(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let tab = self.options.tab;
        let edits = self
            .selections
            .iter()
            .map(|selection| {
                let at = selection.range().start;
                let line = self.buffer.line_of(at);
                let indent: String = self
                    .buffer
                    .line(line)
                    .chars()
                    .take(self.buffer.indent(line))
                    .collect();
                let before = self.buffer.text()[..at].trim_end().chars().next_back();
                let after = self.buffer.text()[selection.range().end..].chars().next();
                let opens = before.is_some_and(|ch| "{[(:".contains(ch));
                let text = match (opens, after) {
                    (true, Some(close)) if "}])".contains(close) => {
                        format!("\n{indent}{}\n{indent}", " ".repeat(tab))
                    }
                    (true, _) => format!("\n{indent}{}", " ".repeat(tab)),
                    _ => format!("\n{indent}"),
                };
                (selection.range(), text)
            })
            .collect::<Vec<_>>();
        let closers: Vec<bool> = edits
            .iter()
            .map(|(_, text)| text.matches('\n').count() == 2)
            .collect();
        self.apply(edits, false, cx);
        let placed: Vec<Selection> = self
            .selections
            .iter()
            .zip(closers)
            .map(|(caret, split)| {
                if split {
                    let line = self.buffer.line_of(caret.head);
                    Selection::caret(self.buffer.line_range(line - 1).end)
                } else {
                    *caret
                }
            })
            .collect();
        self.set_selections(placed, cx);
    }

    /// The lines the selections touch, each once.
    fn touched(&self) -> Vec<usize> {
        let mut lines: Vec<usize> = self
            .selections
            .iter()
            .flat_map(|selection| {
                let range = selection.range();
                let last = if range.end > range.start && self.buffer.point(range.end).1 == 0 {
                    range.end - 1
                } else {
                    range.end
                };
                self.buffer.line_of(range.start)..=self.buffer.line_of(last)
            })
            .collect();
        lines.sort_unstable();
        lines.dedup();
        lines
    }

    /// Tab: takes ghost text when there is some, indents touched lines when a selection spans lines, or fills to the next tab stop.
    pub(crate) fn indent(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        if let Some(ghost) = self.marks.ghost.take() {
            log::info!("code editor: ghost text taken");
            self.set_selections(vec![Selection::caret(ghost.offset)], cx);
            return self.insert(&ghost.text, false, cx);
        }
        let tab = self.options.tab;
        let spans_lines = self.selections.iter().any(|selection| {
            self.buffer.line_of(selection.range().start)
                != self.buffer.line_of(selection.range().end)
        });
        if !spans_lines {
            let edits = self
                .selections
                .iter()
                .map(|selection| {
                    let column = self.buffer.point(selection.range().start).1;
                    (selection.range(), " ".repeat(tab - column % tab))
                })
                .collect();
            return self.apply(edits, false, cx);
        }
        let kept = self.selections.clone();
        let edits = self
            .touched()
            .into_iter()
            .filter(|line| !self.buffer.line(*line).trim().is_empty())
            .map(|line| {
                let start = self.buffer.line_range(line).start;
                (start..start, " ".repeat(tab))
            })
            .collect();
        self.shift_lines(edits, kept, cx);
    }

    /// Shift-Tab: takes one tab stop of indent off each touched line.
    pub(crate) fn outdent(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let tab = self.options.tab;
        let kept = self.selections.clone();
        let edits = self
            .touched()
            .into_iter()
            .filter_map(|line| {
                let start = self.buffer.line_range(line).start;
                let cut = self.buffer.indent(line).min(tab);
                (cut > 0).then(|| (start..start + cut, String::new()))
            })
            .collect();
        self.shift_lines(edits, kept, cx);
    }

    /// Applies line-start edits and keeps each selection over the same text.
    fn shift_lines(
        &mut self,
        edits: Vec<(Range<usize>, String)>,
        kept: Vec<Selection>,
        cx: &mut Context<Self>,
    ) {
        let moves: Vec<(std::ops::Range<usize>, isize)> = edits
            .iter()
            .map(|(range, text)| (range.clone(), text.len() as isize - range.len() as isize))
            .collect();
        let shift = |offset: usize| -> usize {
            let mut moved = offset as isize;
            for (range, delta) in &moves {
                if range.start < offset {
                    if offset < range.end {
                        moved -= (offset - range.start) as isize;
                    } else {
                        moved += delta;
                    }
                }
            }
            moved.max(0) as usize
        };
        let kept: Vec<Selection> = kept
            .into_iter()
            .map(|selection| Selection {
                anchor: shift(selection.anchor),
                head: shift(selection.head),
                goal: None,
            })
            .collect();
        self.apply(edits, false, cx);
        self.set_selections(kept, cx);
    }

    /// Comments the touched lines with `//`, or takes the comment off when every one has it.
    pub(crate) fn toggle_comment(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let lines: Vec<usize> = self
            .touched()
            .into_iter()
            .filter(|line| !self.buffer.line(*line).trim().is_empty())
            .collect();
        let commented = |line: usize| self.buffer.line(line).trim_start().starts_with("//");
        let kept = self.selections.clone();
        let edits = if !lines.is_empty() && lines.iter().all(|line| commented(*line)) {
            lines
                .iter()
                .map(|line| {
                    let start = self.buffer.line_range(*line).start + self.buffer.indent(*line);
                    let text = &self.buffer.text()[start..];
                    let cut = if text.starts_with("// ") { 3 } else { 2 };
                    (start..start + cut, String::new())
                })
                .collect()
        } else {
            let column = lines
                .iter()
                .map(|line| self.buffer.indent(*line))
                .min()
                .unwrap_or(0);
            lines
                .iter()
                .map(|line| {
                    let at = self.buffer.offset(*line, column);
                    (at..at, "// ".to_string())
                })
                .collect()
        };
        self.shift_lines(edits, kept, cx);
    }

    /// Duplicates each touched line below itself; every copy keeps the line's own text and the
    /// selections stay on the original lines (what IDEA leaves behind after `Duplicate Line`).
    pub(crate) fn duplicate_line(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let kept = self.selections.clone();
        let edits = self
            .touched()
            .into_iter()
            .filter_map(|line| {
                let range = self.buffer.line_range(line);
                let slice = self.buffer.text().get(range.clone())?;
                Some((range.end..range.end, format!("\n{slice}")))
            })
            .collect();
        self.shift_lines(edits, kept, cx);
    }

    /// Deletes each touched line with its line break; on the last line, which has no break of its
    /// own, the break **before** it goes too so no empty line is left at the end of the text.
    pub(crate) fn delete_line(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let lines = self.touched();
        let kept = self.selections.clone();
        let text_len = self.buffer.len();
        let last_line = self.buffer.lines() - 1;
        // Taking the previous break only stays correct when nothing before it is being deleted in
        // the same step: two adjacent deletions would otherwise overlap and the batch would be
        // rejected whole.
        let alone_on_last = lines.len() == 1 && lines[0] == last_line;
        let edits = lines
            .into_iter()
            .filter_map(|line| {
                let range = self.buffer.line_range(line);
                if range.end < text_len {
                    Some((range.start..self.buffer.next(range.end), String::new()))
                } else if range.start > 0 && alone_on_last {
                    Some((self.buffer.previous(range.start)..range.end, String::new()))
                } else {
                    Some((range.start..range.end, String::new()))
                }
            })
            .collect();
        self.shift_lines(edits, kept, cx);
    }

    /// A line's bytes together with its line break, when it has one.
    fn block(&self, line: usize) -> (Range<usize>, String) {
        let range = self.buffer.line_range(line);
        let end = if range.end < self.buffer.len() {
            self.buffer.next(range.end)
        } else {
            range.end
        };
        let text = self.buffer.text()[range.start..end].to_string();
        (range.start..end, text)
    }

    /// Swaps two neighbouring blocks of lines, breaks included.
    ///
    /// A break belongs to the line it ends, so when the lower block runs to the end of the text it has
    /// none: after the swap that block no longer ends the text and grows one, while the upper block
    /// takes the end and drops its own. Selections are **not** touched here — the callers know where
    /// the moved block went and place their own.
    fn swap_blocks(
        &mut self,
        upper: (Range<usize>, String),
        lower: (Range<usize>, String),
        cx: &mut Context<Self>,
    ) {
        debug_assert_eq!(
            upper.0.end, lower.0.start,
            "only neighbouring blocks swap, so the edits cannot overlap"
        );
        let edits = if lower.1.ends_with('\n') || !upper.1.ends_with('\n') {
            vec![(upper.0, lower.1), (lower.0, upper.1)]
        } else {
            vec![
                (upper.0.start..upper.0.end, format!("{}\n", lower.1)),
                (
                    upper.0.end..lower.0.end,
                    upper.1.trim_end_matches('\n').to_string(),
                ),
            ]
        };
        self.apply(edits, false, cx);
    }

    /// Moves the touched lines up one, swapping them with the line above. A swap keeps the text
    /// length, so selections outside the moved block need no shift; the ones inside ride along.
    pub(crate) fn move_line_up(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let lines = self.touched();
        let (Some(&first), Some(&last)) = (lines.first(), lines.last()) else {
            return;
        };
        if first == 0 {
            return;
        }
        let (previous_range, previous_text) = self.block(first - 1);
        let swap_at = previous_range.start;
        let block_start = self.buffer.line_range(first).start;
        let block_end = self.block(last).0.end;
        let block_text = self.buffer.text()[block_start..block_end].to_string();
        let moved = |selection: &Selection| {
            (selection.anchor >= block_start && selection.head <= block_end).then(|| Selection {
                anchor: selection.anchor - block_start + swap_at,
                head: selection.head - block_start + swap_at,
                goal: None,
            })
        };
        let kept: Vec<Selection> = self
            .selections
            .iter()
            .map(|selection| moved(selection).unwrap_or(*selection))
            .collect();
        self.swap_blocks(
            (previous_range, previous_text),
            (block_start..block_end, block_text),
            cx,
        );
        self.set_selections(kept, cx);
    }

    /// Moves the touched lines down one, swapping them with the line below.
    pub(crate) fn move_line_down(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let lines = self.touched();
        let (Some(&first), Some(&last)) = (lines.first(), lines.last()) else {
            return;
        };
        let line_count = self.buffer.lines();
        if last + 1 >= line_count {
            return;
        }
        let block_start = self.buffer.line_range(first).start;
        let block_end = self.block(last).0.end;
        let block_text = self.buffer.text()[block_start..block_end].to_string();
        let (next_range, next_text) = self.block(last + 1);
        let swap_at = next_range.start;
        let moved = |selection: &Selection| {
            (selection.anchor >= block_start && selection.head <= block_end).then(|| Selection {
                anchor: selection.anchor - block_start + swap_at,
                head: selection.head - block_start + swap_at,
                goal: None,
            })
        };
        let kept: Vec<Selection> = self
            .selections
            .iter()
            .map(|selection| moved(selection).unwrap_or(*selection))
            .collect();
        self.swap_blocks(
            (block_start..block_end, block_text),
            (next_range, next_text),
            cx,
        );
        self.set_selections(kept, cx);
    }

    pub(crate) fn undo(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        if let Some(previous) = self.history.undo(self.snapshot()) {
            self.restore(previous, cx);
        }
    }

    pub(crate) fn redo(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        if let Some(next) = self.history.redo(self.snapshot()) {
            self.restore(next, cx);
        }
    }

    /// Each selection's text, a line apiece; the whole line for a bare caret.
    fn copied(&self) -> Vec<(Range<usize>, String)> {
        self.in_order()
            .iter()
            .map(|selection| {
                let range = if selection.is_empty() {
                    let line = self.buffer.line_of(selection.head);
                    let range = self.buffer.line_range(line);
                    range.start..self.buffer.next(range.end)
                } else {
                    selection.range()
                };
                (range.clone(), self.buffer.text()[range].to_string())
            })
            .collect()
    }

    pub(crate) fn copy(&mut self, cx: &mut Context<Self>) {
        let text: Vec<String> = self.copied().into_iter().map(|(_, text)| text).collect();
        cx.write_to_clipboard(ClipboardItem::new_string(text.join("\n")));
    }

    pub(crate) fn cut(&mut self, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let copied = self.copied();
        let text: Vec<String> = copied.iter().map(|(_, text)| text.clone()).collect();
        cx.write_to_clipboard(ClipboardItem::new_string(text.join("\n")));
        self.apply(
            copied
                .into_iter()
                .map(|(range, _)| (range, String::new()))
                .collect(),
            false,
            cx,
        );
    }

    /// Pastes one line at each cursor when the counts match, else the whole text at every one.
    pub(crate) fn paste(&mut self, text: &str, cx: &mut Context<Self>) {
        if !self.editable() {
            return;
        }
        let lines: Vec<&str> = text.split('\n').collect();
        let edits = if lines.len() == self.selections.len() && lines.len() > 1 {
            self.in_order()
                .iter()
                .zip(lines)
                .map(|(selection, line)| (selection.range(), line.to_string()))
                .collect()
        } else {
            self.selections
                .iter()
                .map(|selection| (selection.range(), text.to_string()))
                .collect()
        };
        self.apply(edits, false, cx);
    }
}

impl CodeEditor {
    /// The selections from the top of the text down; the primary one is otherwise last.
    fn in_order(&self) -> Vec<Selection> {
        let mut all = self.selections.clone();
        all.sort_by_key(|selection| selection.range().start);
        all
    }
}

/// Edits sorted, with overlapping ranges joined; the first text of a joined run is kept.
fn joined(mut edits: Vec<(Range<usize>, String)>) -> Vec<(Range<usize>, String)> {
    edits.sort_by_key(|(range, _)| range.start);
    let mut out: Vec<(Range<usize>, String)> = Vec::with_capacity(edits.len());
    for (range, text) in edits {
        match out.last_mut() {
            Some((last, _)) if range.start < last.end => last.end = last.end.max(range.end),
            _ => out.push((range, text)),
        }
    }
    out
}

/// Where a caret at `at` lands after `edits`, which are sorted from the bottom of the text up.
///
/// A caret inside an edit moves to the end of that edit's replacement; an edit above it only shifts it by
/// its own delta; an edit below it leaves it where it was. A caret in the text is always a whole character,
/// so the answer is too.
fn caret_after(at: usize, edits: &[(Range<usize>, String)]) -> usize {
    let mut at = at;
    let mut shift: isize = 0;
    for (range, replacement) in edits.iter().rev() {
        if at < range.start {
            break;
        }
        if at <= range.end {
            at = range.start + replacement.len();
            break;
        }
        shift += replacement.len() as isize - range.len() as isize;
    }
    (at as isize + shift).max(0) as usize
}
