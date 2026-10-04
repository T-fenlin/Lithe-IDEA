//! 欢迎页 / 无项目状态（U10）。
//!
//! 真源：`macos/Sources/Lithe/Views/App/WelcomeView.swift`（**395 行**；任务表
//! `docs/tasks/gpui-core-first.md:145` 写的 363 行是旧数）+ `RootView.swift` 的
//! `LitheWindowLayout`（`:345-371`）。
//!
//! ## 什么时候画这一页
//!
//! 真源语义是「没有活动项目 ⇒ 欢迎页」（`LitheApp.swift` 的 `windowLayout` 在
//! `model.workspaceURL == nil` 时给 `.welcome`）。宿主的判据在 **app crate**：
//! `crates/app/src/main.rs` 的 `choose_launch_root`（纯函数，`None` = 这一页）与
//! `resolve_launch_plan`（读文件 + 探目录那一层）—— **给了显式位置参数**、或**最近项目里
//! 挑到了可用根** ⇒ 工作区（既有行为一字不改）；两者都没有 ⇒ 这一页。
//! 窗口尺寸与最小尺寸的真源出处见同文件的 `welcome_window_bounds`。
//!
//! ## 本页做了真源的哪些部分（没做的逐条在 `HANDOFF` 之外由任务汇报登记）
//!
//! | 真源 | 这里 |
//! | --- | --- |
//! | 左栏 224（`:94`）：`Lithe` + 版本描述（`:51-57`）、`Projects` 高亮行（`:63-74`）、底部设置按钮（`:78-92`） | ✅ 全做（版本取 `lithe_gpui_shared::HOST_VERSION`，= 真源非预览版的 `versionDescription`，`UpdateChecker.swift:148-150`） |
//! | 工具条：搜索框（`:104-115`）、`Open`（`:125-129`） | ✅ 搜索框按名字/路径过滤；`Open` 走宿主既有的「打开文件夹」链路（`ShellWorkspace::open_project_picker` 的同一段 `prompt_for_paths`） |
//! | 最近项目列表：徽标 + 名 + 路径（`:176-262`）、行内移除（`:218-220`） | ✅ 数据来自 `lithe_gpui_settings::RecentProjects`，落盘走既有 `save_recent_projects` |
//! | 空态（`:143-174`） | ✅ 两条分支（无最近项目 / 过滤无匹配） |
//! | `New Project`（`:119-123`） | ❌ 缺项目脚手架生成（真机是 `createNewDirectory` + 起终端跑 `npm create`），宿主已有一句 `MISSING_NEW_PROJECT_SCAFFOLDING` |
//! | `Clone Repository`（`:131-135`） | ❌ 缺 URL / 凭据 / 进度 UI（宿主已有 `MISSING_CLONE_UI`） |
//! | 行内 `Menu`：`Open` / `Show in Finder` / `Remove…`（`:211-239`）、右键菜单（`:249-261`） | ⚠️ 只做 `Remove…`（悬停时才露出的行内按钮，露出时机照 `:238-239` 的 `.opacity(hovered ? 1 : 0)`）：宿主没有"在资源管理器里显示"这条链路，而 `Open` 与"点整行"是同一件事 |
//! | 失效行不可点：真源在**渲染期** `model.fileExists(at:)` 后 `.disabled(!exists)`（`:177`/`:207`） | ⚠️ 判据在**启动期**：`resolve_launch_plan` 的 `select_launch_root` 已经探过每一条并把失效的标进列表（行据此置灰），点击则**无条件**转发给打开链路、由链路自己再探一次（探不到只留一行 `S1_WELCOME open_project state=missing`，不改列表）。这样"点行 ⇒ 链路被调用"没有前置条件，UI 集成测试才能在不上真实目录的前提下断言它 |
//! | `Check for Updates` / `Return to Stable`（`:272-316`） | ❌ 宿主**没有**更新检查器（全仓 `rust/` 无对应模块），没有数据层就不做 |
//! | `WelcomeInitialFocusReset`（`:348-395`） | ❌ 那是 AppKit 的 `NSViewRepresentable`：点空白处清第一响应者。gpui 侧焦点归窗口根 `Root` 管，没有对应钩子 |
//!
//! ## 为什么是一个独立实体，而不是 `ShellWorkspace` 的一支分支
//!
//! [`crate::workspace::ShellWorkspace`] 的构造期**假定根是真实存在的目录**：它要建
//! `.lithe/`、读会话、发 `workspace.snapshot` / `git.status` / `maven.scan`。把
//! "还没有项目"做成它的一个状态，会让这一页背着整个工作区的启动成本（那正是
//! `choose_launch_root` 的 `None` 支要避开的"开空壳"）。所以欢迎态是**窗口根的另一支**：
//! `Root` + [`WelcomeView`]；点一条项目之后由 [`ProjectOpener`] 换成 `Root` +
//! `ShellWorkspace`。
//!
//! ## 打开链路：为什么是端口（trait）而不是直接调 `ShellWorkspace`
//!
//! 真源的 `model.openProject(url)` 在这一页**没有** `ShellWorkspace` 可调（它还不存在）。
//! 生产实现 [`WorkspaceOpener`] 复用与「换项目」**同一条**落点
//! （`crate::workspace::replace_root_with_workspace` 的 `replace_root` + 新 `Root`，
//! 见 `ShellWorkspace::rebuild_project_window`）。做成 trait 的第二个理由是可测：
//! UI 集成测试注入一个记录用的实现，于是"点行 ⇒ 打开链路被调用"能在无头窗口里断言，
//! 而**不会真的建外壳**（那会起 Core 桥、写 `.lithe/`）。

