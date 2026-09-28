//! 标题栏「分支弹窗」面板（Windows 规格 → gpui 落地）。
//!
//! 规格真源（一手源码，逐条核对过行号）：`gpui/research/windows/12-branch-manager.md`
//! （下称「研究」，561 行），对应 Windows 前端
//! `windows/tauri/src/features/git/components/git-branch-manager.tsx`（1050 行）。
//! 数据边界在 `lithe-gpui-git::branch_info`（`git.status` + `git.references`）。
//!
//! # 面板结构（自绘三段 + `Command` 的列表）
//!
//! 真源是 `Command`（Base UI Dialog + 遮罩）里的四段（`git-branch-manager.tsx:632-863`）：
//!
//! ```text
//! 搜索行（px 16 / py 12 / 下边框）→ 页签行（px 8 / pt 8）→ 列表（内边距 8 / 行 ≥36）→ 底栏（px 12 / py 12 / 上边框）
//! ```
//!
//! 本侧用 `Dialog` + `Command`（研究 §6.3 的硬结论：`PopupMenu` **不能**当这个面板 ——
//! 没有输入框、任一点击即 dismiss；先例见 `crate::command_palette`），但**四段里只有列表交给
//! `Command`**，其余三段自绘。理由逐条：
//!
//! | 段 | 归谁 | 理由 |
//! | --- | --- | --- |
//! | 搜索行 | **自绘** | `Command` 的搜索行是内部的 `Input`，**没有**放「{n} 个分支」徽章的位置（`command/state.rs:842-857`）；而且徽章的计数是**过滤前**的总数（真源 `:679`），必须由本模块算 |
//! | 页签行 | **自绘** | 结构性缺口：`Command` 的渲染顺序是 header → **搜索框** → 列表 → footer（`command/state.rs:838-935`），`.header()` 画在搜索框**上方**，**没有**「搜索框下方、列表上方」的槽位（研究 §6.3「唯一的结构性缺口」） |
//! | 列表 | `Command` | 行虚拟化 / 悬停与键盘高亮 / 空态 / 滚动条都由它提供；自绘这一层等于重做 `v_virtual_list` |
//! | 底栏 | `Command::footer` | 它的 footer 画在列表**之后**、面板的最下方（`command/state.rs:933-935`），与真源的 sticky 底栏位置一致；行内两个按钮自绘（真源 `:790-809`） |
//!
//! ## ⚠️ 搜索行自绘 → `Command.searchable(false)`，代价逐条
//!
//! 自绘搜索行意味着 `Command` 收不到打字（研究 §6.3 路线 B）。本侧取这条路线的**理由**：
//! 徽章与页签的位置是硬需求（徽章 = 过滤前总数，页签必须在搜索框下方），而 `Command` 的
//! 搜索行两者都放不下。代价与补偿：
//!
//! 1. **过滤自己做**：`Command` 用 `.filterable(false)`（`item_matches` 在 `!searchable` 时
//!    一律返回 `true`，`state.rs:307-313`），本模块的 [`visible_rows`] 按真源的
//!    `matchesSearchQuery` 过滤后**只喂可见行**给 `Command`（研究 §6.3 路线 B 写的正是这一招）。
//!    因此 `Command` 的行号 = 可见行号，`on_confirm` 的 `IndexPath.row` 直接是下标。
//! 2. **键盘导航弱化**：`Command` 的 `↑`/`↓`/`Enter` 处理器挂在它自己的 `key_context
//!    "Command"` 上（`state.rs:28,822-827`），焦点在**自绘的搜索框**里时这些键不归它。
//!    所以本面板的键盘能力只有搜索框自己的编辑键 + `Esc`（见 [`render_search`] 的
//!    `on_key_down`）；鼠标是完整的一条路（悬停高亮 + 点击）。
//!    这是**已知且有意**的弱化，不是漏做：真源用的是同一个 `Command` 的搜索框，
//!    本侧为了让徽章与页签落到正确位置放弃了它。
//! 3. **`.max_h` 仍然给 512**：列表上限与真源的面板最大高一致（研究 §1.8），
//!    `Command` 的默认值是 300（`command/command.rs:34`），不放开只会用到一半。
//!
//! # v1 = 只读面板（研究 §7.1 的 A 档）
//!
//! 能做的（**零新 Core 命令**）：标题栏分支项（图标 + 分支名 + ahead/behind 箭头）、
//! 搜索框、本地分支列表（前导勾 / 分支图标 + 分支名 + 右侧「当前」徽章）、空态两句、
//! 底栏刷新、页签行。
//!
//! 做不到的（**不画可点控件**，逐条给前置条件）：
//!
//! | 缺的 | 缺什么（都**没有**写进界面） |
//! | --- | --- |
//! | 切分支（点列表行） | `git.checkoutPreflight` + `git.write{checkout}` 两条命令本侧都没有调用点（研究 §6.2） |
//! | 新建分支（底栏那一颗） | `git.write{createBranch}`；真源「创建后不检出」（`:442-456`） |
//! | 「…」行内菜单（合并 / 变基 / 删除） | `git.write{merge\|rebase\|deleteBranch}`（研究 §4.3-4.4） |
//! | 工作树页签 | `git.worktrees` + `git.write{createWorktree\|removeWorktree}` |
//! | 仓库页签 | `workspace.repositories`（**非 git 命名空间**，本侧没有调用点） |
//!
//! 三个页签因此是「**分支**可点（默认选中，点了重读列表）+ 另两个**禁用态**」
//! （`disabled(true)` + `muted_foreground` + 无 hover）：它们**在语义与视觉上都是不可用**
//! （不是只靠颜色），并且各自的前置条件每次打开都打一行 `S1_BRANCH_PANEL tab=… state=disabled
//! precondition=…`，所以"为什么灰着"可 grep。这一条与 `crate::project_menu` 的三条动作行
//! 同一口径（维护者 2026-09-26 拍板），**不画"点了没用"的死控件**。
//!
//! # ⚠️ 失败不再静默（与真源的**有意**差异）
//!
//! 真源的新建 / 删除分支失败是**完全静默**的：`git-branches-api.ts:143-146,252-255` 只
//! `console.error` 并返回 `false`，调用方在 `false` 时不提示、不关面板（研究 §4.2/§4.3/§4.6
//! 把这条登记为缺陷）。本侧**不照抄**：任何失败都必须有用户可见的反馈。v1 的两条读命令
//! （`git.status` / `git.references`）失败时：
//!
//! 1. 面板里画一条**错误条**（[`render_error_bar`]，`px_2 py_1.5` + `theme.danger` + 文案），
//!    与真源"只有 toast、面板内没有错误条"（研究 §4.6）不同 —— 这是有意的改进；
//! 2. 诊断行里带上失败的命令名（`S1_BRANCH_PANEL failures=…`），无人值守也能取证。
//!
//! # ⚠️ 触发器必须是拖拽区的**兄弟节点**
//!
//! Windows 的命中测试取 `window_control_hitboxes` 里**第一个**命中项
//! （`gpui-pre-0.3.6/src/window.rs:1952-1956`，按绘制顺序 = 祖先在前），祖先的 `Drag` 会赢 ——
//! 把触发器放进 `drag_region` 内部，点它就只会拖窗口。这一条与窗口三键、菜单栏、项目下拉是
//! 同一个坑（`crate::title_bar` 模块头有完整说明），所以 [`crate::title_bar::title_bar`] 把本模块
//! 画出来的元素插在 `drag_region` **之前**，作为它的兄弟。触发器另外挂了一句
//! `window.prevent_default() + cx.stop_propagation()`（`AppMenuBar` 的同一句，
//! `app_menu_bar.rs:272-280`）作为第二道保险。
//!
//! # 触发器上**没有** `▾`
//!
//! 真源的子节点只有三样：分支图标 / 分支名 / ahead-behind 箭头（`git-branch-manager.tsx:646-661`）。
//! 维护者截图里那个 caret 是**左邻项目下拉**的 `ChevronDownIcon`（`title-project-menu.tsx:157-163`），
//! 两者只隔 2px（`title-bar.tsx:236-238`；研究 §1.2 与 §7.2 第 1 条）。本模块**不画** caret。
//!
//! # 已知偏差（各给理由）
//!
//! | 偏差 | 真源 | 本侧 | 理由 |
//! | --- | --- | --- | --- |
//! | 搜索框所在位置 | 页签在搜索框**下方** | 页签在搜索框**上方**（`Command::header`） | `Command` 没有该槽位；搜索框是主输入，留在顶部比压在页签下更符合"搜索为主"的读法（见上表） |
//! | 键盘高亮 | `↑`/`↓`/`Enter` 全通 | 只有搜索框的编辑键 + `Esc` | 见上「键盘导航弱化」 |
//! | 分支名宽度 | `min(max(len+1,6),40)ch` | `max_w(rems(320. / 16.))` + `truncate` | gpui 没有 `ch` 单位，按 13px 字号的 40ch ≈ 320 逻辑 px 取上限（研究 §7.1 的注意 2 已登记这条偏离） |
//! | 面板内的关闭按钮 | 有（`aria` = 关闭命令面板） | 只有 `Dialog` 自带的关闭按钮 | `Command` 的搜索行没有放它的位置（同上）；`Esc` / 点遮罩 / Dialog 的关闭按钮三条路都在 |
//! | 计数徽章 | `{n} 个分支`，胶囊、`≥28` 高 | 同文案，同一颗胶囊 | — |
//! | 页签胶囊的间距 | 4（`gap-0` + 自身 padding） | `gap_1()` = 4 | — |

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{ActiveTheme as _, Icon, WindowExt as _};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, DefiniteLength, Entity, Focusable as _,
    FontWeight, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, rems,
};

