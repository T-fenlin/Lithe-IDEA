use std::path::PathBuf;

use gpui_kit::component::input::{Position, Rope, RopeExt};
use lithe_gpui_java::{JavaLanguageService, JavaTarget};
use lithe_gpui_shared::core_json;
use serde_json::{Value, json};

const BUILTIN_NAVIGATION: &str = "lsp.builtinNavigation";

const DEFINITION_METHOD: &str = "textDocument/definition";

const JAVA_EXTENSION: &str = "java";

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
    file_path: &str,
    text: &Rope,
    offset: usize,
) -> Result<Option<NavTarget>, String> {
    if let Some(service) = java.filter(|_| is_java_source(file_path)) {
        let (line, utf16_column) = core_position(text, offset);
        match service.definition(
            std::path::Path::new(file_path),
            &text.to_string(),
            line as u32,
            utf16_column as u32,
        ) {
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
            Err(reason) => println!("S1_JAVA_UNAVAILABLE reason={reason} fallback=builtin"),
        }
    }

    Ok(definition_target(file_path, text, offset)?.map(NavTarget::Local))
}

fn is_java_source(file_path: &str) -> bool {
    std::path::Path::new(file_path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(JAVA_EXTENSION))
}

pub(crate) fn definition_target(
    file_path: &str,
    text: &Rope,
    offset: usize,
) -> Result<Option<Position>, String> {
    let (line, utf16_column) = core_position(text, offset);
    let request = json!({
        "filePath": file_path,
        "text": text.to_string(),
        "position": { "line": line, "utf16Column": utf16_column },
        "method": DEFINITION_METHOD,
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

pub(crate) fn core_position(text: &Rope, offset: usize) -> (u64, usize) {
    let point = text.offset_to_point(offset);
    let line = text.slice_line(point.row);
    (point.row as u64, line.byte_to_utf16_idx(point.column))
}

pub(crate) fn editor_position(text: &Rope, line: usize, utf16_column: usize) -> Position {
    let line_text = text.slice_line(line);
    let byte = line_text.utf16_to_byte_idx(utf16_column.min(line_text.len_utf16()));
    Position::new(line as u32, line_text.slice(..byte).chars().count() as u32)
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
    #[allow(dead_code)]
    pub(crate) fn can_go_back(&self) -> bool {
        !self.entries.is_empty() && self.current_index.is_none_or(|index| index > 0)
    }

    #[allow(dead_code)]
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

    fn offset_of(text: &Rope, line: u32, character: u32) -> usize {
        text.position_to_offset(&Position::new(line, character))
    }

    #[test]
    fn definition_lands_on_the_declaration() {
        let text = Rope::from(SAMPLE);
        let offset = offset_of(&text, 11, 18);

        let target = definition_target("Hello.java", &text, offset).expect("Core 调用成功");

        let target = target.expect("样本里 `add` 有声明，应当有目标");
        assert_eq!(target.line, 2, "目标应是第 3 行（0 基 2）的声明");
        assert_eq!(target.character, 15, "`add` 在 `    static int ` 之后");
    }

    #[test]
    fn definition_at_a_declaration_returns_itself() {
        let text = Rope::from(SAMPLE);
        let offset = offset_of(&text, 2, 15);

        let target = definition_target("Hello.java", &text, offset)
            .expect("Core 调用成功")
            .expect("声明自己也是一个目标");

        assert_eq!((target.line, target.character), (2, 15));
    }

    #[test]
    fn definition_without_a_matching_identifier_reports_nothing() {
        let text = Rope::from(SAMPLE);
        let offset = offset_of(&text, 1, 0);

        assert_eq!(
            definition_target("Hello.java", &text, offset).expect("Core 调用成功"),
            None
        );
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
        let text = Rope::from(SAMPLE);
        let offset = offset_of(&text, 11, 18);

        let target = resolve_target(None, "Hello.java", &text, offset)
            .expect("Core 调用成功")
            .expect("样本里 `add` 有声明");
        let NavTarget::Local(position) = target else {
            panic!("没有 Java 服务时只可能是当前文件内的位置");
        };
        assert_eq!((position.line, position.character), (2, 15));
    }

    #[test]
    fn utf16_and_character_columns_are_not_interchangeable() {
        let text = Rope::from("String s = \"🎉\"; int value = 1;\n");

        let offset = offset_of(&text, 0, 20);
        assert_eq!(
            core_position(&text, offset),
            (0, 21),
            "字节偏移要按 UTF-16 报给 Core"
        );

        let position = editor_position(&text, 0, 21);
        assert_eq!(
            position.character, 20,
            "Core 的 UTF-16 列换回编辑器时要变成字符列（少掉的正是 🎉 的第二个码元）"
        );
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
}
