//! 底部工具窗「提交记录」（Windows 的 `GitLogToolWindow`）。
//!
//! 规格真源：`windows/tauri/src/features/git/components/log/*`（Windows 前端 = 视觉与布局唯一真源），
//! 度量速查表 `gpui/UI-MAP-WINDOWS.md` §1.5 与 §2④，调研原文
//! `gpui/research/windows/03-git-and-bottom.md`。挂钩位置见该文 §1.1：默认
//! `terminalWidthMode === "editor"`（`features/terminal/stores/terminal.store.ts:29`）时底部窗
//! **嵌在中央编辑器列内**，是本面板的下一个 flex 兄弟（`features/layout/components/main-layout.tsx:318-322`），
//! 横向**不**跨整个工作台，且**没有自己的标签条**（页签由活动栏 / 命令面板切换单值
//! `bottomPaneActiveTab`）。本文件只负责画面板内容：外框圆角 / 边框 / 可拖高由父级（中央列的竖向分栏）负责。
//!
//! ## 元素 → 出处
//!
//! | 元素 | 出处（`windows/tauri/src/`） | 度量（px） |
//! |---|---|---|
//! | 标题栏容器 / 行 | `features/git/components/log/git-log-title-bar.tsx:28-29` | 高 32、pad 8、gap 8、13px、bg surface、下边框 1 |
//! | 标题分支图标 | 同上 `:30` | 14，`text-subtle-foreground` |
//! | 标题文案 `提交记录` | 同上 `:31`；文案 `i18n/locale.ts:5910` | 13 medium |
//! | 「引用名」胶囊 | 同上 `:32-39` | 高 24、max-w 240、圆角 6.4、px 8、border-strong/60、bg background |
//! | 图标按钮 ×3（刷新 / 设置 / 最小化） | 同上 `:40-71`、`ui/button.tsx:9,27` | 24×24，图标 14 |
//! | 右侧 `只读` | 同上 `:61`；文案 `locale.ts:7309` | `text-subtle-foreground` |
//! | 页签行 | `features/git/components/log/git-log-tool-window.tsx:582-587` | 高 24（py 4 + 行高 16）、gap 16、px 12、**12px** |
//! | 刷新失败横幅 | 同上 `:589-600` | 高 28、pad 8、gap 8、bg destructive/10、下边框、13px |
//! | 无仓库 / 加载中 / 失败占位 | 同上 `:602-612` | 居中；文案 `locale.ts:7198-7200` |
//! | 三栏比例 `19 / 57 / 24` | `features/git/stores/git-log-preferences.store.ts:43-47` | — |
//! | 三栏最小宽 `140 / 320 / 220` | `git-log-tool-window.tsx:622,648,684` | — |
//! | 引用栏左侧竖排工具栏 | `git-reference-tree.tsx:357,371`；按钮 `:146` | 宽 36、按钮 32×32、图标 16、圆角 4.8 |
//! | 引用栏列表头 / 滚动区 | `git-reference-tree.tsx:840,847` | 高 32、px 8 / p 6 |
//! | HEAD 行 | `git-reference-tree.tsx:853-864` | 高 28、mb 4、圆角 6.4、px 8、gap 8、分支名 max-w 96 |
//! | 分区头 / 引用行 | `git-reference-tree.tsx:876,599` | 高 24、gap 6、圆角 6.4 px 6 / 圆角 4.8 |
//! | 引用行缩进 | `git-reference-tree.tsx:594,603` | `10 + depth × 14` |
//! | disclosure / 占位 / 引用图标 | `git-reference-tree.tsx:611,618,630` | 14 |
//! | 「当前」徽章 | `git-reference-tree.tsx:652` | 10、圆角 6.4、px 4 |
//! | ahead / behind 计数 | `features/git/components/git-tracking-counts.tsx:3,36-53` | 10、gap 4、上限 99 → `99+` |
//! | 提交表工具行（筛选条） | `git-commit-table.tsx:200` | 高 32、gap 8、px 8 |
//! | 筛选输入框 | `git-commit-table.tsx:201-227` | 高 24、min-w 144、max-w 288、gap 6、圆角 6.4、px 8、图标 14 |
//! | 字段下拉 | `git-commit-table.tsx:229-238` | 高 24、圆角 6.4、px 6 |
//! | 装饰开关 | `git-commit-table.tsx:239-249` | 24×24，图标 14 |
//! | 计数 `可见/总数` | `git-commit-table.tsx:250-252` | `tabular-nums` |
//! | 表头行 `提交 / 作者 / 日期` | `git-commit-table.tsx:255-259` | 高 24、px 8、作者 112、日期 128 右对齐 |
//! | 提交行 | `git-commit-table.tsx:301-326` | 高 **30**、下边框 1、px 4、作者 112 px 8、日期 128 右对齐 11 等宽 |
//! | 提交行选中 / 悬停 | `git-commit-table.tsx:302-303` | 选中 `primary/22`（悬停 `/28`）、否则 `accent/70` |
//! | 提交行内容最小宽 | `git-commit-table.tsx:276` | 520（`min-w-130`） |
//! | 「加载更多提交」行 | `git-commit-table.tsx:428-443` | 高 36、min-w 520、按钮高 24 |
//! | 泳道图常量 | `features/git/components/log/git-graph-row.tsx:4-6,27,69-78` | 行高 30、`LANE_GAP 13`、`GRAPH_PADDING 8`、svg 宽 `max(30, laneCount×13+16)`、线宽 1.6、节点 r 4.3 描边 2 |
//! | 标签徽章 | `git-graph-row.tsx:87-88` | max-w 112、10px、px 6 py 2、圆角 6.4 |
//! | 标签四档配色 | `git-graph-row.tsx:16-22` | head sky / remote indigo / tag amber / branch emerald |
//! | Inspector 文件区 | `git-commit-inspector.tsx:104-130` | 62%、min 90；表头 32、px 8、gap 8 |
//! | Inspector 空/加载/失败态 | `git-commit-inspector.tsx:131-146` | 文案 `locale.ts:7298-7301` |
//! | Inspector 详情区 | `git-commit-inspector.tsx:164-185` | 38%、min 80；p 12、项间距 8；11 等宽、完整哈希 10 |
//! | 提交文件树 | `features/git/components/log/git-commit-file-tree.tsx:181-184,127` | p 6、行高 `max(24, 13×1.35+6)=24`、缩进 10+14·depth、状态字 10 等宽 |
//! | 控制台 | `git-execution-console.tsx:68-111`、`git-console-entry.tsx:50` | 12 等宽、左栏 32、按钮 24×24、输出区 p 12 |
//!
//! 文案一律逐字取自 `i18n/locale.ts`（中文资源段），行号写在每处字符串旁。
//!
//! ## 数据：Core 命令（`rust/lithe-core/src/`）
//!
//! | 用途 | 命令 | payload | 响应 |
//! |---|---|---|---|
//! | 工作区探测（一级目录，用来兜底找仓库） | `workspace.snapshot` | `{ root }`（`project/files.rs:39-45`，另两个字段有 `#[serde(default)]`） | `{ root: WorkspaceNode, files[] }`（`protocol/contracts.rs:72-75`）；节点 `path` 是**工作区相对路径**（`project/files.rs:576`） |
//! | 仓库根 / 当前分支 | `git.status` | `{ root }`（`git/mod.rs:94-96`） | `{ repositoryRoot, branch, ahead, behind, changes[] }`（`protocol/contracts.rs:532-538`）；非仓库是 `ok:true` + `repositoryRoot: null`（`git/mod.rs:6535-6544`） |
//! | 引用树 | `git.references` | `{ root }`（`git/history.rs:49-51`） | `{ references[], recentReferences[], userName, userEmail }`（`contracts.rs:629-635`；条目 `:583-597`） |
//! | 提交分页 | `git.historyPage` | `{ root, reference, order, cursor, limit }`（`git/history.rs:56-71`） | `{ commits[], nextCursor, hasMore }`（`contracts.rs:640-649`；条目 `:602-611`） |
//! | 释放分页游标 | `git.historyCursorClose` | `{ root, cursor }`（`git/history.rs:96-99`） | `{ closed }`（`contracts.rs:654-657`） |
//! | 提交文件 | `git.commitFiles` | `{ root, commit }`（`git/mod.rs:650-653`） | `{ files[] { status, path } }`（`contracts.rs:703-714`） |
//!
//! 三个硬约束（都在真源里核对过）：
//!
//! 1. **`git.status.repositoryRoot` 可能是相对工作区根的路径**（`relative_or_absolute`，`git/mod.rs:6574`）：
//!    直接当 `root` 用会按**进程 CWD** 解析，必须先与工作区根拼成绝对路径（[`resolve_repository_root`]）。
//! 2. **「非仓库」没有专用错误码**：`git.status` 返回 `ok:true` + `repositoryRoot: null`，其余 `git.*`
//!    返回 `ok:false` + `process_failed`。所以**先探 `git.status`**，不是仓库就不再白跑后面三条。
//! 3. **`nextCursor` 背后是一个活着的 `git log` 子进程**（`git/history.rs:242-257` 把 session 存回注册表；
//!    `:25-29` 空闲 120 s 才回收、每根最多 8 条）。本面板实现了「加载更多提交」，所以**持有**游标，
//!    并在三处归还：换引用 / 刷新时（交给后台任务先关）、面板 `Drop` 时。
//!
//! 调用形态照同目录既有实现：拼 `{id, operationId, timeoutMilliseconds, command, payload}` →
//! `lithe_core::execute_json` → 判 `ok` 取 `data`（[`execute_core`]），整段放 `cx.background_spawn`。
//!
//! ## 与 Windows 源码的刻意偏差（都写在这里，不藏在代码里）
//!
//! 1. **三栏用 flex 百分比而不是 `h_resizable`**：Windows 的 `19 / 57 / 24` 是**比例**，
//!    而 `ResizablePanel::size` 只吃 `Pixels`，且首帧之后会被 `ResizableState` 钉死
//!    （`gpui-base-0.6.6/src/resizable/panel.rs:350-353`、`mod.rs:200-218`），
//!    想在首帧按容器宽换算成像素必须先在 `render` 外测量再 `reset_panel` 重排，成本与风险都高。
//!    这里用 `.w(relative(0.19))` / `.w(relative(0.57))` / `.w(relative(0.24))` + `min_w` 精确复现比例，
//!    **代价是栏间不可拖拽**（Windows 的 `ResizableHandle` 没做）。
//! 2. **泳道图是简化版**：`git-graph-row.tsx` 用 SVG 画贝塞尔曲线 + 虚线；gpui 侧没有可用的 SVG 路径元素，
//!    这里用「每泳道一根 1.6px 竖线 + 节点圆圈」表达，**跨泳道的父边画成目标泳道的竖直段**（没有弧度），
//!    缺失父提交用 `opacity(0.7)` 代替 `strokeDasharray="3 2"`。线性历史（单泳道）与真机一致。
//! 3. **颜色映射到主题 token**：Windows 的泳道 6 色与标签 4 档（sky/indigo/amber/emerald）是裸色值，
//!    本仓库要求「颜色一律 `cx.theme()`」：泳道 6 色→`success / blue_light / magenta_light / warning /
//!    danger / cyan_light`；标签 4 档→head `cyan_light`、remote `blue_light`、tag `yellow_light`、
//!    branch `green_light`。
//! 4. **页签选中态加了前景色区分**：Windows 只用 `aria-selected`、**没有任何选中视觉**
//!    （`git-log-tool-window.tsx:582-587`），照搬会让「当前在哪个页签」不可见。这里保持
//!    「无底色 / 无下划线」，只把选中项文字用 `foreground`、未选中用 `muted_foreground`。
//! 5. **图标按钮自绘**，不用 `Button`：`Button::ghost().with_size(px(24.))` 的图标会被算成
//!    `24 × 0.75 = 18px`（`gpui-component-0.6.6/src/button/button.rs:580-583`），而规格是 14。
//!    自绘 div 拿到 24×24 命中区 + 14px 图标 + `accent` 悬停；代价是**没有悬停 tooltip**
//!    （改用 `aria_label`，与 Windows 的 `aria-label` 同源）。
//! 6. **控制台只有外壳**：Windows 的输出来自 Git 执行事件通道 + `git.consolePresentation`
//!    （`git-execution-console.tsx:31-36`），不在本步拍板的 6 条命令里 → 左栏按钮全部 disabled、
//!    正文只画空态文案（`locale.ts:4560`）。
//! 7. **`git.historyPage` 的提交没有正文**：Core 的格式串是 `%s`（只有主题行，`git/history.rs:302`），
//!    所以 Inspector 里 Windows 的 `commit.description` 那一段没有数据源，整段不画。
//! 8. **引用栏工具栏只保留纯 UI 的三个动作**：Windows 的 10 个动作里其余都要
//!    `git.write` / 远程管理（`git-reference-tree.tsx:209-311`），不在 6 条命令里。
//! 9. 未做：多选（Windows 的 `Set<string>` + Ctrl/Shift 区间）、行右键菜单（14 项写操作）、
//!    双击打开提交差异（要 `git.diff`）、引用树右键动作菜单、提交文件目录折叠、
//!    `git.consolePresentation` 的折叠与查找、栏宽持久化（`git-log-preferences`）。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Div, Entity, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, WeakEntity, Window, div, px,
    relative,
};

// ---------------------------------------------------------------------------
// 度量（出处见模块文档的表格，行号一一对应）
// ---------------------------------------------------------------------------

/// 标题栏行高。出处：`git-log-title-bar.tsx:29`（`h-8`）。
const TITLE_BAR_HEIGHT: f32 = 32.;
const TITLE_BAR_PADDING_X: f32 = 8.;
const TITLE_BAR_GAP: f32 = 8.;
/// 标题栏图标 / 文案字号。出处：`:30`（`size-3.5`）、`:31`（`ui-text-sm` = 13）。
const TITLE_ICON_SIZE: f32 = 14.;
const TITLE_FONT_SIZE: f32 = 13.;
/// 「引用名」胶囊。出处：`:32-39`（`h-6 max-w-60 rounded px-2 border-border-strong/60`）。
const REFERENCE_PILL_HEIGHT: f32 = 24.;
const REFERENCE_PILL_MAX_WIDTH: f32 = 240.;
const REFERENCE_PILL_RADIUS: f32 = 6.4;
const REFERENCE_PILL_PADDING_X: f32 = 8.;
/// 图标按钮 24×24 / 图标 14。出处：`git-log-title-bar.tsx:40-71`、`ui/button.tsx:9,27`。
const ICON_BUTTON_SIZE: f32 = 24.;
const ICON_BUTTON_RADIUS: f32 = 6.4;
const ICON_BUTTON_ICON_SIZE: f32 = 14.;

