//! 编辑区的**表现层**：标签栏（`← →` 导航组 + 各文件的标签 + 关闭按钮）与正文 / 空状态。
//!
//! 从 `shell_probe/editor.rs` 原样拆出（逐字搬迁，只调整可见性与 import）。
//! 本文件只负责画与交互；读盘、图标与标签显示名在 `buffer.rs`。
//!
//! ## 阶段 9 接上的四件事
//!
//! | 项 | 落点 |
//! | --- | --- |
//! | A 脏标记 | [`EditorPane::on_input_change`]（订阅 `InputEvent::Change`）→ [`crate::buffer::Buffer::is_dirty`] |
//! | A `Ctrl+S` | [`crate::SaveBuffer`] action + [`EditorPane::save_active`] → [`EditorPane::write_buffer`] |
//! | A 防抖自动保存 | [`EditorPane::schedule_auto_save`] / [`EditorPane::run_auto_save`]（默认开启，见 [`AUTO_SAVE_ENABLED`]） |
//! | B 状态栏光标 | [`EditorPane::sync_cursor`]（`CursorPosition` + `S1_EDITOR_CURSOR` 诊断） |
//! | C 查找替换 | [`EditorPane::open`] 里显式写出的 `searchable(true)` —— 面板与 `Ctrl+F` / `Ctrl+H` 都归组件（见该处注释） |
//! | E 关闭确认 | [`EditorPane::request_close`]（`保存` / `放弃修改` / `取消`） |
//! | D 语法高亮 | [`EditorPane::open`] / [`EditorPane::open_virtual`] 里 `EditorState::new(..).language(..)`，语言名由 [`crate::buffer::language_for_file`] 按扩展名给出；grammar 开关在 `crates/editor/Cargo.toml` 的 `gpui-kit` feature |
//!
//! ## 阶段 10 第一批接上的三件事（Java 代码跳转的轻量链路，不启 JDTLS）
//!
//! | 项 | 落点 |
//! | --- | --- |
//! | `F12` | [`crate::NavigateToDefinition`] action + [`EditorPane::navigate_to_definition`] |
//! | `Ctrl+单击` | [`EditorPane::render_body`] 里包装层 div 的 `on_mouse_down` → [`EditorPane::on_editor_click`]（同一个落点） |
//! | `← →` 回退 | [`crate::navigation::JumpHistory`] + [`EditorPane::go_back`] / [`EditorPane::go_forward`]；按钮的禁用态见 [`EditorPane::nav_button`] |
//!
//! Core 调用与历史的数据侧都在 [`crate::navigation`]，本文件只负责接线与诊断。

//! ## 阶段 10 第二批接上的四件事（JDTLS 语义结果，`gpui/PLAN.md` §10.3）
//!
//! | 项 | 落点 |
//! | --- | --- |
//! | 跨文件 / 跨模块跳转 | [`EditorPane::apply_definition`] 的 `NavTarget::File` 分支（目标文件没开着就先 [`EditorPane::open`]） |
//! | `jdt://` 库源码 | [`EditorPane::apply_definition`] 的 `NavTarget::Virtual` 分支 + [`EditorPane::open_virtual`]（只读 buffer，身份是虚拟 URI） |
//! | 打开项目生成索引 | [`EditorPane::prepare_java`]（外壳在 `ShellWorkspace::new` 里转发工作区根）→ 后台 `JavaLanguageService::prepare` |
//! | 退出时关会话 | `impl Drop for EditorPane`（同步 `lsp.stopServer` + `lsp.destroyServer`，理由见该处注释） |
//!
//! 结果来源的**选择**在 [`crate::navigation::resolve_target`]（`.java` 优先 JDTLS、
//! 服务不可用退回第一批的轻量导航），本文件只负责把选出来的目标落地。
//!
//! Core 调用与历史的数据侧都在 [`crate::navigation`]，本文件只负责接线与诊断。

//! ## 阶段 11 接上的一件事：标签右键菜单（`gpui/research/windows/09-tab-context-menu.md`）
//!
//! | 项 | 落点 |
//! | --- | --- |
//! | 菜单挂载（per-tab `id` + 当场捕获 index） | [`EditorPane::with_tab_menu`]（由 [`EditorPane::render_tab`] 调） |
//! | 复制路径 / 复制相对路径 | [`EditorPane::copy_path`] / [`EditorPane::copy_relative_path`]（后者要 `workspace_root`） |
//! | 在资源管理器中显示 / 在终端中打开 | [`EditorPane::reveal_in_explorer`] / [`EditorPane::open_in_terminal`]（经 [`TabMenuHostActions`] 转发给外壳） |
//! | 重新加载 | [`EditorPane::request_reload`] → [`EditorPane::reload_buffer`] |
//! | 关闭 / 关闭其他 / 关闭右侧 / 全部关闭 | [`EditorPane::request_close`] / [`EditorPane::close_scope`]（后者**只弹一次**批量确认） |
//! | `Ctrl+W` = 关闭当前 | [`EditorPane::close_active`]（绑定在 `crate::install_actions`） |
//!
//! 诊断行一律 `S1_TAB_MENU`（**stderr**）：`opened=true tab=<名> items=<n> separators=<n>` 在菜单
//! 真的被右键打开时打一次；执行动作打 `run=<动作> …`。
//!
//! ## B1 接上的两件事（菜单侧入口 + 关闭按钮可见性）
//!
//! 1. **非活动标签也有 ×**（悬停显示、活动标签常显）：[`EditorPane::render_tab`] 的
//!    分组悬停，以及 [`EditorPane::close_button`] 的 `stop_propagation`；
//! 2. **主菜单「文件 → 关闭…」那一批入口**：[`EditorPane::close_other_tabs`] /
//!    [`EditorPane::close_tabs_to_left`] / [`EditorPane::close_tabs_to_right`] /
//!    [`EditorPane::close_all_tabs`] / [`EditorPane::close_saved_tabs`] /
//!    [`EditorPane::reopen_closed_tab`] —— **复用**同一条 `close_scope` 链，
//!    锚点从"被右键的标签"换成"当前活动标签"；「重新打开已关闭标签页」读
//!    [`EditorPane::closed`] 那个 LIFO 栈并走同一条 [`EditorPane::open`]。
//!
//! ## 阶段 12 接上的一件事：JDTLS 诊断波浪线
//!
//! | 项 | 落点 |
//! | --- | --- |
//! | 排一次取回（四个触发点） | [`EditorPane::schedule_diagnostics`]：`open` / `prepare_java` / `on_input_change` / `reload_buffer` |
//! | 结果落地（代次 + 修订号 + buffer 还在） | [`EditorPane::apply_diagnostics`] |
//! | 数据侧（同步正文 + 有界重取 + 列换算） | [`crate::diagnostics`] |
//!
//! 上游**有**宿主塞诊断的公开入口（`EditorState::diagnostics_mut()`），所以这是接线不是自绘；
//! 完整的 API 清单、列口径陷阱与"为什么必须有界重取"都写在 [`crate::diagnostics`] 的模块文档里。

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::buffer::{Buffer, display_names, icon_for_file, language_for_file, read_body};
use crate::navigation::{JumpEntry, JumpHistory, NavTarget, editor_position, resolve_target};
use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::input::{Editor, EditorState, InputEvent, Position, Rope, TabSize};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::tab::{Tab, TabBar, TabVariant};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, WindowExt as _};
use gpui_kit::{
    AbsoluteLength, AnyElement, App, AppContext as _, ClipboardItem, Context, Div, Entity,
    InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent, ParentElement as _, Pixels,
    Render, ScrollWheelEvent, SharedString, StatefulInteractiveElement as _, Styled as _, Task,
    Window, div, point, relative, rems,
};
use lithe_gpui_java::JavaLanguageService;
use lithe_gpui_shared::{tr, tr_args};

/// 把规格值（px）按**当前 rem 基准**求值：`rems(P / 16.)` 的 `Pixels` 形式。
///
/// 只有**必须**交出 `Pixels` 的槽才走这里（能做到 rem-based 的样式一律直接写 `rems(P / 16.)`）：
/// 例如此处的滚轮增量换算 —— `event.delta.pixel_delta(line_height)` 吃的是 `Pixels`，
/// 交不了 `AbsoluteLength`。写成 `/ 4.` 是错的：helper 后缀 `N` = `N × 0.25rem`，
/// 而这里的 `P` 是**像素**，1rem = 16px（主题的 `font.size`）。
///
/// `pub(crate)`：[`crate::completion::install`] 的菜单宽度也走这一份（同一个 crate 只该有一处换算）。
pub(crate) fn rem_px(rem: Pixels, spec_px: f32) -> Pixels {
    AbsoluteLength::from(rems(spec_px / 16.)).to_pixels(rem)
}

// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`，与 Windows 规格逐像素相等：
//
// | 规格（Windows 真源） | 值 | 用到的 helper |
// | --- | --- | --- |
// | 标签栏高 `--lithe-tab-bar-height: 2.25rem`（`styles/theme.css:120-121`、`ui/tab-bar.tsx:248`） | 36 | `h_9()` |
// | 标签栏左右内边距 `--lithe-chrome-padding-inline`（`theme.css:132`） | 8 | `px_2()` |
// | 标签栏段间距 `--lithe-chrome-gap`（`theme.css:130`） | 4 | `mr_1()` |
// | 标签 `pl-2` / `pr-6`（`ui/tab-bar.tsx:260`） | 8 / 24 | `pl_2()` / `pr_6()` |
// | 标签图标↔文字间距 `--lithe-chrome-gap-loose`（`theme.css:131`） | 6 | `gap_1p5()` |
// | 标签图标槽 `size-3`（`tab-bar-item.tsx:183`） | 12 | `size_3()` |
// | 导航组 `gap-0.5`（`tab-bar.tsx:633`） | 2 | `gap_0p5()` |
// | 关闭按钮 `absolute right-1`（`tab-bar-item.tsx:164-166`） | 4 | `right_1()` |
// | 脏标记圆点 `size-2`（`tab-bar-item.tsx:284-291`） | 8 | `size_2()` |
// | 空状态 `px-6 py-8` / `gap-3`（`empty-editor-state.tsx:15-16`） | 24 / 32 / 12 | `px_6()` / `py_8()` / `gap_3()` |
// | 空状态内容块 `max-w-md`（`empty-editor-state.tsx:16`） | 448 | `max_w_112()` |
// | 图标块 `size-12` / 主图标 `size-10` / 放大镜 `size-5`（`empty-editor-state.tsx:17-22`） | 48 / 40 / 20 | `size_12()` / `size_10()` / `size_5()` |
//
// 字号：`--ui-text-base` / `--ui-text-sm` 在这里都是 **13px**（`theme.css:116-117`），不在 gpui 的
// 档位（`text_xs()`=12 / `text_sm()`=14）上，按《编码指南》用 **`text_sm()`（14px）**——
// 13 → 14 是经维护者确认的**有意**视觉改动。

/// 单个标签宽度上限 200px：`--lithe-tab-max-width`（12.5rem，`windows/tauri/src/styles/theme.css:123`，
/// 用在 `windows/tauri/src/ui/tab-bar.tsx:260`）。**这是标签文字能截断的前提**。
///
/// 常量保持**规格像素值身份**（`f32`，值不变）。消费方式：`TabBar::max_width(impl Into<Pixels>)`
/// （`gpui-component-0.6.6/src/tab/tab_bar.rs:118`）只吃 `Pixels`，所以调用点走
/// `rem_px(cx.theme().font_size, TAB_MAX_WIDTH)`（见 [`rem_px`]）——「200 不在 rem 档位上」
/// 不构成保留 `px(...)` 的理由，档位外本来就该写 helper 底层的 `rems(P / 16.)`。
const TAB_MAX_WIDTH: f32 = 200.;

/// 滚轮增量换算用的行高兜底值：真机默认行高就是 **20**
/// （`windows/tauri/src/features/editor/config/constants.ts:5` 的 `DEFAULT_LINE_HEIGHT: 20`；
/// 算法是 `ceil(fontSize × 1.4)`，`windows/tauri/src/features/editor/utils/lines.ts:8-15`）。
/// 编辑器还没完成首次布局时 `line_height()` 是 `None`，用这个值兜底，
/// 否则 `ScrollDelta::Lines` 换算出 0，整段滚不动。
///
/// 它是滚轮增量换算里的 `Pixels`（与 `line_height()` 的返回值——`EditorState::line_height()`
/// 返回 `Option<Pixels>`，`gpui-base-0.6.6/src/input/base/state.rs:2820`——同类型做算术：
/// `event.delta.pixel_delta(line_height)`），不是布局样式槽，套不了 `Styled` 的 `line_height`
/// 系列 helper。但它也**不是** :288 意义上的"measured runtime geometry"：20 是写死的规格值，
/// 只是"真源默认字号下的行高"。调用点按当前 rem 基准求值（`rem_px(window.rem_size(), ..)`），
/// 这样把界面字号调大时，这一档兜底值与真源"行高 = ceil(fontSize × 1.4)"的语义一致
/// —— 默认 16px 基准下与原来的 `px(20.)` 逐像素相等。
const FALLBACK_LINE_HEIGHT: f32 = 20.;

/// 防抖自动保存的等待窗口 **150ms**：逐值照 Windows 的 `setTimeout(.., 150)`
/// （`windows/tauri/src/features/editor/stores/editor-app.store.ts:567`）。
const AUTO_SAVE_DEBOUNCE: Duration = Duration::from_millis(150);

/// 自动保存总开关，**默认开启**。
///
/// Windows 的真源是设置项 `editor.autoSave`，默认值就是 `true`
/// （`windows/tauri/src/features/settings/config/default-settings.ts:49`）；语义是"编辑时
/// 防抖自动保存"，脏标记与手动 `Ctrl+S` 是**独立**的另一条路径（同文件 `:503-570`）。
///
/// ⚠️ **这里是常量而不是读设置**：阶段 9 的范围由维护者定稿为"只做编辑器侧"，
/// gpui 的设置文件（`gpui/crates/settings/src/schema.rs` 目前只有主题 / 字号 / 状态栏 /
/// 语言）与设置页这一轮不扩。接设置页时把它改成 `Settings` 的对应字段即可，
/// 调用点只有 [`EditorPane::on_input_change`] 一处。
const AUTO_SAVE_ENABLED: bool = true;

/// 状态栏用的光标位置：**1 基**行列。
///
/// 显式用 1 基而不是拿组件的 0 基 `lsp_types::Position` 直接用，是因为"第几行第几列"
/// 的换算与显示口径（`line + 1` / `column + 1`）只该有一处实现：
/// 真机就是 `editor-status-actions.tsx:29` 的 `` `${line + 1}:${column + 1}` ``。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CursorPosition {
    /// 行号，1 基。
    pub line: usize,
    /// 列号，1 基。
    pub column: usize,
}

impl CursorPosition {
    /// 没有活动 buffer 时的位置。
    ///
    /// 真机拿不到任何光标位置时返回 `INITIAL_CURSOR_POSITION = {line: 0, column: 0}`
    /// （`windows/tauri/src/features/editor/utils/editor-view-cursor-position.ts:3`），
    /// 显示出来同样是 `1:1`。
    const INITIAL: Self = Self { line: 1, column: 1 };
}

/// 一次在飞的跳转请求，用来把"结果回来"和"什么时候发的"对上。
///
/// 三段校验都在 [`EditorPane::apply_definition`] 里：`generation` 防"同一窗口里又跳了一次"，
/// `path` 防"buffer 已经关了/换了一个"，`revision` 防"正文在等待期间被改过"。
struct NavRequest {
    /// 本进程内自增的请求号（新请求会让旧请求的结果作废）。
    generation: u64,
    /// 发起时活动 buffer 的打开路径。
    path: PathBuf,
    /// 发起时该 buffer 的正文修订号。
    revision: u64,
    /// 发起时的光标位置（0 基）：跳转成功后它就是历史里的"跳转前的位置"。
    origin: Position,
}

/// 标签右键菜单里"**只有外壳做得了**"的两件事的回调。
///
/// 落点理由（`gpui/research/windows/09-tab-context-menu.md` §5.3，与
/// `gpui/crates/explorer/src/lib.rs` 已登记的"起进程属平台层"是同一条约束）：
///
/// - **在资源管理器中显示**要起 `explorer.exe`（`/select,<path>`），启动外部进程不是
///   一个视图模块该干的事；
/// - **在终端中打开**要改外壳的底部工具窗可见性（`ShellWorkspace::bottom_visible` 是外壳
///   自己的布局状态，`TerminalPane` 只知道"我有几个页签"）。
///
/// 两者都必须 `&mut Window` + `&mut App`（前者要起进程、后者要 `update` 终端实体），
/// 所以回调签名与 `PopupMenuItem::on_click` 一致。`Arc` 是必要的：回调用 `&self` 取用，
/// 而调用时 `self` 还被 `EditorPane` 可变借着一份（先 `clone` 出 `Arc` 再调）。
pub struct TabMenuHostActions {
    /// 在系统文件管理器里定位 `path`（Windows = `explorer /select,`）。
    pub reveal: Arc<dyn Fn(&Path, &mut Window, &mut App) + 'static>,
    /// 在 `path` **所在目录**新开一个终端页签，并让外壳把底部工具窗切到终端且显示。
    pub open_terminal: Arc<dyn Fn(&Path, &mut Window, &mut App) + 'static>,
}