use lithe_gpui_git::{BranchInfo, BranchSnapshot};
use lithe_gpui_shared::{tr, tr_args};

// ---------------------------------------------------------------------------
// 度量（规格值逐条来自研究 §1.8 的度量表，出处见图示行号）
// ---------------------------------------------------------------------------

/// 面板宽 704：`w-[min(44rem,calc(100vw-2rem))]`（`ui/command.tsx:29`，44 × 16 = 704）。
///
/// 与命令面板**同一个值**（`crate::command_palette::PANEL_WIDTH`）：真源两边都走 `ui/command.tsx`。
const PANEL_WIDTH_SPEC: f32 = 704.;

/// 面板最大高 512：同上 `max-h-[min(68vh,32rem)]`（32 × 16 = 512）。
const PANEL_MAX_HEIGHT_SPEC: f32 = 512.;

/// 距窗口顶 64：遮罩层 `fixed inset-0 … pt-16`（`ui/command.tsx:143`）。
///
/// ⚠️ 与命令面板**不同**（那边是 16，`crate::command_palette::PANEL_TOP_INSET`）：真源两支
/// 用的是同一个 `ui/command.tsx`，`pt-16` = 64 是本面板规格里明确写死的一项
/// （研究 §1.8 第 1 行），验收要按 64 逻辑 = 80 物理量。`Dialog::margin_top` 收绝对像素
/// （`dialog/dialog.rs:529` 的 `y` 直接当 `top` 用），所以这里给的就是"面板上边距窗口顶"。
const PANEL_TOP_INSET_SPEC: f32 = 64.;

/// 搜索行高 28：真源搜索框 `h-7`（`ui/command.tsx:55`）。
const SEARCH_ROW_HEIGHT_SPEC: f32 = 28.;

/// 页签胶囊最小高 32：真源 `min-h-8`（`ui/tabs.tsx:61`）。
const TAB_HEIGHT_SPEC: f32 = 32.;

/// 分支行最小高 36：真源 `min-h-9`（`git-branch-manager.tsx:904`）。
///
/// ⚠️ **必须显式给下限**：`CommandItem::child` 的自绘内容决定行高，不设 `min_h` 时
/// 短分支名会把行压得比同列表里别的行矮（`crate::command_palette` 的同一处结论，
/// `command_palette.rs:475-477`）。
const BRANCH_ROW_HEIGHT_SPEC: f32 = 36.;

/// 行圆角 8：真源 `rounded-lg`（`ui/command.tsx:41`）。
///
/// 8 虽然在 gpui 档位上，但 `rounded_lg()` 是 Tailwind 的 8、与 Lithe 的 `rounded-lg`
/// = `--radius × 1` = 8 恰好同值 —— 这里写规格常量是为了与
/// [`crate::project_menu::ROW_RADIUS_SPEC`]（6.4，真源 `rounded-md`）区分开，不混用两个语义。
/// `Styled::rounded` 收 `impl Into<AbsoluteLength>`，所以调用点写 `rems(ROW_RADIUS_SPEC / 16.)`。
const ROW_RADIUS_SPEC: f32 = 8.;

/// 分支名宽度上限 320：真源 `maxWidth = min(max(len+1,6),40)ch`（`git-branch-manager.tsx:189-190`）。
///
/// gpui 没有 `ch` 单位，按 13px 字号下 40ch ≈ 320 逻辑 px 取固定上限（研究 §7.1 的注意 2）。
const BRANCH_NAME_MAX_WIDTH_SPEC: f32 = 320.;

/// 徽章最大宽 160：真源计数徽章 `max-w-40`（`ui/command.tsx:92`）。
const BADGE_MAX_WIDTH_SPEC: f32 = 160.;

/// 底栏按钮高 32：真源 `Button default size` 的 `h-8`（`ui/button.tsx:22`）。
const FOOTER_BUTTON_HEIGHT_SPEC: f32 = 32.;

/// 触发器的圆角 6.4：真源 `rounded-md` = `--radius × 0.8`（`styles/theme.css:7,134`）。
///
/// 只给 [`trigger`] 用；`Dialog` 的 `margin_top` / `width` 那一侧走 [`crate::rem_px`]，
/// 因为它们是**固有方法**（只吃 `Pixels`）。这里挂在 `div` 上，`Styled::rounded` 收
/// `impl Into<AbsoluteLength>`，所以调用点直接写 `rems(TRIGGER_RADIUS_SPEC / 16.)`。
const TRIGGER_RADIUS_SPEC: f32 = 6.4;

// ---------------------------------------------------------------------------
// 页签行（自绘：`Command` 没有这个槽位）
// ---------------------------------------------------------------------------

/// 三个页签。顺序照真源的数组字面量：`仓库` → `分支` → `工作树`（`git-branch-manager.tsx:608-630`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanelTab {
    /// `git.repositories` = 仓库。
    Repositories,
    /// `git.branches` = 分支。**默认页签**（真源 `:158,528`）。
    Branches,
    /// `git.worktrees` = 工作树。
    Worktrees,
}

impl PanelTab {
    /// 渲染顺序（= 键盘顺序 = 真源数组顺序）。
    const ALL: [Self; 3] = [Self::Branches, Self::Repositories, Self::Worktrees];

    /// v1 唯一可点的页签。
    const ACTIVE: Self = Self::Branches;

