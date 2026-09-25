//! 左右活动栏（图标竖条）。
//!
//! **无状态**：选中项由 `active` 传入，点击通过 `on_select` 交回调用方，本文件不持有任何状态。
//! 度量与配色一律取自 **Windows 前端源码**（规格真源），行号写在对应常量/代码旁边。
//!
//! ## 折叠 / 展开是两个宽度（容易混）
//!
//! - 折叠态 `COLLAPSED_ACTIVITY_RAIL_WIDTH = 38`
//!   （`windows/tauri/src/features/layout/components/sidebar/main-sidebar.tsx:97`）；
//! - 展开态默认 **160**、min **140**、max **320**（同文件 `:98-100`）；
//! - `MainLayout` 固定传 `expanded={false}`（`windows/tauri/src/features/layout/components/main-layout.tsx:302`），
//!   所以 Windows 产品里只有折叠态。
//!
//! **本函数只画折叠态**：展开态还带项目圆点、拖拽热区（宽 4、悬停 `primary/8`）、横向轮播
//! （`main-sidebar.tsx:689-708`、`sidebar-projects.tsx`），未实现。
//!
//! ## 盒宽算术（照抄源码，不要"顺手居中"）
//!
//! - 左 rail 容器宽 `calc(<railPanelWidth>px + var(--lithe-workbench-gap))` = 38 + 4 = **42**
//!   （`main-sidebar.tsx:588-590`）；其内容层左右内衬各 **8**（`:610-611`，`ACTIVITY_RAIL_HORIZONTAL_GUTTER`）
//!   → 图标项盒 **26 宽**，图标（16）中心落在 x=21。
//! - 右 rail 宽 `w-9.5` = **38**、`items-center`、`pt-1` = 4（`plugin-activity-rail.tsx:34`），
//!   而外层 flex row 自带 `pr-(--lithe-workbench-gap)` = 4（`main-layout.tsx:299`）
//!   → 图标项 **28×28 居中**，中心落在 x=19。
//! - 因此两条 rail 的总宽都是 42，但图标中心一个 21、一个 19 —— 这是源码事实，不是笔误。
//!
//! ## 图标：真源优先，缺真源的保持 Lucide（逐项登记）
//!
//! 每个项的画法由 [`ActivityIcon`] 决定，有两条路：
//!
//! 1. **真源**（[`ActivityIcon::Idea`]）—— `gpui/assets/ui-icons/idea/**` 的 IntelliJ `expui` SVG，
//!    经 `lithe_gpui_shared::icons::idea_icon_svg` 按当前主题明暗挑路径渲染（单色，
//!    颜色取 `theme.foreground`/`muted_foreground`，与旧实现的 `IconName` 同口径）。
//! 2. **Lucide**（[`ActivityIcon::Lucide`]）—— 全量 Lucide 目录（1830 个字形）。
//!    留在这条路上的项都有明确理由，逐条写在 [`workspace`] 的 `activity_items()` 调用处。
//!
//! | Windows 用途 | Windows 图标组件 | 现在用 | 真源文件 / 理由 |
//! | --- | --- | --- | --- |
//! | 项目 | `FilesIcon` | **真源** `idea::FILES_ICON` | `ui-icons/idea/expui/general/listFiles.svg`（+`_dark`） |
//! | 搜索 | `MagnifyingGlassIcon` | **真源** `idea::MAGNIFYING_GLASS_ICON` | `expui/general/search.svg` |
//! | 更改（Git） | `GitBranchIcon` | **真源** `idea::GIT_BRANCH_ICON` | `vcs/branch.svg` |
//! | 提交记录 | `GitGraphIcon` | `Lucide` `GitGraph` | 真源是 `Nucleo.IconGitGraphOutline18` → Lucide，**无文件** |
//! | 终端 | `TerminalWindowIcon` | `Lucide` `SquareTerminal` | 同上（`Nucleo.IconSquareTerminalOutline18` → Lucide） |
//! | 诊断 | `WarningIcon` | **真源** `idea::WARNING_CIRCLE_ICON` | `expui/general/warningDialog.svg` |
//! | 运行 | `RunIcon` | `Lucide` `Play` | 真源是内联 React 组件 `run-icon.tsx`，**只有 path 没有 SVG 文件** |
//! | Maven | `MavenIcon` | `Lucide` `Package` | 真源是内联 React 组件 `maven-icon.tsx`，同上 |
//! | 设置 | `GearIcon` | **真源** `idea::GEAR_ICON` | `expui/general/settings.svg` |
//! | 扩展（插件） | `PuzzlePieceIcon` | `Lucide` `Puzzle` | 真源经 `Nucleo` 代理解析到 Lucide `puzzle`，**已经是 1:1** |
//! | 通知 | `BellIcon` | **真源** `idea::BELL_ICON` | `expui/toolwindows/notifications.svg` |
//!
//! 出处与被否方案见 `gpui/research/icon-asset-inventory.md` 第 3.1 节。
//!
//! ## 未实现清单
//!
//! 展开态（140–320 拖拽、`layout.resizeActivityRail`）、项目圆点与横向轮播
//! （`sidebar-projects.tsx` / `handleProjectWheel`）、rail 右键菜单（`main-sidebar.tsx:710-762`）、
//! 项的禁用态（Windows 里终端在后端不可用时 `disabled:opacity-50`）、右 rail 通知的未读徽标
//! （`notifications-trigger.tsx:45`）—— 都没有画；「顶部组视图」与「侧栏是否可见」的二级拆分
//! （`sidebar-pane-selector.tsx:108-111`）由调用方决定，本条栏只按 `is_active` 逐项标底色。
//!
//! ## 选中契约：**同时可以亮多项**
//!
//! [`activity_bar`] 收的是 `is_active: impl Fn(usize) -> bool`（`items` 的下标），不是单个
//! `Option<usize>`。真机就是这么工作的：左栏**顶部组**的选中来自 `activeSidebarView`，
//! **底部组**的选中来自 `isBottomPaneVisible && bottomPaneActiveTab === "<该项>"`
//! （`main-sidebar.tsx:635-652`），两组互不影响，所以「项目」亮着的同时终端也可以亮着。
//! 单一 `Option<usize>` 表达不了这个状态，会让点终端时「项目」的高亮被顶掉。

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Placement, Selectable as _};
use gpui_kit::{
    AnyElement, App, ElementId, IntoElement, Length, ParentElement as _, Pixels, SharedString,
    Styled as _, Window, div, px, rems,
};
use lithe_gpui_shared::icons::idea;

