//! 代码跳转的**数据侧**：Core 轻量导航调用 + 跳转历史。
//!
//! 对应阶段 10 第一批（`lithe-gpui/PLAN.md` §10）：`F12` / `Ctrl+单击` → Core 的**无进程**
//! 轻量导航 → 编辑器寻址，以及 `← →` 回退。**不启 JDTLS**，所以目标只可能在当前文件内
//! （Core 的轻量导航本身就只返回当前文件的位置，见下）。
//!
//! ## 为什么不自己解析 Java
//!
//! `develop-lithe` 的第一条硬规则是"复用成熟上游能力"，而 Core 已经有这条命令：
//!
//! - 命令名 `lsp.builtinNavigation`：`rust/lithe-core/src/protocol/command.rs:368`；
//! - 契约描述："Return lightweight current-file definition/reference locations"
//!   （`shared/contracts/rust-core-api.md:135`），细节在 `:1143-1151`
//!   —— "definition prefers declaration-looking occurrences, while references returns
//!   all matching identifier occurrences"；
//! - 请求字段 `{filePath, text, position{line, utf16Column}, method}`
//!   （`rust/lithe-core/src/lsp/lightweight/symbols.rs:23-28`，`camelCase`）；
//! - 响应 `{locations: [{filePath, range{start{line,utf16Column}, end{...}}, isReadOnly,
//!   displayPath}]}`（同文件 `:68-80`）。
//!
//! 所以本模块**只做两件事**：拼请求 / 读响应，以及把 Core 的 UTF-16 列与编辑器的
//! `Position` 列口径转过来。一行 Java 语法都不在这里解析。
//!
//! ## 为什么不用 `java.sourceDefinition`
//!
//! 那条命令（`rust-core-api.md:1723-1725`）要求调用方先给出 `declarationName`
//! （+ 可选 `memberName`），也就是"**光标下这个标识符叫什么**"得由宿主自己先答出来 ——
//! 那就等于在 gpui 侧写一遍"取光标处标识符"，而 `lsp.builtinNavigation` 本身就是
//! 按**光标位置**取标识符的（`symbols.rs:163-172` 的 `identifier_at`）。选了后者，
//! `F12` 与 `Ctrl+单击` 因此共用同一条路径。
//!
//! ⚠️ **已知的精度边界**（第一批的有意取舍，契约里也是这么写的）：Core 的
//! `looks_like_declaration` 只认 `class`/`func`/`let` 这类**紧邻标识符的声明关键字**
//! （`symbols.rs:336-368`），不认 Java 的 `类型 名字(` 形态（`static int add(` 的前一个
//! token 是 `int`）。所以 Java 里"定义"多半走的是"找不到声明形态 → 退回全部出现位置、
//! 取结果里的第一个"（`symbols.rs:183-185` + `identifier_occurrences` 的文档序）。
//! 类型感知的精确跳转属于第二批（JDTLS）。
//!
//! ## 口径陷阱：三套列
//!
//! | 口径 | 出处 |
//! | --- | --- |
//! | **字节偏移**（`state.cursor()` 的返回值） | `gpui-base-0.6.6/src/input/base/state.rs:2763-2772` |
//! | **字符数列**（`Position.character`，`cursor_position` / `set_cursor_position` 用的） | `gpui-base-0.6.6/src/input/base/rope_ext.rs:190-198`（"The column is in characters"） |
//! | **UTF-16 列**（Core 的 `utf16Column`） | Core 契约；`lsp::builtinNavigation` 按它索引 |
//!
//! `Position.character` **不是** UTF-16（`rope_ext.rs:337-343` 的实现在行内先
//! `byte_to_utf16_idx` 再数 `chars()`），所以这里必须显式换算 —— 只在 ASCII 上
//! 两者才恰好相等。换算集中在本文件的 `core_position` / `editor_position` 两个函数里。

use std::path::PathBuf;

use gpui_kit::component::input::{Position, Rope, RopeExt};
use lithe_gpui_java::{JavaLanguageService, JavaTarget};
use lithe_gpui_shared::core_json;
use serde_json::{Value, json};