    /// 文案键（真源 `git.repositories` / `git.branches` / `git.worktrees`，`locale.ts:6836-6838`）。
    fn label_key(self) -> &'static str {
        match self {
            Self::Repositories => "lithe.git.repositories",
            Self::Branches => "lithe.git.branches",
            Self::Worktrees => "lithe.git.worktrees",
        }
    }

    /// 图标：真源 `FolderOpenIcon` / `GitBranchIcon` / `NodesIcon`（`git-branch-manager.tsx:609-629`）。
    ///
    /// ⚠️ **`工作树` 那一颗是替代品**：真源的 `NodesIcon` 是 Windows 侧的图标代理
    /// （`ui-icons` 的 `expui` 集或 React 内联组件），**Lucide 目录里没有 `nodes` 这个字形**
    /// （`gpui-kit-assets-0.6.6/assets/icons/` 全量清单里没有 `nodes.svg`，
    /// 本轮实测 `IconName::Nodes` 这一变体不存在）。语义最近的是 `folder-tree`，所以取
    /// `IconName::FolderTree` —— 这是**已知的外观偏差**（研究 §6.2 的图标一栏没有提到它，
    /// 因为那一步还没到画图标的时候）。
    fn icon(self) -> IconName {
        match self {
            Self::Repositories => IconName::FolderOpen,
            Self::Branches => IconName::GitBranch,
            Self::Worktrees => IconName::FolderTree,
        }
    }

    /// 诊断行 `S1_BRANCH_PANEL tab=…` 的取值（可 grep 的契约）。
    fn id(self) -> &'static str {
        match self {
            Self::Repositories => "repositories",
            Self::Branches => "branches",
            Self::Worktrees => "worktrees",
        }
    }

    /// 这个页签缺的**前置条件**（只进诊断与 `aria_label`，不是界面文案 —— 所以不走 i18n）。
    fn precondition(self) -> &'static str {
        match self {
            // v1 没有可点的页签，所以 ACTIVE 也有一条（它"缺"的是别的页签，不是它自己）。
            Self::Branches => "read_only_v1",
            Self::Repositories => "no_workspace_repositories_call_site",
            Self::Worktrees => "no_git_worktrees_call_site",
        }
    }
}

/// 页签行：`px 8 / pt 8`，三颗胶囊、间距 4、图标 14、`分支` 选中、其余**禁用态**。
///
/// 度量照研究 §1.5：容器 `shrink-0 gap-0 bg-background px-2 pt-2`（`ui/command.tsx:495`）、
/// 胶囊 `min-h-8 px-3 rounded-full` + 图标与文字间 6（`ui/tabs.tsx:55,61`）、
/// 选中态 `data-active:bg-accent/80 data-active:text-foreground`（`:55`）。
///
/// ⚠️ **必须自绘**：`Command` 渲染顺序是 header → 搜索框 → 列表 → footer
/// （`command/state.rs:838-935`），`.header()` 在搜索框**上方**，没有"搜索框下方、列表上方"
/// 的槽位（研究 §6.3）。这是本面板**唯一**的结构性自绘点。
///
/// ⚠️ **另两个页签是禁用态，不是"点了没用的控件"**：v1 缺 `workspace.repositories` 与
/// `git.worktrees` 的调用点（见模块头的表），所以它们不挂点击、前景 `muted_foreground`
/// （`disabled` 在 `gpui-kit` 里不自动改色，这里显式取色）、没有 hover 高亮，
/// 并且 `aria_label` 里写明缺什么 —— "不可用"在语义与视觉上都能区分。
fn render_tabs(cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let active_bg = theme.accent;
    let active_fg = theme.foreground;
    let muted = theme.muted_foreground;

    let tabs = PanelTab::ALL.into_iter().map(move |tab| {
        let selected = tab == PanelTab::ACTIVE;
        let (background, foreground) = match selected {
            true => (Some(active_bg), active_fg),
            false => (None, muted),
        };
        let mut pill = h_flex()
            .id(SharedString::from(format!("branch-panel-tab-{}", tab.id())))
            .flex_shrink_0()
            .items_center()
            .min_h(rems(TAB_HEIGHT_SPEC / 16.))
            .px_3()
            .gap_1p5()
            .rounded_full()
            .text_sm()
            .text_color(foreground)
            // 页签图标 14：真源 `size-3.5`（`git-branch-manager.tsx:63,616-629`）。
            .child(Icon::new(tab.icon()).size_3p5())
            .child(tr(tab.label_key()))
            .aria_label(tr(tab.label_key()));

        if selected {
            pill = pill.bg(background.unwrap_or(active_bg));
        } else {
            // 禁用态：不挂点击、不挂 hover、前景灰（`disabled` 在 gpui-kit 里**不**自动改色，
            // 所以上面按 `muted` 取过色）；`aria_label` 里补上"缺什么"，
            // 于是"不可用"在读屏与 grep 两侧都说得清（不是只靠颜色）。
            pill = pill
                .cursor_default()
                .aria_label(SharedString::from(format!(
                    "{} — {}",
                    tr(tab.label_key()),
                    tab.precondition()
                )));
        }

        pill
    });

    h_flex()
        .id("branch-panel-tabs")
        .flex_shrink_0()
        .items_center()
        .gap_1()
        .px_2()
        .pt_2()
        // 页签容器的无障碍名：真源 `aria-label` = `git.selectorSections` = Git 选择分区
        // （`git-branch-manager.tsx:691`）。带 `id` 才能挂 `aria_label`
        // （`StatefulInteractiveElement` 的方法）。
        .aria_label(tr("lithe.git.selectorSections"))
        .children(tabs)
}

// ---------------------------------------------------------------------------
// 搜索行（自绘：计数徽章的位置）
// ---------------------------------------------------------------------------

/// 搜索行：`px 16 / py 12`、下边框、输入框高 28、右侧计数徽章。
///
/// 度量照研究 §1.4 与 §1.8：`flex items-center gap-2 px-4 py-3` + `border-b`
/// （`ui/command.tsx:52,210`）、输入框 `h-7 min-w-0 flex-1`（`:55`）、
/// 徽章 `h-auto min-h-7 max-w-40 px-2 … bg-surface/70` + `rounded-full`（`:92`）。
///
/// ⚠️ **徽章的计数是过滤前的总数**（真源 `git-branch-manager.tsx:679` 取 `branches.length`，
/// 不是 `filteredBranches.length`）：本侧照此，所以 `total` 与 `visible` 是两个参数。
///
/// ⚠️ **`Esc` 不在这里处理**：`gpui-kit` 的 `Input` 没有 `on_key_down` 之类的快捷键入口
/// （0.6.6 的 `input/input.rs` 只有 `on_paste`），而 `Dialog` 自己带 `keyboard(true)` +
/// `close_on_escape`（`dialog/dialog.rs:584`）。所以关闭的三条路（`Esc` / 点遮罩 /
/// Dialog 的关闭按钮）全部由 `Dialog` 提供，面板状态经 `.on_close(..)` 回到 `open=false`。
fn render_search(
    input: &Entity<InputState>,
    total: usize,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();

    h_flex()
        .flex_shrink_0()
        .items_center()
        .gap_2()
        .px_4()
        .py_3()
        .border_b_1()
        .border_color(theme.border)
        // 搜索框：`h-7`(28) + `min-w-0 flex-1`；`appearance(false)` 去掉 gpui-kit 输入框
        // 自带的边框 / 底色（真源这里是裸输入框 + 下边框归外层的搜索行），
        // `p_0()` 去掉它自带的水平内距（真源靠搜索行的 `px-4`）。
        .child(
            div()
                .flex_1()
                .min_w_0()
                .h(rems(SEARCH_ROW_HEIGHT_SPEC / 16.))
                .child(
                    Input::new(input)
                        .appearance(false)
                        .p_0()
                        .h_full()
                        .aria_label(tr("lithe.git.searchBranchesAria")),
                ),
        )
        // 计数徽章：`{n} 个分支`（单复数两键同值，真源 `git.branchCount` / `git.branchesCount`）。
        // ⚠️ `{count}` 是**总数**，理由见函数文档。
        .child(
            h_flex()
                .flex_shrink_0()
                .items_center()
                .min_h(rems(SEARCH_ROW_HEIGHT_SPEC / 16.))
                .max_w(rems(BADGE_MAX_WIDTH_SPEC / 16.))
                .px_2()
                .rounded_full()
                .bg(theme.muted.opacity(0.7))
                .text_sm()
                .text_color(theme.muted_foreground)
                .truncate()
                .child(tr_args(
                    match total {
                        1 => "lithe.git.branchCount",
                        _ => "lithe.git.branchesCount",
                    },
                    &[("count", &total.to_string())],
                )),
        )
}

// ---------------------------------------------------------------------------
// 底栏（`Command::footer`）
// ---------------------------------------------------------------------------