/// 批量关闭类菜单项的作用范围。
///
/// **锚点存路径不存下标**：确认对话框弹出期间标签可能增减、顺序可能变，
/// 确认后要按当下这份顺序**重算**集合（维护者口径），路径比下标稳。
///
/// 真源对应 `handleCloseOtherTabs(keepId)` / `handleCloseTabsToLeft(id)` /
/// `handleCloseTabsToRight(id)` / `handleCloseAllTabs()` / `handleCloseSavedTabs()`
/// （`windows/tauri/src/features/editor/stores/buffer.store.ts:1742-1830`；
/// 左侧与已保存两条的真源入口见 `windows/tauri/src/ui/tab-context-menu.tsx:60-84`
/// 的 `closeTabsToLeft` / `closeSavedTabs` 命令）。
#[derive(Clone)]
enum CloseScope {
    /// 关闭其他：除了锚点那个标签，其余全关。
    Others(PathBuf),
    /// 关闭左侧：锚点**之前**的标签全关（`slice(0, index)`）。
    ToLeft(PathBuf),
    /// 关闭右侧：锚点**之后**的标签全关（`slice(index + 1)`）。
    ToRight(PathBuf),
    /// 全部关闭。
    All,
    /// 关闭已保存：**不脏**的标签全关（脏的一个都不动，也不弹确认框）。
    Saved,
}

impl CloseScope {
    /// 诊断行里的动作名（与 `S1_TAB_MENU run=` 一起读）。
    fn tag(&self) -> &'static str {
        match self {
            Self::Others(_) => "closeOthers",
            Self::ToLeft(_) => "closeLeft",
            Self::ToRight(_) => "closeRight",
            Self::All => "closeAll",
            Self::Saved => "closeSaved",
        }
    }

    /// 当下这批 buffer 里该被关掉的**下标**（升序）。
    ///
    /// 集合为空是**正常结果**（只有 1 个标签时"关闭其他/左侧/右侧"、全脏时"关闭已保存"
    /// 都是空集），真源同样静默空转：`buffer.store.ts:1758` 的 `forEach` 什么都不做、
    /// `:1813` 的 `index === -1` 直接 `return`。所以调用方**不要**把它当成错误。
    fn targets(&self, buffers: &[Buffer]) -> Vec<usize> {
        match self {
            Self::Others(anchor) => buffers
                .iter()
                .enumerate()
                .filter(|(_, buffer)| &buffer.path != anchor)
                .map(|(index, _)| index)
                .collect(),
            // 锚点不在（比如刚被别的路径关掉）→ 空集，不猜一个位置（与 `ToRight` 同口径）。
            Self::ToLeft(anchor) => {
                let Some(position) = buffers.iter().position(|buffer| &buffer.path == anchor) else {
                    return Vec::new();
                };
                (0..position).collect()
            }
            Self::ToRight(anchor) => {
                let Some(position) = buffers.iter().position(|buffer| &buffer.path == anchor) else {
                    return Vec::new();
                };
                (position + 1..buffers.len()).collect()
            }
            Self::All => (0..buffers.len()).collect(),
            Self::Saved => buffers
                .iter()
                .enumerate()
                .filter(|(_, buffer)| !buffer.is_dirty)
                .map(|(index, _)| index)
                .collect(),
        }
    }
}

/// 「确认之后做什么」——三种收尾共用一个未保存确认对话框。
#[derive(Clone)]
enum PendingConfirm {
    /// 关一个标签（关闭按钮 / 菜单「关闭」/ `Ctrl+W`）。
    Single(usize),
    /// 批量关闭（关闭其他 / 关闭左侧 / 关闭右侧 / 全部关闭）。
    Batch(CloseScope),
    /// 重新加载（真机也是"先关再开"，`tab-bar.tsx:715-739`，所以脏标签要先过确认）。
    Reload(usize),
}

/// 一次会话恢复的结论（给外壳打诊断用，不参与渲染）。
///
/// 单独一个类型而不是返回 `usize`：调用方要同时知道"开了哪几个"与"最后停在哪个标签上"，
/// 而"活动下标"的语义（见 [`EditorPane::restore_session`]）不写明就会被误读成"会话文件里的
/// 那个下标"——它其实是**恢复之后**的列表下标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRestore {
    /// 真的打开（或切到）的磁盘文件，顺序 = 标签顺序。
    pub opened: Vec<PathBuf>,
    /// 恢复后活动标签在 [`Self::opened`] 里的下标；`None` = 没有打开任何文件。
    ///
    /// 会话里记的下标**越界**时（例如它指向的文件被跳过了）这里的取值：
    /// - 会话没记活动文件（`None`）→ 第一个文件（`0`）；
    /// - 会话记了但越界 → 沿用"打开之前那一刻的活动标签"（`open` 的既有语义），而不是硬选第一个。
    pub active_index: Option<usize>,
}

/// 编辑区视图：标签栏 + 正文（正文没有活动 buffer 时是空状态）。
pub struct EditorPane {
    /// 打开的 buffer，顺序就是标签栏里的顺序。
    buffers: Vec<Buffer>,
    /// 活动 buffer 在 [`Self::buffers`] 里的下标；`None` = 没有活动 buffer → 空状态。
    active: Option<usize>,
    /// 上一次报给外壳的光标位置；用来把"光标真的动了"和"编辑器又重绘了一次"分开，
    /// 否则外壳会跟着每一次编辑器重绘重排状态栏。
    cursor: Option<CursorPosition>,
    /// `← →` 的跳转历史（语义见 [`JumpHistory`]）。
    history: JumpHistory,
    /// 在飞的那次 [`crate::NavigateToDefinition`] 请求；`None` = 没有。
    nav_request: Option<NavRequest>,
    /// 在飞请求的后台任务。
    ///
    /// 必须**被持有**：gpui 的 `Task` 一 drop 就取消（与 [`Buffer::auto_save_task`] 同一约定），
    /// 丢掉它等于请求发出去之后立刻取消。换掉旧任务就是取消旧请求。
    nav_task: Option<Task<()>>,
    /// 请求号的自增源。
    nav_generation: u64,
    /// 诊断刷新请求号的自增源（跨所有 buffer 共享：同一份文件被重排时旧结果必须作废）。
    diagnostics_generation: u64,
    /// Java 语言服务（阶段 10 第二批）。`None` = 外壳还没告诉本视图工作区根，
    /// 或宿主根本没有 Java 工具 —— 两种情况下跳转都走第一批的轻量导航。
    ///
    /// `Arc` 是必要的：每次跳转都要把服务句柄 move 进后台任务，而服务本身被本视图持有。
    java: Option<Arc<JavaLanguageService>>,
    /// 打开项目时那次"起服务 + 生成 / 复用索引"的后台任务，必须被持有（same as above）。
    java_task: Option<Task<()>>,
    /// 工作区根：**只**给标签右键菜单的「复制相对路径」用（真机 `getRelativePath(path,
    /// rootFolderPath)`，无根时拷全路径，`tab-bar.tsx:355-368`）。
    ///
    /// 由外壳在 `ShellWorkspace::new` 里登记（与 [`Self::prepare_java`] 同一个调用点）。
    workspace_root: Option<PathBuf>,
    /// 外壳登记的两个"只有外壳做得了"的动作（见 [`TabMenuHostActions`]）。
    /// `None` = 外壳还没登记（例如组件在测试宿主里单独跑）→ 对应菜单项点了会留一行诊断。
    tab_menu_actions: Option<TabMenuHostActions>,
    /// 制表符宽度（空格数）。真源是设置里的 `tabSize`（Windows 默认 2，
    /// `default-settings.ts:55`），由外壳在启动时与每次设置变化后经
    /// [`EditorPane::set_tab_size`] 登记进来。
    ///
    /// ⚠️ 编辑区**不认识设置 crate**（依赖方向：`workbench` → `editor`，反向会成环），
    /// 所以这里存的是一份值，不是设置句柄；默认值就地写 2，与
    /// `EditorState` 自己的默认档一致（`gpui-base-0.6.6/src/input/editor/indent.rs:20-27`）。
    tab_size: usize,
    /// 「自动补全」开关（设置里的 `autoCompletion`，阶段 18）。
    ///
    /// ⚠️ 与 [`Self::tab_size`] 的**唯一区别**：这是 `Rc<AtomicBool>` 而不是 `bool`。
    /// 原因是补全 provider 已经被装进每个 `EditorState`（`Rc<dyn CompletionProvider>`），
    /// 而 `is_completion_trigger(&self, ..)` 只拿得到 `&self` —— 想让它读到最新设置，
    /// 要么重建每个 buffer 的 provider，要么**共享一个原子**。
    ///
    /// 共享原子是更好的那一半：外壳一次 `store(false)` 就让所有已打开的 Java buffer
    /// 同时停止自动弹菜单（不必遍历 buffer、不会漏掉后打开的），并且 provider 本身
    /// 仍然装着 —— 那是将来做手动触发（`Ctrl+Space`）的基础
    /// （理由逐条写在 `crate::completion` 的字段文档里）。
    ///
    /// 默认 `true`（真源 `default-settings.ts:153`）。
    auto_completion: Rc<AtomicBool>,
    /// **当前被悬停的标签下标**（`None` = 没有任何标签被悬停）。
    ///
    /// 「非活动标签的 × 悬停才显示」要靠它（见 [`Self::render_tab`]）。
    /// 为什么要把它记在本结构体里，而不是用 gpui 的分组悬停（`.group(..)` +
    /// `.group_hover(..)` 让那个 div 自己翻显隐）：**实测后者不生效** ——
    /// 同一套注入步骤（`WM_MOUSEMOVE` 到非活动标签上）下，`.group_hover` 版本
    /// 前后两帧逐像素相同（`.artifacts/p21/10-two-tabs-active-beta.png` 与
    /// `11-hover-alpha-close-visible.png` 的 SHA 相同），而同一时刻把鼠标移到文件树某一行上
    /// 却能看到那行高亮（`12-*` vs `13-*` 的差异框正好是那一行）——说明 hover 事件本身会触发
    /// 重绘，是分组悬停那条链（`GroupHitboxes` + 它在 paint 期注册的 `on_mouse_event` 监听器，
    /// `gpui-pre-0.3.6/src/elements/div.rs:3300-3316`）没把重绘要出来。既然本结构体本来就
    /// 每帧渲染，自己记一个下标是更短、且可实测的路径。
    hovered_tab: Option<usize>,
    /// **已关闭磁盘文件的 LIFO 栈**：菜单「文件 → 重新打开已关闭标签页」的数据源。
    ///
    /// 为什么存路径而不是存整个 `Buffer`：关掉标签的语义就是**释放**那份 `EditorState`
    /// （撤销栈、光标、诊断任务一起丢，见 [`Self::close`] 的文档），真源同样只留 id 级别
    /// 的记录（`buffer.store.ts` 的 `reopenClosedTab` 从 `closedBuffers` 里按 id 重开）。
    /// 重开时走的是**同一条** [`Self::open`]（读盘 + 建状态 + 排诊断），所以这里不需要
    /// 缓存正文。
    ///
    /// 只记**磁盘文件**：`jdt://` 虚拟源码没有"重新读盘"这条路（正文只来自 Core 的虚拟
    /// 文档请求），把它压进栈会让重开变成一份读盘失败的说明文案。
    closed: Vec<PathBuf>,
}

/// 制表符宽度的默认值（= Windows `tabSize` 的默认值 2、也是组件默认档）。
/// 抽成常量是因为 [`EditorPane::set_tab_size`] 的上界校验与 [`EditorPane::new`] 都要用。
const DEFAULT_TAB_SIZE: usize = 2;
/// 制表符宽度的上界（**只为挡住把编辑区改坏的输入**，不是产品档位表：
/// 真源的下拉只有 2/4/8，那三个值在设置 crate 的 `TAB_SIZES` 里）。
const MAX_TAB_SIZE: usize = 64;

/// 「重新打开已关闭标签页」最多回溯多少个（FIFO 淘汰最旧的一条）。
///
/// 20 不是真源里的数（真源那个队列没有公布上限）：它只是**有界**要求的一个具体值 ——
/// 一次会话里连关几十个文件很常见，而用户点"重新打开"只会回溯最近几个；
/// 上限存在的意义是不让一个长会话把已关文件的路径无限攒下去。
const MAX_CLOSED_TABS: usize = 20;