/// Core 的轻量导航命令名（`rust/lithe-core/src/protocol/command.rs:368`）。
const BUILTIN_NAVIGATION: &str = "lsp.builtinNavigation";

/// 语义 = 定义/声明。
///
/// 这三个取值在 Core 里走"优先声明形态的出现位置"的那条分支
/// （`rust/lithe-core/src/lsp/lightweight/symbols.rs:174-185`）。
const DEFINITION_METHOD: &str = "textDocument/definition";

/// 会走 JDTLS 语义跳转的文件后缀。
///
/// 只有 `.java`：`java.workspacePolicy` 的判据也一样（契约 `:1280-1291`），
/// 给非 Java 文件起一个 Java 语言服务没有任何意义。
const JAVA_EXTENSION: &str = "java";

/// 跳转历史的上限，逐值照 Windows 的 `DEFAULT_MAX_ENTRIES`
/// （`windows/tauri/src/features/editor/stores/jump-list.store.ts:43`）。
const MAX_JUMP_ENTRIES: usize = 100;

/// 一次跳转解析出来的目标。
///
/// 第二批新增 `File`（跨文件 / 跨模块）与 `Virtual`（`jdt://` 库源码）两支；
/// 第一批的 `Local` 一支保留为**降级路径**：JDTLS 不可用时（没装 JDK 21、载荷不全）
/// 跳转不该整体失效。
pub(crate) enum NavTarget {
    /// 当前文件内的位置（第一批的 `lsp.builtinNavigation`，返回的列已换算成编辑器口径）。
    Local(Position),
    /// 磁盘上的目标文件 + **Core 口径**的位置（0 基行 / UTF-16 列）。
    ///
    /// 列换算要用**目标文件**的正文，所以推迟到文件真的打开之后再做
    /// （见 `EditorPane::apply_definition`）：提前读一遍盘既多一次 I/O，
    /// 又可能与真正打开的正文不一致。
    File {
        /// 目标文件绝对路径。
        path: PathBuf,
        /// 0 基行。
        line: u32,
        /// 0 基 UTF-16 码元列。
        utf16_column: u32,
    },
    /// JDT 虚拟源码（`jdt://`）：只读正文已经取回来了。
    Virtual {
        /// 不透明虚拟 URI，也是 buffer 的去重键。
        uri: String,
        /// 标签显示名（Core 归一化出的源码形态路径，例如 `java.base/java/lang/String.java`）。
        display_path: String,
        /// 只读源码正文。
        text: String,
        /// 0 基行。
        line: u32,
        /// 0 基 UTF-16 码元列。
        utf16_column: u32,
    },
}

/// 解析一次跳转请求：**优先 JDTLS 的语义结果**，不可用时降级到第一批的轻量导航。
///
/// 三条判据（都写在调用点，避免"有时候语义、有时候启发式"这种不可解释的行为）：
///
/// 1. 文件不是 `.java`，或还没有 Java 语言服务 → 直接走轻量导航；
/// 2. JDTLS 返回**结果**（哪怕是"没有位置"）→ 语义结果是权威的，**不回退**：
///    回退会让"JDT 说这里没有定义"变成"启发式猜一个位置"，那比不跳更糟；
/// 3. JDTLS **不可用**（`Err`：没装 JDK 21、载荷缺失、会话已关闭）→ 回退到轻量导航，
///    并留一条 `S1_JAVA_UNAVAILABLE`。
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

/// 这个路径该不该走 Java 语义跳转。
fn is_java_source(file_path: &str) -> bool {
    std::path::Path::new(file_path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(JAVA_EXTENSION))
}