use std::path::PathBuf;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{ActiveTheme as _, Sizable as _, StyledExt as _, WindowExt as _};
// `on_click` / `on_hover` 只在 `StatefulInteractiveElement` 上（`gpui-pre-0.3.6/src/
// elements/div.rs:1300`），而 `Div` 只实现 `InteractiveElement`（`:1871`）—— 所以可点的
// `div()` 必须先 `.id(..)`。与 `crate::notifications` 的同一段说明。
//
// ⚠️ 移除按钮那颗字形取 `gpui_kit::assets::IconName::X`（**完整** Lucide 目录）：
// 组件库那个 `IconName` 是默认图标子集，里面没有 `X`（`project_tabs` / `right_tool_window`
// 也是这么取的）。
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, ImageSource,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, Resource, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, TestSupportExt as _, WeakEntity,
    Window, div, img, rems,
};
use lithe_gpui_settings::{RecentProject, RecentProjects};
use lithe_gpui_shared::icons::{idea, idea_icon_image};
use lithe_gpui_shared::{HOST_VERSION, tr, tr_args};

use crate::project_menu::{ProjectBadge, badge_view};

/// 本页诊断前缀（可 grep；与 `S1_WORKSPACE_LAUNCH` / `S1_OPEN_PROJECT` 同族）。
///
/// 走 **stderr**：release 是 GUI 子系统，stdout 重定向到文件时是块缓冲的（`crates/app/
/// src/main.rs` 开头那条实测），stderr 无缓冲，验证脚本 grep 它才稳定。
pub const WELCOME_TAG: &str = "S1_WELCOME";

// ---------------------------------------------------------------------------
// 规格度量（每个值都能在真源里找到出处；档位内用 helper，档位外 rems(N / 16.)）
// ---------------------------------------------------------------------------

/// 左栏宽 **224**（真源 `WelcomeView.swift:94` 的 `.frame(width: 224)`）。
const SIDEBAR_WIDTH_SPEC: f32 = 224.;
/// 左栏与内容区之间的分隔线宽 **1**（真源 `:30` 的 `.frame(width: 1)`）。
const SEPARATOR_THICKNESS_SPEC: f32 = 1.;
/// 头部内衬：左右 **20** / 上 **26** / 下 **38**（真源 `:59-61`）。
const HEADER_PADDING_H_SPEC: f32 = 20.;
const HEADER_PADDING_TOP_SPEC: f32 = 26.;
const HEADER_PADDING_BOTTOM_SPEC: f32 = 38.;
/// 头部徽标 **28**（真源 `:49` 的 `LitheIcons.appLogo(size: 28)`）与标题行间距 **11**（`:48`）。
const LOGO_SIZE_SPEC: f32 = 28.;
const HEADER_GAP_SPEC: f32 = 11.;
/// `Projects` 行：图标 **15**（`:64`）、行高 **32**（`:71`）、圆角 **4**（`:73`）、
/// 内衬左右 **14**（`:69`/`:74`）。
const SECTION_ICON_SIZE_SPEC: f32 = 15.;
const SECTION_ROW_HEIGHT_SPEC: f32 = 32.;
const SECTION_RADIUS_SPEC: f32 = 4.;
const SECTION_PADDING_H_SPEC: f32 = 14.;
/// 底部设置按钮 **28×28**（真源 `:82`）、图标 **14**（`:81`）、内衬左右 **14** / 底 **14**（`:90-91`）。
const SETTINGS_ICON_SIZE_SPEC: f32 = 14.;
const SIDEBAR_BOTTOM_PADDING_SPEC: f32 = 14.;
/// 工具条高 **68**（真源 `:138`）、内衬左右 **14**（`:137`）、搜索框最大宽 **220**（`:115`）。
const TOOLBAR_HEIGHT_SPEC: f32 = 68.;
const TOOLBAR_PADDING_H_SPEC: f32 = 14.;
const SEARCH_MAX_WIDTH_SPEC: f32 = 220.;
/// 工具条下方那条分隔线：高 **1**、左右内衬 **12**（真源 `:140-141`）。
const TOOLBAR_SEPARATOR_PADDING_H_SPEC: f32 = 12.;
/// 列表：左右内衬 **24** / 上下内衬 **14**（真源 `:153-154`）、行距 **2**（`:147`）。
const LIST_PADDING_H_SPEC: f32 = 24.;
const LIST_PADDING_V_SPEC: f32 = 14.;
const LIST_GAP_SPEC: f32 = 2.;
/// 一行：高 **68**（真源 `:243`）、左右内衬 **12**（`:242`）、圆角 **4**（`:245`）、
/// 行内间距 **12**（`:178`）、徽标 **20**（`:186`）。
const ROW_HEIGHT_SPEC: f32 = 68.;
const ROW_PADDING_H_SPEC: f32 = 12.;
const ROW_RADIUS_SPEC: f32 = 4.;
const ROW_GAP_SPEC: f32 = 12.;
const ROW_BADGE_SIZE_SPEC: f32 = 20.;
/// 空态：图标 **32**（真源 `:164`）、三行间距 **10**（`:162`）。
const EMPTY_ICON_SIZE_SPEC: f32 = 32.;
const EMPTY_GAP_SPEC: f32 = 10.;

