//! 左栏「源代码管理」视图的**表现层**：`ChangesView` 实体与它的各段渲染。
//!
//! 数据、纯函数与写操作在 `changes.rs`；本文件只负责画。
//!
//! 规格真源：`gpui/research/windows/08-source-control.md`（结构 §2、交互 §3、空态 §5），
//! 对应 Windows 前端 `windows/tauri/src/features/git/components/git-view.tsx`。
//!
//! # 行组件：**自写最小行**，不复用 `explorer` 的树行
//!
//! 研究 §7 第 10 条点名的已知障碍：`gpui/crates/explorer` 的行组件
//! （`explorer_view.rs:562-697` 的 `Tree::new` 逐行闭包）**没有动作槽**，只有
//! caret + 图标 + 文本；变更列表要的「行内暂存按钮」必须先扩那个 crate 的 `Tree` 行渲染。
//!
//! 本文件选择**自写一个最小的行**（[`ChangesView::render_row`]），代价与理由：
//!
//! - **不用 `Tree`/`uniform_list`**：第一版**不做目录树折叠**（真源的 `gitChangesFolderView`
//!   默认 true，`default-settings.ts:193`；研究 §6.4 的第一版范围也没要求），
//!   扁平列表 + `overflow_y_scroll` 就够；而且滚的是「分类头 + 行 + 间隔」的混合高度，
//!   而 `uniform_list` 只支持等高行（`gpui-pre-0.3.6/src/elements/uniform_list.rs:658-680`）。
//! - **代价**：没有虚拟滚动（真源 `SidebarTree` 有 overscan 12）。工作区变更数通常在几十到
//!   几百行，一屏外多画几百个 `div` 的代价远小于"为一个列表去改 explorer 的公共行组件"的
//!   耦合成本；真要收紧再换 `uniform_list`（那时分类头要拆成独立粘性区）。
//! - **换来的收益**：`explorer` 一行代码都不用动（它的 `Tree` 行保持"文件树专用"），
//!   行内可以自由放暂存按钮、状态配色与 hover 行为。
//!
//! # 与 Windows 真源的刻意偏差（承接 `changes.rs` 的 1-3，这里是渲染侧的）
//!
//! 1. **暂存按钮常显**：真源是 `opacity-0 group-hover/git-status-row:opacity-100`
//!    （`git-status-file-item.tsx:68`），只在行悬停时显形。本侧常显但用弱色
//!    （`muted_foreground`）—— 无人值守截图取不到"悬停才出现"的按钮，而且自绘按钮没有
//!    悬停 tooltip（`log_view.rs` 偏差 5 的同一取舍）。
//! 2. **没有 `history` 子页签**：真源左栏自带「更改 / 历史记录」两个子页签
//!    （`git-view.tsx:712-766`）。历史记录**已经有**底部工具窗那一份（`BottomPane`，
//!    研究 §1.3 明确两者"互补不等价"），第一版左栏只做「更改」。
//! 3. **提交面板没有 AI 与「提交并推送」**：那三个动作分别是 AI 服务（与 Git 契约无关）
//!    与远程推送（要凭据 UI），都不在第一版范围（研究 §6.4）。
//! 4. **没有状态栏 / 标题栏的分支项**：真源分支 + ahead/behind 常驻在**标题栏**（研究 §2⑦），
//!    本侧标题栏已由 `crate::branch_panel` 做完，这里只在工具行画 `↑n ↓n` 纯文本，
//!    避免同一个事实出现两个入口、又完全不显示。
//! 5. **写操作失败是常驻红条，不是 toast**：见 [`ChangesView::render_write_error`]。

use std::collections::BTreeSet;
use std::path::PathBuf;

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia};
use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, WindowExt as _};
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, Pixels, Render, Role, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, WeakEntity, Window, div, relative,
    rems,
};

use lithe_gpui_shared::{tr, tr_args};

use crate::CommitChanges;
use crate::changes::{
    ChangeRow, ChangesSnapshot, OperationState, commit as write_commit, discard, load,
    operation_action, stage, stage_all, unstage,
};
use crate::log_view::rem_px;

// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`，与 Windows 规格逐像素相等
// （出处：`08-source-control.md` §2.2 与 `03-git-and-bottom.md` §2.7）：
//
// | 规格 | 值 | 用到的 helper | 出处 |
// | --- | --- | --- | --- |
// | 标题栏高 36（`--lithe-pane-header-height`） | 36 | `h_9()` | `ui/sidebar.tsx:38` |
// | 标题栏 / 工具行 `px-2` `gap-2` | 8 | `px_2()` / `gap_2()` | `ui/sidebar.tsx:43-47` |
// | 工具行高 36 | 36 | `h_9()` | `git-status-panel.tsx:1105` |
// | 分类头高 32 | 32 | `h_8()` | `git-status-panel.tsx:765-776` |
// | 行高 24 | 24 | `h_6()` | `file-tree-row.ts:1-14`（13×1.35+6 → max(24,·)） |
// | 行内 `px-1.5` / `gap-1` | 6 / 4 | `px_1p5()` / `gap_1()` | `sidebar-tree.tsx:241` |
// | 暂存按钮 20×20（`size-5`） | 20 | `size_5()` | `git-status-file-item.tsx:68` |
// | 提交说明框 min-h 64 | 64 | `rems(4.)` | `git-commit-panel.tsx:54-55` |
// | 分类头下 2 / 分区之间 8 | 2 / 8 | `pb_0p5()` / `pb_2()` | `git-status-panel.tsx:139-141,374-385` |
//
// 字号：`ui-text-sm` = 13px（`theme.css:116`）不在 gpui 的档位（12 / 14）上，按《编码指南》用
// **`text_sm()`（14px）**——与 `explorer_view.rs:80-81` 同一条已确认的有意改动。
//
// ⚠️ 圆角一律走应用层具名常量：Lithe 的 `--radius × k` 阶梯（4.8 / 6.4 / 11.2）与 gpui 的
// 同名档位语义不同，也不能从主题读（`ThemeConfig.radius` 是 `usize`，
// `gpui-component-0.6.6/src/theme/schema.rs:67-68`）。常量保持**规格像素值身份**（`f32`），
// 调用点写 `rems(半径 / 16.)`（rem base = 16px，写 `/ 4.` 就错）——`div` 上的 `rounded` 是
// `Styled::rounded(impl Into<AbsoluteLength>)`，能吃 `Rems`；`Button::rounded` 只吃
// `ButtonRounded`（只有 `From<Pixels>`），那里走 `rem_px`。

/// 行 / 分类头圆角 4（`sidebar-tree.css` 的 `--file-tree-row-radius: 4px`）。
const ROW_RADIUS: f32 = 4.;
/// 行高倍数 1.35（`theme.css:4` `--leading-row`）—— 是倍数不是长度，原样保留。
const ROW_LINE_HEIGHT: f32 = 1.35;
/// 图标按钮圆角 6.4（`ui/button.tsx:9` 的 `rounded-md` = `--radius × 0.8`）。
const ICON_BUTTON_RADIUS: f32 = 6.4;
/// 提交面板外壳圆角 11.2（`ui/sidebar.tsx:75` 的 `rounded-xl` = `--radius × 1.4`）。
const COMMIT_PANEL_RADIUS: f32 = 11.2;
/// 提交面板内错误块圆角 6.4（`git-commit-panel.tsx:264-274` 的 `rounded-md`）。
const COMMIT_ERROR_RADIUS: f32 = 6.4;
/// 提交说明框最小高 64（`git-commit-panel.tsx:54-55` 的 `min-h-64`）。
///
/// ⚠️ 走 `rems(64. / 16.)` 而不是 `px(64.)`：`Textarea::h` 吃 `DefiniteLength`
/// （`gpui-pre-0.3.6/src/geometry.rs:3597-3606` 对 `Pixels` / `Rems` 都有 `From`），
/// 用 rem 与 `h_16()` 同值且随主题字号缩放。
const COMMIT_INPUT_HEIGHT: f32 = 64.;
/// 冲突提示最多列出多少个路径（真源只给 `{count}` 个数，`git-operation-banner.tsx:97-100`；
/// 这里补上路径是因为"哪些文件"才是用户下一步唯一要的信息）。
const CONFLICT_PATH_LIMIT: usize = 3;

// ---------------------------------------------------------------------------
// 文案 key
// ---------------------------------------------------------------------------
//
// 全部逐字取自 `windows/tauri/src/i18n/locale.ts` 生成的两份 locale
// （`gpui/crates/shared/locales/`），没有为此新增任何 `GPUI_ONLY_KEYS`：
// 左栏「更改」视图用到的每一条键真源里都已经有了。

