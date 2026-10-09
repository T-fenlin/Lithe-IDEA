use gpui::{Context, ScrollStrategy, Window};

use super::{
    buffer::clamped_range,
    cursor::{Motion, Selection, merged, moved},
    layout::{Row, hidden, rows},
    state::CodeEditor,
    syntax::folds,
};

impl CodeEditor {
    /// Moves every cursor. Up and down step over folded lines.
    pub(crate) fn motion(&mut self, motion: Motion, extend: bool, cx: &mut Context<Self>) {
        let hide = hidden(self.buffer.lines(), &folds(&self.buffer), &self.folded);
        let vertical = matches!(motion, Motion::Up | Motion::Down | Motion::Page(_));
        let moved: Vec<Selection> = self
            .selections
            .iter()
            .map(|selection| {
                let mut next = moved(&self.buffer, *selection, motion, extend);
                while vertical && hide[self.buffer.line_of(next.head)] {
                    let again = moved(&self.buffer, next, motion, extend);
                    if again.head == next.head {
                        break;
                    }
                    next = again;
                }
                next
            })
            .collect();
        let up = matches!(
            motion,
            Motion::Up | Motion::Start | Motion::LineStart | Motion::WordLeft | Motion::Left
        ) || matches!(motion, Motion::Page(lines) if lines < 0);
        self.set_selections(moved, cx);
        self.reveal(!up, cx);
    }

    pub(crate) fn select_all(&mut self, cx: &mut Context<Self>) {
        let all = 0..self.buffer.len();
        self.select([all], cx);
    }

    /// A new cursor a line above or below the primary one, at its column.
    pub(crate) fn add_cursor(&mut self, below: bool, cx: &mut Context<Self>) {
        let primary = self.primary();
        let motion = if below { Motion::Down } else { Motion::Up };
        let next = moved(&self.buffer, Selection::caret(primary.head), motion, false);
        if next.head == primary.head {
            return;
        }
        let mut all = self.selections.clone();
        all.push(Selection { goal: None, ..next });
        self.set_selections(all, cx);
        self.reveal(below, cx);
    }

    /// Selects the word at the primary cursor, then each next place its text appears.
    pub(crate) fn select_next(&mut self, cx: &mut Context<Self>) {
        let primary = self.primary();
        if primary.is_empty() {
            let word = self.buffer.word_at(primary.head);
            let mut all = self.selections.clone();
            all.pop();
            all.push(Selection {
                anchor: word.start,
                head: word.end,
                goal: None,
            });
            return self.set_selections(all, cx);
        }
        let needle = self.buffer.text()[primary.range()].to_string();
        let text = self.buffer.text();
        let found = text[primary.range().end..]
            .find(&needle)
            .map(|at| primary.range().end + at)
            .or_else(|| text.find(&needle));
        let Some(start) = found else {
            return;
        };
        let range = start..start + needle.len();
        if self
            .selections
            .iter()
            .any(|selection| selection.range() == range)
        {
            return;
        }
        let mut all = self.selections.clone();
        all.push(Selection {
            anchor: range.start,
            head: range.end,
            goal: None,
        });
        self.set_selections(all, cx);
        self.reveal(true, cx);
    }

    /// Escape: one cursor again, where the primary one was, and no ghost text.
    pub(crate) fn single_cursor(&mut self, cx: &mut Context<Self>) {
        let primary = self.primary();
        self.marks.ghost = None;
        self.set_selections(vec![Selection::caret(primary.head)], cx);
    }

    /// Puts the primary cursor at `offset` (a byte offset) and collapses the selection to a caret.
    ///
    /// An offset out of bounds or inside a multi-byte character is pulled back onto a whole character first
    /// (`buffer::clamped_range`), never panicking; a fold over the cursor opens (`set_selections` does that),
    /// the viewport scrolls only **as far as it must** to bring the cursor in (`ScrollStrategy::Nearest`, the
    /// same reading as the upstream `set_cursor_position` with `ScrollPadding::Minimal`, rather than pushing the
    /// cursor to a viewport edge the way key-repeat motion does), and the keyboard focus goes to the editor.
    ///
    /// An owner places its cursor with this after a language-server answer (go to definition, a code action,
    /// restoring a remembered position); the editor asks for nothing itself.
    pub fn set_cursor_offset(
        &mut self,
        offset: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let at = clamped_range(self.buffer.text(), offset..offset).start;
        self.set_selections(vec![Selection::caret(at)], cx);
        self.scroll
            .scroll_to_item(self.primary_row(), ScrollStrategy::Nearest);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Folds the block that opens on a line, or opens it again; cursors inside move to its header.
    pub(crate) fn toggle_fold(&mut self, line: usize, cx: &mut Context<Self>) {
        if !self.folded.remove(&line) {
            let Some((_, end)) = folds(&self.buffer)
                .into_iter()
                .find(|(header, _)| *header == line)
            else {
                return;
            };
            let header_end = self.buffer.line_range(line).end;
            let kept: Vec<Selection> = self
                .selections
                .iter()
                .map(|selection| {
                    if (line + 1..=end).contains(&self.buffer.line_of(selection.head)) {
                        Selection::caret(header_end)
                    } else {
                        *selection
                    }
                })
                .collect();
            self.selections = merged(kept);
            self.folded.insert(line);
        }
        log::info!("code editor: folded {:?}", self.folded);
        cx.notify();
    }

    /// Opens every fold that hides a cursor.
    pub(crate) fn unfold_cursors(&mut self) {
        if self.folded.is_empty() {
            return;
        }
        let folds = folds(&self.buffer);
        let lines: Vec<usize> = self
            .selections
            .iter()
            .map(|selection| self.buffer.line_of(selection.head))
            .collect();
        self.folded.retain(|header| {
            let end = folds
                .iter()
                .find(|(start, _)| start == header)
                .map(|(_, end)| *end);
            end.is_some_and(|end| !lines.iter().any(|line| (header + 1..=end).contains(line)))
        });
    }

    /// The row the primary cursor draws on.
    pub(crate) fn primary_row(&self) -> usize {
        let line = self.buffer.line_of(self.primary().head);
        let hide = hidden(self.buffer.lines(), &folds(&self.buffer), &self.folded);
        rows(&self.buffer, &hide, &self.marks)
            .iter()
            .position(|row| *row == Row::Line(line))
            .expect("the primary cursor sits on a shown line")
    }

    /// Scrolls the primary cursor into view: from below when moving down, from above when moving up.
    pub(crate) fn reveal(&mut self, down: bool, cx: &mut Context<Self>) {
        let strategy = if down {
            ScrollStrategy::Bottom
        } else {
            ScrollStrategy::Top
        };
        self.scroll.scroll_to_item(self.primary_row(), strategy);
        cx.notify();
    }
}
