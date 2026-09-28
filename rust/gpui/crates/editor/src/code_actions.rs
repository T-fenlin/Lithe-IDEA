//! Java 文件的**快速修复**（Code Action / `Ctrl+.`）：把 JDT 的 quick fix 接到上游编辑器
//! 自带的 code action 菜单上，并**由我们把编辑落进 buffer**。
//!
//! ## 上游有没有入口：**有**，而且菜单 / 键位都是现成的
//!
//! | 部件 | 位置（`gpui-base-0.6.6/`） | 说明 |
//! | --- | --- | --- |
//! | trait | `src/input/editor/lsp/code_actions.rs:8-34` | `CodeActionProvider::{id, code_actions, perform_code_action}` |
//! | 挂载点 | `src/input/editor/lsp/mod.rs:43` | `Lsp.code_action_providers: Vec<Rc<dyn CodeActionProvider>>`，经 `EditorState::lsp_mut()` 赋值（同文件 `:185`） |
//! | 触发（键盘） | `src/input/base/state.rs:292-294` | 上游自己绑好了：macOS `cmd-.`、其它平台 **`ctrl-.`** → `ToggleCodeActions`（上下文 `Input`，`state.rs:129`） |
//! | 触发（右键菜单） | `gpui-component-0.6.6/src/input/input.rs:565-569` | 编辑器右键菜单里的 `Show Code Actions`，同一个 action；没装 provider 时那一项是**禁用**的（判据 `has_code_actions()` = `!code_action_providers.is_empty()`） |
//! | 菜单 | `gpui-component-0.6.6/src/input/popovers/code_action_menu.rs` | 浮层、`↑`/`↓`、`Enter`、`Esc` 全归上游；它只渲染 `action.title` |
//! | 选中回调 | 同文件 `:188-202` → `src/input/editor/lsp/code_actions.rs:111-132` | 菜单点了某项就调 **我们**的 `perform_code_action` |
//!
//! ⚠️ **所以 `Alt+Enter` 不可用是上游的现状，不是本模块的缺口**：`gpui-base` 0.6.6 全量
//! grep 只有 `cmd-.` / `ctrl-.` 两个绑定，没有 `alt-enter`。按"不自己造键位"的约定，
//! 本模块**不新增**任何按键；要用 `Alt+Enter` 得改上游或在外壳层绑（都不在本批范围内）。
//!
//! ## ⚠️ 落盘**必须异步**：`perform_code_action` 的调用栈上已经借走了这个 entity
//!
//! 这一条是实测踩出来的（第一次实机点菜单直接把进程打崩了）：
//!
//! ```text
//! thread 'main' panicked at gpui-pre-0.3.6/src/app/entity_map.rs:164:36:
//! cannot update gpui_base::input::state::InputBaseState<EditorMode> while it is already being updated
//! ```
//!
//! 原因在上游的调用形状里（`gpui-component-0.6.6/src/input/popovers/code_action_menu.rs:188-202`）：
//!
//! ```ignore
//! state.update_in(cx, |state, window, cx| { state.perform_code_action(&item, window, cx); })
//! ```
//!
//! 也就是说 `perform_code_action` 是在**另一个**对同一个 `EditorState` 的可变借用里被调的
//! —— 这时再 `state.update(..)` / `update_in(..)` 就撞上 gpui 的重入检查并 panic。
//! 上游之所以给 trait 方法返回 `Task`，就是为了这条路：**把落盘放进返回的 `Task`**，
//! 上游会 `cx.spawn_in(..).detach()` 它（`src/input/editor/lsp/code_actions.rs:126-131`），
//! 那时同步借用已经释放，`AsyncApp::update_window(handle)` 进得去。
//!
//! 另外**不能**用 `App::with_window` 重新找窗口：它按"实体最近的窗口"找，窗口已在更新栈上时
//! 直接返回 `None`（`app.rs:1962-1976`）。用 `Window::window_handle()` 记下**被调时那个窗口**
//! 才是对的。
//!
//! ## 最关键的一条：**上游完全不处理 `edit`**，落盘必须我们做
//!
//! 上游把 `CodeAction` 当成"标题 + 我们自己的数据"：
//!
//! - `CodeActionMenu` 只读 `item.action.title`（`code_action_menu.rs:90`）；
//! - `perform_code_action` 是 trait 方法，上游**只有转发**（`code_actions.rs:126`），
//!   没有任何应用 `edit` / `additionalTextEdits` 的代码 —— 全量 grep
//!   `additional_text_edits` 在 `gpui-base` / `gpui-component` 里**零命中**，
//!   `apply_lsp_edits` 虽然存在（`src/input/editor/lsp/mod.rs:115-128`）却**没有任何调用点**；
//! - `handle_code_action_trigger` 甚至不检查代次（`code_actions.rs:62-108`），
//!   所以陈旧保护也在本模块（与 [`crate::completion`] 同一口径）。
//!
//! 于是本模块做两件事：把 JDT 的结果换成 `lsp_types::CodeAction` 交给上游菜单，
//! 并在用户选中时**用上游的 [`EditorState::apply_lsp_edits`] 把编辑写进 buffer**。
//!
//! 为什么用 `apply_lsp_edits` 而不是 Core 的 `lsp.applyTextEdits`（那是"文本进、文本出"的
//! 纯函数）：编辑器里的正文归 `EditorState` 的 rope 所有，走它自己的替换路径才会
//! ①标脏（`InputEvent::Change` → [`crate::editor_view`] 的 `on_input_change`）、
//! ②进撤销栈、③触发诊断重取。用"整篇换正文"（`set_value`）会关掉事件发射
//! （`state.rs:904-907` 的 `emit_events = false`），改完既不算脏也不重取诊断。
//!
//! ## 列口径：`edit` 保持 **UTF-16**，落地时才换算
//!
//! JDT 的编辑范围是 UTF-16 码元列（LSP 口径），而上游 `apply_lsp_edits` 走
//! `position_to_offset` → `char_to_byte_idx`，把 `Position.character` 当**字符列**
//! （`rope_ext.rs:330-343`）。两者只在"行内没有非 BMP 字符"时相等，所以**必须**换算，
//! 唯一答案是 [`crate::navigation::editor_position`]（三套列的坑见 `navigation.rs` 的模块文档）。
//!
//! 换算刻意放在 `perform_code_action`（落地那一刻）而不是列菜单那一刻：
//! `lsp_types::CodeAction` 是一个纯值，拿不到 rope；而且**用它自己的 rope** 才算得对
//! （菜单打开到选中之间正文可能已经变了）。为此 `edit` 里存的是 **JDT 原样的 UTF-16 范围**，
//! 那本来就是 LSP 的正确含义 —— 不是"半个转换"，而是"转换推迟到唯一能算准的地方"。
//!
//! ## 数据从哪来：[`JavaLanguageService::code_actions`]
//!
//! 与跳转 / 补全同一条口径：JDTLS 会话按需启动（`ensure_session`），
//! **先同步正文再请求**（顺序的理由写在那个方法上）。Java 侧已经把"落不了地的 action"
//! 过滤掉了 —— 带 `command` 的、以及只落在别的文件上的（`Create class 'List<T>'` 要新建文件）——
//! 所以这里看到几条就一定是几条能用的。

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use gpui_kit::component::input::{CodeActionProvider, EditorState};
use gpui_kit::{App, AppContext as _, Entity, SharedString, Task, Window};
use lithe_gpui_java::{JavaCodeAction, JavaLanguageService, JavaPosition};
use lsp_types::{CodeAction, CodeActionKind, Position, TextEdit, Uri, WorkspaceEdit};

