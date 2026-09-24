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
//! ## 图标：一律取全量 Lucide 目录里的真实字形
//!
//! 应用注册的是 `gpui_kit::assets::AllAssets`（`shell_probe/mod.rs`），它嵌入
//! `gpui-kit-assets-0.6.6/assets/icons/` 下的全部 **1830 个 Lucide SVG**
//! （`src/native_assets.rs:8-11`），而 `gpui_kit::assets::IconName` 正是按这份目录生成的完整枚举
//! （`build.rs:20-51`，变体名 = svg 文件名的 PascalCase）。所以 **`ActivityItem::icon` 用
//! `gpui_kit::assets::IconName`**，不再需要"默认图标集里没有某字形"的替代：
//!
//! | Windows 用途 | Windows 图标组件 | 采用（`gpui_kit::assets::IconName`） | 字形文件 |
//! | --- | --- | --- | --- |
//! | 项目 | `FilesIcon` | `FolderOpen` | `icons/folder-open.svg` |
//! | 搜索 | `MagnifyingGlassIcon` | `Search` | `icons/search.svg` |
//! | 更改（Git） | `GitBranchIcon` | `GitBranch` | `icons/git-branch.svg` |
//! | 提交记录 | `GitGraphIcon` | `GitGraph` | `icons/git-graph.svg` |
//! | 终端 | `TerminalWindowIcon` | `SquareTerminal` | `icons/square-terminal.svg` |
//! | 诊断 | `WarningIcon` | `TriangleAlert` | `icons/triangle-alert.svg`（Lucide `triangle-alert`） |
//! | 运行 | `RunIcon` | `Play` | `icons/play.svg` |
//! | Maven | `MavenIcon` | `Package` | `icons/package.svg`（Lucide 无 Maven 字形，取"包/构建产物"语义） |
//! | 设置 | `GearIcon` | `Settings` | `icons/settings.svg` |
//! | 扩展（插件） | `PuzzlePieceIcon` | `Puzzle` | `icons/puzzle.svg` |
//! | 通知 | `BellIcon` | `Bell` | `icons/bell.svg` |
//!
//! ## 未实现清单
//!
//! 展开态（140–320 拖拽、`layout.resizeActivityRail`）、项目圆点与横向轮播
//! （`sidebar-projects.tsx` / `handleProjectWheel`）、rail 右键菜单（`main-sidebar.tsx:710-762`）、
//! 项的禁用态（Windows 里终端在后端不可用时 `disabled:opacity-50`）、右 rail 通知的未读徽标
//! （`notifications-trigger.tsx:45`）—— 都没有画；`active` 也只能标一项，没有"侧栏是否可见"的
//! 二级选中逻辑（`sidebar-pane-selector.tsx:108-111`）。

use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Placement, Selectable as _};
use gpui_kit::{
    AnyElement, App, ElementId, IntoElement, ParentElement as _, SharedString, Styled as _, Window,
    div, px,
};

/// 折叠态 rail 宽（`main-sidebar.tsx:97`）。展开态是 160（140–320），本函数不画。
const COLLAPSED_WIDTH: f32 = 38.;
/// 外壳里各栏之间的固定间隔 `--lithe-workbench-gap`
/// （`windows/tauri/src/styles/theme.css:125`；rails 用 `main-sidebar.tsx:588-590`、
/// `main-layout.tsx:299`）。
const WORKBENCH_GAP: f32 = 4.;
/// 左 rail 内容层的左右内衬 `ACTIVITY_RAIL_HORIZONTAL_GUTTER`（`main-sidebar.tsx:101`）。
const HORIZONTAL_GUTTER: f32 = 8.;
/// 同组图标项之间的间隔 `gap-1`（`sidebar-pane-selector.tsx:359`、`:363`）。
const ITEM_GAP: f32 = 4.;
/// 左 rail 面板顶部内衬 `pt-1.5`（`main-sidebar.tsx:604`）。
const PANEL_PAD_TOP: f32 = 6.;
/// 左 rail 面板底部内衬 `pb-1.5`（`main-sidebar.tsx:605`；折叠态走 else 分支）。
const PANEL_PAD_BOTTOM: f32 = 6.;
/// 底部组自己的顶部内衬 `pt-1`（`sidebar-pane-selector.tsx:363`）。
const BOTTOM_GROUP_PAD_TOP: f32 = 4.;
/// 左 rail 图标项盒宽：42（38 + 4）− 左右内衬 16 = 26（`main-sidebar.tsx:610-611`）。
const LEFT_ITEM_WIDTH: f32 = 26.;
/// 左 rail 图标项盒高 `min-h-6`（`main-sidebar.tsx:332` 覆盖掉组件的 `--lithe-tab-height`）。
const LEFT_ITEM_HEIGHT: f32 = 24.;
/// 左 rail 圆角 `--lithe-chrome-radius`（`theme.css:133`，= 4）。
const LEFT_ITEM_RADIUS: f32 = 4.;
/// 右 rail 图标项 `Button variant=ghost size=icon-sm`（`plugin-activity-rail.tsx:39`）= 28×28。
const RIGHT_ITEM_SIZE: f32 = 28.;
/// 右 rail 图标项圆角 `rounded-sm`（`plugin-activity-rail.tsx:45`）= 4.8。
const RIGHT_ITEM_RADIUS: f32 = 4.8;
/// 右 rail 容器的右上/右下圆角 `rounded-r-xl`（`plugin-activity-rail.tsx:34`）= 11.2。
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

