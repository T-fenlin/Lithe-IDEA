//! 语义跳转、以及**编辑器列口径与 LSP 列口径之间的唯一换算点**。
//!
//! 两个口径必须分清，混用不会报错、只会静默跳错位置：
//!
//! * **编辑器**：`(行, 字符列)` —— 一列就是一个字符（`char`），跳转历史里记的是它；
//! * **LSP / Core**：`(行, UTF-16 码元列)` —— 一个 emoji 是两列。协议边界上的一切
//!   （补全、悬停、定义、诊断、代码动作的请求与响应）都走这个口径。
//!
//! 组件库的编辑器只给**字节偏移**一种坐标（`CodeEditor::cursor_offset` / `set_cursor_offset`），
//! 所以换算全在本文件：`core_position` 把偏移读成 UTF-16 列，`editor_position` /
//! `offset_for_utf16` 把协议坐标换回编辑器坐标，`line_character` / `byte_offset` 是跳转历史
//! 用的字符列口径。

use std::ops::Range;
use std::path::{Path, PathBuf};

use lithe_gpui_java::{JavaLanguageService, JavaTarget};
use lithe_gpui_shared::core_json;
use lsp_types::Position;
use serde_json::{Value, json};

const BUILTIN_NAVIGATION: &str = "lsp.builtinNavigation";

const JAVA_EXTENSION: &str = "java";

/// 一个 Go-to 目标。
///
/// 三个目标共用一个枚举而不是三份平行代码：从「取光标位置」到「落到目标」的每一步都只差
/// 几个字符串，复制三遍只会让后续的修复（列口径、超时、虚拟文档）漏掉其中两条。
///
/// 三个取值各有出处，改任一个都要一起看：`method` 是无 Java 服务时 `lsp.builtinNavigation`
/// 用的 LSP 方法名，`tag` 进日志，`label_key` 是「没找到」通知里的目标名。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NavigationTarget {
    Definition,
    Implementation,
    TypeDefinition,
}

impl NavigationTarget {
    /// 无 Java 服务时退回 `lsp.builtinNavigation` 用的 LSP 方法名。
    ///
    /// Core 侧的语义操作名（`definition` / `implementation` / `typeDefinition`）不在这里：
    /// 发 `lsp.request` 的是 `lithe-gpui-java`，那是它与 Core 之间的契约。
    pub(crate) const fn method(self) -> &'static str {
        match self {
            Self::Definition => "textDocument/definition",
            Self::Implementation => "textDocument/implementation",
            Self::TypeDefinition => "textDocument/typeDefinition",
        }
    }

    /// 日志与诊断行里的短名。
    pub(crate) const fn tag(self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::Implementation => "implementation",
            Self::TypeDefinition => "typeDefinition",
        }
    }

    /// 「没找到目标」通知里的目标名（i18n key）。
    pub(crate) const fn label_key(self) -> &'static str {
        match self {
            Self::Definition => "lithe.navigation.definition",
            Self::Implementation => "lithe.navigation.implementation",
            Self::TypeDefinition => "lithe.navigation.typeDefinition",
        }
    }
}

const MAX_JUMP_ENTRIES: usize = 100;

pub(crate) enum NavTarget {
    Local(Position),
    File {
        path: PathBuf,
        line: u32,
        utf16_column: u32,
    },
    Virtual {
        uri: String,
        display_path: String,
        text: String,
        line: u32,
        utf16_column: u32,
    },
}

pub(crate) fn resolve_target(
    java: Option<&JavaLanguageService>,
    target: NavigationTarget,
    file_path: &str,
    text: &str,
    offset: usize,
) -> Result<Option<NavTarget>, String> {
    if let Some(service) = java.filter(|_| is_java_source(file_path)) {
        let (line, utf16_column) = core_position(text, offset);
        let path = std::path::Path::new(file_path);
        let line = line as u32;
        let utf16_column = utf16_column as u32;
        let answer = match target {
            NavigationTarget::Definition => service.definition(path, text, line, utf16_column),
            NavigationTarget::Implementation => {
                service.implementation(path, text, line, utf16_column)
            }
            NavigationTarget::TypeDefinition => {
                service.type_definition(path, text, line, utf16_column)
            }
        };
        match answer {
            Ok(Some(JavaTarget::File {
                path,
                line,
                utf16_column,
            })) => {
                return Ok(Some(NavTarget::File {
                    path,
                    line,
                    utf16_column,
                }));
            }
            Ok(Some(JavaTarget::Virtual {
                uri,
                display_path,
                text,
                line,
                utf16_column,
            })) => {
                return Ok(Some(NavTarget::Virtual {
                    uri,
                    display_path,
                    text,
                    line,
                    utf16_column,
                }));
            }
            Ok(None) => return Ok(None),
            Err(reason) => println!(
                "S1_JAVA_UNAVAILABLE kind={} reason={reason} fallback=builtin",
                target.tag()
            ),
        }
    }

    Ok(builtin_target(target, file_path, text, offset)?.map(NavTarget::Local))
}

