//! Lithe 的 GPUI 宿主里「Git」Feature crate：底部工具窗的「提交记录」页 + 左栏「源代码管理」视图。
//!
//! 从 `lithe-gpui/shell/src/bin/shell_probe/bottom_panel.rs`（3240 行，单个 bin crate 里的扁平模块）
//! 拆出来，按《编码指南》「大型应用按业务能力组织 crate」做成一个拥有独立状态与生命周期的
//! Feature crate。本 crate 做两件事，各占一组模块：
//!
//! 1. **底部工具窗「提交记录」**（[`BottomPane`]）—— 历史阅读器 + 控制台外壳；
//! 2. **左栏「源代码管理」**（[`ChangesView`]）—— 工作区变更列表 + 提交入口，
//!    规格真源 `lithe-gpui/research/windows/08-source-control.md`。
//!
//! ## 文件分工
//!
//! - `model.rs`：**所有不渲染的东西** —— 5 条 `git.*` 命令的请求与解析、分页游标生命周期
//!   （`git.historyPage` 的 `nextCursor` 背后是一个活着的 `git log` 子进程）、
//!   `resolve_repository_root`、行数据（`Reference` / `Commit` / `RefRow` / `CommitFileRow` /
//!   `GraphRow` / `Detail` / `Label` 等）与全部纯函数（`layout_graph` / `build_reference_rows` /
//!   `build_commit_files` / `matches_filter` / `tracking_count` / `parse_*` / 颜色映射）；
//! - `branch_info.rs`：标题栏「分支弹窗」的**只读数据边界** —— `git.status` + `git.references`
//!   合成一份 [`BranchSnapshot`]（仓库根 / 当前分支 / ahead-behind / 本地分支列表），
//!   不渲染任何东西；面板本身由工作台画（`lithe-gpui/crates/workbench/src/branch_panel.rs`）。
//! - `log_view.rs`：`BottomPane` 结构体与它的字段、`new` / `set_visible` / `visible` /
//!   `refresh`、`Drop`、`Render`、所有 `render_*` 与只画界面的辅助函数
//!   （`icon_button` / `toolbar_button` / `title_bar` / `tab_row` / `filter_row` / `commit_row` /
//!   `reference_row` / `graph_cell` / `label_badge` / `commit_file_row` / `banner` /
//!   `centered_notice` / `console_pane` / `log_body`）。
//!   度量常量集中在 `model.rs` 定义（单一真源），由 `log_view.rs` 导入使用。
//! - `changes.rs`：**左栏「源代码管理」的数据层** —— `git.status` + `git.operationState` 合成
//!   [`ChangesSnapshot`]（仓库根 / 分支 / ahead-behind / 变更行 / 进行中的操作），
//!   加文件级写操作（`stage` / `unstage` / `stageAll` / `commit` / `discard` / 操作continue-abort-skip）。
//!   本文件把「Core 信封 `ok:true` 但 Git 退出码非 0」翻成 [`WriteOutcome::failure`]，
//!   这是本侧对真源"写失败静默"那一条的正面修正；不渲染任何东西。
//! - `changes_view.rs`：**左栏「源代码管理」的表现层** —— `ChangesView` 实体
//!   （`new` / `refresh` / `activate` / `Render`）、五态空态、操作横幅、提交面板。
//!   变更列表的行是**本文件自写的最小行**，不复用 `explorer` 的树行（理由写在文件头）。
//!
//! > 拆分后**没有做任何布局、尺寸、颜色、文案、交互或数据接线的改动**：两个文件里的代码都是
//! > 从旧文件逐字搬迁，只加了 `pub(crate)` 可见性与 `use crate::…` / `use lithe_gpui_shared::…`
//! > 的 import。唯一的例外是 `model::execute_core` 的信封实现（见下面「拆分后的偏离」）。
//!
//! ## 公开边界
//!
//! 只有 [`BottomPane`] 与 [`ChangesView`]（两个面板实体）是公开 API，由本文件明确 re-export；
//! 公开方法是 `new` / `set_visible` / `visible` / `refresh` / `activate`（签名见各文件），
//! 字段全部私有。`model` / `log_view` / `changes` / `changes_view` 这些实现路径**不**出现在
//! import 路径里，符合《编码指南》「内部重组时保持 public module path」的反面用法：
//! 新建 crate 直接发布 `pub use`，不把内部文件划分变成契约。
//!
//! 快捷键的**绑定**（`Ctrl+Enter` 提交）也只在这里登记一次，见 [`install_actions`]。
//!
//! 下面是拆分前 `bottom_panel.rs` 的文件头规格说明，**逐字保留**（那是本项目的规格真源）：
//!
//! 底部工具窗「提交记录」（Windows 的 `GitLogToolWindow`）。
//!
//! 规格真源：`windows/tauri/src/features/git/components/log/*`（Windows 前端 = 视觉与布局唯一真源），
//! 度量速查表 `lithe-gpui/UI-MAP-WINDOWS.md` §1.5 与 §2④，调研原文
//! `lithe-gpui/research/windows/03-git-and-bottom.md`。挂钩位置见该文 §1.1：默认
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
//! ## 拆分后的偏离（相对上面这份规格）
//!
//! 除文件位置与可见性之外只有一处实质改动：
//!
//! 1. **本地 `execute_core` 的信封实现收敛到 `lithe_gpui_shared::core_client`**。上面「数据：Core
//!    命令」一节写的「拼 `{id, operationId, timeoutMilliseconds, command, payload}` →
//!    `lithe_core::execute_json` → 判 `ok` 取 `data`」现在由 `shared` 的信封层负责
//!    （`CoreRequest` + `CoreClient`，见 `lithe-gpui/crates/shared/src/core_client.rs`）。返回语义
//!    保持原样：失败仍是错误码原文（`CoreError::code()`；信封级失败取 `CoreError` 的
//!    `Display` 原文），`ok:true` 但没有 `data` 仍当作 `Value::Null`，调用方那些按字段判空的
//!    判断都没动。本 Feature 自己的**超时口径 60 s** 用
//!    `CoreRequest::with_timeout_millis` 显式保留（`model::GIT_TIMEOUT_MILLIS`），不跟随
//!    信封层的默认值。
//! 2. 因此本地 `NEXT_OPERATION_ID` 自增序号删掉了：`operationId` 改由信封层生成，形状从
//!    `shell-probe-<command>-<n>` 变成 `lithe-gpui-<command>-<n>`。它只出现在诊断里，
//!    而 Core 只要求同一次调用的 `id` / `operationId` 相等（信封层两者取同一个值）。
//!
//! ## 左栏「源代码管理」的规格出处与偏离（第二件事）
//!
//! 结构 / 文案 / 交互 / 空态 / 数据来源与第一版范围全部照
//! `lithe-gpui/research/windows/08-source-control.md`（643 行，逐条带 `文件:行号`，真源
//! `windows/tauri/src/features/git/components/`）。**刻意不做**（该文 §6.3 的三类缺子系统
//! 加两条平台依赖）：原生 Git 元数据 watcher（`watch_git_repository` + 事件）、
//! 分支 / 远程 / 工作树面板、冲突三方合并、差异富渲染（Monaco 级）、AI 提交说明、
//! 多仓库聚合、拖拽互操作。刷新通路因此只有三条：**激活视图时 / 手动 / 写操作后**。
//!
//! 与真源的实质性偏离逐条写在各文件的模块文档里（`changes.rs` 3 条、`changes_view.rs` 5 条），
//! 最重要的一条是**失败绝不静默**：真源的写操作失败常常只 `console.error`（研究 §5 末表），
//! 本侧把「Core 信封 `ok:true` 但 Git 退出码非 0」也翻成用户可见的红条。

