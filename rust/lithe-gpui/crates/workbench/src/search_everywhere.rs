//! Search Everywhere 浮层（菜单「视图 → 全局搜索」）：真源的**六个 scope**
//! —— All / Classes / Files / Symbols / **Actions** / Text —— 外加 `/` 命令模式。
//!
//! ## 真源：`SearchEverywhereView.swift`
//!
//! | 规格 | 值 / 行为 | 出处 |
//! | --- | --- | --- |
//! | 浮层宽 / 最大高 / 距顶 | 860 / 560 / 84 | `macos/Sources/Lithe/Views/Search/SearchEverywhereView.swift:192-195` |
//! | scope 标签行高 / 标签高 / 标签左右内衬 / 标签间距 | 40 / 38 / 11 / 2 | 同文件 `:254` / `:228` / `:227` / `:219` |
//! | 活动标签 | 主文本色 + 半粗 + 底部 2px 强调色下划线 | 同文件 `:225-232` |
//! | 输入框行高 | 44 | 同文件 `:282` |
//! | 结果行高 / 行内间距 | 24 / 8 | 同文件 `:423` / `:367` |
//! | 结果行内容 | 图标 + 名字（`symbolName ?? 文件名`）+ `:行号` + 所在目录 + 预览（只有 content） | 同文件 `:364-397` |
//! | **动作行内容** | **bolt 图标（11pt，warning 色，宽 16）+ title + subtitle + 右侧 group（+ keyEquivalent）** | 同文件 `:457-495` |
//! | 搜索中显示 spinner | `feature.isSearchingEverywhere` | 同文件 `:241-243` |
//! | scope ↔ Core `kind`（五个 Core 档） | 见 `crate::search_model::SearchEverywhereScope` 的表 | 同文件 `:120-137` |
//! | `Actions` 档 ↔ 宿主命令目录 | `crate::command_palette::CommandId::ALL`（**22 条**，真源子集） | 同文件 `:138-139` |
//! | `/` 命令模式的判定与查询串 | `query.hasPrefix("/")` → `dropFirst()`；结果源判在 scope **之前** | 同文件 `:20-26,59-63` |
//! | 命令模式空态 | "No matching commands" | 同文件 `:77-78` |
//! | 空查询时输入框里的提示 | "Type / to see commands" | 同文件 `:266-270` |
//! | Tab / `←` / `→` **环绕**切 scope | `(i + offset + n) % n` | 同文件 `:523-529,542-547` |
//! | 防抖 | 180ms | 同文件 `:212` |
//! | 结果上限 | `EVERYWHERE_MAX_RESULTS`（1000，推导见该常量） | Core `project/files.rs:326-332` |
//! | 文案 | scope 名 / 「X 中没有匹配项」/ 命令模式那两句都取真源既有字面量 | `macos/Resources/zh-Hans.lproj/Localizable.strings:886,1139,1169,48` |
//!
//! ## 行的两个来源（本切片的关键）
//!
//! | 档 | 行从哪来 | 真源出处 |
//! | --- | --- | --- |
//! | `All` / `Classes` / `Files` / `Symbols` / `Text` | Core `workspace.searchEverywhere` 的命中投影（`crate::search_model::everywhere_rows`） | `SearchEverywhereView.swift:120-137` |
//! | `Actions` 档 / `/` 命令模式 | **宿主命令目录**（`ShellWorkspace::action_catalog` → `crate::search_model::action_rows`） | 同文件 `:59-63,138-139` |
//!
//! ⚠️ 第二档**一个 Core 请求都不发**。判据是 `crate::search_model` 里两个纯函数
//! （`uses_command_catalog` / `issues_core_request`），浮层的查询（`on_query`）、
//! 防抖到期（`issue`）、切 scope（`set_scope`）三条路径**都只看它** ——
//! 所以"Actions 档不发 Core"只有一个判据、一处可测，不靠"记得别调"。
//!
//! ## 复用（本任务最重要的工程要求）
//!
//! 1. **请求 / 解析 / 代次一份都不用重写**：`workspace.searchEverywhere` 收的是与
//!    `workspace.search` **同一个** `SearchRequest`（`rust/lithe-core/src/project/files.rs:272`），
//!    所以本模块直接用 `crate::search_model` 的 `payload_with_limit` / `parse_matches` /
//!    `QueryTimeline`，只把命令名换成 `EVERYWHERE_COMMAND`、把 `maxResults` 换成
//!    `EVERYWHERE_MAX_RESULTS`（**同一个** `json!`，见 `payload_with_limit` 的文档）。
//!    **本模块不写任何匹配、模糊度或排序** —— 那是 Core 的 `search_with_index`
//!    （`files.rs:190-268`）与符号扫描（`files.rs:296-320`），宿主只做"把结果画出来"。
//! 2. **五个 Core scope 的投影是纯函数**，落在 `crate::search_model::everywhere_rows`
//!    （可确定性单测）；Files 那一档直接**复用 Quick Open 的 `quick_open_rows`** ——
//!    Quick Open 与 Search Everywhere 的 Files scope 是同一份投影，不写第二份。
//! 3. **动作档不再起第二份目录、也不另写匹配器**：候选直接取命令面板那张表
//!    （`ShellWorkspace::action_catalog`，与命令面板同一个 `command_action` 投影），
//!    过滤用 `crate::search_model::action_matches`（逐条照真源 `LitheAction.matches`，
//!    见那个函数的文档：上游两套匹配器一套不认识命令目录、一套是私有 `fn`）。
//!    行型复用命令面板的 `CommandAction`，不为这一档再造一种行。
//! 4. **浮层形状照 [`crate::quick_open`]**：同一个 `Dialog` + `Command` 组合，
//!    不引第三种浮层写法。`Command` 自带搜索框、虚拟列表、`↑`/`↓`/`Enter`/`Esc`、
//!    行高亮与"搜索中"spinner（`gpui-component-0.7.0/src/command/state.rs:905-941`）。
//!    ⚠️ `Command` 的本地过滤**显式关掉**（`.filterable(false)`，
//!    `command/command.rs:117-126`）：列表内容 = Core 给的行或宿主目录过滤后的行，
//!    一个字都不再筛（`CommandItem::matches` 是**子串**匹配，与真源的子序列规则不同）。
//! 5. **打开结果复用既有通路**：`on_open` 回调由外壳注入（与查找面板同一个签名），
//!    落到 `lithe_gpui_editor` 的 `EditorPane::open`（行号为 `None`）或
//!    `EditorPane::open_at`（有行号，`origin_kind = "search-everywhere"`）。本模块不碰编辑区。
//! 6. **执行动作复用唯一执行点**：动作行没有路径，所以 `on_action` 只报"用户选了
//!    `CommandId::id()` 这条"，由外壳落到 `ShellWorkspace::run_command_id`
//!    （菜单与命令面板走的也是它）—— 与 `on_open` 同一口径：低层只报"选了什么"。
//!
//! ## 与真源的有意差异（逐条）
//!
//! 1. **`Actions` 档是子集**：真源 `LitheCommandCatalog.swift:129-182` 有 **39** 条命令、
//!    `LitheActionRegistry.actions(for:)` 列 **35** 条动作；本侧的宿主命令目录是
//!    `CommandId` 的 **22** 个变体（缺的能力在 gpui 侧还不存在）。所以这一档**只可能显示子集**，
//!    不假装与真源等价。
//! 2. **动作行的图标用动作自己的图标，不是统一的 bolt**：真源给每一行动作画同一个
//!    `bolt.fill`（`LitheAction` 不携带图标，`:457-465`）；本侧的动作来自宿主命令目录，
//!    每条自带与命令面板**同一个**字形，所以直接用它 —— 同一个动作在两个入口里视觉同源，
//!    也免得为一个字形再造一份映射。行高 / 字号 / 内衬 / 右侧分类列都照真源 `:457-495`。
//! 3. **没有 keyEquivalent 列**：真源动作行右侧画快捷键（`:480-484`），本侧没有
//!    "命令 id → 可显示快捷键文本"的通路（键位是 `cx.bind_keys` 里登记的行为，不可反查），
//!    硬编一张表就是第二份键位真源 —— 所以这一列留空，等宿主有了键位查询接口再补。
//! 4. **动作的 group 列是宿主自己的分类**（`Category::{Settings, View}`，命令面板既有），
//!    不是真源的五个 `LitheActionGroup`（run / navigation / window / project / history，
//!    `LitheAction.swift:3-9`）：扩分类要动命令面板的选择面与图标映射，不在本切片。
//! 5. **没有相关度重排**：真源 All 会按 `SearchRelevance.score` 重排
//!    （`SearchEverywhereView.swift:11-90,158-170`），而 Core 的响应里**没有** relevance 字段
//!    （`protocol/contracts.rs:94-105` 的 `SearchMatch` 只有 `kind/path/line/preview/symbolName`），
//!    所以在宿主里重排就等于写第二套相关度算法。本模块**沿用 Core 返回的固定顺序**
//!    （它本身确定：file → types → symbols → content，`files.rs:326-330`）。
//!    这是本切片与真源**最大的一条**有意差异，也是"重排等 Core 给字段（main 那一侧）"的落点。
//! 6. **`Tab` / `Shift+Tab` 环绕切 scope，`←` / `→` 不绑**：真源挂了一个 NSEvent 全局监听，
//!    连输入框里的左右键都吞掉（`:512-540`）。gpui 侧做不到这一点：查询框的 `←`/`→` 由
//!    上游 `Input` **无条件**绑定并处理（`gpui-base-0.7.0/src/input/base/state.rs:179-180,4454-4457`），
//!    动作一旦被输入框处理就不再冒泡；而 `Tab` / `Shift+Tab` 在 `Input` 里只对**多行**
//!    输入注册（同文件 `:4447-4450`），单行查询框不处理，所以能安全接管
//!    （本模块用 `.key_context(SCOPE_KEY_CONTEXT)` + 两条 `KeyBinding`，比 gpui-base 根节点
//!    的 focus traversal 绑定 `root.rs:16-17` 更深，因此优先级更高）。
//!    `←`/`→` 留在输入框里做**光标移动** —— 那对单行查询框是更有用的行为。
//! 7. **目录用尾部省略**：gpui 的 `truncate()` 是尾部省略号，真源是 `truncationMode(.middle)`
//!    （`:391`）—— 与 [`crate::quick_open`] 记的同一条。
//! 8. **不画每行右侧的「在文件管理器中显示」按钮与 Maven 模块标签**：前者要宿主级的
//!    reveal 通路（本 crate 没有），后者要模块归属（Maven 上下文，另一个子系统）。
//!    两处都不影响"这一行是什么"（名字 / 行号 / 目录 / 预览都在）。
//! 9. **`Esc` 先清空查询、再关浮层**：`Command` 自己的语义（`command/state.rs:600-606`），
//!    与 [`crate::quick_open`] / [`crate::command_palette`] 记的同一条；真源是一按就关。
//! 10. **scope 标签行没有用上游 `TabBar`**：它的 `on_click` 回调只给**下标**
//!    （`gpui-component-0.7.0/src/tab/tab_bar.rs:171-179`），标签自身的元素 id 由上游按下标发
//!    （`:506-537`），与"`ElementId` 用业务身份"这条仓库口径冲突；而本侧只有六个固定标签，
//!    自己画能同时满足业务 id 与真源的 40/38/2px 三个数。
//! 11. **`/` 命令模式下真源仍会发一次空查询**（`:207-211` 的 `await search("", …)`），
//!    本侧**一次都不发**：Core 对空查询直接回空（`files.rs:274-279`），发出去只是白跑一次
//!    跨进程往返；这与本浮层既有的"空查询不发 Core"口径一致。
//!
//! ## 诊断（stderr）
//!
//! `S1_SEARCH_EVERYWHERE open|scope|actions|idle|issue|complete|stale|failed|cancel|open_result|run_action|close <fields>`。
//! 与 `S1_SEARCH` / `S1_QUICK_OPEN` 同族：gpui 的浮层与键盘路径在本仓库**没有**确定性单测
//! （要真窗口 + 真键事件，而 `ShellWorkspace::new` 会起 JDTLS 预备 / 会话恢复 / 索引 warm），
//! 所以交互路径的机器证据只有这些锚点；能确定性验证的部分全在
//! `crate::search_model` 的纯函数单测里。
//!
//! ⚠️ `scope` / `actions` / `complete` 三行都带 `core=`（`true` / `false`）：这就是
//! "Actions 档不发 Core"在真机上的可 grep 证据 —— 切到 Actions 或打 `/` 之后不再出现
//! `issue` 行，而 `actions … core=false` 每敲一次键都会打一行。

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
use lithe_gpui_shared::icons::{FileIcon, file_icon, idea, idea_icon_image, idea_icon_svg_px};
use lithe_gpui_shared::{CoreClient, CoreError, CoreRequest, tr};
use serde_json::Value;