impl EditorPane {
    /// 建一个没有打开任何文件的编辑区。
    ///
    /// 真机启动时编辑区就是空状态（标签栏在、正文是空状态），
    /// 打开动作由用户触发（`panes/components/pane-container.tsx:1100`）。
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            buffers: Vec::new(),
            active: None,
            cursor: None,
            history: JumpHistory::default(),
            nav_request: None,
            nav_task: None,
            nav_generation: 0,
            diagnostics_generation: 0,
            java: None,
            java_task: None,
            workspace_root: None,
            tab_menu_actions: None,
            tab_size: DEFAULT_TAB_SIZE,
            auto_completion: Rc::new(AtomicBool::new(true)),
            hovered_tab: None,
            closed: Vec::new(),
        }
    }

    /// 登记「自动补全」（设置里的 `autoCompletion`，阶段 18）。**对所有已打开的 Java buffer
    /// 立即生效**，之后新开的 buffer 也按这个值建。
    ///
    /// 落点是共享的那一个 [`std::sync::atomic::AtomicBool`]：所有补全 provider 都持有它的
    /// 克隆，所以这里一次 `store` 就够了（不必遍历 buffer、也不会漏掉"后打开的"）。
    /// 与 [`Self::set_tab_size`] 同一形状（外壳订阅设置实体后转发），差别只有这一点。
    ///
    /// 诊断 `S1_EDITOR_AUTO_COMPLETION` 打的是**开关值 + 已打开的 buffer 数**：
    /// 前者证明值真的传进来了，后者是"这一刻有多少个编辑器受了影响"的旁证
    /// （provider 的触发判据在 [`crate::completion`] 里，那一条由 `S1_JAVA_COMPLETION` 的
    /// 有无来证明 —— 关掉之后那一行**一行都不该出现**）。
    pub fn set_auto_completion(&mut self, enabled: bool, cx: &mut Context<Self>) {
        use std::sync::atomic::Ordering;
        let changed = self.auto_completion.load(Ordering::Acquire) != enabled;
        self.auto_completion.store(enabled, Ordering::Release);
        // 诊断**每次都打**（值没变也打）：外壳在启动时与每次设置变化都会喂一次，
        // 所以这一行同时是"外壳确实转发过"的证据。`buffers=` 是这一刻已经打开的 buffer 数
        // ——它证明**已打开的**编辑器也在同一条开关上（不是只管以后新开的）。
        println!(
            "S1_EDITOR_AUTO_COMPLETION enabled={} changed={} buffers={}",
            enabled,
            changed,
            self.buffers.len()
        );
        if changed {
            cx.notify();
        }
    }

    /// 登记「制表符宽度」（设置里的 `tabSize`）。**对所有已打开的 buffer 立即生效**，
    /// 之后新开的 buffer 也按这个值建。
    ///
    /// 落点是 `EditorState::set_tab_size`（`gpui-base-0.6.6/src/input/editor/indent.rs:504`，
    /// 公开 API），它同时改 Tab 键插入的空白数与 `Tab` 字符的显示宽度。
    /// 越界输入（0 或大得离谱）在这里夹一次：`TabSize { tab_size: 0 }` 会让缩进计算出 0 个空格，
    /// 那是"把编辑区改坏"，不是产品档位。
    ///
    /// 由外壳调用（`ShellWorkspace` 订阅设置实体后转发），所以编辑区不需要认识设置 crate。
    pub fn set_tab_size(&mut self, tab_size: usize, cx: &mut Context<Self>) {
        let tab_size = tab_size.clamp(1, MAX_TAB_SIZE);
        if self.tab_size == tab_size {
            return;
        }
        self.tab_size = tab_size;
        let tab = TabSize {
            tab_size,
            // 真源的 `tabSize` 就是"N 个空格"（`macos-settings-panels.tsx:321` 用
            // `settings.mac.spaces` 拼标签），没有"用制表符"这一档，所以恒为软缩进。
            hard_tabs: false,
        };
        for buffer in &self.buffers {
            buffer
                .editor
                .update(cx, |state, cx| state.set_tab_size(tab, cx));
        }
        // 诊断：`buffers=` 是"这一刻已经打开的 buffer 数"，所以它同时证明了两件事 ——
        // 值真的传进来了、并且**已打开的**编辑器也被重新设过（不是只管以后新开的）。
        println!(
            "S1_EDITOR_TAB_SIZE size={} buffers={}",
            self.tab_size,
            self.buffers.len()
        );
        cx.notify();
    }

    /// 打开项目时调一次（由外壳在 `ShellWorkspace::new` 里转发）：登记工作区根，
    /// 并在后台跑一次 [`JavaLanguageService::prepare`] —— 也就是"生成索引、下次复用"。
    ///
    /// **不阻塞**：`prepare` 要起 JVM、等 JDT 握手与项目导入（实测首次 ~5s，大项目更久），
    /// 所以整段放进 `background_spawn`。它失败也不影响任何现有功能：跳转会退回
    /// 第一批的轻量导航（见 [`crate::navigation::resolve_target`]）。
    ///
    /// 幂等：已经登记过工作区就不再重复起服务。
    pub fn prepare_java(&mut self, workspace_root: PathBuf, cx: &mut Context<Self>) {
        if self.java.is_some() {
            return;
        }
        let service = Arc::new(JavaLanguageService::new(workspace_root));
        self.java = Some(service.clone());

        // ⚠️ **给已经打开的 Java buffer 补装补全 provider**：真实的启动顺序是
        // "外壳建编辑区 → 打开启动文件 →（这里）起 Java 服务"，而 `open()` 里装 provider 时
        // `self.java` 还是 `None`（那份 provider 只剩轻量兜底）。不补装的话，
        // "先打开的文件"永远拿不到 JDTLS 的补全 —— 而它往往正是用户马上要写代码的那个文件。
        // 只补装一次（本方法幂等），且只补 Java buffer（与非 Java 文件不装的口径一致）。
        //
        // 快速修复 provider 走同一条补装路径（`crate::code_actions`）：它**没有兜底数据源**，
        // 所以 `open()` 时 `self.java` 还是 `None` 就干脆不装 —— 这里补上第一次。
        let mut patched = 0usize;
        for buffer in &self.buffers {
            if crate::buffer::language_for_file(&buffer.name) != Some("java") {
                continue;
            }
            let provider = crate::completion::JavaCompletionProvider::new(
                Some(service.clone()),
                buffer.path.clone(),
                // 共用同一个开关：外壳改设置时，**已经打开的** buffer 也立刻跟上
                // （这正是"补装"这一步顺带要把开关传下去的原因）。
                self.auto_completion.clone(),
            );
            let code_actions = crate::code_actions::JavaCodeActionProvider::new(
                service.clone(),
                buffer.path.clone(),
            );
            // `Entity::update` 的两参闭包只给 `(state, cx)`（不给 `Window`），所以 rem 基准
            // 从闭包外取好（`cx.theme().font_size`，与 `Root::render` 写进 `set_rem_size` 的是同一个值）。
            let rem = cx.theme().font_size;
            buffer.editor.update(cx, |state, _cx| {
                crate::completion::install(state, Some(provider), rem);
                crate::code_actions::install(state, Some(code_actions));
            });
            patched += 1;
        }
        if patched > 0 {
            // 诊断：证明"服务起来之后，之前打开的文件也接上了"这条路径真的跑到。
            println!("S1_JAVA_COMPLETION reattached buffers={patched}");
        }

        // 诊断也要给**已经打开的** Java buffer 补排一次：`open()` 里排的那次因为
        // `self.java` 还是 `None` 什么都没做（"先开文件、后起服务"是最常见的启动顺序）。
        // 收集下标再排：`schedule_diagnostics` 要 `&mut self`，不能在 `&self.buffers` 的循环里调。
        let java_buffers: Vec<usize> = self
            .buffers
            .iter()
            .enumerate()
            .filter(|(_, buffer)| crate::buffer::language_for_file(&buffer.name) == Some("java"))
            .map(|(index, _)| index)
            .collect();
        for index in java_buffers {
            // 打开档没有连打要合并，所以不给防抖（退避的第一档已经承担了等待）。
            self.schedule_diagnostics(index, Duration::ZERO, cx);
        }

        // `detach`：这是一次"发出去就不管结果"的预热，结果只走 `S1_JAVA_*` 诊断行。
        // 服务的生命周期由 `self.java` 这个 `Arc` 持有，与任务是否被持有无关。
        self.java_task = Some(cx.background_spawn(async move {
            match service.prepare() {
                Ok(true) => println!("S1_JAVA_PREPARED"),
                Ok(false) => {}
                Err(reason) => println!("S1_JAVA_PREPARE_FAILED reason={reason}"),
            }
        }));
    }

    // -----------------------------------------------------------------------
    // JDTLS 诊断波浪线（数据侧在 `crate::diagnostics`）
    // -----------------------------------------------------------------------

    /// 排一次诊断刷新：**只对"磁盘上的可写 Java buffer"**，其余一律什么都不做。
    ///
    /// 后台任务做两件事（理由与"为什么是有界重取"见 [`crate::diagnostics`] 的模块文档）：
    /// 同步当前正文 → 按退避最多查 5 次快照。任务被存进 `Buffer::diagnostics_task`，
    /// 所以"再排一次"就是取消上一次（gpui 的 `Task` drop 即取消），防抖也是靠这一条。
    ///
    /// 四个调用点与各自"为什么不会漏"的论证在模块文档的表里；这里的三条早退：
    ///
    /// - `self.java` 是 `None`（外壳还没登记工作区根）→ 什么都不做，由
    ///   [`Self::prepare_java`] 给**已经打开的** buffer 补排一次；
    /// - 不是 `.java`（判据复用 [`crate::buffer::language_for_file`]，与补全 / 语言名同源）；
    /// - **不可写**（读不到 / 超大 / 二进制 / 非 UTF-8 的有损正文）：这份正文不是文件的
    ///   忠实副本，把它同步给服务端等于让 JDT 按一段说明文案去算诊断 —— 诊断必然是垃圾，
    ///   而更糟的是会话里那份文档从此与磁盘内容不一致。
    ///
    /// `jdt://` 虚拟源码同样被挡在外面：那是只读的库源码，JDT 不为它发诊断。
    fn schedule_diagnostics(&mut self, index: usize, debounce: Duration, cx: &mut Context<Self>) {
        let Some(service) = self.java.clone() else {
            return;
        };
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        if !buffer.writable
            || is_virtual_source_path(&buffer.path)
            || crate::buffer::language_for_file(&buffer.name) != Some("java")
        {
            return;
        }

        let path = buffer.path.clone();
        let revision = buffer.revision;
        let text = buffer.editor.read(cx).text().to_string();

        // 代次 +1 并记进 buffer：回来时只有"我这次"才允许落地（见模块文档的陈旧保护）。
        let generation = self.diagnostics_generation.wrapping_add(1);
        self.diagnostics_generation = generation;
        self.buffers[index].diagnostics_generation = generation;

        let executor = cx.background_executor().clone();
        let task_path = path.clone();
        let task = cx.spawn(async move |pane, cx| {
            let fetched = cx
                .background_spawn(crate::diagnostics::fetch(
                    service, task_path.clone(), text, executor, debounce,
                ))
                .await;
            // 编辑区可能已经销毁：`update` 返回 `Err` 时静默忽略，不 panic
            // （与 `schedule_auto_save` / `navigate_to_definition` 同一约定）。
            let _ = pane.update(cx, |pane, cx| {
                pane.apply_diagnostics(generation, revision, task_path, fetched, cx)
            });
        });
        self.buffers[index].diagnostics_task = Some(task);
    }

    /// 后台结果回到前台：三段校验都过才写进 `EditorState`，并留一行可 grep 的诊断。
    ///
    /// 每一段失败都**要留证据**（静默丢弃会让"波浪线没出现"变成一个查不出来的问题）：
    /// 三段分别对应"标签关了"、"又有一次更新的刷新"、"正文在这次取回期间被改过"。
    /// 最后一段特别要紧：服务端算的是**那一刻的正文**，行列落到现在的正文上就是错位的波浪线；
    /// 而新的那一次刷新已经在路上（每次变化都会排一次），所以这里直接丢。
    fn apply_diagnostics(
        &mut self,
        generation: u64,
        revision: u64,
        path: PathBuf,
        fetched: crate::diagnostics::Fetched,
        cx: &mut Context<Self>,
    ) {
        let file = path.display();
        let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) else {
            println!("S1_EDITOR_DIAGNOSTICS file={file} result=stale reason=buffer-closed");
            return;
        };
        if self.buffers[index].diagnostics_generation != generation {
            println!("S1_EDITOR_DIAGNOSTICS file={file} result=stale reason=superseded");
            return;
        }
        if self.buffers[index].revision != revision {
            println!("S1_EDITOR_DIAGNOSTICS file={file} result=stale reason=content-changed");
            return;
        }

        let written = self.buffers[index].editor.update(cx, |state, cx| {
            let written = crate::diagnostics::apply(state, &fetched.diagnostics);
            // `diagnostics_mut()` 自己**不**通知（上游只在正文变化时 reset），
            // 不写这一句波浪线就要等下一次无关的重绘才出现。
            cx.notify();
            written
        });

        // `severity_max` 让"有没有 Error 级波浪线"不必靠截图判断；`attempts` 让
        // "重取上限够不够"这件事在真机上可观测（打满 5 次说明这一档的等待偏短）。
        // 同步失败单独一行前缀，避免把"服务不可用"混进正常结果里。
        if let Some(error) = &fetched.sync_error {
            println!("S1_EDITOR_DIAGNOSTICS sync_failed file={file} error={error}");
        }
        println!(
            "S1_EDITOR_DIAGNOSTICS file={file} count={written} severity_max={} attempts={} ms={} revision={revision}",
            crate::diagnostics::severity_max(&fetched.diagnostics),
            fetched.attempts,
            fetched.elapsed_ms,
        );
    }

    /// 登记外壳的两个"只有外壳做得了"的动作（见 [`TabMenuHostActions`]）。
    ///
    /// 由 `ShellWorkspace::new` 在**同一个位置**调（`prepare_java` 之后），因为那两个回调要
    /// 捕获外壳自己的弱引用（终端实体 + 底部工具窗可见性）。
    pub fn set_tab_menu_host_actions(&mut self, actions: TabMenuHostActions) {
        self.tab_menu_actions = Some(actions);
    }

    /// 登记工作区根（标签右键菜单的「复制相对路径」要用它）。
    ///
    /// 与 [`Self::prepare_java`] 同一个调用点；分开两个方法是因为职责不同 ——
    /// 那个是"起 Java 语言服务"，这个是"记住一个路径"（后者不启动任何东西，也永不失败）。
    pub fn set_workspace_root(&mut self, root: PathBuf) {
        self.workspace_root = Some(root);
    }

    /// 当前缩进宽度对应的 `TabSize`（两个开 buffer 的路径共用：
    /// 一个值只在这里翻译一次，避免两处各写一份 `hard_tabs: false`）。
    fn tab(&self) -> TabSize {
        TabSize {
            tab_size: self.tab_size,
            hard_tabs: false,
        }
    }

    /// 打开一个文件：读盘、判定类型、更新标签栏与正文。
    ///
    /// **同一路径重复打开只切换活动标签、不再读盘**（沿用上一轮已验证实现的约定，
    /// git HEAD `panels.rs::open_document`）：正文已经在这个 [`Buffer`] 的
    /// `EditorState` 里，重读会白白丢掉撤销栈与光标。
    /// 注意"同路径"是按传入的 `Path` 字面比较；调用方要保证同一个文件每次传同一种写法
    /// （绝对路径就一直绝对路径）。
    ///
    /// 关闭标签会把该 buffer 的 `EditorState`（以及缓存正文）一起丢掉 —— 真机同样会释放
    /// 对应 model，所以"重复打开不读盘"只对**仍然打开着**的标签成立。
    pub fn open(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.to_path_buf();

        if let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) {
            self.activate(index, cx);
            return;
        }

        let name: SharedString = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
            .into();
        let body = read_body(&path, &name);
        let writable = body.writable;
        // 语言名在 `cx.new` **之前**算好：`.language(..)` 是消费 `self` 的 builder
        // （`gpui-base-0.6.6/src/input/base/state.rs:9240`），只能在建状态的那个闭包里链；
        // `cx.new` 之后剩下的只有 `set_highlighter`（同文件 `:772`）。
        // 认不出的扩展名给 `None` → 不调 `.language(..)` → 与"没接高亮"时一样是纯文本。
        let language = language_for_file(&name);
        // `TabSize` 是 `Copy`：在 `cx.new` 之前取一份，闭包里直接用（不借用 `self`）。
        let tab = self.tab();
        // 补全 provider：**只给 Java 文件装**（`language == Some("java")`）。
        //
        // 为什么不是"所有文件都装"：JDTLS 只会答 Java，非 Java 文件拿到的是 Core 的轻量兜底
        // （当前文件标识符），在 `.md` / `.toml` 上弹这个菜单纯属打扰。
        // `JDTLS 优先、轻量兜底` 这条口径在 provider 内部，见 `crate::completion` 的模块文档。
        //
        // ⚠️ **必须在 `open()` 里就装**（而不是只依赖 `prepare_java`）：外壳是先建编辑区、
        // 打开文件，再（后台）起 Java 服务；`prepare_java` 之后还会给**已经打开的** buffer
        // 补装一次（见那里的注释）。两条路都覆盖，"先开文件后起服务"时菜单才不会永远不出现。
        let completion_provider = (language == Some("java")).then(|| {
            crate::completion::JavaCompletionProvider::new(
                self.java.clone(),
                path.clone(),
                self.auto_completion.clone(),
            )
        });

        // 快速修复 provider：与补全**同一个判据的一部分**，但多一条 ——
        // `crate::code_actions::install` 只在拿到服务句柄时才装（理由在 `install` 的文档：
        // 它没有兜底数据源，装一个永远为空的 provider 会把右键菜单那一项变成"可用但没反应"）。
        // "先开文件、后起服务"那一档由 `prepare_java()` 补装（与补全同一条路径）。
        let code_action_provider = (language == Some("java"))
            .then(|| self.java.clone())
            .flatten()
            .map(|service| {
                crate::code_actions::JavaCodeActionProvider::new(service, path.clone())
            });

        // 一个标签一个 `EditorState`：先建状态再灌正文，然后才入列。
        let editor = cx.new(|cx| {
            // `searchable` 上游默认就是 `true`
            // （`gpui-base-0.6.6/src/input/base/state.rs:9233` 的 `EditorState::new`），
            // 显式写出来是把 C（`Ctrl+F` 查找替换）依赖的前提钉在调用点：
            // **不 searchable 的编辑器不拦截 `Ctrl-F`**，会冒泡到上层
            // （`gpui/docs/gpui-kit/0.6.6/zh-CN/component/editor.md:158-159`）。
            let mut state = EditorState::new(window, cx).searchable(true);
            // 语法高亮：只有"语言名 + 该语言的 grammar feature"两件都齐才会真的上色
            // （grammar 开关在 `Cargo.toml`，名字表在 `buffer.rs::language_for_file`）。
            if let Some(language) = language {
                state = state.language(language);
            }
            // 缩进宽度：新 buffer 直接带上当前的设置值（`set_tab_size` 是 `&mut self`，
            // 所以可以放在 builder 之后）。
            state.set_tab_size(tab, cx);
            // 补全：装 provider + 把菜单调宽（上游默认 320px，Java 签名会被截断，
            // 见 `crate::completion::install`）。
            crate::completion::install(&mut state, completion_provider, window.rem_size());
            // 快速修复（`Ctrl+.` / 右键菜单的 Show Code Actions）：菜单与键位归上游，
            // 我们只给 provider，并且**由我们把编辑落进 buffer**（上游不处理 `edit`，
            // 见 `crate::code_actions` 的模块文档）。
            crate::code_actions::install(&mut state, code_action_provider);
            if !writable {
                // 说明性正文（读不到 / 超大 / 二进制 / 非 UTF-8）不可写：只读能避免
                // "用户以为改了、其实保存的是说明文案"这种更坏的结果。
                // ⚠️ 只读与语言**正交**（`set_readonly` vs `language`）：说明文案本来就用
                // `// ` 前缀，被当成注释着上色反而更清楚地表明"这不是文件内容"。
                state.set_readonly(true, cx);
            }
            state
        });
        editor.update(cx, |state, cx| state.set_value(body.text, window, cx));
        // 诊断：这条读的是 `EditorState` 里**真正存着**的语言名，所以它证明 `.language(..)`
        // 落到了状态上（`None` = 认不出的扩展名，走纯文本）；但它证明不了"正文真的亮了"
        // —— 那要 grammar feature 也开着，靠截图与像素统计取证（`S1_EDITOR_CURSOR` 同一套口径）。
        println!(
            "S1_EDITOR_LANG name={} file={name}",
            editor.read(cx).language_name()
        );

        // 订阅必须在 `set_value` **之后**：`set_value` 内部关掉了事件发射
        // （`gpui-base-0.6.6/src/input/base/state.rs:904-907` 的 `emit_events = false`），
        // 但顺序反过来仍然更容易踩坑，所以固定成"先灌正文、再订阅"。
        let subscriptions = vec![
            // A：脏标记 + 防抖自动保存。
            cx.subscribe_in(
                &editor,
                window,
                |pane: &mut Self,
                 editor: &gpui_kit::Entity<EditorState>,
                 event: &InputEvent,
                 window,
                 cx| {
                    if matches!(event, InputEvent::Change) {
                        pane.on_input_change(editor, window, cx);
                    }
                },
            ),
            // B：光标位置。光标移动不发事件（`InputEvent` 只有 Change / PressEnter /
            // Focus / Blur），但会 `cx.notify()`（例如 `select_to`，
            // `gpui-base-0.6.6/src/input/base/state.rs:3039`），所以走观察。
            cx.observe(&editor, |pane: &mut Self, _editor, cx| pane.sync_cursor(cx)),
        ];

        // 实参从左到右求值，图标要在 `name` 被移进构造函数之前算好。
        let icon = icon_for_file(&name, cx);
        self.buffers
            .push(Buffer::new(path, name, icon, editor, writable, subscriptions));
        self.active = Some(self.buffers.len() - 1);
        self.sync_cursor(cx);
        // 诊断：新开的 Java buffer 立刻排一次取回。`language` 是刚算出来的语言名，
        // 与 `schedule_diagnostics` 内部用的判据同源；重复判一次是为了让"非 Java 文件
        // 一眼看不出会不会去问服务端"这件事在调用点上就明确。
        if language == Some("java") {
            let opened = self.buffers.len() - 1;
            // `Duration::ZERO`：打开档没有连打要合并（退避的第一档已经承担了等待）。
            self.schedule_diagnostics(opened, Duration::ZERO, cx);
        }
        cx.notify();
    }

    /// 当前活动 buffer 的文件名（没有打开任何 buffer 时是空串）。
    ///
    /// 给外壳的状态栏用：那一格的前导图标是**文件类型图标**
    /// （真机 `file-path-breadcrumb.tsx:196-202` 的 `ThemedFileIcon`），所以外壳需要
    /// 文件名去查图标主题。**只回文件名不回路径**：调用点只需要查表，
    /// 而完整路径是本视图的 `Buffer::path`，暴露它会把"谁拥有磁盘路径"这条边界弄糊。
    ///
    /// 与 [`Self::cursor_position`] 同一口径：只读、不触发重绘，外壳通过
    /// `cx.observe(&editor_pane, ..)` 在活动 buffer 变化时重绘（见 `open` / `activate`）。
    ///
    /// **不需要 `&App`**，所以签名里没有它：文件名就存在 `Buffer::name` 上，
    /// 与 [`Self::cursor_position`] 不同（那个要读 `EditorState`）。
    pub fn active_buffer_name(&self) -> String {
        self.active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| buffer.name.to_string())
            .unwrap_or_default()
    }

    /// 打开着的**磁盘文件**的路径，顺序 = 标签顺序（`jdt://` 虚拟 buffer 不在其中）。
    ///
    /// 给外壳的会话落盘用（`.lithe/session.local.json`）：存的是"工作区相对路径"，
    /// 而相对化要工作区根 —— 那是外壳的参数，不是本视图的，所以这里只交出绝对路径。
    ///
    /// **不暴露 `Buffer`**：外壳只需要路径，拿到 `Buffer` 就能改 `EditorState`，
    /// 那会把"谁拥有编辑器状态"这条边界弄糊（与 [`Self::active_buffer_name`] 同一条口径）。
    pub fn open_disk_files(&self) -> Vec<PathBuf> {
        self.buffers
            .iter()
            .filter(|buffer| !is_virtual_source_path(&buffer.path))
            .map(|buffer| buffer.path.clone())
            .collect()
    }

    /// 当前活动 buffer 的**磁盘文件**路径；活动 buffer 是虚拟源码或没有活动 buffer 时 `None`。
    pub fn active_disk_file(&self) -> Option<PathBuf> {
        self.active
            .and_then(|index| self.buffers.get(index))
            .filter(|buffer| !is_virtual_source_path(&buffer.path))
            .map(|buffer| buffer.path.clone())
    }

    /// 按会话记下的路径打开一组文件，并在最后切到 `active` 那个下标（会话恢复用）。
    ///
    /// 三条语义（都是"恢复不能让用户看见一个残缺的界面"逼出来的）：
    ///
    /// - `paths` 里的路径**已经**由调用方（外壳）解析并过滤过磁盘存在性 —— 本视图不再判一遍，
    ///   免得同一件事有两份实现（判据见 `lithe_gpui_shared::workspace_config::resolve_session_files`）；
    /// - `active` 指向**过滤之后**的列表下标：被跳过的文件不在 `paths` 里，所以恢复出来的活动
    ///   标签是"第一个还存在的、位于它之前的那个"或紧随其后的那个（见
    ///   [`SessionRestore::active_index`] 的文档）；
    /// - 一个都没开起来时保持不变（空编辑器还是空编辑器）。
    pub fn restore_session(
        &mut self,
        paths: &[PathBuf],
        active: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> SessionRestore {
        for path in paths {
            // `open` 对**已经开着**的路径只切活动标签、不新建 buffer，两种情况都是我们想要的
            // 结果（会话恢复的语义是"让这些文件都开着"）。
            self.open(path, window, cx);
        }

        let opened = self
            .buffers
            .iter()
            .filter(|buffer| paths.contains(&buffer.path))
            .map(|buffer| buffer.path.clone())
            .collect::<Vec<_>>();
        // 活动标签的三段判据：
        // ① 会话记的下标在恢复出来的列表里 → 用它（这是最常见的一支）；
        // ② 越界（它指向的文件被跳过了）→ 沿用"打开之前那一刻的活动标签"，与 `open`
        //    的既有语义一致，不硬选第一个（那会让用户看到光标跳到一个无关的文件上）；
        // ③ 之前根本没有活动标签（空编辑器）→ 落到第一个。
        let mut active_index = active
            .filter(|index| *index < opened.len())
            .or_else(|| {
                self.active_disk_file()
                    .and_then(|path| opened.iter().position(|candidate| *candidate == path))
            });
        if active_index.is_none() && active.is_none() && !opened.is_empty() {
            active_index = Some(0);
        }
        if let Some(index) = active_index {
            if let Some(path) = opened.get(index) {
                if let Some(position) = self.buffers.iter().position(|buffer| buffer.path == *path) {
                    self.activate(position, cx);
                }
            }
        }

        SessionRestore {
            opened,
            active_index,
        }
    }

    /// 当前活动 buffer 的光标位置（**1 基**行列，状态栏的显示口径）。
    /// 而本视图只在位置**真的变了**的时候 `notify`（见 [`Self::sync_cursor`]）。
    pub fn cursor_position(&self, cx: &App) -> CursorPosition {
        self.active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| {
                // 组件给的是 0 基 `lsp_types::Position`
                // （`gpui-base-0.6.6/src/input/base/state.rs:1232`）。
                let position = buffer.editor.read(cx).cursor_position();
                CursorPosition {
                    line: position.line as usize + 1,
                    column: position.character as usize + 1,
                }
            })
            .unwrap_or(CursorPosition::INITIAL)
    }

    /// `Ctrl+S`：把**活动** buffer 写回磁盘。
    ///
    /// 由 `ShellWorkspace` 根元素的 `SaveBuffer` action 处理器转发进来（见 `install_actions`）。
    /// 说明性正文（[`Buffer::writable`] = `false`）没有可写的内容，直接忽略。
    pub fn save_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            return;
        };
        if !self
            .buffers
            .get(index)
            .is_some_and(|buffer| buffer.writable)
        {
            return;
        }
        self.write_buffer(index, "lithe.editor.saveFailed", window, cx);
    }

    // -----------------------------------------------------------------------
    // 阶段 10 第一批：F12 / Ctrl+单击 / ← →（Core 轻量导航，不启 JDTLS）
    // -----------------------------------------------------------------------

    /// `F12` / 菜单「转到 → 转到定义」：取光标处标识符的定义位置并跳过去。
    ///
    /// 由 [`crate::NavigateToDefinition`] action 的处理器转发进来（见 [`install_actions`]），
    /// 也由主菜单的同一项直接调（`crate::workspace::ShellWorkspace::run_command_id` 的
    /// `CommandId::NavigateToDefinition` 分支）—— 两条入口落到这**同一个**方法上。
    ///
    /// 三步：读光标（**字节偏移**，`state.cursor()`）→ 后台解析目标
    /// （[`crate::navigation::resolve_target`]：`.java` 上优先 JDTLS 语义结果，
    /// 服务不可用时退回第一批的 `lsp.builtinNavigation`）→ 回到前台移动光标。
    /// Core 调用是同步的，所以一定放 `cx.background_spawn`，不能在 UI 线程上直接调
    /// （`gpui/crates/shared/src/core_client.rs:27-28`）。
    pub fn navigate_to_definition(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            println!("S1_NAV_FAILED reason=no-buffer");
            return;
        };
        let Some(buffer) = self.buffers.get(index) else {
            println!("S1_NAV_FAILED reason=no-buffer");
            return;
        };

        let path = buffer.path.clone();
        let revision = buffer.revision;
        let file_path = path.to_string_lossy().to_string();
        // 三次借用分开取：一次 `read` 拿不到两个返回值，而 `Rope` 的克隆很便宜
        // （它是持久化数据结构，`to_string()` 只发生在后台拼请求那一步）。
        let (offset, origin, text) = {
            let state = buffer.editor.read(cx);
            (
                state.cursor(),
                state.cursor_position(),
                state.text().clone(),
            )
        };
        let java = self.java.clone();

        // 同一时刻只有一个请求在飞：替换 `Task` 就是取消上一个
        // （`gpui-pre-scheduler-0.3.6/src/executor.rs:389-390`："If you drop a task it will be
        // cancelled immediately"），`generation` 是第二道闸 —— 万一旧请求的结果还是落了回来，
        // 它也过不了 [`EditorPane::apply_definition`] 的第一段校验。
        let generation = self.nav_generation.wrapping_add(1);
        self.nav_generation = generation;
        self.nav_request = Some(NavRequest {
            generation,
            path: path.clone(),
            revision,
            origin,
        });

        let task = cx.spawn_in(window, async move |pane, cx| {
            let result = cx
                .background_spawn(async move {
                    resolve_target(java.as_deref(), &file_path, &text, offset)
                })
                .await;
            // 编辑区可能已经销毁：`update_in` 返回 `Err` 时静默忽略，不 panic。
            let _ = pane.update_in(cx, |pane, window, cx| {
                pane.apply_definition(generation, result, window, cx)
            });
        });
        self.nav_task = Some(task);
    }

    /// 后台结果回到前台：校验它还算不算数，然后记历史 + 移动光标。
    ///
    /// 三段校验与 [`NavRequest`] 一一对应；任何一段不成立都**不改光标**，
    /// 只留一行 `S1_NAV_FAILED`（跳转失败不能悄悄把用户的光标搬走）。
    fn apply_definition(
        &mut self,
        generation: u64,
        result: Result<Option<NavTarget>, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((origin_path, revision, origin)) = self
            .nav_request
            .as_ref()
            .filter(|request| request.generation == generation)
            .map(|request| (request.path.clone(), request.revision, request.origin))
        else {
            println!("S1_NAV_FAILED reason=stale");
            return;
        };

        let Some(origin_index) = self.buffers.iter().position(|buffer| buffer.path == origin_path)
        else {
            println!("S1_NAV_FAILED reason=buffer-closed");
            return;
        };
        if self.buffers[origin_index].revision != revision {
            println!("S1_NAV_FAILED reason=content-changed");
            return;
        }

        let target = match result {
            Ok(Some(target)) => target,
            // 契约允许"没有目标"：Core 找不到光标处的标识符时 `locations` 是空数组
            // （`rust/lithe-core/src/lsp/lightweight/symbols.rs:164-168`），
            // JDTLS 也会给出空的 `locations`。
            Ok(None) => {
                println!("S1_NAV_FAILED reason=no-target");
                Self::notify_no_target(window, cx);
                return;
            }
            Err(error) => {
                println!("S1_NAV_FAILED reason={error}");
                Self::notify_no_target(window, cx);
                return;
            }
        };

        // 落地：算出目标 buffer 下标 + 编辑器口径的位置 + 诊断行里的 `via`。
        let (index, position, via) = match target {
            NavTarget::Local(position) => (origin_index, position, "builtin"),
            NavTarget::File {
                path,
                line,
                utf16_column,
            } => {
                // 跨文件 / 跨模块：目标文件没开着就先打开（真机同样会打开目标文件）。
                if !self.buffers.iter().any(|buffer| buffer.path == path) {
                    self.open(&path, window, cx);
                }
                let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) else {
                    println!("S1_NAV_FAILED reason=no-target");
                    Self::notify_no_target(window, cx);
                    return;
                };
                // 列换算用**目标文件自己**的正文：UTF-16 列与编辑器的字符列只在 ASCII 上相等
                // （见 `crate::navigation` 的模块文档）。
                let text = self.buffers[index].editor.read(cx).text().clone();
                (
                    index,
                    editor_position(&text, line as usize, utf16_column as usize),
                    "jdtls",
                )
            }
            NavTarget::Virtual {
                uri,
                display_path,
                text,
                line,
                utf16_column,
            } => {
                // 位置先按取回来的正文算（`open_virtual` 会把它 move 走），
                // 之后编辑器里的正文与这里算的完全是同一份内容。
                let position = editor_position(
                    &Rope::from(text.as_str()),
                    line as usize,
                    utf16_column as usize,
                );
                let index = self.open_virtual(&uri, &display_path, text, window, cx);
                (index, position, "jdtls-virtual")
            }
        };

        // 历史记的是**跳转前的位置**，且只在跳转成功后记一次 —— 与真机的顺序一致
        // （先判目标、再 `pushEntry`，`navigation-command-actions.ts:449-467`）。
        self.history.record(JumpEntry {
            path: origin_path,
            position: origin,
        });
        // 位置可能已经被编辑过：读回来的才是**真实**落点，日志与历史都用它。
        let to = self.buffers[index].editor.update(cx, |state, cx| {
            state.set_cursor_position(position, window, cx);
            state.cursor_position()
        });
        self.active = Some(index);

        // `from` / `to` 都是 **1 基**（与 `S1_EDITOR_CURSOR` 的显示口径一致，便于对日志）。
        // `kind=definition` 与第一批同一行格式，新增 `via` 区分结果来源
        // （`builtin` = 轻量导航降级路径，`jdtls` / `jdtls-virtual` = 语义结果）。
        println!(
            "S1_NAV_JUMP from={}:{} to={}:{} kind=definition via={via} file={}",
            origin.line + 1,
            origin.character + 1,
            to.line + 1,
            to.character + 1,
            self.buffers[index].path.display()
        );
        self.sync_cursor(cx);
        cx.notify();
    }

    /// 编辑器正文区上的 `Ctrl+单击`。
    ///
    /// 组件自己**没有**把点击位置换算成文本偏移的公开 API
    /// （`InputBaseState::index_for_mouse_position` 是 `pub(crate)`，
    /// `gpui-base-0.6.6/src/input/base/state.rs:2868-2871`；`resolve_mouse_position` 更窄，
    /// 同文件 `:2879`），但**不需要**：组件的 `on_mouse_down` 已经把光标移到了点击位置
    /// （`state.rs:2311-2315` 的 `move_to_with_affinity`），而包装层的冒泡处理器**在它之后**
    /// 才跑（`gpui-pre-0.3.6/src/window.rs:5778-5787`：bubble 阶段遍历 `.rev()`，
    /// 祖先排在后面）。所以这里读 `state.cursor()` 拿到的就是"点到的那一个字符"——
    /// 与 `F12` 完全同一条路径，不碰组件内部。
    ///
    /// 组件的 `on_mouse_down` 全程不 `stop_propagation()`（`state.rs:2223-2316`），
    /// 所以这个处理器能收到；收不到的位置只有左栏折叠箭头与滚动条
    /// （`input/base/element.rs:1326`、`scrollbar.rs:1716-1718`）。
    fn on_editor_click(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 触发器只有 `Ctrl+单击`：`Modifiers::secondary()` 在 Windows / Linux 上就是 Ctrl，
        // macOS 上是 Cmd（`gpui-pre-0.3.6/src/platform/keystroke.rs:479-493`）。
        if !event.modifiers.secondary() || event.modifiers.alt {
            return;
        }

        // 包装层（[`Self::render_body`] 的滚动容器）比编辑器大：点在编辑器之外时组件的
        // `on_mouse_down` 不会跑，光标还停在上一次的位置 —— 不做这个判据就会"点到空白也跳"。
        // `input_bounds()` 是编辑器自己上报的绝对 bounds（`state.rs:537`）。
        let inside = self
            .active
            .and_then(|index| self.buffers.get(index))
            .is_some_and(|buffer| {
                buffer
                    .editor
                    .read(cx)
                    .input_bounds()
                    .contains(&event.position)
            });
        if !inside {
            return;
        }

        self.navigate_to_definition(window, cx);
    }

    /// 打开（或切到）一个 **JDT 虚拟源码** buffer：`jdt://` 的只读正文。
    ///
    /// 与 [`Self::open`] 的三点不同，都是"这不是一个磁盘文件"的直接后果：
    ///
    /// 1. 正文由调用方给（来自 Core 的 `virtualDocument` 语义请求），**不读盘**；
    /// 2. 去重键是 JDT 的虚拟 URI（`jdt://contents/java.base/java/lang/String.class?=...`），
    ///    同一个类每次跳转都是同一个 URI，所以只会有一个标签；
    /// 3. `writable = false` 且 `set_readonly(true)` —— 契约把 JDT 虚拟位置标成
    ///    `isReadOnly`（`rust/lithe-core/src/lsp/languages/jdt.rs:631-642`），
    ///    而且"反编译出来的源码"写回去没有任何意义。
    ///
    /// ⚠️ **已知边界**：这个 buffer 的正文只来自 Core 的虚拟文档请求，没有"重新读盘"这条路。
    /// 用户把它关掉之后，用 `←` 回到它只会得到一条 `S1_NAV_FAILED reason=buffer-closed`
    /// （见 [`Self::restore_entry`]）—— 想彻底解决要给历史项也保留一份只读正文或异步重取，
    /// 那是后续的事，本轮如实登记。
    fn open_virtual(
        &mut self,
        uri: &str,
        display_path: &str,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> usize {
        let path = PathBuf::from(uri);
        if let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) {
            return index;
        }

        // 显示名取 `display_path` 的最后一段（`java.base/java/lang/String.java` → `String.java`）。
        let name: SharedString = display_path
            .rsplit('/')
            .find(|segment| !segment.is_empty())
            .unwrap_or(display_path)
            .to_string()
            .into();
        // JDT 反编译出来的正文就是 Java 源码，所以走**同一张**语言表：`display_path` 的
        // 扩展名照常判得出 `"java"`，`jdt://` 虚拟源码因此与磁盘上的 `.java` 一样亮。
        let language = language_for_file(&name);
        let tab = self.tab();

        let editor = cx.new(|cx| {
            let mut state = EditorState::new(window, cx).searchable(true);
            if let Some(language) = language {
                state = state.language(language);
            }
            state.set_readonly(true, cx);
            // 只读的 `jdt://` 库源码同样按当前缩进宽度显示（与磁盘上的 buffer 一致）。
            state.set_tab_size(tab, cx);
            state
        });
        editor.update(cx, |state, cx| state.set_value(text, window, cx));
        println!(
            "S1_EDITOR_LANG name={} file={name} virtual=true",
            editor.read(cx).language_name()
        );

        let subscriptions = vec![
            cx.subscribe_in(
                &editor,
                window,
                |pane: &mut Self,
                 editor: &gpui_kit::Entity<EditorState>,
                 event: &InputEvent,
                 window,
                 cx| {
                    if matches!(event, InputEvent::Change) {
                        pane.on_input_change(editor, window, cx);
                    }
                },
            ),
            cx.observe(&editor, |pane: &mut Self, _editor, cx| pane.sync_cursor(cx)),
        ];

        let icon = icon_for_file(&name, cx);
        self.buffers
            .push(Buffer::new(path, name, icon, editor, false, subscriptions));
        // ⚠️ **这里刻意不排诊断刷新**（与 `open` 不同）：这个 buffer 的身份是 `jdt://`
        // 虚拟 URI，它不是磁盘上的文档 —— JDT 不会为它发 `publishDiagnostics`，
        // 而 Core 也会把"文档不在 `open_documents` 里"的发布丢掉
        // （`rust/lithe-core/src/lsp/interface/client.rs:350-352`）。库里反编译出来的源码
        // 本来也不该画诊断波浪线。
        self.buffers.len() - 1
    }
    /// `←`：回到上一个跳转位置。
    ///
    /// ⚠️ **2026-09-27 起没有调用点**：标签栏左侧那对按钮已按维护者口径删除
    /// （见 `render_nav_group` 的墓碑注释）。方法**保留**是为了不把「跳转历史」这条能力
    /// 从 API 上抹掉 —— 历史仍在记录（`F12` / `Ctrl+单击` 都压栈），
    /// 接回来最省事的一条路是绑一对键位（`Alt+←` / `Alt+→`），不要再把按钮画回那个角。
    #[allow(dead_code)]
    fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(present) = self.current_entry(cx) else {
            return;
        };
        let Some(entry) = self.history.go_back(present.clone()) else {
            return;
        };
        self.restore_entry(present, entry, "S1_NAV_BACK", window, cx);
    }

    /// `→`：前进到下一个跳转位置（只可能来自"曾经 `←` 过"的分支）。
    ///
    /// ⚠️ 与 [`EditorPane::go_back`] 同一处置：2026-09-27 起没有调用点，方法保留、标
    /// `#[allow(dead_code)]`。
    #[allow(dead_code)]
    fn go_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(present) = self.current_entry(cx) else {
            return;
        };
        let Some(entry) = self.history.go_forward() else {
            return;
        };
        self.restore_entry(present, entry, "S1_NAV_FORWARD", window, cx);
    }

    /// 把光标挪到 `entry`（`from` 只用于诊断行）。
    ///
    /// 目标 buffer 已经关掉时**重新打开那个文件**：历史项存的是"路径 + 位置"，
    /// 不依赖那个 buffer 还活着（真机的 jump list 同样能在 buffer 被关掉后回到文件）。
    ///
    /// 例外：`jdt://` 虚拟源码的正文只来自 Core 的虚拟文档请求（见 [`Self::open_virtual`]），
    /// 没有"重新读盘"这条路，所以它的 buffer 一旦关掉就只能放弃这次回退并留一行诊断 ——
    /// 不这么做会走 [`Self::open`] 的读盘失败分支，给用户一个"无法打开 jdt://..."的说明 buffer。
    fn restore_entry(
        &mut self,
        from: JumpEntry,
        entry: JumpEntry,
        log_tag: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.buffers.iter().any(|buffer| buffer.path == entry.path) {
            if is_virtual_source_path(&entry.path) {
                println!("S1_NAV_FAILED reason=buffer-closed");
                return;
            }
            self.open(&entry.path, window, cx);
        }
        let Some(index) = self
            .buffers
            .iter()
            .position(|buffer| buffer.path == entry.path)
        else {
            println!("S1_NAV_FAILED reason=buffer-closed");
            return;
        };

        // 位置可能已经被编辑过：`set_cursor_position` 自己按行内容夹住列，
        // 读回来的才是**真实**落点，日志用它而不是用请求值。
        let to = self.buffers[index].editor.update(cx, |state, cx| {
            state.set_cursor_position(entry.position, window, cx);
            state.cursor_position()
        });
        self.active = Some(index);

        println!(
            "{log_tag} from={}:{} to={}:{}",
            from.position.line + 1,
            from.position.character + 1,
            to.line + 1,
            to.character + 1
        );
        self.sync_cursor(cx);
        cx.notify();
    }

    /// 活动 buffer 当前的光标位置，包成历史项（路径就是它的身份）。
    fn current_entry(&self, cx: &App) -> Option<JumpEntry> {
        let index = self.active?;
        let buffer = self.buffers.get(index)?;
        Some(JumpEntry {
            path: buffer.path.clone(),
            position: buffer.editor.read(cx).cursor_position(),
        })
    }

    /// 跳转失败时给一条反馈。
    ///
    /// 真机在这一档是 `toast.info("未找到" + 目标种类)`
    /// （`windows/tauri/src/features/keymaps/commands/navigation-command-actions.ts:449-456`），
    /// 文案与插值直接复用真源既有的两条键：`navigation.noTargetFound` +
    /// `navigation.definition`（`windows/tauri/src/i18n/locale.ts:8435,8439`）。
    /// 静默失败会让用户以为 `F12` 坏了，所以这一条**要**给。
    fn notify_no_target(window: &mut Window, cx: &mut Context<Self>) {
        window.push_notification(
            Notification::info(tr_args(
                "lithe.navigation.noTargetFound",
                &[("target", tr("lithe.navigation.definition").as_ref())],
            )),
            cx,
        );
    }

    /// 切到第 `index` 个标签（重复打开同一路径、点标签都走这里）。
    fn activate(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.buffers.len() {
            return;
        }
        self.active = Some(index);
        // 诊断：这行是"点 × **没有**顺带激活那个标签"的机器判据（B1 的验收项）。
        // 点 × 的 click 若不 `stop_propagation` 就会冒泡到 `TabBar::on_click` →
        // 这里，于是关掉一个**非活动**标签之后活动标签会被抢到那个下标上。
        // 光看界面很难分辨（关掉之后相邻标签本来就会接管高亮），所以留一行可 grep 的。
        eprintln!(
            "S1_EDITOR_ACTIVE index={index} file={}",
            self.buffers[index].name
        );
        // 状态栏的光标项要立刻跟着换 buffer，不能等下一次编辑器通知。
        self.sync_cursor(cx);
        cx.notify();
    }

    /// 光标位置变了就通知外壳重绘，并留一行可 grep 的诊断。
    ///
    /// 光标位置是渲染出来的小字，靠截图逐像素核对不可靠，所以状态一变就打
    /// `S1_EDITOR_CURSOR`（与 `S1_EXPLORER` / `S1_SETTINGS` 同一套探针口径）。
    fn sync_cursor(&mut self, cx: &mut Context<Self>) {
        let position = self.cursor_position(cx);
        if self.cursor == Some(position) {
            return;
        }
        self.cursor = Some(position);
        println!(
            "S1_EDITOR_CURSOR line={} col={}",
            position.line, position.column
        );
        cx.notify();
    }

    /// `InputEvent::Change`：置脏并排一次防抖自动保存。
    fn on_input_change(
        &mut self,
        editor: &gpui_kit::Entity<EditorState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self
            .buffers
            .iter()
            .position(|buffer| buffer.editor.entity_id() == editor.entity_id())
        else {
            return;
        };
        // 只读的说明性 buffer 不会发 Change（没有编辑动作能被接受），这里是第二道闸。
        if !self.buffers[index].writable {
            return;
        }

        if !self.buffers[index].is_dirty {
            self.buffers[index].is_dirty = true;
            cx.notify();
        }
        // 修订号无条件 +1（与脏标记解耦）：在飞的跳转请求靠它判断"结果回来时正文已经变了"。
        self.buffers[index].revision = self.buffers[index].revision.wrapping_add(1);

        // 诊断：正文一变就排一次重取。**必须在这里排**（而不是等自动保存之后再排）：
        // 自动保存是 150ms 防抖 + 可能失败，而 JDT 要的是 `didChange`，与磁盘无关。
        // 防抖窗口把连打合并成一次（见 `crate::diagnostics::CHANGE_DEBOUNCE`）。
        self.schedule_diagnostics(index, crate::diagnostics::CHANGE_DEBOUNCE, cx);

        if AUTO_SAVE_ENABLED {
            self.schedule_auto_save(index, window, cx);
        }
    }

    /// 排一次防抖自动保存。
    ///
    /// 两件事都与 Windows 一一对应：**换掉待执行的任务**（`Task` drop 即取消）等价于
    /// `clearTimeout`（`editor-app.store.ts:505-509`）；**版本号 +1** 是 stale-content 守卫
    /// 的等价物（同文件 `:518-529`，理由见 [`Buffer::save_generation`]）。
    fn schedule_auto_save(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let generation = self.buffers[index].save_generation.wrapping_add(1);
        self.buffers[index].save_generation = generation;

        let task = cx.spawn_in(window, async move |pane, cx| {
            cx.background_executor().timer(AUTO_SAVE_DEBOUNCE).await;
            // 编辑区可能已经销毁：`update_in` 返回 `Err` 时静默忽略，不 panic。
            let _ = pane.update_in(cx, |pane, window, cx| {
                pane.run_auto_save(index, generation, window, cx)
            });
        });
        self.buffers[index].auto_save_task = Some(task);
    }

    /// 防抖窗口结束时真的写盘。
    fn run_auto_save(
        &mut self,
        index: usize,
        generation: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        // stale-content 守卫：这一代已经被后来的改动替换（或者已经被手动保存过），
        // 交给新的那一次写，这里什么都不做。
        if buffer.save_generation != generation || !buffer.is_dirty {
            return;
        }
        self.write_buffer(index, "lithe.editor.autoSaveFailed", window, cx);
    }

    /// 把 buffer 的正文写回磁盘；成功返回 `true`。
    ///
    /// 失败**不静默**（《编码指南》「Handle failures explicitly」）：内容留在编辑器里、
    /// `is_dirty` 保持 `true`、打一行 `S1_EDITOR_SAVE_FAILED`，并推一条通知。
    /// 文案用 Windows 的 `editor.saveFailed` / `editor.autoSaveFailed`
    /// （`windows/tauri/src/i18n/locale.ts:4637-4638`；英文侧同键 `:176-177`），
    /// 手动保存与自动保存各用一条 —— 两句话说的是不同的后果（"重试"vs"更改仍在编辑器里"）。
    fn write_buffer(
        &mut self,
        index: usize,
        failure_key: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(buffer) = self.buffers.get(index) else {
            return false;
        };
        let path = buffer.path.clone();
        let name = buffer.name.clone();
        let value = buffer.editor.read(cx).value();

        match std::fs::write(&path, value.as_bytes()) {
            Ok(()) => {
                if let Some(buffer) = self.buffers.get_mut(index) {
                    buffer.is_dirty = false;
                    // 已经在盘上了：待执行的自动保存没有意义，顺手丢掉。
                    buffer.auto_save_task = None;
                }
                println!(
                    "S1_EDITOR_SAVED path={} bytes={}",
                    path.display(),
                    value.len()
                );
                cx.notify();
                true
            }
            Err(error) => {
                eprintln!(
                    "S1_EDITOR_SAVE_FAILED path={} error={error}",
                    path.display()
                );
                window.push_notification(
                    // 保存失败是**错误**而不是普通提示：用 error 档（自带语义色与图标）。
                    Notification::error(tr_args(failure_key, &[("name", name.as_ref())])),
                    cx,
                );
                false
            }
        }
    }

    /// 关闭一个标签。
    ///
    /// 关掉最后一个标签后编辑区回到空状态（`windows/tauri/src/features/panes/components/pane-container.tsx:1100`）；
    /// 关掉活动标签时顺位接上它的邻居（真机就是"关掉后激活相邻标签"）。
    ///
    /// ⚠️ 调用方要在之后调一次 [`Self::sync_cursor`]：`self.cursor` 是"上次报出去的值"，
    /// 不刷新的话关标签后状态栏会停在旧 buffer 的行列上，直到下一次编辑器通知。
    fn close(&mut self, index: usize) {
        if index >= self.buffers.len() {
            return;
        }

        // 记进"最近关闭"栈：**所有**关闭路径（点 ×、右键关闭、菜单、批量、`Ctrl+W`）都经
        // 这里，所以"重新打开已关闭标签页"能覆盖全部入口，不必在每个入口各记一次。
        // 虚拟源码不进栈（理由见 [`Self::closed`] 的字段文档）。
        let closed = self.buffers[index].path.clone();
        if !is_virtual_source_path(&closed) {
            self.closed.push(closed);
            // 有界：菜单的"重新打开"永远只回溯最近若干个，留着无限长的历史只是内存泄漏
            // （真源 `closedBuffers` 同样是有界队列）。
            if self.closed.len() > MAX_CLOSED_TABS {
                self.closed.remove(0);
            }
        }

        // 被关掉的标签之后的标签整体左移一位，所以旧下标大于 index 的要减一。
        let shift = |active: usize| if active > index { active - 1 } else { active };
        self.buffers.remove(index);

        self.active = if self.buffers.is_empty() {
            None
        } else {
            Some(
                self.active
                    .map(shift)
                    .unwrap_or(index)
                    .min(self.buffers.len() - 1),
            )
        };
    }

    /// E：请求关闭第 `index` 个标签 —— **有未保存修改时先确认**。
    ///
    /// 干净的标签直接关，不打扰用户（真机也只有 `isDirty` 才拦：
    /// `windows/tauri/src/features/editor/stores/buffer.store.ts:1746` 找的就是
    /// `isEditorContent(b) && b.isDirty`）。
    ///
    /// ⚠️ 浮层只能在事件回调或任务里打开；本函数由关闭按钮 / 菜单项的 `on_click` 调用，
    /// 满足约束（`render` 阶段调 `window.open_dialog` 会 panic）。
    fn request_close(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };

        if !buffer.is_dirty {
            eprintln!(
                "S1_TAB_MENU run=close tab={} dirty=false",
                buffer.name.as_ref()
            );
            self.close(index);
            self.sync_cursor(cx);
            cx.notify();
            return;
        }

        // 正文只补"是哪个文件"（指南 `design-guides.md:427-434`：标题写决策、正文写范围、
        // 按钮写结果，不用"您确定要……吗"）。
        let body = tr_args(
            "lithe.editor.gpui.unsavedChangesBody",
            &[("name", buffer.name.as_ref())],
        );
        self.open_unsaved_dialog(PendingConfirm::Single(index), body, window, cx);
    }

    /// `Ctrl+W`：关闭**当前**标签（真源 `file.close` → `closeActiveTab`，
    /// `windows/tauri/src/features/keymaps/commands/file-command-actions.ts:75-86`）。
    ///
    /// **不复刻**真机"没有任何 buffer 时关窗口"那一支（`:85`）：没有 buffer 时直接返回，
    /// 与 [`Self::request_close`] 的既有边界一致（见 `gpui/research/windows/09-tab-context-menu.md`
    /// §2.4 结论 3）。
    pub fn close_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.active else {
            return;
        };
        self.request_close(index, window, cx);
    }

    // -----------------------------------------------------------------------
    // 主菜单「文件 → 关闭…」的一批入口（B1）
    //
    // 主菜单里的这几条与标签右键菜单里的同名项**落同一条链**（`close_scope` →
    // 一次性确认 → `apply_close_scope`），不重写第二份。差别只有锚点从哪来：
    // 右键菜单用"被右键的那个标签"，主菜单用**当前活动标签**（真源
    // `file-command-actions.ts` 的 `closeOtherTabs` / `closeTabsToLeft` 同样是"以当前
    // 编辑器为锚"）。
    //
    // 诊断行沿用 `S1_TAB_MENU run=<tag>`：这是**同一条关闭链**的同一个可 grep 契约，
    // 多一个前缀只会让"关标签"这件事有两个名字。
    // -----------------------------------------------------------------------

    /// 当前活动标签的路径（锚点式批量的输入）。
    fn active_path(&self) -> Option<PathBuf> {
        self.active
            .and_then(|index| self.buffers.get(index))
            .map(|buffer| buffer.path.clone())
    }

    /// 锚点式批量关闭的公共前半段：没有活动标签就**如实打一行**并返回（不静默什么都不做）。
    fn close_scope_at_active(
        &mut self,
        make: impl FnOnce(PathBuf) -> CloseScope,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(anchor) = self.active_path() else {
            eprintln!("S1_TAB_MENU run=close reason=no-active");
            return;
        };
        self.close_scope(make(anchor), window, cx);
    }

    /// 主菜单「关闭其他标签页」：以**当前活动标签**为锚。
    pub fn close_other_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_scope_at_active(CloseScope::Others, window, cx);
    }

    /// 主菜单「关闭左侧标签页」：以**当前活动标签**为锚。
    pub fn close_tabs_to_left(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_scope_at_active(CloseScope::ToLeft, window, cx);
    }

    /// 主菜单「关闭右侧标签页」：以**当前活动标签**为锚。
    pub fn close_tabs_to_right(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_scope_at_active(CloseScope::ToRight, window, cx);
    }

    /// 主菜单「关闭所有标签页」。
    pub fn close_all_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_scope(CloseScope::All, window, cx);
    }

    /// 主菜单「关闭已保存标签页」：不脏的全关，脏的**一个都不动**。
    ///
    /// 判据是「哪些该关」，所以脏标签根本不进 `targets` —— 于是这条路径永远不会弹
    /// 未保存确认框（真源 `handleCloseSavedTabs` 同样不过确认）。
    pub fn close_saved_tabs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_scope(CloseScope::Saved, window, cx);
    }

    /// 主菜单「重新打开已关闭标签页」：从 LIFO 栈取最近一个已关闭的磁盘文件重开。
    ///
    /// 走**同一条** [`Self::open`]（读盘 → 建状态 → 排诊断 → 切活动标签），所以重开出来的
    /// 标签与"从文件树点开"完全一样（不是一份只还原标题的空壳）。
    ///
    /// 栈空时如实打一行（`result=empty`）而不是静默：菜单项点了没反应必须能从日志里分辨出
    /// "本来就是空的"和"接线断了"（与 `close_scope` 的空集诊断同一条口径）。
    pub fn reopen_closed_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.closed.pop() else {
            eprintln!("S1_TAB_MENU run=reopenClosed result=empty");
            return;
        };
        eprintln!("S1_TAB_MENU run=reopenClosed path={}", path.display());
        self.open(&path, window, cx);
    }

    // -----------------------------------------------------------------------
    // 标签右键菜单（`gpui/research/windows/09-tab-context-menu.md`）
    //
    // 真源是 `windows/tauri/src/features/tabs/components/tab-context-menu.tsx`（13 项 +
    // 4 条分隔线）。本侧画 **9 项 + 1 条分隔线**，缺的 4 项按"条件不满足就整项不出现"落地
    // （真源本身就是 spread 掉的，且**整份菜单没有任何禁用项**，所以这里也不置灰）：
    //
    // | 真源项 | 本侧 | 理由 |
    // | --- | --- | --- |
    // | `pin`（固定/取消固定标签页） | **不画** | 本侧 `Buffer` **没有** `is_pinned`（pin 能力不存在：`buffer.rs` 的字段表、`display_names` 的排序、关闭按钮的形态替换三处都缺）。真源里 pin 被 `!isPinned` 三道过滤保护（`buffer.store.ts:1744/1763/1815`），做一半只会留下一个死项 |
    // | `rename-terminal`（重命名） | **不画** | 只对终端标签出现，而本菜单只挂在编辑区标签上（终端页签在 `TerminalPane` 里，另有自己的面板） |
    // | `split-right` / `split-down` | **不画** | 依赖 pane 树（分屏），本侧 `EditorPane` 是扁平 `buffers: Vec<Buffer>` + 单窗格渲染 |
    // | `toggle-editor-group-lock` | **不画** | 同上：没有第二个编辑器组可路由，切 bool 也没有任何可观察后果 |
    //
    // 分隔线：真源有 4 条（`sep-1` / `sep-2` / `sep-3` / `sep-lock`），后三条都依附于上面
    // 不画的那几项，只剩 `sep-3`（"重新加载"与"关闭"之间）有意义 —— **保留那一条**。
    // -----------------------------------------------------------------------

    /// 「复制路径」：把绝对路径写进剪贴板（真机 `writeClipboardText(buffer.path)`，
    /// `windows/tauri/src/features/tabs/components/tab-bar.tsx:353-355`）。
    fn copy_path(&self, index: usize, cx: &mut App) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        let text = buffer.path.to_string_lossy().to_string();
        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        eprintln!("S1_TAB_MENU run=copyPath value={text}");
    }

    /// 「复制相对路径」：相对工作区根；**没有根时拷全路径**（真机同口径，
    /// `tab-bar.tsx:355-368`）。
    fn copy_relative_path(&self, index: usize, cx: &mut App) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        let text = relative_to_workspace(&buffer.path, self.workspace_root.as_deref());
        cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        eprintln!("S1_TAB_MENU run=copyRelativePath value={text}");
    }

    /// 「在资源管理器中显示」：交给外壳（起 `explorer.exe` 属平台层）。
    fn reveal_in_explorer(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        let path = buffer.path.clone();
        // 先把 `Arc` 抠出来：调用它要 `&mut App`，而 `self.tab_menu_actions` 还借着 `self`。
        let Some(reveal) = self
            .tab_menu_actions
            .as_ref()
            .map(|actions| actions.reveal.clone())
        else {
            eprintln!("S1_TAB_MENU run=reveal result=no-host");
            return;
        };
        (reveal)(&path, window, cx);
        eprintln!("S1_TAB_MENU run=reveal path={}", path.display());
    }

    /// 「在终端中打开」：取文件**所在目录**，交给外壳开一个终端页签并显示底部工具窗
    /// （真机 `getDirName(buffer.path)` → 以该目录起终端 buffer，`tab-bar.tsx:157-165`）。
    fn open_in_terminal(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        let path = buffer.path.clone();
        let Some(open_terminal) = self
            .tab_menu_actions
            .as_ref()
            .map(|actions| actions.open_terminal.clone())
        else {
            eprintln!("S1_TAB_MENU run=openInTerminal result=no-host");
            return;
        };
        (open_terminal)(&path, window, cx);
    }

    /// 「重新加载」：重读磁盘、丢掉撤销栈与滚动位置，与真机"关掉再重开"
    /// （`tab-bar.tsx:715-739`）的可见结果一致。
    ///
    /// ⚠️ **脏标签先过确认**：真机走的是 `closeBuffer`（不是 force），所以脏标签会先弹确认
    /// （`buffer.store.ts:1348-1365`）；直接重读会把用户的修改无声丢掉。
    fn request_reload(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get(index) else {
            return;
        };
        if !buffer.is_dirty {
            self.reload_buffer(index, window, cx);
            return;
        }
        let body = tr_args(
            "lithe.editor.gpui.unsavedChangesBody",
            &[("name", buffer.name.as_ref())],
        );
        self.open_unsaved_dialog(PendingConfirm::Reload(index), body, window, cx);
    }

    /// 真的重读第 `index` 个 buffer 的正文。
    ///
    /// 三件事一起重置，缺一件都会留下不一致：`is_dirty`（读回来的就是磁盘内容）、
    /// `revision`（在飞的跳转请求按旧正文算的行列已经无意义）、`save_generation`
    /// 与 `auto_save_task`（待执行的自动保存会把这**旧**正文写回刚读出来的文件）。
    fn reload_buffer(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(buffer) = self.buffers.get_mut(index) else {
            return;
        };
        let path = buffer.path.clone();
        let name = buffer.name.clone();
        let body = read_body(&path, &name);
        let writable = body.writable;
        let bytes = body.text.len();

        buffer.is_dirty = false;
        buffer.writable = writable;
        buffer.revision = buffer.revision.wrapping_add(1);
        buffer.save_generation = buffer.save_generation.wrapping_add(1);
        buffer.auto_save_task = None;

        let editor = buffer.editor.clone();
        editor.update(cx, |state, cx| {
            // `set_value` 内部关掉了事件发射（`gpui-base-0.6.6/src/input/base/state.rs:904-907`），
            // 所以这一步不会把刚清掉的脏标记又打回来。
            state.set_value(body.text, window, cx);
            // 文件可能刚刚变得可读 / 不可读（例如权限变了、或"读不到"的文件后来出现了），
            // 只读态必须跟着**这一次**读盘结果走，不能停在打开时那一档。
            state.set_readonly(!writable, cx);
        });

        eprintln!(
            "S1_TAB_MENU run=reload path={} bytes={bytes}",
            path.display()
        );
        self.sync_cursor(cx);
        // 诊断：正文被整篇换掉，旧诊断的范围已经无意义 —— 重新同步一次再取。
        self.schedule_diagnostics(index, Duration::ZERO, cx);
        cx.notify();
    }

    /// 批量关闭类入口的公共实现：**算集合 → 脏的一次性确认 → 整批关**。
    ///
    /// 两类调用方都汇合在这里：标签右键菜单的三项（关闭其他 / 右侧 / 全部）与主菜单
    /// 「文件 → 关闭…」那一批（见 [`Self::close_other_tabs`] 等入口）。
    ///
    /// ⚠️ **不能逐个调 [`Self::request_close`]**：那个每调一次就 `open_dialog`，
    /// 3 个脏标签会叠 3 个对话框；真机是"**只问一次**"
    /// （`buffer.store.ts:1746-1755` 找到**第一个**脏的就 `return`，其余一条都不问）。
    fn close_scope(&mut self, scope: CloseScope, window: &mut Window, cx: &mut Context<Self>) {
        let targets = scope.targets(&self.buffers);
        if targets.is_empty() {
            // 只有 1 个标签时"关闭其他 / 关闭右侧"**静默空转**：不报错、不弹窗、不置灰
            // （真源 `buffer.store.ts:1758` 的 `forEach` 空转、`:1813` 的 `index === -1` 早退）。
            eprintln!("S1_TAB_MENU run={} closed=0 reason=empty", scope.tag());
            return;
        }

        let mut dirty = 0usize;
        for index in &targets {
            if self.buffers[*index].is_dirty {
                dirty += 1;
            }
        }
        if dirty == 0 {
            let closed = self.apply_close_scope(&scope, cx);
            eprintln!("S1_TAB_MENU run={} closed={closed} dirty=0", scope.tag());
            return;
        }

        // 正文如实说"有几个文件"（维护者口径）—— 真源只显示**第一个**脏文件名，
        // 用户看到"A 没保存"，确认后 B、C 也一起被关掉且没被问过
        // （`pending-buffer-close-dialog.tsx:9-12` + `buffer.store.ts:1746` 的 `find`）。
        eprintln!(
            "S1_TAB_MENU run={} confirm=true dirty={dirty} targets={}",
            scope.tag(),
            targets.len()
        );
        let count = dirty.to_string();
        let body = tr_args(
            "lithe.editor.gpui.unsavedChangesBatchBody",
            &[("count", &count)],
        );
        self.open_unsaved_dialog(PendingConfirm::Batch(scope), body, window, cx);
    }

    /// 按当前这份顺序**重算**要关的下标并整批关掉，返回真的关掉了几个。
    ///
    /// **重算**（维护者口径）：确认对话框弹出期间标签可能已经增减、顺序可能变，
    /// 拿点击那一刻的下标去关会关错人。锚点是**路径**，所以重算永远落在用户当初右键的那个
    /// 标签上，而不是"第 N 个位置"。
    fn apply_close_scope(&mut self, scope: &CloseScope, cx: &mut Context<Self>) -> usize {
        let targets = scope.targets(&self.buffers);
        // **从大到小**关：`close` 会把后面的标签整体左移一位，先关小下标会让后面的下标错位。
        for index in targets.iter().rev() {
            self.close(*index);
        }
        self.sync_cursor(cx);
        cx.notify();
        targets.len()
    }

    /// 未保存确认对话框 —— 「关闭单个 / 批量关闭 / 重新加载」三种收尾**共用同一个**。
    ///
    /// `body` 已经拼好（单个档是文件名、批量档是"有 N 个文件"），所以这个函数只负责
    /// 标题 + 三个按钮与各自的收尾动作。写盘失败的通知由 `write_buffer` 自己拼文件名。
    ///
    /// ⚠️ **所有捕获值都在 `Fn` 闭包体内 `clone`**：`WindowExt::open_dialog` 收的是
    /// `Fn(Dialog, ..) -> Dialog`（可被多次调用），所以内层 `move` 闭包不能把外层捕获的
    /// 变量整个搬走（E0507）。
    fn open_unsaved_dialog(
        &self,
        pending: PendingConfirm,
        body: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pane = cx.weak_entity();
        // 批量档的主按钮是"全部保存"（真源键 `menu.saveAll` = 「全部保存」/ "Save All"），
        // 单个档沿用既有的「保存」（`ui.save`，阶段 9 已接线）。
        let save_label = match &pending {
            PendingConfirm::Batch(_) => tr("lithe.menu.saveAll"),
            _ => tr("lithe.ui.save"),
        };

        window.open_dialog(cx, move |dialog, _, cx| {
            let discard_pane = pane.clone();
            let save_pane = pane.clone();
            let discard_pending = pending.clone();
            let save_pending = pending.clone();
            let body = body.clone();
            let save_label = save_label.clone();
            dialog
                // 标题 = 状态/条件，正文 = 作用范围（哪个文件 / 有几个文件），按钮 = 结果词。
                // 指南 `design-guides.md:427-432`：确认对话框要组成一个紧凑决策；
                // `:434` 明确不用"您确定要……吗"这类套话。
                .title(tr("lithe.unsavedChanges.title"))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(body),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        // 按钮顺序照指南「操作使用"取消"和结果词」；破坏性结果放中间并保持
                        // 次样式，主操作（保存）最右且 primary —— 与设置对话框的
                        // 「恢复默认设置」确认框同一套排法（`settings/src/dialog.rs:586-617`）。
                        .child(
                            Button::new("editor-unsaved-cancel")
                                .label(tr("lithe.ui.cancel"))
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("editor-unsaved-discard")
                                // 真机的字是「不保存」（`unsavedChanges.doNotSave`，
                                // `windows/tauri/src/i18n/locale.ts:7873`），但设计指南
                                // `design-guides.md:421` 对"未保存修改"这一档明确推荐
                                // **「放弃修改」**（"不保存"看不出放弃了什么），所以这里用
                                // gpui 侧自有键，理由登记在 `extract-locale.mjs` 的
                                // `GPUI_ONLY_KEYS`。
                                .label(tr("lithe.editor.gpui.discardChanges"))
                                .on_click(move |_, window, cx| {
                                    let pending = discard_pending.clone();
                                    let _ = discard_pane.update(cx, |pane, cx| {
                                        pane.apply_confirm(pending, window, cx)
                                    });
                                    window.close_dialog(cx);
                                }),
                        )
                        .child(
                            Button::new("editor-unsaved-save")
                                .primary()
                                .label(save_label)
                                .on_click(move |_, window, cx| {
                                    // 写失败**不关**对话框：用户还可以重试或改成"放弃修改"，
                                    // 失败原因由 `write_buffer` 推的通知说明。
                                    let pending = save_pending.clone();
                                    let saved = save_pane
                                        .update(cx, |pane, cx| {
                                            pane.save_confirm(pending, window, cx)
                                        })
                                        .unwrap_or(false);
                                    if saved {
                                        window.close_dialog(cx);
                                    }
                                }),
                        ),
                )
        });
    }

    /// 「放弃修改」按下之后：不保存、直接执行收尾（这里**没有** dirty 检查 ——
    /// 用户刚刚明确选择丢弃）。
    fn apply_confirm(&mut self, pending: PendingConfirm, window: &mut Window, cx: &mut Context<Self>) {
        match pending {
            PendingConfirm::Single(index) => {
                self.close(index);
                self.sync_cursor(cx);
                cx.notify();
            }
            PendingConfirm::Batch(scope) => {
                let closed = self.apply_close_scope(&scope, cx);
                eprintln!(
                    "S1_TAB_MENU run={} closed={closed} via=discard",
                    scope.tag()
                );
            }
            PendingConfirm::Reload(index) => self.reload_buffer(index, window, cx),
        }
    }

    /// 「保存（全部）」按下之后：写盘成功才执行收尾；返回是否成功。
    fn save_confirm(
        &mut self,
        pending: PendingConfirm,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match pending {
            PendingConfirm::Single(index) => {
                let saved = self.write_buffer(index, "lithe.editor.saveFailed", window, cx);
                if saved {
                    self.close(index);
                    self.sync_cursor(cx);
                    cx.notify();
                }
                saved
            }
            PendingConfirm::Batch(scope) => {
                // 「全部保存」= 把这一批里**所有**脏 buffer 都写盘（真源只保存
                // `pendingClose.bufferId` 那一个，然后 force close 其余 —— 那会让用户
                // 明确点了"保存"却丢掉另外几个文件的修改）。任何一个失败都保持对话框打开。
                let targets = scope.targets(&self.buffers);
                for index in &targets {
                    let dirty = self.buffers.get(*index).is_some_and(|buffer| buffer.is_dirty);
                    if dirty && !self.write_buffer(*index, "lithe.editor.saveFailed", window, cx) {
                        return false;
                    }
                }
                let closed = self.apply_close_scope(&scope, cx);
                eprintln!("S1_TAB_MENU run={} closed={closed} via=save", scope.tag());
                true
            }
            PendingConfirm::Reload(index) => {
                let saved = self.write_buffer(index, "lithe.editor.saveFailed", window, cx);
                if saved {
                    self.reload_buffer(index, window, cx);
                }
                saved
            }
        }
    }

    /// 标签栏：左侧 `← →` 导航组 + 各文件的标签。
    ///
    /// 组件的选型与为什么这么用：
    ///
    /// - `TabVariant::Underline` 是 gpui-kit 里唯一"活动标签 = 透明底 + 底边主色条"的变体
    ///   （`tab/tab.rs:253-262`：`bg` 透明、`border_b` 2px `primary`），与 Windows 的
    ///   IntelliJ 风格一致（`windows/tauri/src/ui/tab-bar.tsx:204-209`：`bg-transparent` +
    ///   `before:h-[3px] before:bg-primary`）；
    /// - Underline 变体的标签条自带 1px 下边框（`tab/tab_bar.rs:501-514`，
    ///   `border_b_1()` + `theme.border`），正好是 Windows 的 `border-b border-border`；
    /// - Underline 变体的条底色是**透明**、外层 padding 是 0
    ///   （`tab/tab_bar.rs:393-403`），所以这里显式补 `.bg(theme.tab_bar)` 与
    ///   `.px_2()`（8px），把 Windows 的 `bg-tab-bar` + `px-(--lithe-chrome-padding-inline)` 找回来；
    /// - `.h_9()`（36px）是**必须**的：标签条自身没有高度，没有标签时会被压成 0；
    ///   真机里没有活动 buffer 时标签栏也照常占着 36px（`windows/tauri/src/features/panes/components/pane-container.tsx:1094-1100`）。
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let names = display_names(&self.buffers);

        let mut bar = TabBar::new("editor-tab-bar")
            .with_variant(TabVariant::Underline)
            .bg(cx.theme().tab_bar)
            .h_9()
            .px_2()
            // 2026-09-27：原来这里还有 `.prefix(self.render_nav_group(cx))`（标签栏左侧那对
            // `← →`）。按维护者口径「编辑区那对 ← → 去掉」删除 —— 它与编辑区左边界那个
            // 「收起侧栏」按钮挤在同一个角上，视觉上重叠。
            // `max_width` 让**标签文字**在空间不够时让位（图标与关闭按钮保持原尺寸），
            // 也就是真机那种 `OrderChargeService.j…` 的截断。这一槽是 `Pixels`
            // （`TabBar::max_width(impl Into<Pixels>)`），所以按当前 rem 基准求值。
            .max_width(rem_px(cx.theme().font_size, TAB_MAX_WIDTH))
            .on_click(cx.listener(|pane, index: &usize, _window, cx| {
                // `TabBar::on_click` 给的是被点标签的下标
                // （`tab/tab_bar.rs:168-177`），切换活动 buffer 就是切标签。
                pane.activate(*index, cx);
            }));

        for (index, (buffer, name)) in self.buffers.iter().zip(names).enumerate() {
            bar = bar.child(self.render_tab(index, buffer, name, cx));
        }

        if let Some(active) = self.active {
            bar = bar.selected_index(active);
        }

        bar.into_any_element()
    }

    /// 给标签的**内容 div** 挂上右键菜单，返回可以直接塞进 `Tab::child(..)` 的元素。
    ///
    /// ⚠️ 挂载点的选择（两条都是实测踩过的坑，
    /// `gpui/research/windows/09-tab-context-menu.md` §5.1）：
    ///
    /// 1. **挂在内容 div 上，不是挂在 `Tab` 上**：`TabBar::child` 只收 `impl Into<Tab>`
    ///    （`tab/tab_bar.rs:151`），外面再包一层 div 之后它就不再是 `Tab` 了。
    ///    而 `Tab` 自己的 children 正好是被渲染进 inner_content 的
    ///    （`tab/tab.rs:743`，且它的 **icon 分支会丢掉 children**），Underline 变体的
    ///    inner padding 是 0（`tab/tab.rs:80-82`），所以挂在内容 div 上与挂在标签上的
    ///    命中区域一致。
    /// 2. **必须自己给 `id`**：`context_menu` 生成的元素 id 取
    ///    `self.interactivity().element_id`，`None` 时退回 `ElementId::CodeLocation(调用点)`
    ///    （`gpui-component-0.6.6/src/menu/context_menu.rs:26-35`）—— 也就是**同一个源码位置
    ///    的每个标签共用一个 id**，右键任意一个都会命中同一份元素状态。这里用的 id 是
    ///    **buffer 路径**（[`tab_element_id`]）而不是下标：关掉一个标签之后下标的指向会整体前移，
    ///    用它当 key 等于把元素状态交给"当前位置"。
    /// 3. **上下文必须当场捕获**：菜单的构建闭包签名是
    ///    `Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu`
    ///    （`context_menu.rs:19-22`）—— **没有鼠标事件、没有位置**，而且它是在
    ///    **下一帧**的 `window.defer` 里跑的（`:324-333`）。所以"被点的是哪个标签"
    ///    只能在这里按 `index` 捕获快照（`path` / `name` / 是不是虚拟路径），
    ///    不能指望闭包回头去问 `self`。
    ///
    /// 回调一律走 `window.listener_for(&pane, ..)`（`gpui-pre-0.3.6/src/window.rs:6626-6635`）：
    /// 它内部 `view.downgrade()` 再 `update`，视图已销毁时静默 `ok()`，
    /// 而 `on_click` 给的第三参是 `&mut App`（不是 `Context<EditorPane>`），
    /// 要回到 `EditorPane` 只有这一条路。
    fn with_tab_menu(
        &self,
        index: usize,
        buffer: &Buffer,
        content: Div,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pane: Entity<Self> = cx.entity();
        // 快照：闭包在下一帧才跑，那时 `buffer` 的借用早已结束。
        let path = buffer.path.clone();
        let name = buffer.name.clone();
        // 真源两条出现条件在 gpui 侧都等价于"这是一份磁盘上的文件"：
        // 「在终端中打开」是 `!isVirtualContent(buffer) && !buffer.path.includes("://")`
        // （`tab-context-menu.tsx:151`）、「重新加载」是
        // `buffer.path !== "extensions://marketplace"`（`:169`）。我们的非磁盘路径空间
        // 只有 `jdt://` 虚拟源码（`extensions://` 还没有对应 buffer），所以两条合起来判一次。
        let on_disk = !is_virtual_source_path(&path);

        content
            // 稳定 id：buffer 路径（见 [`tab_element_id`]）。下面的 `index` 只用于"点这一项
            // 操作哪个标签"这层位置语义（下标在点击时重新解析），不参与元素身份。
            .id(tab_element_id(&buffer.path))
            // 悬停进出这一项就更新 [`Self::hovered_tab`]（非活动标签的 × 靠它显隐）。
            //
            // ⚠️ 必须挂在**加了 `id` 之后**：`on_hover` 是 `StatefulInteractiveElement` 上的
            // 方法（`gpui-pre-0.3.6/src/elements/div.rs:1655`），无 id 的 `Div` 上没有它 ——
            // 悬停状态本来就存在元素状态里（同文件 `:1300` 的 trait 文档）。
            //
            // ⚠️ 离开的分支要**带下标守卫**：鼠标从标签 1 移到标签 0 时，同一帧里标签 0 的
            // `true` 先到、标签 1 的 `false` 后到；无守卫的话后者会把刚落下的 `Some(0)`
            // 清成 `None`，× 就会在鼠标停着的时候闪掉。
            .on_hover(cx.listener(move |pane, hovered: &bool, _window, cx| {
                let next = if *hovered {
                    Some(index)
                } else if pane.hovered_tab == Some(index) {
                    None
                } else {
                    pane.hovered_tab
                };
                if pane.hovered_tab != next {
                    pane.hovered_tab = next;
                    cx.notify();
                }
            }))
            .context_menu(move |mut menu: PopupMenu, window: &mut Window, _cx: &mut Context<PopupMenu>| {
                // 诊断：这条行**只在菜单真的被打开时**才打（闭包由右键按下触发），
                // 所以它同时是"右键命中了这个标签"的证据。
                let mut items = 0usize;
                let mut separators = 0usize;

                // ---- 复制路径 / 复制相对路径 -------------------------------------
                menu = menu.item(
                    PopupMenuItem::new(tr("lithe.files.copyPath"))
                        .icon(IconName::Copy)
                        .on_click(window.listener_for(&pane, move |pane, _, _window, cx| {
                            pane.copy_path(index, cx);
                        })),
                );
                items += 1;

                menu = menu.item(
                    PopupMenuItem::new(tr("lithe.files.copyRelativePath"))
                        .icon(IconName::Copy)
                        .on_click(window.listener_for(&pane, move |pane, _, _window, cx| {
                            pane.copy_relative_path(index, cx);
                        })),
                );
                items += 1;

                // ---- 在资源管理器中显示 -----------------------------------------
                menu = menu.item(
                    PopupMenuItem::new(tr("lithe.files.reveal"))
                        .icon(IconName::FolderOpen)
                        .on_click(window.listener_for(&pane, move |pane, _, window, cx| {
                            pane.reveal_in_explorer(index, window, cx);
                        })),
                );
                items += 1;

                // ---- 在终端中打开（磁盘文件才有）--------------------------------
                if on_disk {
                    menu = menu.item(
                        PopupMenuItem::new(tr("lithe.files.openInTerminal"))
                            .icon(IconName::Terminal)
                            .on_click(window.listener_for(
                                &pane,
                                move |pane, _, window, cx| {
                                    pane.open_in_terminal(index, window, cx);
                                },
                            )),
                    );
                    items += 1;
                }

                // ---- 重新加载（磁盘文件才有）------------------------------------
                if on_disk {
                    menu = menu.item(
                        PopupMenuItem::new(tr("lithe.tabs.reload"))
                            .icon(IconName::RotateCcw)
                            .on_click(window.listener_for(
                                &pane,
                                move |pane, _, window, cx| {
                                    pane.request_reload(index, window, cx);
                                },
                            )),
                    );
                    items += 1;
                }

                // ---- sep-3：真源里恒出现的那一条（"重新加载"与"关闭"之间）-------
                menu = menu.separator();
                separators += 1;

                // ---- 关闭（右键的那个标签；`Ctrl+W` 是"关闭当前"，两者不同）------
                menu = menu.item(
                    PopupMenuItem::new(tr("lithe.tabs.close"))
                        .icon(IconName::X)
                        .on_click(window.listener_for(&pane, move |pane, _, window, cx| {
                            pane.request_close(index, window, cx);
                        })),
                );
                items += 1;

                // ---- 关闭其他 / 关闭右侧 / 全部关闭（批量确认见 `close_scope`）----
                menu = menu.item(
                    PopupMenuItem::new(tr("lithe.tabs.closeOthers")).on_click(
                        window.listener_for(&pane, {
                            let anchor = path.clone();
                            move |pane, _, window, cx| {
                                pane.close_scope(CloseScope::Others(anchor.clone()), window, cx);
                            }
                        }),
                    ),
                );
                items += 1;

                menu = menu.item(
                    PopupMenuItem::new(tr("lithe.tabs.closeToRight")).on_click(
                        window.listener_for(&pane, {
                            let anchor = path.clone();
                            move |pane, _, window, cx| {
                                pane.close_scope(CloseScope::ToRight(anchor.clone()), window, cx);
                            }
                        }),
                    ),
                );
                items += 1;

                menu = menu.item(
                    PopupMenuItem::new(tr("lithe.tabs.closeAll")).on_click(
                        window.listener_for(&pane, move |pane, _, window, cx| {
                            pane.close_scope(CloseScope::All, window, cx);
                        }),
                    ),
                );
                items += 1;

                eprintln!("S1_TAB_MENU opened=true tab={name} items={items} separators={separators}");
                menu
            })
            .into_any_element()
    }

    /// 标签栏左侧的后退/前进按钮组 —— **2026-09-27 按维护者口径删除**。
    ///
    /// 维护者原话：「编辑区那对 ← → 去掉」。当时它们与编辑区左边界那个「收起侧栏」按钮
    /// 挤在同一角上（都落在 x≈430、y≈85 附近），视觉上重叠。
    ///
    /// 删掉的只是**渲染**：跳转历史本身（`crate::navigation::JumpHistory`，由 `F12` /
    /// `Ctrl+单击` 记录）与 [`EditorPane::go_back`] / [`EditorPane::go_forward`] 两个方法
    /// 都保留着（那两个方法现在没有调用点，按仓库口径标 `#[allow(dead_code)]` 并写明原因）。
    /// 想接回来最省事的一条路是给它们绑一对键位（`Alt+←` / `Alt+→`），
    /// 而**不要**再把这组按钮画回这个角落。
    ///
    /// 真源那套（`windows/tauri/src/features/tabs/components/tab-bar.tsx:633-660`）与
    /// `icon-xs` 24×24 的尺寸陷阱记录在 git 历史与本轮提交信息里。
    ///
    /// 下面原来还有 `nav_button`（单个按钮，对齐真机 `icon-xs` = 24×24，而不是 gpui-kit
    /// `xsmall()` 的 20×20），随这一组一起删除。

    /// 一个标签：文件类型图标 + 显示名（+ 未保存圆点）+ 关闭按钮。
    ///
    /// ⚠️ **图标 + 文件名必须走 `Tab::child(...)`**：`Tab::icon()` 只渲染图标，
    /// label 与 children 在图标分支里被整个丢掉（`tab/tab.rs:719-744`，
    /// 图标分支没有 `.children(self.children)`）。
    ///
    /// 颜色照 Windows（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:276`：活动 `--foreground`、非活动
    /// `--subtle-foreground`；`:250`：图标恒为 `--subtle-foreground`）。
    /// 这里必须**自己给文字设色**：`TabBar` 会把标签的 `text_color` 换成自己的
    /// `tab_foreground`/`tab_active_foreground`（`tab/tab.rs:253-264`），
    /// 而 Windows 的非活动标签要的是更暗的 `subtle-foreground`。
    ///
    /// 关闭按钮用**绝对定位**放在标签里，不走 `Tab::suffix`：
    /// suffix 是 flex 项，会算进标签宽度，切换标签时标签宽度会跳；
    /// 绝对定位后宽度只由 `pr(24px)` 决定，与真机一致（`right-1` + `pr-6`）。
    fn render_tab(
        &self,
        index: usize,
        buffer: &Buffer,
        name: SharedString,
        cx: &mut Context<Self>,
    ) -> Tab {
        let is_active = self.active == Some(index);
        let label_color = if is_active {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };

        // 无障碍名照真机拼「名称 +（未保存）」（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:137-141`、
        // `windows/tauri/src/i18n/locale.ts:7860` 的 `tabs.ariaUnsavedSuffix`）；
        // 固定/预览后缀本轮没有这两种状态。必须在 `name` 被移进正文之前算好。
        let aria_label = if buffer.is_dirty {
            SharedString::from(format!("{name}（未保存）"))
        } else {
            name.clone()
        };

        // 未保存圆点：8×8、`rounded-full`、`primary`（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:284-291`）。
        // `is_dirty` 现在是**真实值**：`InputEvent::Change` 置位（`EditorPane::on_input_change`）、
        // 写盘成功清零（`EditorPane::write_buffer`）。
        let dirty_dot = buffer.is_dirty.then(|| {
            div()
                .flex_shrink_0()
                .size_2()
                .rounded_full()
                .bg(cx.theme().primary)
        });

        // 关闭按钮：**所有标签都有**；活动标签常显，非活动标签悬停显。
        //
        // 真源默认设置是 `tabCloseButtonVisibility: "active"`（`windows/tauri/src/features/settings/config/default-settings.ts:96`），
        // 规则见 `windows/tauri/src/features/settings/lib/ui-preferences.ts:13-19`
        // （固定标签恒显 / `always` 全显 / `active` 只有活动标签 / 否则悬停才显）。
        // 本侧实现的是**活动常显 + 非活动悬停显**这一档（维护者 B1 口径）：
        // 开关值本身还没有接进设置（那三档留到设置项接线时再分），但"非活动标签也能关掉"
        // 这件事必须先能用 —— 否则用户只能先点一下选中它、× 才出现。
        //
        // 悬停态来自 [`Self::hovered_tab`]（本结构体自己记，理由见那个字段的文档：
        // 分组悬停实测不生效）。悬停时**渲染出**关闭按钮，不悬停时**不渲染** ——
        // 标签宽度不受影响，因为正文常驻着 `pr_6()`（24px）那条右边距。
        //
        // 绝对定位而不是 `Tab::suffix`：suffix 是 flex 项，会算进标签宽度，
        // 切换标签时标签宽度会跳；绝对定位后宽度只由正文常驻的 `pr(24px)` 决定，
        // 与真机一致（关掉按钮绝对定位在 `right-1` 上，见
        // `windows/tauri/src/features/tabs/components/tab-bar-item.tsx:164-166`）。
        let close = (is_active || self.hovered_tab == Some(index)).then(|| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right_1()
                .flex()
                .items_center()
                .child(Self::close_button(&buffer.path, index, cx))
        });

        let content = h_flex()
            .items_center()
            .pl_2()
            .pr_6()
            .gap_1p5()
            // 没有 `min_w_0` 的话 flex 项的最小尺寸会按内容算，文字截断不了。
            .min_w_0()
            .child(
                // 标签图标：真机默认图标主题（`idea-icons`）有真源就用它（彩色，
                // `img()` 渲染），否则用 Lucide 字形 —— 判断全在 `FileIcon::render` 里。
                // 12 = `size_3()`：标签比文件树小一档，档位上，所以写 `rems(0.75)`。
                buffer.icon.render(rems(0.75), cx),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(label_color)
                    .child(name),
            )
            .children(dirty_dot)
            .children(close);

        // 右键菜单挂在内容 div 上（挂载点的选择与两条坑见 [`Self::with_tab_menu`]）。
        // `.h_full()` 把命中区域从"内容的自然高度"撑到标签内高，右键标签上下边缘时
        // 也能命中（它比 `items_center` 的对齐结果只大几像素，不改观感）。
        let content = self.with_tab_menu(index, buffer, content.h_full(), cx);

        Tab::new().aria_label(aria_label).child(content)
    }

    /// 标签上的关闭按钮。
    ///
    /// 真机：ghost 图标按钮 `icon-xs`（24×24），tooltip `tabs.close`（"关闭"，
    /// `windows/tauri/src/i18n/locale.ts:7845`），点击关闭该标签
    /// （`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:150-180`）。
    ///
    /// ⚠️ **订正一条曾经的错误注释**：这里原来写着"gpui-kit 的 `Button` 点击时会
    /// `stop_propagation`（`button/button.rs:797-808`）"——**与上游源码不符**。上游只在
    /// `loading` 分支里 `stop_propagation`（`gpui-component-0.6.6/src/button/button.rs:797-805`），
    /// 普通点击照常冒泡。所以点 × 之后这次 click 会继续冒到 `Tab::on_click`
    /// （`tab/tab.rs:871-873`）→ `TabBar::on_click`（`tab_bar.rs:457-459`）→
    /// `EditorPane::activate(渲染时捕获的旧 index)`。
    /// 从前"看起来没事"只是因为：关掉**当前**标签时那个旧下标已经被 `close()` 改过、
    /// 越界守卫挡住了它。但 B1 之后非活动标签也有 × 了 —— 关一个**非活动**标签时，
    /// 冒泡上来的 `activate(旧 index)` 会把这个已经不该存在的下标变成活动标签，
    /// 于是"点了 ×，标签关了、但活动标签跑到别处"从隐患变成每次都会走。
    /// 所以这里显式 `cx.stop_propagation()`：点 × 只关标签、**不激活它**
    /// （真源同口径：`tab-bar-item.tsx` 的关闭按钮在 `onPointerDown` 里
    /// `e.stopPropagation()`）。
    ///
    /// 尺寸用 `.small()`：真机的 `icon-xs` 是 **24×24**（`windows/tauri/src/ui/button.tsx:27`
    /// 的 `icon-xs size-6`），而 gpui-kit 的 `Sizable::xsmall()` 给图标按钮是 20×20、
    /// `small()` 才是 24×24（`gpui-component-0.6.6/src/button/button.rs:618-623`）。
    /// 这里对齐的是**尺寸值**，所以用 `.small()`，不要被变体名带偏。
    /// 24 装在 `TabVariant::Underline` 的 26px 内高层里是否会被 `overflow_hidden()` 裁掉，
    /// 由 B1 的实机验收量过（结论写在交付报告里）。
    ///
    /// id 由 buffer 路径给出（见 [`tab_close_element_id`]），不用下标：× 的悬停 / 指针状态
    /// 属于"这个标签"，而不属于"标签条的第 N 个位置"。
    fn close_button(path: &Path, index: usize, cx: &mut Context<Self>) -> Button {
        Button::new(tab_close_element_id(path))
            // Windows 的关闭字形是 lucide `x`，`icons/x.svg` 在全量目录里确有该字形。
            .icon(IconName::X)
            .ghost()
            .small()
            .tab_stop(false)
            .tooltip(tr("lithe.tabs.close"))
            .accessibility_label(tr("lithe.tabs.close"))
            .on_click(cx.listener(move |pane, _event, window, cx| {
                // 先掐掉冒泡（理由见本函数的文档）：否则这次点击会顺带激活这个标签。
                cx.stop_propagation();
                // E：脏标签要先过确认对话框（`request_close`）。
                pane.request_close(index, window, cx);
            }))
    }

    /// 正文区：有活动 buffer 时是代码编辑器，没有时是空状态。
    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(buffer) = self.active.and_then(|index| self.buffers.get(index)) else {
            return Self::render_empty_state(cx);
        };
        // 渲染闭包要 `'static`，先把活动编辑器的句柄取出来（`&Buffer` 借用到此结束）。
        // 两份：滚轮闭包会把它 move 走，正文还要再渲染一次同一个编辑器。
        let editor = buffer.editor.clone();
        let scroll_editor = editor.clone();

        div()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            // `Ctrl+单击` 的接线点：**包装层**的 `on_mouse_down`（冒泡阶段）——
            // 组件自己不带点位置换算的公开 API，但它在自己的 `on_mouse_down` 里已经把光标
            // 移到了点击处，而冒泡到这个包装层时那一步已经做完（理由与出处见
            // [`EditorPane::on_editor_click`]）。
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|pane, event: &MouseDownEvent, window, cx| {
                    pane.on_editor_click(event, window, cx);
                }),
            )
            // ⚠️ **滚轮必须自己接**：编辑器元素的样式只有 `position: absolute` + `100%`，
            // **没有** `overflow: scroll`（`gpui-base-0.6.6/src/input/base/element.rs:202-213`），
            // 而 gpui 只把滚轮交给"命中元素样式里带 `Overflow::Scroll`"的那个
            // （`gpui-pre-0.3.6/src/elements/div.rs:3332-3370`）。
            // 做法是在包裹层接，把增量交给编辑器自己的滚动偏移
            // （`InputBaseState::{scroll_offset, set_scroll_offset}`，
            // `gpui-base-0.6.6/src/input/base/state.rs:2806-2817`；后者会自己 clamp、
            // 下一帧生效）。
            .on_scroll_wheel(
                cx.listener(move |_pane, event: &ScrollWheelEvent, window, cx| {
                    let line_height = scroll_editor
                        .read(cx)
                        .line_height()
                        // 兜底行高 20：滚轮增量换算里的 `Pixels`（不是样式槽），按本帧 rem 基准
                        // 求值，见 [`FALLBACK_LINE_HEIGHT`]。
                        .unwrap_or(rem_px(window.rem_size(), FALLBACK_LINE_HEIGHT));
                    let delta = event.delta.pixel_delta(line_height).y;
                    // 零值与缩放无关（`0 × 任何 rem 基准 = 0`），所以用 `Pixels::ZERO`
                    // （`gpui-pre-0.3.6/src/geometry.rs:2795`）而不是 `px(0.)`：
                    // 没有"规格值"可换算，写 rem 反而是伪换算。
                    if delta == Pixels::ZERO {
                        return;
                    }
                    let current = scroll_editor.read(cx).scroll_offset();
                    scroll_editor.update(cx, |state, cx| {
                        state.set_scroll_offset(point(current.x, current.y + delta), cx);
                    });
                }),
            )
            // ⚠️ **多行编辑器必须显式给高度**：多行走 `.h_auto()`，高度 = **内容高度**，
            // 而内容高度来自 `LayoutMode::CodeEditor { rows }`，`rows` 默认只有 2
            // （`gpui-component-0.6.6/src/input/input.rs:706-709`、
            // `gpui-base-0.6.6/src/input/base/mode.rs:83-85`）—— 不给高度时编辑区只有两行高、
            // 下面整片空白。`relative(1.)` 让它填满外层这一格
            // （等价于组件自带的 `Input::full_height()`，`input.rs:250-252`）。
            //
            // `.bordered(false)`：真机的编辑面是**无边框**的纯表面
            // （`windows/tauri/src/features/editor/components/code-editor.tsx:650,731`：
            // `absolute inset-0 bg-background`，没有 border），而 `Editor` 默认
            // `bordered: true`（`gpui-component-0.6.6/src/input/editor.rs:46`）会画出
            // 输入框那种 1px `theme.input` 描边（`input/input.rs:713-715`）。
            // 焦点环不用管：`Editor` 已经把 `focus_bordered(false)` 写死了
            // （`input/editor.rs:146`）。
            .child(Editor::new(&editor).h(relative(1.)).bordered(false))
            .into_any_element()
    }

    /// 没有活动 buffer 时的空状态。
    ///
    /// 结构照 `windows/tauri/src/features/panes/components/empty-editor-state.tsx`：
    /// 居中一列 = 图标块（48×48 的 `FileText` 40×40 + 右下角 20×20 放大镜）
    /// + 标题 + 说明（`:15-30`）。文案取**中文 i18n 原文，逐字**：
    ///
    /// - 标题 `workbench.emptyEditorTitle` = "选择文件以查看"（`windows/tauri/src/i18n/locale.ts:6150`）；
    /// - 说明 `workbench.emptyEditorDescription` = "外部工具产生的更改会自动显示。"
    ///   （`windows/tauri/src/i18n/locale.ts:6151`）；
    /// - 右键菜单只有一项禁用项 `ui.noActionsHere` = "此处无任何内容"
    ///   （`windows/tauri/src/i18n/locale.ts:4630`；空态菜单在
    ///   `windows/tauri/src/features/panes/components/empty-editor-state.tsx:32-34`）。
    fn render_empty_state(cx: &App) -> AnyElement {
        // 外层用 `v_flex` 而不是裸 `div`：`Empty` 自己带 `flex_1`，
        // 但"flex 项"只在 flex 容器里才成立 —— 裸 div 里它会塌成内容高度，
        // 垂直居中就没了。
        v_flex()
            .flex_1()
            .min_h_0()
            .context_menu(|menu, _window, _cx| {
                menu.item(PopupMenuItem::new(tr("lithe.ui.noActionsHere")).disabled(true))
            })
            .child(
                Empty::new()
                    // ⚠️ 真机空状态**没有边框**；`Empty` 的 render 里硬编码了
                    // `.border_dashed().border_color(cx.theme().border)`
                    // （`gpui-component-0.6.6/src/empty.rs:74-75`），
                    // 所以把边框色改成透明来关掉那圈虚线。
                    .border_color(cx.theme().transparent)
                    .gap_3()
                    .px_6()
                    .py_8()
                    .header(
                        EmptyHeader::new()
                            .max_w_112()
                            .gap_3()
                            .media(
                                EmptyMedia::new().child(
                                    div()
                                        .relative()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .size_12()
                                        .child(
                                            Icon::new(IconName::FileText)
                                                .size_10()
                                                .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(
                                            Icon::new(IconName::Search)
                                                .absolute()
                                                .right_0()
                                                .bottom_0()
                                                .size_5()
                                                .text_color(cx.theme().muted_foreground),
                                        ),
                                ),
                            )
                            // 字号：Windows 的 `ui-text-base` / `ui-text-sm` 都是 13px
                            // （`windows/tauri/src/styles/theme.css:116-117`），不在 gpui 的档位上，
                            // 按《编码指南》用 `text_sm()`（14px，经维护者确认的有意改动；
                            // 正好等于组件默认值，写出来是为了标明"这就是规格值"）。
                            // 颜色用组件默认：标题 `foreground`、说明 `muted_foreground`（`empty.rs:264-324`）。
                            .title(
                                EmptyTitle::new()
                                    .text_sm()
                                    .child(tr("lithe.workbench.emptyEditorTitle")),
                            )
                            .description(
                                EmptyDescription::new()
                                    .text_sm()
                                    .child(tr("lithe.workbench.emptyEditorDescription")),
                            ),
                    ),
            )
            .into_any_element()
    }
}

