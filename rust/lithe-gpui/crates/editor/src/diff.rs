//! 把 Core 的 `git.diff` 行归一成组件库的内联差异标记。
//!
//! 组件库的模型是 [`DiffHunk`]：在**某一行之前**插入若干「已删除」行，并把紧跟着的
//! `added` 行标成新增（`DiffHunk::line` 是 0 基行号）。所以这里只做一件事：按 Core 给出的
//! 行序把「删除 / 新增」攒成一段一段，遇到上下文行就收一段。

use lithe_ui::editor::DiffHunk;

/// Core `git.diff` 的一行（只留这里用得到的字段）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffRow {
    /// `context` / `insertion` / `deletion` / `changed` / `information`。
    pub kind: String,
    /// 旧文件里的行号（1 基）；纯新增行没有。
    pub old_line: Option<usize>,
    /// 新文件里的行号（1 基）；纯删除行没有。
    pub new_line: Option<usize>,
    /// 左侧（旧）文本。
    pub left: Option<String>,
}

/// 这一段差异占新文件的哪些行、前面挂了哪些被删掉的文本。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffSection {
    /// 0 基行号：被删文本插在这一行之前。
    pub line: usize,
    /// 从 `line` 起连续多少行是新增/修改后的内容。
    pub added: usize,
    /// 这些新增行前面被删掉的旧文本（按原顺序）。
    pub removed: Vec<String>,
}

impl DiffSection {
    /// 转成组件库的 `DiffHunk`。
    pub fn to_hunk(&self) -> DiffHunk {
        DiffHunk {
            line: self.line,
            added: self.added,
            removed: self.removed.iter().cloned().map(Into::into).collect(),
        }
    }
}

/// 按 Core 的行序把差异攒成一段一段。
///
/// 规则只有一条：`deletion` 攒进「待插入的删除文本」，`insertion` / `changed` 开一段或续一段，
/// 其余 kinds（`context` / `information`）结束当前这一段。
pub fn sections(rows: &[DiffRow]) -> Vec<DiffSection> {
    let mut out: Vec<DiffSection> = Vec::new();
    let mut pending_removed: Vec<String> = Vec::new();

    for row in rows {
        match row.kind.as_str() {
            "deletion" => {
                if let Some(text) = &row.left {
                    pending_removed.push(text.clone());
                }
            }
            "insertion" | "changed" => {
                // `changed` 的左侧文本就是被替换掉的那一行。
                if row.kind == "changed"
                    && let Some(text) = &row.left
                {
                    pending_removed.push(text.clone());
                }
                let Some(new_line) = row.new_line else {
                    continue;
                };
                let line = new_line.saturating_sub(1);
                match out.last_mut() {
                    // 紧挨着上一段（行号连续、且中间没被上下文断开）就续上。
                    Some(last) if last.line + last.added == line && pending_removed.is_empty() => {
                        last.added += 1;
                    }
                    Some(last) if last.line + last.added == line => {
                        last.added += 1;
                        last.removed.append(&mut pending_removed);
                    }
                    _ => {
                        out.push(DiffSection {
                            line,
                            added: 1,
                            removed: std::mem::take(&mut pending_removed),
                        });
                    }
                }
            }
            _ => {
                // 上下文行把这一段收住：后面再出现的删除文本属于下一段。
                if !pending_removed.is_empty() {
                    if let Some(last) = out.last_mut() {
                        last.removed.append(&mut pending_removed);
                    } else {
                        pending_removed.clear();
                    }
                }
            }
        }
    }

    // 收尾：末尾还有没挂上去的删除文本，就贴在最后一段后面。
    if !pending_removed.is_empty()
        && let Some(last) = out.last_mut()
    {
        last.removed.append(&mut pending_removed);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(kind: &str, old_line: Option<usize>, new_line: Option<usize>, left: &str) -> DiffRow {
        DiffRow {
            kind: kind.to_string(),
            old_line,
            new_line,
            left: Some(left.to_string()),
        }
    }

    #[test]
    fn an_insertion_becomes_one_section_at_the_line_before_it() {
        let rows = [
            row("context", Some(1), Some(1), "a"),
            row("insertion", None, Some(2), "b"),
            row("insertion", None, Some(3), "c"),
            row("context", Some(2), Some(4), "d"),
        ];
        let sections = sections(&rows);
        assert_eq!(sections.len(), 1, "连续新增是一段");
        assert_eq!(sections[0].line, 1, "0 基行号，插在原第 2 行之前");
        assert_eq!(sections[0].added, 2);
        assert!(sections[0].removed.is_empty());
    }

    #[test]
    fn a_changed_row_carries_its_old_text_as_removed() {
        let rows = [
            row("context", Some(1), Some(1), "a"),
            row("changed", Some(2), Some(2), "old"),
            row("context", Some(3), Some(3), "c"),
        ];
        let sections = sections(&rows);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].line, 1);
        assert_eq!(sections[0].added, 1);
        assert_eq!(sections[0].removed, vec!["old".to_string()]);
    }

    #[test]
    fn a_pure_deletion_hangs_on_the_section_that_follows_it() {
        let rows = [
            row("context", Some(1), Some(1), "a"),
            row("deletion", Some(2), None, "gone"),
            row("insertion", None, Some(2), "new"),
            row("context", Some(3), Some(3), "c"),
        ];
        let sections = sections(&rows);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].line, 1);
        assert_eq!(sections[0].added, 1);
        assert_eq!(sections[0].removed, vec!["gone".to_string()]);
    }

    #[test]
    fn context_separates_two_sections() {
        let rows = [
            row("insertion", None, Some(1), "one"),
            row("context", Some(1), Some(2), "a"),
            row("insertion", None, Some(3), "three"),
        ];
        let sections = sections(&rows);
        assert_eq!(sections.len(), 2, "中间隔了一行上下文就是两段");
        assert_eq!(sections[0].line, 0);
        assert_eq!(sections[1].line, 2);
    }

    #[test]
    fn an_empty_diff_has_no_sections() {
        assert!(sections(&[]).is_empty());
    }

    #[test]
    fn a_section_converts_to_the_library_hunk() {
        let section = DiffSection {
            line: 3,
            added: 2,
            removed: vec!["old".to_string()],
        };
        let hunk = section.to_hunk();
        assert_eq!(hunk.line, 3);
        assert_eq!(hunk.added, 2);
        assert_eq!(hunk.removed, vec![gpui::SharedString::from("old")]);
    }
}
