//! 状态栏（Windows 外壳的 `Footer` 区域）。
//!
//! **无状态**：条目数据由调用方通过参数传入，本模块不持有任何 `Entity`、不轮询、不发命令。
//!
//! # 规格真源（Windows 前端，逐条给出出处）
//!
//! | 元素 | 值 | 出处 |
//! | --- | --- | --- |
//! | 状态栏高 | 24 | `windows/tauri/src/styles/theme.css:119`（`--lithe-footer-height: 1.5rem`） |
//! | 容器 | `justify-between gap-2 px-(--lithe-chrome-padding-inline) bg-surface` | `features/layout/components/footer/footer.tsx:47`、`ui/chrome.tsx:6,12-13` |
//! | 两组间距 | 8 | 同上 `gap-2` |
//! | 容器左右内边距 | 8 | `theme.css:132`（`--lithe-chrome-padding-inline: 8px`） |
//! | 容器字号 / 文字色 | 13 / `--subtle-foreground` | `theme.css:115`（`--ui-text-chrome: 13px`）、`ui/chrome.tsx:6` |
//! | 前导组 | `gap="tight" grow min-w-0` | `footer.tsx:50`、gap 值 `theme.css:129`（`2px`） |
//! | 前导路径槽 | `min-h 24 min-w-0 flex-1 overflow-hidden` | `footer.tsx:53` |
//! | 尾随组 | `gap="tight" align="end" shrink-0` | `footer.tsx:70-72` |
//! | 条目（chip） | 高 24、`max-w-50`=200、gap 4、`rounded-md`=6.4、`px-1.5`=6、13px、`hover:bg-accent hover:text-foreground` | `footer/footer-status-chip.tsx:4-5`；6.4 = `theme.css:7,134`（`calc(8px * 0.8)`） |
//! | 条目（label，不可点） | 同上但 `hover:bg-transparent hover:text-subtle-foreground` | `footer-status-chip.tsx:26-29` |
//! | 光标位置 chip | 高 **20**、全圆角、`px-1.5` | `features/editor/components/toolbar/editor-status-actions.tsx:9` |
//! | 条目顺序 | 前导 `filePath`,`branch`；尾随 `cursor`,`encoding`,`indent`,`readOnly`,`memory`,`gitChanges` | `features/layout/config/item-order.ts:20-28` |
//!
//! # 为什么没有直接用 `component::status_bar::StatusBar` 的 `.left()/.right()`
//!
//! 组件本体已核实（`gpui-component-0.6.6/src/status_bar.rs:32,41,51,57,71,77`），本模块**仍然
//! 用它的外框**（高度/底色/字号/内边距都走它），但**左右分栏没有用它**，原因只有一条，是实测
//! 源码得出的布局缺陷：
//!
//! - 它把左右两个区域都包进 `h_flex().overflow_hidden()`（`status_bar.rs:84`）作为 flex 项，
//!   `flex-shrink` 取默认值 1，且外层没有暴露 `shrink-0` 的入口。于是**内容溢出时两组会一起被
//!   压缩**（`overflow_hidden` 让自动最小尺寸退化为 0），尾随的"UTF-8 / 2 个空格 / 内存 / 更改"
//!   会被裁掉；而 Windows 的规则是"**只有前导的文件路径截断**、尾随组 `shrink-0` 永不压缩"
//!   （`footer.tsx:50` `grow min-w-0`、`:70` `shrink-0`）。路径一长就会看到差异。
//!
//! 所以这里把**整行**作为唯一 child 交给组件的中间区域（该区域 `flex_1`，独占整幅宽度，
//! `status_bar.rs:99-100`；组件文档也写明"没有 left/right 时它就是普通容器"，`:63-64,81`），
//! 行内自己排 `justify-between` + 左 `flex_1 min-w_0` + 右 `shrink_0`，与 Windows 源码一一对应。
//!
//! # 图标真源（全量 Lucide 目录 `gpui_kit::assets::IconName`）
//!
//! 应用注册的是 `gpui_kit::assets::AllAssets`（`shell_probe/mod.rs`），嵌入
//! `gpui-kit-assets-0.6.6/assets/icons/` 下全部 1830 个字形；`gpui_kit::assets::IconName`
//! 就是按这份目录生成的完整枚举（`gpui-kit-assets-0.6.6/build.rs:20-51`），所以这里用**真实字形**，
//! 不再需要"默认图标集里没有某字形"的替代：
//!
//! | Windows 源字形 | 出处 | 本模块采用 | 说明 |
//! | --- | --- | --- | --- |
//! | `HardDrivesIcon` | `footer-editor-status.tsx:113` | `IconName::HardDrive` | 同名，1:1（`icons/hard-drive.svg`） |
//! | `CheckCircleIcon` | `footer-editor-status.tsx:135` | `IconName::CircleCheck` | 同名，1:1（`icons/circle-check.svg`）；Windows 带 `text-success` 上色（`--success: var(--primary)`，`theme.css:151`），本契约无颜色字段故未实现 |
//! | `LockIcon` / `LockOpenIcon` | `footer-editor-status.tsx:102` | `IconName::Lock` / `IconName::LockOpen` | 同名，1:1（`icons/lock.svg` / `icons/lock-open.svg`） |
//! | `GitBranchIcon` | `features/git/components/git-branch-manager.tsx:647` | `IconName::GitBranch` | 同名，1:1（`icons/git-branch.svg`） |
//! | 文件类型图标 | `file-path-breadcrumb.tsx:199` | `IconName::FileText`（文本）/ `File` | 14×14：调用方自己 `Icon::new(..).with_size(px(14.))`，并把 `gpui_kit::component::Sizable` 引入作用域 |
//! | `CaretRightIcon`（面包屑分隔符） | `ui/breadcrumb.tsx:78` | `IconName::ChevronRight` | 同名，1:1，14×14 |
//!
//! 条目内的图标**不要**显式设尺寸：`Icon` 未设尺寸时会退回继承的字号
//! （`gpui-component-0.6.6/src/icon.rs:169-180,218`），正好等于 Windows 图标默认的
//! `size="1em"`（`windows/tauri/src/ui/icons.tsx:154`），状态栏里即 13×13。
//!
//! # 未实现 / 需要回调的部分（本契约无法表达，均不改签名）
//!
//! 1. **文件路径面包屑**（前导第 1 项）：分段可点、弹出目录下拉
//!    （`features/editor/components/toolbar/file-path-breadcrumb.tsx:187-204,205-273`
//!    与 `path-breadcrumb.tsx:47-69`），需要 `Fn(segment_index, MouseEvent)`。
//! 2. **Git 分支项**（前导第 2 项）：`GitBranchManager` 的 trigger 是按钮，点击开分支面板
//!    （`git-branch-manager.tsx:634-661`，footer 形态另有 `font-medium`、`px-2`、
//!    `hover:bg-accent/80`、`max-w-full shrink overflow-hidden`，且 `showCounts={false}`
//!    所以 ahead/behind 在页脚**不显示**数字），需要 `on_click`。
//! 3. **光标位置 chip**（尾随第 1 项）：点击后原位变成输入框、回车派发 `menu-go-to-line`
//!    （`editor-status-actions.tsx:29,41-58,60-83,86-93`），需要 `on_click` + 编辑态。
//! 4. **Git 更改 chip**（尾随第 6 项）：点击 `openSidebarView("git")`
//!    （`footer-editor-status.tsx:127`），需要 `on_click`。
//! 5. **哪些条目可点**：本契约的 `StatusEntry` 只有图标+文字，**无法区分** Windows 的
//!    `FooterStatusChip`（可点，有 hover 底色）与 `FooterStatusLabel`（纯展示，Windows 里
//!    显式把 hover 覆盖成透明）。因此这里**统一按 chip 的悬停观感渲染**（静止态两者本来
//!    完全一样，见 `footer-status-chip.tsx:4-5,26-29`）。要还原差异需给 `StatusEntry`
//!    加一个 `interactive: bool`（或 `on_click`）。
//! 6. **光标 chip 的 20 高 + 全圆角**（`editor-status-actions.tsx:9`）与普通 chip 的 24 高 +
//!    6.4 圆角冲突，同样因为区分不出条目种类而统一按 24。
//! 7. **内存条目**的数值来自 10s 轮询的原生命令 `get_application_memory_usage`
//!    （`footer-editor-status.tsx:24,45-67`），属数据层，不在本模块。
//! 8. **窗口透明（毛玻璃）模式的覆盖底色**（`.lithe-footer-bar` 覆盖
//!    `window-transparency.css:64-72`）没有输入来源，未实现；底色走 `cx.theme()`。
//! 9. **密度档位**：`comfortable` 下页脚高变 32、字号 14、gap 变 4
//!    （`theme.css:188-200`），本模块按默认档（24/13/2）用常量写死；换档要改常量或引入主题配置。
//! 10. **计划明确排除的死代码**（`01-shell.md:104`）：`useFooterDebuggerItem`
//!     （`footer-debugger-item.tsx:29`）与 `FooterControlBadge`（`footer-tab-control.tsx:23`）不实现。
//!
//! 另外：状态栏**没有分隔符**。`ui/chrome.tsx:122` 的 `ChromeSeparator` 全仓仅有一处定义、
//! 零处引用，`footer.tsx` 也没有用它——所以本模块不画竖线（`01-shell.md:101` 把它列成
//! "标题栏/状态栏共用原语"，与源码不符）。

