//! Java 文件的**补全**：把 JDTLS 的 `textDocument/completion` 接到上游编辑器自带的补全菜单上。
//!
//! ## 为什么这个文件这么短
//!
//! 补全菜单本身**不是我们要写的东西**：`gpui-base` 0.6.6 的编辑器已经带了一整套
//! （弹窗渲染、`↑`/`↓` 选择、`Enter`/`Tab` 接受、`Esc` 关闭、`filterText` 过滤），
//! 只要给它一个 `CompletionProvider`：
//!
//! | 部件 | 位置 |
//! | --- | --- |
//! | 挂载点 | `Lsp.completion_provider`（`gpui-base-0.6.6/src/input/editor/lsp/mod.rs:39`），经 `EditorState::lsp_mut()` 赋值 |
//! | 触发 | 上游 `handle_completion_trigger`（`.../lsp/completions.rs:122`）：**每次文本变化**都问 `is_completion_trigger`，返回 `true` 就发请求并弹菜单 —— 所以"打字就提示"不需要我们绑按键 |
//! | 接受 | 上游 `insert_completion`（`.../lsp/overlay.rs:166-197`）：优先用 `textEdit` 的 range 替换前缀，否则在光标处插入 |
//! | 菜单宽度 | `Lsp.completion_menu.max_width`（默认 320px） |
//!
//! 官方的接线样例就在同一个依赖树里：`gpui-component-0.6.6/src/inspector.rs:104-107`
//! （`state.lsp_mut().completion_provider = Some(provider); cx.notify();`）。
//!
//! ## 数据从哪来：JDTLS 优先，Core 的轻量补全兜底
//!
//! 与 [`crate::navigation`] 的跳转**完全同一条口径**：
//!
//! 1. JDTLS 会话可用 → `JavaLanguageService::completion`（项目感知：JDK、依赖、Spring 类型都能出）；
//! 2. 会话不可用 / 超时 / 工作区不是 Java 工程 → Core 的 `lsp.builtinCompletions`
//!    （无进程，当前文件的标识符 + 前缀 `textEdit`，`rust/lithe-core/src/lsp/lightweight/symbols.rs:91-139`）。
//!
//! 降级不是"少一点"而是"不能一点都没有"：用户敲下 `.` 时，一个空菜单比慢一点的菜单更让人迷惑。
//! 两条路都打 `S1_JAVA_COMPLETION`，`source=jdtls|builtin` 一眼能看出这次是谁答的。
//!
//! ## 陈旧结果保护
//!
//! 上游接受**任何一次**请求的结果（它不检查代次），而打字会连发请求 —— 所以代次校验放在
//! **本模块**：每次 `completions()` 自增一个全局计数，回来时代次不是最新的就返回空列表
//! （空列表只会让菜单不更新，不会把上一个前缀的候选插进来）。与跳转的 `nav_generation` 同一口径。

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use gpui_kit::component::input::{CompletionProvider, Rope};
use gpui_kit::{App, AppContext as _, Task, Window};
use lithe_gpui_java::{JavaCompletionItem, JavaLanguageService};
use lithe_gpui_shared::core_json;
use lsp_types::{
    CompletionContext, CompletionItem, CompletionItemKind, CompletionResponse, CompletionTextEdit,
    Documentation, Position, TextEdit,
};
use serde_json::{Value, json};

/// Core 的轻量补全命令（无进程）。契约见 `shared/contracts/rust-core-api.md:137`。
const BUILTIN_COMPLETIONS: &str = "lsp.builtinCompletions";

/// 补全菜单的宽度。
///
/// 上游默认 **320px**（`gpui-base-0.6.6/src/input/editor/lsp/completions.rs:22-37`），
/// 它自己的文档就写了"长标签会被截断、宿主可以调宽"。Java 的候选普遍是带参数的签名
/// （`greet(String name) : String`），再加中文 `detail`，320px 会截得看不出区别；
/// 480px = 30rem 在 980px 最小窗口里也放得下，且不会盖住整屏。
const COMPLETION_MENU_WIDTH_PX: f32 = 480.;

/// 发出请求前的防抖窗口（毫秒）。
///
/// 打字会**连发**请求（上游每个字符都问一次 `is_completion_trigger`），而我们的 Core 调用是
/// **阻塞式**的、且要拿 `JavaLanguageService` 的会话锁 —— 不防抖的话，一个单词打完会有
/// 七八个后台任务排队抢锁，最后一个才是有用的。120ms 是"手感上察觉不到、又把连打合并掉"的量级
/// （与 Core 自己的 300ms inline 补全防抖同一思路，只是我们更短：菜单比行内提示更依赖即时感）。
const COMPLETION_DEBOUNCE_MS: u64 = 120;

