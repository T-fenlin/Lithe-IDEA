use std::cell::RefCell;
use std::fmt;
use std::ops::Range;

use crate::language::{Highlighter, Language, Slot};

/// `range` pulled inside `text` and onto whole characters.
///
/// Every byte-indexed consumer of a buffer goes through this, so a range a caller worked out
/// slightly wrong cannot panic on a split character or answer with a neighbouring line's bytes.
pub(crate) fn clamped_range(text: &str, range: Range<usize>) -> Range<usize> {
    let whole = |offset: usize| {
        let mut at = offset.min(text.len());
        while at > 0 && !text.is_char_boundary(at) {
            at -= 1;
        }
        at
    };
    let start = whole(range.start);
    start..whole(range.end).max(start)
}

/// Source text and where each line starts, in bytes.
pub struct Buffer {
    text: String,
    starts: Vec<usize>,
    /// The registered language that highlights this buffer, when a plugin declared one.
    language: Option<&'static Language>,
    /// Bumped by every text change and every language change. The highlighter remembers the
    /// version it parsed, so a stale span table is never drawn over newer text.
    version: u64,
    /// The tree-sitter engine and its span table, built on first use (see [`Highlight`]).
    highlight: RefCell<Highlight>,
}

/// A buffer's tree-sitter engine, built on first use and kept in step with the text.
///
/// Views hold only `&Buffer`, and building a [`Highlighter`] compiles the language's query, so the
/// engine sits behind a [`RefCell`] and is built once instead of once per painted row.
#[derive(Default)]
struct Highlight {
    /// `None` until the first request; the inner `None` records that the language's grammar or
    /// query would not install, so a broken language plugin costs one attempt, not one per frame.
    engine: Option<Option<Highlighter>>,
    /// The text version the engine holds spans for; `None` means it has not parsed yet.
    parsed: Option<u64>,
}

impl Highlight {
    /// The spans of `text` inside `range`, re-parsing first when the text has moved on.
    fn spans(
        &mut self,
        language: &'static Language,
        text: &str,
        version: u64,
        range: Range<usize>,
    ) -> Vec<(Range<usize>, Slot)> {
        let Self { engine, parsed } = self;
        if engine.is_none() {
            *engine = Some(Highlighter::new(language));
        }
        let Some(highlighter) = engine.as_mut().and_then(|built| built.as_mut()) else {
            return Vec::new();
        };
        if *parsed != Some(version) {
            highlighter.reparse(text);
            *parsed = Some(version);
        }
        highlighter.spans_in(range)
    }

    /// Whether the language's engine installed; builds it on the first call.
    fn ready(&mut self, language: &'static Language) -> bool {
        if self.engine.is_none() {
            self.engine = Some(Highlighter::new(language));
        }
        self.engine.as_ref().is_some_and(Option::is_some)
    }
}

impl Clone for Buffer {
    /// The text, its line table and its language; the highlight cache rebuilds on demand.
    fn clone(&self) -> Self {
        Self {
            text: self.text.clone(),
            starts: self.starts.clone(),
            language: self.language,
            version: self.version,
            highlight: RefCell::new(Highlight::default()),
        }
    }
}

impl fmt::Debug for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Buffer")
            .field("text", &self.text)
            .field("starts", &self.starts)
            .field("language", &self.language.map(|language| language.id))
            .finish_non_exhaustive()
    }
}

/// Two buffers are equal when they hold the same text in the same language; the cached
/// tree-sitter spans are derived data and take no part.
impl PartialEq for Buffer {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.language.map(|language| language.id)
                == other.language.map(|language| language.id)
    }
}

impl Eq for Buffer {}

fn starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(at, _)| at + 1))
        .collect()
}

impl Buffer {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let starts = starts(&text);
        Self {
            text,
            starts,
            language: None,
            version: 0,
            highlight: RefCell::new(Highlight::default()),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// The language highlighting this buffer, when a language plugin declared one.
    pub fn language(&self) -> Option<&'static Language> {
        self.language
    }

    /// Sets the language that highlights this buffer, or `None` for the built-in lexer.
    ///
    /// The cached spans were produced by the previous language's grammar, so a change drops them
    /// and the next request builds the new engine.
    pub fn set_language(&mut self, language: Option<&'static Language>) {
        if self.language.map(|each| each.id) == language.map(|each| each.id) {
            return;
        }
        self.language = language;
        self.version += 1;
        self.highlight = RefCell::new(Highlight::default());
    }

    /// Whether a language is set **and** its highlighter actually installed.
    ///
    /// This is the fork the editor's colours take: `true` means spans come from tree-sitter,
    /// `false` means the built-in lexer draws them. So a language whose grammar or query will not
    /// install degrades visibly to the lexer rather than painting nothing.
    pub fn has_highlighter(&self) -> bool {
        let Some(language) = self.language else {
            return false;
        };
        // A borrow that is already taken reads as "no highlighter", which sends the caller to the
        // lexer instead of panicking inside a render.
        self.highlight
            .try_borrow_mut()
            .is_ok_and(|mut highlight| highlight.ready(language))
    }