/// 活动栏的一个图标项。
///
/// 图标类型是全量目录的 `gpui_kit::assets::IconName`（1830 个变体，derive
/// `Clone + Copy + Debug`，`gpui-kit-assets-0.6.6/build.rs:52-53`）；本结构体仍然不 derive
/// 它们，因为 `SharedString` 不是 `Copy`。
pub struct ActivityItem {
    /// 图标。取 `gpui_kit::assets::IconName`，见模块文档的对照表。
    pub icon: IconName,
    /// 文案：中文逐字取自 `windows/tauri/src/i18n/locale.ts`（`workbench.project` = 项目 …），
    /// 同时用作 tooltip 文本与无障碍名称。
    pub label: SharedString,
    /// `true` = 归入**底部组**。Windows 的底部组是
    /// `SIDEBAR_BOTTOM_ACTIVITY_ITEM_IDS = [maven, run, terminal, diagnostics, gitLog, settings]`
    /// （`windows/tauri/src/features/layout/config/item-order.ts:12-19`），
    /// 顶部组只剩 files / git / search（`sidebar-pane-selector.tsx:311-319`）。
    pub bottom: bool,
}

/// 画一条活动栏。
///
/// - `items` 按 Windows `SIDEBAR_ACTIVITY_ITEM_IDS` 的顺序传；`bottom` 的项自动落到底部组
///   （组内保持传入顺序）。
/// - `active` 是 **`items` 的下标**（不是底部组的下标）。
/// - `on_select(index, window, cx)` 在点击时回调，`index` 同样是 `items` 的下标。
///
/// `window` 按契约保留（当前实现不需要：tooltip / 焦点环都由 `Button` 内部处理）。
pub fn activity_bar(
    side: ActivitySide,
    items: &[ActivityItem],
    active: Option<usize>,
    on_select: impl Fn(usize, &mut Window, &mut App) + 'static,
    _window: &Window,
    cx: &App,
) -> impl IntoElement {
    let on_select: Rc<dyn Fn(usize, &mut Window, &mut App)> = Rc::new(on_select);

    // 两个组各自保持 `items` 里的相对顺序。`cx` 只被 `item_button` 只读借用（取主题色）。
    let top_items: Vec<AnyElement> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.bottom)
        .map(|(index, item)| {
            item_button(side, index, item, active, on_select.clone(), cx).into_any_element()
        })
        .collect();
    let bottom_items: Vec<AnyElement> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.bottom)
        .map(|(index, item)| {
            item_button(side, index, item, active, on_select.clone(), cx).into_any_element()
        })
        .collect();
    let has_bottom_group = !bottom_items.is_empty();

    // 左：42 宽的盒（38 + 4 gap）＋ 内衬 8 → 图标项 26 宽，图标中心 x=21。
    // 右：38 宽的盒、无内衬、图标项 28 居中 → 中心 x=19；外侧再补 4px 间隔，总宽同样是 42。
    let (column_width, gutter, outer_pad_end) = match side {
        ActivitySide::Left => (COLLAPSED_WIDTH + WORKBENCH_GAP, HORIZONTAL_GUTTER, 0.),
        ActivitySide::Right => (COLLAPSED_WIDTH, 0., WORKBENCH_GAP),
    };
    let border = cx.theme().border;

    // 顶部组：`flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto`（`sidebar-pane-selector.tsx:359`）。
    // 溢出按 `overflow_hidden` 处理：gpui-kit 的滚动条要自己挂，本阶段不做。
    let top_group = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h(px(0.))
        .w_full()
        .gap(px(ITEM_GAP))
        .pt(px(PANEL_PAD_TOP))
        .px(px(gutter))
        .overflow_hidden()
        .children(top_items);

    // 底部组：`flex shrink-0 flex-col gap-1 pt-1`（`sidebar-pane-selector.tsx:363`）。
    let bottom_group = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .w_full()
        .gap(px(ITEM_GAP))
        .pt(px(BOTTOM_GROUP_PAD_TOP))
        .pb(px(PANEL_PAD_BOTTOM))
        .px(px(gutter))
        .children(bottom_items);

    let mut rail = div()
        .flex()
        .flex_col()
        .items_center()
        .h_full()
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
        .pr(px(outer_pad_end))
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
    active: Option<usize>,
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
    let (id_name, placement, width, height, radius) = match side {
        ActivitySide::Left => (
            "lithe-activity-bar-left",
            Placement::Right,
            LEFT_ITEM_WIDTH,
            LEFT_ITEM_HEIGHT,
            LEFT_ITEM_RADIUS,
        ),
        ActivitySide::Right => (
            "lithe-activity-bar-right",
            Placement::Left,
            RIGHT_ITEM_SIZE,
            RIGHT_ITEM_SIZE,
            RIGHT_ITEM_RADIUS,
        ),
    };

    let is_active = active == Some(index);
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
        .icon(item.icon.clone())
        .tooltip(item.label.clone())
        .tooltip_placement(placement)
        .accessibility_label(item.label.clone())
        .rounded(px(radius))
        .w(px(width))
        .h(px(height))
        .on_click(move |_event, window, cx| on_select(index, window, cx))
}
