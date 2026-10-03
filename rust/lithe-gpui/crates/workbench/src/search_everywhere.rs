//! Search Everywhere 浮层（菜单「视图 → 全局搜索」）：五个 scope —— All / Classes / Files / Symbols / Text。
//!
//! ## 真源：`SearchEverywhereView.swift`，但只做五个 scope
//!
//! | 规格 | 值 / 行为 | 出处 |
//! | --- | --- | --- |
//! | 浮层宽 / 最大高 / 距顶 | 860 / 560 / 84 | `macos/Sources/Lithe/Views/Search/SearchEverywhereView.swift:192-195` |
//! | scope 标签行高 / 标签高 / 标签左右内衬 / 标签间距 | 40 / 38 / 11 / 2 | 同文件 `:254` / `:228` / `:227` / `:219` |
//! | 活动标签 | 主文本色 + 半粗 + 底部 2px 强调色下划线 | 同文件 `:225-232` |
//! | 输入框行高 | 44 | 同文件 `:282` |
//! | 结果行高 / 行内间距 | 24 / 8 | 同文件 `:423` / `:367` |
//! | 结果行内容 | 图标 + 名字（`symbolName ?? 文件名`）+ `:行号` + 所在目录 + 预览（只有 content） | 同文件 `:364-397` |
//! | 搜索中显示 spinner | `feature.isSearchingEverywhere` | 同文件 `:241-243` |
//! | 五个 scope ↔ Core `kind` | 见 `crate::search_model::SearchEverywhereScope` 的表 | 同文件 `:120-141` |
//! | Tab / `←` / `→` **环绕**切 scope | `(i + offset + n) % n` | 同文件 `:523-529,542-547` |
//! | 防抖 | 180ms | 同文件 `:212` |
//! | 结果上限 | `EVERYWHERE_MAX_RESULTS`（1000，推导见该常量） | Core `project/files.rs:326-332` |
//! | 文案 | scope 名与「X 中没有匹配项」都取真源既有的 SwiftUI 字面量 | `macos/Resources/zh-Hans.lproj/Localizable.strings:194,884,885,887,1119,1164-1168` |
//!
//! ## 复用（本任务最重要的工程要求）
//!
//! 1. **请求 / 解析 / 代次一份都不用重写**：`workspace.searchEverywhere` 收的是与
//!    `workspace.search` **同一个** `SearchRequest`（`rust/lithe-core/src/project/files.rs:272`），
//!    所以本模块直接用 `crate::search_model` 的 [`payload_with_limit`] / [`parse_matches`] /
//!    [`QueryTimeline`]，只把命令名换成 [`EVERYWHERE_COMMAND`]、把 `maxResults` 换成
//!    [`EVERYWHERE_MAX_RESULTS`]（**同一个** `json!`，见 `payload_with_limit` 的文档）。
//!    **本模块不写任何匹配、模糊度或排序** —— 那是 Core 的 `search_with_index`
//!    （`files.rs:190-268`）与符号扫描（`files.rs:296-320`），宿主只做"把结果画出来"。
//! 2. **五个 scope 的投影是纯函数**，落在 `crate::search_model::everywhere_rows`
//!    （可确定性单测）；Files 那一档直接**复用 Quick Open 的 `quick_open_rows`** ——
//!    Quick Open 与 Search Everywhere 的 Files scope 是同一份投影，不写第二份。
//! 3. **浮层形状照 [`crate::quick_open`]**：同一个 `Dialog` + `Command` 组合，
//!    不引第三种浮层写法。`Command` 自带搜索框、虚拟列表、`↑`/`↓`/`Enter`/`Esc`、
//!    行高亮与"搜索中"spinner（`gpui-component-0.7.0/src/command/state.rs:905-941`）。
//!    ⚠️ `Command` 的本地过滤**显式关掉**（`.filterable(false)`，
//!    `command/command.rs:117-126`）：列表内容 = Core 给的行，一个字都不再筛。
//! 4. **打开结果复用既有通路**：`on_open` 回调由外壳注入（与查找面板同一个签名），
//!    落到 `lithe_gpui_editor` 的 `EditorPane::open`（行号为 `None`）或
//!    `EditorPane::open_at`（有行号，`origin_kind = "search-everywhere"`）。本模块不碰编辑区。
//!
//! ## 与真源的有意差异（逐条）
//!
//! 1. **只做五个 scope**：真源六个，`Actions` **仍不在本切片**（它是宿主自己的命令目录，
//!    不是工作区搜索）。`Classes` 已做：它对应 Core 的 `kind == "type"`，取法与 Symbols
//!    那一档**不同 kind**（详见 `SearchEverywhereScope` 的表）。`type` 行本来就**出现在 All**
//!    里，所以这一档只是把同一批行**再单独给一个标签页**，不动 All 的口径。
//! 2. **没有相关度重排**：真源 All 会按 `SearchRelevance.score` 重排
//!    （`SearchEverywhereView.swift:11-90,158-170`），而 Core 的响应里**没有** relevance 字段
//!    （`protocol/contracts.rs:94-105` 的 `SearchMatch` 只有 `kind/path/line/preview/symbolName`），
//!    所以在宿主里重排就等于写第二套相关度算法。本模块**沿用 Core 返回的固定顺序**
//!    （它本身确定：file → types → symbols → content，`files.rs:326-330`）。
//!    这是本切片与真源**最大的一条**有意差异，也是"重排等 Core 给字段（main 那一侧）"的落点。
//! 3. **`Tab` / `Shift+Tab` 环绕切 scope，`←` / `→` 不绑**：真源挂了一个 NSEvent 全局监听，
//!    连输入框里的左右键都吞掉（`:512-540`）。gpui 侧做不到这一点：查询框的 `←`/`→` 由
//!    上游 `Input` **无条件**绑定并处理（`gpui-base-0.7.0/src/input/base/state.rs:179-180,4454-4457`），
//!    动作一旦被输入框处理就不再冒泡；而 `Tab` / `Shift+Tab` 在 `Input` 里只对**多行**
//!    输入注册（同文件 `:4447-4450`），单行查询框不处理，所以能安全接管
//!    （本模块用 `.key_context(SCOPE_KEY_CONTEXT)` + 两条 `KeyBinding`，比 gpui-base 根节点
//!    的 focus traversal 绑定 `root.rs:16-17` 更深，因此优先级更高）。
//!    `←`/`→` 留在输入框里做**光标移动** —— 那对单行查询框是更有用的行为。
//! 4. **目录用尾部省略**：gpui 的 `truncate()` 是尾部省略号，真源是 `truncationMode(.middle)`
//!    （`:391`）—— 与 [`crate::quick_open`] 记的同一条。
//! 5. **不画每行右侧的「在文件管理器中显示」按钮与 Maven 模块标签**：前者要宿主级的
//!    reveal 通路（本 crate 没有），后者要模块归属（Maven 上下文，另一个子系统）。
//!    两处都不影响"这一行是什么"（名字 / 行号 / 目录 / 预览都在）。
//! 6. **`Esc` 先清空查询、再关浮层**：`Command` 自己的语义（`command/state.rs:600-606`），
//!    与 [`crate::quick_open`] / [`crate::command_palette`] 记的同一条；真源是一按就关。
//! 7. **scope 标签行没有用上游 `TabBar`**：它的 `on_click` 回调只给**下标**
//!    （`gpui-component-0.7.0/src/tab/tab_bar.rs:171-179`），标签自身的元素 id 由上游按下标发
//!    （`:506-537`），与"`ElementId` 用业务身份"这条仓库口径冲突；而本侧只有五个固定标签，
//!    自己画能同时满足业务 id 与真源的 40/38/2px 三个数。
//!
//! ## 诊断（stderr）
//!
//! `S1_SEARCH_EVERYWHERE open|scope|idle|issue|complete|stale|failed|cancel|open_result|close <fields>`。
//! 与 `S1_SEARCH` / `S1_QUICK_OPEN` 同族：gpui 的浮层与键盘路径在本仓库**没有**确定性单测
//! （要真窗口 + 真键事件，而 `ShellWorkspace::new` 会起 JDTLS 预备 / 会话恢复 / 索引 warm），
//! 所以交互路径的机器证据只有这些锚点；能确定性验证的部分全在
//! `crate::search_model` 的纯函数单测里。

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui_kit::component::{ActiveTheme as _, WindowExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, AppContext as _, Context, DefiniteLength, ElementId, Entity, FontWeight,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Task, Window, div, rems,
};
use lithe_gpui_shared::icons::{FileIcon, file_icon, idea, idea_icon_image};
use lithe_gpui_shared::{CoreClient, CoreError, CoreRequest, tr};
use serde_json::Value;