/// 把补全装到一个 `EditorState` 上（`None` 时**什么都不做**：非 Java 文件不装）。
///
/// 两件事一起做，因为它们总是成对出现：
/// 1. 赋值 `Lsp.completion_provider`（挂载点见模块文档的表）；
/// 2. 调宽菜单（[`COMPLETION_MENU_WIDTH_PX`]）——上游默认 320px 会把 Java 签名截断。
///
/// 由 [`crate::editor_view`] 在两个位置调用：`open()`（新开的 Java buffer）与
/// `prepare_java()`（服务就绪后给**已经打开**的 Java buffer 补装）。
pub(crate) fn install(
    state: &mut gpui_kit::component::input::EditorState,
    provider: Option<Rc<dyn CompletionProvider>>,
) {
    let Some(provider) = provider else {
        return;
    };
    let lsp = state.lsp_mut();
    lsp.completion_provider = Some(provider);
    lsp.completion_menu.max_width = gpui_kit::px(COMPLETION_MENU_WIDTH_PX);
}

/// Java 的补全数据源。
pub(crate) struct JavaCompletionProvider {
    /// JDTLS 门面；`None` = 宿主没起 Java 工具（例如不是 Java 工程）→ 只剩轻量兜底。
    service: Option<Arc<JavaLanguageService>>,
    /// 这个 buffer 对应的磁盘路径（`jdt://` 虚拟源码不装本 provider，见 [`crate::editor_view`]）。
    file_path: PathBuf,
    /// 请求代次（跨线程共享：`completions()` 返回的 `Task` 在后台线程上跑）。
    generation: Arc<AtomicU64>,
}

impl JavaCompletionProvider {
    /// 建一个 provider。`service` 为 `None` 时只有轻量兜底（见模块文档）。
    pub(crate) fn new(
        service: Option<Arc<JavaLanguageService>>,
        file_path: PathBuf,
    ) -> Rc<dyn CompletionProvider> {
        Rc::new(Self {
            service,
            file_path,
            generation: Arc::new(AtomicU64::new(0)),
        })
    }

    /// 当前光标前缀（只用于诊断：`S1_JAVA_COMPLETION prefix=…`）。
    ///
    /// ⚠️ **只用字节域**：trait 给的 `offset` 是**字节**偏移（上游文档原话
    /// "The `offset` is in bytes of current cursor"）。这里刻意**不碰 ropey 的索引 API** ——
    /// `ropey 2.0.0-beta.1` 的 `Rope::char(idx)` 是按**字节**索引的（实测传入字符下标会 panic），
    /// 而 `RopeExt::offset_to_char_index` 又是另一套口径；两套混用正是"字节 / 字符 / UTF-16"
    /// 三套列最容易出错的地方（`navigation.rs` 的模块文档专门列了这张表）。
    /// 所以先把正文拿成 `&str`，再在字节域上按 `char::len_utf8` 往回走：
    /// `str` 的下标就是字节下标，语义唯一、不需要任何换算。
    fn prefix(body: &str, offset: usize) -> String {
        // 两端都夹到字符边界：越界或落在多字节字符中间时退到前一个边界，绝不 panic。
        let mut end = offset.min(body.len());
        while end > 0 && !body.is_char_boundary(end) {
            end -= 1;
        }
        let mut begin = end;
        while begin > 0 {
            let Some(previous) = body[..begin].chars().next_back() else {
                break;
            };
            if is_identifier_char(previous) || previous == '.' || previous == '@' {
                begin -= previous.len_utf8();
            } else {
                break;
            }
        }
        body[begin..end].to_string()
    }
}