/// 页签行：行高 16 + 上下 4 = 24，p x 12、gap 16、字号 12。出处：`git-log-tool-window.tsx:582`。
const TAB_ROW_HEIGHT: f32 = 24.;
const TAB_ROW_PADDING_X: f32 = 12.;
const TAB_ROW_GAP: f32 = 16.;
const TAB_ROW_FONT_SIZE: f32 = 12.;

/// 刷新失败横幅。出处：`git-log-tool-window.tsx:590`（`h-7 px-2`、13px）。
const BANNER_HEIGHT: f32 = 28.;
const BANNER_PADDING_X: f32 = 8.;
const BANNER_GAP: f32 = 8.;

/// 筛选条工具行。出处：`git-commit-table.tsx:200`（`h-8 gap-2 px-2`）。
const FILTER_ROW_HEIGHT: f32 = 32.;
const FILTER_ROW_PADDING_X: f32 = 8.;
const FILTER_ROW_GAP: f32 = 8.;
/// 筛选输入框。出处：`:201`（`h-6 min-w-36 max-w-72 gap-1.5 px-2`）。
const FILTER_INPUT_MIN_WIDTH: f32 = 144.;
const FILTER_INPUT_MAX_WIDTH: f32 = 288.;
/// 字段下拉。出处：`:232`（`h-6 rounded px-1.5`）。
const FIELD_SELECT_PADDING_X: f32 = 6.;
/// 提交表字号。出处：`git-commit-table.tsx:199`（`ui-text-sm` = 13）。
const COMMIT_FONT_SIZE: f32 = 13.;
/// 表头行。出处：`:255`（`h-6 px-2`）。
const COMMIT_HEADER_HEIGHT: f32 = 24.;
/// 提交行高。出处：`:43`（`const ROW_HEIGHT = 30`）。
const COMMIT_ROW_HEIGHT: f32 = 30.;
/// 提交行左右内边距。出处：`:302`（`px-1`）。
const COMMIT_ROW_PADDING_X: f32 = 4.;
/// 作者列 112 / 内边距 8。出处：`:321`（`w-28 px-2`）。
const AUTHOR_COLUMN_WIDTH: f32 = 112.;
const AUTHOR_COLUMN_PADDING_X: f32 = 8.;
/// 日期列 128、字号 11、右对齐、等宽。出处：`:324`。
const DATE_COLUMN_WIDTH: f32 = 128.;
const DATE_FONT_SIZE: f32 = 11.;
/// 提交行内容最小宽。出处：`:276`（`min-w-130`）。
const COMMIT_CONTENT_MIN_WIDTH: f32 = 520.;
/// 「加载更多提交」行。出处：`:431`（`h-9 min-w-130`）、`:434`（按钮 `size="xs"` = 24）。
const LOAD_MORE_HEIGHT: f32 = 36.;
const LOAD_MORE_BUTTON_HEIGHT: f32 = 24.;

/// 泳道图常量。出处：`git-graph-row.tsx:4-6,27`。
const GRAPH_LANE_GAP: f32 = 13.;
const GRAPH_PADDING: f32 = 8.;
const GRAPH_MIN_WIDTH: f32 = 30.;
const GRAPH_LINE_WIDTH: f32 = 1.6;
const GRAPH_NODE_RADIUS: f32 = 4.3;
const GRAPH_NODE_STROKE: f32 = 2.;
/// 标签徽章。出处：`git-graph-row.tsx:87-88`。
const LABEL_MAX_WIDTH: f32 = 112.;
const LABEL_FONT_SIZE: f32 = 10.;
const LABEL_RADIUS: f32 = 6.4;
const LABEL_PADDING_X: f32 = 6.;
const LABEL_PADDING_Y: f32 = 2.;
const LABEL_GAP: f32 = 6.;

/// 三栏比例与最小宽。出处：`git-log-preferences.store.ts:43-47`、`git-log-tool-window.tsx:622,648,684`。
const REFERENCE_PANE_FRACTION: f32 = 0.19;
const COMMIT_PANE_FRACTION: f32 = 0.57;
const INSPECTOR_PANE_FRACTION: f32 = 0.24;
const REFERENCE_PANE_MIN_WIDTH: f32 = 140.;
const COMMIT_PANE_MIN_WIDTH: f32 = 320.;
const INSPECTOR_PANE_MIN_WIDTH: f32 = 220.;

/// 引用栏左侧竖排工具栏。出处：`git-reference-tree.tsx:357,371,146`。
const REFERENCE_TOOLBAR_WIDTH: f32 = 36.;
const REFERENCE_TOOLBAR_PADDING_Y: f32 = 4.;
const REFERENCE_TOOLBAR_BUTTON_SIZE: f32 = 32.;
const REFERENCE_TOOLBAR_BUTTON_RADIUS: f32 = 4.8;
const REFERENCE_TOOLBAR_ICON_SIZE: f32 = 16.;
const REFERENCE_TOOLBAR_GAP: f32 = 4.;
const REFERENCE_TOOLBAR_SEPARATOR_WIDTH: f32 = 20.;
const REFERENCE_TOOLBAR_SEPARATOR_HEIGHT: f32 = 1.;
const REFERENCE_TOOLBAR_SEPARATOR_MARGIN_Y: f32 = 4.;
/// 引用栏列表头 / 滚动区。出处：`git-reference-tree.tsx:840,847`。
const REFERENCE_HEADER_HEIGHT: f32 = 32.;
const REFERENCE_LIST_PADDING: f32 = 6.;
/// HEAD 行。出处：`git-reference-tree.tsx:853`。
const REFERENCE_HEAD_ROW_HEIGHT: f32 = 28.;
const REFERENCE_HEAD_ROW_RADIUS: f32 = 6.4;
const REFERENCE_HEAD_ROW_PADDING_X: f32 = 8.;
const REFERENCE_HEAD_ROW_GAP: f32 = 8.;
const REFERENCE_HEAD_ROW_MARGIN_BOTTOM: f32 = 4.;
/// 分区头 / 引用行 / 缩进。出处：`git-reference-tree.tsx:876,599,594`。
const REFERENCE_SECTION_ROW_HEIGHT: f32 = 24.;
const REFERENCE_SECTION_RADIUS: f32 = 6.4;
const REFERENCE_SECTION_PADDING_X: f32 = 6.;
const REFERENCE_ROW_HEIGHT: f32 = 24.;
const REFERENCE_ROW_RADIUS: f32 = 4.8;
const REFERENCE_ROW_GAP: f32 = 6.;
const REFERENCE_INDENT_BASE: f32 = 10.;
const REFERENCE_INDENT_STEP: f32 = 14.;
const REFERENCE_DISCLOSURE_SIZE: f32 = 14.;
const REFERENCE_ICON_SIZE: f32 = 14.;
const REFERENCE_SECTION_MARGIN_BOTTOM: f32 = 4.;
/// 「当前」徽章 / ahead-behind。出处：`git-reference-tree.tsx:652`、`git-tracking-counts.tsx:36-53`。
const REFERENCE_BADGE_FONT_SIZE: f32 = 10.;
const REFERENCE_BADGE_PADDING_X: f32 = 4.;
const TRACKING_COUNT_FONT_SIZE: f32 = 10.;
const TRACKING_COUNT_GAP: f32 = 4.;
const TRACKING_COUNT_MAX: usize = 99;
/// 空分区占位。出处：`git-reference-tree.tsx:919`（`h-6 pl-8`）。
const REFERENCE_EMPTY_HEIGHT: f32 = 24.;
const REFERENCE_EMPTY_PADDING_LEFT: f32 = 32.;

/// Inspector。出处：`git-commit-inspector.tsx:96-185`、`git-log-preferences.store.ts:49-52`。
const INSPECTOR_FILES_FRACTION: f32 = 0.62;
const INSPECTOR_DETAILS_FRACTION: f32 = 0.38;
const INSPECTOR_FILES_MIN_HEIGHT: f32 = 90.;
const INSPECTOR_DETAILS_MIN_HEIGHT: f32 = 80.;
const INSPECTOR_HEADER_HEIGHT: f32 = 32.;
const INSPECTOR_HEADER_PADDING_X: f32 = 8.;
const INSPECTOR_HEADER_GAP: f32 = 8.;
const INSPECTOR_DETAIL_PADDING: f32 = 12.;
const INSPECTOR_DETAIL_GAP: f32 = 8.;
const INSPECTOR_MONO_FONT_SIZE: f32 = 11.;
const INSPECTOR_HASH_FONT_SIZE: f32 = 10.;
/// 提交文件树。出处：`git-commit-file-tree.tsx:181-184,127`、`file-explorer/lib/file-tree-row.ts:1-13`。
const COMMIT_FILE_TREE_PADDING: f32 = 6.;
const COMMIT_FILE_ROW_HEIGHT: f32 = 24.;
const COMMIT_FILE_INDENT_BASE: f32 = 10.;
const COMMIT_FILE_INDENT_STEP: f32 = 14.;
const COMMIT_FILE_STATUS_FONT_SIZE: f32 = 10.;

/// 控制台。出处：`git-execution-console.tsx:68-103`。
const CONSOLE_FONT_SIZE: f32 = 12.;
const CONSOLE_TOOLBAR_WIDTH: f32 = 32.;
const CONSOLE_TOOLBAR_PADDING_Y: f32 = 4.;
const CONSOLE_TOOLBAR_GAP: f32 = 4.;
const CONSOLE_OUTPUT_PADDING: f32 = 12.;

/// `git.historyPage` 的页大小。Core 默认 300（`git/history.rs:20`），上一轮实现与 macOS
/// 口径都用 100（`GitFeatureModel.swift:298`）；这里取 100：提交行是自绘的（见模块文档），
/// 一页 100 行在首帧的布局量可控。
const HISTORY_PAGE_LIMIT: usize = 100;
/// 不是仓库时，最多向上层目录探测多少个一级子目录（[`discover_repository_root`]）。
const REPOSITORY_PROBE_LIMIT: usize = 6;

/// 每次 Core 调用递增的 `operationId` 序列。
static NEXT_OPERATION_ID: AtomicUsize = AtomicUsize::new(1);
/// Git 读命令超时（毫秒）。照 `shell_probe/files.rs` 的 `workspace.snapshot` 口径取 60 s。
const GIT_TIMEOUT_MILLIS: u32 = 60_000;

// ---------------------------------------------------------------------------
// 数据模型
// ---------------------------------------------------------------------------

/// 面板内的两个页签。Windows 是局部 `useState<"log" | "console">("log")`
/// （`git-log-tool-window.tsx:80`）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Log,
    Console,
}

/// 仓库数据的加载状态。对应 `git-log-tool-window.tsx:602-612` 的分支：
/// 无仓库 / 加载中 / 失败且无数据 / （有数据时的）刷新失败横幅。
#[derive(Clone, Copy, PartialEq, Eq)]
enum LoadState {
    /// 首屏还没回来（`git-log-tool-window.tsx:607-610`）。
    Loading,
    /// 有数据。
    Ready,
    /// **有**数据但最近一次刷新失败 → 画横幅（`:589-600`）。
    Stale,
    /// 失败且没有数据 → 居中错误 + 重试。
    Failed,
    /// `git.status` 说这不是 Git 仓库（`:602-606`）。
    NoRepository,
}

/// 筛选字段。对应 `git-log-preferences.store.ts:8` 的 `"text" | "author" | "branch"`。
#[derive(Clone, Copy, PartialEq, Eq)]
enum FilterScope {
    Text,
    Author,
    Branch,
}

impl FilterScope {
    /// 文案逐字取自 `i18n/locale.ts:7279-7281`。
    fn label(self) -> SharedString {
        SharedString::from(match self {
            FilterScope::Text => "文本",
            FilterScope::Author => "作者",
            FilterScope::Branch => "分支",
        })
    }

    /// 占位文案 `{field} 筛选`（`git.log.filterPlaceholder`，`locale.ts:7282`）。
    fn placeholder(self) -> SharedString {
        SharedString::from(format!("{} 筛选", self.label()))
    }

    fn all() -> [FilterScope; 3] {
        [FilterScope::Text, FilterScope::Author, FilterScope::Branch]
    }
}

/// 引用种类。Core 的 `kind` 字段（`git/mod.rs:5896-5902`）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum RefKind {
    Local,
    Remote,
    Tag,
}

impl RefKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "local" => Some(RefKind::Local),
            "remote" => Some(RefKind::Remote),
            "tag" => Some(RefKind::Tag),
            _ => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            RefKind::Local => "local",
            RefKind::Remote => "remote",
            RefKind::Tag => "tag",
        }
    }

    /// 分区标题文案逐字取自 `i18n/locale.ts:7252-7254`。
    fn title(self) -> SharedString {
        SharedString::from(match self {
            RefKind::Local => "本地",
            RefKind::Remote => "远程",
            RefKind::Tag => "标签",
        })
    }

    fn sections() -> [RefKind; 3] {
        [RefKind::Local, RefKind::Remote, RefKind::Tag]
    }
}

/// 一条引用（Core `GitReferenceResponse`，`protocol/contracts.rs:583-597`）。
#[derive(Clone)]
struct Reference {
    full_name: SharedString,
    short_name: SharedString,
    kind: RefKind,
    is_current: bool,
    upstream_short_name: Option<SharedString>,
    ahead: usize,
    behind: usize,
}

/// 提交标签的种类。Windows 的 `GitGraphLabel.kind`（`git-graph-row.tsx:13-24`）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum LabelKind {
    /// `HEAD`（或 `HEAD -> x` 里的 HEAD 那一段）。
    Head,
    /// 远端分支（名字里含 `/`）。
    Remote,
    /// `tag: x`。
    Tag,
    /// 本地分支。
    Branch,
}

/// 一个标签徽章。
#[derive(Clone)]
struct Label {
    title: SharedString,
    kind: LabelKind,
}

/// 一行提交（Core `GitCommitResponse`，`protocol/contracts.rs:602-611`）。
#[derive(Clone)]
struct Commit {
    hash: SharedString,
    short_hash: SharedString,
    parent_hashes: Vec<SharedString>,
    /// Core 只给 `%s`（主题行，`git/history.rs:302`），没有正文。
    subject: SharedString,
    author: SharedString,
    email: SharedString,
    date: SharedString,
    labels: Vec<Label>,
}