use crate::search_index::SearchIndexReadiness;
use crate::search_model::{
    EVERYWHERE_COMMAND, EVERYWHERE_MAX_RESULTS, EverywhereRow, EverywhereState, QueryTimeline,
    SearchEverywhereScope, SearchOptions, absolute, parse_matches, payload_with_limit,
};

/// 诊断前缀（`eprintln!`，stderr）。与 `S1_SEARCH` / `S1_QUICK_OPEN` 一族同口径。
const TAG: &str = "S1_SEARCH_EVERYWHERE";

/// 输入防抖窗口（毫秒）。值与真源一致：`SearchEverywhereView.swift:212`。
const DEBOUNCE_MS: u64 = 180;

/// 浮层宽 860：`SearchEverywhereView.swift:192` 的 `.frame(width: 860)`。
const PANEL_WIDTH: f32 = 860.;
/// 列表最大高 560：同文件 `:193` 的 `.frame(maxHeight: 560, alignment: .top)`。
const PANEL_MAX_HEIGHT: f32 = 560.;
/// 距窗口顶 84：同文件 `:195` 的 `.padding(.top, 84)`。
const PANEL_TOP_INSET: f32 = 84.;

/// scope 标签行高 40：同文件 `:254` 的 `.frame(height: 40)`。
///
/// 40 恰是**档位内**的 `h_10()`（2.5rem），所以这里不写常量、直接用档位助手 ——
/// 见 `scope_tabs`。
///
/// 单个标签高 38：同文件 `:228` 的 `.frame(height: 38)`。**不在档位上** ⇒ `rems(38 / 16.)`。
const TAB_HEIGHT_SPEC: f32 = 38.;
/// 标签左右内衬 11：同文件 `:227` 的 `.padding(.horizontal, 11)`。档位外 ⇒ `rems(11 / 16.)`。
const TAB_PADDING_SPEC: f32 = 11.;
/// 活动标签的 2px 下划线：同文件 `:231` 的 `.frame(height: 2)`。档位外 ⇒ `rems(2. / 16.)`。
const TAB_UNDERLINE_SPEC: f32 = 2.;