    /// The tree-sitter spans inside `range`, in buffer offsets and clipped to it.
    ///
    /// Empty when no language is set, when its grammar or query will not install, or when nothing
    /// in `range` is captured. It never panics: an out-of-bounds or character-splitting range is
    /// pulled back onto whole characters inside the text first, so a caller's arithmetic mistake
    /// cannot take a neighbouring line's colours or crash a frame.
    pub fn highlight_spans(&self, range: Range<usize>) -> Vec<(Range<usize>, Slot)> {
        let range = clamped_range(&self.text, range);
        if range.is_empty() {
            return Vec::new();
        }
        let Some(language) = self.language else {
            return Vec::new();
        };
        let Ok(mut highlight) = self.highlight.try_borrow_mut() else {
            return Vec::new();
        };
        highlight.spans(language, &self.text, self.version, range)
    }

    /// How many lines it holds; an empty buffer holds one.
    pub fn lines(&self) -> usize {
        self.starts.len()
    }

    /// A line's bytes, without its line break.
    pub fn line_range(&self, line: usize) -> Range<usize> {
        let start = self.starts[line];
        let end = self
            .starts
            .get(line + 1)
            .map_or(self.text.len(), |next| next - 1);
        start..end
    }

    pub fn line(&self, line: usize) -> &str {
        &self.text[self.line_range(line)]
    }

    /// The line holding `offset`.
    pub fn line_of(&self, offset: usize) -> usize {
        self.starts.partition_point(|start| *start <= offset) - 1
    }

    /// An offset's line, and its column in characters.
    pub fn point(&self, offset: usize) -> (usize, usize) {
        let line = self.line_of(offset);
        let column = self.text[self.starts[line]..offset].chars().count();
        (line, column)
    }

    /// The offset at a line and a column in characters, held to the line's end.
    pub fn offset(&self, line: usize, column: usize) -> usize {
        let range = self.line_range(line.min(self.lines() - 1));
        self.text[range.clone()]
            .char_indices()
            .nth(column)
            .map_or(range.end, |(at, _)| range.start + at)
    }

    /// The character after `offset`, or the end.
    pub fn next(&self, offset: usize) -> usize {
        self.text[offset..]
            .chars()
            .next()
            .map_or(offset, |ch| offset + ch.len_utf8())
    }

    /// The character before `offset`, or the start.
    pub fn previous(&self, offset: usize) -> usize {
        self.text[..offset]
            .chars()
            .next_back()
            .map_or(offset, |ch| offset - ch.len_utf8())
    }

    /// How far a line is indented, in characters.
    pub fn indent(&self, line: usize) -> usize {
        self.line(line)
            .chars()
            .take_while(|ch| *ch == ' ' || *ch == '\t')
            .count()
    }

    pub(crate) fn replace(&mut self, range: Range<usize>, text: &str) {
        assert!(
            range.start <= range.end
                && range.end <= self.text.len()
                && self.text.is_char_boundary(range.start)
                && self.text.is_char_boundary(range.end),
            "an edit covers whole characters inside the text: {range:?} of {}",
            self.text.len()
        );
        self.text.replace_range(range, text);
        self.starts = starts(&self.text);
        // The span table now describes text that is gone; the next request re-parses.
        self.version += 1;
    }
}

/// What kind of character a word motion sees.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Space,
    Word,
    Mark,
}

fn class(ch: char) -> Class {
    if ch.is_alphanumeric() || ch == '_' {
        Class::Word
    } else if ch.is_whitespace() {
        Class::Space
    } else {
        Class::Mark
    }
}

impl Buffer {
    /// The start of the word before `offset`, over any spaces first; a line start stops it.
    pub fn word_start(&self, offset: usize) -> usize {
        let line_start = self.starts[self.line_of(offset)];
        if offset == line_start {
            return self.previous(offset);
        }
        let mut chars = self.text[line_start..offset]
            .char_indices()
            .rev()
            .peekable();
        while chars
            .next_if(|(_, ch)| class(*ch) == Class::Space)
            .is_some()
        {}
        let Some(&(_, first)) = chars.peek() else {
            return line_start;
        };
        let kind = class(first);
        let mut at = offset;
        for (ix, ch) in chars {
            if class(ch) != kind {
                break;
            }
            at = line_start + ix;
        }
        at.min(offset)
    }

    /// The end of the word after `offset`, over any spaces first; a line end stops it.
    pub fn word_end(&self, offset: usize) -> usize {
        let line_end = self.line_range(self.line_of(offset)).end;
        if offset == line_end {
            return self.next(offset);
        }
        let mut chars = self.text[offset..line_end].char_indices().peekable();
        while chars
            .next_if(|(_, ch)| class(*ch) == Class::Space)
            .is_some()
        {}
        let Some(&(_, first)) = chars.peek() else {
            return line_end;
        };
        let kind = class(first);
        let mut at = line_end;
        for (ix, ch) in chars {
            if class(ch) != kind {
                at = offset + ix;
                break;
            }
        }
        at
    }