impl CompletionProvider for JavaCompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui_kit::Result<CompletionResponse>> {
        // ⚠️ **位置必须用 `core_position` 换算**：trait 给的 `offset` 是**字节**偏移
        // （上游文档原话："The `offset` is in bytes of current cursor"），而 Core/JDT 要的是
        // `(0 基行, UTF-16 列)`。`RopeExt::offset_to_position` **不能**用在这里 ——
        // 它回的 `character` 是**字符**列（`gpui-base-0.6.6/src/input/base/rope_ext.rs:337-343`
        // 的 `chars().count()`），与 UTF-16 码元列只在"行内没有非 BMP 字符"时相等。
        // 用错的话，中文/emoji 之后的光标位置会整体偏左 ⇒ 补全算的是别的 token。
        let (line, utf16_column) = crate::navigation::core_position(text, offset);
        let position = Position::new(line as u32, utf16_column as u32);
        let body = text.to_string();
        let prefix = Self::prefix(&body, offset);
        // 映射 `textEdit` 时要按**这个 buffer 自己的 Rope** 把 Core 的 UTF-16 列换成编辑器口径，
        // 所以克隆一份带进后台任务（`Rope` 是 `Clone + Send`）。
        let rope = text.clone();
        let file_path = self.file_path.clone();
        let service = self.service.clone();
        let executor = cx.background_executor().clone();

        // 代次：这次请求是最新的才允许其结果生效（见模块文档的"陈旧结果保护"）。
        let mine = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = self.generation.clone();

        cx.background_spawn(async move {
            let started = Instant::now();
            // 防抖：把"连打同一个单词"合并成一次请求（理由见 [`COMPLETION_DEBOUNCE_MS`]）。
            // 等完先看一眼代次：已经有更新的请求了就直接放弃，连 Core 调用都不发。
            executor
                .timer(std::time::Duration::from_millis(COMPLETION_DEBOUNCE_MS))
                .await;
            if generation.load(Ordering::SeqCst) != mine {
                return Ok(CompletionResponse::Array(Vec::new()));
            }

            // 1) JDTLS 优先。
            if let Some(service) = service {
                match service.completion(&file_path, &body, position.line, position.character) {
                    Ok(items) => {
                        let count = items.len();
                        println!(
                            "S1_JAVA_COMPLETION source=jdtls prefix={prefix:?} items={count} ms={}",
                            started.elapsed().as_millis()
                        );
                        return Ok(deferred(generation, mine, items, &rope));
                    }
                    Err(error) => {
                        // 服务不可用/超时 → 降级到轻量补全，但**必须留证据**（不外抛，
                        // 否则用户看到的是"补全坏了"，而不是"降级了"）。
                        // ⚠️ 用 `println!` 与同族的 `S1_JAVA_*` 一致（它们全在 stdout，
                        // 见 `java/src/service.rs` 的诊断行）；验证脚本两个流都要重定向再 grep。
                        // 注意"文档被更新的版本取代"不会走到这里 —— 那一条在
                        // `JavaLanguageService::completion` 里已经翻成 `Ok(空)`（见那里的注释）。
                        println!(
                            "S1_JAVA_COMPLETION source=jdtls result=failed error={error} fallback=builtin"
                        );
                    }
                }
            }

            // 2) 兜底：Core 的轻量补全（无进程，当前文件标识符）。
            let items = builtin_completions(&file_path, &body, position);
            println!(
                "S1_JAVA_COMPLETION source=builtin prefix={prefix:?} items={} ms={}",
                items.len(),
                started.elapsed().as_millis()
            );
            Ok(deferred(generation, mine, items, &rope))
        })
    }

    /// **什么时候弹菜单**：这是"用户体验"最要紧的一条判据。
    ///
    /// 判据本身是纯函数 [`triggers_on`]，这里只是把上游给的两个参数里的 `new_text` 交过去
    /// （`offset` 用不上：上游只在**有文本变化**时调用本函数）。
    fn is_completion_trigger(&self, _offset: usize, new_text: &str, _cx: &mut App) -> bool {
        triggers_on(new_text)
    }
}

/// 补全触发判据（纯函数，便于单测 —— `is_completion_trigger` 需要 `App`，测起来重）。
///
/// - 标识符字符（字母 / 数字 / `_` / `$`）→ `true`：用户正在敲名字，边打边给候选是 IDE 的默认体验；
/// - `.` → `true`：成员访问，Java 里最常用的一次补全；
/// - `@` → `true`：注解（**Spring 的补全入口几乎全在这里**：`@Autowired` / `@Service` …）；
/// - 其它（空格、`)`、`;`、回车、粘贴一整段…）→ `false`：不然用户敲个分号也会弹菜单。
fn triggers_on(new_text: &str) -> bool {
    !new_text.is_empty()
        && new_text
            .chars()
            .all(|character| is_identifier_char(character) || character == '.' || character == '@')
}

