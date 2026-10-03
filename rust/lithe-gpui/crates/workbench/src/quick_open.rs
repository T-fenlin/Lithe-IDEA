//! Quick Open 浮层（`Ctrl+P`）：输入查询 → 列出**文件** → `↑`/`↓` 选择 → `Enter` 打开。
//!
//! ## 真源：macOS 侧**没有** Quick Open 视图
//!
//! 已用 grep 确认 `macos/Sources/**` 对 `QuickOpen` / `quickOpen` 是**零命中**：`Search/`
//! 那六个文件里唯一的浮层是 `SearchEverywhereView`（六个 scope 的全局搜索），Quick Open 在
//! macOS 前端不存在。所以本模块的规格来自两处，逐条写明出处：
//!
//! | 规格 | 值 / 行为 | 出处 |
//! | --- | --- | --- |
//! | 浮层宽 / 最大高 / 距顶 | 860 / 560 / 84 | `macos/Sources/Lithe/Views/Search/SearchEverywhereView.swift:192-195`（macOS 唯一的全局搜索浮层） |
//! | 结果行高 / 行内间距 | 24 / 8 | 同文件 `:367,423`（`frame(height: 24)`、`HStack(spacing: 8)`） |
//! | 行内字号 | 文件名 12.5、目录 11 | 同文件 `:377,388` → 12.5 / 11 都不在 gpui 档位上，按仓库口径各取最近的 `text_xs()`（12） |
//! | 选中态底色 | `LitheTheme.selection` | 同文件 `:424` → 本侧 `cx.theme().list_active`（`UI-MAP.md` §1.2 的 token 映射） |
//! | 列表只在有查询时出现 | `if hasQuery { resultsList }` | 同文件 `:187` |
//! | `↑` / `↓` | 到头**夹住**（不环绕） | 同文件 `:517-522`（`min`/`max`） |
//! | `Enter` / `Esc` | 打开选中项 / 关闭 | 同文件 `:530-535` |
//! | 换查询时高亮回第一项 | `onChange(of: query) { selectedIndex = 0 }` | 同文件 `:205` |
//! | 输入防抖 | 180ms | 同文件 `:212`（`Task.sleep(for: .milliseconds(180))`） |
//! | 结果上限 | 200 | `macos/Sources/LitheSearchModule/Models/SearchResults.swift:58`（`matchLimit`），与本侧 `search_model::MAX_RESULTS` 同值 |
//! | 文案 | 占位符「输入以搜索文件...」、空态「未找到匹配的文件」 | 真源既有键 `quickOpen.searchFiles` / `quickOpen.noMatch`（`locales/lithe.zh-CN.yml:6786,6800`） |
//!
//! ## 复用（本任务最重要的工程要求）
//!
//! 1. **请求 / 解析 / 代次一份都不用重写**：`workspace.searchEverywhere` 收的是与
//!    `workspace.search` **同一个** `SearchRequest`（`rust/lithe-core/src/project/files.rs:272`），
//!    所以本模块直接用 [`crate::search_model`] 的 [`payload`] / [`parse_matches`] /
//!    [`QueryTimeline`]，只把命令名换成 [`EVERYWHERE_COMMAND`]。
//!    **本模块不写任何文件名匹配、模糊度或排序** —— 那是 Core 的
//!    `search_with_index`（`files.rs:190-268`），宿主只做"把结果画出来"。
//! 2. **浮层形状照 [`crate::command_palette`]**：同一个 `Dialog` + `Command` 组合，
//!    不引第三种浮层写法。`Command` 还白送搜索框、虚拟列表、`↑`/`↓`/`Enter`/`Esc`、
//!    行高亮与"搜索中"spinner（`gpui-component-0.7.0/src/command/state.rs:905-935`）。
//! 3. **打开文件复用既有通路**：`on_open` 回调由外壳注入，落到 `lithe_gpui_editor` 的
//!    `EditorPane::open`（Core 的路径命中 `line` 恒为 `null`，`files.rs:225`，所以走 `open`
//!    而不是 `open_at`）。本模块不碰编辑区。
//!
//! ### ⚠️ `filterable(false)`：不许第二套匹配
//!
//! `Command` 默认会拿 label 在本地再过一遍（`state.rs:332-338` 的 `item_matches`），
//! 那正是"第二套匹配"。上游给异步源留的开关是 `.filterable(false)`
//! （`command/command.rs:117-126`："Turn it off when an external source already answers the
//! query … every supplied item stays visible"），所以本模块**显式关掉本地过滤**：
//! 列表内容 = Core 给的行，一个字都不重排、不重筛。
//!
//! ## 与真源的有意差异（逐条）
//!
//! 1. **`Esc` 先清空查询、再关浮层**：`Command` 自己的语义（`state.rs:600-606`：
//!    "Escape clears a non-empty query first, and only then leaves the palette"），
//!    与 `crate::command_palette` 记的同一条一致。真源 macOS 是**一次 `Esc` 就关**
//!    （`SearchEverywhereView.swift:533-535`）—— 要改成那样只能绕开组件的 `Cancel` 处理器，
//!    代价（自建键上下文 + 与 Dialog 的 `Escape` 抢事件）大于收益，故保留组件语义。
//! 2. **`↑`/`↓` 到头环绕**：`Command::select_by` 用 `rem_euclid`（`state.rs:555-579`），
//!    真源是夹住（`:517-522`）。上游行为，不在本侧重写。
//! 3. **目录用尾部省略**：gpui 的 `truncate()` 是尾部省略号，真源是 `truncationMode(.middle)`
//!    （`SearchEverywhereView.swift:391`）。
//! 4. **没有「… more」截断提示**：真源在命中数顶到 `matchLimit` 时多画一行
//!    （`:316-318,331-334`），而仓里**没有**可复用的文案键（`quickOpen.*` 段没有 "more"），
//!    按"尽量不新增键"的口径不画这一行；上限本身仍由 Core 的 `maxResults` 生效。
//! 5. **没有 scope 标签行 / 相关度排序 / `/` 命令模式**：那些属于 Search Everywhere
//!    （另一个任务），Quick Open 只做 Files 一类。
//!
//! ## 诊断（stderr）
//!
//! `S1_QUICK_OPEN open|idle|issue|complete|stale|failed|cancel|open_file|close <fields>`。
//! 与 `S1_SEARCH` / `S1_COMMAND_PALETTE` 同族：交互路径在本仓库只能靠这些锚点做机器核对
//! （gpui 的键盘 / 浮层路径没有确定性单测，理由见 `crate::search_panel` 的同一段）。

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::h_flex;
use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui_kit::component::{ActiveTheme as _, WindowExt as _};
use gpui_kit::{
    App, AppContext as _, Context, DefiniteLength, ElementId, Entity, InteractiveElement as _,
    IntoElement, KeyBinding, ParentElement as _, Render, SharedString, Styled as _, Task, Window,
    div, rems,
};
use lithe_gpui_shared::icons::{FileIcon, file_icon};
use lithe_gpui_shared::{CoreClient, CoreError, CoreRequest, tr};
use serde_json::Value;