/// 标签的 ElementId：由 **buffer 路径**决定。
///
/// `Buffer::path` 是打开时的去重键（同一路径重复打开只切标签、不读盘，
/// `crate::buffer::Buffer::path`），所以它就是标签的稳定 identity。下标只是标签在
/// `buffers` 里的当前位置：关掉 / 关到右侧 / 重新打开已关闭标签之后，同一个下标会指向
/// 另一个文件，元素状态（悬停、右键菜单的 element state）就会串到别的标签上
/// （《编码指南》「稳定标识」/「精确区分领域词汇」：index 是位置，id 是 identity）。
///
/// 收 `&Path` 而不是 `&Buffer`：`Buffer` 里持有 `Entity<EditorState>`，纯函数测试造不出来
/// （《编码指南》「测试策略」第 1 层要求纯测试），而路径正是唯一被用到的字段。
fn tab_element_id(path: &Path) -> String {
    format!("editor-tab:{}", path.display())
}

/// 标签上的关闭按钮：与标签同一个 owning object，用同一路径 namespace 开 child ID。
///
/// 与 [`tab_element_id`] 必须不同（否则标签与它的关闭按钮共享元素状态）；两者又必须同时
/// 由路径决定，否则下标漂移后按钮的悬停 / 禁用态会落到另一个标签的 × 上。
fn tab_close_element_id(path: &Path) -> String {
    format!("editor-tab-close:{}", path.display())
}

