//! 「转到行:列」的**纯逻辑层**：输入解析 + 按当前正文收敛行列。
//!
//! 逐行照 macOS 真源 `macos/Sources/Lithe/Models/Editor/GoToLineInput.swift`
//! （`parse` 在 `:24-35`、`clamped` 在 `:46-84`），它自己那 14 条单测在
//! `macos/Tests/LitheTests/GoToLineInputTests.swift` —— 这里把那 14 条**逐条**搬过来，
//! 所以两个平台的边界行为（CRLF / CR / 行尾空行 / emoji / 越界收敛）是同一份口径。
//!
//! ## 为什么这一层必须是纯函数
//!
//! 它同时被三处读：①输入框的"确定按钮可用吗"（每次按键都要判，**不能**碰 `EditorState`）；
//! ②提交时按**当时的**正文重新收敛（真源 `AppModel+GoToLine.swift:29-44` 原句
//! "Line and column are converged against the current document text right before jumping,
//! without caching stale line counts"）；③对话框预填串。三者都是"值进值出"，
//! 于是这里不起窗口、不建 `EditorState`、不碰时钟 —— 与 `navigation.rs` 的位置换算同一层。
//!
//! ## 列口径：**UTF-16 码元**（不是字符数）
//!
//! 真源的 `clamped` 走 `NSString`，所以列上限是行的 UTF-16 长度
//! （它自己的测试 `clampsColumnUsingUTF16LengthOfEmojiLine` 就是为这条边界写的）。
//! 本模块保持同一口径：Core 的 LSP 契约、`navigation::editor_position` 的入参都是 UTF-16 列，
//! 换一次口径就多一处"emoji 行跳错列"。

/// 解析后的「转到行:列」输入。
///
/// `line` / `column` 都是 **0 基**、列是 **UTF-16 码元**下标：真源的输入是 1 基文本，
/// 1 基 → 0 基的换算只发生在 [`GoToLineInput::parse`] 里（显示侧各自加 1 回去，
/// 不在这里做两套偏移）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GoToLineInput {
    /// 0 基行号。
    pub(crate) line: usize,
    /// 0 基列号（UTF-16 码元）。
    pub(crate) column: usize,
    /// 输入里**显式**带了 `:列` 那一段。
    ///
    /// 真源用它分两种收尾（`AppModel+GoToLine.swift:38-43`）：只有行号 ⇒ 选中整行；带列号 ⇒
    /// 光标落到那一列。⚠️ 本仓**只做后者**：上游 `gpui-base-0.7.0` 的 `InputBaseState` 没有
    /// 公开的选区 setter（`selected_range` 只有 getter，`:2992`；`select_to` 是 `pub(crate)`，
    /// `:3181`），所以"选中整行"这一半落不了地 —— 这个字段仍然留着并如实参与收敛，
    /// 因为它同时是"用户到底输了几段"的机器判据（诊断行要打它）。
    pub(crate) has_explicit_column: bool,
}

impl GoToLineInput {
    /// 解析 `"120"` / `"120:35"`（允许两侧与冒号两侧带空白）。
    ///
    /// 非法输入一律 `None`：空串、非数字、多个冒号、`0`、负数、缺一段的 `"120:"` / `":35"`。
    /// 返回 `None` 时调用方把"确定"置灰（真源 `GoToLineDialog.swift:155-158` 就是这么做的），
    /// 所以这里**不**做"宽容等于 1"之类的兜底。
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        // `split` 保留空段：`"120:"` / `":35"` 会在下面被 `one_based` 挡掉，
        // 而 `"1:2:3"` 直接因为段数不对被挡掉。
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

    /// 把 0 基行列收敛进 `content`：越界的行落到**最后一行**，越界的列落到**该行行尾**
    /// （不含换行符），负数落到原点（本侧的入参是 `usize`，所以只可能是 `0`）。
    ///
    /// 空文档只寻址原点。换行按 LF / CRLF / CR 三档切（与真源一致），结尾带换行符时
    /// 多出那一行**空行**是可寻址的。调用方必须在真的要跳之前拿**当时的**正文再收敛一次。
    pub(crate) fn clamped(
        line: usize,
        column: usize,
        has_explicit_column: bool,
        content: &str,
    ) -> Self {
        let mut line_index = 0usize;
        // 当前行起点（字节下标）。
        let mut scan = 0usize;

        loop {
            let rest = &content[scan..];
            // 本行的 UTF-16 长度（= 列上限）。
            let mut units = 0usize;
            // 行内容结束的字节下标（不含换行符）。
            let mut end_byte = scan;
            // 换行符占的字节数；`None` = 扫到文件尾都没遇到换行符。
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
                        // CRLF 是一个终止符（真源 `:66-68`）；CR 与 LF 都是单字节，
                        // 所以这里的字节数与真源的 UTF-16 单元数在终止符上恒等。
                        let next_is_lf = rest[offset + 1..].starts_with('\n');
                        terminator = Some(if next_is_lf { 2 } else { 1 });
                        break;
                    }
                    _ => units += ch.len_utf16(),
                }
            }

            // 命中了请求的行，或者已经扫过最后一行内容（`None` = 文件尾）——都停在这一行。
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

    /// 对话框的预填串：`"行:列"`，**1 基**（与状态栏的光标格同一个显示口径）。
    pub(crate) fn prefill(line: usize, column: usize) -> String {
        format!("{}:{}", line + 1, column + 1)
    }
}

