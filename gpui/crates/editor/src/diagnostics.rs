//! Java 文件的**诊断波浪线**：把 JDTLS 推来的诊断写进上游编辑器自带的诊断集合。
//!
//! ## 上游有没有"宿主塞诊断"的入口：**有**，而且是公开 API
//!
//! 这一条是本模块成立的前提，先把证据列清楚（都在 `gpui-base-0.6.6` 里）：
//!
//! | 部件 | 位置 |
//! | --- | --- |
//! | 存取器 | `EditorState::diagnostics()` / `diagnostics_mut() -> Option<&mut DiagnosticSet>`（`src/input/base/state.rs:840-847`，`pub`） |
//! | 底层集合 | `input::DiagnosticSet`（`src/input/editor/diagnostics.rs:186`）：`new` / `clear` / `push(impl Into<Diagnostic>)` / `extend` / `len` / `for_offset` |
//! | 诊断类型 | `input::Diagnostic { range: Range<Position>, severity, code, code_description, source, message, related_information, tags, data }`（同文件 `:17-52`）+ `DiagnosticSeverity`（`:76-83`），以及构造器 `Diagnostic::new(range, message)` / `.with_severity(..)` / `.with_code(..)` / `.with_source(..)`（`:97-120`） |
//! | 渲染 | `input/base/element.rs:1602-1610` 把集合里的每一条变成 `HighlightStyle { underline: Some(UnderlineStyle { color, thickness: 1px, wavy: true }) }`（`diagnostic_highlight_style`，`:29-47`），再在 `:1637` 与语法 / 语义高亮合成；颜色来自主题的 `highlight_theme.style.status.{error,warning,info,hint}`（`gpui-component-0.6.6/src/input/input.rs:504-509`） |
//!
//! 三件由这段证据**直接决定**的实现约束：
//!
//! 1. **可见性**：`Diagnostic` / `DiagnosticSet` / `DiagnosticSeverity` 只在 `gpui_base::input`
//!    重导出（`src/input/mod.rs:76-79`），**没有**经 `gpui_component::input` 再导一次
//!    （`gpui-component-0.6.6/src/input/mod.rs:19-33` 的清单里没有它们），所以要用
//!    `gpui_kit::base::input::…`（`gpui_kit::base` = `gpui_base`，`gpui-kit-0.6.6/src/lib.rs:106`）；
//! 2. **只在 `CodeEditor` 模式下存在**：`diagnostics_mut()` 对别的 `LayoutMode` 返回 `None`
//!    （`base/mode.rs:321-326`）。`EditorState::new` 建的正是 `CodeEditor`
//!    （`state.rs:9230-9234`），所以编辑区的每个标签都有这个集合；
//! 3. ⚠️ **渲染被语法高亮器挡在前面**：`highlight_lines` 在没有 highlighter 时**提前返回**
//!    （`base/element.rs:1527-1537`），而那一段在算 `diagnostic_styles`（`:1602`）**之前** ——
//!    也就是"没上色 ⇒ 没波浪线"。Java 的高亮在本 crate 是齐的
//!    （`Cargo.toml` 的 `tree-sitter-java` + `buffer.rs::language_for_file` 给 `"java"`），
//!    所以这条约束当前成立；它一旦坏掉，波浪线会跟着一起消失（登记在已知边界里）。
//!
//! ## 列口径：**必须复用 `navigation::editor_position`**
//!
//! `DiagnosticSet::push` 把 `Diagnostic.range` 的 `Position.character` 当**字符列**用
//! （`diagnostics.rs:206-207` → `position_to_offset` → `rope_ext.rs:330-335` 的 `char_to_byte_idx`；
//! `EditorState::cursor_position()` 那一族也是同一个口径，`rope_ext.rs:337-343`），
//! 而 Core / JDT 给的是 **UTF-16 码元列**。两者只在"这一行没有非 BMP 字符"时相等 ——
//! 换算只有一条答案，就是跳转 / 补全用的那个 [`crate::navigation::editor_position`]
//! （`navigation.rs` 的模块文档列了三套列）。这里再写一遍就是第三个真相源。
//!
//! ## 取诊断的时机（服务端是**异步推送**，这是本模块最要紧的一条）
//!
//! 诊断到不了"请求 → 立刻有结果"的模型里：`didOpen` / `didChange` 之后服务端**稍后**才推
//! `publishDiagnostics`，而查询接口只回"最近一次快照"。更麻烦的是**空数组有两种含义** ——
//! "刚发布了一次清空"和"还没发布过"在查询口径下**完全不可区分**。
//!
//! 所以本模块的做法是：
//!
//! 1. [**同步**] 先把当前正文同步给服务端（[`JavaLanguageService::sync_document`]），
//!    否则服务端不会为新正文重算（首次 `didOpen`、之后 `didChange`）；
//! 2. [**有界重取**] 按 [`RETRY_BACKOFF_MS`] 的退避**最多查 5 次**，收手判据见 [`fetch`]：
//!    "非空**且**与上一次逐条相同"。没有"空就断定干净"这一步 —— 干净的文件的 0 条与
//!    "还没算完"的 0 条本来就分不开，退避上限是唯一有界又诚实的做法
//!    （总等待上限 7.75s，见 [`RETRY_BACKOFF_MS`]）；
//! 3. [**不缓存、不重放**] 每次都重新问服务端。**不能**把上一次的快照缓存下来按断言重放 ——
//!    上游 `DiagnosticSet` 在**每次正文变化**时都被 `reset`（`state.rs:3551-3553` /
//!    `:3891-3893` / `:3999-4001`），重放旧快照只会让波浪线回到已经修好的位置。
//!
//! ## 事务边界（谁在什么条件下重取、为什么不会漏）
//!
//! 触发点只有四个，都在 `editor_view.rs`（**都在 UI 线程上只排任务，重活全在后台**）：
//!
//! | 触发 | 时机 | 为什么不会漏 |
//! | --- | --- | --- |
//! | [`crate::EditorPane::open`] | 新开一个 Java buffer | 首次诊断的入口；`self.java` 还没登记时什么都不做，由下一条补 |
//! | [`crate::EditorPane::prepare_java`] | 外壳登记 Java 服务之后 | 补上"先开文件、后起服务"那一档（`open()` 那次是空转） |
//! | [`crate::EditorPane::on_input_change`] | 每一次 `InputEvent::Change` | 防抖窗口 [`CHANGE_DEBOUNCE`]；换掉上一个 `Task` 就是取消（gpui 的 `Task` drop 即取消） |
//! | [`crate::EditorPane::reload_buffer`] | 从磁盘重读 | 正文被整篇换掉，旧诊断的范围已经无意义 |
//!
//! 陈旧保护三层，缺一层就会出现"波浪线画在别的行"：
//!
//! 1. **代次**：`EditorPane` 里一个自增计数，每次排任务 +1 并记进 buffer；结果回来时不一致直接丢
//!    （同一份文件被排了两次时，只认新的那次）；
//! 2. **正文修订号**：排任务时记下 `Buffer::revision`，回前台时不相等就丢 ——
//!    服务端算的是**那时**的正文，行号列号对现在的正文已经没有意义
//!    （新的那一次刷新已经在路上，因为每次变化都会排一次）；
//! 3. **buffer 还在**：按路径找回 buffer，找不到就是标签已经关了，直接丢。
//!
//! 三者都过才写进 `EditorState`，并打一行 `S1_EDITOR_DIAGNOSTICS`
//! （`count` / `severity_max` / `attempts`），这样"界面上有没有波浪线"能从日志判定。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::BackgroundExecutor;
use gpui_kit::base::input::{Diagnostic, DiagnosticSet, DiagnosticSeverity};
use gpui_kit::component::input::{EditorState, Rope};
use lithe_gpui_java::{JavaDiagnostic, JavaLanguageService};