/// `workbench.sourceControl` → 「源代码管理」（`locale.ts:5909`）。
const TITLE_KEY: &str = "lithe.workbench.sourceControl";
/// `git.refreshStatus` → 「刷新状态」（`locale.ts:8636`）。
const REFRESH_KEY: &str = "lithe.git.refreshStatus";
/// `git.actions` → 「Git 操作」（`git-view.tsx:474` 的「更多」按钮 tooltip；研究 §7 第 1 条
/// 当时记为"未确认"，本轮已在生成物里核到）。
const ACTIONS_KEY: &str = "lithe.git.actions";
/// `git.statusLoadFailed` → 「无法加载 Git 数据，请重试刷新仓库。」（`locale.ts:7011`）。
const LOAD_FAILED_KEY: &str = "lithe.git.statusLoadFailed";
/// `git.log.retry` → 「重试」（`locale.ts:7197`）。
const RETRY_KEY: &str = "lithe.git.log.retry";
/// `git.loadingGitStatus` → 「正在加载 Git 状态」（`locale.ts:7013`）。
const LOADING_KEY: &str = "lithe.git.loadingGitStatus";
/// `git.tracked` → 「已跟踪」（`locale.ts:6817`）。
const TRACKED_KEY: &str = "lithe.git.tracked";
/// `git.untracked` → 「未跟踪」（`locale.ts:6818`）。
const UNTRACKED_KEY: &str = "lithe.git.untracked";
/// `git.workingTreeClean` → 「工作区干净」（`locale.ts:7158`）。
const CLEAN_KEY: &str = "lithe.git.workingTreeClean";
/// `git.setup.notRepository` → 「此项目尚未初始化 Git 仓库」（`locale.ts:4595`）。
const NOT_REPOSITORY_KEY: &str = "lithe.git.setup.notRepository";
/// `git.setup.initializeHint` → 「初始化 Git 后即可跟踪更改并创建首次提交。」（`locale.ts:4596`）。
const NOT_REPOSITORY_HINT_KEY: &str = "lithe.git.setup.initializeHint";
/// `git.setup.initialize` → 「初始化 Git 仓库」（`locale.ts:4597`）。
const INITIALIZE_KEY: &str = "lithe.git.setup.initialize";
/// `git.setup.noCommits` → 「当前分支尚无提交」（`locale.ts:4600`）。
const NO_COMMITS_KEY: &str = "lithe.git.setup.noCommits";
/// `git.setup.firstCommit` → 「在“更改”面板暂存文件、填写提交消息，然后创建首次提交。」
/// （`locale.ts:4601`）。
const NO_COMMITS_HINT_KEY: &str = "lithe.git.setup.firstCommit";
/// `git.stageAllChanges` → 「暂存所有更改」（`locale.ts:7154`）。
const STAGE_ALL_KEY: &str = "lithe.git.stageAllChanges";
/// `git.unstageAllChanges` → 「取消暂存所有更改」（`locale.ts:7155`）。
const UNSTAGE_ALL_KEY: &str = "lithe.git.unstageAllChanges";
/// `git.trackedFiles` / `git.untrackedFiles` → 「已跟踪文件」/「未跟踪文件」（`locale.ts:7156-7157`）。
const TRACKED_FILES_KEY: &str = "lithe.git.trackedFiles";
const UNTRACKED_FILES_KEY: &str = "lithe.git.untrackedFiles";
/// `git.commitMessagePlaceholder` → 「提交说明...」（`locale.ts:6819`）。
const COMMIT_PLACEHOLDER_KEY: &str = "lithe.git.commitMessagePlaceholder";
/// `git.commit` → 「提交」（`locale.ts:6832`）。
const COMMIT_KEY: &str = "lithe.git.commit";
/// `git.committing` → 「正在提交...」（`locale.ts:6835`）。
const COMMITTING_KEY: &str = "lithe.git.committing";
/// `git.filesStaged` → 「已暂存 {count} 个文件」（`locale.ts:6813`）。
///
/// ⚠️ 用 `filesStaged` 而**不是** `filesSelected`：本侧提交的是**索引**（`changes.rs` 偏差 1，
/// 没有"提交范围复选框"），所以计数的语义是"已暂存"而不是"已选择"。
const FILES_STAGED_KEY: &str = "lithe.git.filesStaged";
/// `git.noFilesStaged` → 「没有已暂存的文件」。
const NO_FILES_STAGED_KEY: &str = "lithe.git.noFilesStaged";
/// `git.selectFilesToCommit` → 「请选择要提交的文件。」（`locale.ts:6830`）。
const SELECT_FILES_KEY: &str = "lithe.git.selectFilesToCommit";
/// `git.enterCommitMessage` → 「输入提交说明：」（`locale.ts:5125`）。
const ENTER_MESSAGE_KEY: &str = "lithe.git.enterCommitMessage";
/// `git.changesCommitted` → 「更改已提交」（`locale.ts:5127`）。
const COMMITTED_KEY: &str = "lithe.git.changesCommitted";
/// `git.finishOperationBeforeCommit` → 「请先完成或中止当前 Git 操作，再提交文件。」
/// （`locale.ts:6993`）。
const FINISH_OPERATION_KEY: &str = "lithe.git.finishOperationBeforeCommit";
/// `git.resolveConflictsFirst` → 「请先解决冲突：{paths}」（`locale.ts:6997`）。
const RESOLVE_CONFLICTS_KEY: &str = "lithe.git.resolveConflictsFirst";
/// `git.operationStep` → 「第 {step} / {total} 步。」（`locale.ts:7042`）。
const OPERATION_STEP_KEY: &str = "lithe.git.operationStep";
/// `git.resolveConflicts` → 「解决 {count} 个冲突文件，暂存后继续。」（`locale.ts:7043`）。
const RESOLVE_CONFLICTS_HINT_KEY: &str = "lithe.git.resolveConflicts";
/// `git.conflictsResolved` → 「所有冲突已解决。继续完成，或中止以撤销。」（`locale.ts:7044`）。
const CONFLICTS_RESOLVED_KEY: &str = "lithe.git.conflictsResolved";
/// `git.skipCommit` → 「跳过提交」（`locale.ts:7045`）。
const SKIP_KEY: &str = "lithe.git.skipCommit";
/// `git.abort` → 「中止」（`locale.ts:7046`）。
const ABORT_KEY: &str = "lithe.git.abort";
/// `git.discardAllChanges` → 「丢弃全部更改」（`git-actions-menu.tsx:308`）。
const DISCARD_ALL_KEY: &str = "lithe.git.discardAllChanges";
/// `git.discardChangesConfirm` → 「丢弃所有未暂存的更改吗？此操作无法撤销。」
/// （`git-actions-menu.tsx:146-153`）。
const DISCARD_CONFIRM_KEY: &str = "lithe.git.discardChangesConfirm";
/// `git.rollback` → 「回滚」（`locale.ts:7160`；同一件事在真源里叫"回滚"）。
const ROLLBACK_KEY: &str = "lithe.git.rollback";
/// `ui.cancel` → 「取消」（`locale.ts:4631`）。
const CANCEL_KEY: &str = "lithe.ui.cancel";

// ---------------------------------------------------------------------------
// 状态
// ---------------------------------------------------------------------------

/// 读取状态。**只描述"读"**：写操作的失败单独放在 [`ChangesView::error`]，
/// 因为一次写失败不该把整张列表换成错误页。
enum LoadState {
    Loading,
    Ready,
    /// `git.*` 读命令失败（存命令名 + 错误码，见 `changes.rs` 的 `failures`）。
    /// **不静默**：画错误条 + 重试。
    Failed(String),
}

/// 提交被守卫挡住的原因（一条；优先级照真源 `git-commit-panel.tsx:148-172`）。
///
/// 做成枚举而不是"一句文案"是因为**两种消费者要不同的东西**：按钮只要
/// "能不能提交"（[`CommitBlocker::is_none`]），而面板错误块要区分"必须解释给用户看的
/// 硬守卫"（进行中的操作 / 冲突）与"一眼就看得出来的软守卫"（没暂存 / 没写说明）。
enum CommitBlocker {
    /// 有冲突未解决（Core 会拒绝 continue，也会拒绝提交）。
    Conflicts(SharedString),
    /// 有进行中的 merge / rebase / cherry-pick / revert。
    Operation,
    /// 索引里没有文件。
    NoStagedFiles,
    /// 提交说明为空（Core 用 `required_text` 校验，空消息是 `invalid_request`）。
    EmptyMessage,
}