/// `"12"` → `12`；空白、非数字、`0`、负数都是 `None`（真源 `oneBasedNumber`，`:86-91`）。
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

    /// 逐条对应真源 `GoToLineInputTests.parsesLineOnlyInputAsZeroBasedLineWithZeroColumn`。
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

    /// 真源 `parsesLineAndColumnInput`。
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

    /// 真源 `marksExplicitColumnOnlyForColonInput`：只有显式输入了列号才标记 ——
    /// 这个标记决定真源是"选中整行"还是"光标落到该列"，本侧只做后者（见结构体文档）。
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

    /// 真源 `toleratesWhitespaceAroundAndBetweenNumbers`。
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

    /// 真源 `rejectsEmptyNonNumericAndMultiColonInput`。反证：把 `"120:"` 当成
    /// "列缺省 = 1" 就会让确定键在明显残缺的输入上可用。
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

    /// 真源 `rejectsZeroAndNegativeNumbers`：1 基输入里 0 与负数非法。
    #[test]
    fn zero_and_negative_numbers_are_rejected() {
        assert_eq!(G::parse("0"), None);
        assert_eq!(G::parse("-1"), None);
        assert_eq!(G::parse("0:5"), None);
        assert_eq!(G::parse("5:0"), None);
        assert_eq!(G::parse("5:-2"), None);
    }

    /// 真源 `keepsInBoundsLineAndColumnUnchanged`。
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

    /// 真源 `clampsOutOfRangeLineToLastLine`。
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

    /// 真源 `clampsOutOfRangeColumnToLineEnd`。
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

    /// 真源 `clampsAnyInputInEmptyDocumentToOrigin`：空文档只寻址原点。
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

    /// 真源 `clampsToTrailingEmptyLineAfterFinalNewline`：`"a\n"` 有第二行（尾随空行）。
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

    /// 真源 `clampsLineAndColumnInCRLFContent`：CRLF 的终止符不计入上一行列上限。
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

    /// 真源 `clampsLineAndColumnInCRonlyContent`：CR-only 换行同样切行。
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

    /// 真源 `clampsColumnUsingUTF16LengthOfEmojiLine`：列上限数的是 **UTF-16 码元**。
    ///
    /// 反证：按 `chars().count()` 数会把 `"a😀b"` 的行长算成 3，于是 `"a😀b"` 的第 4 列
    /// 会被夹到 3 —— 而 LSP / Core 的列口径是 UTF-16，用户输入 4 就该落在 `b` 之后。
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

    /// 真源 `preservesExplicitColumnThroughClamping`：收敛不改变"带没带列号"这个标记 ——
    /// 它决定真源的收尾方式，被收敛顺手抹掉就会让"输 99:2 却选中整行"。
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

    /// 预填串是 **1 基**、且列与状态栏同口径（真源 `GoToLineDialog.swift:50-53`）。
    #[test]
    fn prefill_is_one_based_line_and_column() {
        assert_eq!(G::prefill(0, 0), "1:1");
        assert_eq!(G::prefill(119, 34), "120:35");
    }

    /// 预填串必须是**自己能解析的**，且位置逐字不变 —— 否则"打开对话框什么都不改直接确定"
    /// 会把用户的光标挪到别处（真源 `GoToLineDialog.swift:50-53` 的预填同样是 `行:列`）。
    ///
    /// ⚠️ `has_explicit_column` 会从 `false` 变 `true`，这是**对的**：预填永远写成 `行:列`
    /// 两段（真源也这样），所以"只输行号"那一档只存在于用户手敲的场景。断言里把它显式写出来，
    /// 免得后人把"往返后标记变了"当成 bug。
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