/// 提交行左侧的泳道图（简化版，见模块文档偏差 2）。
struct GraphRow {
    /// 每条泳道的线色下标；`None` = 这条泳道本行上半部分没有线。
    lanes: Vec<Option<usize>>,
    /// 本提交所在的泳道。
    lane: usize,
    /// 本行的节点颜色下标。
    node_color: usize,
    /// 往下连的边：`(泳道, 颜色下标, 父提交不在本页)`。
    edges: Vec<(usize, usize, bool)>,
}

/// 提交文件树的一行。
#[derive(Clone)]
struct CommitFileRow {
    depth: usize,
    name: SharedString,
    is_folder: bool,
    /// 目录行的文件数（含子目录，递归）。
    file_count: usize,
    /// 文件行的 name-status 码（Core `GitFileResponse.status`，`contracts.rs:703-707`）。
    status: SharedString,
}

/// 文件列表的加载状态。对应 `git-commit-inspector.tsx:19` 的
/// `"idle" | "loading" | "ready" | "failed"`。
#[derive(Clone, Copy, PartialEq, Eq)]
enum FilesState {
    /// 还没选中提交。
    Idle,
    Loading,
    Ready,
    Failed,
}

/// Inspector 下半部分的提交详情。取选中的那一行的字段（Windows 也是同一个 `GitCommit`）。
struct Detail {
    subject: SharedString,
    short_hash: SharedString,
    author: SharedString,
    email: SharedString,
    date: SharedString,
    /// 装饰拼回一行文本（Windows 直接渲染 `commit.decorations`，`git-commit-inspector.tsx:180`）。
    decorations: SharedString,
    hash: SharedString,
}

/// 交给后台任务的游标句柄：`git.historyCursorClose` 需要 `root` 与 `cursor` 成对
/// （`git/history.rs:344` 会校验游标属于哪个 root）。
struct HistoryCursor {
    root: String,
    cursor: String,
}

impl HistoryCursor {
    /// 把游标还给 Core（同步、很快：只是摘掉注册表项并停掉子进程）。
    fn close(self) {
        let _ = execute_core(
            "git.historyCursorClose",
            serde_json::json!({ "root": self.root, "cursor": self.cursor }),
        );
    }
}

/// 一次「首屏」读取的结果（工作区探测 → `git.status` → `git.references` → `git.historyPage`）。
///
/// 必须在后台线程上构造再送回前台，所以只装 `String` / `SharedString` 这些 `Send` 数据。
struct FirstLoad {
    /// 解析成绝对路径的仓库根；`None` = 不是 Git 仓库。
    repository_root: Option<String>,
    branch: Option<SharedString>,
    references: Vec<Reference>,
    commits: Vec<Commit>,
    next_cursor: Option<String>,
    has_more: bool,
    /// 失败的命令（空 = 全部成功）。
    failures: Vec<String>,
}

/// 追加一页的结果。
struct MoreLoad {
    commits: Vec<Commit>,
    next_cursor: Option<String>,
    has_more: bool,
    failure: Option<String>,
}

/// 一个 Icon 按钮的点击回调。
type ButtonHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// 把闭包包成 [`ButtonHandler`]。
///
/// 走一层泛型函数而不是直接 `Box::new(..)`：闭包的参数类型由 `Fn(&ClickEvent, &mut Window,
/// &mut App)` 这个 bound 推出来，不必在每个调用点手写三处参数类型。
fn handler(f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> ButtonHandler {
    Box::new(f)
}

// ---------------------------------------------------------------------------
// Core 调用与解析
// ---------------------------------------------------------------------------

/// 调一次 Core 命令：拼请求、判 `ok`、取 `data`。
///
/// 形态与 `shell_probe/files.rs` 一致：`id` / `operationId` / `timeoutMilliseconds` / `command` /
/// `payload`。失败返回错误码字符串（`/error/code`）；非仓库那种「`ok:true` + 字段为 null」
/// 由调用方判字段。
fn execute_core(command: &str, payload: serde_json::Value) -> Result<serde_json::Value, String> {
    let call = NEXT_OPERATION_ID.fetch_add(1, Ordering::Relaxed);
    let operation_id = format!("shell-probe-{command}-{call}");
    let request = serde_json::json!({
        "id": operation_id,
        "operationId": operation_id,
        "timeoutMilliseconds": GIT_TIMEOUT_MILLIS,
        "command": command,
        "payload": payload,
    })
    .to_string();

    let raw = lithe_core::execute_json(&request);
    let value: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| format!("响应不是合法 JSON：{error}"))?;

    if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        let code = value
            .pointer("/error/code")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        return Err(code.to_string());
    }

    value
        .get("data")
        .cloned()
        .ok_or_else(|| "响应缺少 data".to_string())
}

/// 把 `git.status.repositoryRoot` 变成可用的绝对根。
///
/// Core 给的是 `relative_or_absolute(&repository_root, &root)`（`rust/lithe-core/src/git/mod.rs:6574`）：
/// 仓库根就在工作区里时会返回**相对路径**，直接拿去当后面几条命令的 `root` 会按进程工作目录解析。
fn resolve_repository_root(root: &Path, repository_root: &str) -> String {
    let path = Path::new(repository_root);
    if path.is_absolute() {
        repository_root.to_string()
    } else {
        root.join(path).to_string_lossy().to_string()
    }
}