// ---------------------------------------------------------------------------
// 组件
// ---------------------------------------------------------------------------

/// 左栏「源代码管理」。对外只暴露 `new` / `refresh` / `activate` 与 `Render`，字段全部私有。
pub struct ChangesView {
    /// 工作区根（`git.*` 命令都要求带 `root`）。
    root: PathBuf,
    /// 最近一次成功的快照（仓库根 / 分支 / ahead-behind / 变更行 / 操作状态）。
    snapshot: ChangesSnapshot,
    state: LoadState,
    /// **写操作**失败时的用户可见提示（常驻红条的原文）。
    ///
    /// ⚠️ 这是本侧对真源最刻意的偏离之一：真源的写操作失败常常只 `console.error`
    /// （研究 §5 的"命令失败时的表现"一节），本侧**不允许静默**——任何失败都进这里，
    /// 由 [`ChangesView::render_write_error`] 画成红条。
    error: Option<SharedString>,
    /// 最近一次写操作**成功**后的短提示（例如「更改已提交」）。
    notice: Option<SharedString>,
    /// 正在暂存 / 取消暂存的路径（对应真源的 `stagePendingPaths`，
    /// `git-status-panel.tsx:234-250`）：这些行显示转圈、按钮禁用。
    pending: BTreeSet<SharedString>,
    /// 正在跑一个"整体"写操作（暂存全部 / 提交 / 横幅的继续中止跳过）。
    busy: bool,
    /// 提交说明输入框。
    message: Entity<TextareaState>,
    /// 输入事件订阅（不存下来会被立刻丢掉，输入框就不再触发重绘）。
    _message_subscription: Subscription,
    /// 请求代次：晚到的旧回包直接丢掉。
    request_serial: u64,
    /// 设置项 `confirmBeforeDiscard`（默认 `true`）：丢弃未暂存更改前是否先弹确认框。
    ///
    /// **这是本视图唯一消费的设置键**，也是「Git」设置页里第一个"真的能被消费"的开关
    /// （研究 `07-settings-ui.md` §7.3-D：没有消费方的开关一律不画）。
    /// 值由外壳推过来（[`ChangesView::set_confirm_before_discard`]）：本 crate 不认识
    /// `lithe-gpui-settings`（依赖方向 `workbench` → `git`，反向会成环），
    /// 与 `terminal/src/session.rs` 的 `default_shell` 同一条路子。
    confirm_before_discard: bool,
}

impl ChangesView {
    /// 建立视图。**构造期不取数据**，由外壳在第一帧之后调 [`ChangesView::refresh`]
    /// （与 `Explorer::new` 同一口径：构造期读 Core 会把首帧卡住）。
    pub fn new(root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let message = cx
            .new(|cx| TextareaState::new(window, cx).placeholder(tr(COMMIT_PLACEHOLDER_KEY)));
        let message_subscription = cx.subscribe_in(
            &message,
            window,
            |_view: &mut Self, _state: &Entity<TextareaState>, event: &InputEvent, _window, cx| {
                // 说明文字变化**只影响提交按钮的禁用态**，所以这里只重绘，不落派生状态。
                if matches!(event, InputEvent::Change) {
                    cx.notify();
                }
            },
        );

        Self {
            root,
            snapshot: ChangesSnapshot::default(),
            state: LoadState::Loading,
            error: None,
            notice: None,
            pending: BTreeSet::new(),
            busy: false,
            message,
            _message_subscription: message_subscription,
            request_serial: 0,
            // 真源默认 `true`（`default-settings.ts:194`）：**先按真源默认值起步**，
            // 外壳拿到设置后立刻用 `set_confirm_before_discard` 覆盖它。
            confirm_before_discard: true,
        }
    }

    /// 推入设置项 `confirmBeforeDiscard`（外壳在启动时与每次设置变化时各调一次）。
    ///
    /// **幂等**：值没变就直接返回（照 `TerminalPane::set_default_shell` 的口径）——
    /// 这条设置只影响下一次丢弃，重复推同一个值不该产生任何副作用。
    pub fn set_confirm_before_discard(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.confirm_before_discard == enabled {
            return;
        }
        self.confirm_before_discard = enabled;
        // 诊断行（可 grep）：证明"设置真的被消费方拿到了"。
        // 这一条是机器验证的关键证据 —— 光看界面勾选框的变化，证明不了开关接上了。
        eprintln!("S1_SOURCE_CONTROL confirm_before_discard={enabled}");
        cx.notify();
    }

    /// 重新读一遍工作区状态（手动刷新 / 写操作之后 / 切到本视图时）。
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.request_serial = self.request_serial.wrapping_add(1);
        let serial = self.request_serial;
        // 已有仓库数据时保持原状态：刷新失败会切到 `Failed` 并画错误条，但**不清空**列表
        // （清空会让"刷新失败"看起来像"改动没了"）。
        if self.snapshot.repository_root.is_none() {
            self.state = LoadState::Loading;
        }
        let root = self.root.clone();
        cx.notify();