use crate::search_model::{
    EVERYWHERE_COMMAND, QueryTimeline, QuickOpenRow, QuickOpenState, SearchOptions, absolute,
    parse_matches, payload,
};

/// 诊断前缀（`eprintln!`，stderr）。与 `S1_SEARCH` / `S1_COMMAND_PALETTE` 一族同口径。
const TAG: &str = "S1_QUICK_OPEN";

/// 输入防抖窗口（毫秒）。值与真源一致：`SearchEverywhereView.swift:212`。
const DEBOUNCE_MS: u64 = 180;

/// 浮层宽 860：`SearchEverywhereView.swift:192` 的 `.frame(width: 860)`。
const PANEL_WIDTH: f32 = 860.;
/// 列表最大高 560：同文件 `:193` 的 `.frame(maxHeight: 560, alignment: .top)`。
const PANEL_MAX_HEIGHT: f32 = 560.;
/// 距窗口顶 84：同文件 `:195` 的 `.padding(.top, 84)`。
const PANEL_TOP_INSET: f32 = 84.;

/// 行内文件图标边长 14：同文件 `:372` 的 `LitheIcon(kind:size: 14)`。
///
/// 14 不在 gpui 的档位上，档位外一律写 `rems(P / 16.)`（rem base = 16px）。
const ROW_ICON_SPEC: f32 = 14.;