use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, Pixels, SharedString,
    Styled as _, Window, prelude::FluentBuilder as _, px,
};

/// 状态栏总高：`--lithe-footer-height: 1.5rem`（`styles/theme.css:119`）。
const BAR_HEIGHT: Pixels = px(24.);
/// 容器左右内边距：`--lithe-chrome-padding-inline: 8px`（`styles/theme.css:132`）。
const BAR_PADDING_INLINE: Pixels = px(8.);
/// 前导组与尾随组之间的间距：`gap-2`（`footer/footer.tsx:47`）。
const BAR_GAP: Pixels = px(8.);
/// 组内条目间距：`--lithe-chrome-gap-tight: 2px`（`styles/theme.css:129`、`footer.tsx:50,70` 的 `gap="tight"`）。
const GROUP_GAP: Pixels = px(2.);
/// 条目高：`--lithe-chrome-control-height: 1.5rem`（`styles/theme.css:126`、`footer-status-chip.tsx:5`）。
const CHIP_HEIGHT: Pixels = px(24.);
/// 条目最大宽：`max-w-50` = 200px（`footer-status-chip.tsx:5`）。
const CHIP_MAX_WIDTH: Pixels = px(200.);
/// 条目内图标与文字的间距：`gap-1` = 4px（`footer-status-chip.tsx:5`）。
const CHIP_GAP: Pixels = px(4.);
/// 条目左右内边距：`px-1.5` = 6px（`footer-status-chip.tsx:5`）。
const CHIP_PADDING_INLINE: Pixels = px(6.);
/// 条目圆角：`rounded-md` = `--radius-md` = `calc(8px * 0.8)` = 6.4px（`styles/theme.css:7,134`）。
const CHIP_RADIUS: Pixels = px(6.4);
/// 状态栏字号：`--ui-text-chrome: 13px`（`styles/theme.css:115`、`ui/chrome.tsx:6`）。
const CHROME_TEXT_SIZE: Pixels = px(13.);