        cx.spawn(async move |this, cx| {
            let snapshot = cx.background_spawn(async move { load(&root) }).await;
            let _ = this.update(cx, |view, cx| view.apply(serial, snapshot, cx));
        })
        .detach();
    }

    /// 切到本视图时调用：刷新一次并打一行诊断。
    ///
    /// 真源在"视图由不可见变可见"时后台整刷一次（`use-git-data-controller.ts:276-281`）。
    /// 本侧没有 watcher（刻意不做，见 crate 文档），所以「激活时刷新 + 手动刷新 + 写后刷新」
    /// 就是全部的刷新通路。
    pub fn activate(&mut self, cx: &mut Context<Self>) {
        self.refresh(cx);
    }

    /// 把一次读取结果写进视图。
    fn apply(&mut self, serial: u64, snapshot: ChangesSnapshot, cx: &mut Context<Self>) {
        if serial != self.request_serial {
            return;
        }

        // 诊断一行（可 grep；口径同 `S1_EXPLORER`）。**走 stderr**：写操作失败也是失败信号，
        // 混进 stdout 会和正常输出纠缠。
        eprintln!("{}", snapshot.diagnose());
        if !snapshot.failures.is_empty() {
            eprintln!("S1_SOURCE_CONTROL failures={}", snapshot.failures.join(" "));
        }

        // "加载失败"的判据是「一条仓库根都没读到 **且** 有失败命令」：
        // `git.status` 成功但返回 `repositoryRoot: null` 是"不是仓库"（正常态），
        // 不是失败（硬约束 2）。
        self.state = if snapshot.repository_root.is_none() && !snapshot.failures.is_empty() {
            LoadState::Failed(snapshot.failures.join(" "))
        } else {
            LoadState::Ready
        };
        self.snapshot = snapshot;
        cx.notify();
    }

    /// 索引里的文件数（= 提交按钮的"已暂存 N 个文件"）。
    fn staged_count(&self) -> usize {
        self.snapshot.staged_count()
    }

    /// 提交说明原文。
    fn message_text(&self, cx: &App) -> SharedString {
        self.message.read(cx).value()
    }

    /// 提交是否被守卫挡住。`None` = 可以提交。
    fn commit_blocker(&self, cx: &App) -> Option<CommitBlocker> {
        if let Some(operation) = &self.snapshot.operation {
            if operation.has_conflicts() {
                let paths = operation
                    .conflicted_paths
                    .iter()
                    .take(CONFLICT_PATH_LIMIT)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("、");
                return Some(CommitBlocker::Conflicts(tr_args(
                    RESOLVE_CONFLICTS_KEY,
                    &[("paths", &paths)],
                )));
            }
            return Some(CommitBlocker::Operation);
        }
        if self.snapshot.staged_count() == 0 {
            return Some(CommitBlocker::NoStagedFiles);
        }
        if self.message_text(cx).trim().is_empty() {
            return Some(CommitBlocker::EmptyMessage);
        }
        None
    }

    // ---- 写操作 ----

    /// 暂存 / 取消暂存一组路径（行内按钮与工具行「取消暂存全部」都走这里）。
    ///
    /// 写操作**一律在后台线程**跑，回前台才刷新：`git add` 在大仓库上不是瞬时的，
    /// 放 UI 线程会把整帧挡住（与 `Explorer::load` 同一条理由）。
    fn set_staged(&mut self, paths: Vec<SharedString>, staged: bool, cx: &mut Context<Self>) {
        if paths.is_empty() || self.busy {
            return;
        }
        let Some(root) = self.repository_root_text() else {
            return;
        };

        // 记在途：这些行显示转圈、按钮禁用（真源的 `stagePendingPaths`）。
        for path in &paths {
            self.pending.insert(path.clone());
        }
        self.error = None;
        self.notice = None;
        cx.notify();

        let target: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
        let touched = paths.clone();
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(async move {
                    if staged {
                        stage(&root, &target)
                    } else {
                        unstage(&root, &target)
                    }
                })
                .await;

            let _ = this.update(cx, |view, cx| {
                for path in &touched {
                    view.pending.remove(path);
                }
                let verb = if staged { "stage" } else { "unstage" };
                view.record_write(verb, touched.len(), outcome.failure, None, cx);
            });
        })
        .detach();
    }

    /// 暂存全部更改 / 取消暂存全部（工具行右侧两个按钮）。
    ///
    /// ⚠️ 「取消暂存全部」**不照抄 Windows 的 `git_reset_all`**（那是
    /// `git.command ["reset","HEAD"]`，`platform.rs:580-583`；研究 §4.2 第一条点名它），
    /// 本侧用 `unstage` + 全部已暂存路径（Core 的 `discardAll` 是**丢弃**，不能拿来当取消暂存）。
    fn set_all_staged(&mut self, staged: bool, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if !staged {
            let paths: Vec<SharedString> = self
                .snapshot
                .changes
                .iter()
                .filter(|row| row.staged)
                .map(|row| row.path.clone())
                .collect();
            self.set_staged(paths, false, cx);
            return;
        }

        let Some(root) = self.repository_root_text() else {
            return;
        };
        self.busy = true;
        self.error = None;
        self.notice = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let outcome = cx.background_spawn(async move { stage_all(&root) }).await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.record_write("stage_all", 0, outcome.failure, None, cx);
            });
        })
        .detach();
    }

    /// 提交（按钮与 `Ctrl+Enter` 共用）。
    ///
    /// 守卫顺序照真源（`git-commit-panel.tsx:148-172`）：冲突 → 进行中的操作 → 没有暂存文件
    /// → 空消息。前两条**必须**在界面上说清楚，否则用户看到的是"点了没反应"
    /// （这正是 `operationState` 横幅要进第一版的原因，研究 §6.4 理由最后一条）。
    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if let Some(blocker) = self.commit_blocker(cx) {
            // 守卫原因**一定**落到可见的错误条上（不给"静默 return"留口子）。
            self.error = Some(match blocker {
                CommitBlocker::Conflicts(message) => message,
                CommitBlocker::Operation => tr(FINISH_OPERATION_KEY),
                CommitBlocker::NoStagedFiles => tr(SELECT_FILES_KEY),
                CommitBlocker::EmptyMessage => tr(ENTER_MESSAGE_KEY),
            });
            cx.notify();
            return;
        }

        let Some(root) = self.repository_root_text() else {
            return;
        };
        let staged = self.staged_count();
        let text = self.message_text(cx).to_string();
        self.busy = true;
        self.error = None;
        self.notice = None;
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            let outcome = cx
                .background_spawn(async move { write_commit(&root, &text) })
                .await;
            let _ = this.update_in(cx, |view, window, cx| {
                view.busy = false;
                let committed = outcome.failure.is_none();
                view.record_write("commit", staged, outcome.failure, None, cx);
                if committed {
                    view.notice = Some(tr(COMMITTED_KEY));
                    // 提交成功后清空说明框（真源也不保留原文）。
                    // ⚠️ **必须**在 `update_in` 里做：`set_value` 要 `&mut Window`，
                    // 而 `update` 拿不到窗口（`gpui-base-0.6.6/src/input/base/state.rs:897-903`）。
                    view.message.update(cx, |state, cx| {
                        state.set_value(SharedString::default(), window, cx)
                    });
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// 操作横幅的继续 / 中止 / 跳过（`git.write` 的 `operationContinue` 等）。
    fn run_operation_action(&mut self, operation: &'static str, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(root) = self.repository_root_text() else {
            return;
        };
        self.busy = true;
        self.error = None;
        self.notice = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(async move { operation_action(&root, operation) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.record_write(operation, 0, outcome.failure, None, cx);
            });
        })
        .detach();
    }

    /// 「丢弃全部更改」：**默认先弹确认框**，确认后才跑 `git.write discard`。
    ///
    /// 只作用于**未暂存的已跟踪文件**（真源那一句文案就是「丢弃所有未暂存的更改吗？」）。
    /// **不含未跟踪路径**：Core 的 `discard` 对未跟踪路径会走 `clean -f -d`
    /// （`git/mod.rs:907-941`），那是**删文件**，而 Core 没有"删除工作区文件"命令
    /// （研究 §4.2 第三条）—— 与"不画没有命令支撑的删除动作"同一条口径。
    ///
    /// ⚠️ **关掉确认框的条件是设置项 `confirmBeforeDiscard === false`**
    /// （真源 `settings.git.confirmDiscard`，默认 `true`；消费点是同一个动作，
    /// 真源 `git-status-panel.tsx` 的丢弃路径同样只看这一个开关）。
    /// 关掉之后**直接走同一条写操作**（[`Self::confirm_discard`]），不绕过任何校验：
    /// 被省掉的只有"弹窗"这一段，`git.write discard` 的参数、失败红条与写后刷新完全一致。
    fn discard_unstaged(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let paths: Vec<SharedString> = self
            .snapshot
            .changes
            .iter()
            .filter(|row| !row.kind.is_untracked() && row.worktree)
            .map(|row| row.path.clone())
            .collect();
        if paths.is_empty() {
            return;
        }

        // 设置关掉了确认框：直接丢弃（同一段写操作与同一套失败处理）。
        if !self.confirm_before_discard {
            eprintln!(
                "S1_SOURCE_CONTROL discard_confirm=off paths={} action=immediate",
                paths.len()
            );
            self.confirm_discard(paths, cx);
            return;
        }

        eprintln!(
            "S1_SOURCE_CONTROL discard_confirm=on paths={} action=dialog",
            paths.len()
        );
        let view = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _window, cx| {
            let confirm_view = view.clone();
            let confirm_paths = paths.clone();
            dialog
                .title(tr(DISCARD_ALL_KEY))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr(DISCARD_CONFIRM_KEY)),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        .child(
                            Button::new("changes-discard-cancel")
                                .label(tr(CANCEL_KEY))
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            Button::new("changes-discard-confirm")
                                .danger()
                                .label(tr(ROLLBACK_KEY))
                                .on_click(move |_, window, cx| {
                                    let paths = confirm_paths.clone();
                                    let _ = confirm_view.update(cx, |view, cx| {
                                        view.confirm_discard(paths, cx)
                                    });
                                    window.close_dialog(cx);
                                }),
                        ),
                )
        });
    }

    /// 确认后的丢弃：跑 `git.write discard` + 刷新。
    fn confirm_discard(&mut self, paths: Vec<SharedString>, cx: &mut Context<Self>) {
        let Some(root) = self.repository_root_text() else {
            return;
        };
        let target: Vec<String> = paths.iter().map(|path| path.to_string()).collect();
        let count = target.len();
        self.busy = true;
        self.error = None;
        self.notice = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(async move { discard(&root, &target) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.busy = false;
                view.record_write("discard", count, outcome.failure, None, cx);
            });
        })
        .detach();
    }

    /// 一次写操作的统一收尾：诊断一行 + 结果进界面 + 写后刷新。
    ///
    /// `notice` = 成功后要显示的短提示（只有提交用得上，其余操作的结果直接体现在列表上）。
    fn record_write(
        &mut self,
        verb: &str,
        paths: usize,
        failure: Option<String>,
        notice: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        match failure {
            Some(failure) => {
                eprintln!(
                    "S1_SOURCE_CONTROL write={verb} paths={paths} result=failed error={failure}"
                );
                // **失败一定可见**：常驻红条（不是 console.error、也不是自动消失的 toast）。
                self.error = Some(SharedString::from(failure));
            }
            None => {
                eprintln!("S1_SOURCE_CONTROL write={verb} paths={paths} result=ok");
                if notice.is_some() {
                    self.notice = notice;
                }
            }
        }
        // **写后刷新**：不管成败都重读一次，界面与仓库不会长期不一致
        // （真源是 `emitGitChanged` + 100 ms 防抖；本侧刻意不做 watcher，见 crate 文档）。
        self.refresh(cx);
    }

    fn repository_root_text(&self) -> Option<String> {
        self.snapshot
            .repository_root
            .as_ref()
            .map(|root| root.to_string())
    }

    /// 卸载时打一行（与 `BottomPane::drop` 同一口径：面板消失这件事要被机器验证看见）。
    fn diagnose_drop(&self) {
        eprintln!(
            "S1_SOURCE_CONTROL run=drop files={} staged={}",
            self.snapshot.changes.len(),
            self.snapshot.staged_count(),
        );
    }

    // ---- 渲染 ----

    /// 标题栏：标题 + 刷新按钮 + 「更多」下拉。
    ///
    /// 组成与度量照 `git-view.tsx:707-710` + `ui/sidebar.tsx:38-47`：高 36、`px-2`、`gap-2`、
    /// 下边框 1px、图标按钮 24×24。
    ///
    /// 「更多」菜单**只放本版真能执行的项**（维护者口径「操作行只列真能执行的」）：
    /// 「刷新状态」与「丢弃全部更改」。真源其余 12 项（选择仓库 / 管理分支 / 推送拉取获取 /
    /// 管理远程标签贮藏 / 工作树 / 交互式变基 / 创建应用补丁 / 初始化仓库）分别需要
    /// 分支-远程-贮藏-补丁四个子系统，都不在第一版范围（研究 §6.3 / §6.4）。
    ///
    /// `this` 是弱引用而不是 `cx.listener`：`dropdown_menu` 的 builder 拿到的是
    /// `&mut App`（没有 `Context<Self>`），只能靠 `WeakEntity::update`
    /// （与 `log_view.rs::scope_button` 同一取法）。
    fn render_title_bar(&self, this: &WeakEntity<Self>, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.busy;
        let discard_enabled = !busy
            && self
                .snapshot
                .changes
                .iter()
                .any(|row| !row.kind.is_untracked() && row.worktree);

        let menu_this = this.clone();
        // 圆角那一槽与 `icon_button` 同因：`Button::rounded` 只吃 `ButtonRounded`（只有
        // `From<Pixels>`），所以这里先按当前 rem 基准求值（见 [`rem_px`]）。
        let more = Button::new("changes-more")
            .ghost()
            .icon(IconName::Ellipsis)
            .tab_stop(false)
            .size_6()
            .rounded(rem_px(cx.theme().font_size, ICON_BUTTON_RADIUS))
            .tooltip(tr(ACTIONS_KEY))
            .accessibility_label(tr(ACTIONS_KEY))
            .dropdown_menu(move |menu, _window, _cx| {
                let refresh_this = menu_this.clone();
                let discard_this = menu_this.clone();
                menu.item(
                    PopupMenuItem::new(tr(REFRESH_KEY)).on_click(move |_event, _window, cx| {
                        let _ = refresh_this.update(cx, |view, cx| view.refresh(cx));
                    }),
                )
                .item(
                    PopupMenuItem::new(tr(DISCARD_ALL_KEY))
                        .disabled(!discard_enabled)
                        .on_click(move |_event, window, cx| {
                            let _ = discard_this.update(cx, |view, cx| {
                                view.discard_unstaged(window, cx)
                            });
                        }),
                )
            });

        h_flex()
            .w_full()
            .flex_shrink_0()
            .h_9()
            .items_center()
            .gap_2()
            .px_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    // `line_height` 没有 rem 档位 helper，用 helper 底层的 `rems()`（原 16px）。
                    .line_height(rems(1.))
                    .text_color(cx.theme().foreground)
                    .child(tr(TITLE_KEY)),
            )
            .child(self.icon_button(
                "changes-refresh",
                IconName::RotateCw,
                tr(REFRESH_KEY),
                !matches!(self.state, LoadState::Loading),
                cx.theme().font_size,
                cx.listener(|view: &mut Self, _event, _window, cx| view.refresh(cx)),
            ))
            .child(more)
            .into_any_element()
    }

    /// 一个自绘图标按钮：24×24 命中区、`ghost` 变体、圆角 6.4。
    ///
    /// 用 `Button` 而不是自绘 `div`（`log_view.rs` 偏差 5 走了另一条路）：这里的按钮
    /// **需要悬停 tooltip**（"暂存 / 取消暂存"的语义只有 tooltip 说得清），而真源给这些
    /// 头部按钮的也正是 `tooltip`（`git-view.tsx:481-492`）。代价是图标会被
    /// `Button` 按 `size` 算成 `24 × 0.75 = 18px`（`gpui-component-0.6.6/src/button/button.rs:580-583`）
    /// 而不是真源的 14 —— 这是本视图已知的一处像素偏差。
    ///
    /// `rem` 是逐层传进来的 rem 基准（`cx.theme().font_size`）：圆角那一槽必须交出 `Pixels`
    /// —— `Button::rounded` 只吃 `ButtonRounded`，而 `ButtonRounded` 只有 `From<Pixels>`
    /// （`gpui-component-0.6.6/src/button/button.rs:29-33`），**没有** `From<Rems>`；
    /// 顺带也省得 `icon_button` 收一个 `&App`（它其余部分不读主题）。
    fn icon_button(
        &self,
        id: &'static str,
        icon: IconName,
        label: SharedString,
        enabled: bool,
        rem: Pixels,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> AnyElement {
        let button = Button::new(id)
            .ghost()
            .icon(icon)
            .tab_stop(false)
            .size_6()
            .rounded(rem_px(rem, ICON_BUTTON_RADIUS))
            .tooltip(label.clone())
            .accessibility_label(label);
        if enabled {
            button.on_click(on_click).into_any_element()
        } else {
            button.disabled(true).into_any_element()
        }
    }

    /// 加载失败错误条（`git-view.tsx:677-685`）：`role="alert"` + `p-3` + destructive 色 + 重试。
    fn render_load_error(&self, message: SharedString, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .p_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().danger.opacity(0.12))
            .text_sm()
            .text_color(cx.theme().danger)
            .child(message)
            .child(
                Button::new("changes-retry")
                    .label(tr(RETRY_KEY))
                    .h_6()
                    .px_1p5()
                    .on_click(cx.listener(|view: &mut Self, _event, _window, cx| view.refresh(cx))),
            )
            .into_any_element()
    }

    /// 写操作失败错误条（**本侧刻意加的真源没有的东西**）。
    ///
    /// 真源的暂存 / 提交失败只 `console.error` 或 toast（研究 §5），而"点了没反应"是本仓库
    /// 明令禁止的交互形态。这里做成**常驻红条**（可手动关闭）：它比 toast 更容易被截图取证，
    /// 也不会在用户还没看清时自动消失。
    fn render_write_error(&self, message: SharedString, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .items_start()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().danger.opacity(0.12))
            .text_sm()
            .text_color(cx.theme().danger)
            .child(
                Icon::new(IconName::TriangleAlert)
                    .size_3p5()
                    .flex_shrink_0()
                    .text_color(cx.theme().danger),
            )
            .child(div().flex_1().min_w_0().child(message))
            .child(self.icon_button(
                "changes-error-dismiss",
                IconName::X,
                tr(CANCEL_KEY),
                true,
                cx.theme().font_size,
                cx.listener(|view: &mut Self, _event, _window, cx| {
                    view.error = None;
                    cx.notify();
                }),
            ))
            .into_any_element()
    }

    /// 写操作成功提示条（只有提交用得上，绿色）。
    fn render_notice(&self, message: SharedString, cx: &mut Context<Self>) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().success.opacity(0.12))
            .text_sm()
            .text_color(cx.theme().success)
            .child(
                Icon::new(IconName::Check)
                    .size_3p5()
                    .flex_shrink_0()
                    .text_color(cx.theme().success),
            )
            .child(div().flex_1().min_w_0().child(message))
            .child(self.icon_button(
                "changes-notice-dismiss",
                IconName::X,
                tr(CANCEL_KEY),
                true,
                cx.theme().font_size,
                cx.listener(|view: &mut Self, _event, _window, cx| {
                    view.notice = None;
                    cx.notify();
                }),
            ))
            .into_any_element()
    }

    /// 操作横幅（`GitOperationBanner`，`git-operation-banner.tsx:71-137`）。
    ///
    /// **必须有**（研究 §6.4 理由最后一条）：提交会被进行中的操作挡住
    /// （`git-commit-panel.tsx:163-172` 的两条守卫），没有横幅用户会看到"提交点了没反应"。
    ///
    /// 外观照真源：`flex flex-col gap-2 border-b bg-raised px-3 py-2.5`、`role="status"`、13px；
    /// `bg-raised` 取主题里最接近的 `tab_bar`（与 `log_view.rs` 的标题栏同一取法）。
    fn render_operation_banner(
        &self,
        operation: &OperationState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut banner = v_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .px_3()
            .py_2p5()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .text_sm()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(IconName::TriangleAlert)
                            // rem 而不是固定 px：真源这一条就是 `size-3.5`（`git-operation-banner.tsx:71-84`），
                            // 即 3.5 × base font —— 走 `size_3p5()` 后横幅的图标与同一行的
                            // `text_sm()` 文字一起随主题字号缩放（《编码指南》「渲染与组合」：
                            // 混用 rem-based text 与 fixed-px icon 必须记录理由，这里没有理由，
                            // 因为值本身就在 rem 档位上）。
                            .size_3p5()
                            .text_color(cx.theme().warning),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child(tr(operation.kind.title_key())),
                    )
                    // 引用短哈希：真源画前 7 位（`git-operation-banner.tsx:79-83`）。
                    .children(operation.reference.as_ref().map(|reference| {
                        div()
                            .min_w_0()
                            .truncate()
                            .text_color(cx.theme().muted_foreground)
                            .child(SharedString::from(format!(
                                "— {}",
                                reference.chars().take(7).collect::<String>()
                            )))
                    })),
            );

        // 进度（只有 rebase 有；Core 也只对 rebase 填这两个字段，`contracts.rs:799-800`）。
        if let (Some(step), Some(total)) = (operation.step, operation.total) {
            banner = banner.child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr_args(
                        OPERATION_STEP_KEY,
                        &[("step", &step.to_string()), ("total", &total.to_string())],
                    )),
            );
        }

        // 冲突提示：真源给「解决 N 个冲突文件，暂存后继续。」，这里补一行路径
        // （上限 `CONFLICT_PATH_LIMIT`），因为"哪些文件"是用户下一步唯一要的信息。
        if operation.has_conflicts() {
            let paths = operation
                .conflicted_paths
                .iter()
                .take(CONFLICT_PATH_LIMIT)
                .map(|path| path.to_string())
                .collect::<Vec<_>>()
                .join("、");
            banner = banner.child(
                v_flex()
                    .gap_1()
                    .child(
                        div().text_color(cx.theme().danger).child(tr_args(
                            RESOLVE_CONFLICTS_HINT_KEY,
                            &[("count", &operation.conflicted_paths.len().to_string())],
                        )),
                    )
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(SharedString::from(paths)),
                    ),
            );
        } else {
            banner = banner.child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr(CONFLICTS_RESOLVED_KEY)),
            );
        }

        // 按钮行：继续（有冲突时禁用，Core 也会拒）/ 跳过（仅 rebase）/ 中止。
        let can_continue = !self.busy && !operation.has_conflicts();
        let mut actions = h_flex().items_center().gap_2().child(
            Button::new("changes-operation-continue")
                .label(tr(operation.kind.continue_key()))
                .h_6()
                .px_1p5()
                .disabled(!can_continue)
                .on_click(cx.listener(|view: &mut Self, _event, _window, cx| {
                    view.run_operation_action("operationContinue", cx)
                })),
        );

        if operation.kind.can_skip() {
            actions = actions.child(
                Button::new("changes-operation-skip")
                    .label(tr(SKIP_KEY))
                    .h_6()
                    .px_1p5()
                    .disabled(self.busy)
                    .on_click(cx.listener(|view: &mut Self, _event, _window, cx| {
                        view.run_operation_action("operationSkip", cx)
                    })),
            );
        }

        actions = actions.child(
            Button::new("changes-operation-abort")
                .label(tr(ABORT_KEY))
                .ghost()
                .h_6()
                .px_1p5()
                .disabled(self.busy)
                .on_click(cx.listener(|view: &mut Self, _event, _window, cx| {
                    view.run_operation_action("operationAbort", cx)
                })),
        );

        banner.child(actions).into_any_element()
    }

    /// 工具行：左 = 分支与 ahead/behind 纯文本，右 = 两个「全部」按钮。
    ///
    /// 真源这一行还有「查看差异」主按钮 + caret 下拉（`git-status-panel.tsx:1108-1142`），
    /// 那需要差异视图子系统（研究 §6.2），第一版不做，所以**整项不出现**
    /// （不画禁用按钮：它旁边没有可解释的替代动作）。
    fn render_toolbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let unstaged = self.snapshot.unstaged_count();
        let staged = self.staged_count();

        let mut row = h_flex()
            .w_full()
            .flex_shrink_0()
            .h_9()
            .items_center()
            .gap_2()
            .px_2()
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap_1()
                    .text_color(cx.theme().muted_foreground)
                    .children(
                        self.snapshot
                            .branch
                            .as_ref()
                            .map(|branch| div().min_w_0().truncate().child(branch.clone())),
                    )
                    // ahead / behind：真源在提交面板底部画两个计数按钮（推送 / 拉取），
                    // 本侧第一版没有远程动作，所以只画计数（偏差 4）。
                    .children((self.snapshot.ahead > 0).then(|| {
                        div().child(SharedString::from(format!("↑{}", self.snapshot.ahead)))
                    }))
                    .children((self.snapshot.behind > 0).then(|| {
                        div().child(SharedString::from(format!("↓{}", self.snapshot.behind)))
                    })),
            );

        if unstaged > 0 {
            row = row.child(self.icon_button(
                "changes-stage-all",
                IconName::Plus,
                tr(STAGE_ALL_KEY),
                !self.busy,
                cx.theme().font_size,
                cx.listener(|view: &mut Self, _event, _window, cx| {
                    view.set_all_staged(true, cx)
                }),
            ));
        }
        if staged > 0 {
            row = row.child(self.icon_button(
                "changes-unstage-all",
                IconName::Minus,
                tr(UNSTAGE_ALL_KEY),
                !self.busy,
                cx.theme().font_size,
                cx.listener(|view: &mut Self, _event, _window, cx| {
                    view.set_all_staged(false, cx)
                }),
            ));
        }

        row.into_any_element()
    }

    /// 分类头：高 32、`px-2`、`text_sm`、右边一个计数徽章
    /// （`git-status-panel.tsx:765-776,858-864`）。
    fn render_section_header(label: SharedString, count: usize, cx: &App) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .h_8()
            .items_center()
            .gap_2()
            .px_2()
            .pb_0p5()
            .text_sm()
            .font_weight(FontWeight::MEDIUM)
            .text_color(cx.theme().foreground)
            .child(label)
            .child(
                // `Badge variant="muted" size="compact"` 的计数（`git-status-panel.tsx:861`）。
                div()
                    .flex_shrink_0()
                    .px_1p5()
                    .rounded(rems(ROW_RADIUS / 16.))
                    .bg(cx.theme().muted)
                    .text_color(cx.theme().muted_foreground)
                    .child(SharedString::from(count.to_string())),
            )
            .into_any_element()
    }

    /// 一行变更（**自写**，理由见模块文档「行组件」一节）。
    ///
    /// 内容照 `git-status-file-item.tsx:119-182`：文件名（按状态染色）+ 目录（弱色）
    /// + 行尾暂存 / 取消暂存按钮（**常显**，偏差 1）。
    fn render_row(&self, row: &ChangeRow, cx: &mut Context<Self>) -> AnyElement {
        let file_name = row
            .path
            .rsplit('/')
            .next()
            .map(SharedString::from)
            .unwrap_or_else(|| row.path.clone());
        let directory = row
            .path
            .rsplit_once('/')
            .map(|(directory, _)| SharedString::from(directory.to_string()));

        let pending = self.pending.contains(&row.path) || self.busy;
        let staged = row.staged;
        let label = tr_args(
            if staged {
                "lithe.git.unstageFileNamed"
            } else {
                "lithe.git.stageFileNamed"
            },
            &[("name", file_name.as_ref())],
        );

        let path = row.path.clone();
        let action = if pending {
            div()
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .size_5()
                .child(Spinner::new().color(cx.theme().muted_foreground))
                .into_any_element()
        } else {
            Button::new(stage_button_element_id(row))
                .ghost()
                .icon(if staged {
                    IconName::Minus
                } else {
                    IconName::Plus
                })
                .tab_stop(false)
                .size_5()
                .rounded(rem_px(cx.theme().font_size, ROW_RADIUS))
                .tooltip(label.clone())
                .accessibility_label(label)
                .on_click(cx.listener(move |view: &mut Self, _event, _window, cx| {
                    view.set_staged(vec![path.clone()], !staged, cx);
                }))
                .into_any_element()
        };

        h_flex()
            .id(change_row_element_id(row))
            .w_full()
            .flex_shrink_0()
            .h_6()
            .items_center()
            .gap_1()
            .px_1p5()
            .rounded(rems(ROW_RADIUS / 16.))
            .text_sm()
            .line_height(relative(ROW_LINE_HEIGHT))
            .whitespace_nowrap()
            .overflow_hidden()
            // 行**可见内容**只有文件名 + 目录（与真源 `git-status-file-item.tsx:132-133` 一致），
            // 但无障碍名带上 Core 的两字符 porcelain 码与 rename 源路径：这两条信息在界面上
            // 没有位置，却正是"这行为什么被算成已跟踪"与"这个文件从哪来"的唯一答案。
            .aria_label(row_aria_label(row))
            .hover(|style| style.bg(cx.theme().list_hover))
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(row.kind.color(cx))
                    .child(file_name),
            )
            .children(directory.map(|directory| {
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(cx.theme().muted_foreground)
                    .child(directory)
            }))
            .child(div().flex_1())
            .child(action)
            .into_any_element()
    }

    /// 变更列表（两个分类：已跟踪 / 未跟踪）。
    fn render_list(&self, cx: &mut Context<Self>) -> AnyElement {
        let tracked = self.snapshot.tracked_rows();
        let untracked = self.snapshot.untracked_rows();
        // 树容器无障碍名 = 「已跟踪文件 / 未跟踪文件」拼接（真源 `git-status-panel.tsx:1190`，
        // 键 `git.trackedFiles` / `git.untrackedFiles`）。
        let tree_label = SharedString::from(format!(
            "{} / {}",
            tr(TRACKED_FILES_KEY),
            tr(UNTRACKED_FILES_KEY)
        ));
        let mut children: Vec<AnyElement> = Vec::new();

        // 两类都各自画分类头（真机只在**该类为 0** 时不画，`git-status-panel.tsx:374`）：
        // 分类头是"这个文件是已跟踪还是未跟踪"在界面上唯一的标识。
        if !tracked.is_empty() {
            children.push(Self::render_section_header(
                tr(TRACKED_KEY),
                tracked.len(),
                cx,
            ));
            for row in &tracked {
                children.push(self.render_row(row, cx));
            }
        }

        if !untracked.is_empty() {
            if !tracked.is_empty() {
                // 分区之间 8（`git-status-panel.tsx:139-141`）。
                children.push(div().w_full().flex_shrink_0().pb_2().into_any_element());
            }
            children.push(Self::render_section_header(
                tr(UNTRACKED_KEY),
                untracked.len(),
                cx,
            ));
            for row in &untracked {
                children.push(self.render_row(row, cx));
            }
        }

        // 滚动容器必须有 `id`：`overflow_y_scroll` 是 `StatefulInteractiveElement` 的方法，
        // 而它只对 `Stateful<Div>` 实现；`id` 同时让滚动偏移跨帧保留。
        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(self.render_toolbar(cx))
            .child(
                v_flex()
                    .id("changes-scroll")
                    .role(Role::Tree)
                    .aria_label(tree_label)
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .px_1p5()
                    .py_2()
                    .overflow_y_scroll()
                    .children(children),
            )
            .into_any_element()
    }

    /// 五态空态 / 加载中。
    ///
    /// 逐条对应研究 §5 的五个返回分支：
    /// ① 加载中 ② 不是仓库 / 未初始化 ③ 是仓库但无提交 ④ 工作区干净 ⑤ 加载失败。
    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        match &self.state {
            LoadState::Loading => self.centered(
                Some(
                    Spinner::new()
                        .color(cx.theme().muted_foreground)
                        .into_any_element(),
                ),
                tr(LOADING_KEY),
                None,
                cx,
            ),
            // ⑤ 加载失败：错误条已经在上面画了（`render_load_error`），这里只留一条重试提示。
            LoadState::Failed(_) => {
                self.centered(None, tr(LOAD_FAILED_KEY), None, cx)
            }
            LoadState::Ready => {
                if self.snapshot.repository_root.is_none() {
                    // ② 未初始化 / 不是仓库。
                    // ⚠️ 「初始化 Git 仓库」是一个真实存在的 Core 命令（`git.initialize`），
                    // 但仓库初始化会改变工作区语义（要重建外壳的项目状态），不在第一版范围，
                    // 所以按约定渲染成**禁用态**并写清原因，而不是画一个点了没反应的按钮。
                    return self.centered(
                        None,
                        tr(NOT_REPOSITORY_KEY),
                        Some((tr(NOT_REPOSITORY_HINT_KEY), tr(INITIALIZE_KEY), false)),
                        cx,
                    );
                }
                if self.snapshot.changes.is_empty() {
                    // ③ 是仓库但还没有任何提交（`changes.rs` 偏差 2：用 `git.references` 判）。
                    if self.snapshot.has_commits == Some(false) {
                        return self.centered(
                            None,
                            tr(NO_COMMITS_KEY),
                            Some((tr(NO_COMMITS_HINT_KEY), SharedString::default(), true)),
                            cx,
                        );
                    }
                    // ④ 工作区干净（真源是 `Empty tone="success"` + Check 图标，
                    // `git-status-panel.tsx:1220-1226`）。
                    return self.centered(
                        Some(
                            Icon::new(IconName::Check)
                                .size_3p5()
                                .text_color(cx.theme().success)
                                .into_any_element(),
                        ),
                        tr(CLEAN_KEY),
                        None,
                        cx,
                    );
                }
                self.render_list(cx)
            }
        }
    }

    /// 空态 / 加载态的公共骨架，照 `ui/empty.tsx:111-130` 的 `EmptyState`：
    /// `Empty`（居中、`gap-2`、`p-3`）+ 可选图标 + `EmptyDescription` 文案
    /// + 可选 `EmptyContent` 里的说明与一个按钮。
    ///
    /// `action` 的三元组是 `(说明, 按钮文案, 是否禁用)`；按钮文案为空时不画按钮
    /// （"无提交"那一档用说明里的话就够了）。
    fn centered(
        &self,
        icon: Option<AnyElement>,
        message: SharedString,
        action: Option<(SharedString, SharedString, bool)>,
        cx: &App,
    ) -> AnyElement {
        let mut header = EmptyHeader::new().gap_2();
        if let Some(icon) = icon {
            header = header.media(EmptyMedia::new().child(icon));
        }
        header = header.description(
            EmptyDescription::new()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(message),
        );

        let mut empty = Empty::new().p_3().header(header);
        if let Some((hint, label, enabled)) = action {
            let mut content = v_flex().gap_2().items_center().child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(hint),
            );
            if !label.is_empty() {
                content = content.child(
                    Button::new("changes-empty-action")
                        .label(label)
                        .h_6()
                        .px_1p5()
                        .disabled(!enabled),
                );
            }
            empty = empty.content(EmptyContent::new().child(content));
        }

        // `Empty` 的根是 `v_flex().flex_1()`，会吃掉剩余高度并把内容居中；外面这层必须是
        // 弹性容器（`v_flex`），否则空态会贴在顶部（`explorer_view.rs:759-762` 同一条）。
        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(empty)
            .into_any_element()
    }

    /// 提交面板（`GitCommitPanel`，`git-commit-panel.tsx:57-419`）。
    ///
    /// 只画三个东西：错误块（有的话）、说明输入框、「已暂存 N 个文件」+「提交」。
    /// **不画**的：AI 生成与设置（`git-commit-panel.tsx:338-372`，属 AI 子系统）、
    /// 「提交并推送」caret（要远程与凭据 UI）、ahead/behind 两个推送拉取按钮
    /// （第一版没有远程动作，计数画在工具行）。
    fn render_commit_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let staged = self.staged_count();
        // 面板内的错误块只承担**硬守卫**（进行中的操作 / 冲突）与最近一次写失败：
        // "没有暂存文件""没写说明"这两种软守卫在下面那行计数文案里已经说清楚了。
        let panel_error: Option<SharedString> =
            self.error.clone().or_else(|| match &self.snapshot.operation {
                Some(operation) if operation.has_conflicts() => Some(tr_args(
                    RESOLVE_CONFLICTS_KEY,
                    &[(
                        "paths",
                        &operation
                            .conflicted_paths
                            .iter()
                            .take(CONFLICT_PATH_LIMIT)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("、"),
                    )],
                )),
                Some(_) => Some(tr(FINISH_OPERATION_KEY)),
                None => None,
            });

        let mut panel = v_flex()
            .w_full()
            .flex_shrink_0()
            .mx_2()
            .mb_2()
            .rounded(rems(COMMIT_PANEL_RADIUS / 16.))
            .border_1()
            .border_color(cx.theme().border)
            .overflow_hidden();

        if let Some(error) = panel_error {
            panel = panel.child(
                div()
                    .mx_2()
                    .mt_2()
                    .px_2()
                    .py_1p5()
                    .rounded(rems(COMMIT_ERROR_RADIUS / 16.))
                    .border_1()
                    .border_color(cx.theme().danger.opacity(0.3))
                    .bg(cx.theme().danger.opacity(0.1))
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(error),
            );
        }

        panel = panel.child(
            Textarea::new(&self.message)
                .h(rems(COMMIT_INPUT_HEIGHT / 16.))
                .disabled(self.busy)
                .w_full()
                .text_sm(),
        );

        let count_label = if staged == 0 {
            tr(NO_FILES_STAGED_KEY)
        } else {
            tr_args(FILES_STAGED_KEY, &[("count", &staged.to_string())])
        };
        let can_commit = !self.busy && self.commit_blocker(cx).is_none();

        panel
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .px_1()
                    .pt_1p5()
                    .pb_1()
                    .text_sm()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(cx.theme().muted_foreground)
                            .child(count_label),
                    )
                    .child(
                        Button::new("changes-commit")
                            .primary()
                            .label(if self.busy {
                                tr(COMMITTING_KEY)
                            } else {
                                tr(COMMIT_KEY)
                            })
                            .h_7()
                            .px_3()
                            .disabled(!can_commit)
                            .on_click(cx.listener(|view: &mut Self, _event, window, cx| {
                                view.commit(window, cx)
                            })),
                    ),
            )
            .into_any_element()
    }

    /// 是否画提交面板：真源只在"正常态"画 footer（`git-view.tsx:768-787`），
    /// 五个空态分支都 `return` 得更早。这里的「正常态」= 读到过仓库根。
    fn shows_commit_panel(&self) -> bool {
        matches!(self.state, LoadState::Ready)
            && self.snapshot.repository_root.is_some()
            && self.snapshot.has_commits != Some(false)
    }
}