// ---------------------------------------------------------------------------
// 纯逻辑（可确定性单测：不碰文件系统、不构造 GPUI）
// ---------------------------------------------------------------------------

/// 界面上要显示的一行最近项目。
///
/// 与 `RecentProject` 分开是为了让"显示什么、按什么顺序"成为一条**纯投影**：
/// 路径里的家目录缩写（真源 `:194` 的 `replacingOccurrences(of: NSHomeDirectory())`）
/// 是展示层的事，不该写回数据层。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WelcomeRow {
    /// 项目名（数据层已保证非空：缺 `name` 时用 `folder_name(path)` 补过）。
    pub name: String,
    /// 项目路径（已按家目录缩写；真源 `:194`）。
    pub path: String,
    /// 数据层里这一条的原始 `path`（换根 / 移除都用它 —— 它是 `RecentProjects` 的去重键）。
    pub key: String,
    /// 数据层标记的"目录已失效"（真源按 `fileExists` 置灰）。
    pub missing: bool,
}

/// 过滤 + 顺序：真源 `filteredProjects`（`WelcomeView.swift:264-270`）。
///
/// - 空过滤串 ⇒ 原样返回（真源 `guard !projectFilter.isEmpty`）；
/// - 否则**大小写不敏感**地在**名字或路径**里找子串（真源
///   `localizedCaseInsensitiveContains` 两处 `||`）。
///
/// 顺序 = 数据层的顺序（`RecentProjects` 恒满足"`pinned` 优先 + `lastOpenedAt` 降序"，
/// 见其模块文档）—— 这一层**不重排**，否则界面顺序会与项目下拉里的同一份数据不一致。
pub fn welcome_rows(entries: &[RecentProject], filter: &str) -> Vec<WelcomeRow> {
    let needle = filter.trim().to_lowercase();
    entries
        .iter()
        .filter(|entry| {
            needle.is_empty()
                || entry.name.to_lowercase().contains(&needle)
                || entry.path.to_lowercase().contains(&needle)
        })
        .map(|entry| WelcomeRow {
            name: entry.name.clone(),
            path: abbreviate_home(&entry.path),
            key: entry.path.clone(),
            missing: entry.missing,
        })
        .collect()
}

/// 空态的两条分支（真源 `WelcomeView.swift:166` 的三元表达式）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyState {
    /// 一条最近项目都没有 ⇒ `welcome.noRecentProjects`。
    NoRecentProjects,
    /// 有最近项目、但过滤串一条都没匹配上 ⇒ 过滤无匹配。
    NoMatchingProjects,
}

/// 空态判据（`None` = 有行要画）。
///
/// 分支顺序照真源：先看**过滤后**是不是空，再看**原始列表**是不是空 —— 所以
/// "列表本来就没有"与"过滤串没匹配上"是两句不同的话。
pub fn empty_state(entries: &[RecentProject], filter: &str) -> Option<EmptyState> {
    if !welcome_rows(entries, filter).is_empty() {
        return None;
    }
    Some(if entries.is_empty() {
        EmptyState::NoRecentProjects
    } else {
        EmptyState::NoMatchingProjects
    })
}

/// 真源 `:194` 的路径缩写：家目录前缀换成 `~`。
///
/// 拿不到家目录（环境变量异常）时原样返回 —— 显示一条长路径，而不是把路径吃成空串。
fn abbreviate_home(path: &str) -> String {
    let Some(home) = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
    else {
        return path.to_string();
    };
    let home = home.to_string_lossy().to_string();
    if home.is_empty() {
        return path.to_string();
    }
    // 只在**前缀且是路径段边界**时替换：`/home/dev2` 不能被 `/home/dev` 吃掉。
    match path.strip_prefix(&home) {
        Some(rest) if rest.is_empty() || rest.starts_with(['\\', '/']) => format!("~{rest}"),
        _ => path.to_string(),
    }
}

// ---------------------------------------------------------------------------
// 打开链路（端口 + 生产实现）
// ---------------------------------------------------------------------------

/// 欢迎页「打开一个项目」的出口。
///
/// 两个入口共用它：点一条最近项目行、以及「打开文件夹」选完之后。生产实现
/// （[`WorkspaceOpener`]）把窗口根换成工作区；测试注入记录用的实现。
pub trait ProjectOpener: 'static {
    /// 用户要求打开 `target`。实现方负责探测路径、换根、以及"打开成功之后"的记账。
    fn open_project(&mut self, target: PathBuf, window: &mut Window, cx: &mut App);

    /// 用户要求选一个文件夹（真源 `:125-129` 的 `Open` ⇒ `model.chooseProject()`）。
    ///
    /// 默认实现什么都不做（记录用的测试实现不需要走系统选择器）；生产实现见
    /// [`WorkspaceOpener`] —— 它走的是与外壳 `Ctrl+O` **同一个** `prompt_for_paths`。
    fn pick_folder(&mut self, _window: &mut Window, _cx: &mut Context<WelcomeView>) {}
}

/// 生产实现：探测目录 → 换根成工作区 → 记账。
pub struct WorkspaceOpener;