use crate::command_palette::{CommandAction, CommandId};
use crate::search_index::SearchIndexReadiness;
use crate::search_model::{
    COMMANDS_EMPTY_KEY, EVERYWHERE_COMMAND, EVERYWHERE_MAX_RESULTS, EverywhereRow, EverywhereState,
    QueryTimeline, SearchEverywhereScope, SearchOptions, absolute, action_rows, command_query,
    is_command_query, issues_core_request, parse_matches, payload_with_limit, uses_command_catalog,
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

/// 动作行的 bolt 字形边长 11：同文件 `:463` 的 `.font(.system(size: 11, weight: .semibold))`。
///
/// 档位外 ⇒ `rems(11 / 16.)`（**不是** `/ 4.`：rem 基准是 16px）。
const ACTION_ICON_SPEC: f32 = 11.;
/// 动作行图标槽宽 16：同文件 `:465` 的 `.frame(width: 16)`。档位内（正好 1rem）。
const ACTION_ICON_SLOT_SPEC: f32 = 16.;

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
///
/// ⚠️ 这里**不缓存动作目录**：它归 `ShellWorkspace`（朝向 / 主题会变），每帧现取
/// （与 `CommandPalette::render` 同一处置），缓存一份就会过期。
pub struct SearchEverywhere {
    /// 工作区根：Core 的 `root`，也是"相对路径 → 绝对路径"的基准。
    root: PathBuf,
    /// 打开一条结果（由外壳注入；落到 `EditorPane::open` / `open_at`）。
    ///
    /// 第二个参数 = 要跳的行号（真源 `result.line`）；`None` 就是只打开文件。
    on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static>,
    /// 执行一条**动作**（由外壳注入；落到 `ShellWorkspace::run_command_id`）。
    ///
    /// 动作行没有路径，所以这里只报"用户选了哪一条 `CommandId`"——
    /// 与 `on_open` 同一口径：低层只报"选了什么"，怎么执行是外壳的事
    /// （真源 `performSelectedItem` 的 `case .action(let action): performAction(action)`，
    /// `SearchEverywhereView.swift:549-555`）。
    on_action: Rc<dyn Fn(CommandId, &mut Window, &mut App) + 'static>,
    /// `Command` 的状态（查询框、可见项、高亮）。
    command: Entity<CommandState>,
    /// 当前 scope（**本地状态**：切它不再打一次 Core，理由见 `EverywhereState`）。
    scope: SearchEverywhereScope,
    /// 当前查询串（`Command` 只在查询**真的变了**时回调 `on_query`）。
    query_text: SharedString,
    /// 四态（`idle` / `loading` / `ready` / `failed`）。
    ///
    /// ⚠️ 只描述 **Core 那一侧**：`Actions` 档 / `/` 命令模式不用它（见 `EverywhereState`）。
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
    /// `on_open`：打开一条 Core 结果；`on_action`：执行一条动作（`Actions` 档 / `/` 模式）。
    /// 两个回调都由外壳注入 —— 浮层不认识编辑区，也不认识工作台的命令执行点。
    ///
    /// `search_index_readiness`：**打开这一刻**的索引就绪状态（外壳从 `search_index` 读，
    /// 唯一 owner 是它）。
    pub(crate) fn new(
        root: PathBuf,
        on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static>,
        on_action: Rc<dyn Fn(CommandId, &mut Window, &mut App) + 'static>,
        search_index_readiness: SearchIndexReadiness,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            root,
            on_open,
            on_action,
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

    /// 这一帧行的**来源**：`Some(查询串)` = 宿主命令目录（`Actions` 档或 `/` 命令模式），
    /// `None` = Core 命中。
    ///
    /// 真源把两条来源分开判（`SearchEverywhereView.swift:59-73` 的
    /// `SearchEverywhereResultSource.init`）：**先判 `/` 命令模式**（`commandQuery` 非 nil
    /// 就直接走 `.commands`，与当前 scope 无关），再按 scope 落到 `.actions(...)`
    /// 或工作区那几档。所以 `/` 覆盖 scope 这件事在真源里就是这一句。
    fn action_query(&self) -> Option<String> {
        if !uses_command_catalog(self.scope, &self.query_text) {
            return None;
        }
        // `/` 模式下取斜杠**之后**的那一段（`:22` 的 `dropFirst()`），否则就是查询串本身。
        Some(
            command_query(&self.query_text)
                .unwrap_or(&self.query_text)
                .to_string(),
        )
    }

    /// 这一帧要画的行 —— **行的唯一来源**：`render` 画它、`confirm` 按同一顺序取第 `row` 条、
    /// 诊断数它。三处必须经过同一个函数，否则行号会错位（与 `CommandPalette` 的
    /// "`command_actions` 是唯一行序真源"同一条纪律）。
    ///
    /// - 动作目录：宿主命令目录按 [`action_rows`] 过滤（`Actions` 档空查询时**零行** ——
    ///   真源"有查询才画结果区"，`SearchEverywhereView.swift:187`）；
    /// - Core：当前 scope 的命中投影（切 scope 只换投影，不重发请求）。
    fn items(&self, cx: &App) -> Vec<EverywhereItem> {
        match self.action_query() {
            Some(query) if !self.query_text.trim().is_empty() => {
                action_rows(Self::action_catalog(cx), &query)
                    .into_iter()
                    .map(EverywhereItem::Action)
                    .collect()
            }
            // `Actions` 档 + 空查询：真源连结果区都不画（`:187`）。
            Some(_) => Vec::new(),
            None => self
                .state
                .rows(self.scope)
                .into_iter()
                .map(EverywhereItem::Result)
                .collect(),
        }
    }

    /// 宿主命令目录（`Actions` 档 / `/` 命令模式的数据源）。
    ///
    /// 每帧现取（与 `CommandPalette::render` 同一处置）：动作表归 `ShellWorkspace` ——
    /// 终端 / Maven / 状态栏的朝向与主题深浅由它算，浮层不缓存一份会过期的副本。
    /// 工作台还没登记（或窗口正在关）时给空表：那时浮层也不会被画出来。
    fn action_catalog(cx: &App) -> Vec<CommandAction> {
        crate::command_palette::shell()
            .and_then(|handle| handle.upgrade())
            .map(|shell| shell.read(cx).action_catalog(cx))
            .unwrap_or_default()
    }

    /// 撤掉在飞的 Core 查询：取消请求 + 丢掉防抖任务 + 收起 spinner。
    ///
    /// 三条路径共用它：进 `Actions` 档、`Enter` 之后收尾、浮层关闭。
    fn abandon(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        self.debounce_task = None;
        self.set_loading(false, window, cx);
    }

    /// 查询串变了（`Command` 的 `on_query`，只在**真的变了**时来）。
    ///
    /// 三条分支，判据都在 `crate::search_model` 的纯函数里：
    ///
    /// 1. **动作目录**（`Actions` 档 / `/` 命令模式）：**一个 Core 请求都不发**，
    ///    在飞的请求撤掉，只更新查询串与列表；
    /// 2. 空查询（Core 档）：不发 Core（`search_everywhere` 对空查询直接回空，`files.rs:274-279`）；
    /// 3. 其余：旧查询**立刻作废**（换代 + 尽力取消）后重排 180ms 防抖。
    ///
    /// ⚠️ 分支 2/3 里**状态在防抖窗口里不动**：上一次的结果继续留在屏幕上（真机同样不清空
    /// `visibleItems`，`SearchEverywhereView.swift:302-328`）。
    fn on_query(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.query_text.as_ref() == query {
            return;
        }
        self.query_text = SharedString::from(query);
        // 动作目录这一档：**这一档不发 Core**（判据见 `uses_command_catalog`）。
        // 动作列表是纯投影，每帧现算，所以这里只撤在飞的请求 + 打一行诊断。
        if uses_command_catalog(self.scope, &self.query_text) {
            self.abandon(window, cx);
            // 触到 Core 的状态**不动**（切回 Core 档时同一个查询串能直接复用）。
            let rows = self.items(cx).len();
            Self::diagnose(
                "actions",
                &format!(
                    "commands={} scope={} query={query} rows={rows} core=false",
                    is_command_query(&self.query_text),
                    self.scope.token()
                ),
            );
            cx.notify();
            return;
        }
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
        self.schedule(window, cx);
        cx.notify();
    }

    /// 排一次 180ms 防抖查询（`on_query` 与"从动作档切回 Core 档"共用）。
    fn schedule(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.debounce_task = Some(cx.spawn_in(window, async move |this, async_cx| {
            async_cx
                .background_executor()
                .timer(Duration::from_millis(DEBOUNCE_MS))
                .await;
            // 实体可能已经销毁（浮层已关）：`update_in` 返回 `Err` 时静默忽略，不 panic。
            let _ = this.update_in(async_cx, |overlay, window, cx| overlay.issue(window, cx));
        }));
    }

    /// 真的发一次查询（防抖到期 / 从动作档切回 Core 档）。
    fn issue(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // ⚠️ 闸门：防抖窗口里可能已经切到 `Actions` 档或敲了 `/`
        // （那时 `on_query` 已经把在飞的请求撤掉），**这一档一个 Core 请求都不许发**。
        // 判据与 `on_query` 用的是同一个纯函数，所以两处不会漂移。
        if !issues_core_request(self.scope, &self.query_text) {
            Self::diagnose(
                "issue",
                &format!(
                    "skipped=true core=false scope={} commands={}",
                    self.scope.token(),
                    is_command_query(&self.query_text)
                ),
            );
            return;
        }
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
                // 每个 scope 各自的可画行数都报出来：B10（符号饥饿）是不是真的绕开了，
                // 靠的就是这一行里 `symbols=` 不为 0。
                // ⚠️ `actions=` **恒为 0**：动作行不来自 Core（见 `everywhere_rows` 的
                // `Actions` 分支），它的行数在 `actions` 那条诊断里报。
                let counts = SearchEverywhereScope::ALL
                    .iter()
                    .map(|scope| format!("{}={}", scope.token(), next.rows(*scope).len()))
                    .collect::<Vec<_>>()
                    .join(" ");
                Self::diagnose(
                    "complete",
                    &format!(
                        "generation={generation} query={query} matches={} core=true {counts}",
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
    /// 三件事，判据都是 `crate::search_model` 的纯函数：
    ///
    /// 1. **切 Core 档之间不打第二次 Core**：scope 是本地投影口径（真源的搜索任务只按查询串
    ///    作 key，`SearchEverywhereView.swift:207`），五个 Core scope 共用同一份命中；
    /// 2. **进 `Actions` 档撤掉在飞的 Core 查询**：这一档不发 Core（`uses_command_catalog`），
    ///    留着在飞的请求既没人要、又会在稍后回来时白白过一遍解析；
    /// 3. **从 `Actions` 档切回 Core 档时按需补一次**：手上的命中不属于当前查询串
    ///    （在 Actions 里改过字，或进 Actions 时把在飞的请求撤了）才补
    ///    （`issues_core_request` + `EverywhereState::is_ready_for`）；同一个查询串且已经有
    ///    `ready` 结果就直接复用 —— 与真源"任务不因切 scope 重跑"同效。
    ///
    /// ⚠️ `/` 命令模式**覆盖 scope**（真源 `:59-63` 把 `commandQuery` 判在 scope 之前），
    /// 那时换标签不改数据源，所以上面 2/3 两条都不做。
    ///
    /// 高亮回到第一行这件事不在这里手工算：`Command` 在模型重装时若旧的选中项不在新集合里
    /// 会自己 `reset_selection()`（`gpui-component-0.7.0/src/command/state.rs:200-220`），
    /// 而切 scope 必然换掉整个行集合 —— 与真源 `onChange(of: scope) { selectedIndex = 0 }`
    /// （同文件 `:206`）同一效果，且不用自己去操作上游的 `IndexPath`。
    fn set_scope(
        &mut self,
        scope: SearchEverywhereScope,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.scope == scope {
            return;
        }
        let previous = self.scope;
        self.scope = scope;
        if !is_command_query(&self.query_text) {
            if !scope.is_core_backed() {
                self.abandon(window, cx);
            } else if previous == SearchEverywhereScope::Actions
                && issues_core_request(scope, &self.query_text)
                && !self.state.is_ready_for(&self.query_text)
            {
                self.issue(window, cx);
            }
        }
        let rows = self.items(cx).len();
        Self::diagnose(
            "scope",
            &format!(
                "scope={} rows={rows} core={} commands={}",
                scope.token(),
                scope.is_core_backed(),
                is_command_query(&self.query_text)
            ),
        );
        cx.notify();
    }

    /// 环绕切一格（`Tab` / `Shift+Tab` → [`SearchEverywhereScope::moved`]）。
    fn on_scope_next(&mut self, _: &NextScope, window: &mut Window, cx: &mut Context<Self>) {
        let next = self.scope.moved(1);
        self.set_scope(next, window, cx);
    }

    /// 反向环绕一格。
    fn on_scope_prev(&mut self, _: &PreviousScope, window: &mut Window, cx: &mut Context<Self>) {
        let previous = self.scope.moved(-1);
        self.set_scope(previous, window, cx);
    }

    /// `Enter`（或鼠标点击某一行）：打开第 `row` 条 → 关浮层。
    ///
    /// 行来自 [`Self::items`]（**唯一行序真源**，与 `render` 画的是同一份），所以两种行型
    /// 各走各的出口：
    ///
    /// - `Result`（Core 命中）→ `on_open`（打开文件 / 跳行）；
    /// - `Action`（宿主命令目录）→ `on_action`（执行 `CommandId`）——
    ///   真源 `performSelectedItem` 同样是 `case .action: performAction(action)`
    ///   （`SearchEverywhereView.swift:549-555`）。
    ///
    /// ⚠️ **先关浮层再放行动作**：被选中的动作里就有「命令面板 / 全局搜索 / 打开设置」这类
    /// 会再开一个对话框的，而 `open_*` 都有"已经开着对话框就不叠第二层"的保护
    /// （与 `CommandPalette::confirm` 是同一条理由）。
    fn confirm(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.items(cx).into_iter().nth(row) else {
            // 行号越界（结果刚换代 / 刚切了 scope）：什么都不做，不 panic、也不落到别的行。
            return;
        };
        window.close_dialog(cx);
        self.abandon(window, cx);
        match item {
            EverywhereItem::Action(action) => {
                // 诊断行给的是 `CommandId::id()`（同时是搜索关键词，见 `CommandId` 的文档）。
                Self::diagnose(
                    "run_action",
                    &format!(
                        "id={} scope={} commands={}",
                        action.id.id(),
                        self.scope.token(),
                        is_command_query(&self.query_text)
                    ),
                );
                (self.on_action)(action.id, window, cx);
            }
            EverywhereItem::Result(target) => {
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
        }
    }

    /// 浮层关了（`Esc` / 点遮罩 / 点关闭按钮）：放弃在飞查询并复位。
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.abandon(window, cx);
        self.state = EverywhereState::Idle;
        self.scope = SearchEverywhereScope::All;
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
            .on_click(cx.listener(move |this, _, window, cx| this.set_scope(scope, window, cx)))
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
        // 行也是先取出来（同一个理由）：`Command` 的 items 归 `CommandState`。
        // ⚠️ 行的来源只有 [`Self::items`] 一处（`confirm` 按同一顺序取第 row 条），
        // 所以这里不自己再判一次 scope。
        let rows: Vec<EverywhereItem> = self.items(cx);
        // 空查询的判据分两种行型：Core 档看**手上那份命中**属于哪个查询串（切回 Core 档、
        // 请求还没回来时不能把"在路上"画成"没有匹配"，见 `EverywhereState`）；
        // 动作档看**查询框里的字** —— 真源是 `hasQuery`（`:172-174`），有查询才画结果区
        // （`:187`）。`Actions` 档空查询 ⇒ 零行 + 这条判据 ⇒ 结果区整个不画。
        let empty_query = if uses_command_catalog(self.scope, &self.query_text) {
            self.query_text.trim().is_empty()
        } else {
            self.state.query().trim().is_empty()
        };
        // 空态分句：`/` 命令模式有自己那一句（真源 `:77-78`），其余按 scope（同文件 `:79-90`）。
        let empty_key = if is_command_query(&self.query_text) {
            COMMANDS_EMPTY_KEY
        } else {
            self.scope.empty_key()
        };
        // 输入框里那句提示只在查询为空时可见。真源在空查询时叠的正是 "Type / to see commands"
        // （`:266-270`），它是 `/` 命令模式唯一的可发现入口，所以这一档用它；
        // 非空时那段文字本来就不会被画出来（placeholder 的语义），保留既有键只为不改动
        // 查找面板那条共用文案的接线。
        let placeholder = if self.query_text.is_empty() {
            tr("lithe.searchEverywhere.typeSlash")
        } else {
            tr("lithe.search.gpui.queryPlaceholder")
        };
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
                    .placeholder(placeholder)
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
                            // 空查询：什么都不画 —— 真源也是"有查询才画结果区"
                            // （`SearchEverywhereView.swift:187`），
                            // 而 `/` 后面什么都没写时查询串是 `/`（非空），所以会列出全部命令。
                            return div().into_any_element();
                        }
                        // 有查询但这一档没有命中：真源按 scope 分句
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
                            // 不改这两个坐标）。它同时也是"当前这一档的第 row 行"
                            // —— `items` 是唯一行序真源，两种行型共用这一条。
                            let _ = overlay
                                .update(cx, |overlay, cx| overlay.confirm(index.row, window, cx));
                        }
                    }),
            )
    }
}