/// 把快速修复装到一个 `EditorState` 上（`None` 时**什么都不做**）。
///
/// 由 [`crate::editor_view`] 在两个位置调用：`open()`（新开的 Java buffer，此时服务可能
/// 还没登记）与 `prepare_java()`（服务就绪后给**已经打开的** Java buffer 补装）。
///
/// ⚠️ **只有拿到服务句柄才装**（调用方负责判）：与 [`crate::completion::install`] 不同，
/// 快速修复**没有兜底数据源**（Core 里没有无进程的 code action 命令），装一个永远返回空
/// 的 provider 只会把右键菜单那一项变成"可用但点了没反应"（`has_code_actions()` 只看
/// provider 存不存在）。没装 provider 时上游会把那一项画成**禁用**，那是更诚实的表达。
///
/// 用 `vec![provider]` 而不是 `push`：`prepare_java()` 会给已经装过的 buffer 补装一次，
/// 叠加会让同一份 action 在菜单里出现两次。
pub(crate) fn install(state: &mut EditorState, provider: Option<Rc<dyn CodeActionProvider>>) {
    let Some(provider) = provider else {
        return;
    };
    state.lsp_mut().code_action_providers = vec![provider];
}

/// Java 的快速修复数据源。
pub(crate) struct JavaCodeActionProvider {
    /// JDTLS 门面（`Arc`：请求要带进后台任务）。
    service: Arc<JavaLanguageService>,
    /// 这个 buffer 对应的磁盘路径（`jdt://` 虚拟源码不装本 provider，见 [`crate::editor_view`]）。
    file_path: PathBuf,
    /// 请求代次（跨线程共享：`code_actions()` 返回的 `Task` 在后台线程上跑）。
    generation: Arc<AtomicU64>,
}