/// JDT 虚拟源码路径的判据：`crate::navigation` 用 `uri` 直接当 `PathBuf` 建的 buffer。
fn is_virtual_source_path(path: &Path) -> bool {
    path.to_string_lossy().starts_with("jdt://")
}

/// 把绝对路径相对化到工作区根；**没有根 / 不在根下时返回全路径**。
///
/// 真机是 `getRelativePath(path, rootFolderPath)`，`rootFolderPath` 为空时**拷全路径**
/// （`windows/tauri/src/features/tabs/components/tab-bar.tsx:355-368`）。
/// 归一化口径照 `windows/tauri/src/utils/path-helpers.ts:6-18`：比较前先把 `\` 换成 `/`
/// （Windows 上 `C:\a\b` 与 `C:/a/b` 是同一个路径），输出的分隔符也用 `/`
/// —— 与仓库「工作区相对路径用 `/` 分隔」的契约一致（`AGENTS.md` 的 shared contracts 一节）。
fn relative_to_workspace(path: &Path, root: Option<&Path>) -> String {
    let full = path.to_string_lossy().to_string();
    let Some(root) = root else {
        return full;
    };

    let normalized_path = full.replace('\\', "/");
    let normalized_root = root.to_string_lossy().replace('\\', "/");
    let root_trimmed = normalized_root.trim_end_matches('/');

    normalized_path
        .strip_prefix(root_trimmed)
        .map(|rest| rest.trim_start_matches('/').to_string())
        .filter(|rest| !rest.is_empty())
        .unwrap_or(full)
}