/// 结果列表里的一行：**Core 命中**的投影 / **宿主命令目录**的动作。
///
/// 与真源的 `SearchItem { case result / case action }` 同构
/// （`SearchEverywhereView.swift:115-118`）：两种行型在**同一个** `Command` 列表与同一套
/// 键盘导航里（真源 `itemRow` 也是二选一，`:345-353`），不另起一套浮层、也不建第二份目录。
/// 两者的身份、形状、来源都不同，所以它们是两个变体而不是一种行的两种填充。
enum EverywhereItem {
    /// Core 命中（五个 Core scope）。
    Result(EverywhereRow),
    /// 宿主命令目录里的一条动作（`Actions` 档 / `/` 命令模式）。
    Action(CommandAction),
}

/// 让 [`EverywhereItem`] 能被 `Command` 直接消费的适配。
///
/// 放在这里而不是 `search_model`：`CommandItem` 是**浮层**的零件，模型层只出数据
/// （与 [`crate::quick_open`] 的同一处置）。
trait IntoCommandItem {
    fn into_command_item(self) -> CommandItem;
}

impl IntoCommandItem for EverywhereItem {
    fn into_command_item(self) -> CommandItem {
        match self {
            Self::Result(row) => row.into_command_item(),
            Self::Action(action) => action.into_command_item(),
        }
    }
}