mod branch_info;
mod changes;
mod changes_view;
mod identity;
mod log_view;
mod model;

pub use branch_info::{BranchInfo, BranchSnapshot, TrackingCounts};
pub use changes_view::ChangesView;
pub use identity::{IdentityField, IdentityScope, IdentitySetup, configure_identity, inspect_identity};
pub use log_view::BottomPane;

use gpui_kit::{App, KeyBinding};

gpui_kit::actions!(lithe_git, [CommitChanges]);

/// 登记本 Feature 的应用级快捷键。**每个窗口调用一次**（`ShellWorkspace::new`）。
///
/// 只有一条：`Ctrl+Enter` → [`CommitChanges`]（左栏「更改」视图的提交说明框内提交，
/// 真源 `git-commit-panel.tsx:229-234` 的 `Mod+Enter`）。命中后由
/// `ChangesView` 根元素的 `on_action` 处理器接住。
///
/// `KeyBinding::new(.., None)` 的上下文谓词是空，`binding_enabled` 会按 `contexts.len()`
/// （**最深**）算深度（`gpui-pre-0.3.6/src/keymap.rs:246-252`），也就是"任何焦点下都可能命中、
/// 且优先级最高"。`ctrl-enter` 在 `gpui-base` / `gpui-component` 里**没有任何绑定**
/// （全量 grep `ctrl-enter` / `ctrl-return` 零命中），所以给它 `None` 是安全的 ——
/// 与 `lithe_gpui_editor::install_actions` 给 `ctrl-s` / `ctrl-w` 同一条判据。
pub fn install_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-enter", CommitChanges, None)]);
}