/// 底栏：右边一颗「刷新」。真源是 `px 12 / py 12 / 上边框 / gap 8`（`ui/command.tsx:343,345`）。
///
/// ⚠️ **只画「刷新」，不画「新建分支」**：真源左边那颗是 `git.newBranch`（`:790-809`），
/// 它走 `git.write{createBranch}` —— 本侧没有这条命令的调用点（见模块头）。画一颗按下去
/// 什么都不发生的按钮就是对用户说谎，所以整颗不画（底栏因此元素更少，但每个都真的能用）。
/// 新建分支的前置条件写在诊断行里（`S1_BRANCH_PANEL action=newBranch state=absent …`）。
fn render_footer(panel: &Entity<BranchPanel>, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let handle = panel.downgrade();
    let idle_fg = theme.muted_foreground;
    let hover_bg = theme.accent;
    let hover_fg = theme.foreground;

    h_flex()
        .flex_shrink_0()
        .items_center()
        .justify_end()
        .gap_2()
        .px_3()
        .py_3()
        .border_t_1()
        .border_color(theme.border)
        .child(
            // 刷新按钮：真源 `Button variant=default`（`bg-accent text-foreground`）+
            // `h-8 px-3 gap-1.5 [&_svg]:size-4`（`ui/button.tsx:13,22`；`ui/command.tsx:714-720`）。
            // 自绘而不是用 gpui-kit 的 `Button`：`Button::default()` 的底色是 `primary`
            // （`button.rs` 的 `ButtonVariant::Default`），而真源这里是 `bg-accent`；
            // 自绘与 `crate::project_menu` 的触发器同一口径（`trigger` 的文档）。
            div()
                .id("branch-panel-refresh")
                .flex()
                .flex_shrink_0()
                .items_center()
                .h(rems(FOOTER_BUTTON_HEIGHT_SPEC / 16.))
                .px_3()
                .gap_1p5()
                .rounded(rems(ROW_RADIUS_SPEC / 16.))
                .bg(theme.accent)
                .text_sm()
                .text_color(theme.foreground)
                .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
                .on_click(move |_event, window, cx| {
                    let _ = handle.update(cx, |panel, cx| panel.refresh(window, cx));
                })
                .child(Icon::new(IconName::RefreshCw).size_4().text_color(idle_fg))
                .child(tr("lithe.git.refresh")),
        )
}

// ---------------------------------------------------------------------------
// 列表行（`CommandItem::child` 全自绘）
// ---------------------------------------------------------------------------

/// 让 [`BranchInfo`] 能被 `Command` 消费的适配（行序 = 过滤后的可见行序）。
///
/// ⚠️ **一旦用 `CommandItem::child`，`.icon()` 与 `.label()` 都不再渲染**
/// （`command/state.rs:731-742`），所以前导图标 / 分支名 / 右侧「当前」徽章**全部画在 child 里**；
/// `.label()` 仍然要设 —— 它是搜索文本（研究 §6.3 的注意）。
trait IntoBranchItem {
    fn into_branch_item(self) -> CommandItem;
}

impl IntoBranchItem for BranchInfo {
    fn into_branch_item(self) -> CommandItem {
        let name = self.name.clone();
        let search = self.name.clone();
        let is_current = self.is_current;

        CommandItem::new()
            .label(search)
            .child(move |_window, cx| {
                let theme = cx.theme();
                // 真源：当前分支文字 `text-foreground`，其它 `text-subtle-foreground`
                // （`git-branch-manager.tsx:903-906`）；gpui-kit 没有 `subtle` token，
                // 语义最近的是 `muted_foreground`（与 `crate::project_menu` 同一取值）。
                let name_color = if is_current {
                    theme.foreground
                } else {
                    theme.muted_foreground
                };
                // 前导槽固定 20×20（真源 `CommandItemIcon` = `size-5`，`ui/command.tsx:547-560`），
                // 里面是 14 的图标（真源 `size-3.5`）。
                let leading = h_flex()
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .size_5()
                    .child(match is_current {
                        // 当前分支：绿勾（真源 `CheckIcon size-3.5 text-success`，`:892-893`）。
                        true => Icon::new(IconName::Check)
                            .size_3p5()
                            .text_color(theme.success),
                        // 其它分支：分支图标（真源 `GitBranchIcon text-subtle-foreground`，`:894-896`）。
                        false => Icon::new(IconName::GitBranch)
                            .size_3p5()
                            .text_color(theme.muted_foreground),
                    });

                let mut row = h_flex()
                    .w_full()
                    .min_w_0()
                    // ⚠️ `min_h_9()`(36) **不能省**：行高由自绘内容决定（见 [`BRANCH_ROW_HEIGHT_SPEC`]）。
                    .min_h(rems(BRANCH_ROW_HEIGHT_SPEC / 16.))
                    .gap_2p5()
                    .child(leading)
                    .child(
                        // 分支名 `min-w-0 truncate` + 宽度上限（真源的 `maxWidth` 夹取，
                        // 见 [`BRANCH_NAME_MAX_WIDTH_SPEC`]）。
                        div()
                            .min_w_0()
                            .max_w(rems(BRANCH_NAME_MAX_WIDTH_SPEC / 16.))
                            .truncate()
                            .text_sm()
                            .text_color(name_color)
                            .child(name.clone()),
                    );

                // 右侧「当前」徽章：真源 `CommandItemBadge variant="success"`，文案 `git.current`
                // （`git-branch-manager.tsx:907-909`；`ui/command.tsx:564-570`；`locale.ts:6948`）。
                // 真源还有一颗只属于**非当前**分支的「…」按钮（`:910-932`），它要
                // `merge` / `rebase` / `deleteBranch` 三条写命令（研究 §4.3-4.4）—— 本侧没有，
                // 所以整颗不画（诊断行里登记为 `state=absent`）。
                if is_current {
                    row = row.child(
                        div()
                            .ml_auto()
                            .flex_shrink_0()
                            .px_1p5()
                            .rounded_full()
                            .bg(theme.success.opacity(0.1))
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.success)
                            .child(tr("lithe.git.current")),
                    );
                }

                row.into_any_element()
            })
    }
}

// ---------------------------------------------------------------------------
// 诊断（`S1_BRANCH_PANEL_*`）
// ---------------------------------------------------------------------------

/// 开合状态那一行诊断。`opened=false` 在构造期打（启动证据），`opened=true` 在面板
/// **真的被画出来**的那一帧打 —— 后者才是"面板出现"的机器证据，而不是"状态位被置了"。
///
/// 格式照任务口径：`S1_BRANCH_PANEL opened=true branches=N current=main`。
fn diagnose(opened: bool, branches: usize, current: Option<&str>) {
    eprintln!(
        "S1_BRANCH_PANEL opened={opened} branches={branches} current={}",
        current.unwrap_or("—")
    );
}

/// 结构那行诊断：把"只能从像素里量出来"的事实写成可 grep 的契约（实现值，不是量出来的像素）。
fn diagnose_structure() {
    eprintln!(
        "S1_BRANCH_PANEL panel_w={} max_h={} top={} tabs={} tab={} search_h={} row_h={} \
         badge_max_w={} name_max_w={} refresh=true new_branch=absent rows_menu=absent",
        PANEL_WIDTH_SPEC as i32,
        PANEL_MAX_HEIGHT_SPEC as i32,
        PANEL_TOP_INSET_SPEC as i32,
        PanelTab::ALL.len(),
        PanelTab::ACTIVE.id(),
        SEARCH_ROW_HEIGHT_SPEC as i32,
        BRANCH_ROW_HEIGHT_SPEC as i32,
        BADGE_MAX_WIDTH_SPEC as i32,
        BRANCH_NAME_MAX_WIDTH_SPEC as i32,
    );
}

/// 三个页签各自的状态：`分支` 可点（重读列表），另两个**禁用**，各自带前置条件。
fn diagnose_tabs() {
    for tab in PanelTab::ALL {
        eprintln!(
            "S1_BRANCH_PANEL tab={} state={} precondition={}",
            tab.id(),
            match tab {
                PanelTab::Branches => "active",
                _ => "disabled",
            },
            tab.precondition()
        );
    }
}