/// 折叠态 rail 宽（`main-sidebar.tsx:97`）。展开态是 160（140–320），本函数不画。
///
/// ⚠️ **保留 `px(...)`**：38 不在 gpui 的 rem 档位上（档位里 `_9()`=36、`_10()`=40），
/// 且它参与 `column_width = 38 + 4` 的盒宽算术，没有对应的固定 helper。
const COLLAPSED_WIDTH: f32 = 38.;
/// 外壳里各栏之间的固定间隔 `--lithe-workbench-gap`
/// （`windows/tauri/src/styles/theme.css:125`；rails 用 `main-sidebar.tsx:588-590`、
/// `main-layout.tsx:299`）。只出现在 `column_width` 的算术里（见上）。
const WORKBENCH_GAP: f32 = 4.;
/// 左 rail 图标项盒宽：42（38 + 4）− 左右内衬 16 = 26（`main-sidebar.tsx:610-611`）。
///
/// ⚠️ **保留 `px(...)`**：26 不在 gpui 的 rem 档位上（档位里 `_6()`=24、`_7()`=28）。
const LEFT_ITEM_WIDTH: f32 = 26.;
/// 左 rail 图标项盒高 `min-h-6`（`main-sidebar.tsx:332` 覆盖掉组件的 `--lithe-tab-height`）= 24。
/// 24 在 rem 档位上（见 `item_button` 里的 `rems(LEFT_ITEM_HEIGHT / 16.)`，与 `h_6()` 同值）。
const LEFT_ITEM_HEIGHT: f32 = 24.;
/// 左 rail 圆角 `--lithe-chrome-radius`（`theme.css:133`，= 4）。
///
/// ⚠️ **保留 `px(...)`**：这是 Lithe 自己的圆角规格（不是 gpui 的 rem 档位）——
/// 见 [`crate::project_tabs`] 模块头「圆角换算」；`ThemeConfig.radius` 是 `usize`
/// （`gpui-component-0.6.6/src/theme/schema.rs:67-68`），也表达不了 Lithe 的
/// `--radius × k` 阶梯。
const LEFT_ITEM_RADIUS: f32 = 4.;
/// 右 rail 图标项 `Button variant=ghost size=icon-sm`（`plugin-activity-rail.tsx:39`）= 28×28。
/// 28 在 rem 档位上，调用点写 `rems(1.75)`（= `size_7()`，值随 `side` 变化所以套不了固定 helper）。
const RIGHT_ITEM_SIZE: f32 = 28.;
/// 右 rail 图标项圆角 `rounded-sm`（`plugin-activity-rail.tsx:45`）= 4.8。
///
/// ⚠️ **保留 `px(...)`**：4.8 不是 gpui 的 rem 档位（`rounded_sm()` 是 4），理由同上。
const RIGHT_ITEM_RADIUS: f32 = 4.8;
/// 右 rail 容器的右上/右下圆角 `rounded-r-xl`（`plugin-activity-rail.tsx:34`）= 11.2。
///
/// ⚠️ **保留 `px(...)`**：11.2 不是 gpui 的 rem 档位（`rounded_xl()` 是 12），理由同上。
const RIGHT_RAIL_RADIUS: f32 = 11.2;
/// 悬停底色不透明度：左 rail 是 `hover:bg-accent/70`（`ui/sidebar.tsx:241`）。
const HOVER_OPACITY: f32 = 0.7;