/// 行内图标边长 14：同文件 `:372` 与 `:501-505`。档位外 ⇒ `rems(14 / 16.)`。
const ROW_ICON_SPEC: f32 = 14.;

/// 浮层根元素 id（业务身份，不是下标）。
const OVERLAY_ID: &str = "search-everywhere";
/// scope 标签行的元素 id。
const SCOPE_ROW_ID: &str = "search-everywhere-scopes";

/// scope 标签行提供的 key context。
///
/// 只在**这个浮层**的焦点路径上出现（`Command` 自己的 `"Command"` context 与查询框的
/// `"Input"` 都更深，但它们在 `tab` 上不处理单行输入 —— 见模块文档的差异第 3 条），
/// 所以 `Tab` / `Shift+Tab` 不会漏到命令面板或编辑器里。
const SCOPE_KEY_CONTEXT: &str = "SearchEverywhere";

/// `Tab` / `Shift+Tab` 环绕切 scope（真源 `SearchEverywhereView.swift:526-529`）。
///
/// 只登记绑定，**处理在各浮层自己的根元素上**：绑定是应用级、只登记一次，而"当前有没有
/// 这个浮层"由 key context 判定 —— 命令面板（也用 `Command`）里按 `Tab` 不会命中这里。
pub fn install_actions(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", NextScope, Some(SCOPE_KEY_CONTEXT)),
        KeyBinding::new("shift-tab", PreviousScope, Some(SCOPE_KEY_CONTEXT)),
    ]);
}

gpui_kit::actions!(lithe_workbench, [NextScope, PreviousScope]);

/// 打开浮层（**只能从事件回调或任务里调用**；`render` 阶段调 `window.open_dialog` 会 panic）。
///
/// 已经开着别的对话框时不叠第二层 —— 与 `open_command_palette` / `open_quick_open` /
/// `open_branch_panel` 同一条口径。
pub(crate) fn open_dialog(overlay: Entity<SearchEverywhere>, window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        SearchEverywhere::diagnose("open", "false reason=active_dialog");
        return;
    }
    // 两个入口（主菜单「视图 → 全局搜索」与 `CommandId::OpenSearchEverywhere`）都只到
    // 这一个开浮层的点，所以这一行就是"浮层真的被要求打开"的锚点。
    SearchEverywhere::diagnose("open", "true");
    window.open_dialog(cx, move |dialog, window, _cx| {
        // `Dialog::width` / `margin_top` 是**固有方法**、只吃 `Pixels`，所以按**当帧的**
        // rem 基准换算（写死就是假 rem，见 `crate::rem_px`）。
        let rem = window.rem_size();
        dialog
            .close_button(false)
            .overlay(true)
            .overlay_closable(true)
            .keyboard(true)
            .width(crate::rem_px(rem, PANEL_WIDTH))
            .margin_top(crate::rem_px(rem, PANEL_TOP_INSET))
            .p_0()
            // 关闭的**唯一**收尾点：`Esc` / 点遮罩 / 点关闭按钮三条路都经过它。
            .on_close({
                let handle = overlay.clone();
                move |_event, window, cx| {
                    let _ = handle.update(cx, |overlay, cx| overlay.close(window, cx));
                }
            })
            .content({
                let handle = overlay.clone();
                move |content, _, _| content.p_0().gap_0().child(handle.clone())
            })
    });
}