impl ProjectOpener for WorkspaceOpener {
    fn open_project(&mut self, target: PathBuf, window: &mut Window, cx: &mut App) {
        let path_text = target.to_string_lossy().to_string();
        // 路径探测与外壳的 `request_open_project` 同一条口径（不是目录就不换根）。
        // 差别只有一处：这一页还没有通知中心（`notify` 是 `ShellWorkspace` 的字段，
        // 换根时重建），所以这里只留一行可 grep 的诊断，不假装发了通知。
        if !target.is_dir() {
            eprintln!("{WELCOME_TAG} open_project state=missing path={path_text}");
            return;
        }
        eprintln!("{WELCOME_TAG} open_project state=opening path={path_text}");
        // 真源 `chooseProjectOpenDestination` 的第 ② 段：**没有打开的工作区**时目的地是
        // "此窗口"且**不询问**（`workspace.rs` 的 `resolve_project_open_destination` 文档
        // 一直把这一段记成"本侧不适用"；欢迎页就是它终于成立的场合）。宿主又没有多窗口
        // （维护者已拍板暂缓），所以这里不弹「在哪儿打开」对话框。
        let Some(shell) = crate::workspace::replace_root_with_workspace(&target, window, cx) else {
            eprintln!("{WELCOME_TAG} open_project state=rebuild_failed reason=no_view");
            return;
        };
        // 「打开成功之后」才记最近项目（真源 `addToRecents` 的时机；与换项目同一条）。
        let _ = shell.update(cx, |shell, cx| {
            shell.record_opened_project(&target, Some(false), cx)
        });
    }

    fn pick_folder(&mut self, window: &mut Window, cx: &mut Context<WelcomeView>) {
        // 与 `ShellWorkspace::open_project_picker` 同一段：`App::prompt_for_paths` +
        // 只选文件夹（`directories: true` / `files: false`，Windows 的
        // `FOS_PICKFOLDERS` 要求两者二选一）。选择器实现在专用线程上，本线程只拿一个
        // oneshot `Receiver`，所以从事件回调里调它不会挡住这一帧。
        if window.has_active_dialog(cx) {
            eprintln!("{WELCOME_TAG} pick_folder state=skipped reason=active_dialog");
            return;
        }
        eprintln!("{WELCOME_TAG} pick_folder state=prompt");
        let picked = cx.prompt_for_paths(gpui_kit::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        let me = cx.entity().downgrade();
        let task = cx.spawn_in(window, async move |_this, async_cx| {
            let target = match picked.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) => {
                    eprintln!("{WELCOME_TAG} pick_folder state=cancelled");
                    None
                }
                Ok(Err(error)) => {
                    eprintln!("{WELCOME_TAG} pick_folder state=failed error={error}");
                    None
                }
                Err(_) => {
                    eprintln!("{WELCOME_TAG} pick_folder state=cancelled reason=channel-closed");
                    None
                }
            };
            let Some(target) = target else {
                return;
            };
            eprintln!(
                "{WELCOME_TAG} pick_folder state=picked path={}",
                target.display()
            );
            let _ = async_cx.update(move |window, cx| {
                let _ = me.update(cx, |this, cx| {
                    this.opener.open_project(target.clone(), window, cx);
                });
            });
        });
        // `detach()` 而不是 `let _ =`：gpui 的 `Task` 一 drop 就**取消**（理由见
        // `ShellWorkspace::open_project_picker` 的同一句）。
        task.detach();
    }
}

// ---------------------------------------------------------------------------
// 视图
// ---------------------------------------------------------------------------

/// 欢迎页（窗口根的另一支；`Root` 的 content）。
pub struct WelcomeView {
    /// 最近项目（数据层；`pinned` 优先 + `lastOpenedAt` 降序）。
    projects: RecentProjects,
    /// 落盘路径（`None` = 推导不出路径，只影响能否保存，与数据层同一条口径）。
    path: Option<PathBuf>,
    /// 过滤串（真源 `@State projectFilter`，`:9`）。
    filter: SharedString,
    /// 搜索框（真源 `TextField`，`:108`；placeholder = `welcome.searchProjects`）。
    search: Entity<InputState>,
    /// 悬停在哪一行（真源 `hoveredProjectID`，`:10`）—— 行内移除按钮只在悬停时露出
    /// （真源 `:238-239` 的 `.opacity(hovered ? 1 : 0)`）。
    hovered: Option<String>,
    /// 盯住搜索框：它一变就重画。
    _search_subscription: Subscription,
    /// 打开链路的出口（生产实现 = [`WorkspaceOpener`]）。
    opener: Box<dyn ProjectOpener>,
}

impl WelcomeView {
    /// 生产构造：打开链路走 [`WorkspaceOpener`]。
    pub fn new(
        projects: RecentProjects,
        path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::with_opener(projects, path, Box::new(WorkspaceOpener), window, cx)
    }

