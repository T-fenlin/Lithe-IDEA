use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, EntityInputHandler, FontStyle, HighlightStyle, Pixels,
    Point, UTF16Selection, Window,
};

use super::{
    cursor::Selection,
    layout::Shown,
    state::CodeEditor,
    wrap::{Wrap, byte_at},
};
use crate::{
    forms::{from_utf16, to_utf16},
    theme::ActiveTheme,
};

impl CodeEditor {
    /// Notes set into a line: inlay hints, and the ghost text's first line.
    pub(crate) fn notes(&self, line: usize, cx: &App) -> Vec<(usize, String, HighlightStyle)> {
        let colors = &cx.theme().colors;
        let range = self.row_range(line);
        let hint = HighlightStyle {
            color: Some(colors.fg_subtle),
            background_color: Some(colors.hover),
            ..HighlightStyle::default()
        };
        let mut notes: Vec<(usize, String, HighlightStyle)> = self
            .marks
            .hints
            .iter()
            .filter(|hint| range.contains(&hint.offset) || hint.offset == range.end)
            .map(|note| (note.offset - range.start, note.text.to_string(), hint))
            .collect();
        if let Some(ghost) = &self.marks.ghost
            && self.buffer.line_of(ghost.offset) == line
        {
            let first = ghost.text.split('\n').next().unwrap_or_default();
            let style = HighlightStyle {
                color: Some(colors.fg_subtle),
                font_style: Some(FontStyle::Italic),
                ..HighlightStyle::default()
            };
            notes.push((ghost.offset - range.start, first.to_string(), style));
        }
        notes
    }

    /// A line's bytes as its row shows them: a multi-line ghost cuts the rest to follow its last line.
    pub(crate) fn row_range(&self, line: usize) -> Range<usize> {
        let range = self.buffer.line_range(line);
        match &self.marks.ghost {
            Some(ghost) if ghost.text.contains('\n') && range.contains(&ghost.offset) => {
                range.start..ghost.offset
            }
            _ => range,
        }
    }

    pub(crate) fn row_text(&self, line: usize) -> &str {
        &self.buffer.text()[self.row_range(line)]
    }

    /// Ends a composition: all of it, from its first mark, is one undo step.
    fn end_composition(&mut self) {
        if let Some(start) = self.composing.take()
            && start.text != self.buffer.text()
        {
            self.history.record(start, false);
        }
    }

    /// The buffer offset nearest window x inside one visual sub-row, and the drawn column it stands at.
    ///
    /// The sub-row draws its own slice of the line, so x is read against the text that slice holds: a click
    /// below the first sub-row of a wrapped line lands on the characters that sub-row draws rather than on the
    /// line's first columns. Both answers are in the whole line's coordinates — the drawn column is what
    /// [`CodeEditor::placed`] puts a caret at.
    ///
    /// Notes set into the line belong to the sub-row the line starts drawing them in: a sub-row's own drawn
    /// slice of them is what x is read against, so a note the row drew in another sub-row would put this
    /// answer inside its text.
    pub(crate) fn line_offset(
        &self,
        line: usize,
        wrap: &Wrap,
        x: Pixels,
        cx: &App,
    ) -> (usize, usize) {
        let range = self.row_range(line);
        let text = &self.buffer.text()[range.clone()];
        let shown = Shown::new(text, self.notes(line, cx), Vec::new());
        // x is read against the whole drawn line: this sub-row starts drawing at its own column, so the column
        // a click lands on there is that column plus what it measured inside the sub-row.
        let clicked = wrap.column
            + ((x - self.metrics.left) / self.metrics.advance)
                .round()
                .max(0.0) as usize;
        let column = shown.back(text, clicked.min(shown.text.chars().count()));
        // The sub-row draws its own slice of the line, so the byte is read inside that slice: the column the
        // click landed on is a column of the **line**, and the sub-row's own start takes it back to one of the
        // slice. The line's own columns and the slice's characters run one for one because a note set into the
        // line belongs to the sub-row that starts drawing it, never to a later one.
        let local = byte_at(
            &text[wrap.range.clone()],
            column.saturating_sub(wrap.column),
        );
        (range.start + wrap.range.start + local, column)
    }

    fn utf16_range(&self, range: &Range<usize>) -> Range<usize> {
        let text = self.buffer.text();
        to_utf16(text, range.start)..to_utf16(text, range.end)
    }

    fn byte_range(&self, range: &Range<usize>) -> Range<usize> {
        let text = self.buffer.text();
        from_utf16(text, range.start)..from_utf16(text, range.end)
    }
}

impl EntityInputHandler for CodeEditor {
    fn paste(&mut self, item: ClipboardItem, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = item.text() {
            self.paste(&text, cx);
        }
    }

    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.byte_range(&range_utf16);
        actual_range.replace(self.utf16_range(&range));
        Some(self.buffer.text()[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let primary = self.primary();
        Some(UTF16Selection {
            range: self.utf16_range(&primary.range()),
            reversed: primary.head < primary.anchor,
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|range| self.utf16_range(range))
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.marked = None;
        self.end_composition();
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .map(|range| self.byte_range(&range))
            .or(self.marked.clone());
        match range {
            Some(range) => {
                self.marked = None;
                self.apply(vec![(range, text.to_string())], true, cx);
            }
            None => self.type_text(text, cx),
        }
        self.end_composition();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        selected_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.editable() {
            return;
        }
        let range = range_utf16
            .map(|range| self.byte_range(&range))
            .or(self.marked.clone())
            .unwrap_or(self.primary().range());
        if self.composing.is_none() {
            self.composing = Some(self.snapshot());
        }
        self.apply(vec![(range.clone(), text.to_string())], true, cx);
        self.marked = (!text.is_empty()).then(|| range.start..range.start + text.len());
        if self.marked.is_none() {
            self.end_composition();
        }
        let caret = match selected_utf16 {
            Some(inner) => range.start + from_utf16(text, inner.end),
            None => range.start + text.len(),
        };
        self.set_selections(vec![Selection::caret(caret)], cx);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.byte_range(&range_utf16);
        self.placed(range.start, cx).map(|(_, bounds)| bounds)
    }

    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        let offset = self.offset_at(position, cx)?;
        Some(to_utf16(self.buffer.text(), offset))
    }
}