fn is_java_source(file_path: &str) -> bool {
    std::path::Path::new(file_path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(JAVA_EXTENSION))
}

/// 没有语言服务时的兜底：Core 的轻量导航只看当前这份正文。
///
/// `target` 决定问的是定义、实现还是类型定义 —— Core 侧三条路都认（`builtin_navigation`），
/// 所以退回兜底不会把「转到实现」悄悄降级成「转到定义」。
pub(crate) fn builtin_target(
    target: NavigationTarget,
    file_path: &str,
    text: &str,
    offset: usize,
) -> Result<Option<Position>, String> {
    let (line, utf16_column) = core_position(text, offset);
    let request = json!({
        "filePath": file_path,
        "text": text,
        "position": { "line": line, "utf16Column": utf16_column },
        "method": target.method(),
    });

    let data = core_json(BUILTIN_NAVIGATION, request).map_err(|error| match error.code() {
        Some(code) => code.to_string(),
        None => error.to_string(),
    })?;

    let Some(start) = data
        .as_ref()
        .and_then(|data| data.get("locations"))
        .and_then(Value::as_array)
        .and_then(|locations| locations.first())
        .and_then(|location| location.pointer("/range/start"))
    else {
        return Ok(None);
    };

    let line = start
        .get("line")
        .and_then(Value::as_u64)
        .ok_or("响应缺少 locations[0].range.start.line")?;
    let utf16_column = start
        .get("utf16Column")
        .and_then(Value::as_u64)
        .ok_or("响应缺少 locations[0].range.start.utf16Column")?;

    Ok(Some(editor_position(
        text,
        line as usize,
        utf16_column as usize,
    )))
}

/// A byte offset pulled onto a whole character inside `text` (an offset past its end lands on the end).
pub(crate) fn clamped_offset(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

/// The zero-based line a byte offset stands on, and the byte offset that line starts at.
fn line_start(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 0usize;
    let mut start = 0usize;
    for (at, ch) in text.char_indices() {
        if at >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            start = at + 1;
        }
    }
    (line, start)
}

/// Where a zero-based line starts, and its own text — its terminator (`\n`, `\r\n` or `\r`) left out.
/// `None` when the document has no such line at all.
fn line_span(text: &str, line: usize) -> Option<(usize, &str)> {
    let mut start = 0usize;
    for _ in 0..line {
        let at = text[start..].find('\n')?;
        start += at + 1;
    }
    let rest = &text[start..];
    let end = rest.find('\n').map_or(rest.len(), |at| at);
    let body = rest[..end].strip_suffix('\r').unwrap_or(&rest[..end]);
    Some((start, body))
}

/// The LSP reading of a byte offset: zero-based line and a column in **UTF-16 code units**.
pub(crate) fn core_position(text: &str, offset: usize) -> (u64, usize) {
    let offset = clamped_offset(text, offset);
    let (line, start) = line_start(text, offset);
    let column = text[start..offset].chars().map(char::len_utf16).sum();
    (line as u64, column)
}

/// The byte offset of a zero-based line and a **UTF-16** column, pulled onto a whole character inside that
/// line (a column past the line's end lands on its end, a column inside a character lands on that character's
/// first byte, and a line the document does not have lands on the end of the text).
pub(crate) fn offset_for_utf16(text: &str, line: usize, utf16_column: usize) -> usize {
    let Some((start, body)) = line_span(text, line) else {
        return text.len();
    };
    let mut units = 0usize;
    let mut here = start;
    for (at, ch) in body.char_indices() {
        if units > utf16_column {
            return here;
        }
        here = start + at;
        units += ch.len_utf16();
    }
    if units > utf16_column {
        here
    } else {
        start + body.len()
    }
}

/// The editor's own reading of a byte offset: zero-based line and a **character** column.
pub(crate) fn line_character(text: &str, offset: usize) -> Position {
    let offset = clamped_offset(text, offset);
    let (line, start) = line_start(text, offset);
    Position::new(line as u32, text[start..offset].chars().count() as u32)
}