/// `Ctrl+P`：**全仓唯一**的 Quick Open 入口（键位与菜单项 dispatch 同一个 `Action`）。
pub fn install_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-p", OpenQuickOpen, None)]);
    cx.on_action(|_: &OpenQuickOpen, cx: &mut App| {
        let Some(window_handle) = cx.active_window() else {
            return;
        };
        let _ = window_handle.update(cx, |_, window, cx| {
            // 外壳才是 `root` / 编辑区的所有者，句柄与命令面板共用同一个
            // （`command_palette::shell`，理由见那里的 `SHELL` 文档：两者都要 `'static`）。
            let Some(shell) = crate::command_palette::shell().and_then(|shell| shell.upgrade())
            else {
                return;
            };
            let _ = shell.update(cx, |shell, cx| shell.open_quick_open(window, cx));
        });
    });
}

gpui_kit::actions!(lithe_workbench, [OpenQuickOpen]);

/// 打开浮层（**只能从事件回调或任务里调用**；`render` 阶段调 `window.open_dialog` 会 panic）。
///
/// 已经开着别的对话框时不叠第二层 —— 与 `open_command_palette` / `open_branch_panel`
/// 同一条口径，也免得 `Esc` 的"只关最上层"语义被两个面板搞乱。
pub(crate) fn open_dialog(quick_open: Entity<QuickOpen>, window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        QuickOpen::diagnose("open", "false reason=active_dialog");
        return;
    }
    // 两个入口（`Ctrl+P` 与菜单「转到 → 快速打开」）都只到这一个开浮层的点，
    // 所以这一行就是"浮层真的被要求打开"的锚点。
    QuickOpen::diagnose("open", "true");
    window.open_dialog(cx, move |dialog, window, _cx| {
        // `Dialog::width` / `margin_top` 是**固有方法**、只吃 `Pixels`（它们遮蔽了 `Styled`
        // 上的同名方法），所以按**当帧的** rem 基准换算（写死就是假 rem，见 `crate::rem_px`）。
        let rem = window.rem_size();
        dialog
            // 面板整个由 `Command` 自己画：搜索行、列表、圆角与边框都不要 Dialog 的。
            .close_button(false)
            .overlay(true)
            .overlay_closable(true)
            .keyboard(true)
            .width(crate::rem_px(rem, PANEL_WIDTH))
            .margin_top(crate::rem_px(rem, PANEL_TOP_INSET))
            .p_0()
            // 关闭的**唯一**收尾点：`Esc` / 点遮罩 / 点关闭按钮三条路都经过它
            // （`dialog/dialog.rs:598-607`）。在飞的 Core 查询在这里被取消。
            .on_close({
                let handle = quick_open.clone();
                move |_event, window, cx| {
                    let _ = handle.update(cx, |quick, cx| quick.close(window, cx));
                }
            })
            .content({
                let handle = quick_open.clone();
                move |content, _, _| content.p_0().gap_0().child(handle.clone())
            })
    });
}