/// 编辑区销毁时关掉 Java 会话。
///
/// **同步**在 drop 里调（而不是丢给一个 detached 线程）：Core 拥有 JDTLS 的**子进程**，
/// 而进程退出时不会替我们回收它 —— 一个分离线程很可能在 `main` 返回时被一起杀掉，
/// 留下一个孤儿 JVM。代价是窗口关闭时最多卡住 [`JavaLanguageService::shutdown`] 的有界时间
/// （`lsp.stopServer` 的 shutdown 期限 2s + 终态等待 5s，实测 <1s）。
impl Drop for EditorPane {
    fn drop(&mut self) {
        if let Some(service) = self.java.take() {
            service.shutdown();
        }
    }
}

impl Render for EditorPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 两层：标签栏在上，正文占剩下的高度。容器要 `min_h_0` + `overflow_hidden`，
        // 否则正文里的编辑器会把整个视图撑出父容器。
        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            // `F12`：绑定登记在 `lithe_gpui_editor::install_actions`，处理器在本视图根元素上。
            // 放根元素而不是编辑器元素：焦点在编辑器里时 action 从焦点节点往祖先冒泡
            // （`gpui-pre-0.3.6/src/window.rs:6333` 一带），根元素必然经过；
            // 而焦点在资源管理器 / 终端时这一串里没有编辑区，`F12` 自然什么都不做。
            .on_action(
                cx.listener(|pane, _: &crate::NavigateToDefinition, window, cx| {
                    pane.navigate_to_definition(window, cx);
                }),
            )
            // `Ctrl+W`：绑定登记在 `lithe_gpui_editor::install_actions`，处理器同样放根元素
            // （理由与 `F12` 完全相同：焦点在编辑器里时 action 从焦点节点往祖先冒泡，
            // 焦点在终端 / 资源管理器时这一串里没有编辑区，按键自然什么都不做）。
            .on_action(cx.listener(|pane, _: &crate::CloseActiveTab, window, cx| {
                pane.close_active(window, cx);
            }))
            .child(self.render_tab_bar(cx))
            .child(self.render_body(cx))
    }
}