/// 活动栏所在的一侧。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivitySide {
    /// 左侧活动栏（Windows `SidebarActivityRail`）：透明底、无边框，图标列在 42 宽的盒内。
    Left,
    /// 右侧活动栏（Windows `PluginActivityRail`）：38 宽 + 外侧 4 间隔、1px 右边框、右侧圆角。
    Right,
}

/// 活动栏一项的图标来源。
///
/// 两个变体对应两条真源路线（见模块文档的对照表）：
///
/// - [`ActivityIcon::Lucide`]：全量 Lucide 目录（`gpui_kit::assets::IconName`）。
/// - [`ActivityIcon::Idea`]：`gpui/assets/ui-icons/idea/**` 的 IntelliJ `expui` SVG，
///   用 `lithe_gpui_shared::icons::idea::IdeaIcon` 常量引用（生成物，
///   见 `gpui/tools/generate-idea-icons.mjs`）。
///
/// **调用点只写图标名**：明暗选择、尺寸、前景色都在 [`ActivityIcon::render`] 里按
/// `idea_icon_svg` / `FileIcon::render` 的同一口径做掉（理由见 `shared::icons` 的模块文档：
/// 裸 `svg().path(..)` 缺 `text_color` 或尺寸就是**白板**，不报错也不 panic）。
#[derive(Clone, Debug)]
pub enum ActivityIcon {
    /// Lucide 字形（真源本来就没有文件的那 4 项）。
    Lucide(IconName),
    /// IntelliJ `expui` 真源 SVG（`ui-icons/idea/**`）。
    Idea(&'static idea::IdeaIcon),
}

impl ActivityIcon {
    /// 交给 `Button::icon(..)` 的图标值。
    ///
    /// 两条路都收敛到 gpui-kit 的 `Icon`：
    ///
    /// - Lucide → `IconName`（`Button::icon(impl Into<ButtonIcon>)` 直接吃它）；
    /// - expui → `Icon::default().path(资源路径)`，路径由 [`idea_icon_svg_px`] 的同一份
    ///   明暗选择逻辑给出（[`idea::IdeaIcon::path`]）。
    ///
    /// **尺寸不在这里设**：`Button` 渲染时会 `icon.with_size(icon_size)`（按 `size` 档位算），
    /// 左 rail 该项 26×24、右 rail 28×28，两者都落到 16 —— 与旧实现（`.icon(IconName)`）逐像素一致。
    /// 这样也避免把 `Icon` 的 `size` 写死而让两侧按钮尺寸算错。
    fn button_icon(&self, cx: &App) -> gpui_kit::component::Icon {
        match self {
            Self::Lucide(name) => gpui_kit::component::Icon::new(*name),
            Self::Idea(icon) => gpui_kit::component::Icon::default()
                .path(icon.path(gpui_kit::component::Theme::global(cx).is_dark())),
        }
    }
}

/// 活动栏的一个图标项。
///
/// 字段私有 + [`ActivityItem::new`] / [`ActivityItem::idea`] / [`ActivityItem::bottom`]：
/// 跨 crate 之后结构体字面量不再是合法构造方式（《编码指南》「公共 API 设计」）。
#[non_exhaustive]
pub struct ActivityItem {
    /// 图标来源，见 [`ActivityIcon`]。
    icon: ActivityIcon,
    /// 文案：中文逐字取自 `windows/tauri/src/i18n/locale.ts`（`workbench.project` = 项目 …），
    /// 同时用作 tooltip 文本与无障碍名称。
    label: SharedString,
    /// `true` = 归入**底部组**。Windows 的底部组是
    /// `SIDEBAR_BOTTOM_ACTIVITY_ITEM_IDS = [maven, run, terminal, diagnostics, gitLog, settings]`
    /// （`windows/tauri/src/features/layout/config/item-order.ts:12-19`），
    /// 顶部组只剩 files / git / search（`sidebar-pane-selector.tsx:311-319`）。
    bottom: bool,
}

impl ActivityItem {
    /// 用 Lucide 字形的项（真源没有 SVG 文件的那几项，见模块文档的表）。
    pub fn new(icon: IconName, label: impl Into<SharedString>) -> Self {
        Self {
            icon: ActivityIcon::Lucide(icon),
            label: label.into(),
            bottom: false,
        }
    }