/// 动作行的适配（真源 `actionRow`，`SearchEverywhereView.swift:457-495`）。
///
/// 画的是「bolt 图标 + title + subtitle +（撑开的空白）+ group」
/// （真源还有一列 keyEquivalent，本侧没有那条通路，见模块文档的差异）。
/// `Enter` 之后怎么执行**不在这里**：`on_confirm` → [`SearchEverywhere::confirm`] → `on_action`。
impl IntoCommandItem for CommandAction {
    fn into_command_item(self) -> CommandItem {
        // 分类名与机器 id 在 `CommandItem::label` 之外单独取（`Command` 的本地过滤已关，
        // 所以 keywords 不影响本浮层的匹配 —— 匹配归 `search_model::action_matches`）。
        let group = self.category_label();
        let label = self.label.clone();
        let description = self.description.clone();
        let identity = self.id.id();
        CommandItem::new().label(label.clone()).child(move |_, cx| {
            // ⚠️ 元素 id 用**业务身份**（`CommandId::id()` 是稳定 token），不用下标：
            // 结果集换代 / 切档时下标会把元素状态串到别的行上
            // （`crate::search_panel` 与 `crate::quick_open` 同一口径）。
            // ⚠️ 这个工厂是 `Fn`（测量 / 进视口 / 排版失效都会重跑），所以只能从捕获的
            // `SharedString` 克隆（原子引用计数，很便宜），不能把它们 move 出去。
            h_flex()
                .id(ElementId::from(format!(
                    "search-everywhere-action-{identity}"
                )))
                .w_full()
                .min_w_0()
                // 24：真源动作行高（`SearchEverywhereView.swift:489`），档位内（`h_6`）。
                .h_6()
                // 8：同文件 `:461` 的 `HStack(spacing: 8)`。
                .gap_2()
                .items_center()
                .child(
                    // 真源 `:462-465` 的 `Image(systemName: "bolt.fill")`：11pt、warning 色，
                    // 外面套一个宽 16 的槽（`.frame(width: 16)`），标签才不会随字形宽度左右跳。
                    // 宿主图标集里的 `lightning` 就是那个 bolt 字形；单色 svg 管线跟主题色，
                    // 所以显式给 warning（与真源同一处着色）。
                    div()
                        .flex_shrink_0()
                        .w(rems(ACTION_ICON_SLOT_SPEC / 16.))
                        .items_center()
                        .justify_center()
                        .child(
                            idea_icon_svg_px(
                                &idea::LIGHTNING_ICON,
                                cx,
                                rems(ACTION_ICON_SPEC / 16.),
                            )
                            .text_color(cx.theme().warning),
                        ),
                )
                // 标题：真源 `.fixedSize(horizontal: true, vertical: false)`（`:466-470`）
                // ⇒ 不压缩、不省略。
                .child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().foreground)
                        .child(label.clone()),
                )
                .child(
                    // 次级色描述：真源只有 `.lineLimit(1)`（`:471-474`），可以压缩
                    // ⇒ 这里给它 `min_w_0` + 尾部省略。
                    div()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(description.clone()),
                )
                // 真源 `:475` 的 `Spacer(minLength: 12)`：把分类列推到右端。
                .child(div().flex_1().min_w_0())
                .child(
                    // 右侧的分类列（真源 `:476-479`）；keyEquivalent 那一列本侧没有，见模块文档。
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(group.clone()),
                )
                .into_any_element()
        })
    }
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