#[cfg(test)]
mod tests {
    use super::{rem_px, tab_close_element_id, tab_element_id};
    use gpui_kit::px;
    use std::path::{Path, PathBuf};

    /// 回归判据（《编码指南》「稳定标识」）：同一批标签在两种顺序下算出的 id 必须相同，
    /// 不同标签算出的 id 必须不同。用下标当 id 时倒序的每一项都会拿到对方的 id，这条会失败。
    ///
    /// 纯函数测试（「测试策略」第 1 层）：不起窗口、不建 `EditorState`，所以这里直接喂路径。
    #[test]
    fn tab_element_ids_do_not_depend_on_the_tab_order() {
        let tabs = [
            PathBuf::from("/workspace/src/main.rs"),
            PathBuf::from("/workspace/README.md"),
            PathBuf::from("jdt://contents/java.base/java/lang/String.class"),
        ];

        let forward: Vec<String> = tabs.iter().map(|path| tab_element_id(path)).collect();
        let reversed: Vec<String> = tabs.iter().rev().map(|path| tab_element_id(path)).collect();

        assert_eq!(forward[0], reversed[2]);
        assert_eq!(forward[1], reversed[1]);
        assert_eq!(forward[2], reversed[0]);
        assert_ne!(forward[0], forward[1]);
        assert_ne!(forward[0], forward[2]);
        assert_ne!(forward[1], forward[2]);
        // 虚拟源码（`jdt://`）与磁盘文件同处一个身份空间：路径本身就是它的身份。
        assert!(tab_element_id(Path::new("jdt://x")).starts_with("editor-tab:jdt://x"));
    }