impl JavaCodeActionProvider {
    /// 建一个 provider。调用方保证 `service` 是活的 Java 语言服务（理由见 [`install`]）。
    pub(crate) fn new(
        service: Arc<JavaLanguageService>,
        file_path: PathBuf,
    ) -> Rc<dyn CodeActionProvider> {
        Rc::new(Self {
            service,
            file_path,
            generation: Arc::new(AtomicU64::new(0)),
        })
    }

    /// 这个 buffer 的 `file://` URI：既是 `edit.changes` 的键，也是"这条编辑是不是我的"的判据。
    ///
    /// 与 Java 侧 `file_uri` 同一个换算（都走 `url` 的 `from_file_path` —— 那条实现现在是
    /// `pub`，两侧共用一份，避免各拼一次导致键对不上），所以两侧算出来的键逐字节相同。
    /// 路径不是绝对路径（理论上不该发生）或 URI 解析不了时返回 `None`：
    /// 宁可这次不落地，也不要造一个对不上的键。
    fn file_uri(&self) -> Option<Uri> {
        let uri = lithe_gpui_java::file_uri(&self.file_path).ok()?;
        Uri::from_str(&uri).ok()
    }
}

impl CodeActionProvider for JavaCodeActionProvider {
    fn id(&self) -> SharedString {
        "lithe-java-code-actions".into()
    }