/// 状态栏里的一个条目。
///
/// 字段私有 + [`StatusEntry::new`] / [`StatusEntry::with_icon`]：跨 crate 之后结构体字面量
/// 不再是合法构造方式（《编码指南》「公共 API 设计」）。图标取自全量目录
/// （`gpui_kit::assets::IconName`，模块文档的对照表）。
#[non_exhaustive]
pub struct StatusEntry {
    /// 条目前导图标；`None` = 纯文字条目。
    icon: Option<IconName>,
    /// 条目文字；中文一律逐字取 `windows/tauri/src/i18n/locale.ts` 的原文，不要自己编。
    text: SharedString,
}

impl StatusEntry {
    /// 一个纯文字条目。
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            icon: None,
            text: text.into(),
        }
    }

    /// 带前导图标的条目（non-boolean builder 用 `with_` 前缀，《编码指南》词汇表）。
    pub fn with_icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// 渲染状态栏：`left` 贴左并可被压缩（文件路径在这里截断），`right` 贴右且 `shrink-0`。
///
/// `window` 当前未使用（契约要求保留该参数，后续接点击/焦点时才会用到），故以 `_window` 命名。
pub fn status_bar(
    left: &[StatusEntry],
    right: &[StatusEntry],
    _window: &Window,
    cx: &App,
) -> impl IntoElement {
    StatusBar::new()
        // 高度 24：覆盖组件默认的 `py_1`（`status_bar.rs:89`）撑出来的高度。
        .h(BAR_HEIGHT)
        // 清掉默认 `py_1`：条目自己就是 24 高，再留上下 4px 会让条目溢出内容盒。
        .py_0()
        // 清掉默认 `border_t_1`（`status_bar.rs:91`）：Windows 页脚**无上边框**。
        .border_0()
        .px(BAR_PADDING_INLINE)
        // 底色 = Windows 的 `--surface`（`footer.tsx:47` `bg-surface`）。用状态栏专用 token
        // （组件自身也用它，`status_bar.rs:93`），不写裸色值。
        // ⚠️ 值必须由项目主题配置给到：gpui-kit 默认深色的 `status_bar.background` 是
        // `#171717`（`theme/default-theme.json:293`），而 Lithe 的深色 `--surface` 是
        // `#2b2d30`（`01-shell.md:311`）—— 要一致就得在这个 token（或 `background`）上配。
        .bg(cx.theme().tokens.status_bar)
        // 字号 13，覆盖组件默认的 `text_xs()`（12px，`status_bar.rs:94`）。
        .text_size(CHROME_TEXT_SIZE)
        // 文字色 = `--subtle-foreground`（`ui/chrome.tsx:6`）→ `muted_foreground`（`status_bar.rs:95`）。
        .text_color(cx.theme().muted_foreground)
        .child(status_row(left, right, cx))
}

