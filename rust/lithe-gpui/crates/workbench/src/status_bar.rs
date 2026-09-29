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
//! # 图标真源：真源 SVG 优先，缺真源的保持 Lucide
//!
//! 与活动栏同一口径（见 [`crate::activity_bar`] 的模块文档）：**有 `lithe-gpui/assets/ui-icons/idea/**`
//! 真源文件的用真源（[`StatusEntry::with_idea_icon`]），真源只有内联 React 组件或本来就是
//! Lucide 的保持 `IconName`（[`StatusEntry::with_icon`]）**。
//!
//! | Windows 源字形 | 出处 | 本模块采用 | 说明 |
//! | --- | --- | --- | --- |
//! | `GitBranchIcon` | `features/git/components/git-branch-manager.tsx:647` | **真源** `idea::GIT_BRANCH_ICON` | `ui-icons/idea/vcs/branch.svg`(+`_dark`) |
//! | `LockIcon` / `LockOpenIcon` | `footer-editor-status.tsx:102` | **真源** `idea::LOCK_ICON` / `LOCK_OPEN_ICON` | `expui/general/locked.svg` / `unlocked.svg` |
//! | `CheckCircleIcon` | `footer-editor-status.tsx:135` | **真源** `idea::CHECK_CIRCLE_ICON` | `expui/general/successDialog.svg`；Windows 带 `text-success` 上色（`--success: var(--primary)`，`theme.css:151`），本契约无颜色字段故未实现 |
//! | 文件类型图标 | `file-path-breadcrumb.tsx:196-202` 的 `ThemedFileIcon` | **真源**：当前活动文件名的图标主题查表 | 与文件树 / 标签同一条 `icons::FileIcon` 路线（`explorer` / `editor` 的 `icon_for_file`） |
//! | `HardDrivesIcon` | `footer-editor-status.tsx:113` | `IconName::HardDrive` | 真源是 `Nucleo.IconHardDriveOutline18` → Lucide `hard-drive`，**已经是 1:1** |
//! | `CaretRightIcon`（面包屑分隔符） | `ui/breadcrumb.tsx:78` | `IconName::ChevronRight` | 真源 `expui/general/chevronRight.svg` 存在，但面包屑分段本身还没实现（见「未实现」第 1 条），本轮不动 |
//!
//! 条目内的图标**不要**显式设尺寸：`Icon` 未设尺寸时会退回继承的字号
//! （`gpui-component-0.6.6/src/icon.rs:169-180,218`），正好等于 Windows 图标默认的
//! `size="1em"`（`windows/tauri/src/ui/icons.tsx:154`），状态栏里即 13×13。
//! 真源那一路由 [`lithe_gpui_shared::icons::FileIcon::render`] 画，边长要显式给，
//! 所以 [`StatusEntry`] 存**已解析好的**图标（见字段说明）。
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

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::{ActiveTheme as _, Icon};
use gpui_kit::{
    AnyElement, App, ClickEvent, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, prelude::FluentBuilder as _,
    rems,
};

use lithe_gpui_shared::icons::{FileIcon, file_icon, idea};
// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`，与 Windows 规格逐像素相等：
//
// | 规格（Windows 真源） | 值 | 用到的 helper |
// | --- | --- | --- |
// | `--lithe-footer-height: 1.5rem`（`styles/theme.css:119`） | 24 | `h_6()` |
// | `--lithe-chrome-padding-inline: 8px`（`styles/theme.css:132`） | 8 | `px_2()` |
// | `gap-2`（`footer/footer.tsx:47`） | 8 | `gap_2()` |
// | `--lithe-chrome-gap-tight: 2px`（`styles/theme.css:129`） | 2 | `gap_0p5()` |
// | `--lithe-chrome-control-height: 1.5rem`（`styles/theme.css:126`） | 24 | `h_6()` |
// | `gap-1` / `px-1.5`（`footer-status-chip.tsx:5`） | 4 / 6 | `gap_1()` / `px_1p5()` |
//
// 字号：`--ui-text-chrome` 是 **13px**（`styles/theme.css:115`、`ui/chrome.tsx:6`），gpui 的档位
// 只有 `text_xs()`(12) / `text_sm()`(14)，13 不在档位上。按《编码指南》用 **`text_sm()`（14px）**
// ——13 → 14 是经维护者确认的**有意**视觉改动，不是等价换算。

/// 条目最大宽：`max-w-50` = 200px（`footer-status-chip.tsx:5`）。
///
/// 200 不在 gpui 的固定 rem 档位上（档位里 48 → 192、56 → 224，
/// `gpui-pre-macros-0.3.6/src/styles.rs:1039-1047`）；Tailwind v4 的任意整数档 `max-w-50`
/// 在 gpui 里没有对应 helper，不能自己发明一个 —— 档位外就写 helper 底层的
/// `rems(CHIP_MAX_WIDTH_SPEC / 16.)`（`Styled::max_w` 收 `impl Into<Length>`，
/// 而 `Length: From<Rems>`）。
const CHIP_MAX_WIDTH_SPEC: f32 = 200.;