/// 读命令的失败诊断。**不静默**：空列表有两种原因（真的没有分支 / 读失败），
/// 这一行把它们分开（真源把两者都吞成空列表，研究 §4.5/§4.6 登记为缺陷）。
fn diagnose_failures(failures: &[String]) {
    if failures.is_empty() {
        return;
    }
    eprintln!(
        "S1_BRANCH_PANEL failures={} detail={}",
        failures.len(),
        failures.join("+")
    );
}

// ---------------------------------------------------------------------------
// 状态
// ---------------------------------------------------------------------------

/// 标题栏分支项 + 分支弹窗的状态。
///
/// ⚠️ **为什么是 `Entity` 而不是无状态渲染函数**：面板的开关与**数据**都要跨帧存在，而且验证
/// 需要一条"启动态就把面板画出来"的通路（[`BranchPanel::open_by_probe`]）—— 那要求开合状态
/// 有一个**可以从外面拿到句柄去改**的落点。理由与 [`crate::project_menu::ProjectMenu`] 完全相同。
///
/// `root` 是工作区根（每次刷新/打开都从它现读，所以换工作区之后不必改本结构体）；
/// `branches` 是标题栏那一项与列表共用的**唯一数据来源**（打开、刷新都写它）。
pub struct BranchPanel {
    /// 工作区根（`git.status` 会在它下面解析出真正的仓库根）。
    root: std::path::PathBuf,
    /// 面板是否展开（点击触发器与诊断入口写的是**同一个**字段）。
    open: bool,
    /// `Command` 的交互状态（列表虚拟化 / 高亮 / 滚动位置）。**跨帧持有**，否则每帧重置。
    command: Entity<CommandState>,
    /// 自绘的搜索框（真源 `git.searchBranches` = 搜索分支...）。
    search: Entity<InputState>,
    /// 搜索框上一次的值：`InputEvent::Change` 只改它，面板 `render` 时按它过滤
    /// （见模块头的"过滤自己做"）。
    query: SharedString,
    /// 最近一次读到的分支数据（标题栏那一项 + 列表）。
    ///
    /// ⚠️ **只有读命令的产出入这里**，界面状态一概不进：`open` 变了不该重读 Git，
    /// 重读 Git 也不该重置搜索词。
    branches: BranchSnapshot,
    /// `InputEvent::Change` 的订阅（`Subscription` 一 drop 就取消，所以必须持有）。
    _search_subscription: Option<Subscription>,
}

impl BranchPanel {
    /// 建状态、**立刻读一次数据**，并在构造期打一行 `opened=false` 的诊断（启动证据）。
    ///
    /// 构造期就读是刻意的：标题栏那一项（分支名 + ahead/behind）在第一帧就要画对，
    /// 而 `git.status` + `git.references` 都是本地命令（`BranchSnapshot::load` 的文档）。
    /// 读不到（非仓库 / 命令失败）时快照是空的，标题栏整项不渲染 —— 真源同样是整项不渲染
    /// （研究 §5 第 1 行），所以这里不需要 loading 态。
    pub fn new(
        root: std::path::PathBuf,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let branches = BranchSnapshot::load(&root);
        diagnose(false, branches.branches.len(), branches.branch.as_deref());
        diagnose_failures(&branches.failures);

        let search = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr("lithe.git.searchBranches"))
        });
        cx.new(|cx| {
            let command = cx.new(|cx| CommandState::new(window, cx));
            let subscription = cx.subscribe_in(
                &search,
                window,
                |panel: &mut Self, state: &Entity<InputState>, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        panel.query = state.read(cx).value();
                        cx.notify();
                    }
                },
            );
            Self {
                root,
                open: false,
                command,
                search,
                query: SharedString::default(),
                branches,
                _search_subscription: Some(subscription),
            }
        })
    }

    /// 面板是否展开。
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// 标题栏那一项 / 列表要显示的数据（唯一来源）。
    pub(crate) fn branches(&self) -> &BranchSnapshot {
        &self.branches
    }

    /// 重读数据并把面板打开（`Esc` / 点遮罩收起来之后还能再开）。
    ///
    /// 真源在打开时才读（`git-branch-manager.tsx:526-531`），本侧同一条：面板不订阅任何
    /// 变更事件（研究 §3.3），所以"打开"就是唯一的刷新时机之一。
    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.branches = BranchSnapshot::load(&self.root);
        self.open = true;
        // 三行诊断都在这里打（每开一次一组）：`opened=true` 是"面板出现"的证据，
        // 结构行与页签行是"画成了什么"的可 grep 契约。**不打在 render 里** ——
        // 面板每帧都会重画（动画 / 悬停），打在那里会刷屏。
        diagnose(true, self.branches.branches.len(), self.branches.branch.as_deref());
        diagnose_structure();
        diagnose_tabs();
        diagnose_failures(&self.branches.failures);
        // 关闭时搜索词被清空（真源 `git-branch-manager.tsx:270-274`）；
        // 打开时也复位一次，免得上一轮的词留在框里而列表是全量。
        self.query = SharedString::default();
        self.search
            .update(cx, |state, cx| state.set_value("", window, cx));
        cx.notify();
    }

    /// 收起面板。焦点由 `Dialog` 自己归还（`Esc` / 点遮罩 / Dialog 的关闭按钮都会走这里）。
    fn close(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        cx.notify();
    }

    /// 点底栏「刷新」：重读列表（真源 `git-branch-manager.tsx:800-807`）。
    ///
    /// ⚠️ 真源的「刷新」**不重读 `git.status`**（研究 §4.5 与 §7.2 第 6 条：标题栏的分支名 /
    /// ahead-behind 靠各自的 `onBranchChange` 更新）。本侧两份数据来自同一个快照，分开读反而
    /// 会让标题栏与列表打架，所以一次重读两条命令 —— 这是有意的偏离（更符合直觉，
    /// 研究 §7.2 第 6 条留的就是这个判断）。
    fn refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.branches = BranchSnapshot::load(&self.root);
        eprintln!(
            "S1_BRANCH_PANEL refresh=true branches={} current={}",
            self.branches.branches.len(),
            self.branches.branch.as_deref().unwrap_or("—")
        );
        diagnose_failures(&self.branches.failures);
        cx.notify();
    }

    /// 把焦点交给自绘的搜索框。
    ///
    /// **必须有这一步**：`window.open_dialog` 把焦点给了 `Dialog` 自己
    /// （`root.rs:309-310`），不显式要一次的话 `WM_CHAR` 会被丢掉 —— Windows 的字符输入走
    /// **已安装的输入处理器**（`gpui-pre-windows-0.3.6/src/events.rs:477-484` 的
    /// `with_input_handler`），没有焦点就没有输入处理器，搜索框看得见却打不进字
    /// （`crate::command_palette` 的 `PENDING_FOCUS` 记的是同一条）。
    fn focus_search(&self, window: &mut Window, cx: &mut App) {
        let focus = self.search.read(cx).focus_handle(cx);
        focus.focus(window, cx);
    }

    /// **验证/诊断入口**：把面板打开（`--branch-panel-probe`）。
    ///
    /// 这不是产品能力：它与"点一下触发器"落到**同一个** [`BranchPanel::open`]、
    /// 同一段渲染代码（`ShellWorkspace::open_branch_panel` 先调它再 `open_dialog`），
    /// 唯一被绕开的是"操作系统把这次点击送进窗口"那一段。
    pub fn open_by_probe(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        eprintln!("S1_BRANCH_PANEL_PROBE open=true");
        self.open(window, cx);
    }
}