/// 取光标（字节偏移 `offset`）处标识符的**定义位置**；没有可跳的目标时返回 `None`。
///
/// `text` 传 `Rope` 而不是 `String`：列换算要用行切片（`RopeExt::slice_line`），
/// 而整篇 `to_string()` 只在拼请求时发生一次。
///
/// 失败返回 Core 的错误码原文（`CoreError::code`）；信封级失败（响应不是合法 JSON）
/// 没有错误码，退回 `CoreError` 的说明性文案 —— 与 `explorer` / `git` 两个 feature
/// 的口径一致（`lithe-gpui/crates/explorer/src/model.rs:236-242`）。
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

    // `definition` 语义下 Core 返回的是**候选列表**（最多 200 条，`symbols.rs:190-199`）；
    // 真机同样只跳第一个（`navigation-command-actions.ts:479` 的 `const target = locations[0]`）。
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

/// 字节偏移 → Core 契约要的 `(0 基行, UTF-16 列)`。
///
/// `pub(crate)`：补全（`crate::completion`）也要求 Core 口径的位置，
/// 而"编辑器偏移 → Core 列"的换算**只该有一条实现** —— 三套列（字节 / 字符 / UTF-16）
/// 混用是这个文件最容易出错的地方（见模块文档的列表与
/// `utf16_and_character_columns_are_not_interchangeable` 那条测试）。
pub(crate) fn core_position(text: &Rope, offset: usize) -> (u64, usize) {
    let point = text.offset_to_point(offset);
    let line = text.slice_line(point.row);
    (
        point.row as u64,
        // `point.column` 是行内**字节**列（`RopeExt::offset_to_point`，`rope_ext.rs:313-319`）。
        line.byte_to_utf16_idx(point.column),
    )
}

/// Core 的 `(0 基行, UTF-16 列)` → 编辑器的 `Position`（列是**字符数**）。
///
/// 越界的列按行尾夹住（`position_to_offset` 自己也会夹，但这里先夹掉可以少一次
/// 越界路径）：`utf16_to_byte_idx` 的入参要求不超过行的 UTF-16 长度。
///
/// `pub(crate)`：跨文件 / `jdt://` 两支拿到的是**目标文件**的 Core 口径位置，
/// 换算必须用目标文件自己的 `Rope`，所以 `editor_view.rs` 也要用这个函数。
pub(crate) fn editor_position(text: &Rope, line: usize, utf16_column: usize) -> Position {
    let line_text = text.slice_line(line);
    let byte = line_text.utf16_to_byte_idx(utf16_column.min(line_text.len_utf16()));
    Position::new(line as u32, line_text.slice(..byte).chars().count() as u32)
}

/// 跳转历史里的一项：**打开路径**（buffer 的去重键，也是它的身份）+ 光标位置。
///
/// 不用 buffer 下标：关掉一个标签会让它后面所有下标左移一位，历史项会指错人。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JumpEntry {
    /// 打开时用的路径，与 `Buffer::path` 同源。
    pub(crate) path: PathBuf,
    /// 0 基行列；列与 `EditorState::cursor_position()` 同口径（**字符数**）。
    pub(crate) position: Position,
}

/// 跳转历史：`entries` + `current_index`（`None` = 停在"现在"，不在回看中）。
///
/// 语义逐条照 Windows 的 jump list（`windows/tauri/src/features/editor/stores/jump-list.store.ts`）：
///
/// - 新跳转**先截断前进分支**再入栈（`truncateForwardEntries`，`:49-53`）；
/// - 与栈顶同一位置不重复入栈（`pushEntry` 的 `isSamePosition`，`:77-92`）；
/// - `go_back` 在"现在"时**先把当前位置压栈**，再退到它的前一项（`:136-142`）——
///   所以"跳一次 + 点一次 `←`"就能回到跳转前的位置；
/// - `can_go_back` / `can_go_forward` 的判据同 `:189-200`（空历史时两个都 `false`）。
///
/// 与真机的两点差异（都是有意缩小范围，不改变上面四条语义）：
///
/// 1. 真机还记 `bufferId` / `paneId` / `scrollTop` / `scrollLeft` / `timestamp`；
///    第一批只有单窗格，滚动位置也还没有"跳转时恢复"的需求，所以只留**路径 + 位置**；
/// 2. 真机的 `appendEntry` 在超上限时 `shift()` 掉最旧一项却**不调整** `currentIndex`
///    （`:55-60`）；本实现做到上限时把下标一起减一，避免回看中途错位。
#[derive(Debug, Default)]
pub(crate) struct JumpHistory {
    /// 时间顺序（旧 → 新）。
    entries: Vec<JumpEntry>,
    /// 当前停在 `entries` 的哪一项；`None` = 停在"现在"（栈顶之外）。
    current_index: Option<usize>,
}