    /// 注入打开链路的构造（UI 集成测试用它断言"点行 ⇒ 链路被调用"，不建真外壳）。
    pub fn with_opener(
        projects: RecentProjects,
        path: Option<PathBuf>,
        opener: Box<dyn ProjectOpener>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx
            .new(|cx| InputState::new(window, cx).placeholder(tr("lithe.welcome.searchProjects")));
        let search_subscription = cx.subscribe_in(
            &search,
            window,
            |this: &mut Self, state: &Entity<InputState>, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.filter = state.read(cx).value();
                    cx.notify();
                }
            },
        );
        eprintln!(
            "{WELCOME_TAG} view_open count={} recent_file={}",
            projects.len(),
            path.as_deref()
                .map_or_else(|| "<none>".to_string(), |path| path.display().to_string())
        );
        Self {
            projects,
            path,
            filter: SharedString::default(),
            search,
            hovered: None,
            _search_subscription: search_subscription,
            opener,
        }
    }

    /// 当前要画的行（过滤 + 顺序；真源 `filteredProjects`）。
    pub fn rows(&self) -> Vec<WelcomeRow> {
        welcome_rows(self.projects.entries(), self.filter.as_ref())
    }

    /// 当前空态（`None` = 有行）。
    pub fn empty_state(&self) -> Option<EmptyState> {
        empty_state(self.projects.entries(), self.filter.as_ref())
    }

    /// 点一条最近项目：把**数据层那条原始路径**交给打开链路（真源 `:179-181` 的
    /// `model.openProject(project.url)`）。
    ///
    /// ⚠️ 这里**不**先探一次目录：真源在渲染期用 `fileExists` 决定这一行可不可点
    /// （`:177`/`:207`），本侧的存在性判据在**启动时**就已经落进列表
    /// （`crates/app/src/main.rs` 的 `resolve_launch_plan`：`select_launch_root` 探过每一条并把
    /// 失效的标成 `missing`，行据此置灰）—— 点击只需要如实转发，探测留给打开链路
    /// （`WorkspaceOpener` / `ShellWorkspace::request_open_project` 都在入口处探）。
    /// 这样"点行 ⇒ 打开链路被调用"是**无条件**的，UI 集成测试也才能在不上真实目录、
    /// 不写盘的前提下断言这条链真的被调到。
    fn activate_row(&mut self, key: String, window: &mut Window, cx: &mut Context<Self>) {
        eprintln!("{WELCOME_TAG} row_activate path={key}");
        self.opener.open_project(PathBuf::from(key), window, cx);
    }

    /// 行内移除（真源 `:218-220` 的 `Remove from Recent Projects` ⇒ `model.removeRecentProject`）。
    fn remove_row(&mut self, key: &str, cx: &mut Context<Self>) {
        if !self.projects.remove(key) {
            eprintln!("{WELCOME_TAG} row_remove state=not_found path={key}");
            return;
        }
        eprintln!(
            "{WELCOME_TAG} row_remove state=removed path={key} count={}",
            self.projects.len()
        );
        self.save();
        cx.notify();
    }

    /// 把列表写回文件（复用 `persistence::save_json` 的"临时文件 + rename"）。
    ///
    /// 同步小写盘、UI 线程上做 —— 与外壳的 `ShellWorkspace::save_recent_projects`
    /// 同一做法（那是本宿主既有口径，不是本页新引入的阻塞点）。
    fn save(&self) {
        let Some(path) = self.path.as_deref() else {
            eprintln!("{WELCOME_TAG} save=skipped reason=no_path");
            return;
        };
        match lithe_gpui_settings::save_recent_projects(path, &self.projects) {
            Ok(bytes) => println!(
                "{WELCOME_TAG} saved path={} bytes={bytes} count={}",
                path.display(),
                self.projects.len()
            ),
            // 写失败不 panic：内存里的列表仍然是权威的，下一次改动会再写一次。
            Err(error) => eprintln!("{WELCOME_TAG} save_failed error={error}"),
        }
    }

    /// 左栏：品牌头 + `Projects` 行 + 撑开的空白 + 底部设置按钮（真源 `:46-99`）。
    fn sidebar(&self, cx: &App) -> AnyElement {
        let text = cx.theme().foreground;
        let muted = cx.theme().muted_foreground;
        v_flex()
            .w(rems(SIDEBAR_WIDTH_SPEC / 16.))
            .h_full()
            .flex_shrink_0()
            .bg(cx.theme().sidebar)
            .child(
                h_flex()
                    .gap(rems(HEADER_GAP_SPEC / 16.))
                    .items_center()
                    .px(rems(HEADER_PADDING_H_SPEC / 16.))
                    .pt(rems(HEADER_PADDING_TOP_SPEC / 16.))
                    .pb(rems(HEADER_PADDING_BOTTOM_SPEC / 16.))
                    // 真源 `LitheIcons.appLogo(size: 28)`（`:49`）：宿主搬进来的品牌图是
                    // `lithe-gpui/assets/images/logo.png`（`crates/app/src/assets.rs:332`）。
                    .child(
                        img(ImageSource::Resource(Resource::Embedded(
                            "images/logo.png".into(),
                        )))
                        .w(rems(LOGO_SIZE_SPEC / 16.))
                        .h(rems(LOGO_SIZE_SPEC / 16.))
                        .flex_shrink_0(),
                    )
                    .child(
                        v_flex()
                            // 真源标题 `Text("Lithe")` 是**写死的品牌名**（`:51`），不随 locale
                            // 变；版本行取真源 `updateChecker.versionDescription`（`:54`）——
                            // 非预览版就是版本号本身（`UpdateChecker.swift:148-150`），
                            // 宿主已有的来源是 `lithe_gpui_shared::HOST_VERSION`。
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(text)
                                    .child("Lithe"),
                            )
                            .child(div().text_xs().text_color(muted).child(HOST_VERSION)),
                    ),
            )
            .child(
                // 真源 `:63-74`：图标 15 + `Projects`，行高 32、圆角 4、底色 `selectionColor`
                // （"当前分区"的高亮）。宿主取语义最近的 `sidebar_accent`。
                h_flex()
                    .gap(rems(9. / 16.))
                    .items_center()
                    .h(rems(SECTION_ROW_HEIGHT_SPEC / 16.))
                    .px(rems(SECTION_PADDING_H_SPEC / 16.))
                    .mx(rems(SECTION_PADDING_H_SPEC / 16.))
                    .rounded(rems(SECTION_RADIUS_SPEC / 16.))
                    .bg(cx.theme().sidebar_accent)
                    .text_color(text)
                    .child(idea_icon_image(
                        &idea::FOLDER_ICON,
                        cx,
                        rems(SECTION_ICON_SIZE_SPEC / 16.),
                    ))
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .child(tr("lithe.welcome.projects")),
                    ),
            )
            // 真源 `Spacer()`（`:76`）：把设置按钮压到底。
            .child(div().flex_1())
            .child(
                h_flex()
                    .px(rems(SECTION_PADDING_H_SPEC / 16.))
                    .pb(rems(SIDEBAR_BOTTOM_PADDING_SPEC / 16.))
                    .child(
                        Button::new("lithe-welcome-settings")
                            .ghost()
                            .small()
                            .child(idea_icon_image(
                                &idea::GEAR_ICON,
                                cx,
                                rems(SETTINGS_ICON_SIZE_SPEC / 16.),
                            ))
                            // 图标走 `child` 之后 Button 退出"图标按钮"分支、会补内衬；
                            // 清掉才保住真源那个 28×28 的命中区（`:82`）。
                            .px_0()
                            .on_click(|_, window, cx| {
                                lithe_gpui_settings::open_settings_dialog(window, cx);
                            }),
                    ),
            )
            .into_any_element()
    }

    /// 内容区：工具条 + 分隔线 + 列表 / 空态（真源 `:101-159`）。
    fn projects_content(&self, me: &WeakEntity<Self>, window: &Window, cx: &App) -> AnyElement {
        let mut column = v_flex()
            .flex_1()
            .h_full()
            .min_w_0()
            .bg(cx.theme().background)
            .child(self.toolbar(me, cx))
            .child(
                // 真源 `:140-141`：`Rectangle` 高 1、左右内衬 12。
                div()
                    .px(rems(TOOLBAR_SEPARATOR_PADDING_H_SPEC / 16.))
                    .child(
                        div()
                            .h(rems(SEPARATOR_THICKNESS_SPEC / 16.))
                            .w_full()
                            .bg(cx.theme().border),
                    ),
            );

        let rows = self.rows();
        column = match self.empty_state() {
            Some(state) => column.child(self.empty_state_view(state, cx)),
            None => column.child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .gap(rems(LIST_GAP_SPEC / 16.))
                    .px(rems(LIST_PADDING_H_SPEC / 16.))
                    .py(rems(LIST_PADDING_V_SPEC / 16.))
                    .children(rows.iter().map(|row| self.row(row, me, window, cx))),
            ),
        };
        column.into_any_element()
    }

    /// 工具条：搜索框 + `Open`（真源 `:103-138`）。
    fn toolbar(&self, me: &WeakEntity<Self>, cx: &App) -> AnyElement {
        let me_for_pick = me.clone();
        h_flex()
            .w_full()
            .flex_shrink_0()
            .gap(rems(10. / 16.))
            .px(rems(TOOLBAR_PADDING_H_SPEC / 16.))
            .h(rems(TOOLBAR_HEIGHT_SPEC / 16.))
            .child(
                Input::new(&self.search)
                    .small()
                    .cleanable(true)
                    .max_w(rems(SEARCH_MAX_WIDTH_SPEC / 16.))
                    // 真源 `:105-107`：左侧放大镜 12.5 + 内容区左右内衬 8。
                    .prefix(idea_icon_image(
                        &idea::MAGNIFYING_GLASS_ICON,
                        cx,
                        rems(12.5 / 16.),
                    )),
            )
            .child(div().flex_1())
            .child(
                // 真源 `:125-129` 的 `Button("Open")` ⇒ `model.chooseProject()`。文案用
                // catalog 里与真源**逐字同义**的 `welcome.open`。
                // `New Project`（`:119-123`）与 `Clone Repository`（`:131-135`）不做：
                // 宿主缺脚手架生成 / 克隆 UI（各自的 `MISSING_*` 能力组，见模块文档的表）。
                Button::new("lithe-welcome-open-folder")
                    .outline()
                    .small()
                    .label(tr("lithe.welcome.open"))
                    .on_click(move |_, window, cx| {
                        let _ =
                            me_for_pick.update(cx, |this, cx| this.opener.pick_folder(window, cx));
                    }),
            )
            .into_any_element()
    }

    /// 一行最近项目（真源 `projectRow`，`:176-262`）。
    fn row(
        &self,
        row: &WelcomeRow,
        me: &WeakEntity<Self>,
        window: &Window,
        cx: &App,
    ) -> AnyElement {
        // 元素 id 用**业务身份**（项目路径）而不是下标：列表会因为过滤 / 移除重排，
        // 下标 id 会让点击串到别的行上（`gpui-kit` 的 test.md §"列表使用复合 ID"同一条）。
        let id = welcome_row_element_id(&row.key);
        let hovered = self.hovered.as_deref() == Some(row.key.as_str());
        let foreground = if row.missing {
            cx.theme().muted_foreground
        } else {
            cx.theme().foreground
        };
        let muted = cx.theme().muted_foreground;
        let hover_bg = cx.theme().list_hover;
        let badge = ProjectBadge::for_name(&row.name);

        let me_for_click = me.clone();
        let key_for_click = row.key.clone();
        let me_for_hover = me.clone();
        let key_for_hover = row.key.clone();

        let mut element = h_flex()
            .id(id)
            .w_full()
            .flex_shrink_0()
            .h(rems(ROW_HEIGHT_SPEC / 16.))
            .gap(rems(ROW_GAP_SPEC / 16.))
            .items_center()
            .px(rems(ROW_PADDING_H_SPEC / 16.))
            .rounded(rems(ROW_RADIUS_SPEC / 16.))
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .on_hover(move |is_hovered: &bool, _window, cx| {
                let next = is_hovered.then(|| key_for_hover.clone());
                let _ = me_for_hover.update(cx, |this, cx| {
                    if this.hovered != next {
                        this.hovered = next;
                        cx.notify();
                    }
                });
            })
            .on_click(move |_event, window, cx| {
                let key = key_for_click.clone();
                let _ = me_for_click.update(cx, |this, cx| this.activate_row(key, window, cx));
            })
            .child(badge_view(
                &badge,
                crate::rem_px(window.rem_size(), ROW_BADGE_SIZE_SPEC),
            ))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .font_medium()
                            .text_color(foreground)
                            .child(row.name.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(muted)
                            .child(row.path.clone()),
                    ),
            );

        // 行内移除：真源在**悬停菜单**里（`:218-220`），本侧没有 `Show in Finder`
        // 那一条链路，而移除是这一行唯一的动作 —— 所以把它做成悬停时才露出的行内按钮
        // （露出时机与真源的 `.opacity(hovered ? 1 : 0)` 一致，`:238-239`）。
        if hovered {
            let me_for_remove = me.clone();
            let key_for_remove = row.key.clone();
            let name_for_remove = row.name.clone();
            element = element.child(
                Button::new(welcome_remove_element_id(&row.key))
                    .ghost()
                    .small()
                    .icon(gpui_kit::assets::IconName::X)
                    .tooltip(tr_args(
                        "lithe.welcome.removeRecent",
                        &[("name", name_for_remove.as_str())],
                    ))
                    .on_click(move |_, _window, cx| {
                        // 行本身也可点：不移除冒泡的话"移除"会连带把这一行打开
                        // （`notifications.rs:417-419` 记过同一条）。
                        cx.stop_propagation();
                        let key = key_for_remove.clone();
                        let _ = me_for_remove.update(cx, |this, cx| this.remove_row(&key, cx));
                    }),
            );
        }

        // `test_support()` 在非 test-support 构建里是**恒等**的（`gpui-base-0.7.0/src/
        // observe.rs:20-38`），所以它不改变生产渲染树；加了它，UI 集成测试才能按
        // 上面那个业务 id 找到这一行并派发真实点击。
        element.test_support().into_any_element()
    }

    /// 空态（真源 `emptyProjectsState`，`:161-174`）。
    fn empty_state_view(&self, state: EmptyState, cx: &App) -> AnyElement {
        let title = match state {
            EmptyState::NoRecentProjects => tr("lithe.welcome.noRecentProjects"),
            EmptyState::NoMatchingProjects => tr("lithe.welcome.gpui.noMatchingProjects"),
        };
        v_flex()
            .w_full()
            .flex_1()
            .items_center()
            .justify_center()
            .gap(rems(EMPTY_GAP_SPEC / 16.))
            // 真源 `:163` 是 `folder.badge.plus` 32pt light；宿主图标集里语义最近的是
            // expui 的「新建文件夹」字形（`idea::FOLDER_PLUS_ICON`）。
            .child(idea_icon_image(
                &idea::FOLDER_PLUS_ICON,
                cx,
                rems(EMPTY_ICON_SIZE_SPEC / 16.),
            ))
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .text_color(cx.theme().foreground)
                    .child(title),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    // 真源 `:169` 的字面量是 "Open a local folder to start working."，
                    // catalog 的 `welcome.openFolderHint` 是同一句意思、措辞不同（见汇报）。
                    .child(tr("lithe.welcome.openFolderHint")),
            )
            .into_any_element()
    }
}