/// 条目圆角：`rounded-md` = `--radius-md` = `calc(8px * 0.8)` = 6.4px（`styles/theme.css:7,134`）。
///
/// 6.4 **不是** gpui 的 rem 档位（gpui 的 `rounded_md()` 是 6px，
/// `gpui-pre-macros-0.3.6/src/styles.rs:1245-1249`）；也不能从主题读 —— `ThemeConfig.radius`
/// 是 `usize`（`gpui-component-0.6.6/src/theme/schema.rs:67-68`），装不下 6.4。同样写
/// `rems(CHIP_RADIUS_SPEC / 16.)`。
const CHIP_RADIUS_SPEC: f32 = 6.4;

/// 状态栏里的一个条目。
///
/// 字段私有 + [`StatusEntry::new`] / [`StatusEntry::with_icon`] / [`StatusEntry::with_idea_icon`] /
/// [`StatusEntry::with_file_icon`]：跨 crate 之后结构体字面量不再是合法构造方式
/// （《编码指南》「公共 API 设计」）。
#[non_exhaustive]
pub struct StatusEntry {
    /// 条目前导图标的**已解析**资源路径（`None` = 纯文字条目）。
    ///
    /// 存路径而不是存 `IconName`：状态栏的条目在 `ShellWorkspace::footer_right` 里**当帧**
    /// 构造（光标位置每次都可能变），所以"构造时就定好明暗"和"渲染时再查"是同一帧，
    /// 没有陈旧风险。这样 [`StatusEntry`] 仍然只是两个 `Copy` 字段，`status_bar` 的
    /// `use<>` 也不用跟着改。
    ///
    /// 真源那一路存 `ui-icons/idea/**` 的路径；Lucide 那一路存 `None` 且用
    /// [`StatusEntry::lucide`] 记字形（两者互斥，同时给出时真源优先 —— 与
    /// [`FileIcon::render`] 的优先关系一致）。
    icon: Option<&'static str>,
    /// `icon` 为 `None` 时用的 Lucide 字形。
    lucide: Option<IconName>,
    /// 条目文字；中文一律逐字取 `windows/tauri/src/i18n/locale.ts` 的原文，不要自己编。
    text: SharedString,
    /// 点击回调（`None` = 纯展示条目）。
    ///
    /// 2026-09-27 为通知中心的「点开通知中心」入口加的（IDEA 的口径是状态栏那条消息
    /// **就是**通知本身，点它打开工具窗）。**那个入口已于 2026-09-29 随状态栏回显一起
    /// 删除**（通知的即时反馈只剩左下角 toast），但能力留下：它就是本模块文档第 5 条
    /// 预言的那个扩展点（"要还原差异需给 `StatusEntry` 加一个 `interactive: bool`
    /// （或 `on_click`）"）—— 选了 `on_click` 而不是布尔，因为 Windows 那边是
    /// `FooterStatusChip`（可点）与 `FooterStatusLabel`（纯展示）两种组件，带回调就顺便
    /// 把两者区分开了。
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl StatusEntry {
    /// 一个纯文字条目。
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            icon: None,
            lucide: None,
            text: text.into(),
            on_click: None,
        }
    }

    /// 让这一格可点。
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// 带 Lucide 前导图标的条目（non-boolean builder 用 `with_` 前缀，《编码指南》词汇表）。
    pub fn with_icon(mut self, icon: IconName) -> Self {
        self.lucide = Some(icon);
        self
    }

    /// 带**真源**前导图标的条目：`ui-icons/idea/**` 的 expui SVG（`idea::` 常量）。
    ///
    /// 明暗在这里就定（`cx` 是当帧的），见 [`StatusEntry::icon`] 的字段说明。
    pub fn with_idea_icon(mut self, icon: &'static idea::IdeaIcon, cx: &App) -> Self {
        self.icon = Some(icon.path(gpui_kit::component::Theme::global(cx).is_dark()));
        self
    }

    /// 带**文件类型**图标的条目：与文件树 / 编辑器标签同一条主题查表路线
    /// （真源优先、Lucide 回落），只是换成"状态栏这一格想要的 14×14"。
    ///
    /// `name` 是文件名（不是完整路径）。
    pub fn with_file_icon(mut self, name: &str, cx: &App) -> Self {
        let icon = FileIcon::lucide(file_icon::lucide_fallback(name)).with_theme_icon(name, cx);
        self.icon = icon.themed;
        self.lucide = Some(icon.fallback);
        self
    }
}

