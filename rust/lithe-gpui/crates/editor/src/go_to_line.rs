#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GoToLineInput {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) has_explicit_column: bool,
}

impl GoToLineInput {
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        let parts: Vec<&str> = trimmed.split(':').collect();
        if parts.len() != 1 && parts.len() != 2 {
            return None;
        }
        let line = one_based(parts[0])?;
        let has_explicit_column = parts.len() == 2;
        let column = if has_explicit_column {
            one_based(parts[1])?
        } else {
            1
        };
        Some(Self {
            line: line - 1,
            column: column - 1,
            has_explicit_column,
        })
    }

    pub(crate) fn clamped(
        line: usize,
        column: usize,
        has_explicit_column: bool,
        content: &str,
    ) -> Self {
        let mut line_index = 0usize;
        let mut scan = 0usize;

        loop {
            let rest = &content[scan..];
            let mut units = 0usize;
            let mut end_byte = scan;
            let mut terminator: Option<usize> = None;

            for (offset, ch) in rest.char_indices() {
                match ch {
                    '\n' => {
                        end_byte = scan + offset;
                        terminator = Some(1);
                        break;
                    }
                    '\r' => {
                        end_byte = scan + offset;
                        let next_is_lf = rest[offset + 1..].starts_with('\n');
                        terminator = Some(if next_is_lf { 2 } else { 1 });
                        break;
                    }
                    _ => units += ch.len_utf16(),
                }
            }

            if line_index == line || terminator.is_none() {
                return Self {
                    line: line_index,
                    column: column.min(units),
                    has_explicit_column,
                };
            }

            line_index += 1;
            scan = end_byte + terminator.unwrap_or(0);
        }
    }

    pub(crate) fn prefill(line: usize, column: usize) -> String {
        format!("{}:{}", line + 1, column + 1)
    }
}

fn one_based(part: &str) -> Option<usize> {
    let value: i64 = part.trim().parse().ok()?;
    if value < 1 {
        return None;
    }
    usize::try_from(value).ok()
}

#[cfg(test)]
mod tests {
    use super::GoToLineInput as G;

    #[test]
    fn line_only_input_is_zero_based_with_column_zero() {
        assert_eq!(
            G::parse("120"),
            Some(G {
                line: 119,
                column: 0,
                has_explicit_column: false
            })
        );
        assert_eq!(
            G::parse("1"),
            Some(G {
                line: 0,
                column: 0,
                has_explicit_column: false
            })
        );
    }

    #[test]
    fn line_and_column_input_is_zero_based() {
        assert_eq!(
            G::parse("120:35"),
            Some(G {
                line: 119,
                column: 34,
                has_explicit_column: true
            })
        );
    }

    #[test]
    fn explicit_column_is_marked_only_for_colon_input() {
        assert!(
            !G::parse("120")
                .expect("行号输入应可解析")
                .has_explicit_column
        );
        assert!(
            G::parse("120:35")
                .expect("行:列输入应可解析")
                .has_explicit_column
        );
        assert!(
            G::parse(" 120 : 35 ")
                .expect("带空白的行:列输入应可解析")
                .has_explicit_column
        );
    }

    #[test]
    fn whitespace_around_and_between_numbers_is_tolerated() {
        assert_eq!(
            G::parse("  120 "),
            Some(G {
                line: 119,
                column: 0,
                has_explicit_column: false
            })
        );
        assert_eq!(
            G::parse("12 : 34"),
            Some(G {
                line: 11,
                column: 33,
                has_explicit_column: true
            })
        );
    }

    #[test]
    fn empty_non_numeric_and_multi_colon_input_is_rejected() {
        assert_eq!(G::parse(""), None);
        assert_eq!(G::parse("   "), None);
        assert_eq!(G::parse("abc"), None);
        assert_eq!(G::parse("12abc"), None);
        assert_eq!(G::parse("1:2:3"), None);
        assert_eq!(G::parse("120:"), None);
        assert_eq!(G::parse(":35"), None);
    }

    #[test]
    fn zero_and_negative_numbers_are_rejected() {
        assert_eq!(G::parse("0"), None);
        assert_eq!(G::parse("-1"), None);
        assert_eq!(G::parse("0:5"), None);
        assert_eq!(G::parse("5:0"), None);
        assert_eq!(G::parse("5:-2"), None);
    }

    #[test]
    fn in_bounds_line_and_column_are_unchanged() {
        let content = "first\nsecond line\nthird";
        assert_eq!(
            G::clamped(0, 2, false, content),
            G {
                line: 0,
                column: 2,
                has_explicit_column: false
            }
        );
        assert_eq!(
            G::clamped(2, 4, false, content),
            G {
                line: 2,
                column: 4,
                has_explicit_column: false
            }
        );
    }