/// 代次校验 + 组装响应。
///
/// 代次不是最新的（用户又敲了一个字符）→ 返回**空数组**：菜单保持不动，
/// 不会把旧前缀的候选显示成新前缀的候选。
fn deferred(
    generation: Arc<AtomicU64>,
    mine: u64,
    items: Vec<JavaCompletionItem>,
    text: &Rope,
) -> CompletionResponse {
    if generation.load(Ordering::SeqCst) != mine {
        return CompletionResponse::Array(Vec::new());
    }
    CompletionResponse::Array(
        items
            .into_iter()
            .map(|item| to_lsp_item(item, text))
            .collect(),
    )
}

/// 本 crate 的结构化候选 → `lsp_types` 的补全条目。
///
/// 四件事必须做对（都直接影响用户看到的与插入的内容）：
///
/// 1. **`text_edit` 必须带上**：上游优先用它替换已敲进去的前缀（`overlay.rs:175-187`），
///    没有它就退化成"在光标处再插一段"，`Sys` + `System` 会变成 `SysSystem`；
/// 2. ⚠️ **`textEdit` 的两个端点必须把 UTF-16 列换成编辑器的字符列**：
///    Core / JDT 的 `utf16Column` 是 **UTF-16 码元**，而上游接受补全时走
///    `self.text.position_to_offset(&edit.range.start)`，那个函数把 `character` 当**字符数**
///    （`gpui-base-0.6.6/src/input/base/rope_ext.rs:330-343`）。纯 ASCII 上两者恰好相等，
///    一旦这一行有非 BMP 字符（emoji、部分 CJK 扩展区）就会错位 —— 于是替换范围落错地方，
///    用户看到的就是"补全把别的字删了"或者"插重复"。换算复用 [`crate::navigation::editor_position`]
///    （跳转跨文件时用的就是它，同一个问题只有一条答案）；
/// 3. **`sort_text` / `filter_text` 原样透传**：排序与过滤听服务端的，它比标签更懂上下文；
/// 4. **`insert_text_format` 标成 `PLAIN_TEXT`**：文本在 `JavaLanguageService::completion` 里
///    已经用 Core 的 `lsp.plainSnippet` 转成纯文本了，再标成 snippet 会让下游按 snippet 解释。
///
/// ⚠️ 已知边界：上游 `insert_completion`（`overlay.rs:166-197`）**只应用 `text_edit` / `insert_text`，
/// 完全忽略 `additional_text_edits`**。JDT 的"接受候选时自动补 `import`"正是靠后者，
/// 所以本版接受一个需要 import 的类时不会自动补 import（登记在 `gpui/PLAN.md` §16）。
fn to_lsp_item(item: JavaCompletionItem, text: &Rope) -> CompletionItem {
    CompletionItem {
        label: item.label,
        kind: item.kind.and_then(completion_kind),
        detail: item.detail,
        documentation: item.documentation.map(Documentation::String),
        sort_text: item.sort_text,
        filter_text: item.filter_text,
        insert_text: Some(item.insert_text),
        insert_text_format: Some(lsp_types::InsertTextFormat::PLAIN_TEXT),
        text_edit: item.text_edit.map(|edit| {
            CompletionTextEdit::Edit(TextEdit {
                range: lsp_types::Range {
                    start: crate::navigation::editor_position(
                        text,
                        edit.start.line as usize,
                        edit.start.utf16_column as usize,
                    ),
                    end: crate::navigation::editor_position(
                        text,
                        edit.end.line as usize,
                        edit.end.utf16_column as usize,
                    ),
                },
                new_text: edit.new_text,
            })
        }),
        ..Default::default()
    }
}

/// LSP 的 `CompletionItemKind` 是**只有常量的透明结构体**（`lsp-types-0.97.0/src/completion.rs:24-56`：
/// 私有字段 + `lsp_enum!` 只生成常量与 `TryFrom<&str>`），无法从数字直接构造，
/// 所以这里把 Core 透传的数值映射到对应常量；未知值给 `None`（用户仍然能看到标签与图标以外的信息）。
fn completion_kind(kind: i64) -> Option<CompletionItemKind> {
    Some(match kind {
        1 => CompletionItemKind::TEXT,
        2 => CompletionItemKind::METHOD,
        3 => CompletionItemKind::FUNCTION,
        4 => CompletionItemKind::CONSTRUCTOR,
        5 => CompletionItemKind::FIELD,
        6 => CompletionItemKind::VARIABLE,
        7 => CompletionItemKind::CLASS,
        8 => CompletionItemKind::INTERFACE,
        9 => CompletionItemKind::MODULE,
        10 => CompletionItemKind::PROPERTY,
        11 => CompletionItemKind::UNIT,
        12 => CompletionItemKind::VALUE,
        13 => CompletionItemKind::ENUM,
        14 => CompletionItemKind::KEYWORD,
        15 => CompletionItemKind::SNIPPET,
        16 => CompletionItemKind::COLOR,
        17 => CompletionItemKind::FILE,
        18 => CompletionItemKind::REFERENCE,
        19 => CompletionItemKind::FOLDER,
        20 => CompletionItemKind::ENUM_MEMBER,
        21 => CompletionItemKind::CONSTANT,
        22 => CompletionItemKind::STRUCT,
        23 => CompletionItemKind::EVENT,
        24 => CompletionItemKind::OPERATOR,
        25 => CompletionItemKind::TYPE_PARAMETER,
        _ => return None,
    })
}