/// 浮层的交互状态。
///
/// 与 `QuickOpen` / `CommandPalette` 同一口径：`open_dialog` 的 builder **每帧重跑**，
/// 状态必须存在 `Entity` 里；这里持有 `CommandState`（查询框与高亮的归属方）、当前 scope
/// 与本模块自己的四态、代次、防抖任务。
pub struct SearchEverywhere {
    /// 工作区根：Core 的 `root`，也是"相对路径 → 绝对路径"的基准。
    root: PathBuf,
    /// 打开一条结果（由外壳注入；落到 `EditorPane::open` / `open_at`）。
    ///
    /// 第二个参数 = 要跳的行号（真源 `result.line`）；`None` 就是只打开文件。
    on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static>,
    /// `Command` 的状态（查询框、可见项、高亮）。
    command: Entity<CommandState>,
    /// 当前 scope（**本地状态**：切它不再打一次 Core，理由见 `EverywhereState`）。
    scope: SearchEverywhereScope,
    /// 当前查询串（`Command` 只在查询**真的变了**时回调 `on_query`）。
    query_text: SharedString,
    /// 四态（`idle` / `loading` / `ready` / `failed`）。
    state: EverywhereState,
    /// 代次与在飞 `operationId`（stale 保护 + 取消句柄），三个搜索界面共用同一个类型。
    timeline: QueryTimeline,
    /// 防抖任务：**换掉旧的 = 取消旧的唤醒**（`Task` 一 drop 就取消）。
    debounce_task: Option<Task<()>>,
    /// 搜索索引的就绪状态（`U18` 收尾）。**只读的展示值**：owner 是 `crate::search_index`，
    /// 浮层在**打开时**由外壳读一次推进来（构造参数），之后由外壳在状态变化时推
    /// （[`Self::set_search_index_readiness`]）。浮层不据此禁用 / 排队 / 改 scope 语义。
    search_index_readiness: SearchIndexReadiness,
}