    /// The word around `offset`, or the run of marks or spaces it sits in.
    pub fn word_at(&self, offset: usize) -> Range<usize> {
        let range = self.line_range(self.line_of(offset));
        let line = &self.text[range.clone()];
        let at = offset - range.start;
        let kind = line[at..]
            .chars()
            .next()
            .or_else(|| line[..at].chars().next_back())
            .map(class);
        let Some(kind) = kind else {
            return offset..offset;
        };
        let start = line[..at]
            .char_indices()
            .rev()
            .take_while(|(_, ch)| class(*ch) == kind)
            .last()
            .map_or(at, |(ix, _)| ix);
        let end = line[at..]
            .char_indices()
            .find(|(_, ch)| class(*ch) != kind)
            .map_or(line.len(), |(ix, _)| at + ix);
        range.start + start..range.start + end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_split_at_breaks_and_points_count_characters() {
        let buffer = Buffer::new("fn main() {\n    let π = 3.14;\n}");
        assert_eq!(buffer.lines(), 3);
        assert_eq!(buffer.line(1), "    let π = 3.14;");
        let pi = buffer.text().find('π').expect("pi");
        assert_eq!(buffer.point(pi), (1, 8));
        assert_eq!(
            buffer.point(pi + 'π'.len_utf8()),
            (1, 9),
            "a column is a character"
        );
        assert_eq!(buffer.offset(1, 9), pi + 'π'.len_utf8());
        assert_eq!(buffer.offset(0, 99), 11, "held to the line's end");
        assert_eq!(Buffer::new("").lines(), 1);
        assert_eq!(buffer.indent(1), 4);
    }

    #[test]
    fn edits_keep_line_starts_in_step() {
        let mut buffer = Buffer::new("a\nb");
        buffer.replace(1..1, "\nc");
        assert_eq!(buffer.text(), "a\nc\nb");
        assert_eq!(buffer.lines(), 3);
        assert_eq!(buffer.line(2), "b");
    }

    #[test]
    fn words_stop_at_their_class_and_at_line_edges() {
        let buffer = Buffer::new("let total = price * 2;\nnext");
        assert_eq!(buffer.word_start(9), 4, "back to the start of total");
        assert_eq!(buffer.word_start(4), 0, "over the space to let");
        assert_eq!(buffer.word_end(4), 9);
        assert_eq!(buffer.word_end(9), 11, "over the space, then the mark");
        assert_eq!(buffer.word_end(22), 23, "a line end steps to the next line");
        assert_eq!(buffer.word_at(6), 4..9);
        assert_eq!(buffer.word_at(10), 10..11, "a mark is its own run");
    }

    #[test]
    fn a_range_is_pulled_inside_the_text_and_onto_whole_characters() {
        // π is two bytes at 1..3, so a range that splits it has to step back rather than panic.
        let text = "aπb";
        assert_eq!(clamped_range(text, 0..4), 0..4);
        assert_eq!(clamped_range(text, 0..99), 0..4, "past the end");
        assert_eq!(clamped_range(text, 3..1), 3..3, "backwards");
        assert_eq!(clamped_range(text, 2..2), 1..1, "inside π");
        assert_eq!(clamped_range(text, 2..3), 1..3, "its start is inside π");
        assert_eq!(clamped_range("", 5..9), 0..0);
    }

    /// No language means no tree-sitter spans, and no range a caller works out can panic.
    #[test]
    fn spans_stay_empty_and_safe_without_a_language() {
        let mut buffer = Buffer::new("let π = 1;");
        assert!(buffer.language().is_none());
        assert!(!buffer.has_highlighter());
        let long = 0..usize::MAX;
        for range in [long, 5..2, 100..200, 0..0, 0..buffer.len()] {
            assert!(buffer.highlight_spans(range).is_empty());
        }
        let pi = buffer.text().find('π').expect("π");
        assert!(
            buffer.highlight_spans(pi + 1..pi + 2).is_empty(),
            "a range that splits π has to be survivable, not sliced"
        );
        buffer.set_language(None);
        assert!(buffer.language().is_none());
        assert!(!buffer.has_highlighter());
    }

    /// The highlight cache compares this version, so every text or language change has to age it.
    #[test]
    fn an_edit_and_a_new_language_age_the_highlight_cache() {
        let mut buffer = Buffer::new("a");
        let fresh = buffer.version;
        buffer.replace(1..1, "b");
        assert_eq!(buffer.text(), "ab");
        assert!(
            buffer.version > fresh,
            "an edit leaves the span table stale"
        );
        let edited = buffer.version;
        buffer.set_language(None);
        assert_eq!(
            buffer.version, edited,
            "re-setting the language it already has changes nothing"
        );
    }
}