/// 浮层的交互状态。
///
/// 与 `CommandPalette` / `SettingsDialog` 同一口径：`open_dialog` 的 builder **每帧重跑**，
/// 状态必须存在 `Entity` 里；这里持有 `CommandState`（查询框与高亮的归属方）与本模块自己的
/// 四态、代次、防抖任务。
pub struct QuickOpen {
    /// 工作区根：Core 的 `root`，也是"相对路径 → 绝对路径"的基准。
    root: PathBuf,
    /// 打开一个文件（由外壳注入，落到 `EditorPane::open`）。
    ///
    /// 存 `Rc<dyn Fn>` 而不是 `Entity<EditorPane>`：`Command` 的回调要求 `'static`，
    /// 而且"打开文件是编辑区的通路"这一层归属与 `crate::search_panel` 的 `on_open` 完全同源。
    on_open: Rc<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>,
    /// `Command` 的状态（查询框、可见项、高亮）。
    command: Entity<CommandState>,
    /// 当前查询串。`Command` 只在查询**真的变了**时回调 `on_query`（`state.rs:464-475`），
    /// 这里存一份用于"结果属于哪次查询"的诊断与防抖。
    query_text: SharedString,
    /// 四态（`idle` / `loading` / `ready` / `failed`）。
    state: QuickOpenState,
    /// 代次与在飞 `operationId`（stale 保护 + 取消句柄），与查找面板共用同一个类型。
    timeline: QueryTimeline,
    /// 防抖任务：**换掉旧的 = 取消旧的唤醒**（`Task` 一 drop 就取消）。
    debounce_task: Option<Task<()>>,
}

impl QuickOpen {
    /// 建浮层状态（`Ctrl+P` 那条回调里建一次，之后由 `Entity` 跨帧持有）。
    pub(crate) fn new(
        root: PathBuf,
        on_open: Rc<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            root,
            on_open,
            command: cx.new(|cx| CommandState::new(window, cx)),
            query_text: SharedString::default(),
            state: QuickOpenState::Idle,
            timeline: QueryTimeline::default(),
            debounce_task: None,
        }
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
        // 空查询**不发 Core 调用**：`search_everywhere` 对空查询直接回空（`files.rs:274-279`），
        // 发过去只是白跑一趟信封。
        if self.query_text.trim().is_empty() {
            self.debounce_task = None;
            self.state = QuickOpenState::Idle;
            self.set_loading(false, window, cx);
            Self::diagnose("idle", "query=empty");
            cx.notify();
            return;
        }
        // 排新的防抖；换掉旧任务 = 取消旧唤醒。**不**在这里置 loading：真机的 spinner
        // 只在查询真的发出去之后才转（`isSearchingEverywhere`）。
        self.debounce_task = Some(cx.spawn_in(window, async move |this, async_cx| {
            async_cx
                .background_executor()
                .timer(Duration::from_millis(DEBOUNCE_MS))
                .await;
            // 实体可能已经销毁（浮层已关）：`update_in` 返回 `Err` 时静默忽略，不 panic。
            let _ = this.update_in(async_cx, |quick, window, cx| quick.issue(window, cx));
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
        let previous = self.state.rows().to_vec();
        self.state = QuickOpenState::loading(query.clone(), &previous);
        // ⚠️ spinner 不只是装饰：`Command` 只在**非 loading 且零行**时才画空态
        // （`command/state.rs:989-993`），不置这一位的话"在路上"会被画成"没有匹配"。
        self.set_loading(true, window, cx);
        Self::diagnose(
            "issue",
            &format!("generation={generation} operation={operation_id} query={query}"),
        );
        cx.notify();

        // 同步的 Core 调用只在 `background_spawn` 里跑（UI 线程不做阻塞工作）。
        // 任务 detach 掉：代次闸门才是采纳判据（与 `crate::search_panel::issue` 同一条）。
        let background_query = query.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    // Quick Open 没有选项界面（真机 Windows 的 Quick Open 也没有大小写 / 全词 /
                    // 正则三个开关），所以固定默认选项；请求形状与查找面板共用 `payload`。
                    let request = CoreRequest::command(EVERYWHERE_COMMAND)
                        .with_payload(payload(&root, &background_query, &SearchOptions::default()))
                        .with_operation_id(operation_id);
                    CoreClient::new().execute(&request)
                })
                .await;
            let _ = this.update_in(cx, |quick, window, cx| {
                quick.adopt(generation, &query, result, window, cx)
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
            // 过期结果：**不采纳**（契约要求），也不改任何状态（连 spinner 都不动 ——
            // 它属于更年轻的那次查询）。留一行证据。
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
                Ok(matches) => QuickOpenState::ready(query, &matches),
                // 契约异常（缺 data / 形状不对）：给稳定 token，不给句子 ——
                // 宿主不自己编界面文案（见 `search_model::ParseError`）。
                Err(error) => QuickOpenState::failed(query, error.token()),
            },
            // Core 的失败原文是**数据**：`CoreError` 的 Display 给 `code: message`。
            Err(error) => QuickOpenState::failed(query, error.to_string()),
        };
        match &next {
            QuickOpenState::Failed { reason, .. } => Self::diagnose(
                "failed",
                &format!("generation={generation} query={query} reason={reason}"),
            ),
            _ => Self::diagnose(
                "complete",
                &format!(
                    "generation={generation} query={query} files={} state={}",
                    next.rows().len(),
                    // 空结果与"有命中"必须是两个可区分的状态词（契约要求分开）。
                    if next.rows().is_empty() {
                        "empty"
                    } else {
                        "ready"
                    }
                ),
            ),
        }
        self.state = next;
        self.timeline.settle(generation);
        self.set_loading(false, window, cx);
        cx.notify();
    }