/// `git.status` 的 `repositoryRoot`（`contracts.rs:533`；非仓库为 `null`）。
fn repository_root_of(data: &serde_json::Value) -> Option<String> {
    data.get("repositoryRoot")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

/// 读 `git.status`，返回 `(仓库根, 分支)`；仓库根为 `None` 表示不是仓库。
fn read_status(root: &str) -> Result<(Option<String>, Option<SharedString>), String> {
    let data = execute_core("git.status", serde_json::json!({ "root": root }))?;
    let branch = data
        .get("branch")
        .and_then(serde_json::Value::as_str)
        .map(SharedString::from);
    Ok((repository_root_of(&data), branch))
}

/// 工作区根不是仓库时的兜底：用 `workspace.snapshot` 拿一级子目录，逐个探 `git.status`。
///
/// Windows 用 `git_discover_workspace_repos` 找子目录里的仓库，那不在本步拍板的 6 条命令里；
/// 这里用 `workspace.snapshot`（`project/files.rs:160-167`）+ `git.status` 复现同一件事，
/// 探测**有上限**（[`REPOSITORY_PROBE_LIMIT`]），且只探一级目录：`.git` 本身被
/// `BUILT_IN_HIDDEN_DIRECTORIES` 过滤掉（`files.rs:15-30`），所以看的是「哪个子目录是仓库」。
fn discover_repository_root(root: &Path) -> Option<String> {
    let data = execute_core(
        "workspace.snapshot",
        serde_json::json!({ "root": root.to_string_lossy() }),
    )
    .ok()?;

    let children = data.pointer("/root/children")?.as_array()?;
    let directories = children.iter().filter_map(|child| {
        let is_directory = child
            .get("isDirectory")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let path = child.get("path").and_then(serde_json::Value::as_str)?;
        is_directory.then(|| root.join(path).to_string_lossy().to_string())
    });

    for candidate in directories.take(REPOSITORY_PROBE_LIMIT) {
        if let Ok(data) = execute_core("git.status", serde_json::json!({ "root": candidate })) {
            if let Some(found) = repository_root_of(&data) {
                return Some(resolve_repository_root(root, &found));
            }
        }
    }

    None
}

/// 解析 `git.references` 的 `references[]`（`contracts.rs:583-597`）。
fn parse_references(data: &serde_json::Value) -> Vec<Reference> {
    data.get("references")
        .and_then(serde_json::Value::as_array)
        .map(|references| {
            references
                .iter()
                .filter_map(|reference| {
                    Some(Reference {
                        full_name: SharedString::from(reference.get("fullName")?.as_str()?),
                        short_name: SharedString::from(reference.get("shortName")?.as_str()?),
                        kind: RefKind::parse(reference.get("kind")?.as_str()?)?,
                        is_current: reference
                            .get("isCurrent")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false),
                        upstream_short_name: reference
                            .get("upstreamShortName")
                            .and_then(serde_json::Value::as_str)
                            .map(SharedString::from),
                        ahead: reference
                            .get("ahead")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0) as usize,
                        behind: reference
                            .get("behind")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0) as usize,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 解析 `%D` 装饰串（`commit.decorations`，`contracts.rs:610`）。
///
/// Core 用 `--pretty=…%x1f%D`（`git/history.rs:302`），`%D` 是逗号分隔的引用名，例如
/// `HEAD -> main, origin/main, tag: v1.0.0`。种类判定同 Windows 的四档
/// （`git-graph-row.tsx:13-24`）：`HEAD -> x` → head + branch；`tag: x` → tag；
/// 含 `/` → remote；其余 → branch。
///
/// **偏差（已登记）**：`%D` 不带种类信息，名字里含 `/` 的**本地**分支（如 `fix/order`）会被判成
/// remote；要判准得把 `git.references` 的远端名单一起传进来。
fn parse_decorations(value: &str) -> Vec<Label> {
    let mut labels = Vec::new();

    for part in value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some(branch) = part.strip_prefix("HEAD -> ") {
            labels.push(Label {
                title: SharedString::from("HEAD"),
                kind: LabelKind::Head,
            });
            labels.push(Label {
                title: SharedString::from(branch.to_string()),
                kind: LabelKind::Branch,
            });
            continue;
        }

        if let Some(tag) = part.strip_prefix("tag: ") {
            labels.push(Label {
                title: SharedString::from(tag.to_string()),
                kind: LabelKind::Tag,
            });
            continue;
        }

        let (title, kind) = if part == "HEAD" {
            (part, LabelKind::Head)
        } else if part.contains('/') {
            (part, LabelKind::Remote)
        } else {
            (part, LabelKind::Branch)
        };
        labels.push(Label {
            title: SharedString::from(title.to_string()),
            kind,
        });
    }

    labels
}

/// 解析 `git.historyPage` 的 `commits[]`（`contracts.rs:602-611`）。
fn parse_commits(data: &serde_json::Value) -> Vec<Commit> {
    data.get("commits")
        .and_then(serde_json::Value::as_array)
        .map(|commits| {
            commits
                .iter()
                .filter_map(|commit| {
                    Some(Commit {
                        hash: SharedString::from(commit.get("hash")?.as_str()?),
                        short_hash: SharedString::from(
                            commit
                                .get("shortHash")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        parent_hashes: commit
                            .get("parentHashes")
                            .and_then(serde_json::Value::as_array)
                            .map(|parents| {
                                parents
                                    .iter()
                                    .filter_map(serde_json::Value::as_str)
                                    .map(SharedString::from)
                                    .collect()
                            })
                            .unwrap_or_default(),
                        subject: SharedString::from(
                            commit
                                .get("subject")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        author: SharedString::from(
                            commit
                                .get("authorName")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        email: SharedString::from(
                            commit
                                .get("authorEmail")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        date: SharedString::from(
                            commit
                                .get("date")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        labels: parse_decorations(
                            commit
                                .get("decorations")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 一页历史的公共解析：`(commits, nextCursor, hasMore)`。
fn parse_history_page(data: &serde_json::Value) -> (Vec<Commit>, Option<String>, bool) {
    let commits = parse_commits(data);
    let next_cursor = data
        .get("nextCursor")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let has_more = data
        .get("hasMore")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(next_cursor.is_some());
    (commits, next_cursor, has_more)
}

/// 读一页提交：`git.historyPage { root, reference, order, cursor, limit }`。
///
/// `order: "date"`（`git/history.rs:80-82` 的 `HistoryOrder::Date`，IDEA 的 Normal 排序）；
/// `reference` / `order` 必须与创建 session 时一致，否则 Core 返回 `invalidCursor`
/// （`git/history.rs:228-235`）。
fn read_history_page(
    root: &str,
    reference: Option<&str>,
    cursor: Option<&str>,
) -> Result<(Vec<Commit>, Option<String>, bool), String> {
    let data = execute_core(
        "git.historyPage",
        serde_json::json!({
            "root": root,
            "reference": reference,
            "order": "date",
            "cursor": cursor,
            "limit": HISTORY_PAGE_LIMIT,
        }),
    )?;
    Ok(parse_history_page(&data))
}

/// 首屏读取：工作区探测 → `git.status` → `git.references` → `git.historyPage`。
///
/// 非仓库时**不再白跑**后面三条（约束 2）。`old_cursor` 是上一次持有的游标，进函数第一件事就是
/// 把它还回去，免得新 session 建起来后旧 `git log` 进程还挂着。
fn load_first(
    root: &Path,
    reference: Option<String>,
    old_cursor: Option<HistoryCursor>,
) -> FirstLoad {
    if let Some(cursor) = old_cursor {
        cursor.close();
    }

    let mut load = FirstLoad {
        repository_root: None,
        branch: None,
        references: Vec::new(),
        commits: Vec::new(),
        next_cursor: None,
        has_more: false,
        failures: Vec::new(),
    };

    let root_text = root.to_string_lossy().to_string();

    // 1) git.status：仓库根 + 当前分支。
    match read_status(&root_text) {
        Ok((repository_root, branch)) => {
            load.repository_root =
                repository_root.map(|found| resolve_repository_root(root, &found));
            load.branch = branch;
        }
        Err(code) => load.failures.push(format!("git.status（{code}）")),
    }

    if load.repository_root.is_none() {
        // 不是仓库：用工作区快照的一级目录再找一次（有上限），仍找不到就走「未打开仓库」。
        load.repository_root = discover_repository_root(root);
    }

    let Some(repository_root) = load.repository_root.clone() else {
        return load;
    };

    // 2) git.references：引用树。
    match execute_core(
        "git.references",
        serde_json::json!({ "root": repository_root.clone() }),
    ) {
        Ok(data) => load.references = parse_references(&data),
        Err(code) => load.failures.push(format!("git.references（{code}）")),
    }

    // 3) git.historyPage：第一页。
    match read_history_page(&repository_root, reference.as_deref(), None) {
        Ok((commits, next_cursor, has_more)) => {
            load.commits = commits;
            load.next_cursor = next_cursor;
            load.has_more = has_more;
        }
        Err(code) => load.failures.push(format!("git.historyPage（{code}）")),
    }

    load
}

/// 追加一页：`git.historyPage` 带上游标继续读同一个 `git log` 流。
///
/// 成功时**不关游标**（session 还在，`nextCursor` 与传入的游标是同一个串，`git/history.rs:244`）；
/// 失败时 Core 已经 `session.stop()`（`:258-261`），游标作废。
fn load_more(root: &str, reference: Option<String>, cursor: &str) -> MoreLoad {
    match read_history_page(root, reference.as_deref(), Some(cursor)) {
        Ok((commits, next_cursor, has_more)) => MoreLoad {
            commits,
            next_cursor,
            has_more,
            failure: None,
        },
        Err(code) => MoreLoad {
            commits: Vec::new(),
            next_cursor: None,
            has_more: false,
            failure: Some(code),
        },
    }
}

/// 读一个提交的文件列表：`git.commitFiles { root, commit }`（`git/mod.rs:2013-2038`）。
///
/// 返回 `(status, path)` 对，由 [`build_commit_files`] 聚成多层目录树。
fn load_commit_files(root: &str, commit: &str) -> Result<Vec<(String, String)>, String> {
    let data = execute_core(
        "git.commitFiles",
        serde_json::json!({ "root": root, "commit": commit }),
    )?;

    let files = data
        .get("files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "响应缺少 files".to_string())?;

    Ok(files
        .iter()
        .filter_map(|file| {
            let path = file.get("path")?.as_str()?;
            let status = file
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("M");
            (!path.is_empty()).then(|| (status.to_string(), path.to_string()))
        })
        .collect())
}

// ---------------------------------------------------------------------------
// 纯函数：泳道布局 / 引用树 / 文件树 / 过滤
// ---------------------------------------------------------------------------

/// 泳道颜色下标 → 主题色（偏差 3）。
fn lane_color(index: usize, cx: &App) -> Hsla {
    let theme = cx.theme();
    match index % 6 {
        0 => theme.success,
        1 => theme.blue_light,
        2 => theme.magenta_light,
        3 => theme.warning,
        4 => theme.danger,
        _ => theme.cyan_light,
    }
}

/// 标签四档的颜色（偏差 3）。
fn label_color(kind: LabelKind, cx: &App) -> Hsla {
    let theme = cx.theme();
    match kind {
        LabelKind::Head => theme.cyan_light,
        LabelKind::Remote => theme.blue_light,
        LabelKind::Tag => theme.yellow_light,
        LabelKind::Branch => theme.green_light,
    }
}

/// 简化泳道布局（模块文档偏差 2）。
///
/// 规则与 `git-graph-layout.ts` 同构：每条泳道记住「在等哪个提交」；本提交进到自己那条泳道，
/// 第一父提交留在原泳道，其余父提交塞进空泳道（没有空位就新开一条）；行尾回收尾部空泳道。
/// 颜色按泳道下标取 6 色循环，跨泳道父边只画目标泳道的竖直段。
fn layout_graph(commits: &[Commit]) -> Vec<GraphRow> {
    let known: BTreeSet<&SharedString> = commits.iter().map(|commit| &commit.hash).collect();
    let mut lanes: Vec<Option<SharedString>> = Vec::new();
    let mut rows = Vec::with_capacity(commits.len());

    for commit in commits {
        // 本提交在哪条泳道：等它的那条；没有就是新开一条。
        let lane = match lanes
            .iter()
            .position(|expected| expected.as_ref() == Some(&commit.hash))
        {
            Some(lane) => lane,
            None => {
                lanes.push(None);
                lanes.len() - 1
            }
        };

        // 入线：本行上半部分经过的泳道。
        let mut lane_colors: Vec<Option<usize>> = lanes
            .iter()
            .enumerate()
            .map(|(index, expected)| expected.as_ref().map(|_| index % 6))
            .collect();
        let node_color = lane_colors.get(lane).copied().flatten().unwrap_or(lane % 6);

        // 出线：把父提交排进泳道，再按结果画下半段。
        let mut edges: Vec<(usize, usize, bool)> = Vec::new();
        for (position, parent) in commit.parent_hashes.iter().enumerate() {
            let target = if position == 0 {
                lane
            } else if let Some(existing) = lanes
                .iter()
                .position(|expected| expected.as_ref() == Some(parent))
            {
                existing
            } else if let Some(free) = lanes.iter().position(Option::is_none) {
                free
            } else {
                lanes.push(None);
                lanes.len() - 1
            };

            let missing = !known.contains(parent);
            edges.push((target, target % 6, missing));
            lanes[target] = Some(parent.clone());
            while lane_colors.len() <= target {
                lane_colors.push(None);
            }
        }
        if commit.parent_hashes.is_empty() {
            lanes[lane] = None;
        }

        // 跨泳道父边要在目标泳道的下半部分补一段线，所以泳道数要覆盖到最远的边。
        if let Some((widest, _, _)) = edges.iter().max_by_key(|(lane, _, _)| *lane) {
            while lane_colors.len() <= *widest {
                lane_colors.push(None);
            }
        }

        rows.push(GraphRow {
            lanes: lane_colors,
            lane,
            node_color,
            edges,
        });

        while lanes.last().is_some_and(|lane| lane.is_none()) {
            lanes.pop();
        }
    }

    rows
}

/// 引用树的一个节点（Windows `buildGitReferenceTree` 的目录分组）。
struct RefNode {
    /// 这一段的名字（目录名或引用短名）。
    name: SharedString,
    /// 分组 id（`"<kind>/<path>"`），用于折叠状态。
    id: String,
    /// 叶子节点指向 `BottomPane::references` 的下标。
    reference: Option<usize>,
    children: Vec<RefNode>,
}

/// 摊平后的一行。
struct RefRow {
    depth: usize,
    name: SharedString,
    id: String,
    reference: Option<usize>,
}

/// 把一类引用按 `/` 分组（顺序保持 Core 给的顺序，Core 已保证确定性）。
fn build_reference_rows(references: &[Reference], kind: RefKind) -> Vec<RefRow> {
    let mut roots: Vec<RefNode> = Vec::new();

    for (index, reference) in references.iter().enumerate() {
        if reference.kind != kind {
            continue;
        }

        let segments: Vec<&str> = reference
            .short_name
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        let Some((leaf, directories)) = segments.split_last() else {
            continue;
        };

        let mut path = String::new();
        let mut level = &mut roots;
        for directory in directories {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(directory);
            let id = format!("{}/{}", kind.id(), path);
            let position = match level.iter().position(|node| node.id == id) {
                Some(position) => position,
                None => {
                    level.push(RefNode {
                        name: SharedString::from((*directory).to_string()),
                        id: id.clone(),
                        reference: None,
                        children: Vec::new(),
                    });
                    level.len() - 1
                }
            };
            level = &mut level[position].children;
        }

        let mut leaf_path = path.clone();
        if !leaf_path.is_empty() {
            leaf_path.push('/');
        }
        leaf_path.push_str(leaf);
        level.push(RefNode {
            name: SharedString::from((*leaf).to_string()),
            id: format!("{}/{}", kind.id(), leaf_path),
            reference: Some(index),
            children: Vec::new(),
        });
    }

    let mut rows = Vec::new();
    flatten_reference_nodes(&roots, 0, &mut rows);
    rows
}

fn flatten_reference_nodes(nodes: &[RefNode], depth: usize, rows: &mut Vec<RefRow>) {
    for node in nodes {
        rows.push(RefRow {
            depth,
            name: node.name.clone(),
            id: node.id.clone(),
            reference: node.reference,
        });
        flatten_reference_nodes(&node.children, depth + 1, rows);
    }
}

/// 提交文件树的中间节点（Windows `GitCommitFileTreeNode.build`）。
#[derive(Default)]
struct FileNode {
    dirs: Vec<(String, FileNode)>,
    files: Vec<(String, String)>,
}

impl FileNode {
    fn insert(&mut self, path: &str, status: &str) {
        let segments: Vec<&str> = path.split('/').collect();
        let Some((name, directories)) = segments.split_last() else {
            return;
        };

        let mut node = self;
        for directory in directories {
            let position = match node.dirs.iter().position(|(key, _)| key == directory) {
                Some(position) => position,
                None => {
                    node.dirs
                        .push(((*directory).to_string(), FileNode::default()));
                    node.dirs.len() - 1
                }
            };
            node = &mut node.dirs[position].1;
        }
        node.files.push(((*name).to_string(), status.to_string()));
    }

    fn file_count(&self) -> usize {
        self.files.len() + self.dirs.iter().map(|(_, child)| child.file_count()).sum::<usize>()
    }

    fn flatten(&self, depth: usize, rows: &mut Vec<CommitFileRow>) {
        for (name, child) in &self.dirs {
            rows.push(CommitFileRow {
                depth,
                name: SharedString::from(name.clone()),
                is_folder: true,
                file_count: child.file_count(),
                status: SharedString::from(""),
            });
            child.flatten(depth + 1, rows);
        }

        for (name, status) in &self.files {
            rows.push(CommitFileRow {
                depth,
                name: SharedString::from(name.clone()),
                is_folder: false,
                file_count: 0,
                status: SharedString::from(status.clone()),
            });
        }
    }
}

/// 把 `git.commitFiles` 的 `(status, path)` 聚成多层目录树。
fn build_commit_files(files: &[(String, String)]) -> Vec<CommitFileRow> {
    let mut root = FileNode::default();
    for (status, path) in files {
        root.insert(path, status);
    }

    let mut rows = Vec::new();
    root.flatten(0, &mut rows);
    rows
}

/// 提交过滤。逐字照 `features/git/utils/git-log-filter.ts:4-19`。
fn matches_filter(commit: &Commit, query: &str, scope: FilterScope) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }

    match scope {
        FilterScope::Author => {
            contains(&commit.author, &query) || contains(&commit.email, &query)
        }
        FilterScope::Branch => commit.labels.iter().any(|label| contains(&label.title, &query)),
        FilterScope::Text => {
            contains(&commit.subject, &query)
                || contains(&commit.hash, &query)
                || contains(&commit.short_hash, &query)
        }
    }
}

fn contains(value: &SharedString, query: &str) -> bool {
    value.to_lowercase().contains(query)
}

/// ahead / behind 计数，上限 [`TRACKING_COUNT_MAX`] → `99+`
/// （`git-tracking-counts.tsx:9-14`）。
fn tracking_count(value: usize) -> SharedString {
    if value > TRACKING_COUNT_MAX {
        SharedString::from(format!("{TRACKING_COUNT_MAX}+"))
    } else {
        SharedString::from(value.to_string())
    }
}

// ---------------------------------------------------------------------------
// 面板
// ---------------------------------------------------------------------------

/// 底部工具窗「提交记录」。
///
/// 对外只暴露 [`BottomPane::new`] 与 [`BottomPane::refresh`] / [`BottomPane::set_visible`] /
/// [`BottomPane::visible`]；字段全部私有。本面板不开浮层（对话框 / 抽屉 / 通知的挂层由窗口
/// 根视图负责）。
pub struct BottomPane {
    /// 工作区根，来自命令行参数（`git.*` 命令都要求带 `root`）。
    root: PathBuf,
    /// `git.status.repositoryRoot` 解析成绝对路径后的仓库根（见 [`resolve_repository_root`]）。
    repository_root: Option<String>,
    /// `git.status.branch`：没有选中引用时标题栏的兜底。
    branch: Option<SharedString>,
    load_state: LoadState,
    /// 当前页签（局部状态，`git-log-tool-window.tsx:80`）。
    panel: Panel,
    /// 引用（`git.references`）。
    references: Vec<Reference>,
    /// 提交（`git.historyPage`，可能有多页）。
    commits: Vec<Commit>,
    /// 持有的分页游标；`None` = 读完了 / 没在读。
    cursor: Option<HistoryCursor>,
    has_more: bool,
    loading_more: bool,
    /// 选中的引用（[`Self::references`] 的下标）。
    selected_reference: Option<usize>,
    /// 选中的提交（[`Self::commits`] 的下标）。
    selected_commit: Option<usize>,
    /// 折叠的分区（`local / remote / tag`）。
    collapsed_sections: BTreeSet<&'static str>,
    /// 折叠的引用分组 id。
    collapsed_groups: BTreeSet<String>,
    /// 「只显示我的分支」（`git-log-preferences.store.ts:64`）。
    show_my_branches_only: bool,
    /// 引用树是否画装饰（`git-log-preferences.store.ts:63`，默认 true）。
    show_decorations: bool,
    /// 提交文件列表状态。
    files_state: FilesState,
    files: Vec<CommitFileRow>,
    /// 提交详情（选中提交后填）。
    detail: Option<Detail>,
    /// 筛选输入框（`git-commit-table.tsx:201` 的 `<input>`）。
    filter: Entity<InputState>,
    filter_query: SharedString,
    filter_scope: FilterScope,
    /// 输入事件订阅：不存下来会被立刻丢掉，输入框就不再触发重绘。
    _filter_subscription: Subscription,
    /// 可见性（Windows 由外部 `setIsBottomPaneVisible` 控制；这里给父级一个钩子）。
    visible: bool,
    /// 请求代次：晚到的旧回包直接丢掉。
    request_serial: u64,
}

impl BottomPane {
    /// 建立面板并立刻在后台跑一遍首屏读取。
    ///
    /// `root` 是工作区根；`git.status` 会在它下面解析出真正的仓库根。
    pub fn new(root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let filter_scope = FilterScope::Text;
        let placeholder = filter_scope.placeholder();
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));

        let filter_subscription = cx.subscribe_in(
            &filter,
            window,
            |pane: &mut Self, state: &Entity<InputState>, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    pane.filter_query = state.read(cx).value();
                    cx.notify();
                }
            },
        );

        let mut pane = Self {
            root: root.clone(),
            repository_root: None,
            branch: None,
            load_state: LoadState::Loading,
            panel: Panel::Log,
            references: Vec::new(),
            commits: Vec::new(),
            cursor: None,
            has_more: false,
            loading_more: false,
            selected_reference: None,
            selected_commit: None,
            collapsed_sections: BTreeSet::new(),
            collapsed_groups: BTreeSet::new(),
            show_my_branches_only: false,
            show_decorations: true,
            files_state: FilesState::Idle,
            files: Vec::new(),
            detail: None,
            filter,
            filter_query: SharedString::from(""),
            filter_scope,
            _filter_subscription: filter_subscription,
            visible: true,
            request_serial: 0,
        };

        pane.spawn_first_load(cx);
        pane
    }

    /// 可见性钩子（Windows 的 `setIsBottomPaneVisible(false)`，`git-log-tool-window.tsx:579`）。
    /// 父级在布局里应先问 [`Self::visible`]，否则隐藏后只会留一块空白。
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            cx.notify();
        }
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    /// 重新跑一遍首屏读取（标题栏刷新按钮 / 引用选中变化都走这里）。
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.spawn_first_load(cx);
    }

    /// 起一次首屏读取：把当前游标交给后台任务去归还，重排状态，再 spawn。
    fn spawn_first_load(&mut self, cx: &mut Context<Self>) {
        let root = self.root.clone();
        let reference = self.selected_reference_full_name();
        let cursor = self.cursor.take();
        self.has_more = false;
        self.loading_more = false;
        // 已有数据时保持原状态（失败会切到 `Stale`，画横幅而不是把列表清空）。
        if self.commits.is_empty() {
            self.load_state = LoadState::Loading;
        }
        self.request_serial = self.request_serial.wrapping_add(1);
        let serial = self.request_serial;
        let this = cx.entity().downgrade();
        cx.notify();

        cx.spawn(async move |_this, cx| {
            let load = cx
                .background_spawn(async move { load_first(&root, reference, cursor) })
                .await;
            let _ = this.update(cx, |pane, cx| pane.apply_first(serial, load, cx));
        })
        .detach();
    }

    /// 把首屏结果写进面板。
    fn apply_first(&mut self, serial: u64, load: FirstLoad, cx: &mut Context<Self>) {
        if serial != self.request_serial {
            return;
        }

        self.references = load.references;
        if let Some(repository_root) = load.repository_root {
            self.repository_root = Some(repository_root);
        }
        if load.branch.is_some() {
            self.branch = load.branch;
        }

        if self.repository_root.is_none() {
            self.load_state = LoadState::NoRepository;
            self.commits = Vec::new();
            self.has_more = false;
            self.cursor = None;
            self.selected_commit = None;
            self.selected_reference = None;
            self.detail = None;
            self.files = Vec::new();
            self.files_state = FilesState::Idle;
            cx.notify();
            return;
        }

        let cursor_root = self.repository_root.clone().unwrap_or_default();
        if load.failures.is_empty() {
            self.commits = load.commits;
            self.cursor = load.next_cursor.map(|cursor| HistoryCursor {
                root: cursor_root,
                cursor,
            });
            self.has_more = load.has_more;
            self.load_state = LoadState::Ready;
        } else if self.commits.is_empty() {
            self.load_state = LoadState::Failed;
            self.has_more = false;
            self.cursor = None;
        } else {
            // 有旧数据 → 横幅（`git-log-tool-window.tsx:589-600`），列表保持不动。
            self.load_state = LoadState::Stale;
            self.has_more = false;
            self.cursor = None;
        }

        // 换仓库 / 换引用后旧的选中下标可能越界，统一清掉再选第一条。
        self.selected_commit = None;
        self.selected_reference = self
            .selected_reference
            .filter(|index| *index < self.references.len());
        cx.notify();

        if !self.commits.is_empty() {
            self.select_commit(0, cx);
        }
    }

    /// 追加一页（「加载更多提交」）。
    fn load_more(&mut self, cx: &mut Context<Self>) {
        let Some(cursor_text) = self.cursor.as_ref().map(|cursor| cursor.cursor.clone()) else {
            return;
        };
        let Some(root) = self.repository_root.clone() else {
            return;
        };
        if self.loading_more {
            return;
        }

        let reference = self.selected_reference_full_name();
        self.loading_more = true;
        let this = cx.entity().downgrade();
        cx.notify();

        cx.spawn(async move |_this, cx| {
            let more = cx
                .background_spawn(async move { load_more(&root, reference, &cursor_text) })
                .await;
            let _ = this.update(cx, |pane, cx| pane.apply_more(more, cx));
        })
        .detach();
    }

    fn apply_more(&mut self, more: MoreLoad, cx: &mut Context<Self>) {
        self.loading_more = false;
        match more.failure {
            // 失败时 Core 已经停掉 session（`git/history.rs:258-261`），游标作废。
            Some(_) => {
                self.has_more = false;
                self.cursor = None;
            }
            None => {
                let cursor_root = self.repository_root.clone().unwrap_or_default();
                self.commits.extend(more.commits);
                self.cursor = more.next_cursor.map(|cursor| HistoryCursor {
                    root: cursor_root,
                    cursor,
                });
                self.has_more = more.has_more;
            }
        }
        cx.notify();
    }

    /// 选中一行提交：填详情 + 后台读 `git.commitFiles`。
    /// Windows 的选中会同时驱动 Inspector（`git-log-tool-window.tsx:685-704`）。
    fn select_commit(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(commit) = self.commits.get(index).cloned() else {
            return;
        };

        self.selected_commit = Some(index);
        self.detail = Some(Detail {
            subject: commit.subject.clone(),
            short_hash: commit.short_hash.clone(),
            author: commit.author.clone(),
            email: commit.email.clone(),
            date: commit.date.clone(),
            decorations: SharedString::from(
                commit
                    .labels
                    .iter()
                    .map(|label| label.title.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
            ),
            hash: commit.hash.clone(),
        });
        self.files = Vec::new();
        self.files_state = FilesState::Loading;
        cx.notify();

        let root = self
            .repository_root
            .clone()
            .unwrap_or_else(|| self.root.to_string_lossy().to_string());
        let hash = commit.hash.to_string();
        let this = cx.entity().downgrade();

        cx.spawn(async move |_this, cx| {
            let loaded = cx
                .background_spawn(async move { load_commit_files(&root, &hash) })
                .await;
            let _ = this.update(cx, |pane, cx| pane.apply_commit_files(loaded, cx));
        })
        .detach();
    }

    fn apply_commit_files(
        &mut self,
        loaded: Result<Vec<(String, String)>, String>,
        cx: &mut Context<Self>,
    ) {
        match loaded {
            Ok(files) => {
                self.files = build_commit_files(&files);
                self.files_state = FilesState::Ready;
            }
            Err(_) => {
                self.files = Vec::new();
                self.files_state = FilesState::Failed;
            }
        }
        cx.notify();
    }

    /// 选中/清空引用。选中后按该引用重查历史（Windows `selectReference` + 刷新）。
    fn select_reference(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        if self.selected_reference == index {
            return;
        }
        self.selected_reference = index;
        self.selected_commit = None;
        self.detail = None;
        self.files = Vec::new();
        self.files_state = FilesState::Idle;
        self.spawn_first_load(cx);
    }

    fn selected_reference_full_name(&self) -> Option<String> {
        self.selected_reference
            .and_then(|index| self.references.get(index))
            .map(|reference| reference.full_name.to_string())
    }

    fn toggle_section(&mut self, kind: RefKind, cx: &mut Context<Self>) {
        if !self.collapsed_sections.remove(kind.id()) {
            self.collapsed_sections.insert(kind.id());
        }
        cx.notify();
    }

    fn toggle_group(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.collapsed_groups.remove(id) {
            self.collapsed_groups.insert(id.to_string());
        }
        cx.notify();
    }

    /// 可见的提交下标（筛选之后）。Windows 的 `visibleRows`（`git-commit-table.tsx:105-108`）。
    fn visible_commits(&self) -> Vec<usize> {
        self.commits
            .iter()
            .enumerate()
            .filter(|(_, commit)| matches_filter(commit, &self.filter_query, self.filter_scope))
            .map(|(index, _)| index)
            .collect()
    }

    /// 可见引用：`showMyBranchesOnly` 只保留当前分支，其余原样
    /// （`filterGitLogReferences`，`git-reference-tree.tsx:756-762`）。
    fn visible_reference(&self, index: usize) -> bool {
        if !self.show_my_branches_only {
            return true;
        }
        self.references
            .get(index)
            .is_some_and(|reference| reference.kind == RefKind::Local && reference.is_current)
    }

    // -----------------------------------------------------------------------
    // 自绘基元
    // -----------------------------------------------------------------------

    /// 一个自绘图标按钮：24×24 命中区、14px 图标、悬停 `accent` 底。
    /// 偏差 5：不用 `Button`（它的图标会被算成 18px），因此也没有悬停 tooltip，只有 `aria_label`。
    fn icon_button(
        id: (&'static str, usize),
        icon: IconName,
        label: &'static str,
        enabled: bool,
        handler: ButtonHandler,
        cx: &App,
    ) -> impl IntoElement {
        let color = if enabled {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };

        div()
            .id(id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(px(ICON_BUTTON_SIZE))
            .rounded(px(ICON_BUTTON_RADIUS))
            .aria_label(label)
            .when(enabled, |this| {
                this.hover(|style| style.bg(cx.theme().accent))
                    .on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when(!enabled, |this| this.opacity(0.4))
            .child(
                Icon::new(icon)
                    .size(px(ICON_BUTTON_ICON_SIZE))
                    .text_color(color),
            )
    }

    /// 引用树工具栏按钮：32×32、图标 16、圆角 4.8（`git-reference-tree.tsx:146`）。
    fn toolbar_button(
        id: (&'static str, usize),
        icon: IconName,
        label: &'static str,
        enabled: bool,
        handler: ButtonHandler,
        cx: &App,
    ) -> impl IntoElement {
        let color = if enabled {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };
        div()
            .id(id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(px(REFERENCE_TOOLBAR_BUTTON_SIZE))
            .rounded(px(REFERENCE_TOOLBAR_BUTTON_RADIUS))
            .aria_label(label)
            .when(enabled, |this| {
                this.hover(|style| style.bg(cx.theme().accent))
                    .on_click(move |event, window, cx| handler(event, window, cx))
            })
            .when(!enabled, |this| this.opacity(0.3))
            .child(
                Icon::new(icon)
                    .size(px(REFERENCE_TOOLBAR_ICON_SIZE))
                    .text_color(color),
            )
    }

    /// 标题栏。`git-log-title-bar.tsx:28-72`。
    fn title_bar(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        // 引用名：选中引用 → 该引用短名；没有选中但 `git.status` 给了分支 → 分支名；
        // 两者都没有才是 `全部`（`git.log.all`，`locale.ts:7188`）。
        let reference_name = self
            .selected_reference
            .and_then(|index| self.references.get(index))
            .map(|reference| reference.short_name.clone())
            .or_else(|| self.branch.clone())
            .unwrap_or_else(|| SharedString::from("全部"));
        // `日志：{name}`（`git.log.logLabel`，`locale.ts:7189`）。
        let pill_label = SharedString::from(format!("日志：{reference_name}"));

        let show_all: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| pane.select_reference(None, cx));
            })
        };
        let refresh: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| pane.refresh(cx));
            })
        };
        let hide: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| pane.set_visible(false, cx));
            })
        };
        let settings: ButtonHandler = handler(|_event, _window, _cx| {});

        h_flex()
            .w_full()
            .flex_shrink_0()
            .h(px(TITLE_BAR_HEIGHT))
            .items_center()
            .gap(px(TITLE_BAR_GAP))
            .px(px(TITLE_BAR_PADDING_X))
            .border_b_1()
            .border_color(cx.theme().border)
            // 底色 `bg-surface`（`git-log-title-bar.tsx:28`）；主题里最接近的是 `tab_bar`。
            .bg(cx.theme().tab_bar)
            .text_size(px(TITLE_FONT_SIZE))
            .child(
                Icon::new(IconName::GitBranch)
                    .size(px(TITLE_ICON_SIZE))
                    .text_color(cx.theme().muted_foreground),
            )
            // `workbench.gitLog` = 提交记录（`locale.ts:5910`）。
            .child(div().font_weight(FontWeight::MEDIUM).child("提交记录"))
            // 「引用名」胶囊：点击 = 显示全部引用（`git.log.showAll`，`locale.ts:7190`）。
            .child(
                div()
                    .id("bottom-git-reference-pill")
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .h(px(REFERENCE_PILL_HEIGHT))
                    .max_w(px(REFERENCE_PILL_MAX_WIDTH))
                    .px(px(REFERENCE_PILL_PADDING_X))
                    .rounded(px(REFERENCE_PILL_RADIUS))
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|style| style.bg(cx.theme().accent))
                    .aria_label("显示全部引用")
                    .on_click(move |event, window, cx| show_all(event, window, cx))
                    .child(div().min_w_0().text_ellipsis().child(pill_label)),
            )
            // 刷新（`git.log.refresh`，`locale.ts:7191`）。
            .child(Self::icon_button(
                ("bottom-git-refresh", 0),
                IconName::RotateCw,
                "刷新 Git 日志",
                self.load_state != LoadState::Loading,
                refresh,
                cx,
            ))
            // 设置（`git.log.settings`，`locale.ts:7219`）：设置面板不属本步范围 → 禁用。
            .child(Self::icon_button(
                ("bottom-git-settings", 0),
                IconName::Settings,
                "打开 Git 日志设置",
                false,
                settings,
                cx,
            ))
            .child(div().flex_1())
            // `footer.readOnly` = 只读（`locale.ts:7309`）。
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(cx.theme().muted_foreground)
                    .child("只读"),
            )
            // 隐藏（`git.log.hide`，`locale.ts:7192`）。
            .child(Self::icon_button(
                ("bottom-git-hide", 0),
                IconName::Minus,
                "隐藏提交记录",
                true,
                hide,
                cx,
            ))
    }

    /// 页签行。`git-log-tool-window.tsx:582-587`（12px、gap 16、px 12、py 4，**没有**选中底色）。
    fn tab_row(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let mut row = h_flex()
            .w_full()
            .flex_shrink_0()
            .h(px(TAB_ROW_HEIGHT))
            .items_center()
            .gap(px(TAB_ROW_GAP))
            .px(px(TAB_ROW_PADDING_X))
            .border_b_1()
            .border_color(cx.theme().border)
            .text_size(px(TAB_ROW_FONT_SIZE));

        // `git.console.log` = 日志（`locale.ts:4556`）、`git.console.title` = 控制台（`locale.ts:4555`）。
        for (index, (panel, label)) in [
            (Panel::Log, SharedString::from("日志")),
            (Panel::Console, SharedString::from("控制台")),
        ]
        .into_iter()
        .enumerate()
        {
            let selected = self.panel == panel;
            let this = this.clone();
            row = row.child(
                div()
                    .id(("bottom-git-tab", index))
                    .flex_shrink_0()
                    .aria_selected(selected)
                    // 偏差 4：源码没有选中视觉，这里只补前景色区分。
                    .text_color(if selected {
                        cx.theme().foreground
                    } else {
                        cx.theme().muted_foreground
                    })
                    .hover(|style| style.text_color(cx.theme().foreground))
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = this.update(cx, |pane, cx| {
                            if pane.panel != panel {
                                pane.panel = panel;
                                cx.notify();
                            }
                        });
                    })
                    .child(label),
            );
        }

        row
    }

    /// 字段下拉按钮。`git-commit-table.tsx:229-238`（h 24、圆角 6.4、px 6）。
    ///
    /// 不用 `cx`：外观全部来自 `Button` 自己的主题样式（所以这里收不到 `&App`）。
    fn scope_button(&self, this: &WeakEntity<Self>) -> impl IntoElement {
        let current = self.filter_scope;
        let this = this.clone();

        Button::new("bottom-git-filter-field")
            .ghost()
            .with_size(px(ICON_BUTTON_SIZE))
            .h(px(ICON_BUTTON_SIZE))
            .px(px(FIELD_SELECT_PADDING_X))
            .label(current.label())
            .tooltip("Git 日志筛选字段")
            .dropdown_menu(move |menu, _window, _cx| {
                let mut menu = menu;
                for scope in FilterScope::all() {
                    let this = this.clone();
                    menu = menu.item(
                        PopupMenuItem::new(scope.label())
                            .checked(scope == current)
                            .on_click(move |_event, window: &mut Window, cx: &mut App| {
                                let _ = this.update(cx, |pane, cx| {
                                    pane.filter_scope = scope;
                                    // 占位文案跟着字段变（`git.log.filterPlaceholder`，`locale.ts:7282`）。
                                    let placeholder = scope.placeholder();
                                    pane.filter.update(cx, |state, cx| {
                                        state.set_placeholder(placeholder, window, cx)
                                    });
                                    cx.notify();
                                });
                            }),
                    );
                }
                menu
            })
    }

    /// 筛选条。`git-commit-table.tsx:200-253`。
    fn filter_row(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let visible = self.visible_commits().len();
        let total = self.commits.len();

        // 装饰开关（`git.log.showDecorations` / `hideDecorations`，`locale.ts:7283-7284`）。
        let toggle_decorations: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| {
                    pane.show_decorations = !pane.show_decorations;
                    cx.notify();
                });
            })
        };

        h_flex()
            .w_full()
            .flex_shrink_0()
            .h(px(FILTER_ROW_HEIGHT))
            .items_center()
            .gap(px(FILTER_ROW_GAP))
            .px(px(FILTER_ROW_PADDING_X))
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .child(
                Input::new(&self.filter)
                    .cleanable(true)
                    .small()
                    .flex_1()
                    .min_w(px(FILTER_INPUT_MIN_WIDTH))
                    .max_w(px(FILTER_INPUT_MAX_WIDTH))
                    .prefix(
                        Icon::new(IconName::Search)
                            .size(px(ICON_BUTTON_ICON_SIZE))
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            .child(self.scope_button(this))
            .child(Self::icon_button(
                ("bottom-git-decorations", 0),
                if self.show_decorations {
                    IconName::Eye
                } else {
                    IconName::EyeOff
                },
                if self.show_decorations {
                    "隐藏分支和标签"
                } else {
                    "显示分支和标签"
                },
                true,
                toggle_decorations,
                cx,
            ))
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(cx.theme().muted_foreground)
                    .child(SharedString::from(format!("{visible}/{total}"))),
            )
    }

    /// 表头行：`提交 / 作者 / 日期`。出处：`git-commit-table.tsx:255-259`。
    fn commit_header(cx: &App) -> Div {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .h(px(COMMIT_HEADER_HEIGHT))
            .items_center()
            .px(px(FILTER_ROW_PADDING_X))
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .text_size(px(COMMIT_FONT_SIZE))
            .text_color(cx.theme().muted_foreground)
            // `git.log.commit` / `author` / `date`（`locale.ts:7285-7287`）。
            // 表头「提交」只占 `flex-1`，**没有**预留泳道图宽度（源码就是不对齐的，
            // 见 `research/windows/03-git-and-bottom.md` §2.3 的注）。
            .child(div().min_w_0().flex_1().child("提交"))
            .child(div().w(px(AUTHOR_COLUMN_WIDTH)).flex_shrink_0().child("作者"))
            .child(
                h_flex()
                    .w(px(DATE_COLUMN_WIDTH))
                    .flex_shrink_0()
                    .justify_end()
                    .child("日期"),
            )
    }

    /// 泳道图单元格（简化版，偏差 2）。
    fn graph_cell(row: &GraphRow, cx: &App) -> Div {
        let half = COMMIT_ROW_HEIGHT / 2.;

        h_flex()
            .h_full()
            .flex_shrink_0()
            .min_w(px(GRAPH_MIN_WIDTH))
            .pl(px(GRAPH_PADDING))
            .pr(px(GRAPH_PADDING))
            .children(row.lanes.iter().enumerate().map(move |(lane, color)| {
                let lane_width = px(GRAPH_LANE_GAP);
                let line_width = px(GRAPH_LINE_WIDTH);
                let edge_here = row.edges.iter().find(|(target, _, _)| *target == lane);

                match (lane == row.lane, color) {
                    // 本提交所在泳道：上半段线 + 节点 + 下半段线。
                    (true, _) => v_flex()
                        .w(lane_width)
                        .h_full()
                        .items_center()
                        .child(
                            div()
                                .w(line_width)
                                .h(px(half - GRAPH_NODE_RADIUS))
                                .bg(lane_color(row.node_color, cx)),
                        )
                        .child(
                            div()
                                .size(px(GRAPH_NODE_RADIUS * 2.))
                                .flex_shrink_0()
                                .rounded_full()
                                .bg(cx.theme().background)
                                .border(px(GRAPH_NODE_STROKE))
                                .border_color(lane_color(row.node_color, cx)),
                        )
                        .child(
                            div()
                                .w(line_width)
                                .flex_1()
                                .when(!row.edges.is_empty(), |this| {
                                    this.bg(lane_color(row.node_color, cx))
                                }),
                        )
                        .into_any_element(),
                    // 过路线：整条竖线。
                    (false, Some(index)) => v_flex()
                        .w(lane_width)
                        .h_full()
                        .items_center()
                        .child(div().w(line_width).h_full().bg(lane_color(*index, cx)))
                        .into_any_element(),
                    // 只被父边指到、本行上半没有线的泳道：补下半段。
                    (false, None) => v_flex()
                        .w(lane_width)
                        .h_full()
                        .items_center()
                        .child(div().flex_1())
                        .when_some(edge_here, |this, (_, index, missing)| {
                            this.child(
                                div()
                                    .w(line_width)
                                    .h(px(half))
                                    .opacity(if *missing { 0.7 } else { 1.0 })
                                    .bg(lane_color(*index, cx)),
                            )
                        })
                        .into_any_element(),
                }
            }))
    }

    /// 一个标签徽章（`git-graph-row.tsx:87-88`）。
    fn label_badge(label: &Label, cx: &App) -> Div {
        let color = label_color(label.kind, cx);
        h_flex()
            .h(px(LABEL_FONT_SIZE + LABEL_PADDING_Y * 2.))
            .max_w(px(LABEL_MAX_WIDTH))
            .flex_shrink_0()
            .items_center()
            .px(px(LABEL_PADDING_X))
            .rounded(px(LABEL_RADIUS))
            .border_1()
            .border_color(color.opacity(0.45))
            .bg(color.opacity(0.2))
            .text_size(px(LABEL_FONT_SIZE))
            .text_color(color)
            .whitespace_nowrap()
            .child(div().min_w_0().text_ellipsis().child(label.title.clone()))
    }

    /// 一行提交（自绘，`git-commit-table.tsx:301-326`）。
    ///
    /// ⚠️ **不要给提交信息加 `overflow_hidden`**（`gpui/BLOCKERS.md` B4：提交信息文字顶部被切掉约
    /// 1/4 行高）。这里只用 `text_ellipsis()` + `min_h`，与上一轮实现的定案一致。
    fn commit_row(
        &self,
        index: usize,
        graph: &GraphRow,
        commit: &Commit,
        selected: bool,
        this: &WeakEntity<Self>,
        cx: &App,
    ) -> impl IntoElement {
        let hover_bg = if selected {
            cx.theme().primary.opacity(0.28)
        } else {
            cx.theme().accent.opacity(0.7)
        };
        let this = this.clone();

        h_flex()
            .id(("bottom-git-commit", index))
            .w_full()
            .min_w(px(COMMIT_CONTENT_MIN_WIDTH))
            .min_h(px(COMMIT_ROW_HEIGHT))
            .items_center()
            .px(px(COMMIT_ROW_PADDING_X))
            .border_b_1()
            .border_color(cx.theme().border.opacity(0.5))
            .whitespace_nowrap()
            .when(selected, |row| row.bg(cx.theme().primary.opacity(0.22)))
            .hover(move |style| style.bg(hover_bg))
            .on_click(move |_event, _window, cx: &mut App| {
                let _ = this.update(cx, |pane, cx| pane.select_commit(index, cx));
            })
            .child(Self::graph_cell(graph, cx))
            // 标签 + 提交说明：`flex min-w-0 flex-1 items-center gap-1.5`。
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap(px(LABEL_GAP))
                    .when(self.show_decorations, |labels| {
                        labels.children(
                            commit
                                .labels
                                .iter()
                                .map(|label| Self::label_badge(label, cx)),
                        )
                    })
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .min_h(px(COMMIT_ROW_HEIGHT))
                            .text_size(px(COMMIT_FONT_SIZE))
                            .line_height(px(COMMIT_ROW_HEIGHT))
                            .child(commit.subject.clone()),
                    ),
            )
            .child(
                div()
                    .w(px(AUTHOR_COLUMN_WIDTH))
                    .flex_shrink_0()
                    .px(px(AUTHOR_COLUMN_PADDING_X))
                    .text_ellipsis()
                    .text_size(px(COMMIT_FONT_SIZE))
                    .text_color(cx.theme().muted_foreground)
                    .child(commit.author.clone()),
            )
            .child(
                h_flex()
                    .w(px(DATE_COLUMN_WIDTH))
                    .flex_shrink_0()
                    .justify_end()
                    .text_size(px(DATE_FONT_SIZE))
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_color(cx.theme().muted_foreground)
                    .child(commit.date.clone()),
            )
    }

    /// 提交栏。`git-commit-table.tsx:198-446`。
    fn commit_pane(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let visible = self.visible_commits();
        let total = self.commits.len();
        let graphs = layout_graph(&self.commits);

        let mut list = v_flex().w_full().min_w(px(COMMIT_CONTENT_MIN_WIDTH));

        if visible.is_empty() {
            let message = if total == 0 {
                // `git.log.noCommits` = 此视图中没有提交（`locale.ts:7289`）。
                SharedString::from("此视图中没有提交")
            } else {
                // `git.log.noMatch` = 没有符合筛选条件的提交（`locale.ts:7288`）。
                SharedString::from("没有符合筛选条件的提交")
            };
            list = list.child(
                h_flex()
                    .w_full()
                    .min_h(px(COMMIT_ROW_HEIGHT * 4.))
                    .items_center()
                    .justify_center()
                    .text_color(cx.theme().muted_foreground)
                    .child(message),
            );
        } else {
            let empty_graph = GraphRow {
                lanes: Vec::new(),
                lane: 0,
                node_color: 0,
                edges: Vec::new(),
            };
            for index in visible {
                let Some(commit) = self.commits.get(index) else {
                    continue;
                };
                let graph = graphs.get(index).unwrap_or(&empty_graph);
                list = list.child(self.commit_row(
                    index,
                    graph,
                    commit,
                    self.selected_commit == Some(index),
                    this,
                    cx,
                ));
            }
        }

        // 「加载更多提交」行（`git-commit-table.tsx:428-443`）。
        if self.has_more {
            let this = this.clone();
            list = list.child(
                h_flex()
                    .w_full()
                    .min_w(px(COMMIT_CONTENT_MIN_WIDTH))
                    .h(px(LOAD_MORE_HEIGHT))
                    .flex_shrink_0()
                    .items_center()
                    .justify_center()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("bottom-git-load-more")
                            .ghost()
                            .with_size(px(LOAD_MORE_BUTTON_HEIGHT))
                            .label(if self.loading_more {
                                // `git.log.loadingCommits` = 正在加载提交…（`locale.ts:7294`）。
                                SharedString::from("正在加载提交…")
                            } else {
                                // `git.log.loadMore` = 加载更多提交（`locale.ts:7295`）。
                                SharedString::from("加载更多提交")
                            })
                            .disabled(self.loading_more)
                            .on_click(move |_event, _window, cx: &mut App| {
                                let _ = this.update(cx, |pane, cx| pane.load_more(cx));
                            }),
                    ),
            );
        }

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_size(px(COMMIT_FONT_SIZE))
            .child(self.filter_row(this, cx))
            .child(Self::commit_header(cx))
            .child(
                // 滚动容器必须有 `id`：`overflow_y_scroll` 是 `StatefulInteractiveElement` 的方法
                // （`gpui-pre-0.3.6/src/elements/div.rs:1300,1529`），而 `StatefulInteractiveElement`
                // 只对 `Stateful<Div>` 实现（`:4074`）；`id` 同时让滚动偏移跨帧保留。
                div()
                    .id("bottom-git-commit-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .w_full()
                    .overflow_y_scroll()
                    .child(list),
            )
    }

    /// 引用树左侧竖排工具栏。`git-reference-tree.tsx:118-361`。
    ///
    /// 偏差 8：Windows 的 10 个动作里只有「全部展开 / 全部折叠 / 只显示我的分支」是纯 UI，
    /// 这里只画这三个（另一个「新建分支」按禁用态保留位置）。
    fn reference_toolbar(
        pane: &Self,
        this: &WeakEntity<Self>,
        cx: &App,
    ) -> impl IntoElement {
        let has_references = !pane.references.is_empty();
        let has_my_branches = pane
            .references
            .iter()
            .any(|reference| reference.kind == RefKind::Local && reference.is_current);
        let show_my_branches_only = pane.show_my_branches_only;

        let expand: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| {
                    pane.collapsed_sections.clear();
                    pane.collapsed_groups.clear();
                    cx.notify();
                });
            })
        };
        let collapse: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| {
                    let mut groups = Vec::new();
                    for kind in RefKind::sections() {
                        pane.collapsed_sections.insert(kind.id());
                        for row in build_reference_rows(&pane.references, kind) {
                            if row.reference.is_none() {
                                groups.push(row.id);
                            }
                        }
                    }
                    pane.collapsed_groups.extend(groups);
                    cx.notify();
                });
            })
        };
        let toggle_mine: ButtonHandler = {
            let this = this.clone();
            handler(move |_event, _window, cx| {
                let _ = this.update(cx, |pane, cx| {
                    pane.show_my_branches_only = !pane.show_my_branches_only;
                    cx.notify();
                });
            })
        };
        let create_branch: ButtonHandler = handler(|_event, _window, _cx| {});

        v_flex()
            .w(px(REFERENCE_TOOLBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .items_center()
            .gap(px(REFERENCE_TOOLBAR_GAP))
            .py(px(REFERENCE_TOOLBAR_PADDING_Y))
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar.opacity(0.6))
            .child(Self::toolbar_button(
                ("bottom-git-ref-expand", 0),
                IconName::UnfoldVertical,
                "全部展开",
                has_references,
                expand,
                cx,
            ))
            .child(Self::toolbar_button(
                ("bottom-git-ref-collapse", 0),
                IconName::FoldVertical,
                "全部折叠",
                has_references,
                collapse,
                cx,
            ))
            .child(
                div()
                    .w(px(REFERENCE_TOOLBAR_SEPARATOR_WIDTH))
                    .h(px(REFERENCE_TOOLBAR_SEPARATOR_HEIGHT))
                    .flex_shrink_0()
                    .my(px(REFERENCE_TOOLBAR_SEPARATOR_MARGIN_Y))
                    .bg(cx.theme().border),
            )
            .child(Self::toolbar_button(
                ("bottom-git-ref-mine", 0),
                IconName::ListFilter,
                if show_my_branches_only {
                    "显示全部分支"
                } else {
                    "显示我的分支"
                },
                has_my_branches || show_my_branches_only,
                toggle_mine,
                cx,
            ))
            .child(div().flex_1())
            // 新建分支要 `git.write`（不在 6 条命令里）→ 禁用占位。
            .child(Self::toolbar_button(
                ("bottom-git-ref-create", 0),
                IconName::Plus,
                "新建分支",
                false,
                create_branch,
                cx,
            ))
    }

    /// 引用树的一行（`git-reference-tree.tsx:596-660`）。
    #[allow(clippy::too_many_arguments)]
    fn reference_row(
        row_id: String,
        depth: usize,
        name: SharedString,
        is_group: bool,
        selected: bool,
        disclosure: Option<AnyElement>,
        icon: Option<IconName>,
        reference: Option<(usize, Option<(usize, usize)>, bool)>,
        this: &WeakEntity<Self>,
        cx: &App,
    ) -> impl IntoElement {
        let this = this.clone();
        let index = reference.map(|(index, _, _)| index);
        let tracking = reference.and_then(|(_, tracking, _)| tracking);
        let is_current = reference.is_some_and(|(_, _, current)| current);

        let row = h_flex()
            .id(SharedString::from(format!("bottom-git-ref-row:{row_id}")))
            .w_full()
            .min_w_0()
            .h(px(REFERENCE_ROW_HEIGHT))
            .items_center()
            .gap(px(REFERENCE_ROW_GAP))
            .pl(px(REFERENCE_INDENT_BASE + depth as f32 * REFERENCE_INDENT_STEP))
            .rounded(px(REFERENCE_ROW_RADIUS))
            .when(selected, |row| row.bg(cx.theme().accent))
            .when(is_current, |row| {
                row.font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().yellow_light)
            })
            .hover(|style| style.bg(cx.theme().accent.opacity(0.8)))
            .when_some(index, |row, index| {
                row.on_click(move |_event, _window, cx: &mut App| {
                    let _ = this.update(cx, |pane, cx| pane.select_reference(Some(index), cx));
                })
            });

        let row = match disclosure {
            Some(disclosure) => row.child(disclosure),
            None => row.child(div().size(px(REFERENCE_DISCLOSURE_SIZE)).flex_shrink_0()),
        };

        let row = match icon {
            Some(icon) => row.child(
                Icon::new(icon)
                    .size(px(REFERENCE_ICON_SIZE))
                    .text_color(if is_current {
                        cx.theme().yellow_light
                    } else {
                        cx.theme().muted_foreground
                    }),
            ),
            None if is_group => row.child(
                Icon::new(IconName::Folder)
                    .size(px(REFERENCE_ICON_SIZE))
                    .text_color(cx.theme().muted_foreground),
            ),
            None => row,
        };

        let mut row = row
            .child(div().min_w_0().text_ellipsis().child(name))
            .child(div().flex_1());

        // ahead / behind 计数（`git-reference-tree.tsx:641-650`、`git-tracking-counts.tsx:36-53`）。
        if let Some((ahead, behind)) = tracking {
            let mut counts = h_flex()
                .flex_shrink_0()
                .items_center()
                .gap(px(TRACKING_COUNT_GAP))
                .text_size(px(TRACKING_COUNT_FONT_SIZE));
            if behind > 0 {
                counts = counts.child(
                    div()
                        .text_color(cx.theme().info)
                        .child(SharedString::from(format!("↙{}", tracking_count(behind)))),
                );
            }
            if ahead > 0 {
                counts = counts.child(
                    div()
                        .text_color(cx.theme().success)
                        .child(SharedString::from(format!("↗{}", tracking_count(ahead)))),
                );
            }
            row = row.child(counts);
        }

        if is_current {
            // `git.current` = 当前（`locale.ts:6948`）。
            row = row.child(
                h_flex()
                    .flex_shrink_0()
                    .px(px(REFERENCE_BADGE_PADDING_X))
                    .rounded(px(REFERENCE_SECTION_RADIUS))
                    .bg(cx.theme().yellow_light.opacity(0.12))
                    .text_size(px(REFERENCE_BADGE_FONT_SIZE))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().yellow_light)
                    .child("当前"),
            );
        }

        row
    }

    /// 引用栏。`git-reference-tree.tsx:803-928`。
    fn reference_pane(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let current = self
            .references
            .iter()
            .find(|reference| reference.is_current)
            .cloned();
        let visible_count = self
            .references
            .iter()
            .enumerate()
            .filter(|(index, _)| self.visible_reference(*index))
            .count();

        let mut body = v_flex().w_full().gap(px(REFERENCE_SECTION_MARGIN_BOTTOM));

        // HEAD 行（`git-reference-tree.tsx:849-865`）。
        let current_index = self
            .references
            .iter()
            .position(|reference| reference.is_current);
        let head_selected = self.selected_reference.is_some() && self.selected_reference == current_index;
        let head = {
            let this = this.clone();
            h_flex()
                .id("bottom-git-head-row")
                .w_full()
                .h(px(REFERENCE_HEAD_ROW_HEIGHT))
                .mb(px(REFERENCE_HEAD_ROW_MARGIN_BOTTOM))
                .items_center()
                .gap(px(REFERENCE_HEAD_ROW_GAP))
                .px(px(REFERENCE_HEAD_ROW_PADDING_X))
                .rounded(px(REFERENCE_HEAD_ROW_RADIUS))
                .font_weight(FontWeight::MEDIUM)
                .when(head_selected, |row| row.bg(cx.theme().accent))
                .hover(|style| style.bg(cx.theme().accent.opacity(0.8)))
                .on_click(move |_event, _window, cx: &mut App| {
                    let _ = this.update(cx, |pane, cx| pane.select_reference(current_index, cx));
                })
                .child(div().text_color(cx.theme().primary).child("→"))
                // `git.log.headCurrentBranch` = HEAD（当前分支）（`locale.ts:7220`）。
                .child(div().min_w_0().text_ellipsis().child("HEAD（当前分支）"))
                .when_some(current, |row, reference| {
                    row.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_ellipsis()
                            .text_color(cx.theme().muted_foreground)
                            .child(reference.short_name),
                    )
                })
        };
        body = body.child(head);

        // 三个分区（本地 / 远程 / 标签）。
        for kind in RefKind::sections() {
            let collapsed = self.collapsed_sections.contains(kind.id());
            let rows = build_reference_rows(&self.references, kind);

            let mut section = v_flex().w_full();
            let header = {
                let this = this.clone();
                h_flex()
                    .id(("bottom-git-ref-section", kind as usize))
                    .w_full()
                    .h(px(REFERENCE_SECTION_ROW_HEIGHT))
                    .items_center()
                    .gap(px(REFERENCE_ROW_GAP))
                    .px(px(REFERENCE_SECTION_PADDING_X))
                    .rounded(px(REFERENCE_SECTION_RADIUS))
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|style| style.bg(cx.theme().accent.opacity(0.8)))
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = this.update(cx, |pane, cx| pane.toggle_section(kind, cx));
                    })
                    .child(
                        Icon::new(if collapsed {
                            IconName::ChevronRight
                        } else {
                            IconName::ChevronDown
                        })
                        .size(px(12.))
                        .text_color(cx.theme().muted_foreground),
                    )
                    .child(kind.title())
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_color(cx.theme().muted_foreground)
                            .child(SharedString::from(
                                self.references
                                    .iter()
                                    .enumerate()
                                    .filter(|(index, reference)| {
                                        reference.kind == kind && self.visible_reference(*index)
                                    })
                                    .count()
                                    .to_string(),
                            )),
                    )
            };
            section = section.child(header);

            if !collapsed {
                if rows.is_empty() {
                    // `git.log.none` = 无（`locale.ts:7255`）。
                    section = section.child(
                        div()
                            .h(px(REFERENCE_EMPTY_HEIGHT))
                            .pl(px(REFERENCE_EMPTY_PADDING_LEFT))
                            .line_height(px(REFERENCE_EMPTY_HEIGHT))
                            .text_color(cx.theme().muted_foreground)
                            .child("无"),
                    );
                } else {
                    for row in rows {
                        if let Some(index) = row.reference {
                            if !self.visible_reference(index) {
                                continue;
                            }
                        }
                        let is_group = row.reference.is_none();
                        let selected =
                            row.reference.is_some() && row.reference == self.selected_reference;
                        let reference = row
                            .reference
                            .and_then(|index| self.references.get(index))
                            .cloned();
                        let collapsed_group = self.collapsed_groups.contains(&row.id);

                        let disclosure = if is_group {
                            let this = this.clone();
                            let id = row.id.clone();
                            Some(
                                div()
                                    .id(SharedString::from(format!(
                                        "bottom-git-ref-group:{id}"
                                    )))
                                    .size(px(REFERENCE_DISCLOSURE_SIZE))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .on_click(move |_event, _window, cx: &mut App| {
                                        let id = id.clone();
                                        let _ = this
                                            .update(cx, |pane, cx| pane.toggle_group(&id, cx));
                                    })
                                    .child(
                                        Icon::new(if collapsed_group {
                                            IconName::ChevronRight
                                        } else {
                                            IconName::ChevronDown
                                        })
                                        .size(px(12.))
                                        .text_color(cx.theme().muted_foreground),
                                    )
                                    .into_any_element(),
                            )
                        } else {
                            None
                        };

                        if is_group && collapsed_group {
                            section = section.child(Self::reference_row(
                                row.id,
                                row.depth,
                                row.name,
                                true,
                                selected,
                                disclosure,
                                None,
                                None,
                                this,
                                cx,
                            ));
                            continue;
                        }
                        if is_group {
                            section = section.child(Self::reference_row(
                                row.id,
                                row.depth,
                                row.name,
                                true,
                                selected,
                                disclosure,
                                None,
                                None,
                                this,
                                cx,
                            ));
                            continue;
                        }

                        let reference_index = row.reference;
                        let is_current =
                            reference.as_ref().is_some_and(|reference| reference.is_current);
                        let tracking = reference.as_ref().and_then(|reference| {
                            reference
                                .upstream_short_name
                                .as_ref()
                                .map(|_| (reference.ahead, reference.behind))
                        });
                        let icon = if is_current {
                            IconName::Check
                        } else if let Some(reference) = reference.as_ref() {
                            match reference.kind {
                                RefKind::Tag => IconName::Tag,
                                RefKind::Remote => IconName::Network,
                                RefKind::Local => IconName::GitBranch,
                            }
                        } else {
                            IconName::GitBranch
                        };

                        section = section.child(Self::reference_row(
                            row.id,
                            row.depth,
                            row.name,
                            false,
                            selected,
                            None,
                            Some(icon),
                            reference_index.map(|index| (index, tracking, is_current)),
                            this,
                            cx,
                        ));
                    }
                }
            }

            body = body.child(section);
        }

        let reference_label = self
            .selected_reference
            .and_then(|index| self.references.get(index))
            .map(|reference| reference.short_name.clone());

        h_flex()
            .size_full()
            .bg(cx.theme().tab_bar.opacity(0.45))
            .text_size(px(COMMIT_FONT_SIZE))
            .child(Self::reference_toolbar(self, this, cx))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    // 列表头：`git.log.references` = 引用（`locale.ts:7207`）。
                    .child(
                        h_flex()
                            .w_full()
                            .flex_shrink_0()
                            .h(px(REFERENCE_HEADER_HEIGHT))
                            .items_center()
                            .px(px(FILTER_ROW_PADDING_X))
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .text_color(cx.theme().muted_foreground)
                            .child("引用")
                            .child(div().flex_1())
                            .when_some(reference_label, |row, label| {
                                row.child(div().min_w_0().text_ellipsis().child(label))
                            })
                            .child(SharedString::from(visible_count.to_string())),
                    )
                    .child(
                        div()
                            .id("bottom-git-reference-scroll")
                            .flex_1()
                            .min_h(px(0.))
                            .w_full()
                            .overflow_y_scroll()
                            .p(px(REFERENCE_LIST_PADDING))
                            .child(body),
                    ),
            )
    }

    /// Inspector。`git-commit-inspector.tsx:95-195`（上 62% 文件 / 下 38% 详情）。
    fn inspector_pane(&self, cx: &App) -> impl IntoElement {
        let file_count = self.files.iter().filter(|row| !row.is_folder).count();

        v_flex()
            .size_full()
            .bg(cx.theme().tab_bar.opacity(0.35))
            .text_size(px(COMMIT_FONT_SIZE))
            .child(
                v_flex()
                    .w_full()
                    .h(relative(INSPECTOR_FILES_FRACTION))
                    .min_h(px(INSPECTOR_FILES_MIN_HEIGHT))
                    .min_w_0()
                    // 文件区表头（`git-commit-inspector.tsx:106-130`）。
                    .child(
                        h_flex()
                            .w_full()
                            .flex_shrink_0()
                            .h(px(INSPECTOR_HEADER_HEIGHT))
                            .items_center()
                            .gap(px(INSPECTOR_HEADER_GAP))
                            .px(px(INSPECTOR_HEADER_PADDING_X))
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .bg(cx.theme().tab_bar)
                            .text_color(cx.theme().muted_foreground)
                            // `git.log.commitFiles` = 提交文件（`locale.ts:7296`）。
                            .child("提交文件")
                            .child(div().flex_1())
                            .child(if self.files_state == FilesState::Loading {
                                // `git.log.loadingShort` = 加载中…（`locale.ts:7303`）。
                                SharedString::from("加载中…")
                            } else {
                                // `git.log.filesCount` = {count} 个文件（`locale.ts:7297`）。
                                SharedString::from(format!("{file_count} 个文件"))
                            })
                            // 「打开提交差异」要 `git.diff`（不在 6 条命令里）→ 禁用。
                            .child(Self::icon_button(
                                ("bottom-git-open-diff", 0),
                                IconName::GitCompare,
                                "打开提交差异",
                                false,
                                handler(|_event, _window, _cx| {}),
                                cx,
                            )),
                    )
                    .child(self.commit_files_body(cx)),
            )
            .child(
                v_flex()
                    .id("bottom-git-detail-scroll")
                    .w_full()
                    .h(relative(INSPECTOR_DETAILS_FRACTION))
                    .min_h(px(INSPECTOR_DETAILS_MIN_HEIGHT))
                    .min_w_0()
                    .overflow_y_scroll()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().background)
                    .p(px(INSPECTOR_DETAIL_PADDING))
                    .gap(px(INSPECTOR_DETAIL_GAP))
                    .child(self.commit_detail_body(cx)),
            )
    }

    /// 提交文件区正文（空 / 加载 / 失败 / 无文件 / 文件树）。`git-commit-inspector.tsx:131-160`。
    fn commit_files_body(&self, cx: &App) -> impl IntoElement {
        let centered = |message: SharedString, color: Hsla| {
            h_flex()
                .w_full()
                .flex_1()
                .min_h(px(0.))
                .items_center()
                .justify_center()
                .text_color(color)
                .child(message)
                .into_any_element()
        };

        match self.files_state {
            // `git.log.selectCommit` = 选择一个提交（`locale.ts:7298`）。
            FilesState::Idle => centered(
                SharedString::from("选择一个提交"),
                cx.theme().muted_foreground,
            ),
            // `git.log.loadingChangedFiles` = 正在加载更改的文件…（`locale.ts:7299`）。
            FilesState::Loading => centered(
                SharedString::from("正在加载更改的文件…"),
                cx.theme().muted_foreground,
            ),
            // `git.log.unableToLoadFiles` = 无法加载更改的文件（`locale.ts:7300`）。
            FilesState::Failed => {
                centered(SharedString::from("无法加载更改的文件"), cx.theme().danger)
            }
            // `git.log.noChangedFiles` = 没有更改的文件（`locale.ts:7301`）。
            FilesState::Ready if self.files.is_empty() => centered(
                SharedString::from("没有更改的文件"),
                cx.theme().muted_foreground,
            ),
            FilesState::Ready => {
                let mut tree = v_flex().w_full();
                for row in &self.files {
                    tree = tree.child(Self::commit_file_row(row, cx));
                }
                div()
                    .id("bottom-git-files-scroll")
                    .flex_1()
                    .min_h(px(0.))
                    .w_full()
                    .overflow_y_scroll()
                    .p(px(COMMIT_FILE_TREE_PADDING))
                    .child(tree)
                    .into_any_element()
            }
        }
    }

    /// 提交文件树的一行。`git-commit-file-tree.tsx:120-134,77,127`。
    fn commit_file_row(row: &CommitFileRow, cx: &App) -> Div {
        let status = row.status.chars().next().unwrap_or(' ');
        let status_color = match status {
            'A' => cx.theme().success,
            'D' => cx.theme().danger,
            'R' => cx.theme().primary,
            _ => cx.theme().warning,
        };

        let row_element = h_flex()
            .w_full()
            .min_w_0()
            .h(px(COMMIT_FILE_ROW_HEIGHT))
            .items_center()
            .gap(px(REFERENCE_ROW_GAP))
            .pl(px(COMMIT_FILE_INDENT_BASE + row.depth as f32 * COMMIT_FILE_INDENT_STEP))
            .pr(px(COMMIT_FILE_TREE_PADDING))
            .whitespace_nowrap();

        if row.is_folder {
            row_element
                .child(
                    Icon::new(IconName::Folder)
                        .size(px(REFERENCE_ICON_SIZE))
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_ellipsis()
                        .child(row.name.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(px(COMMIT_FILE_STATUS_FONT_SIZE))
                        .text_color(cx.theme().muted_foreground)
                        .child(SharedString::from(format!("{} 个文件", row.file_count))),
                )
        } else {
            row_element
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_ellipsis()
                        .child(row.name.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_size(px(COMMIT_FILE_STATUS_FONT_SIZE))
                        .text_color(status_color)
                        // Core 的 `status` 是 name-status 码（可能是 `R100`），Windows 原样渲染
                        // （`git-commit-file-tree.tsx:127`）；这里取首字母，与 macOS 的
                        // `statusColor` 同一口径（`GitCommitFileTreeView.swift:459-464`）。
                        .child(SharedString::from(status.to_string())),
                )
        }
    }

    /// 提交详情区正文。`git-commit-inspector.tsx:165-190`。
    fn commit_detail_body(&self, cx: &App) -> impl IntoElement {
        let mono = cx.theme().mono_font_family.clone();

        let Some(detail) = self.detail.as_ref() else {
            // `git.log.commitDetails` = 提交详情（`locale.ts:7302`）。
            return h_flex()
                .w_full()
                .flex_1()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child("提交详情")
                .into_any_element();
        };

        v_flex()
            .w_full()
            .gap(px(INSPECTOR_DETAIL_GAP))
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .child(detail.subject.clone()),
            )
            .child(
                div()
                    .font_family(mono.clone())
                    .text_size(px(INSPECTOR_MONO_FONT_SIZE))
                    .text_color(cx.theme().muted_foreground)
                    .child(SharedString::from(if detail.email.is_empty() {
                        format!("{} · {}", detail.short_hash, detail.author)
                    } else {
                        format!(
                            "{} · {} <{}>",
                            detail.short_hash, detail.author, detail.email
                        )
                    })),
            )
            .child(
                div()
                    .font_family(mono.clone())
                    .text_size(px(INSPECTOR_MONO_FONT_SIZE))
                    .text_color(cx.theme().muted_foreground)
                    .child(detail.date.clone()),
            )
            .when(!detail.decorations.is_empty(), |this| {
                this.child(
                    div()
                        .text_color(cx.theme().primary)
                        .child(detail.decorations.clone()),
                )
            })
            // 完整哈希：Windows 是 `break-all`，gpui 没有这个属性，这里让它自然换行。
            .child(
                div()
                    .font_family(mono)
                    .text_size(px(INSPECTOR_HASH_FONT_SIZE))
                    .text_color(cx.theme().muted_foreground)
                    .child(detail.hash.clone()),
            )
            .into_any_element()
    }

    /// 控制台。`git-execution-console.tsx:68-111`。
    ///
    /// 偏差 6：输出数据源（Git 执行事件 + `git.consolePresentation`）不在本步的 6 条命令里，
    /// 所以左栏按钮全部禁用、正文只画空态。
    fn console_pane(cx: &App) -> impl IntoElement {
        let mut toolbar = v_flex()
            .w(px(CONSOLE_TOOLBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .items_center()
            .gap(px(CONSOLE_TOOLBAR_GAP))
            .py(px(CONSOLE_TOOLBAR_PADDING_Y))
            .border_r_1()
            .border_color(cx.theme().border);

        // 文案逐字取自 `git.console.find/wrap/scrollToEnd/cancel/clear/copy`
        // （`locale.ts:4473,4493,4494,4557,4558,4559`）。
        for (index, (icon, label)) in [
            (IconName::Search, "在 Git 控制台中查找"),
            (IconName::TextWrap, "自动换行"),
            (IconName::ArrowDown, "滚动到底部"),
            (IconName::CircleSlash, "取消正在运行的操作"),
            (IconName::Trash, "清空"),
            (IconName::Copy, "复制输出"),
        ]
        .into_iter()
        .enumerate()
        {
            toolbar = toolbar.child(Self::icon_button(
                ("bottom-git-console-action", index),
                icon,
                label,
                false,
                handler(|_event, _window, _cx| {}),
                cx,
            ));
        }

        h_flex()
            .size_full()
            .font_family(cx.theme().mono_font_family.clone())
            .text_size(px(CONSOLE_FONT_SIZE))
            .child(toolbar)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .items_center()
                    .justify_center()
                    .p(px(CONSOLE_OUTPUT_PADDING))
                    .text_color(cx.theme().muted_foreground)
                    // `git.console.empty` = Git 命令及其输出将显示在这里。（`locale.ts:4560`）。
                    .child("Git 命令及其输出将显示在这里。"),
            )
    }

    /// 三栏（引用 / 提交 / 提交详情）。比例与最小宽见模块文档的表格。
    ///
    /// 偏差 1：用 `relative()` 百分比而不是 `h_resizable`（理由见模块文档），所以栏间不可拖拽。
    fn log_body(&self, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        h_flex()
            .w_full()
            .flex_1()
            .min_h(px(0.))
            .child(
                div()
                    .w(relative(REFERENCE_PANE_FRACTION))
                    .min_w(px(REFERENCE_PANE_MIN_WIDTH))
                    .h_full()
                    .min_h(px(0.))
                    .child(self.reference_pane(this, cx)),
            )
            .child(
                div()
                    .w(px(1.))
                    .h_full()
                    .flex_shrink_0()
                    .bg(cx.theme().border),
            )
            .child(
                div()
                    .w(relative(COMMIT_PANE_FRACTION))
                    .min_w(px(COMMIT_PANE_MIN_WIDTH))
                    .h_full()
                    .min_h(px(0.))
                    .child(self.commit_pane(this, cx)),
            )
            .child(
                div()
                    .w(px(1.))
                    .h_full()
                    .flex_shrink_0()
                    .bg(cx.theme().border),
            )
            .child(
                div()
                    .w(relative(INSPECTOR_PANE_FRACTION))
                    .min_w(px(INSPECTOR_PANE_MIN_WIDTH))
                    .h_full()
                    .min_h(px(0.))
                    .child(self.inspector_pane(cx)),
            )
    }

    /// 刷新失败横幅。`git-log-tool-window.tsx:589-600`。
    fn banner(text: SharedString, this: &WeakEntity<Self>, cx: &App) -> impl IntoElement {
        let this = this.clone();
        h_flex()
            .w_full()
            .flex_shrink_0()
            .h(px(BANNER_HEIGHT))
            .items_center()
            .gap(px(BANNER_GAP))
            .px(px(BANNER_PADDING_X))
            .border_b_1()
            .border_color(cx.theme().danger.opacity(0.3))
            .bg(cx.theme().danger.opacity(0.1))
            .text_size(px(COMMIT_FONT_SIZE))
            .text_color(cx.theme().danger)
            .child(div().min_w_0().flex_1().text_ellipsis().child(text))
            // `git.log.retry` = 重试（`locale.ts:7197`）。
            .child(
                Button::new("bottom-git-retry")
                    .ghost()
                    .with_size(px(LOAD_MORE_BUTTON_HEIGHT))
                    .label("重试")
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = this.update(cx, |pane, cx| pane.refresh(cx));
                    }),
            )
    }

    /// 居中占位（无仓库 / 加载中 / 失败，`git-log-tool-window.tsx:602-612`）。
    fn centered_notice(
        title: SharedString,
        detail: Option<SharedString>,
        retry: Option<&WeakEntity<Self>>,
        cx: &App,
    ) -> Div {
        let mut block = v_flex()
            .w_full()
            .flex_1()
            .min_h(px(0.))
            .items_center()
            .justify_center()
            .gap(px(BANNER_GAP))
            .text_color(cx.theme().muted_foreground)
            .child(div().text_color(cx.theme().foreground).child(title));

        if let Some(detail) = detail {
            block = block.child(div().child(detail));
        }

        if let Some(this) = retry {
            let this = this.clone();
            block = block.child(
                Button::new("bottom-git-empty-retry")
                    .ghost()
                    .with_size(px(LOAD_MORE_BUTTON_HEIGHT))
                    .label("重试")
                    .on_click(move |_event, _window, cx: &mut App| {
                        let _ = this.update(cx, |pane, cx| pane.refresh(cx));
                    }),
            );
        }

        block
    }
}