impl SearchEverywhere {
    /// 建浮层状态（`open_search_everywhere` 那条回调里建一次，之后由 `Entity` 跨帧持有）。
    ///
    /// `search_index_readiness`：**打开这一刻**的索引就绪状态（外壳从 `search_index` 读，
    /// 唯一 owner 是它）。
    pub(crate) fn new(
        root: PathBuf,
        on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static>,
        search_index_readiness: SearchIndexReadiness,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            root,
            on_open,
            command: cx.new(|cx| CommandState::new(window, cx)),
            // 真源默认落在 All（`SearchEverywhereView.swift:111` 的 `@State scope = .all`）。
            scope: SearchEverywhereScope::All,
            query_text: SharedString::default(),
            state: EverywhereState::Idle,
            timeline: QueryTimeline::default(),
            debounce_task: None,
            search_index_readiness,
        }
    }

    /// 外壳推来一个新的索引就绪状态（**只在真的变了时** `cx.notify()`）。
    pub(crate) fn set_search_index_readiness(
        &mut self,
        readiness: SearchIndexReadiness,
        cx: &mut Context<Self>,
    ) {
        if self.search_index_readiness == readiness {
            return;
        }
        self.search_index_readiness = readiness;
        cx.notify();
    }

    /// 查询串变了（`Command` 的 `on_query`，只在**真的变了**时来）。
    ///
    /// 旧查询**立刻作废**（换代 + 尽力取消），再排一次 180ms 防抖；
    /// **状态在防抖窗口里不动**：上一次的结果继续留在屏幕上（真机同样不清空
    /// `visibleItems`，`SearchEverywhereView.swift:302-328`）。
    fn on_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.query_text.as_ref() == query {
            return;
        }
        self.query_text = SharedString::from(query);
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        // 空查询**不发 Core 调用**：`search_everywhere` 对空查询直接回空（`files.rs:274-279`）。
        if self.query_text.trim().is_empty() {
            self.debounce_task = None;
            self.state = EverywhereState::Idle;
            self.set_loading(false, window, cx);
            Self::diagnose("idle", "query=empty");
            cx.notify();
            return;
        }
        self.debounce_task = Some(cx.spawn_in(window, async move |this, async_cx| {
            async_cx
                .background_executor()
                .timer(Duration::from_millis(DEBOUNCE_MS))
                .await;
            // 实体可能已经销毁（浮层已关）：`update_in` 返回 `Err` 时静默忽略，不 panic。
            let _ = this.update_in(async_cx, |overlay, window, cx| overlay.issue(window, cx));
        }));
        cx.notify();
    }

    /// 真的发一次查询（防抖到期）。
    fn issue(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.query_text.trim().to_string();
        if query.is_empty() {
            // 防御：防抖窗口里被清空（那时 `on_query` 已经把状态打回 `Idle`）。
            return;
        }
        let scheduled = self.timeline.schedule();
        if let Some(superseded) = scheduled.superseded.clone() {
            Self::cancel_in_background(superseded, cx);
        }
        let generation = scheduled.generation;
        let operation_id = scheduled.operation_id.clone();
        let root = self.root.to_string_lossy().to_string();
        let previous = self.state.matches().to_vec();
        self.state = EverywhereState::loading(query.clone(), &previous);
        // ⚠️ spinner 不只是装饰：`Command` 只在**非 loading 且零行**时才画空态
        // （`command/state.rs:989-993`），不置这一位的话"在路上"会被画成"没有匹配"。
        self.set_loading(true, window, cx);
        Self::diagnose(
            "issue",
            &format!(
                "generation={generation} operation={operation_id} scope={} query={query}",
                self.scope.token()
            ),
        );
        cx.notify();

        // 同步的 Core 调用只在 `background_spawn` 里跑（UI 线程不做阻塞工作）。
        // 任务 detach 掉：代次闸门才是采纳判据。
        let background_query = query.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    // 三个搜索开关固定默认值（真机 Search Everywhere 也没有它们；那是
                    // Find in Files 侧栏的三个 Checkbox），请求形状与查找面板共用同一个构造器，
                    // 只有上限不同 —— 上限为什么是 1000 见 `EVERYWHERE_MAX_RESULTS` 的文档。
                    let request = CoreRequest::command(EVERYWHERE_COMMAND)
                        .with_payload(payload_with_limit(
                            &root,
                            &background_query,
                            &SearchOptions::default(),
                            EVERYWHERE_MAX_RESULTS,
                        ))
                        .with_operation_id(operation_id);
                    CoreClient::new().execute(&request)
                })
                .await;
            let _ = this.update_in(cx, |overlay, window, cx| {
                overlay.adopt(generation, &query, result, window, cx)
            });
        })
        .detach();
    }

    /// 后台结果回到前台：**先过代次闸门**，再进 `ready` / `failed`。
    fn adopt(
        &mut self,
        generation: u64,
        query: &str,
        result: Result<Option<Value>, CoreError>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.timeline.accepts(generation) {
            // 过期结果：**不采纳**（契约要求），也不改任何状态（连 spinner 都不动）。
            Self::diagnose(
                "stale",
                &format!(
                    "generation={generation} current_query_len={}",
                    self.query_text.len()
                ),
            );
            return;
        }
        let next = match result {
            Ok(data) => match parse_matches(data.as_ref()) {
                Ok(matches) => EverywhereState::ready(query, &matches),
                // 契约异常（缺 data / 形状不对）：给稳定 token，不给句子。
                Err(error) => EverywhereState::failed(query, error.token()),
            },
            // Core 的失败原文是**数据**：`CoreError` 的 Display 给 `code: message`。
            Err(error) => EverywhereState::failed(query, error.to_string()),
        };
        match &next {
            EverywhereState::Failed { reason, .. } => Self::diagnose(
                "failed",
                &format!("generation={generation} query={query} reason={reason}"),
            ),
            _ => {
                // 五个 scope 各自的可画行数都报出来：B10（符号饥饿）是不是真的绕开了，
                // 靠的就是这一行里 `symbols=` 不为 0。
                let counts = SearchEverywhereScope::ALL
                    .iter()
                    .map(|scope| format!("{}={}", scope.token(), next.rows(*scope).len()))
                    .collect::<Vec<_>>()
                    .join(" ");
                Self::diagnose(
                    "complete",
                    &format!(
                        "generation={generation} query={query} matches={} {counts}",
                        next.matches().len()
                    ),
                );
            }
        }
        self.state = next;
        self.timeline.settle(generation);
        self.set_loading(false, window, cx);
        cx.notify();
    }

    /// 切 scope（点标签 / `Tab` / `Shift+Tab`）。
    ///
    /// ⚠️ **不打第二次 Core**：scope 是本地投影口径（真源的搜索任务只按查询串作 key，
    /// `SearchEverywhereView.swift:207`），五个 scope 共用同一份命中。
    /// 高亮回到第一行这件事不在这里手工算：`Command` 在模型重装时若旧的选中项不在新集合里
    /// 会自己 `reset_selection()`（`gpui-component-0.7.0/src/command/state.rs:200-220`），
    /// 而切 scope 必然换掉整个行集合 —— 与真源 `onChange(of: scope) { selectedIndex = 0 }`
    /// （同文件 `:206`）同一效果，且不用自己去操作上游的 `IndexPath`。
    fn set_scope(&mut self, scope: SearchEverywhereScope, cx: &mut Context<Self>) {
        if self.scope == scope {
            return;
        }
        self.scope = scope;
        Self::diagnose(
            "scope",
            &format!(
                "scope={} rows={}",
                scope.token(),
                self.state.rows(scope).len()
            ),
        );
        cx.notify();
    }

    /// 环绕切一格（`Tab` / `Shift+Tab` → [`SearchEverywhereScope::moved`]）。
    fn on_scope_next(&mut self, _: &NextScope, _window: &mut Window, cx: &mut Context<Self>) {
        let next = self.scope.moved(1);
        self.set_scope(next, cx);
    }

    /// 反向环绕一格。
    fn on_scope_prev(&mut self, _: &PreviousScope, _window: &mut Window, cx: &mut Context<Self>) {
        let previous = self.scope.moved(-1);
        self.set_scope(previous, cx);
    }

    /// `Enter`（或鼠标点击某一行）：打开第 `row` 条结果 → 关浮层。
    ///
    /// ⚠️ **先关浮层再打开文件**：与 `QuickOpen::confirm` 同一个顺序。
    /// 有行号的结果（符号 / 类型 / 正文命中）走 `open_at` 跳到那一行，`origin_kind` 传
    /// `"search-everywhere"` —— 与问题列表的 `"problems"` / 查找面板的 `"search"` 分开，
    /// `S1_NAV_JUMP kind=` 那一行才能说清这一跳是哪来的。
    fn confirm(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(target) = self.state.rows(self.scope).get(row).cloned() else {
            // 行号越界（结果刚换代 / 刚切了 scope）：什么都不做，不 panic、也不落到别的行。
            return;
        };
        window.close_dialog(cx);
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        self.debounce_task = None;
        self.set_loading(false, window, cx);
        let path = absolute(&self.root, &target.path);
        let line = target.line.map(|line| line as u32);
        Self::diagnose(
            "open_result",
            &format!(
                "scope={} file={} line={}",
                self.scope.token(),
                path.display(),
                line.map(|line| line.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
        );
        (self.on_open)(path, line, window, cx);
    }

    /// 浮层关了（`Esc` / 点遮罩 / 点关闭按钮）：放弃在飞查询并复位。
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        self.debounce_task = None;
        self.state = EverywhereState::Idle;
        self.scope = SearchEverywhereScope::All;
        self.set_loading(false, window, cx);
        Self::diagnose("close", "reason=dialog");
        cx.notify();
    }

    /// 把 `Command` 搜索框的 spinner 拨到 `loading`。
    fn set_loading(&self, loading: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.command
            .update(cx, |state, cx| state.set_loading(loading, window, cx));
    }

    /// 把 `operationId` 交给后台取消。
    ///
    /// **不在 UI 线程调**：它要拿 Core 的操作注册表锁（`cancellation.rs:84`）。
    fn cancel_in_background(operation_id: String, cx: &mut Context<Self>) {
        cx.background_spawn(async move {
            let requested = lithe_gpui_shared::cancel_operation(&operation_id);
            Self::diagnose(
                "cancel",
                &format!("operation={operation_id} requested={requested}"),
            );
        })
        .detach();
    }

    /// 一行 stderr 诊断（`S1_SEARCH_EVERYWHERE <event> <fields>`）。
    fn diagnose(event: &str, fields: &str) {
        eprintln!("{TAG} {event} {fields}");
    }

    /// scope 标签行（真源 `scopeTabs`，`SearchEverywhereView.swift:218-256`）。
    fn scope_tabs(&self, cx: &Context<Self>) -> impl IntoElement {
        let mut row = h_flex()
            .id(SCOPE_ROW_ID)
            .w_full()
            // 40：真源 `:254`（= `h_10()`，档位内）。
            .h_10()
            // 6：真源 `:253` 的 `.padding(.horizontal, 6)`。
            .px_1p5()
            // 2：真源 `:219` 的 `HStack(spacing: 2)`。
            .gap_0p5()
            .items_center()
            .bg(cx.theme().tab_bar)
            .border_b_1()
            .border_color(cx.theme().border);
        for scope in SearchEverywhereScope::ALL {
            row = row.child(self.scope_tab(scope, cx));
        }
        row
    }

    /// 一个 scope 标签：活动态 = 主文本色 + 半粗 + 底部 2px 强调色下划线
    /// （真源 `:225-232`）。
    ///
    /// ⚠️ token 映射照 `UI-MAP.md` §1.2：macOS 的 `accent`（强调色）对应 gpui 的
    /// `primary`，**不是** gpui 的 `accent`（那是悬停底色）；`toolHeader` 在 gpui-kit 里
    /// 没有同名 token，取语义最近的 `tab_bar`。
    fn scope_tab(&self, scope: SearchEverywhereScope, cx: &Context<Self>) -> impl IntoElement {
        let active = self.scope == scope;
        let foreground = if active {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };
        v_flex()
            // 元素 id 用**业务身份**（scope token），不用下标：切 scope 时下标会把元素状态
            // 串到别的标签上。
            .id(ElementId::from(format!(
                "search-everywhere-scope-{}",
                scope.token()
            )))
            .relative()
            // 38：真源 `:228`。档位外 ⇒ rems(38 / 16.)。
            .h(rems(TAB_HEIGHT_SPEC / 16.))
            .justify_center()
            .items_center()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.set_scope(scope, cx)))
            .child(
                h_flex()
                    // 11：真源 `:227`。档位外 ⇒ rems(11 / 16.)。
                    .px(rems(TAB_PADDING_SPEC / 16.))
                    .text_xs()
                    .font_weight(if active {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .text_color(foreground)
                    .child(tr(scope.label_key())),
            )
            .when(active, |this| {
                this.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        // 2：真源 `:231`。档位外 ⇒ rems(2. / 16.)。
                        .h(rems(TAB_UNDERLINE_SPEC / 16.))
                        .bg(cx.theme().primary),
                )
            })
    }
}