/// 同步正文之后的**重取退避**（毫秒，逐次等待）。
///
/// 为什么要退避而不是"查一次"：诊断是服务端异步推的，`sync_document` 一返回就查几乎必然
/// 拿到"还没发布"的空快照。为什么是**有界**的：它是一次后台任务的等待总量，
/// 无界等待会在会话卡死时留下一个永不结束的任务。5 次、总等待 7.75s
/// 覆盖了"JDT 解析一个文档并发布"的实际量级（语法 / 类型错误在 `didOpen` 之后就到，
/// 不需要等整个项目构建）；正常情况一两档就收手（收手判据见 [`fetch`]）。
///
/// 对"这个文件真的很干净"这一档，我们会老老实实等满 5 次再写下 0 条 ——
/// 那是"分不清空与没算完"的必然代价，如实登记。
const RETRY_BACKOFF_MS: [u64; 5] = [250, 500, 1_000, 2_000, 4_000];

/// 文档变化后的**防抖窗口**。
///
/// 打字会连发 `InputEvent::Change`（每个字符一次），而每一次都要同步正文 + 等退避 ——
/// 不合并的话，一个单词打完会有七八个后台任务排队抢 `JavaLanguageService` 的会话锁。
/// 400ms 是"停下打字才去问服务端"的量级（比补全的 120ms 长：诊断不需要即时感，
/// 而它比补全贵得多 —— 一次同步 + 最多 5 次查询）。
pub(crate) const CHANGE_DEBOUNCE: Duration = Duration::from_millis(400);