/// 状态栏的一整行：前导组贴左、尾随组贴右（`footer.tsx:47` `justify-between`）。
///
/// 见模块文档"为什么没有直接用 `.left()/.right()`"：组件左右区域的 `overflow_hidden` flex 项
/// 无法设成 `shrink-0`，溢出时会把尾随项一起裁掉。这里走组件的中间区域（`flex_1` 独占整幅宽），
/// 行内自己排 flex，与 `footer.tsx:50,70` 一一对应。
fn status_row(left: &[StatusEntry], right: &[StatusEntry], cx: &App) -> impl IntoElement {
    h_flex()
        .w_full()
        .items_center()
        .justify_between()
        .gap(BAR_GAP)
        .child(entry_group(left, "status-bar-leading", true, cx))
        .child(entry_group(right, "status-bar-trailing", false, cx))
}

/// 一组条目。
///
/// `leading` 决定 flex 行为，对应 Windows 两条相反的规则：
/// 前导组 `grow min-w-0 overflow-hidden`（`footer.tsx:50,53`）——剩余宽度归文件路径，且路径先被截断；
/// 尾随组 `shrink-0`（`footer.tsx:70`）——永远不压缩。
fn entry_group(
    entries: &[StatusEntry],
    id_prefix: &str,
    leading: bool,
    cx: &App,
) -> impl IntoElement {
    h_flex()
        .items_center()
        .gap(GROUP_GAP)
        .when(leading, |this| this.flex_1().min_w_0().overflow_hidden())
        .when(!leading, |this| this.flex_shrink_0())
        .children(
            entries
                .iter()
                .enumerate()
                .map(|(index, entry)| entry_chip(format!("{id_prefix}-{index}"), entry, cx)),
        )
}

/// 单个条目：静止态与 Windows 的 `FooterStatusChip` / `FooterStatusLabel` 完全一致
/// （两者共用同一组类名，`footer-status-chip.tsx:4-5,26-29`）。
///
/// `id` 由调用方按"组前缀 + 序号"生成：`.hover()` 需要元素有 id 才会记录 hover 状态
/// （`gpui-pre-0.3.6/src/elements/div.rs:2844-2849`），否则悬停底色永远不生效。
fn entry_chip(id: String, entry: &StatusEntry, cx: &App) -> impl IntoElement {
    let hover_bg = cx.theme().accent;
    let hover_fg = cx.theme().foreground;

    h_flex()
        .id(id)
        .h(CHIP_HEIGHT)
        .max_w(CHIP_MAX_WIDTH)
        .flex_shrink_0()
        .items_center()
        .gap(CHIP_GAP)
        .rounded(CHIP_RADIUS)
        .px(CHIP_PADDING_INLINE)
        .whitespace_nowrap()
        // `hover:bg-accent hover:text-foreground`（`footer-status-chip.tsx:5`）。
        // UI-MAP §1.2：gpui 的 `accent` 语义就是"悬停底色"。
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        .when_some(entry.icon.clone(), |this, icon| this.child(Icon::new(icon)))
        .child(entry.text.clone())
}