impl JumpHistory {
    /// `←` 可不可用。
    ///
    /// ⚠️ **2026-09-27 起没有生产调用点**：它唯一的用主是标签栏那对导航按钮，
    /// 已按维护者口径删除（见 `editor_view.rs` 里 `render_nav_group` 的墓碑注释）。
    /// 方法保留、标 `#[allow(dead_code)]` —— 单测仍在用（`can_go_back` 那几条断言），
    /// 而且把 `← →` 接到键位（`Alt+←` / `Alt+→`）时还是靠它算禁用态。
    #[allow(dead_code)]
    pub(crate) fn can_go_back(&self) -> bool {
        !self.entries.is_empty() && self.current_index.is_none_or(|index| index > 0)
    }

    /// `→` 可不可用。
    ///
    /// ⚠️ 与 [`JumpHistory::can_go_back`] 同一处置：2026-09-27 起没有生产调用点，保留 + `allow`。
    #[allow(dead_code)]
    pub(crate) fn can_go_forward(&self) -> bool {
        self.current_index
            .is_some_and(|index| index + 1 < self.entries.len())
    }

    /// 记一次**新的跳转**（跳转前的位置）。
    ///
    /// 调用点只在跳转**成功之后**（真机也是先判目标、再 `pushEntry`，
    /// `navigation-command-actions.ts:449-467`），所以失败的 `F12` 不会污染历史。
    pub(crate) fn record(&mut self, entry: JumpEntry) {
        self.truncate_forward();
        if self.entries.last() != Some(&entry) {
            self.push(entry);
        }
        self.current_index = None;
    }

    /// `←`：返回要跳回的位置。
    ///
    /// `present` 是**当前**位置：停在"现在"时它会被先压栈，这样 `→` 还能回到这里。
    pub(crate) fn go_back(&mut self, present: JumpEntry) -> Option<JumpEntry> {
        if self.entries.is_empty() {
            return None;
        }

        let index = match self.current_index {
            None => {
                self.push(present);
                // 刚压进去的 `present` 在栈顶，所以目标是它的前一项。
                self.entries.len().checked_sub(2)?
            }
            Some(0) => return None,
            Some(index) => index - 1,
        };

        self.current_index = Some(index);
        self.entries.get(index).cloned()
    }

    /// `→`：返回要前进到的位置。
    pub(crate) fn go_forward(&mut self) -> Option<JumpEntry> {
        let index = self.current_index?;
        let next = index + 1;
        let entry = self.entries.get(next)?.clone();
        self.current_index = Some(next);
        Some(entry)
    }

    /// 回看中时把"前进分支"丢掉（真机 `truncateForwardEntries`）。
    fn truncate_forward(&mut self) {
        match self.current_index {
            Some(index) if index + 1 < self.entries.len() => self.entries.truncate(index + 1),
            _ => {}
        }
    }