/// 渲染状态栏：`left` 贴左并可被压缩（文件路径在这里截断），`right` 贴右且 `shrink-0`。
///
/// `window` 当前未使用（契约要求保留该参数，后续接点击/焦点时才会用到），故以 `_window` 命名。
///
/// ⚠️ 返回类型带 **`use<>`**（不捕获任何入参生命周期）：调用方 `ShellWorkspace::render`
/// 的尾随组是**当帧算出来的临时 `Vec`**（光标位置每次重绘都可能不同，
/// 见 `ShellWorkspace::footer_right`）。没有 `use<>` 时 edition 2024 的 RPIT 会把
/// `&[StatusEntry]` 的生命周期捕获进返回类型，临时值活不过返回的元素（E0515）。
/// 本条链路上的元素本来就只持有 `SharedString` / `&'static str` / `IconName` 的**拷贝**，
/// 不借入参，所以 `use<>` 成立。
pub fn status_bar(
    left: &[StatusEntry],
    right: &[StatusEntry],
    _window: &Window,
    cx: &App,
) -> impl IntoElement + use<> {
    StatusBar::new()
        // 高度 24：覆盖组件默认的 `py_1`（`status_bar.rs:89`）撑出来的高度。
        .h_6()
        // 清掉默认 `py_1`：条目自己就是 24 高，再留上下 4px 会让条目溢出内容盒。
        .py_0()
        // 清掉默认 `border_t_1`（`status_bar.rs:91`）：Windows 页脚**无上边框**。
        .border_0()
        .px_2()
        // 底色 = Windows 的 `--surface`（`footer.tsx:47` `bg-surface`）。用状态栏专用 token
        // （组件自身也用它，`status_bar.rs:93`），不写裸色值。
        // ⚠️ 值必须由项目主题配置给到：gpui-kit 默认深色的 `status_bar.background` 是
        // `#171717`（`theme/default-theme.json:293`），而 Lithe 的深色 `--surface` 是
        // `#2b2d30`（`01-shell.md:311`）—— 要一致就得在这个 token（或 `background`）上配。
        .bg(cx.theme().tokens.status_bar)
        // 字号：`text_sm()`（14px，原 Lithe 基准 13px，见上方映射表说明）。
        .text_sm()
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
        .gap_2()
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
        .gap_0p5()
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
    let on_click = entry.on_click.clone();

    h_flex()
        .id(id)
        .h_6()
        .max_w(rems(CHIP_MAX_WIDTH_SPEC / 16.))
        .flex_shrink_0()
        .items_center()
        .gap_1()
        .rounded(rems(CHIP_RADIUS_SPEC / 16.))
        .px_1p5()
        .whitespace_nowrap()
        // `hover:bg-accent hover:text-foreground`（`footer-status-chip.tsx:5`）。
        // UI-MAP §1.2：gpui 的 `accent` 语义就是"悬停底色"。
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        .when_some(entry_icon(entry, cx), |this, icon| this.child(icon))
        .child(entry.text.clone())
        // `on_click` 挂在 `StatefulInteractiveElement` 上，而这格已经有 `.id(..)`（变
        // `Stateful<Div>`），所以能挂。`cursor_pointer` 只给可点的那格 —— 纯展示条目
        // 保持默认指针，Windows 的 `FooterStatusLabel` 也是这样（`footer-status-chip.tsx:26-29`）。
        .when_some(on_click, |this, handler| {
            this.cursor_pointer().on_click(move |event, window, cx| handler(event, window, cx))
        })
}

/// 条目要画的那个图标元素。
///
/// ## 为什么这里用 `svg().path(..)` 而不是 `img()`（与文件树/标签的取舍相反）
///
/// 文件类型图标**颜色就是信息**，所以文件树与编辑器标签走 `img()`（保留原色，见
/// `lithe_gpui_shared::icons::file_icon`）。状态栏这一格不同：它是 13px 单色字形，
/// 与旁边的文字同色（`--subtle-foreground`）才是真机的样子。
///
/// 而且实测（125% DPI，本机）：**`Img` 放在这里根本不画**。同一帧里文件树的 `Img`
/// 正常显示（16×16 的 `svg_renderer` 位图），状态栏这一格的 `Img` 一个像素都没有
/// （`statuszoom-before/after.png` 的对比里，分支字形整块消失）。
/// 原因是 `Img::request_layout` 只在有 `GlobalElementId` 时才会 `use_data`
/// （`gpui-pre-0.3.6/src/elements/img.rs:289-315`），而这一格（`h_flex` + `.id(..)` 的
/// 状态栏 chip）没给它稳定的全局 id；`svg()` 没有这个约束。
///
/// 走 `svg()` 的代价是丢掉原色、改由 `text_color` 上色 —— 对状态栏这正是想要的。
///
/// - **真源**（`entry.icon` 有值）：`ui-icons/idea/**` 的 expui SVG，14 = `rems(14. / 16.)`
///   （与旧实现的 `with_size(px(14.))` 同值；14 不在 rem 档位上，所以自己换算）。
/// - **Lucide**（`entry.lucide` 有值）：`Icon::new(..)`，**不设尺寸** ——
///   `Icon` 会退回继承的字号，正好等于 Windows 图标默认的 `size="1em"`（13×13）。
fn entry_icon(entry: &StatusEntry, cx: &App) -> Option<AnyElement> {
    if let Some(path) = entry.icon {
        let theme = gpui_kit::component::Theme::global(cx);
        return Some(
            gpui_kit::svg()
                .path(path)
                .flex_shrink_0()
                .text_color(theme.muted_foreground)
                .w(rems(14. / 16.))
                .h(rems(14. / 16.))
                .into_any_element(),
        );
    }
    entry
        .lucide
        .map(|name| Icon::new(name).into_any_element())
}