    /// 关闭按钮与它所在的标签是两个元素：必须由同一路径 namespace，但彼此 id 不同
    /// （否则按钮的悬停 / 指针状态会污染标签本身的元素状态）。
    #[test]
    fn close_button_id_is_namespaced_apart_from_its_tab() {
        let path = Path::new("/workspace/src/main.rs");
        let other = Path::new("/workspace/src/lib.rs");

        assert_ne!(tab_element_id(path), tab_close_element_id(path));
        assert_ne!(tab_close_element_id(path), tab_close_element_id(other));
        assert_eq!(tab_close_element_id(path), "editor-tab-close:/workspace/src/main.rs");
    }

    /// [`rem_px`] 必须按**传入的 rem 基准**求值，而不是写死 16。
    ///
    /// `AbsoluteLength::from(rems(P / 16.)).to_pixels(px(16.))` 这种**假 rem**写法在 16px 基准下
    /// 与规格值逐像素相等，却完全不随界面字号缩放 —— 所以只断言"16px 基准下等于原值"证明不了
    /// 它真在按基准缩放，必须再断言基准变化时结果跟着变。两个调用点
    /// （`FALLBACK_LINE_HEIGHT` 兜底行高、`TAB_MAX_WIDTH` 标签宽上限）都靠这条保证语义。
    #[test]
    fn rem_px_scales_with_the_runtime_rem_base() {
        // 默认基准（`uiFontSize = 13` → `theme_font_size_for(13) = 16.0`）：改动前后逐像素相等。
        assert_eq!(rem_px(px(16.), 20.), px(20.));
        assert_eq!(rem_px(px(16.), 200.), px(200.));
        // 基准翻倍 → 换算结果翻倍（假 rem 会得到 px(20.) / px(200.) 而失败）。
        assert_eq!(rem_px(px(32.), 20.), px(40.));
        assert_eq!(rem_px(px(32.), 200.), px(400.));
        // 基准 ×1.25（`uiFontSize = 20` → 25.0）：线性缩放。
        assert_eq!(rem_px(px(20.), 20.), px(25.));
    }
}