    /// 用 `gpui/assets/ui-icons/idea/**` 真源 SVG 的项。
    ///
    /// 传的是 `idea::` 常量（生成物里的 `IdeaIcon`），**不要**手写资源路径字符串：
    /// 常量表由 `node gpui/tools/generate-idea-icons.mjs --check` 守着，路径写错会红。
    pub fn idea(icon: &'static idea::IdeaIcon, label: impl Into<SharedString>) -> Self {
        Self {
            icon: ActivityIcon::Idea(icon),
            label: label.into(),
            bottom: false,
        }
    }

    /// 归入**底部组**（non-boolean builder 用 `with_`/语义动词；这里是布尔标记，
    /// 按《编码指南》词汇表用形容词式 `bottom`）。
    pub fn bottom(mut self, bottom: bool) -> Self {
        self.bottom = bottom;
        self
    }
}

/// 画一条活动栏。
///
/// - `items` 按 Windows `SIDEBAR_ACTIVITY_ITEM_IDS` 的顺序传；`bottom` 的项自动落到底部组
///   （组内保持传入顺序）。
/// - `is_active(index)` 决定第 `index` 项（**`items` 的下标**，不是底部组的下标）是否画选中底色。
///   它是个**谓词而不是单个下标**：顶部组与底部组各自有自己的选中来源，可以同时亮多项
///   （见模块文档「选中契约」）。传 `|_| false` 就是"一项都不选中"。
/// - `on_select(index, window, cx)` 在点击时回调，`index` 同样是 `items` 的下标。
///
/// `window` 按契约保留（当前实现不需要：tooltip / 焦点环都由 `Button` 内部处理）。
pub fn activity_bar(
    side: ActivitySide,
    items: &[ActivityItem],
    is_active: impl Fn(usize) -> bool + 'static,
    on_select: impl Fn(usize, &mut Window, &mut App) + 'static,
    _window: &Window,
    cx: &App,
) -> impl IntoElement {
    let on_select: Rc<dyn Fn(usize, &mut Window, &mut App)> = Rc::new(on_select);
    // `is_active` 被每个项借用一次，而每个 `item_button` 只读借用，所以包一层 `Rc` 共享
    // （`Rc<dyn Fn>` 而不是 `&dyn Fn`：同一份谓词要在两个组里各用一遍）。
    let is_active: Rc<dyn Fn(usize) -> bool> = Rc::new(is_active);

    // 两个组各自保持 `items` 里的相对顺序。`cx` 只被 `item_button` 只读借用（取主题色）。
    let top_items: Vec<AnyElement> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.bottom)
        .map(|(index, item)| {
            item_button(side, index, item, &*is_active, on_select.clone(), cx).into_any_element()
        })
        .collect();
    let bottom_items: Vec<AnyElement> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.bottom)
        .map(|(index, item)| {
            item_button(side, index, item, &*is_active, on_select.clone(), cx).into_any_element()
        })
        .collect();
    let has_bottom_group = !bottom_items.is_empty();

    // 左：42 宽的盒（38 + 4 gap）＋ 内衬 8 → 图标项 26 宽，图标中心 x=21。
    // 右：38 宽的盒、无内衬、图标项 28 居中 → 中心 x=19；外侧再补 4px 间隔，总宽同样是 42。
    //
    // `gutter` / `outer_pad_end` 的取值随 `side` 变化，套不了固定的 rem 档位 helper，所以用
    // helper 底层的 `rems()` 表达同一个单位：`rems(0.5)` = `px_2()` 的 8、`rems(0.25)` = `pr_1()` 的 4、
    // `rems(0.)` = 0。它们仍然随主题基准字号缩放（这正是换算成 rem 的目的）。
    let (column_width, gutter, outer_pad_end) = match side {
        ActivitySide::Left => (COLLAPSED_WIDTH + WORKBENCH_GAP, rems(0.5), rems(0.)),
        ActivitySide::Right => (COLLAPSED_WIDTH, rems(0.), rems(0.25)),
    };
    let border = cx.theme().border;

    // 顶部组：`flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto`（`sidebar-pane-selector.tsx:359`）。
    // 溢出按 `overflow_hidden` 处理：gpui-kit 的滚动条要自己挂，本阶段不做。
    let top_group = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .w_full()
        .gap_1()
        .pt_1p5()
        .px(gutter)
        .overflow_hidden()
        .children(top_items);

    // 底部组：`flex shrink-0 flex-col gap-1 pt-1`（`sidebar-pane-selector.tsx:363`）。
    let bottom_group = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .w_full()
        .gap_1()
        .pt_1()
        .pb_1p5()
        .px(gutter)
        .children(bottom_items);

    let mut rail = div()
        .flex()
        .flex_col()
        .items_center()
        .h_full()
        // 42 = 38 + 4：不在 gpui 的 rem 档位上，且是运行时算出来的值，保留 `px(...)`。
        .w(px(column_width))
        .flex_shrink_0()
        .overflow_hidden()
        .child(top_group);
    if has_bottom_group {
        rail = rail.child(bottom_group);
    }
    if side == ActivitySide::Right {
        // `rounded-r-xl border-border border-r`（`plugin-activity-rail.tsx:34`）。
        // 底色不画：Windows 右 rail 的 `bg-surface` 与外壳底色同色（`main-layout.tsx:280`），
        // 这里保持透明由调用方决定外壳底色，效果一致。
        rail = rail
            .border_r_1()
            .border_color(border)
            .rounded_r(px(RIGHT_RAIL_RADIUS));
    }

    // 外侧 4px 间隔：Windows 放在父 flex row 的 `pr-(--lithe-workbench-gap)` 上
    // （`main-layout.tsx:299`），这里自带，保证两条 rail 的对外盒宽都是 42。
    div()
        .flex()
        .flex_row()
        .h_full()
        .flex_shrink_0()
        .pr(outer_pad_end)
        .child(rail)
}