impl Drop for BottomPane {
    fn drop(&mut self) {
        // 面板销毁（底部窗被拆掉 / 应用退出）时把游标还给 Core，别让 `git log` 子进程一直挂着
        // （`rust/lithe-core/src/git/history.rs:25-29`：空闲 120 s 才回收、每根最多 8 条）。
        if let Some(cursor) = self.cursor.take() {
            cursor.close();
        }
    }
}

impl Render for BottomPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = v_flex()
            .size_full()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground);

        if !self.visible {
            return root;
        }

        let this = cx.entity().downgrade();

        root = root
            .child(self.title_bar(&this, cx))
            .child(self.tab_row(&this, cx));

        if self.panel == Panel::Console {
            return root.child(Self::console_pane(cx));
        }

        match self.load_state {
            // 错误横幅 + 三栏（`git-log-tool-window.tsx:589-621`）。
            LoadState::Stale => {
                // `git.log.unableToRefresh` = 无法刷新 Git 日志。（`locale.ts:7196`）。
                root = root.child(Self::banner(
                    SharedString::from("无法刷新 Git 日志。"),
                    &this,
                    cx,
                ));
                root.child(self.log_body(&this, cx))
            }
            // `git.log.noRepository` / `git.log.openWorkspace`（`locale.ts:7198-7199`）。
            LoadState::NoRepository => root.child(Self::centered_notice(
                SharedString::from("未打开仓库"),
                Some(SharedString::from("打开一个 Git 工作区以查看提交记录。")),
                None,
                cx,
            )),
            // `git.log.loading` = 正在加载 Git 日志…（`locale.ts:7200`）。
            LoadState::Loading => root.child(Self::centered_notice(
                SharedString::from("正在加载 Git 日志…"),
                None,
                None,
                cx,
            )),
            LoadState::Failed => root.child(Self::centered_notice(
                SharedString::from("无法刷新 Git 日志。"),
                None,
                Some(&this),
                cx,
            )),
            LoadState::Ready => root.child(self.log_body(&this, cx)),
        }
    }
}