/// The editor position of a protocol position: the UTF-16 column is read back as a character column, which
/// is what a jump history remembers.
pub(crate) fn editor_position(text: &str, line: usize, utf16_column: usize) -> Position {
    line_character(text, offset_for_utf16(text, line, utf16_column))
}

/// The byte offset of an editor position: zero-based line and a **character** column, clamped the same way
/// as [`offset_for_utf16`].
pub(crate) fn byte_offset(text: &str, position: Position) -> usize {
    let Some((start, body)) = line_span(text, position.line as usize) else {
        return text.len();
    };
    let column = position.character as usize;
    let mut chars = 0usize;
    let mut here = start;
    for (at, _) in body.char_indices() {
        if chars > column {
            return here;
        }
        here = start + at;
        chars += 1;
    }
    if chars > column {
        here
    } else {
        start + body.len()
    }
}

/// The identifier the byte offset **stands on** — letters, digits and `_` — or `None` over whitespace and
/// punctuation, and `None` for an offset that is not on a whole character.
pub(crate) fn word_range(text: &str, offset: usize) -> Option<Range<usize>> {
    let offset = clamped_offset(text, offset);
    let is_word = |ch: char| ch.is_alphanumeric() || ch == '_';
    if !text[offset..].chars().next().is_some_and(is_word) {
        return None;
    }
    let mut start = offset;
    while start > 0 {
        let Some(previous) = text[..start].chars().next_back() else {
            break;
        };
        if !is_word(previous) {
            break;
        }
        start -= previous.len_utf8();
    }
    let mut end = offset;
    for ch in text[offset..].chars() {
        if !is_word(ch) {
            break;
        }
        end += ch.len_utf8();
    }
    Some(start..end)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JumpEntry {
    pub(crate) path: PathBuf,
    pub(crate) position: Position,
}

#[derive(Debug, Default)]
pub(crate) struct JumpHistory {
    entries: Vec<JumpEntry>,
    current_index: Option<usize>,
}

impl JumpHistory {
    // 这两个过去挂着 `#[allow(dead_code)]`：库实现了但从没有调用点。现在 `EditorPane`
    // 把它们透出去给菜单的启用判定（`EditorPane::can_go_back` / `can_go_forward`），
    // 有了真调用点就不能再靠 allow 包着了。
    pub(crate) fn can_go_back(&self) -> bool {
        !self.entries.is_empty() && self.current_index.is_none_or(|index| index > 0)
    }

    pub(crate) fn can_go_forward(&self) -> bool {
        self.current_index
            .is_some_and(|index| index + 1 < self.entries.len())
    }

    pub(crate) fn record(&mut self, entry: JumpEntry) {
        self.truncate_forward();
        if self.entries.last() != Some(&entry) {
            self.push(entry);
        }
        self.current_index = None;
    }

    pub(crate) fn go_back(&mut self, present: JumpEntry) -> Option<JumpEntry> {
        if self.entries.is_empty() {
            return None;
        }

        let index = match self.current_index {
            None => {
                self.push(present);
                self.entries.len().checked_sub(2)?
            }
            Some(0) => return None,
            Some(index) => index - 1,
        };

        self.current_index = Some(index);
        self.entries.get(index).cloned()
    }

    pub(crate) fn go_forward(&mut self) -> Option<JumpEntry> {
        let index = self.current_index?;
        let next = index + 1;
        let entry = self.entries.get(next)?.clone();
        self.current_index = Some(next);
        Some(entry)
    }

    fn truncate_forward(&mut self) {
        match self.current_index {
            Some(index) if index + 1 < self.entries.len() => self.entries.truncate(index + 1),
            _ => {}
        }
    }

    fn push(&mut self, entry: JumpEntry) {
        if self.entries.len() >= MAX_JUMP_ENTRIES {
            self.entries.remove(0);
            if let Some(index) = self.current_index.as_mut() {
                *index = index.saturating_sub(1);
            }
        }
        self.entries.push(entry);
    }

    /// Points every entry whose path moved with `from` at its new home (a renamed file, or a whole
    /// folder's subtree).
    pub(crate) fn remap(&mut self, from: &Path, to: &Path) {
        for entry in &mut self.entries {
            if let Some(moved) = remap_path(&entry.path, from, to) {
                entry.path = moved;
            }
        }
    }

    /// Drops every entry under `target` (a deleted file or folder). The current position is reset
    /// because the surviving indices shift; the next Back re-derives it.
    pub(crate) fn remove_path(&mut self, target: &Path) {
        let before = self.entries.len();
        self.entries
            .retain(|entry| remap_path(&entry.path, target, target).is_none());
        if self.entries.len() != before {
            self.current_index = None;
        }
    }
}

/// Rewrites `path` when it sits at or under `from`, moving it under `to`. Matching is by whole
/// path components (`Path::strip_prefix`), so renaming `a/b` leaves `a/bc` alone. `None` when
/// `path` is elsewhere.
pub(crate) fn remap_path(path: &Path, from: &Path, to: &Path) -> Option<PathBuf> {
    let rest = path.strip_prefix(from).ok()?;
    Some(to.join(rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = concat!(
        "public class Hello {\n",
        "\n",
        "    static int add(int a, int b) {\n",
        "        return a + b;\n",
        "    }\n",
        "\n",
        "    static int twice(int value) {\n",
        "        return value * 2;\n",
        "    }\n",
        "\n",
        "    static int compute() {\n",
        "        int sum = add(1, 2);\n",
        "        return twice(sum);\n",
        "    }\n",
        "}\n",
    );

    fn entry(path: &str, line: u32, character: u32) -> JumpEntry {
        JumpEntry {
            path: PathBuf::from(path),
            position: Position::new(line, character),
        }
    }

    fn offset_of(text: &str, line: u32, character: u32) -> usize {
        byte_offset(text, Position::new(line, character))
    }

    #[test]
    fn definition_lands_on_the_declaration() {
        let offset = offset_of(SAMPLE, 11, 18);

        let target = builtin_target(NavigationTarget::Definition, "Hello.java", SAMPLE, offset)
            .expect("Core 调用成功");

        let target = target.expect("样本里 `add` 有声明，应当有目标");
        assert_eq!(target.line, 2, "目标应是第 3 行（0 基 2）的声明");
        assert_eq!(target.character, 15, "`add` 在 `    static int ` 之后");
    }

    #[test]
    fn definition_at_a_declaration_returns_itself() {
        let offset = offset_of(SAMPLE, 2, 15);

        let target = builtin_target(NavigationTarget::Definition, "Hello.java", SAMPLE, offset)
            .expect("Core 调用成功")
            .expect("声明自己也是一个目标");

        assert_eq!((target.line, target.character), (2, 15));
    }

    #[test]
    fn definition_without_a_matching_identifier_reports_nothing() {
        let offset = offset_of(SAMPLE, 1, 0);

        assert_eq!(
            builtin_target(NavigationTarget::Definition, "Hello.java", SAMPLE, offset)
                .expect("Core 调用成功"),
            None
        );
    }

    /// 三个目标各有各的 Core 操作名、LSP 方法与通知文案 —— 混用会让「转到实现」
    /// 悄悄变成「转到定义」，而那正是这单要修的东西。
    #[test]
    fn each_navigation_target_keeps_its_own_core_operation_and_method() {
        let table = [
            (
                NavigationTarget::Definition,
                "definition",
                "textDocument/definition",
                "lithe.navigation.definition",
            ),
            (
                NavigationTarget::Implementation,
                "implementation",
                "textDocument/implementation",
                "lithe.navigation.implementation",
            ),
            (
                NavigationTarget::TypeDefinition,
                "typeDefinition",
                "textDocument/typeDefinition",
                "lithe.navigation.typeDefinition",
            ),
        ];
        for (target, operation, method, label_key) in table {
            assert_eq!(target.tag(), operation, "诊断行的短名与 Core 操作名不一致");
            assert_eq!(target.method(), method, "{operation} 的 LSP 方法名不对");
            assert_eq!(
                target.label_key(),
                label_key,
                "{operation} 的通知文案键不对"
            );
        }
    }

    /// 兜底路径要按目标换 `method`：Core 的 `builtin_navigation` 认实现与类型定义，
    /// 一律发 `textDocument/definition` 会把两者静默降级成定义。
    #[test]
    fn the_builtin_fallback_asks_for_the_same_kind_it_was_given() {
        let offset = offset_of(SAMPLE, 11, 18);
        for target in [
            NavigationTarget::Definition,
            NavigationTarget::Implementation,
            NavigationTarget::TypeDefinition,
        ] {
            // 断言的是「这条路走得通」：`add` 在样本里有声明，三种口径都该给出当前文件内的位置。
            let resolved = resolve_target(None, target, "Hello.java", SAMPLE, offset)
                .unwrap_or_else(|error| panic!("{} 的兜底不该失败：{error}", target.tag()));
            assert!(resolved.is_some(), "{} 在样本里应当有目标", target.tag());
        }
    }

    #[test]
    fn only_java_sources_take_the_semantic_path() {
        assert!(is_java_source(r"src\demo\App.java"));
        assert!(is_java_source("/ws/src/demo/App.JAVA"));
        assert!(!is_java_source("/ws/src/App.kt"));
        assert!(!is_java_source("/ws/README"));
    }

    #[test]
    fn without_a_java_service_the_builtin_navigation_still_answers() {
        let offset = offset_of(SAMPLE, 11, 18);

        let target = resolve_target(
            None,
            NavigationTarget::Definition,
            "Hello.java",
            SAMPLE,
            offset,
        )
        .expect("Core 调用成功")
        .expect("样本里 `add` 有声明");
        let NavTarget::Local(position) = target else {
            panic!("没有 Java 服务时只可能是当前文件内的位置");
        };
        assert_eq!((position.line, position.character), (2, 15));
    }

    #[test]
    fn utf16_and_character_columns_are_not_interchangeable() {
        let text = "String s = \"🎉\"; int value = 1;\n";

        let offset = offset_of(text, 0, 20);
        assert_eq!(
            core_position(text, offset),
            (0, 21),
            "字节偏移要按 UTF-16 报给 Core"
        );

        let position = editor_position(text, 0, 21);
        assert_eq!(
            position.character, 20,
            "Core 的 UTF-16 列换回编辑器时要变成字符列（少掉的正是 🎉 的第二个码元）"
        );
    }

    /// 行首、行尾、行尾之后、以及没有的那一行：都夹到该行的字符上，不切字符、不 panic。
    #[test]
    fn columns_are_clamped_onto_whole_characters_of_their_line() {
        let text = "ab\n🎉 cd\nlast";
        assert_eq!(offset_of(text, 0, 0), 0);
        assert_eq!(offset_of(text, 0, 99), 2, "行尾就是该行的换行符之前");
        assert_eq!(offset_of(text, 1, 0), 3);
        assert_eq!(
            offset_of(text, 1, 1),
            7,
            "**字符**列 1 是 emoji 之后那个空格（编辑器口径，一个 emoji 算一列）"
        );
        assert_eq!(offset_of(text, 1, 3), 9, "字符列 3 是 `d`");
        assert_eq!(offset_of(text, 2, 99), text.len());
        assert_eq!(offset_of(text, 99, 0), text.len(), "没有的行夹到文本末尾");
    }

    /// UTF-16 列与字符列不是一回事：emoji 占两列，落在它中间的那一列夹回它的起点。
    #[test]
    fn utf16_columns_are_clamped_too() {
        let text = "ab\n🎉 cd\nlast";
        assert_eq!(offset_for_utf16(text, 1, 0), 3);
        assert_eq!(offset_for_utf16(text, 1, 1), 3, "emoji 中间 → 它的起点");
        assert_eq!(offset_for_utf16(text, 1, 2), 7, "emoji 之后");
        assert_eq!(
            core_position(text, 7),
            (1, 2),
            "emoji 之后第 7 个字节按 UTF-16 报 2 列"
        );
    }

    #[test]
    fn a_word_is_the_identifier_an_offset_stands_on() {
        let text = "let total = 2;";
        assert_eq!(word_range(text, 5), Some(4..9), "`total` 从 4 到 9");
        assert_eq!(word_range(text, 4), Some(4..9));
        assert_eq!(word_range(text, 8), Some(4..9));
        assert_eq!(word_range(text, 2), Some(0..3), "`let`");
        assert_eq!(word_range(text, 3), None, "空格上不是词");
        assert_eq!(word_range(text, 9), None, "词之后紧邻的空格上不是词");
        assert_eq!(word_range(text, 15), None, "文本末尾之后也不是词");
        assert_eq!(word_range("中文 x", 0), Some(0..6), "中日韩字符算词字符");
    }

    #[test]
    fn new_jump_truncates_the_forward_branch() {
        let mut history = JumpHistory::default();
        history.record(entry("A.java", 5, 0));
        history.record(entry("A.java", 1, 0));
        history.record(entry("A.java", 2, 0));

        assert_eq!(
            history
                .go_back(entry("A.java", 3, 0))
                .map(|it| it.position.line),
            Some(2)
        );
        assert_eq!(
            history
                .go_back(entry("A.java", 2, 0))
                .map(|it| it.position.line),
            Some(1)
        );
        assert!(history.can_go_forward(), "还有 2 / 3 可以前进");

        history.record(entry("A.java", 1, 0));
        assert!(!history.can_go_forward(), "新跳转必须截断前进分支");
        assert_eq!(
            history
                .go_back(entry("A.java", 9, 0))
                .map(|it| it.position.line),
            Some(1),
            "截断后剩下的唯一一项就是刚记下的跳转起点"
        );
    }

    #[test]
    fn empty_history_cannot_go_either_way() {
        let mut history = JumpHistory::default();

        assert!(!history.can_go_back(), "没有历史时 `←` 必须是禁用态");
        assert!(!history.can_go_forward());
        assert_eq!(history.go_back(entry("A.java", 0, 0)), None);
        assert_eq!(history.go_forward(), None);
    }

    #[test]
    fn back_then_forward_visits_origin_and_target() {
        let mut history = JumpHistory::default();
        let origin = entry("A.java", 11, 19);

        history.record(origin.clone());
        assert!(history.can_go_back());
        assert!(!history.can_go_forward());

        assert_eq!(
            history.go_back(entry("A.java", 2, 16)),
            Some(origin.clone())
        );
        assert!(!history.can_go_back(), "已经到头了");
        assert!(history.can_go_forward());

        assert_eq!(history.go_forward(), Some(entry("A.java", 2, 16)));
    }

    #[test]
    fn repeated_records_at_the_same_place_do_not_grow_the_stack() {
        let mut history = JumpHistory::default();
        history.record(entry("A.java", 1, 1));
        history.record(entry("A.java", 1, 1));

        assert_eq!(
            history.go_back(entry("A.java", 5, 5)),
            Some(entry("A.java", 1, 1))
        );
    }

    #[test]
    fn history_stops_growing_at_the_cap() {
        let mut history = JumpHistory::default();
        for line in 0..(MAX_JUMP_ENTRIES as u32 + 10) {
            history.record(entry("A.java", line, 0));
        }

        assert_eq!(history.entries.len(), MAX_JUMP_ENTRIES);
        assert_eq!(
            history.entries.first().map(|it| it.position.line),
            Some(10),
            "溢出时丢的是最旧的一项"
        );
    }

    #[test]
    fn renaming_a_folder_moves_its_entries_and_leaves_neighbours_alone() {
        let mut history = JumpHistory::default();
        history.record(entry("/ws/a/b/Main.java", 1, 0));
        history.record(entry("/ws/a/bc/Other.java", 2, 0));
        history.record(entry("/ws/z/Third.java", 3, 0));

        history.remap(Path::new("/ws/a/b"), Path::new("/ws/a/renamed"));

        let paths: Vec<String> = history
            .entries
            .iter()
            .map(|entry| entry.path.to_string_lossy().replace('\\', "/"))
            .collect();
        assert_eq!(
            paths,
            vec![
                "/ws/a/renamed/Main.java",
                "/ws/a/bc/Other.java",
                "/ws/z/Third.java",
            ],
            "只改前缀在 `a/b` 下的项；`a/bc` 是按组件匹配的邻居，不受影响"
        );
    }

    #[test]
    fn remap_matches_a_whole_file_path_exactly() {
        assert_eq!(
            remap_path(
                Path::new("/ws/Old.java"),
                Path::new("/ws/Old.java"),
                Path::new("/ws/New.java")
            ),
            Some(PathBuf::from("/ws/New.java"))
        );
        assert_eq!(
            remap_path(
                Path::new("/ws/Old.java.bak"),
                Path::new("/ws/Old.java"),
                Path::new("/ws/New.java")
            ),
            None,
            "同目录下的相似名字不是同一个文件"
        );
    }

    #[test]
    fn a_deleted_folder_takes_its_entries_with_it() {
        let mut history = JumpHistory::default();
        history.record(entry("/ws/gone/Main.java", 1, 0));
        history.record(entry("/ws/gone/deep/Other.java", 2, 0));
        history.record(entry("/ws/gonex/Keep.java", 3, 0));
        history.current_index = Some(1);

        history.remove_path(Path::new("/ws/gone"));

        let paths: Vec<String> = history
            .entries
            .iter()
            .map(|entry| entry.path.to_string_lossy().replace('\\', "/"))
            .collect();
        assert_eq!(paths, vec!["/ws/gonex/Keep.java"]);
        assert_eq!(history.current_index, None, "下标已被删项打乱，需要重算");
    }
}