/// 一次取回的结论（进 `S1_EDITOR_DIAGNOSTICS` 诊断行）。
pub(crate) struct Fetched {
    /// 服务端给的最近一次快照（空 = 没有诊断，或还没发布过，见模块文档）。
    pub(crate) diagnostics: Vec<JavaDiagnostic>,
    /// 真的问了几次（第一次就拿到非空时是 1，问满 [`RETRY_BACKOFF_MS`] 时是 5）。
    pub(crate) attempts: usize,
    /// 同步正文失败的原因；`None` = 同步成功。
    pub(crate) sync_error: Option<String>,
    /// 从进入后台任务到出结论的墙钟毫秒（含防抖）。
    pub(crate) elapsed_ms: u128,
}

/// 后台取一次诊断：**先同步正文**（让服务端为新正文重算），**再有界重取**快照。
///
/// `debounce` 是进任务之后的第一个等待：打开文件时给 `Duration::ZERO`（没有连打要合并），
/// 正文变化时给 [`CHANGE_DEBOUNCE`]（合并连打）。理由见模块文档的"取诊断的时机"。
///
/// **收手判据**（这是本函数唯一不好一眼看懂的地方）：拿到"非空且与上一次查询逐条相同"的快照。
/// - 只判"非空"不够：撤消 / 连续两次改动会让服务端**连发两次**发布，第一次算的还是上一个版本，
///   只认第一次非空就会把略旧的条数留在界面上（实测：撤消后先推 2 条、随后才推 3 条）；
/// - "两次相同"可以收手，是因为 `publishDiagnostics` 推的是"这个文件的完整快照"，
///   服务端不再改动时两次查询必然逐条相等（`JavaDiagnostic` 实现了 `PartialEq`）；
/// - **空快照永远不收手**：那是"还没发布过"（与"真的很干净"不可区分），只能等预算用完。
///
/// 代价：非空时至少多查一次（多等一档退避）。它发生在后台线程上，UI 不受影响。
///
/// 同步失败**不外抛**：JDTLS 起不来是"能力降级"而不是崩溃（与 `navigation` 的
/// `S1_JAVA_UNAVAILABLE` 同一口径），结果里带 `sync_error` 让调用方打一行证据即可。
pub(crate) async fn fetch(
    service: Arc<JavaLanguageService>,
    path: PathBuf,
    text: String,
    executor: BackgroundExecutor,
    debounce: Duration,
) -> Fetched {
    let started = Instant::now();
    if !debounce.is_zero() {
        executor.timer(debounce).await;
    }

    let sync_error = service.sync_document(&path, &text).err();

    let mut attempts = 0;
    let mut diagnostics: Vec<JavaDiagnostic> = Vec::new();
    let mut previous: Option<Vec<JavaDiagnostic>> = None;
    for delay in RETRY_BACKOFF_MS {
        executor.timer(Duration::from_millis(delay)).await;
        attempts += 1;
        let snapshot = service.diagnostics(&path);

        // 收手判据：**已拿到非空，且与上一次查询逐条相同** ⇒ 服务端不再变，这就是终态。
        //
        // 为什么"非空"还不够：撤消 / 连续两次改动会让服务端**连发两次**发布
        // （第一次算的还是上一个版本），只认第一次非空会把略旧的条数留在界面上。
        // 为什么"相同"就可以收手：`publishDiagnostics` 推的是"这个文件的完整快照"，
        // 服务端不再改动时两次查询必然逐条相等（`JavaDiagnostic` 是 `PartialEq`）。
        // 空快照**永远不收手**：那是"还没发布过"，只能靠预算兜底（见模块文档）。
        let settled = !snapshot.is_empty() && previous.as_ref() == Some(&snapshot);
        previous = Some(snapshot.clone());
        diagnostics = snapshot;
        if settled {
            break;
        }
    }

    Fetched {
        diagnostics,
        attempts,
        sync_error,
        elapsed_ms: started.elapsed().as_millis(),
    }
}