/// Core 的轻量补全：`{ filePath, text, position }` → `{ items: [ { label, kind?, textEdit } ] }`。
///
/// 只有"当前文件里的标识符"，但**不需要任何进程**，所以在 JDTLS 还没起来的那几秒里
/// 用户照样有候选（模块文档里的降级口径）。
fn builtin_completions(file_path: &std::path::Path, text: &str, position: Position) -> Vec<JavaCompletionItem> {
    let payload = json!({
        "filePath": file_path.to_string_lossy(),
        "text": text,
        "position": { "line": position.line, "utf16Column": position.character },
    });
    let Ok(Some(response)) = core_json(BUILTIN_COMPLETIONS, payload) else {
        return Vec::new();
    };
    response
        .get("items")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(builtin_item).collect())
        .unwrap_or_default()
}

/// 轻量补全的一条候选（字段见 `rust/lithe-core/src/lsp/lightweight/symbols.rs:40-45`）。
fn builtin_item(value: &Value) -> Option<JavaCompletionItem> {
    let label = value.get("label").and_then(Value::as_str)?.to_string();
    let edit = value.get("textEdit")?;
    let range = edit.get("range")?;
    let start = range.get("start")?;
    let end = range.get("end")?;
    let read_position = |value: &Value| {
        Some((
            value.get("line").and_then(Value::as_u64)? as u32,
            value.get("utf16Column").and_then(Value::as_u64)? as u32,
        ))
    };
    let (start_line, start_column) = read_position(start)?;
    let (end_line, end_column) = read_position(end)?;
    let new_text = edit
        .get("newText")
        .and_then(Value::as_str)
        .unwrap_or(label.as_str())
        .to_string();

    Some(JavaCompletionItem {
        label,
        insert_text: new_text.clone(),
        kind: value.get("kind").and_then(Value::as_i64),
        detail: None,
        documentation: None,
        sort_text: None,
        filter_text: None,
        text_edit: Some(lithe_gpui_java::JavaTextEdit {
            start: lithe_gpui_java::JavaPosition {
                line: start_line,
                utf16_column: start_column,
            },
            end: lithe_gpui_java::JavaPosition {
                line: end_line,
                utf16_column: end_column,
            },
            new_text,
        }),
    })
}