/// 画一个图标项（两侧共用 `Button`，只是度量与配色不同）。
///
/// - 左 = Windows `SidebarListItem`（`iconOnly`）：`w-full min-h-6 rounded-(--lithe-chrome-radius) px-0
///   justify-center`，图标 `size-4` = 16（`windows/tauri/src/ui/sidebar.tsx:236-247`、
///   `main-sidebar.tsx:321-354`、`sidebar-pane-selector.tsx:107,327`）。
/// - 右 = `Button variant=ghost size=icon-sm`：28×28、图标 `size-4.5` = 18
///   （`plugin-activity-rail.tsx:39-48`）。gpui-kit 的图标字号只有 12 / 14 / 16 / 24 四档
///   （`gpui-component-0.6.6/src/icon.rs:181-187`），16 是离 18 最近的一档，所以右 rail 用 16。
/// - 配色：左 rail 内 `--subtle-foreground` 被覆盖成 `--foreground`
///   （`windows/tauri/src/styles/theme.css:183-186`）→ 静止前景 `theme.foreground`，
///   悬停 `bg-accent/70`、选中 `bg-accent`（`ui/sidebar.tsx:240-245`）；
///   右 rail 静止是 `--subtle-foreground` → `theme.muted_foreground`，
///   悬停/选中都是 `bg-accent`（`windows/tauri/src/ui/button.tsx:17`）。
/// - `ButtonCustomVariant` 只有一个前景色，所以右 rail 悬停时前景不会跟着变亮
///   （Windows 会变成 `--foreground`）—— 这是已知偏差。
fn item_button(
    side: ActivitySide,
    index: usize,
    item: &ActivityItem,
    is_active: &dyn Fn(usize) -> bool,
    on_select: Rc<dyn Fn(usize, &mut Window, &mut App)>,
    cx: &App,
) -> Button {
    let theme = cx.theme();
    let accent = theme.accent;

    let (foreground, hover_background) = match side {
        ActivitySide::Left => (theme.foreground, accent.opacity(HOVER_OPACITY)),
        ActivitySide::Right => (theme.muted_foreground, accent),
    };
    // tooltip 朝栏内：左 rail `tooltipSide="right"`、右 rail `"left"`
    // （`sidebar-pane-selector.tsx:106`、`plugin-activity-rail.tsx:42`）。
    // 宽度/高度是随 `side` 变化的运行时值，两边类型要一致，所以统一收敛到 `Length`：
    // px 值走 `rems(P / 16.)`（与 `_N()` 同值），档位外的 26 保留 `px(...)`。
    let (id_name, placement, width, height, radius): (&str, Placement, Length, Length, Pixels) =
        match side {
            // 26×24：26 不在 gpui 的 rem 档位上（档位里 24 / 28），所以保留 `px(...)`；
            // 24 在档位上 → `rems(24. / 16.)` 与 `h_6()` 同值。
            ActivitySide::Left => (
                "lithe-activity-bar-left",
                Placement::Right,
                px(LEFT_ITEM_WIDTH).into(),
                rems(LEFT_ITEM_HEIGHT / 16.).into(),
                px(LEFT_ITEM_RADIUS),
            ),
            // 28 = `w-7` → `rems(28. / 16.)` 与 `size_7()` 同值。
            ActivitySide::Right => (
                "lithe-activity-bar-right",
                Placement::Left,
                rems(RIGHT_ITEM_SIZE / 16.).into(),
                rems(RIGHT_ITEM_SIZE / 16.).into(),
                px(RIGHT_ITEM_RADIUS),
            ),
        };

    let is_active = is_active(index);
    Button::new(ElementId::named_usize(id_name, index))
        .custom(
            ButtonCustomVariant::new(cx)
                .foreground(foreground)
                .hover(hover_background)
                .active(accent),
        )
        // `Selectable::selected` 出选中底色；`toggled` 只补 `aria-pressed`（`button.rs:477-486`）。
        .selected(is_active)
        .toggled(is_active)
        // 图标：真源（expui）与 Lucide 都交给 `Button::icon(..)`，见 [`ActivityIcon::button_icon`]。
        .icon(item.icon.button_icon(cx))
        .tooltip(item.label.clone())
        .tooltip_placement(placement)
        .accessibility_label(item.label.clone())
        .rounded(radius)
        .w(width)
        .h(height)
        .on_click(move |_event, window, cx| on_select(index, window, cx))
}