    /// 入栈并守住上限。
    fn push(&mut self, entry: JumpEntry) {
        // 先截断前进分支再压栈的调用点：`record`；`go_back` 里压的是"现在"，
        // 那时 `current_index` 就是 `None`（停在现在），压完必然成为栈顶，所以只需在这里
        // 处理"溢出时下标跟着左移"。
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

    /// 「声明在使用之前」的样本：这正是 `lsp.builtinNavigation` 的轻量启发式所需要的顺序
    /// （见模块文档的"已知的精度边界"）。
    ///
    /// 用 `concat!` 一行一段而不是续行转义：续行里的前导空白会被 rustfmt 重新对齐，
    /// 那会**静默改掉**这些列断言测的东西。
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

    /// 起点的字节偏移：`definition_target` 收的是 `state.cursor()` 的返回值。
    fn offset_of(text: &Rope, line: u32, character: u32) -> usize {
        text.position_to_offset(&Position::new(line, character))
    }

    // 这两条是**与 Core 的真实往返**，不是自造桩：`lsp.builtinNavigation` 一行声明
    // 一处使用的样本里，"定义"必须落在声明那一行（第 3 行，0 基 2）。
    #[test]
    fn definition_lands_on_the_declaration() {
        let text = Rope::from(SAMPLE);
        // 第 12 行（0 基 11）第 19 列（0 基 18）是 `add(1, 2)` 里的 `add`。
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
        // 空行（第 2 行）上没有标识符。
        let offset = offset_of(&text, 1, 0);

        assert_eq!(
            definition_target("Hello.java", &text, offset).expect("Core 调用成功"),
            None
        );
    }

    /// 只有 `.java` 会走 JDTLS 语义跳转（`java.workspacePolicy` 的判据同样只看 Java 源）。
    #[test]
    fn only_java_sources_take_the_semantic_path() {
        assert!(is_java_source(r"src\demo\App.java"));
        assert!(is_java_source("/ws/src/demo/App.JAVA"));
        assert!(!is_java_source("/ws/src/App.kt"));
        assert!(!is_java_source("/ws/README"));
    }

    /// **没有 Java 服务时必须退回第一批的轻量导航**（有服务那一路的真实验证在
    /// `lithe-gpui-java` 的选定式端到端测试里，需要本地 JDTLS + JDK 21）。
    ///
    /// 这条守住的是"降级不是空手"：同一个光标位置，`java = None` 得到的结果必须与
    /// 第一批完全相同。
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

    /// 非 ASCII 正文：Core 的 `utf16Column` 与编辑器的字符列**不同**，换算必须两边都对。
    ///
    /// `中文` 在 UTF-16 里是 2 个码元、在字符列里也是 2 个，分不开两个口径；用 `🎉`
    /// （非 BMP：2 个 UTF-16 码元、**1** 个字符）才能把它们分开。
    #[test]
    fn utf16_and_character_columns_are_not_interchangeable() {
        let text = Rope::from("String s = \"🎉\"; int value = 1;\n");

        // 光标放在 `value` 的首字符（0 基**字符**列 20）。到它之前的 UTF-16 长度是
        // 20 个字符 + 🎉 多出来的 1 个码元 = 21。
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
        // 三次跳转：5 → 1 → 2 → 3。历史记的是**跳转前**的位置，
        // 所以栈里是 [5, 1, 2]，`current_index` 停在"现在"。
        history.record(entry("A.java", 5, 0));
        history.record(entry("A.java", 1, 0));
        history.record(entry("A.java", 2, 0));

        // 回退两步：现在停在 1，前面还留着 2 / 3 可以前进。
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

        // 新的跳转截断前进分支（2 与 3 都没了）。
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

    /// 「跳一次 → `←` → `→`」必须回到三个已知位置，这是本阶段验收的主链路。
    #[test]
    fn back_then_forward_visits_origin_and_target() {
        let mut history = JumpHistory::default();
        let origin = entry("A.java", 11, 19);

        // 一次成功的跳转：只把"跳转前的位置"记进历史。
        history.record(origin.clone());
        assert!(history.can_go_back());
        assert!(!history.can_go_forward());

        // `←`：先压入"现在"（声明处），再回到跳转前的位置。
        assert_eq!(
            history.go_back(entry("A.java", 2, 16)),
            Some(origin.clone())
        );
        assert!(!history.can_go_back(), "已经到头了");
        assert!(history.can_go_forward());

        // `→`：回到声明处。
        assert_eq!(history.go_forward(), Some(entry("A.java", 2, 16)));
    }

    #[test]
    fn repeated_records_at_the_same_place_do_not_grow_the_stack() {
        let mut history = JumpHistory::default();
        history.record(entry("A.java", 1, 1));
        history.record(entry("A.java", 1, 1));

        // 栈里只有一个位置：`←` 会先压入"现在"再去前一项，所以仍然能回到它。
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