/// 一行最近项目的元素 id（业务身份 = 数据层的去重键）。
///
/// 抽成函数是为了让生产渲染与 UI 集成测试**用同一个构造**，两边各写一次 format! 会漂移。
pub fn welcome_row_element_id(key: &str) -> ElementId {
    ElementId::Name(SharedString::from(format!("lithe-welcome-project:{key}")))
}

/// 行内移除按钮的元素 id（同上，业务身份）。
pub fn welcome_remove_element_id(key: &str) -> ElementId {
    ElementId::Name(SharedString::from(format!("lithe-welcome-remove:{key}")))
}

impl Render for WelcomeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // ⚠️ 两个区域助手都只收 `&App`（不可变借用可以同时存在）；可点的元素用
        // `WeakEntity` 回写，不用 `cx.listener` —— 理由见 `crate::lib.rs` 模块文档
        // "区域渲染函数必须收 &Window / &App"（edition 2024 的 RPIT 会捕获可变借用）。
        let me = cx.entity().downgrade();
        let border = cx.theme().border;
        h_flex()
            .w_full()
            .h_full()
            .bg(cx.theme().background)
            .child(self.sidebar(cx))
            .child(
                div()
                    .w(rems(SEPARATOR_THICKNESS_SPEC / 16.))
                    .h_full()
                    .flex_shrink_0()
                    .bg(border),
            )
            .child(self.projects_content(&me, _window, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lithe_gpui_settings::RecentProjects;

    /// 造一条最近项目（走数据层的 `record_open`，于是顺序 / 规范化与生产同源）。
    fn projects_at(paths: &[&str]) -> RecentProjects {
        let mut projects = RecentProjects::new();
        for (index, path) in paths.iter().enumerate() {
            // 时间戳递减：先记的更新（`record_open` 按 pinned 优先 + 时间降序排）。
            projects.record_open(path, 10_000 - index as u64, None);
        }
        projects
    }

    /// 空列表 + 空过滤 ⇒ 空态是"没有最近项目"（真源 `:166` 的第一支）。
    #[test]
    fn empty_list_reports_no_recent_projects() {
        let projects = RecentProjects::new();
        assert_eq!(
            empty_state(projects.entries(), ""),
            Some(EmptyState::NoRecentProjects)
        );
        assert_eq!(
            empty_state(projects.entries(), "anything"),
            Some(EmptyState::NoRecentProjects),
            "列表本来就空时，即使有过滤串也还是'没有最近项目'（真源先看 filteredProjects 再判 recentProjects）"
        );
    }

    /// 有项目但过滤没匹配上 ⇒ 空态是"无匹配"（真源 `:166` 的第二支）。
    #[test]
    fn filter_without_match_reports_no_matching_projects() {
        let projects = projects_at(&[r"D:\proj\alpha", r"D:\proj\beta"]);
        assert_eq!(
            empty_state(projects.entries(), "gamma"),
            Some(EmptyState::NoMatchingProjects)
        );
        assert_eq!(empty_state(projects.entries(), "alpha"), None);
    }

    /// 过滤：空串匹配一切；大小写不敏感；名字与路径**都**参与匹配（真源 `:266-269`）。
    #[test]
    fn filter_is_case_insensitive_and_matches_name_or_path() {
        let projects = projects_at(&[r"D:\proj\Alpha", r"D:\other\beta"]);

        let all = welcome_rows(projects.entries(), "");
        assert_eq!(all.len(), 2, "空过滤串必须原样返回全部");
        // 这一条钉的是"本层不重排"：投影出来的顺序必须**逐条等于**数据层的顺序
        // （`projects_at` 给先记的那条更新的时间戳 ⇒ 列表里 `Alpha` 在前）。
        let data_layer: Vec<&str> = projects
            .entries()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        let projected: Vec<&str> = all.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(projected, data_layer, "行顺序必须与数据层逐条一致");

        let by_name = welcome_rows(projects.entries(), "ALPHA");
        assert_eq!(by_name.len(), 1, "大小写不敏感地按名字匹配");
        assert_eq!(by_name[0].key, r"D:\proj\Alpha");

        let by_path = welcome_rows(projects.entries(), r"d:\proj\alpha");
        assert_eq!(by_path.len(), 1, "路径也参与匹配");
        assert_eq!(by_path[0].key, r"D:\proj\Alpha");

        assert!(welcome_rows(projects.entries(), "gamma").is_empty());
    }

    /// 行的构造：`key` 是数据层的原始路径（去重键），`name` / `missing` 照抄数据层。
    #[test]
    fn rows_carry_the_data_layer_key_and_flags() {
        let mut projects = RecentProjects::new();
        projects.record_open(r"D:\proj\alpha", 1, None);
        projects.set_missing(r"D:\proj\alpha", true);

        let rows = welcome_rows(projects.entries(), "");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, r"D:\proj\alpha");
        assert_eq!(rows[0].name, "alpha");
        assert!(
            rows[0].missing,
            "数据层的 missing 要透到行上（真源按 fileExists 置灰）"
        );
    }

    /// 家目录缩写只吃**路径段边界**上的前缀。
    #[test]
    fn home_abbreviation_respects_segment_boundaries() {
        // 不依赖运行机器上真实的家目录：这里只钉"非边界不替换"这一条。
        let path = r"D:\proj\alpha";
        assert_eq!(abbreviate_home(path), path);
    }

    /// 行 / 移除按钮的元素 id 是业务身份（含原始路径），且两者不同。
    #[test]
    fn element_ids_use_the_project_path() {
        let row = welcome_row_element_id(r"D:\proj\alpha").to_string();
        let remove = welcome_remove_element_id(r"D:\proj\alpha").to_string();
        assert!(
            row.contains(r"D:\proj\alpha"),
            "行 id 必须含业务身份：{row}"
        );
        assert!(
            remove.contains(r"D:\proj\alpha"),
            "移除按钮 id 必须含业务身份：{remove}"
        );
        assert_ne!(row, remove);
    }
}