/// 把一份诊断**整份**写进编辑器的诊断集合，返回写入的条数。
///
/// 调用方负责在正文已经对得上时调用（见模块文档的陈旧保护）。
///
/// ⚠️ **这个函数不 `notify`**：`cx` 在调用方手里（`EditorPane::apply_diagnostics`），
/// 写完之后由它 `cx.notify()` —— `DiagnosticSet` 只在绘制阶段被读，
/// 不通知的话波浪线要等到下一次无关的重绘才出现。
pub(crate) fn apply(state: &mut EditorState, diagnostics: &[JavaDiagnostic]) -> usize {
    // `Rope` 的克隆很便宜（持久化数据结构），而 `diagnostics_mut()` 要独占借用整个 state，
    // 所以先把正文克隆出来做列换算。
    let text = state.text().clone();
    let Some(set) = state.diagnostics_mut() else {
        // 非 `CodeEditor` 模式没有诊断集合（`base/mode.rs:321-326`）：静默返回 0，
        // 由调用方那行诊断记下 count=0 —— 不 panic，因为"模式不对"是可恢复的配置问题。
        return 0;
    };
    write(set, &text, diagnostics)
}

/// [`apply`] 的纯数据一半：整份替换 + 列换算。拆出来是为了能**直接单测**——
/// 它只需要一个 `Rope` 和一个 `DiagnosticSet`，不需要窗口与 `App`。
fn write(set: &mut DiagnosticSet, text: &Rope, diagnostics: &[JavaDiagnostic]) -> usize {
    // 先清空再全量写：服务端推的本来就是"这个文件的完整快照"（一次 `publishDiagnostics`
    // 给全部），增量合并只会让已经修好的错误留在界面上。
    set.clear();
    for diagnostic in diagnostics {
        set.push(to_upstream(diagnostic, text));
    }
    diagnostics.len()
}

/// 一条 JDT 诊断 → 上游的 `Diagnostic`。
///
/// 三件事：
///
/// 1. ⚠️ **两个端点都要把 UTF-16 码元列换成编辑器的字符列**（理由见模块文档的"列口径"），
///    换算复用 [`crate::navigation::editor_position`]。行号是 0 基，两边一致；
///    越界的行由 `RopeExt::slice_line` 夹住（`rope_ext.rs:278-293`）而不会 panic；
/// 2. **`severity`**：[`severity`] 负责 LSP 数值 → 上游枚举；
/// 3. **`source` / `code`**：界面暂时只画波浪线、气泡里显示 `message`，但这两个是服务端给的
///    事实，丢掉就再也拿不回来了。传 `code` 时**不传** `code_description`：
///    JDT 的 `codeDescription.href` 是 URI，点击行为归 UI（当前没有），
///    先不塞一个无人消费的字段。
///
/// **不映射 `tags`**：上游只在 `entry.severity` 与 `entry.range` 上做决定
/// （`base/element.rs:1602-1610`），`tags` 是纯数据、不参与渲染，写进去就是死字段。
/// 已知边界里登记了"`Unnecessary` / `Deprecated` 的淡色不会出现"。
fn to_upstream(diagnostic: &JavaDiagnostic, text: &Rope) -> Diagnostic {
    let start = crate::navigation::editor_position(
        text,
        diagnostic.range.start.line as usize,
        diagnostic.range.start.utf16_column as usize,
    );
    let end = crate::navigation::editor_position(
        text,
        diagnostic.range.end.line as usize,
        diagnostic.range.end.utf16_column as usize,
    );

    let mut mapped = Diagnostic::new(start..end, diagnostic.message.clone())
        .with_severity(severity(diagnostic.severity));
    if let Some(source) = &diagnostic.source {
        mapped = mapped.with_source(source.clone());
    }
    if let Some(code) = &diagnostic.code {
        mapped = mapped.with_code(code.clone());
    }
    mapped
}