impl Render for BranchPanel {
    /// ⚠️ 本面板**不是**由本 `Entity` 画出来的：面板是模态浮层，只能在事件回调 / 任务 /
    /// （首帧之后的）render 里 `window.open_dialog`，而 `Dialog` 的内容由
    /// [`crate::workspace::ShellWorkspace::render`] 构造。这个 `Render` 实现只是 gpui
    /// 允许 `Entity` 被 `cx.observe` 的最小要求，画一个零尺寸占位。
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

// ---------------------------------------------------------------------------
// 渲染
// ---------------------------------------------------------------------------

/// 搜索过滤：真源的 `matchesSearchQuery`（`utils/search-match.ts:1-35`）的 gpui 版。
///
/// 判定两条（**或**关系）：
///
/// 1. **归一化后包含**：两边都转小写、把所有非字母数字字符折成空格（真源用 NFKD + 去音标 +
///    `[^a-z0-9]` → 空格；本侧用 `char::is_alphanumeric`，等价于 `\p{L}\p{N}`），
///    再判 `contains`；
/// 2. **紧凑串包含**：把上面的空格全部去掉再判一次 —— 所以把分隔符打成空格的查询
///    （`feature log` vs `feature/login`）仍然命中（真源的同一招，`search-match.ts:26-35`）。
///
/// ⚠️ **它不做模糊匹配**：漏字（`featrecent` vs `feature/recent`）两边都只是 `contains`，
/// **不**命中。研究 §1.4 的例子说的是"紧凑串"那一条，别把它读成子序列匹配
/// （本轮单测实测纠正过两次这种想当然）。
///
/// ⚠️ **与真源的一处偏差（有意）**：不做 NFKD / 去音标（那需要 `unicode-normalization` 依赖）。
/// 分支名是 ASCII 路径片段，带音标的引用名极罕见；少这一层只会让"输入带音标的查询"匹配不到，
/// 不会误匹配。登记在研究 §1.4 的匹配算法那一栏之下（本文件模块头的偏差表未列它，
/// 因为它不影响任何界面文案与布局）。
fn normalize(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut last_was_space = true;
    for character in text.chars() {
        if character.is_alphanumeric() {
            normalized.extend(character.to_lowercase());
            last_was_space = false;
        } else if !last_was_space {
            normalized.push(' ');
            last_was_space = true;
        }
    }
    while normalized.ends_with(' ') {
        normalized.pop();
    }
    normalized
}

/// 一条分支是否命中查询词。
fn matches_query(name: &str, query: &str) -> bool {
    let query = normalize(query);
    if query.is_empty() {
        return true;
    }
    let name = normalize(name);
    let compact_query: String = query.chars().filter(|c| *c != ' ').collect();
    if name.contains(&query) {
        return true;
    }
    let compact_name: String = name.chars().filter(|c| *c != ' ').collect();
    compact_name.contains(&compact_query)
}

/// 过滤后的可见行（真源只看**分支名**一个字段，`git-branch-manager.tsx:65-76`）。
///
/// 两条路都用它：`Command` 的 items 与计数徽章之外的可见数（诊断）。纯函数，单测直接覆盖。
fn visible_rows(branches: &[BranchInfo], query: &str) -> Vec<BranchInfo> {
    branches
        .iter()
        .filter(|branch| matches_query(branch.name.as_ref(), query))
        .cloned()
        .collect()
}

/// 空态那两句话：有搜索词 → **没有匹配的分支**，否则 **未找到分支**
/// （真源 `git-branch-manager.tsx:694-698`；`locale.ts:6920,6934`）。
fn render_empty(query: &str, cx: &App) -> impl IntoElement {
    let key = match query.trim().is_empty() {
        true => "lithe.git.noBranchesFound",
        false => "lithe.git.noMatchingBranches",
    };
    div()
        .p_3()
        .w_full()
        .text_center()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(tr(key))
}

/// 读命令失败时的**错误条**（真源没有这一条：它只 `console.error`，见模块头）。
///
/// 位置在页签行与列表之间（`Command::header` 的下半段），所以"面板里有东西但列表是空的"
/// 与"这个仓库读不出来"能一眼分开。
fn render_error_bar(failures: &[String], cx: &App) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .mx_2()
        .mb_2()
        .px_2()
        .py_1p5()
        .rounded(rems(ROW_RADIUS_SPEC / 16.))
        .bg(cx.theme().danger.opacity(0.1))
        .text_xs()
        .text_color(cx.theme().danger)
        .child(SharedString::from(failures.join("；")))
}

/// 画一整块面板内容（`Dialog` 的 content）：自绘的「错误条 + 页签行 + 搜索行」，加上
/// `Command` 的「列表 + 底栏」。
///
/// ⚠️ **为什么搜索行与页签行都在 `Command` 之外**：`Command` 的渲染顺序是
/// header → **搜索框** → 列表 → footer（`command/state.rs:838-935`）。`.header()` 里的东西
/// 一律落在 `Command` 那个（本侧已关掉的）搜索框**之上**；而列表与底栏是 `Command` 自己的
/// 孩子，接不进 header。所以本面板的自绘三段只能整体放在 `Command` **之前**。
///
/// 结果是自上而下的顺序：
///
/// ```text
/// 错误条（仅失败时）→ 页签行 → 搜索行 → Command（列表 + 底栏）
/// ```
///
/// 与真源的 `搜索行 → 页签行 → 列表 → 底栏`（研究 §1.8 的骨架）差**一处**：搜索行与页签行的
/// 先后。这一处的取舍理由写在模块头的偏差表里；四段的**数量与内容**与真源一一对应。
///
/// ⚠️ 收 `&App` 而不是 `&mut App`：本 crate 的所有区域渲染函数都只读主题与实体状态
/// （`lib.rs` 模块头的"区域渲染函数必须收 `&Window` / `&App`"），而 `Command` 的动作全在
/// 回调里，渲染这一层不需要可变借用。
fn render_panel(panel: &Entity<BranchPanel>, cx: &App) -> AnyElement {
    let (command, query, search, snapshot) = {
        let panel = panel.read(cx);
        (
            panel.command.clone(),
            panel.query.clone(),
            panel.search.clone(),
            panel.branches.clone(),
        )
    };

    let rows = visible_rows(&snapshot.branches, query.as_ref());
    let total = snapshot.branches.len();
    let failures = snapshot.failures.clone();
    let empty_query = query.clone();
    let confirm_handle = panel.downgrade();

    let command_element = Command::new(&command)
        .bordered(false)
        // ⚠️ 搜索行由本模块自绘（见模块头），所以 `Command` 自带的搜索框关掉、
        // 本地过滤也关掉（`item_matches` 在 `!searchable` 时一律放行，
        // `state.rs:307-313`）—— 只喂**已经过滤好的**可见行。
        .searchable(false)
        .filterable(false)
        .w_full()
        // 真源面板最大高 512；`Command` 列表默认 18.75rem = 300（`command/command.rs:34`）。
        .max_h(DefiniteLength::from(rems(PANEL_MAX_HEIGHT_SPEC / 16.)))
        .items(rows.into_iter().map(IntoBranchItem::into_branch_item))
        .empty(move |_, _, cx| render_empty(empty_query.as_ref(), cx).into_any_element())
        .on_confirm(move |index, window, cx| {
            // v1 只读：列表行的语义是"切分支"，而 `git.checkoutPreflight` + `git.write{checkout}`
            // 本侧没有调用点，所以**不执行任何写操作**。这里显式记一行诊断（可 grep），
            // 并关掉面板 —— 真源点**当前**分支是不关面板的（`:282`），点别的分支才关（切完就关）；
            // 本侧两条都走"关面板"，因为两条都做不到切换，留着面板只会让人以为点漏了。
            eprintln!("S1_BRANCH_PANEL action=checkout row={} state=read_only", index.row);
            let _ = confirm_handle.update(cx, |panel, cx| panel.close(window, cx));
        })
        .footer({
            let panel = panel.clone();
            move |_, _, cx| render_footer(&panel, cx).into_any_element()
        });

    let mut container = v_flex().w_full();
    if !failures.is_empty() {
        container = container.child(render_error_bar(&failures, cx));
    }
    container
        .child(render_tabs(cx))
        .child(render_search(&search, total, cx))
        .child(command_element)
        .into_any_element()
}