    /// `Enter`（或鼠标点击某一行）：打开第 `row` 个文件 → 关浮层。
    ///
    /// ⚠️ **先关浮层再打开文件**：与 `CommandPalette::confirm` 同一个顺序（那里是"动作可能开
    /// 第二个对话框"，这里是"文件打开会改中央区"），免得遮罩下面的界面还没回过神。
    fn confirm(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(target) = self.state.rows().get(row).cloned() else {
            // 行号越界（结果刚换代）：什么都不做，不 panic、也不落到别的行。
            return;
        };
        window.close_dialog(cx);
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        self.debounce_task = None;
        self.set_loading(false, window, cx);
        let path = absolute(&self.root, &target.path);
        Self::diagnose("open_file", &format!("file={}", path.display()));
        (self.on_open)(path, window, cx);
    }

    /// 浮层关了（`Esc` / 点遮罩 / 点关闭按钮）：放弃在飞查询并复位。
    ///
    /// 三条关闭路径都汇到这里（`Dialog::on_close`），所以"关了之后 Core 还在扫"这件事
    /// 只有这一个收口：`timeline.abandon()` 换代 + 取消在飞 `operationId`。
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        self.debounce_task = None;
        self.state = QuickOpenState::Idle;
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
    /// **不在 UI 线程调**：它要拿 Core 的操作注册表锁（`cancellation.rs:84`）——
    /// 与 `crate::search_panel::cancel_in_background` 逐字同一条理由。
    fn cancel_in_background(operation_id: String, cx: &mut Context<Self>) {
        cx.background_spawn(async move {
            let requested = lithe_gpui_shared::cancel_operation(&operation_id);
            // `false` = 这个 id 没有在飞的操作（已经跑完了）—— 正常情况，不是失败。
            Self::diagnose(
                "cancel",
                &format!("operation={operation_id} requested={requested}"),
            );
        })
        .detach();
    }

    /// 一行 stderr 诊断（`S1_QUICK_OPEN <event> <fields>`）。
    fn diagnose(event: &str, fields: &str) {
        eprintln!("{TAG} {event} {fields}");
    }
}