/// 标识符字符（Java 允许 `$`，`Character.isJavaIdentifierPart` 也认它）。
fn is_identifier_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 触发判据：标识符、`.`、`@` 触发；空白与标点不触发（敲分号不该弹菜单），空串不触发。
    #[test]
    fn trigger_policy_follows_java_input() {
        for text in ["a", "Z", "_", "$", "1", ".", "@", "System.out"] {
            assert!(triggers_on(text), "{text:?} 应当触发补全");
        }
        for text in ["", " ", ";", ")", "\n", "()", "a ", "= ", "//"] {
            assert!(!triggers_on(text), "{text:?} 不该触发补全");
        }
    }

    /// 前缀提取：只吃标识符 / `.` / `@`，遇到别的字符就停（诊断行的可读性靠它）。
    ///
    /// ⚠️ 传进去的是**字节**偏移（trait 的契约）；这里特意用非 ASCII 正文断言一遍，
    /// 因为"字节当字符用"在纯 ASCII 样本上永远测不出来。
    #[test]
    fn prefix_stops_at_non_identifier_characters() {
        let source = "System.out.pri";
        assert_eq!(JavaCompletionProvider::prefix(source, source.len()), source);

        let source = "foo(ba";
        assert_eq!(JavaCompletionProvider::prefix(source, source.len()), "ba");

        // 中文一个字符 3 字节：把字节偏移当字符下标用会取到错的前缀（实测会得到 "Syste"）。
        let source = "中文 System.ou";
        assert_eq!(JavaCompletionProvider::prefix(source, source.len()), "System.ou");

        // 越界 / 落在多字节字符中间：夹到字符边界，不 panic。
        assert_eq!(JavaCompletionProvider::prefix(source, 999), "System.ou");
        assert_eq!(JavaCompletionProvider::prefix(source, 1), "");
    }

    /// `kind` 映射：Core 透传的数值必须落到对应常量，未知值给 `None`（不 panic）。
    #[test]
    fn completion_kinds_map_and_unknown_values_are_dropped() {
        assert_eq!(completion_kind(2), Some(CompletionItemKind::METHOD));
        assert_eq!(completion_kind(7), Some(CompletionItemKind::CLASS));
        assert_eq!(completion_kind(21), Some(CompletionItemKind::CONSTANT));
        assert_eq!(completion_kind(0), None);
        assert_eq!(completion_kind(99), None);
    }

    /// 组装：`textEdit` 必须落到 `lsp_types` 的 `TextEdit` 上（否则会插重复），
    /// 且格式标成纯文本（snippet 已在 java crate 里转换过）。
    ///
    /// 这一行是纯 ASCII，所以**字符列 == UTF-16 列**；非 ASCII 的换算在下面那条测试里。
    #[test]
    fn item_mapping_keeps_the_replacement_range() {
        // 第 3 行 = `    greet(na)`：`na` 在这行的第 10..12 列（0 基），正是要被替换的前缀。
        let rope = Rope::from_str("line0\nline1\nline2\n    greet(na)\n");
        let mapped = to_lsp_item(sample_item(3, 10, 3, 12, "greet(name)"), &rope);
        assert_eq!(mapped.label, "greet(String)");
        assert_eq!(mapped.sort_text.as_deref(), Some("0001"));
        assert_eq!(
            mapped.insert_text_format,
            Some(lsp_types::InsertTextFormat::PLAIN_TEXT)
        );
        match mapped.text_edit.expect("必须带 textEdit") {
            CompletionTextEdit::Edit(edit) => {
                assert_eq!(edit.range.start, Position::new(3, 10));
                assert_eq!(edit.range.end, Position::new(3, 12));
                assert_eq!(edit.new_text, "greet(name)");
            }
            other => panic!("必须是普通 Edit：{other:?}"),
        }
    }

    /// ⚠️ **列口径**：Core 给的是 UTF-16 码元列，而上游 `insert_completion` 把它当**字符列**解
    /// （`overlay.rs:179-180` → `position_to_offset`），行内含非 BMP 字符（emoji / 部分 CJK 扩展区，
    /// 一个字符 = 两个 UTF-16 码元）时两者不等，不换算就会把替换范围落到错误的位置
    /// （表现是"补全删错字"或"插重复"）。
    #[test]
    fn item_mapping_converts_utf16_columns_to_character_columns() {
        // 第 1 行：`🙂`（1 个字符 = 2 个 UTF-16 码元）+ 4 个空格 + `na`。
        // `na` 的 UTF-16 列 = 2（emoji）+ 4（空格）= 6；字符列 = 1 + 4 = 5。
        let rope = Rope::from_str("x\n🙂    na\n");
        let mapped = to_lsp_item(sample_item(1, 6, 1, 8, "name"), &rope);
        match mapped.text_edit.expect("必须带 textEdit") {
            CompletionTextEdit::Edit(edit) => {
                assert_eq!(edit.range.start, Position::new(1, 5), "UTF-16 列 6 → 字符列 5");
                assert_eq!(edit.range.end, Position::new(1, 7), "UTF-16 列 8 → 字符列 7");
            }
            other => panic!("必须是普通 Edit：{other:?}"),
        }
    }

    /// 一条样例候选（`greet(String)`，带替换范围）。
    fn sample_item(
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
        new_text: &str,
    ) -> JavaCompletionItem {
        JavaCompletionItem {
            label: "greet(String)".to_string(),
            insert_text: new_text.to_string(),
            kind: Some(2),
            detail: None,
            documentation: None,
            sort_text: Some("0001".to_string()),
            filter_text: None,
            text_edit: Some(lithe_gpui_java::JavaTextEdit {
                start: lithe_gpui_java::JavaPosition {
                    line: start_line,
                    utf16_column: start_column,
                },
                end: lithe_gpui_java::JavaPosition {
                    line: end_line,
                    utf16_column: end_column,
                },
                new_text: new_text.to_string(),
            }),
        }
    }
}