#[cfg(test)]
mod tests {
    //! 浮层的**交互层**测试（`D26` 的 UI 测试层在 `tests/ui.rs`，这里是它在 crate 内的
    //! 第一条：只有 crate 内部才看得见 `state` / `debounce_task` 这些私有状态）。
    //!
    //! 只测一件事，但它正是本切片最容易错的地方：**`Actions` 档与 `/` 命令模式一个 Core
    //! 请求都不发**。判据不看日志，而是浮层自己的 Core 状态机 —— `CoreClient::execute`
    //! 在整份文件里**唯一**的调用点在 [`SearchEverywhere::issue`] 的闸门**之后**，而闸门
    //! 之后紧接着就是 `state = Loading` / `set_loading(true)` / `timeline.schedule()`。
    //! 所以"`state` 仍是 `Idle`"= "根本没走到那次 Core 调用"。
    //!
    //! ⚠️ 测试刻意**不**让任何一次查询真的发出去（Core 请求会起 Core 进程）：对照组排上
    //! 防抖之后立刻 [`SearchEverywhere::abandon`] 把它撤掉（`Task` 一 drop 就取消唤醒），
    //! 其余分支都在闸门内返回。全程无 sleep / 无时钟推进 / 无 I/O。

    use std::rc::Rc;

    use gpui_kit::{
        AppContext as _, Bounds, Entity, Point, Size, TestAppContext, Window, WindowBounds,
        WindowHandle, WindowOptions, base::Root, px,
    };