impl Drop for ChangesView {
    fn drop(&mut self) {
        self.diagnose_drop();
    }
}

/// 行的无障碍名：`<porcelain 码> <路径>`，rename 时再补 ` ← <源路径>`。
///
/// 三个字段都来自 Core 的 `GitChange`（`contracts.rs:518-527`）：`status` 是**未归一**的两字符
/// porcelain 码（`git/mod.rs:6645-6652`），`originalPath` 只在 rename / copy 时存在。
fn row_aria_label(row: &ChangeRow) -> SharedString {
    let mut label = format!("{} {}", row.raw_status, row.path);
    if let Some(original) = &row.original_path {
        label.push_str(&format!(" ← {original}"));
    }
    SharedString::from(label)
}

/// 变更行的 ElementId：用**仓库相对路径**（[`ChangeRow::path`]）—— 它是 Core 与真源都认的
/// 行身份（同一路径的索引 / 工作树两条记录会先合并成一行，见 `changes.rs` 的 `parse_changes`）。
///
/// 不用下标：暂存 / 取消暂存会让行在两个分类（已跟踪 ↔ 未跟踪）之间移动，`snapshot` 刷新后
/// 同一个下标指向的文件可能整个换掉，元素状态（hover / 滚动锚点）就会串到别的文件上
/// （《编码指南》「稳定标识」/「精确区分领域词汇」：index 是位置，id 是 identity）。
fn change_row_element_id(row: &ChangeRow) -> String {
    format!("changes-row:{}", row.path)
}