impl Render for SearchEverywhere {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 失败原因先取出来：空态闭包要求 `'static`，不能借用 `self`。
        let failure: Option<SharedString> = match &self.state {
            EverywhereState::Failed { reason, .. } => Some(SharedString::from(reason.clone())),
            _ => None,
        };
        // 行也是先克隆出来（同一个理由）：`Command` 的 items 归 `CommandState`。
        let scope = self.scope;
        let rows: Vec<EverywhereRow> = self.state.rows(scope);
        let empty_query = self.state.query().trim().is_empty();
        let empty_key = scope.empty_key();
        // 索引就绪那一行（`Ready` 时是 `None`）：判据与元素都归 `crate::search_index`。
        // 位置在 scope 标签行与列表之间 —— 它解释的是"下面这列为什么可能慢"，不是 scope 的一部分。
        let index_notice = crate::search_index::readiness_notice(self.search_index_readiness, cx);

        v_flex()
            .id(OVERLAY_ID)
            // `Tab` / `Shift+Tab` 的归属：只有这个浮层在焦点路径上才带这个 context
            // （见 `SCOPE_KEY_CONTEXT` 的文档）。
            .key_context(SCOPE_KEY_CONTEXT)
            .w_full()
            .on_action(cx.listener(Self::on_scope_next))
            .on_action(cx.listener(Self::on_scope_prev))
            .child(self.scope_tabs(cx))
            .children(index_notice)
            .child(
                Command::new(&self.command)
                    .bordered(false)
                    .w_full()
                    .filterable(false)
                    // 查询框的 placeholder 复用查找面板那条（真源 Search Everywhere 的
                    // 输入框没有 placeholder，只有查询为空时叠一句
                    // "Type / to see commands" —— 那是 `/` 命令模式，不在本切片）。
                    .placeholder(tr("lithe.search.gpui.queryPlaceholder"))
                    // 列表跟着 560 的面板一起长（`Command` 的列表默认上限 18.75rem = 300）。
                    .max_h(DefiniteLength::from(rems(PANEL_MAX_HEIGHT / 16.)))
                    .items(rows.into_iter().map(IntoCommandItem::into_command_item))
                    .empty(move |_state, _window, cx| {
                        if let Some(reason) = failure.clone() {
                            // 失败：Core 的原文（或契约异常的稳定 token）。
                            return div()
                                .py_6()
                                .w_full()
                                .text_center()
                                .text_sm()
                                .text_color(cx.theme().danger)
                                .child(reason)
                                .into_any_element();
                        }
                        if empty_query {
                            // 空查询：什么都不画 —— 占位符已经说了"搜索文件和内容"，
                            // 真源也是"有查询才画结果区"（`SearchEverywhereView.swift:187`）。
                            return div().into_any_element();
                        }
                        // 有查询但这个 scope 没有命中：真源按 scope 分句
                        // （`SearchEverywhereView.swift:75-92`）。
                        div()
                            .py_6()
                            .w_full()
                            .text_center()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(tr(empty_key))
                            .into_any_element()
                    })
                    .on_query({
                        let overlay = cx.entity();
                        move |query, window, cx| {
                            let _ = overlay
                                .update(cx, |overlay, cx| overlay.on_query(query, window, cx));
                        }
                    })
                    .on_confirm({
                        let overlay = cx.entity();
                        move |index, window, cx| {
                            // 未分组条目的 `IndexPath` 是 `section = 0` + `row` = **传入 items
                            // 的行号**（`command/state.rs:384`；`filterable(false)` 下本地过滤
                            // 不改这两个坐标）。它同时也是"当前 scope 的第 row 行"。
                            let _ = overlay
                                .update(cx, |overlay, cx| overlay.confirm(index.row, window, cx));
                        }
                    }),
            )
    }
}