    use super::*;

    /// 在无头窗口里建一个浮层（形状照 `tests/common/mod.rs` 的 `open_window`）。
    ///
    /// 两个回调都注入空实现：本测试不打开结果、也不执行动作，只关心"查询走哪条路"。
    /// `SearchIndexReadiness::Ready`：就绪提示那一行不画，测试里没有别的分支。
    fn open_overlay(cx: &mut TestAppContext) -> (WindowHandle<Root>, Entity<SearchEverywhere>) {
        let on_open: Rc<dyn Fn(PathBuf, Option<u32>, &mut Window, &mut App) + 'static> =
            Rc::new(|_, _, _, _| {});
        let on_action: Rc<dyn Fn(CommandId, &mut Window, &mut App) + 'static> =
            Rc::new(|_, _, _| {});
        cx.update(|cx| {
            gpui_kit::init(cx);
            let (window, overlay) = gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: Size {
                            width: px(800.),
                            height: px(600.),
                        },
                    })),
                    ..Default::default()
                },
                cx,
                move |window, cx| {
                    cx.new(|cx| {
                        SearchEverywhere::new(
                            PathBuf::from("D:/proj"),
                            on_open,
                            on_action,
                            SearchIndexReadiness::Ready,
                            window,
                            cx,
                        )
                    })
                },
            )
            .expect("open test window");
            (window.downcast::<Root>().expect("Base Root"), overlay)
        })
    }

    /// 在窗口里跑一段需要 `&mut Window` 的浮层方法。
    fn with_overlay(
        handle: WindowHandle<Root>,
        overlay: &Entity<SearchEverywhere>,
        cx: &mut TestAppContext,
        run: impl FnOnce(&mut SearchEverywhere, &mut Window, &mut Context<SearchEverywhere>),
    ) {
        cx.update_window(handle.into(), |_, window, cx| {
            overlay.update(cx, |overlay, cx| run(overlay, window, cx))
        })
        .expect("update overlay");
    }

    /// "没有发出 Core 请求"的机器判据：没排防抖、Core 状态机没进 `Loading`。
    ///
    /// `issue` 的闸门之后**紧接着**就是 `state = Loading` + `set_loading(true)` +
    /// `timeline.schedule()`，`CoreClient::execute` 在它们后面 —— 所以"不是 `Loading`"
    /// 就等于"没走到那次 Core 调用"。
    fn assert_no_core_query(
        overlay: &Entity<SearchEverywhere>,
        cx: &mut TestAppContext,
        what: &str,
    ) {
        overlay.read_with(cx, |overlay, _| {
            assert!(
                !matches!(overlay.state, EverywhereState::Loading { .. }),
                "{what}：Core 状态机不该进 `Loading`"
            );
            assert!(
                overlay.debounce_task.is_none(),
                "{what}：不该排防抖任务（那是通往 `issue` 的唯一一条路）"
            );
        });
    }

    /// **"Actions 档与 `/` 命令模式都不发 Core 请求"** 的交互层证据。
    #[gpui_kit::test]
    fn the_actions_scope_and_command_mode_never_start_a_core_query(cx: &mut TestAppContext) {
        let (handle, overlay) = open_overlay(cx);

        // 1. 对照组：Core 档 + 非空查询 ⇒ 真的排上了防抖。**没有这一步，下面的断言是空的**
        //    （一个永远不排防抖的实现也能让它们全绿）。
        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.set_scope(SearchEverywhereScope::Files, window, cx);
            overlay.on_query("alpha", window, cx);
        });
        overlay.read_with(cx, |overlay, _| {
            assert_eq!(overlay.query_text.as_ref(), "alpha");
            assert!(
                overlay.debounce_task.is_some(),
                "Core 档的非空查询必须排上防抖（否则本测试证明不了任何事）"
            );
        });
        // 立刻撤掉：不让 180ms 之后真的发出一次 Core 请求（那会起 Core 进程）。
        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.abandon(window, cx);
        });

        // 2. Actions 档 + 非空查询：不排防抖、Core 状态机不动。
        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.set_scope(SearchEverywhereScope::Actions, window, cx);
            overlay.on_query("alpha", window, cx);
        });
        overlay.read_with(cx, |overlay, _| {
            assert_eq!(
                overlay.query_text.as_ref(),
                "alpha",
                "查询串本身要照常更新（动作列表每帧按它现算）"
            );
            assert!(matches!(overlay.state, EverywhereState::Idle));
        });
        assert_no_core_query(&overlay, cx, "Actions 档敲字");

        // 3. 就算有人**直接**叫 `issue`（现实中是防抖任务在切档之后才到期），闸门也要挡住。
        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.issue(window, cx);
        });
        assert_no_core_query(&overlay, cx, "Actions 档 + issue");

        // 4. `/` 命令模式：在 **Core 档**下也一样（真源把 `/` 判在 scope 之前）。
        //    先把状态摆成"已经有 alpha 的 `ready` 命中"，这样切回 Core 档不会走
        //    "补一次查询"那条路（那条路**本来就该**发请求，而这里要测的是 `/`）——
        //    预置状态是测试自己构造的确定性前提，不是产品行为。
        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.query_text = SharedString::from("alpha");
            overlay.state = EverywhereState::ready("alpha", &[]);
            overlay.set_scope(SearchEverywhereScope::All, window, cx);
        });
        assert_no_core_query(&overlay, cx, "回到 Core 档（同一查询串已有 ready 结果）");
        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.on_query("/alpha", window, cx);
            overlay.issue(window, cx);
        });
        overlay.read_with(cx, |overlay, _| {
            assert_eq!(overlay.action_query().as_deref(), Some("alpha"));
            assert!(
                matches!(overlay.state, EverywhereState::Ready { .. }),
                "`/` 模式下 `issue` 必须被闸门挡住：否则它会把 Ready 覆盖成 Loading"
            );
        });
        assert_no_core_query(&overlay, cx, "`/` 命令模式");

        // 5. 把 `/` 去掉：同一个查询串回到工作区搜索，必须重新排队（这次是**该**排的）——
        //    反过来证明第 4 步不是"碰巧不排"。
        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.on_query("alpha", window, cx);
        });
        overlay.read_with(cx, |overlay, _| {
            assert!(
                overlay.debounce_task.is_some(),
                "离开 `/` 模式回到工作区查询时必须重新排队"
            );
        });
        with_overlay(handle, &overlay, cx, |overlay, window, cx| {
            overlay.abandon(window, cx);
        });
        assert_no_core_query(&overlay, cx, "收尾");
    }
}