    #[test]
    fn out_of_range_line_clamps_to_last_line() {
        let content = "first\nsecond line\nthird";
        assert_eq!(
            G::clamped(99, 0, false, content),
            G {
                line: 2,
                column: 0,
                has_explicit_column: false
            }
        );
    }

    #[test]
    fn out_of_range_column_clamps_to_line_end() {
        let content = "first\nsecond line\nthird";
        assert_eq!(
            G::clamped(1, 99, false, content),
            G {
                line: 1,
                column: 11,
                has_explicit_column: false
            }
        );
    }

    #[test]
    fn any_input_in_an_empty_document_clamps_to_origin() {
        assert_eq!(
            G::clamped(4, 9, false, ""),
            G {
                line: 0,
                column: 0,
                has_explicit_column: false
            }
        );
    }

    #[test]
    fn trailing_newline_leaves_an_addressable_empty_line() {
        assert_eq!(
            G::clamped(9, 3, false, "a\n"),
            G {
                line: 1,
                column: 0,
                has_explicit_column: false
            }
        );
        assert_eq!(
            G::clamped(9, 3, false, "a\nb"),
            G {
                line: 1,
                column: 1,
                has_explicit_column: false
            }
        );
    }

    #[test]
    fn crlf_content_clamps_per_line() {
        let content = "first\r\nsecond\r\nthird";
        assert_eq!(
            G::clamped(0, 99, false, content),
            G {
                line: 0,
                column: 5,
                has_explicit_column: false
            }
        );
        assert_eq!(
            G::clamped(1, 99, false, content),
            G {
                line: 1,
                column: 6,
                has_explicit_column: false
            }
        );
        assert_eq!(
            G::clamped(2, 1, false, content),
            G {
                line: 2,
                column: 1,
                has_explicit_column: false
            }
        );
        assert_eq!(
            G::clamped(9, 0, false, content),
            G {
                line: 2,
                column: 0,
                has_explicit_column: false
            }
        );
    }

    #[test]
    fn cr_only_content_clamps_per_line() {
        let content = "a\rb";
        assert_eq!(
            G::clamped(0, 99, false, content),
            G {
                line: 0,
                column: 1,
                has_explicit_column: false
            }
        );
        assert_eq!(
            G::clamped(1, 0, false, content),
            G {
                line: 1,
                column: 0,
                has_explicit_column: false
            }
        );
        assert_eq!(
            G::clamped(9, 0, false, content),
            G {
                line: 1,
                column: 0,
                has_explicit_column: false
            }
        );
    }

    #[test]
    fn column_limit_counts_utf16_units_of_an_emoji_line() {
        let content = "a\u{1F600}b";
        assert_eq!(
            G::clamped(0, 3, false, content),
            G {
                line: 0,
                column: 3,
                has_explicit_column: false
            }
        );
        assert_eq!(
            G::clamped(0, 4, false, content),
            G {
                line: 0,
                column: 4,
                has_explicit_column: false
            }
        );
        assert_eq!(
            G::clamped(0, 5, false, content),
            G {
                line: 0,
                column: 4,
                has_explicit_column: false
            }
        );
    }

    #[test]
    fn explicit_column_marker_survives_clamping() {
        let parsed = G::parse("99:2").expect("行:列输入应可解析");
        let clamped = G::clamped(
            parsed.line,
            parsed.column,
            parsed.has_explicit_column,
            "a\nb",
        );
        assert!(clamped.has_explicit_column);
        assert_eq!(
            clamped,
            G {
                line: 1,
                column: 1,
                has_explicit_column: true
            }
        );
    }

    #[test]
    fn prefill_is_one_based_line_and_column() {
        assert_eq!(G::prefill(0, 0), "1:1");
        assert_eq!(G::prefill(119, 34), "120:35");
    }

    #[test]
    fn parse_then_prefill_keeps_the_position() {
        for text in ["1:1", "120:35", "  12 : 34  ", "999"] {
            let parsed = G::parse(text).expect("这些输入都该可解析");
            let (line, column) = (parsed.line, parsed.column);
            let again = G::parse(&G::prefill(line, column)).expect("预填串必须自己能解析");
            assert_eq!(
                (again.line, again.column),
                (line, column),
                "{text} 往返后位置变了"
            );
            assert!(again.has_explicit_column, "预填串总是 `行:列` 两段");
        }
    }
}
