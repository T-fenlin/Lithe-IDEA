use gpui::{Context, ScrollStrategy, Window};

use super::{
    buffer::clamped_range,
    cursor::{Motion, Selection, merged, moved},
    layout::{Row, Shown, hidden, rows},
    state::CodeEditor,
    syntax::folds,
    wrap::byte_at,
};

impl CodeEditor {
    /// Moves every cursor. Up and down step over folded lines, and over the visual sub-rows a wrapped line
    /// draws: a wrapped line is walked a sub-row at a time before the next line is reached.
    pub(crate) fn motion(&mut self, motion: Motion, extend: bool, cx: &mut Context<Self>) {
        let hide = hidden(self.buffer.lines(), &folds(&self.buffer), &self.folded);
        let moved: Vec<Selection> = self
            .selections
            .iter()
            .map(
                |selection| match self.row_step(*selection, motion, extend, &hide) {
                    Some(next) => next,
                    None => {
                        let vertical =
                            matches!(motion, Motion::Up | Motion::Down | Motion::Page(_));
                        let mut next = moved(&self.buffer, *selection, motion, extend);
                        while vertical && hide[self.buffer.line_of(next.head)] {
                            let again = moved(&self.buffer, next, motion, extend);
                            if again.head == next.head {
                                break;
                            }
                            next = again;
                        }
                        next
                    }
                },
            )
            .collect();
        let up = matches!(
            motion,
            Motion::Up | Motion::Start | Motion::LineStart | Motion::WordLeft | Motion::Left
        ) || matches!(motion, Motion::Page(lines) if lines < 0);
        self.set_selections(moved, cx);
        self.reveal(!up, cx);
    }

    /// The cursor after stepping one visual sub-row up or down, `None` when this motion is not one the wrap
    /// table decides.
    ///
    /// Up and down keep the drawn column they left, the same reading as the column-goal a vertical move keeps;
    /// a step inside a wrapped line lands on the neighbour sub-row, and a step off either end of the line lands
    /// on the neighbouring line's nearest sub-row — so the cursor walks what is drawn, one row at a time.
    /// A fold hides lines rather than sub-rows, and `Page` counts lines, so both go to the unwrapped motion.
    fn row_step(
        &self,
        selection: Selection,
        motion: Motion,
        extend: bool,
        hide: &[bool],
    ) -> Option<Selection> {
        let down = match motion {
            Motion::Down => true,
            Motion::Up => false,
            _ => return None,
        };
        if !self.options.soft_wrap {
            return None;
        }
        let head = selection.head;
        let line = self.buffer.line_of(head);
        if hide[line] {
            return None;
        }
        let shown = self.line_shown(line);
        let text = &self.buffer.text()[self.row_range(line)];
        let head_column = shown.column(text, self.buffer.point(head).1.min(text.len()));
        // The column the step aims at is the one a run of steps started from, so walking a wrapped line down
        // and back up returns the caret to where it began rather than to the wrap column it passed. Where the
        // caret **is** decides which sub-row step; the aim decides only the column within it.
        let column = selection.goal.unwrap_or(head_column);
        let wraps = self.line_wraps(line, hide);
        let sub = self.sub_row(line, head_column).map_or(0, |(sub, _)| sub);
        let next = if down { sub + 1 } else { sub.wrapping_sub(1) };
        let (line, wraps, sub) = match wraps.get(next) {
            Some(_) => (line, wraps, next),
            None => {
                let step = if down { 1 } else { -1 };
                let target = line as isize + step;
                if target < 0 {
                    return Some(Selection {
                        anchor: if extend { selection.anchor } else { 0 },
                        head: 0,
                        goal: Some(column),
                    });
                }
                if target >= self.buffer.lines() as isize {
                    return Some(Selection {
                        anchor: if extend {
                            selection.anchor
                        } else {
                            self.buffer.len()
                        },
                        head: self.buffer.len(),
                        goal: Some(column),
                    });
                }
                let line = target as usize;
                if hide[line] {
                    return None;
                }
                let wraps = self.line_wraps(line, hide);
                let sub = if down { 0 } else { wraps.len() - 1 };
                (line, wraps, sub)
            }
        };
        let wrap = wraps[sub].clone();
        let range = self.row_range(line);
        let text = &self.buffer.text()[range.clone()];
        let head = range.start
            + wrap.range.start
            + byte_at(
                &text[wrap.range.clone()],
                column.saturating_sub(wrap.column),
            );
        Some(Selection {
            anchor: if extend { selection.anchor } else { head },
            head,
            goal: Some(column),
        })
    }

    /// The `Shown` view of a row, which is what turns a byte of the line into a drawn column.
    ///
    /// Notes are left out on purpose: a motion works in the text's own columns, and a sub-row boundary is a
    /// byte of the text, so the two readings have to be the same one.
    fn line_shown(&self, line: usize) -> Shown {
        let range = self.row_range(line);
        Shown::new(&self.buffer.text()[range], Vec::new(), Vec::new())
    }

    pub(crate) fn select_all(&mut self, cx: &mut Context<Self>) {
        let all = 0..self.buffer.len();
        self.select([all], cx);
    }

    /// A new cursor a line above or below the primary one, at its column.
    ///
    /// A wrapped line is walked by its visual sub-rows here too, so the cursor this adds is the one the arrow
    /// keys would have reached.
    pub(crate) fn add_cursor(&mut self, below: bool, cx: &mut Context<Self>) {
        let primary = self.primary();
        let motion = if below { Motion::Down } else { Motion::Up };
        let hide = hidden(self.buffer.lines(), &folds(&self.buffer), &self.folded);
        let next = self
            .row_step(Selection::caret(primary.head), motion, false, &hide)
            .unwrap_or_else(|| moved(&self.buffer, Selection::caret(primary.head), motion, false));
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
    ///
    /// On a wrapped line that is the row of the cursor's own visual sub-row, so revealing and centring the
    /// cursor scroll to where it is drawn rather than to the row the line starts at.
    pub(crate) fn primary_row(&self) -> usize {
        let head = self.primary().head;
        let line = self.buffer.line_of(head);
        let sub = self
            .sub_row(line, self.buffer.point(head).1)
            .map_or(0, |(sub, _)| sub);
        let hide = hidden(self.buffer.lines(), &folds(&self.buffer), &self.folded);
        rows(&self.buffer, &hide, &self.marks, |each| {
            self.line_wraps(each, self.frame_hidden()).len()
        })
        .iter()
        .position(|row| *row == Row::Line(line, sub))
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