/// 让 [`EverywhereRow`] 能被 `Command` 直接消费的适配。
///
/// 放在这里而不是 `search_model`：`CommandItem` 是**浮层**的零件，模型层只出数据
/// （与 [`crate::quick_open`] 的同一处置）。
trait IntoCommandItem {
    fn into_command_item(self) -> CommandItem;
}

impl IntoCommandItem for EverywhereRow {
    fn into_command_item(self) -> CommandItem {
        // 「这一行画什么」的判据全部来自模型层（真源 `:349,392`），视图不自己再判一遍。
        let shows_line = self.shows_line();
        let shows_preview = self.shows_preview();
        let class_icon = self.kind.is_type();
        let java_icon = self.kind.is_symbol();
        let EverywhereRow {
            path,
            name,
            parent,
            line,
            preview,
            ..
        } = self;
        // 真源 `:377` 画的是 `result.symbolName ?? url.lastPathComponent`，也就是这里的 `name`。
        let label = SharedString::from(name.clone());
        let item_name = SharedString::from(name);
        let item_parent = SharedString::from(parent);
        let item_preview = SharedString::from(preview);
        // 图标：符号 / 类型命中走 IDEA 的 class / java 文件图标（真源 `:497-505` 的
        // `LitheIconKind.javaClass` / `.javaGeneric`），其余按**文件类型**查表
        // （与资源管理器、Quick Open 同一套共享层查表）。
        CommandItem::new().label(label).child(move |_, cx| {
            // 真源每行是「图标 + 名字 + :行号 + 所在目录 +（正文命中才有的）预览」
            // （`SearchEverywhereView.swift:364-397`）。
            //
            // ⚠️ 元素 id 用**业务身份**（`scope 行身份` = 路径 + 行号 + 名字），不用下标：
            // 结果集换代 / 切 scope 时下标会把元素状态串到别的行上
            // （`crate::search_panel` 与 `crate::quick_open` 同一口径）。
            // ⚠️ 行的**点击与选中** id 是上游 `Command` 自己的 `IndexPath`，本侧改不了。
            // ⚠️ 这个工厂是 `Fn`（测量 / 进视口 / 排版失效都会重跑），所以只能从捕获的
            // `SharedString` 克隆（原子引用计数，很便宜），不能把它们 move 出去。
            let identity = match line {
                Some(line) => format!("{path}-{line}-{item_name}"),
                None => path.clone(),
            };
            h_flex()
                .id(ElementId::from(format!("search-everywhere-row-{identity}")))
                .w_full()
                .min_w_0()
                // 24：真源结果行高（`SearchEverywhereView.swift:423`），档位内（`h_6`）。
                .h_6()
                // 8：同文件 `:367` 的 `HStack(spacing: 8)`。
                .gap_2()
                .items_center()
                .child(if class_icon {
                    idea_icon_image(&idea::CLASS_ICON, cx, rems(ROW_ICON_SPEC / 16.))
                } else if java_icon {
                    idea_icon_image(&idea::JAVA_ICON, cx, rems(ROW_ICON_SPEC / 16.))
                } else {
                    // 文件类型图标走共享层（主题包真源 → Lucide 回落）。
                    FileIcon::lucide(file_icon::lucide_fallback(item_name.as_ref()))
                        .with_theme_icon(item_name.as_ref(), cx)
                        .render(rems(ROW_ICON_SPEC / 16.), cx)
                })
                .child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().foreground)
                        .child(item_name.clone()),
                )
                .when(shows_line && line.is_some(), |this| {
                    this.child(
                        div()
                            .flex_shrink_0()
                            // 真源是 10.5px 等宽（`:382-383`）：字号取最近的 `text_xs()`，
                            // 等宽字族不在本切片的槽位上（主题的 mono 字族归编辑器）。
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(SharedString::from(format!(":{}", line.unwrap_or(0)))),
                    )
                })
                .child(
                    // 目录：占满剩余宽度、尾部省略（真源是 middle，见模块文档的差异第 4 条）。
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(item_parent.clone()),
                )
                .when(shows_preview, |this| {
                    this.child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(item_preview.clone()),
                    )
                })
                .into_any_element()
        })
    }
}