    /// 取光标 / 选区处的快速修复（`textDocument/codeAction`）。
    ///
    /// `range` 是**字节**偏移（上游 `selected_range()` 的契约，`state.rs:2824-2833`），
    /// 换算成 Core 的 `(0 基行, UTF-16 列)` 只有 [`crate::navigation::core_position`] 一条路。
    ///
    /// 代次校验的理由与 [`crate::completion`] 相同：上游接受**任何一次**请求的结果，
    /// 而"连按两次 `Ctrl+.`"会让两个请求同时在飞；不是最新的那个返回**空列表**
    /// （上游拿到空列表会把菜单关掉而不是弹出旧候选）。
    fn code_actions(
        &self,
        state: Entity<EditorState>,
        range: std::ops::Range<usize>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui_kit::Result<Vec<CodeAction>>> {
        let text = state.read(cx).text().clone();
        let body = text.to_string();
        // 两端都按字节偏移换算；`core_position` 自己会处理越界（它只读 rope 的行信息）。
        let (start_line, start_column) = crate::navigation::core_position(&text, range.start);
        let (end_line, end_column) = crate::navigation::core_position(&text, range.end);
        let start = JavaPosition {
            line: start_line as u32,
            utf16_column: start_column as u32,
        };
        let end = JavaPosition {
            line: end_line as u32,
            utf16_column: end_column as u32,
        };

        let Some(uri) = self.file_uri() else {
            let file = self.file_path.display().to_string();
            println!("S1_EDITOR_CODE_ACTION file={file} actions=0 reason=no-file-uri");
            return Task::ready(Ok(Vec::new()));
        };
        let file = self.file_path.display().to_string();
        let file_path = self.file_path.clone();
        let service = self.service.clone();
        let mine = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = self.generation.clone();

        cx.background_spawn(async move {
            let started = Instant::now();
            let actions = match service.code_actions(&file_path, &body, start, end) {
                Ok(actions) => actions,
                // 服务不可用 / 超时：**不外抛**（上游会把 `Err` 静默吃掉，
                // `code_actions.rs:73` 的 `.ok()`），所以这里自己留一行证据再返回空。
                Err(error) => {
                    println!("S1_EDITOR_CODE_ACTION file={file} actions=0 result=failed error={error}");
                    return Ok(Vec::new());
                }
            };

            if generation.load(Ordering::SeqCst) != mine {
                println!(
                    "S1_EDITOR_CODE_ACTION file={file} actions={} result=stale reason=superseded",
                    actions.len()
                );
                return Ok(Vec::new());
            }

            let kinds = kind_summary(&actions);
            let mapped: Vec<CodeAction> = actions
                .into_iter()
                .map(|action| to_lsp_action(action, &uri))
                .collect();
            println!(
                "S1_EDITOR_CODE_ACTION file={file} actions={} kind={kinds} line={} col={} ms={}",
                mapped.len(),
                start.line,
                start.utf16_column,
                started.elapsed().as_millis()
            );
            Ok(mapped)
        })
    }

    /// 用户从菜单里选中一项：**由我们把 `edit` 落进 buffer**（理由见模块文档）。
    ///
    /// `push_to_history` 上游恒传 `true`（`code_actions.rs:126`）；这里不单独处理它 ——
    /// 上游的替换路径本来就会记撤销（`state.rs:3457-3490` 的 `replace_text_in_ranges`），
    /// 而"不记历史"需要另一条 API，本批不需要（菜单是唯一入口，永远传 `true`）。
    fn perform_code_action(
        &self,
        state: Entity<EditorState>,
        action: CodeAction,
        _push_to_history: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui_kit::Result<()>> {
        let file = self.file_path.display().to_string();
        let title = action.title.clone();
        let Some(uri) = self.file_uri() else {
            println!("S1_EDITOR_CODE_ACTION file={file} result=skipped reason=no-file-uri title={title:?}");
            return Task::ready(Ok(()));
        };
        let Some(edits) = action
            .edit
            .as_ref()
            .and_then(|edit| edit.changes.as_ref())
            .and_then(|changes| changes.get(&uri))
        else {
            // 本模块只会把"本文件的编辑"放进 `edit`，走到这里说明上游换了形状或有人塞了别的
            // action —— 留证据，不 panic。
            println!(
                "S1_EDITOR_CODE_ACTION file={file} result=skipped reason=no-edits-for-this-file title={title:?}"
            );
            return Task::ready(Ok(()));
        };
        // 所有权取出来，进异步块（闭包要 `'static` 的捕获）。
        let edits = edits.clone();
        let handle = window.window_handle();

        // ⚠️ **不能在本次同步调用里碰这个 entity**（实测会 panic，见模块文档的"为什么异步落盘"）。
        // 上游是在 `CodeActionMenu::select_item` 的 `state.update_in(cx, |state, window, cx|
        // state.perform_code_action(..))` 里调我们的（`code_action_menu.rs:188-202`）——
        // 也就是说调用栈上**已经**有一份对同一个 `EditorState` 的可变借用，
        // 再 `state.update(..)` 会撞上 gpui 的重入检查：
        // `entity_map.rs:164` 的 `cannot update … while it is already being updated`（实测崩过一次）。
        // 所以编辑的落地放进**返回的 `Task`**：上游拿到它之后是 `cx.spawn_in(..).detach()`
        // （`code_actions.rs:128-131`），那时同步借用已经释放，`update_window` 进得去。
        //
        // 为什么用 `update_window(handle)` 而不是 `App::with_window`：后者按"实体最近的窗口"
        // 找，且窗口已在更新栈上时返回 `None`（`app.rs:1962-1976`）——
        // 这里要的是**我们被调时那个窗口**，`Window::window_handle()` 给的正是它。
        cx.spawn(async move |cx| {
            let outcome = cx.update_window(handle, |_, window, cx| {
                state.update(cx, |state, cx| {
                    // ⚠️ **用这一刻的 rope** 做换算（见模块文档的列口径）。
                    let text = state.text().clone();
                    let converted = to_editor_edits(&text, &edits);
                    let count = converted.len();
                    state.apply_lsp_edits(&converted, window, cx);
                    count
                })
            });

            match outcome {
                Ok(count) => println!(
                    "S1_EDITOR_CODE_ACTION file={file} result=applied edits={count} title={title:?}"
                ),
                Err(error) => println!(
                    "S1_EDITOR_CODE_ACTION file={file} result=skipped reason=window-gone error={error}"
                ),
            }
            Ok(())
        })
    }
}

/// 把一条action 里的编辑转成**编辑器口径**（`apply_lsp_edits` 要的形状）。
///
/// 两件事，都必须做对：
///
/// 1. ⚠️ **两个端点都要把 UTF-16 码元列换成编辑器的字符列**（理由见模块文档的列口径），
///    换算复用 [`crate::navigation::editor_position`] —— 与补全 / 诊断同一个答案；
/// 2. **按位置倒序**：`apply_lsp_edits` 是**顺序**应用每一条（`lsp/mod.rs:115-128`），
///    先应用靠后的，靠前那条的范围才不会被前一次替换挪走。Java 侧已经排过一次，
///    这里再排一次是因为 `perform` 不该依赖"上游没换过顺序"。
///
/// 拆成纯函数是为了能**直接单测**（emoji 那一档在 `apply_lsp_edits` 里测不到：
/// 它要窗口与 `Context`）。
fn to_editor_edits(text: &gpui_kit::component::input::Rope, edits: &[TextEdit]) -> Vec<TextEdit> {
    let mut converted: Vec<TextEdit> = edits
        .iter()
        .map(|edit| TextEdit {
            range: lsp_types::Range {
                start: crate::navigation::editor_position(
                    text,
                    edit.range.start.line as usize,
                    edit.range.start.character as usize,
                ),
                end: crate::navigation::editor_position(
                    text,
                    edit.range.end.line as usize,
                    edit.range.end.character as usize,
                ),
            },
            new_text: edit.new_text.clone(),
        })
        .collect();
    converted.sort_by_key(|edit| {
        std::cmp::Reverse((edit.range.start.line, edit.range.start.character))
    });
    converted
}

/// 本 crate 的结构化 action → 上游菜单用的 `lsp_types::CodeAction`。
///
/// 三件事：
///
/// 1. **`edit` 的范围保持 UTF-16**（JDT 原样），落地时才换算 —— 理由见模块文档；
///    键用 `file://` URI，与 [`JavaCodeActionProvider::file_uri`] 逐字节一致；
/// 2. **`kind` 原样带过去**：菜单当前只渲染标题，但分类是服务端的事实，
///    诊断行也要用它（`kind=`），将来按类别过滤菜单时是唯一依据；
/// 3. **不塞 `command` / `data` / `isPreferred`**：Java 侧已经把带 command 的 action
///    过滤掉了（本批不执行命令）；`data` 是给 `codeAction/resolve` 用的而我们不走 resolve；
///    `isPreferred` 上游菜单**没有任何读取点** —— 写一个没人消费的字段就是死字段
///    （与 [`crate::diagnostics`] 拒绝映射 `tags` 同一条理由）。
fn to_lsp_action(action: JavaCodeAction, uri: &Uri) -> CodeAction {
    let edits: Vec<TextEdit> = action
        .edits
        .iter()
        .map(|edit| TextEdit {
            range: lsp_types::Range {
                start: Position::new(edit.start.line, edit.start.utf16_column),
                end: Position::new(edit.end.line, edit.end.utf16_column),
            },
            new_text: edit.new_text.clone(),
        })
        .collect();
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), edits);