/// 行尾暂存按钮的 ElementId：与行同一个 owning object，用同一路径 namespace 分开两个 id
/// （同一个 control 重复出现时以 owning object namespace child ID，`Button::new(("delete-project", project.id))`）。
fn stage_button_element_id(row: &ChangeRow) -> String {
    format!("changes-stage-row:{}", row.path)
}

impl Render for ChangesView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 各段都返回 `AnyElement`（具体类型，不借用 self）：edition 2024 下
        // `-> impl IntoElement` 会捕获作用域内的生命周期，返回 `&self` 派生的类型会和下面
        // `cx.theme()` 的共享借用打架（E0502，`explorer_view.rs:810-817` 记过同一个坑）。
        let this = cx.entity().downgrade();
        let title_bar = self.render_title_bar(&this, cx);
        let load_error = match &self.state {
            // 界面文案 = 通用说明 + Core 的错误码原文：错误码（如 `process_failed`）
            // 比编出来的中文更有排查价值，且不暴露环境细节。
            LoadState::Failed(message) => Some(self.render_load_error(
                SharedString::from(format!("{} {message}", tr(LOAD_FAILED_KEY))),
                cx,
            )),
            _ => None,
        };
        let write_error = self
            .error
            .clone()
            .map(|message| self.render_write_error(message, cx));
        let notice = self
            .notice
            .clone()
            .map(|message| self.render_notice(message, cx));
        let banner = self
            .snapshot
            .operation
            .clone()
            .map(|operation| self.render_operation_banner(&operation, cx));
        let body = self.render_body(cx);
        let commit_panel = self
            .shows_commit_panel()
            .then(|| self.render_commit_panel(cx));

        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(title_bar)
            .children(load_error)
            .children(write_error)
            .children(notice)
            .children(banner)
            .child(body)
            .children(commit_panel)
            // `Ctrl+Enter`：绑定登记在 `crate::install_actions`，处理器放**本视图根元素**上。
            //
            // ⚠️ 能收到是因为 gpui 的 action 沿"焦点节点 → 祖先"冒泡
            // （`gpui-pre-0.3.6/src/window.rs:6333`），而提交说明框是本视图的后代；
            // `ctrl-enter` 在 `gpui-base` / `gpui-component` 里**没有任何绑定**
            // （全量 grep 零命中），所以不会被输入框抢先消费。
            .on_action(
                cx.listener(|view: &mut Self, _: &CommitChanges, window, cx| {
                    view.commit(window, cx)
                }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{change_row_element_id, stage_button_element_id};
    use crate::changes::{ChangeKind, ChangeRow};
    use gpui_kit::SharedString;

    /// 一行的最小 fixture（与 `changes.rs` 的 `row(..)` 同风格）：只填身份与渲染用得到的字段。
    fn row(path: &str, status: &str, untracked: bool) -> ChangeRow {
        ChangeRow {
            path: SharedString::from(path),
            original_path: None,
            raw_status: SharedString::from(status),
            kind: ChangeKind::parse(status, untracked),
            staged: !status.starts_with(' ') && !untracked,
            worktree: !status.ends_with(' ') && !untracked,
        }
    }

    /// 回归判据（《编码指南》「稳定标识」）：同一批行在两种顺序下算出的 id 必须相同，
    /// 不同行算出的 id 必须不同。用下标当 id 时倒序的每一行都会拿到对方的 id，这条会失败。
    #[test]
    fn row_element_ids_do_not_depend_on_the_row_order() {
        // 列表顺序就是"已跟踪分类在前 + 分类内状态序 + 路径升序"（`changes.rs` 的
        // `rows_sort_by_category_then_status_then_path`），这里刻意把它打乱。
        let rows = [
            row("src/main.rs", " M", false),
            row("README.md", "??", true),
            row("src/lib.rs", "D ", false),
        ];

        let forward: Vec<String> = rows.iter().map(change_row_element_id).collect();
        let reversed: Vec<String> = rows.iter().rev().map(change_row_element_id).collect();

        assert_eq!(forward[0], reversed[2], "同一行在不同顺序下必须算出同一个 id");
        assert_eq!(forward[1], reversed[1], "同一行在不同顺序下必须算出同一个 id");
        assert_eq!(forward[2], reversed[0], "同一行在不同顺序下必须算出同一个 id");
        assert_eq!(forward[0], "changes-row:src/main.rs", "id 必须由路径构成");
        // 两两不同：不同文件不能共享元素状态。
        for (left, right) in [(0, 1), (0, 2), (1, 2)] {
            assert_ne!(forward[left], forward[right], "不同文件的 id 必须互不相同");
        }
    }

    /// 同一行的「行」与「暂存按钮」是两个元素：共用同一个 owning object（路径）做 namespace，
    /// 但彼此的 id 必须不同，否则 hover 状态会互相覆盖。
    #[test]
    fn stage_button_id_is_namespaced_apart_from_its_row() {
        let target = row("src/main.rs", " M", false);
        let other = row("src/lib.rs", " M", false);

        assert_ne!(change_row_element_id(&target), stage_button_element_id(&target));
        assert_ne!(stage_button_element_id(&target), stage_button_element_id(&other));
        assert_eq!(
            stage_button_element_id(&target),
            "changes-stage-row:src/main.rs"
        );
    }
}
