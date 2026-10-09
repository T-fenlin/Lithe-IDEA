use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

/// One visual sub-row of a line: where it stands in the line's bytes, and in its drawn columns.
///
/// A line that fits draws in one sub-row that spans it whole, so every line has at least one. A line too
/// wide for the code area is cut into several, which the row table then holds as several rows — the list
/// that draws rows is a uniform-height one, so a sub-row's own row is what lets it be as tall as any other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Wrap {
    /// The line's bytes this sub-row draws, a whole number of characters long.
    pub range: Range<usize>,
    /// Where those bytes start among the line's characters — where the wrapped line begins drawing them.
    pub column: usize,
}

/// Where `column` characters into `text` stand, in bytes.
///
/// A column past the end gives the end, the same reading as `Buffer::offset`.
pub(crate) fn byte_at(text: &str, column: usize) -> usize {
    text.char_indices()
        .nth(column)
        .map_or(text.len(), |(at, _)| at)
}

/// How many characters of `text` a sub-row holds, given where its next one starts.
fn drawn(text: &str, to: usize) -> usize {
    text[..to].chars().count()
}

/// The visual sub-rows of one row's text, none longer than `columns` characters.
///
/// Soft wrap: a row too wide for the code area is cut into several sub-rows, each drawn on a row of its own.
/// A break is taken at the last **word** boundary that fits, so words stay whole where they can and no
/// sub-row opens on a space; a run of non-space characters too long to fit is cut at a character, because a
/// minified line or a long literal has no boundary to break at. `columns` of zero — nothing measured yet, or
/// a code area narrower than the gutter — gives one sub-row holding the whole text, which is also what a row
/// with no soft wrap gives: every row of the table is then one whole line, and the table reads exactly as it
/// did without.
pub(crate) fn segments(text: &str, columns: usize) -> Vec<Wrap> {
    if columns == 0 || text.is_empty() {
        return vec![Wrap {
            range: 0..text.len(),
            column: 0,
        }];
    }
    let whole = drawn(text, text.len());
    if whole <= columns {
        return vec![Wrap {
            range: 0..text.len(),
            column: 0,
        }];
    }
    let mut out = Vec::with_capacity(whole.div_ceil(columns));
    // Where the sub-row being cut begins, and how far into the line's characters that is.
    let (mut start, mut column) = (0, 0);
    while start < text.len() {
        let mut taken = 0;
        // The byte just past the last space the limit reaches, which is where a word boundary break goes:
        // the space ends its sub-row, so the next one opens on the word rather than on the space.
        let mut word = None;
        // The byte just past the last grapheme that fits, which is where a break inside a run goes.
        let mut reach = None;
        for (at, _) in text[start..].grapheme_indices(true) {
            if taken == columns {
                break;
            }
            taken += 1;
            let at = start + at;
            let next = at + text[at..].chars().next().map_or(0, char::len_utf8);
            if text[at..].starts_with(' ') {
                word = Some(next);
            }
            reach = Some(next);
        }
        // In a run with no boundary to break at, and in the last sub-row the limit is never reached in.
        let end = word.or(reach).unwrap_or(text.len()).min(text.len());
        // `columns` counts graphemes and covers the first of them, so a step always moves on; the guard keeps
        // a caller that reached here some other way from spinning on an empty sub-row.
        let end = end.max(byte_at(&text[start..], 1) + start);
        out.push(Wrap {
            range: start..end,
            column,
        });
        start = end;
        column = drawn(text, end);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bytes each sub-row draws, which is what the row table and every offset lookup read.
    fn cut(text: &str, columns: usize) -> Vec<Range<usize>> {
        segments(text, columns)
            .into_iter()
            .map(|wrap| wrap.range)
            .collect()
    }

    /// Nothing measured wraps nothing: the row table has to read exactly as it did before soft wrap existed.
    #[test]
    fn no_width_and_no_soft_wrap_leave_a_row_whole() {
        let text = "a row far too wide to fit in any code area";
        for columns in [0, text.chars().count()] {
            assert_eq!(
                segments(text, columns),
                vec![Wrap {
                    range: 0..text.len(),
                    column: 0
                }],
                "a row within the limit, or a limit of none, is one sub-row"
            );
        }
        assert_eq!(
            segments("", 4),
            vec![Wrap {
                range: 0..0,
                column: 0
            }],
            "an empty row still stands on a row"
        );
    }

    /// Words stay whole: a break is taken at the space that ends the last whole word that fits, and the
    /// spaces that would open the next sub-row go with it.
    #[test]
    fn a_break_is_taken_at_the_last_word_boundary_that_fits() {
        assert_eq!(cut("alpha beta gamma delta", 11), [0..11, 11..17, 17..22]);
        assert_eq!(cut("alpha beta gamma", 6), [0..6, 6..11, 11..16]);
        assert_eq!(
            cut("a  b", 2),
            [0..2, 2..3, 3..4],
            "a space the limit lands on ends its sub-row, and no sub-row opens on one"
        );
    }

    /// A run too long to fit has no boundary to break at, so it is cut at a character — and never splits one.
    #[test]
    fn a_run_with_no_boundary_is_cut_at_characters() {
        assert_eq!(cut("abcdefghij", 4), [0..4, 4..8, 8..10]);
        assert_eq!(
            cut("ααααα", 2),
            [0..4, 4..8, 8..10],
            "a two-byte character is never cut"
        );
        assert_eq!(
            cut("a\u{301}\u{301}bcd", 2),
            [0..6, 6..8],
            "a combining mark stays with the letter under it"
        );
    }

    /// Every sub-row carries the column it starts at, which is the coordinate a caret and a click work in.
    #[test]
    fn each_sub_row_knows_the_column_it_starts_at() {
        let wraps = segments("alpha beta gamma", 6);
        assert_eq!(
            wraps.iter().map(|wrap| wrap.column).collect::<Vec<_>>(),
            [0, 6, 11],
            "each one starts where the text before it ended"
        );
        assert_eq!(
            wraps
                .iter()
                .map(|wrap| wrap.range.end - wrap.range.start)
                .collect::<Vec<_>>(),
            [6, 5, 5],
            "and the space a break falls on goes with the sub-row it ends"
        );
    }

    /// Characters, not bytes: a limit counted in bytes would cut a line of wide text short.
    #[test]
    fn the_limit_counts_characters_and_always_advances() {
        assert_eq!(cut("ππππ", 1), [0..2, 2..4, 4..6, 6..8]);
        assert_eq!(cut("ab", 1), [0..1, 1..2]);
        assert_eq!(
            cut("x", 1),
            [0..1],
            "a single character that fits is not cut into nothing"
        );
    }
}