impl Render for QuickOpen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 失败原因先取出来：空态闭包要求 `'static`，不能借用 `self`。
        let failure: Option<SharedString> = match &self.state {
            QuickOpenState::Failed { reason, .. } => Some(SharedString::from(reason.clone())),
            _ => None,
        };
        // 行也是先克隆出来（同一个理由）：`Command` 的 items 归 `CommandState`。
        let rows: Vec<QuickOpenRow> = self.state.rows().to_vec();
        let empty_query = self.state.query().trim().is_empty();

        Command::new(&self.command)
            .bordered(false)
            .w_full()
            .filterable(false)
            .placeholder(tr("lithe.quickOpen.searchFiles"))
            // 列表跟着 560 的面板一起长（`Command` 的列表默认上限 18.75rem = 300，
            // `command/command.rs:34`）。
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
                    // 空查询：什么都不画 —— 占位符已经说了"输入以搜索文件"，
                    // 真源也是"有查询才画结果区"（`SearchEverywhereView.swift:187`）。
                    return div().into_any_element();
                }
                // 有查询但没有匹配：真源 `quickOpen.noMatch`。
                div()
                    .py_6()
                    .w_full()
                    .text_center()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.quickOpen.noMatch"))
                    .into_any_element()
            })
            .on_query({
                let quick = cx.entity();
                move |query, window, cx| {
                    let _ = quick.update(cx, |quick, cx| quick.on_query(query, window, cx));
                }
            })
            .on_confirm({
                let quick = cx.entity();
                move |index, window, cx| {
                    // 未分组条目的 `IndexPath` 是 `section = 0` + `row` = **传入 items 的行号**
                    // （`command/state.rs:384`；`filterable(false)` 下本地过滤不改这两个坐标，
                    // 见 `Command::on_confirm` 的文档）。
                    let _ = quick.update(cx, |quick, cx| quick.confirm(index.row, window, cx));
                }
            })
    }
}

/// 让 [`QuickOpenRow`] 能被 `Command` 直接消费的适配。
///
/// 放在这里而不是 `search_model`：`CommandItem` 是**浮层**的零件，模型层只出数据
/// （与 `crate::command_palette` 的 `IntoCommandItem` 同一处置）。
trait IntoCommandItem {
    fn into_command_item(self) -> CommandItem;
}

impl IntoCommandItem for QuickOpenRow {
    fn into_command_item(self) -> CommandItem {
        let QuickOpenRow { path, name, parent } = self;
        // `label` 同时是搜索文本，但本浮层关了本地过滤（`.filterable(false)`），
        // 所以它只是"这一行长什么样"与无障碍名；真源渲染的就是文件名（`:376`）。
        let label = SharedString::from(name.clone());
        let item_name = SharedString::from(name);
        let item_parent = SharedString::from(parent);
        CommandItem::new().label(label).child(move |_, cx| {
            // 真源每行是「图标 + 名字 + 所在目录」（`SearchEverywhereView.swift:367-397`）。
            //
            // ⚠️ 元素 id 用**业务身份**（`quick-open-result-<path>`），不用下标：结果集换代时
            // 下标会把元素状态串到别的行上（`crates/workbench/src/search_panel.rs:560-561`
            // 同一口径）。⚠️ 行的**点击与选中** id 是上游 `Command` 自己的 `IndexPath`
            // （`command/state.rs:828`），本侧改不了，也不该改。
            //
            // ⚠️ 这个工厂是 `Fn`（测量 / 进视口 / 排版失效都会重跑，
            // `command/item.rs:90-104`），所以只能从捕获的 `SharedString` 克隆
            // （原子引用计数，很便宜），不能把它们 move 出去。
            h_flex()
                .id(ElementId::from(format!("quick-open-result-{path}")))
                .w_full()
                .min_w_0()
                // 24：真源结果行高（`SearchEverywhereView.swift:423`），档位内（`h_6`）。
                .h_6()
                // 8：同文件 `:367` 的 `HStack(spacing: 8)`。
                .gap_2()
                .items_center()
                .child(
                    // 文件类型图标走共享层（主题包真源 → Lucide 回落），
                    // 与资源管理器同一套查表（`shared/src/icons/mod.rs:100-139`）。
                    FileIcon::lucide(file_icon::lucide_fallback(item_name.as_ref()))
                        .with_theme_icon(item_name.as_ref(), cx)
                        .render(rems(ROW_ICON_SPEC / 16.), cx),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().foreground)
                        .child(item_name.clone()),
                )
                .child(
                    // 目录：占满剩余宽度、尾部省略（真源是 middle，见模块文档的差异第 3 条）。
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(item_parent.clone()),
                )
                .into_any_element()
        })
    }
}