    CodeAction {
        title: action.title,
        kind: action.kind.map(CodeActionKind::from),
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        ..Default::default()
    }
}

/// 诊断行里的 `kind=`：列出这次拿到的 action 分类（去重、保序、最多 4 个）。
///
/// 为什么要有：`actions=3` 看不出"这三条是快速修复还是生成 getter 的 source action"，
/// 而这两者对应完全不同的用户预期；`Ctrl+.` 没给出修复时，这一项是唯一的现场。
fn kind_summary(actions: &[JavaCodeAction]) -> String {
    let mut kinds: Vec<&str> = Vec::new();
    for action in actions {
        let kind = action.kind.as_deref().unwrap_or("none");
        if !kinds.contains(&kind) {
            kinds.push(kind);
        }
    }
    if kinds.is_empty() {
        return "none".to_string();
    }
    if kinds.len() > 4 {
        kinds.truncate(4);
        kinds.push("…");
    }
    kinds.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一条能落地的 action（`Import 'List' (java.util)` 的编辑范围，见
    /// `.artifacts/p17/probe2.log`）。
    fn sample_action() -> JavaCodeAction {
        JavaCodeAction {
            title: "Import 'List' (java.util)".to_string(),
            kind: Some("quickfix".to_string()),
            edits: vec![lithe_gpui_java::JavaTextEdit {
                start: JavaPosition {
                    line: 0,
                    utf16_column: 0,
                },
                end: JavaPosition {
                    line: 2,
                    utf16_column: 0,
                },
                new_text: "package demo;\n\nimport java.util.List;\n\n".to_string(),
            }],
            other_file_edits: 0,
        }
    }

    /// `edit` 必须挂到**本文件**的 URI 上，范围保持 JDT 的 UTF-16 列（落地时才换算），
    /// 标题与 kind 原样透传 —— 上游菜单只渲染标题，标题错了菜单就没用了。
    #[test]
    fn action_mapping_keeps_title_kind_and_utf16_edit() {
        // 与 provider 走同一处换算（`lithe_gpui_java::file_uri` → `lsp_types::Uri`）。
        let uri = test_uri();
        let mapped = to_lsp_action(sample_action(), &uri);

        assert_eq!(mapped.title, "Import 'List' (java.util)");
        assert_eq!(
            mapped.kind.as_ref().map(CodeActionKind::as_str),
            Some("quickfix")
        );
        let changes = mapped
            .edit
            .expect("必须带 edit：上游不会替我们应用")
            .changes
            .expect("changes 是 Core 归一化后的形状");
        let edits = changes.get(&uri).expect("键必须是本文件的 URI");
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].range.start, Position::new(0, 0));
        assert_eq!(edits[0].range.end, Position::new(2, 0));
        assert_eq!(edits[0].new_text, "package demo;\n\nimport java.util.List;\n\n");
    }

    /// `kind` 缺失（服务端没给）时**不 panic**，也不编一个默认值出来：
    /// 上游菜单不看它，诊断行会打 `none`。
    #[test]
    fn action_mapping_tolerates_a_missing_kind() {
        let uri = test_uri();
        let mut action = sample_action();
        action.kind = None;
        let mapped = to_lsp_action(action, &uri);
        assert!(mapped.kind.is_none());
        assert_eq!(
            kind_summary(&[JavaCodeAction {
                kind: None,
                ..sample_action()
            }]),
            "none"
        );
    }

    /// 一个真实的文件 URI（`file:///…`），与 provider 用的是同一次换算。
    ///
    /// 这条同时也是"`lsp_types::Uri` 解析得了 `url` 给的 `file://` 串"的钉子：
    /// 解析方式一旦不兼容，`file_uri()` 会返回 `None`，编辑器就再也落不了地。
    fn test_uri() -> Uri {
        let raw = lithe_gpui_java::file_uri(std::path::Path::new(
            "C:/ws/src/main/java/demo/ImportFix.java",
        ))
        .expect("路径 → file URI");
        assert!(raw.starts_with("file:///"), "{raw}");
        Uri::from_str(&raw).expect("lsp_types 必须解析得了这条 URI")
    }

    /// 路径 → 键 → 取回：**同一份换算在两侧都对得上**（`to_lsp_action` 写的键
    /// 就是 `perform_code_action` 用来取的那把）。这把钥匙对不上，修复就永远落不了地。
    #[test]
    fn edit_key_round_trips_through_the_workspace_edit() {
        let uri = test_uri();
        let mapped = to_lsp_action(sample_action(), &uri);
        let changes = mapped.edit.expect("edit").changes.expect("changes");
        assert!(
            changes.contains_key(&uri),
            "键必须与查询用的 URI 逐字节相同：{:?}",
            changes.keys().map(|key| key.as_str()).collect::<Vec<_>>()
        );
    }

    /// ⚠️ **列口径**：JDT 的编辑范围是 UTF-16 码元列，而上游 `apply_lsp_edits` 把
    /// `Position.character` 当**字符列**（`rope_ext.rs:330-343` 的 `char_to_byte_idx`）。
    /// 行内含非 BMP 字符（emoji / 部分 CJK 扩展区，一个字符 = 两个 UTF-16 码元）时两者不等，
    /// 不换算就会把编辑落到错位置 —— 表现是"修复把别的代码改乱了"。
    ///
    /// 同时钉住**倒序**：`apply_lsp_edits` 顺序应用，正序会让靠后那条的列号失效。
    #[test]
    fn edits_are_converted_to_character_columns_and_sorted_back_to_front() {
        // 行 0 = `x`；行 1 = `🙂    List`（emoji 占 2 个 UTF-16 码元）。
        // `List` 的 UTF-16 列 = 2（emoji）+ 4（空格）= 6，字符列 = 1 + 4 = 5；
        // 行 0 的 `x` 在 UTF-16 列 0、字符列 0。
        let rope = gpui_kit::component::input::Rope::from_str("x\n🙂    List<String> y;\n");
        let edits = vec![
            TextEdit {
                range: lsp_types::Range {
                    start: Position::new(0, 0),
                    end: Position::new(0, 0),
                },
                new_text: "import java.util.List;\n".to_string(),
            },
            TextEdit {
                range: lsp_types::Range {
                    start: Position::new(1, 6),
                    end: Position::new(1, 10),
                },
                new_text: "java.util.List".to_string(),
            },
        ];
        let converted = to_editor_edits(&rope, &edits);

        assert_eq!(converted.len(), 2);
        assert_eq!(
            converted[0].range.start,
            Position::new(1, 5),
            "UTF-16 列 6 → 字符列 5（倒序之后第一条是行 1 那条）"
        );
        assert_eq!(converted[0].range.end, Position::new(1, 9), "UTF-16 列 10 → 字符列 9");
        assert_eq!(converted[0].new_text, "java.util.List");
        assert_eq!(converted[1].range.start, Position::new(0, 0), "行 0 在后面");
    }

    /// 纯 ASCII 的对照：两套列相等，换算必须**原样**通过 ——
    /// 它证明上一条的差异确实来自 emoji，而不是换算函数一律偏移。
    #[test]
    fn ascii_edits_are_unchanged() {
        let rope = gpui_kit::component::input::Rope::from_str("package demo;\n\npublic class A {}\n");
        let edits = vec![TextEdit {
            range: lsp_types::Range {
                start: Position::new(0, 0),
                end: Position::new(2, 0),
            },
            new_text: "package demo;\n\nimport java.util.List;\n\n".to_string(),
        }];
        let converted = to_editor_edits(&rope, &edits);
        assert_eq!(converted[0].range.start, Position::new(0, 0));
        assert_eq!(converted[0].range.end, Position::new(2, 0));
    }

    /// `kind=` 是**去重保序**的（同一类 action 出现多次只报一次），超过 4 类时截断。
    #[test]
    fn kind_summary_dedupes_and_truncates() {
        let make = |kind: &str| JavaCodeAction {
            kind: Some(kind.to_string()),
            ..sample_action()
        };
        assert_eq!(
            kind_summary(&[
                make("quickfix"),
                make("quickfix"),
                make("source.generate.accessors"),
            ]),
            "quickfix,source.generate.accessors"
        );
        assert_eq!(
            kind_summary(&[
                make("a"),
                make("b"),
                make("c"),
                make("d"),
                make("e"),
            ]),
            "a,b,c,d,…"
        );
        assert_eq!(kind_summary(&[]), "none");
    }
}