/// LSP 的 `DiagnosticSeverity` 数值 → 上游的严重度。
///
/// 数值口径由 LSP 定死：`1=Error 2=Warning 3=Information 4=Hint`
/// （`lsp-types-0.97.0/src/lib.rs:447-458`）。
///
/// **缺字段与未知值都给 `Info`**：上游自己的 `From<lsp_types::Diagnostic>` 就是这么做的
/// （`diagnostics.rs:58-61` 的 `unwrap_or(DiagnosticSeverity::Info)`），我们照抄它的口径，
/// 而不是用上游 `DiagnosticSeverity::default()` 的 `Hint` —— Hint 的颜色在主题里几乎看不见
/// （`input.rs:508` 的 `style.status.hint`），把"服务端没说严重度"画成几乎不可见，
/// 等于把一条真诊断藏起来。
fn severity(value: Option<i64>) -> DiagnosticSeverity {
    match value {
        Some(1) => DiagnosticSeverity::Error,
        Some(2) => DiagnosticSeverity::Warning,
        Some(3) => DiagnosticSeverity::Info,
        Some(4) => DiagnosticSeverity::Hint,
        _ => DiagnosticSeverity::Info,
    }
}

/// `S1_EDITOR_DIAGNOSTICS` 里的 `severity_max=`：这一份快照里**最严重**的那一档。
///
/// LSP 的数值越小越严重，所以取最小值；空快照给 `none`。缺字段 / 未知值按 [`severity`]
/// 的口径算 `info`（两处必须一致，否则日志说 `info` 而画出来的是别的颜色）。
pub(crate) fn severity_max(diagnostics: &[JavaDiagnostic]) -> &'static str {
    // 5 = 比 LSP 的四档都"轻"，空快照与"只有未知严重度"因此不会混淆：前者 `none`。
    let mut rank = 5i64;
    for diagnostic in diagnostics {
        rank = rank.min(match diagnostic.severity {
            Some(value @ 1..=4) => value,
            _ => 3,
        });
    }
    match rank {
        1 => "error",
        2 => "warning",
        3 => "info",
        4 => "hint",
        _ => "none",
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::component::input::Position;
    use lithe_gpui_java::{JavaDiagnostic, JavaDiagnosticRange, JavaPosition};

    use super::*;

    /// 造一条诊断（UTF-16 列口径，与 Core 给的一致）。
    fn diagnostic(
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
        severity: Option<i64>,
    ) -> JavaDiagnostic {
        JavaDiagnostic {
            range: JavaDiagnosticRange {
                start: JavaPosition {
                    line: start_line,
                    utf16_column: start_column,
                },
                end: JavaPosition {
                    line: end_line,
                    utf16_column: end_column,
                },
            },
            severity,
            message: "Type mismatch: cannot convert from String to int".to_string(),
            source: Some("Java".to_string()),
            code: Some("TypeMismatch".to_string()),
            tags: Vec::new(),
        }
    }

    /// 严重度映射：LSP 的四个数值各归各位，**缺字段与未知值都落到 `Info`**
    /// （不是上游 `default()` 的 `Hint` —— 那个在主题里几乎看不见，见 `severity` 的文档）。
    #[test]
    fn severity_follows_lsp_numbers_and_defaults_to_info() {
        assert_eq!(severity(Some(1)), DiagnosticSeverity::Error);
        assert_eq!(severity(Some(2)), DiagnosticSeverity::Warning);
        assert_eq!(severity(Some(3)), DiagnosticSeverity::Info);
        assert_eq!(severity(Some(4)), DiagnosticSeverity::Hint);
        assert_eq!(severity(None), DiagnosticSeverity::Info);
        assert_eq!(severity(Some(0)), DiagnosticSeverity::Info);
        assert_eq!(severity(Some(99)), DiagnosticSeverity::Info);
    }

    /// `severity_max=` 取的是**最严重**的那一档（LSP 数值最小），空快照是 `none`。
    #[test]
    fn severity_max_reports_the_most_severe_entry() {
        assert_eq!(severity_max(&[]), "none");
        assert_eq!(severity_max(&[diagnostic(0, 0, 0, 1, Some(2))]), "warning");
        assert_eq!(
            severity_max(&[
                diagnostic(0, 0, 0, 1, Some(3)),
                diagnostic(1, 0, 1, 1, Some(1)),
                diagnostic(2, 0, 2, 1, Some(2)),
            ]),
            "error",
            "一档 Error 就该报 error，与顺序无关"
        );
        // 缺字段按 `info` 算（与 `severity` 一致），所以它比 Hint 严重。
        assert_eq!(
            severity_max(&[
                diagnostic(0, 0, 0, 1, None),
                diagnostic(1, 0, 1, 1, Some(4)),
            ]),
            "info"
        );
    }

    /// 映射到 `Diagnostic` 的字段：消息 / 严重度 / 来源 / 错误码都落到上游类型上。
    #[test]
    fn mapping_keeps_message_severity_source_and_code() {
        let rope = Rope::from_str("int x = \"nope\";\n");
        let mapped = to_upstream(&diagnostic(0, 8, 0, 14, Some(1)), &rope);

        assert_eq!(mapped.severity, DiagnosticSeverity::Error);
        assert_eq!(
            mapped.message.as_str(),
            "Type mismatch: cannot convert from String to int"
        );
        assert_eq!(mapped.source.as_ref().map(|s| s.as_str()), Some("Java"));
        assert_eq!(
            mapped.code.as_ref().map(|s| s.as_str()),
            Some("TypeMismatch")
        );
        assert_eq!(mapped.range.start, Position::new(0, 8));
        assert_eq!(mapped.range.end, Position::new(0, 14));
    }

    /// ⚠️ 这一条是本模块最容易错的地方：Core / JDT 的列是 **UTF-16 码元**，
    /// 而上游 `DiagnosticSet::push` 把它当**字符列**（`char_to_byte_idx`）。
    ///
    /// 断言走**整条链路**（`to_upstream` → `DiagnosticSet::push` → `DiagnosticEntry::range`
    /// 的**字节**范围），比只断言中间的 `Position` 更接近"波浪线画在哪几个字节上"：
    /// 断言 `Position` 只能证明换算函数被调用了，断言字节范围才能证明落在原地。
    ///
    /// 样本特意在行内含一个非 BMP 字符（`🙂`，1 字符 = 2 个 UTF-16 码元）：
    /// 不换算的话整条波浪线会左移一列，纯 ASCII 样本**永远测不出来**。
    #[test]
    fn utf16_columns_are_converted_to_character_columns_for_the_underline() {
        // 行 0 = `// 🙂 int y`。
        // UTF-16 码元：`/`=0 `/`=1 ` `=2 `🙂`=3..4 ` `=5 `i`=6 `n`=7 `t`=8 ` `=9 `y`=10
        // （`🙂` 占两个码元，所以 `y` 在 **10**）；字符列：`y` 是第 **9** 个字符。
        // 字节：`// `=3 + `🙂`=4 + ` `=1 + `int `=4 ⇒ `y` 从第 **12** 字节起。
        let text = Rope::from_str("// 🙂 int y\n");
        let mapped = to_upstream(&diagnostic(0, 10, 0, 11, Some(1)), &text);
        assert_eq!(
            mapped.range.start,
            Position::new(0, 9),
            "UTF-16 列 10 → 字符列 9"
        );

        let mut set = DiagnosticSet::new(&text);
        set.push(mapped);
        assert_eq!(set.len(), 1);
        let entry = set.for_offset(12).expect("第 12 字节应当落在诊断范围内");
        assert_eq!(
            entry.range,
            12..13,
            "波浪线必须落在 `y` 这一个字节上（把 UTF-16 当字符解会落到别的字节）"
        );
    }

    /// 纯 ASCII 的对照：两套列相等，所以范围必须逐字节相等 ——
    /// 它证明上一条的差异确实来自 `🙂`，而不是换算函数一律偏移。
    #[test]
    fn ascii_columns_are_unchanged() {
        let text = Rope::from_str("// int y\n");
        let mut set = DiagnosticSet::new(&text);
        set.push(to_upstream(&diagnostic(0, 7, 0, 8, Some(1)), &text));
        let entry = set.for_offset(7).expect("第 7 字节应当落在诊断范围内");
        assert_eq!(entry.range, 7..8);
    }

    /// 多行文本里**第二行**的端点也按各行切片换算（`editor_position` 是逐行的，
    /// 用第一行的 `RopeSlice` 换算第二行的列会整体偏移）。
    #[test]
    fn ranges_on_later_lines_use_that_line_slice() {
        // 行 0 = `int a = 1;`（11 字节，含换行）；行 1 = `// 🙂 oops`。
        // 行 1 内：字节 `/`=0 `/`=1 ` `=2 `🙂`=3..6 ` `=7 `o`=8 `o`=9 `p`=10 `s`=11；
        // UTF-16 码元：`🙂` 占 3..4，所以 `o`=6 `o`=7 `p`=8 `s`=9。
        // 取 `oops` ⇒ UTF-16 6..10 → 字符 6..9 → 行内字节 8..12 → 全文 19..23。
        let text = Rope::from_str("int a = 1;\n// 🙂 oops\nint b = 2;\n");
        let mut set = DiagnosticSet::new(&text);
        set.push(to_upstream(&diagnostic(1, 6, 1, 10, Some(2)), &text));

        let entry = set.for_offset(19).expect("诊断必须落在行 1 的 oops 上");
        assert_eq!(entry.range, 19..23);
        assert_eq!(
            &text.to_string()[entry.range.clone()],
            "oops",
            "字节范围必须正好覆盖 oops（差一列就会切到别的字符）"
        );
    }

    /// 越界的行列**不 panic**（JDT 与服务端版本错位时可能出现）：上游把越界的行夹到文本末尾
    /// （`rope_ext.rs:321-324` 的 `point_to_offset`），这里只要求"不炸 + 仍然入集合"。
    #[test]
    fn out_of_range_positions_do_not_panic() {
        let text = Rope::from_str("int x = 1;\n");
        let mut set = DiagnosticSet::new(&text);
        set.push(to_upstream(&diagnostic(99, 99, 99, 100, Some(1)), &text));
        assert_eq!(set.len(), 1, "越界的诊断仍然入集合（范围落在文本末尾）");
    }

    /// `write` 是**整份替换**语义：第二次写一份更少的诊断时，旧的那些必须消失
    /// （增量合并会让已经修好的错误留在界面上）。
    #[test]
    fn writing_a_smaller_snapshot_drops_the_previous_entries() {
        let text = Rope::from_str("int a = 1;\nint b = 2;\n");
        let mut set = DiagnosticSet::new(&text);
        let written = write(
            &mut set,
            &text,
            &[
                diagnostic(0, 0, 0, 3, Some(1)),
                diagnostic(1, 0, 1, 3, Some(1)),
            ],
        );
        assert_eq!(written, 2);
        assert_eq!(set.len(), 2);

        let written = write(&mut set, &text, &[diagnostic(0, 0, 0, 3, Some(2))]);
        assert_eq!(written, 1);
        assert_eq!(set.len(), 1, "旧的那一条必须被整份替换掉");
        assert_eq!(
            set.for_offset(0).expect("新的那条还在").severity,
            DiagnosticSeverity::Warning
        );
        assert!(
            set.for_offset(11).is_none(),
            "第二行（字节 11 起）的诊断必须已经被清掉"
        );

        // 空快照 = 服务端清空：界面上一条都不剩。
        write(&mut set, &text, &[]);
        assert_eq!(set.len(), 0, "空快照必须把波浪线全部清掉");
    }

    /// 上游 `DiagnosticSet` 的 `reset` 语义（正文一变就整份清空）是**我们不做断言重放的依据**：
    /// 这一条把它的行为钉住，将来上游改成"增量保留"时这里会红，提醒重新评估重取策略。
    #[test]
    fn upstream_reset_clears_diagnostics_on_text_change() {
        let text = Rope::from_str("int a = 1;\n");
        let mut set = DiagnosticSet::new(&text);
        set.push(Diagnostic::new(
            Position::new(0, 0)..Position::new(0, 3),
            "boom",
        ));
        assert_eq!(set.len(), 1);

        set.reset(&Rope::from_str("int a = 2;\n"));
        assert_eq!(
            set.len(),
            0,
            "上游在每次正文变化时 reset：缓存旧快照再重放会让波浪线回到旧位置"
        );
    }
}