/// 打开分支弹窗（**只能从事件回调 / 任务 / 首帧之后的 render 里调用**）。
///
/// 三件事，顺序不能换：
///
/// 1. 重读数据（研究 §3.3：面板**不订阅任何变更事件**，只在打开与手动刷新时读）；
/// 2. 置 `open` + 清搜索词（[`BranchPanel::open`]）；
/// 3. `window.open_dialog` 把浮层放上窗口（`Root` 的 dialog 层已由
///    `ShellWorkspace::render` 挂好，理由见 `crate::command_palette`）。
///
/// 已经开着别的对话框时不叠第二层 —— 与 `open_settings_dialog` / `open_command_palette`
/// 同一条口径，也免得 `Esc` 的"只关最上层"语义被两个面板搞乱。
pub(crate) fn open_branch_panel(
    panel: &Entity<BranchPanel>,
    window: &mut Window,
    cx: &mut App,
) {
    if window.has_active_dialog(cx) {
        return;
    }
    // 先重读数据 + 置位（`BranchPanel::open`），诊断行也由它打（含失败清单）。
    let _ = panel.update(cx, |panel, cx| panel.open(window, cx));
    // 焦点交给自绘的搜索框（理由见 [`BranchPanel::focus_search`]）。
    let _ = panel.update(cx, |panel, cx| panel.focus_search(window, cx));

    let handle = panel.clone();
    window.open_dialog(cx, move |dialog, window, _cx| {
        // `Dialog::width` / `Dialog::margin_top` 是**固有方法**、只吃 `Pixels`（它们遮蔽了
        // `Styled` 上的同名方法，而 gpui 没有 `impl From<Rems> for Pixels`），所以按**运行时**
        // rem 基准换算一次 —— 基准取自这一帧的窗口，不是写死的 16（见 [`crate::rem_px`]）。
        let rem = window.rem_size();
        dialog
            // 面板整个由 `Command` + 自绘三段构成：搜索行、圆角与边框都不要 Dialog 的。
            .close_button(false)
            .overlay(true)
            .overlay_closable(true)
            .keyboard(true)
            .width(crate::rem_px(rem, PANEL_WIDTH_SPEC))
            .margin_top(crate::rem_px(rem, PANEL_TOP_INSET_SPEC))
            .p_0()
            // 关闭的唯一收尾点：`Esc` / 点遮罩 / Dialog 的关闭按钮三条路都经过它
            // （`dialog/dialog.rs:598-607` 的 `on_close`），所以面板状态在这里回到 `false`，
            // 下次点触发器还能再打开。
            .on_close({
                let handle = handle.clone();
                move |_event, window, cx| {
                    let _ = handle.update(cx, |panel, cx| panel.close(window, cx));
                }
            })
            .content({
                let handle = handle.clone();
                move |content, _window, cx| {
                    content
                        .p_0()
                        .gap_0()
                        .child(render_panel(&handle, cx))
                }
            })
    });
}

// ---------------------------------------------------------------------------
// 标题栏那一项（触发器）
// ---------------------------------------------------------------------------

/// 标题栏分支项：**分支图标 + 分支名 + ahead/behind 箭头**，**没有 caret**。
///
/// 度量照研究 §1.2：`Button ghost xs`（`h-6 gap-1`）+ `px-2` = 高 **24**、gap **4**、
/// px **8**（`git-branch-manager.tsx:634-646`；`ui/button.tsx:23`）；图标 14（`:647`）；
/// 分支名 `truncate` + 宽度上限（`:189-190,648-653`）；ahead/behind 只有箭头（`:654-660`）。
///
/// `None` = **整项不渲染**（真源 `useFooterGitBranchItem` 在没有仓库 / 没有分支名时返回
/// `null`，`footer-git-branch-item.tsx:20-27`；`GitBranchManager` 自己也兜一道
/// `if (!currentBranch) return null`，`:604-606`）。调用方因此传一块空 `div` 占位。
///
/// ⚠️ **不画 `▾`**：真源的子节点只有三样（见模块头）。
///
/// 数据从 `panel` 里现取（[`BranchPanel::branches`]）：标题栏那一项与列表**同一个来源**，
/// 刷新之后两处一起变，不会出现"列表刷新了、标题栏还是旧 ahead/behind"。
pub(crate) fn trigger(panel: &Entity<BranchPanel>, cx: &App) -> Option<AnyElement> {
    let branch = panel.read(cx).branches().branch.clone()?;
    let tracking = panel.read(cx).branches().tracking;
    let theme = cx.theme();
    let idle = theme.muted_foreground;
    let hover_bg = theme.accent;
    let hover_fg = theme.foreground;
    let open_handle = panel.clone();
    let name = branch.clone();

    Some(
        div()
            .id("lithe-title-branch-trigger")
            .flex()
            .flex_shrink_0()
            .items_center()
            .h_6()
            .px_2()
            .gap_1()
            .max_w(rems(BRANCH_NAME_MAX_WIDTH_SPEC / 16.))
            .rounded(rems(TRIGGER_RADIUS_SPEC / 16.))
            .text_sm()
            .text_color(idle)
            // 真源触发器**只有 hover 态**（ghost），展开时不给底色（研究 §1.2 的外观一栏）。
            .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
            // 点触发器**不能**拖窗口：它本身不在 `Drag` 命中区里（是 `drag_region` 的兄弟，
            // 见模块头），这两行是与 `AppMenuBar` 相同的第二道保险（`app_menu_bar.rs:272-280`）。
            .on_mouse_down(MouseButton::Left, |_event, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .on_click(move |_event, window, cx| {
                if open_handle.read(cx).is_open() {
                    return;
                }
                // 打开才读（真源同样在打开时读，研究 §3.3：面板不订阅任何变更事件）。
                open_branch_panel(&open_handle, window, cx);
            })
            // 分支图标 14（真源 `GitBranchIcon` + `[&_svg]:size-3.5`，`git-branch-manager.tsx:647`）。
            .child(Icon::new(IconName::GitBranch).size_3p5())
            .child(
                // 分支名 `min-w-0 truncate`（真源 `:648-653`）。
                div()
                    .min_w_0()
                    .truncate()
                    .child(branch),
            )
            // ahead/behind：**只有箭头，没有数字**（真源 `showCounts={false}`，`:654-660`）。
            // 两个都为 0 时整段不画（`git-tracking-counts.tsx:33`）。
            // 颜色：behind → `theme.info`、ahead → `theme.success`（真源 `--info` 与 `--git-added`，
            // 与 `gpui/crates/git/src/log_view.rs` 的引用行同一组 token）。
            // 字号 10 不在 gpui 档位上 → `rems(10. / 16.)`。
            .children(render_tracking(tracking, cx))
            // 无障碍名：真源触发器的 `aria-label` 是 `git.searchBranchesAria` = 搜索分支
            // （`git-branch-manager.tsx:645`；`locale.ts:6917`）。
            .aria_label(SharedString::from(format!(
                "{} {}",
                tr("lithe.git.searchBranchesAria"),
                name
            )))
            .into_any_element(),
    )
}

/// ahead / behind 两个箭头（真源 `GitTrackingCounts showCounts={false}`）。
///
/// 上限 99 → `99+` 的逻辑在 `lithe_gpui_git` 里（`model::tracking_count`），本侧**不画数字**，
/// 所以这里只需要"谁大于 0"。顺序照真源：behind（`↙`）在前、ahead（`↗`）在后
/// （`git-tracking-counts.tsx:36-53`）。
fn render_tracking(tracking: lithe_gpui_git::TrackingCounts, cx: &App) -> Vec<AnyElement> {
    if tracking.is_empty() {
        return Vec::new();
    }
    let mut arrows = Vec::new();
    if tracking.behind > 0 {
        arrows.push(
            div()
                .flex_shrink_0()
                .text_size(rems(10. / 16.))
                .text_color(cx.theme().info)
                .child(SharedString::from("↙"))
                .into_any_element(),
        );
    }
    if tracking.ahead > 0 {
        arrows.push(
            div()
                .flex_shrink_0()
                .text_size(rems(10. / 16.))
                .text_color(cx.theme().success)
                .child(SharedString::from("↗"))
                .into_any_element(),
        );
    }
    arrows
}

/// 面板内一行的通用形状（给单测用：`h_flex` 的度量无法在无窗口环境断言）。
///
/// ⚠️ 这个函数**只给单测**提供"行高下限"的读数，渲染路径不经过它
/// （渲染用 `CommandItem::child` 里的 `h_flex`）。写成函数而不是常量断言，
/// 是为了让"36 这个下限真的被用上了"有一处可读的落点；收 `rem` 是为了让单测能把
/// **运行时基准**喂进来（见 [`crate::rem_px`] 与 `branch_row_height_scales_with_the_rem_base`）。
// `Pixels` 只在这条 `#[cfg(test)]` 函数签名里出现，所以它的 `use` 也按 `cfg(test)` 收窄：
// 无条件导入会让非测试构建报 `unused_imports`（`cargo check` 必须零 warning）。
#[cfg(test)]
use gpui_kit::Pixels;

#[cfg(test)]
fn branch_row_height(rem: Pixels) -> Pixels {
    crate::rem_px(rem, BRANCH_ROW_HEIGHT_SPEC)
}

/// 触发器上**没有** caret：这条测试把"不画 `▾`"变成可执行的契约。
///
/// 真源的子节点只有图标 / 分支名 / ahead-behind（`git-branch-manager.tsx:646-661`），
/// 维护者截图里的 `▾` 属于左邻的项目下拉（研究 §7.2 第 1 条）。实现里一旦有人补一个
/// `IconName::ChevronDown`，[`trigger`] 的元素树就会多一个图标 —— 那条路没有便宜的断言，
/// 所以这里退一步只钉住"分支项用到的图标集合里没有 caret"。
#[cfg(test)]
fn trigger_icon_names() -> [IconName; 1] {
    // 触发器的图标只有分支图标一个（ahead/behind 是文字箭头，不是图标）。
    [IconName::GitBranch]
}

#[cfg(test)]
mod tests {
    use super::{
        BRANCH_ROW_HEIGHT_SPEC, IconName, PANEL_MAX_HEIGHT_SPEC, PANEL_TOP_INSET_SPEC,
        PANEL_WIDTH_SPEC, PanelTab, SEARCH_ROW_HEIGHT_SPEC, branch_row_height, matches_query,
        trigger_icon_names, visible_rows,
    };
    use gpui_kit::{SharedString, px};
    use lithe_gpui_git::BranchInfo;

    fn branch(name: &str, is_current: bool) -> BranchInfo {
        BranchInfo {
            name: SharedString::from(name),
            is_current,
            upstream: None,
        }
    }

    /// 面板度量：704 × 512、距顶 64（真源 `ui/command.tsx:29,143`，研究 §1.8）。
    ///
    /// 这三个值是验收要**按像素量**的，所以钉住规格常量本身。
    #[test]
    fn panel_geometry_matches_the_source() {
        assert_eq!(PANEL_WIDTH_SPEC, 704.);
        assert_eq!(PANEL_MAX_HEIGHT_SPEC, 512.);
        assert_eq!(PANEL_TOP_INSET_SPEC, 64.);
        assert_eq!(SEARCH_ROW_HEIGHT_SPEC, 28.);
        assert_eq!(BRANCH_ROW_HEIGHT_SPEC, 36.);
    }

    /// 行高下限真的经 [`crate::rem_px`] 求值成 36（125% DPI 下是 45 物理），
    /// 并且**随运行时 rem 基准缩放** —— 只断言 16px 基准下等于 36 证明不了它不是假 rem
    /// （写死 `to_pixels(px(16.))` 也能过），所以这里再加一条"基准翻倍 ⇒ 行高翻倍"。
    #[test]
    fn branch_row_height_scales_with_the_rem_base() {
        assert_eq!(f32::from(branch_row_height(px(16.))), 36.);
        // 基准 32px（界面字号翻倍）⇒ 36 × 2 = 72，而不是仍然是 36。
        assert_eq!(f32::from(branch_row_height(px(32.))), 72.);
    }

    /// 页签顺序照真源的数组字面量：`仓库` → `分支` → `工作树`，默认选中**分支**
    /// （`git-branch-manager.tsx:608-630,158,528`）。
    #[test]
    fn tabs_are_ordered_and_branches_is_active() {
        // 渲染顺序是 `PanelTab::ALL`，真源数组顺序是 repositories → branches → worktrees。
        assert_eq!(PanelTab::ALL.len(), 3);
        assert_eq!(PanelTab::ACTIVE, PanelTab::Branches);
        let ids: Vec<&str> = PanelTab::ALL.iter().map(|tab| tab.id()).collect();
        assert_eq!(ids, vec!["branches", "repositories", "worktrees"]);
        // 另两个页签的前置条件必须各说各的（v1 一条都做不到），免得诊断行说不清缺什么。
        let mut preconditions: Vec<&str> = PanelTab::ALL
            .iter()
            .map(|tab| tab.precondition())
            .collect();
        preconditions.sort_unstable();
        preconditions.dedup();
        assert_eq!(preconditions.len(), 3);
    }

    /// 触发器没有 `▾`：图标集合里只有分支图标（研究 §7.2 第 1 条）。
    #[test]
    fn trigger_has_no_chevron() {
        let icons = trigger_icon_names();
        assert_eq!(icons, [IconName::GitBranch]);
        assert!(!icons.contains(&IconName::ChevronDown));
        assert!(!icons.contains(&IconName::ChevronUp));
    }

    /// 搜索匹配：照真源 `matchesSearchQuery` 的两条判定（归一化包含 + 紧凑串包含）。
    #[test]
    fn search_matches_like_the_source() {
        // 空查询匹配一切。
        assert!(matches_query("feature/login", ""));
        // 大小写不敏感的子串。
        assert!(matches_query("feature/login", "LOGIN"));
        assert!(matches_query("feature/login", "feature"));
        // 分隔符两边都折成空格，所以**空格与分隔符等价**：
        // `fix order` 命中 `fix/order-list`、`fix-order` 也命中。
        assert!(matches_query("fix/order-list", "fix order"));
        assert!(matches_query("fix/order-list", "order list"));
        // 紧凑串那一条（真源 `search-match.ts:26-35`）：查询里的空格被去掉之后再比一次，
        // 所以 `feature log`（用户把分隔符打成空格）也能命中 `feature/login`。
        assert!(matches_query("feature/login", "feature log"));
        // 不匹配的不能被放过。
        assert!(!matches_query("main", "dev"));
        assert!(!matches_query("feature/login", "zzz"));
        // ⚠️ 反例（本轮实测纠正了一条想当然）：**漏字不在这个算法的能力范围内** ——
        // `featrecent`（少一个 `u`）**不**命中 `feature/recent`（两边都只是 `contains`，
        // 没有模糊 / 子序列匹配）。真源的算法同样如此，写在这里免得后人照着错例子改。
        assert!(!matches_query("feature/recent", "featrecent"));
    }

    /// 过滤只作用于**分支名**一个字段，且保持原有顺序（真源 `git-branch-manager.tsx:65-76`）。
    #[test]
    fn filtering_keeps_order_and_only_reads_the_name() {
        let branches = vec![
            branch("main", true),
            branch("feature/login", false),
            branch("fix/order-list", false),
        ];
        let all = visible_rows(&branches, "");
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].name.as_ref(), "main");

        let filtered = visible_rows(&branches, "fix");
        let names: Vec<&str> = filtered.iter().map(|b| b.name.as_ref()).collect();
        assert_eq!(names, vec!["fix/order-list"]);

        // 当前分支的那一行被过滤掉之后，`visible_rows` 不再包含它（计数徽章用的是**总数**，
        // 所以两者会不同 —— 真源同样如此，`:679`）。
        let none = visible_rows(&branches, "zzz");
        assert!(none.is_empty());
    }
}
