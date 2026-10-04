//! 左右活动栏（图标竖条）。
//!
//! **无状态**：选中项由 `active` 传入，点击通过 `on_select` 交回调用方，本文件不持有任何状态。
//! 各常量的出处写在它们旁边；**规格真源是 `macos/`**（`lithe-gpui/UI-MAP.md` §2）。其中不少
//! 常量标的仍是**已作废**的 `windows/tauri` 前端 —— 那是本文件的历史出处，不再算真源；
//! rail 宽已按真源对齐（见下）。
//!
//! ## 折叠 / 展开是两个宽度（容易混）
//!
//! - 折叠态 rail 宽 = [`design::ACTIVITY_RAIL_WIDTH_SPEC`] = **40**：真源
//!   `macos/Sources/Lithe/Views/Workbench/WorkbenchView.swift:7`（右）与 `:12`（左）**各自**
//!   都是 40；完整出处、对 B14 原记"真源只定义右侧"的补正、以及维护者 2026-10-03 的裁决
//!   （选 (a)）都写在那个 owner 的文档里。**改前的 38 出自已作废的**
//!   `windows/tauri/.../sidebar/main-sidebar.tsx:97`；
//! - 展开态默认 **160**、min **140**、max **320**（同文件 `:98-100`，已作废前端）；
//! - `MainLayout` 固定传 `expanded={false}`（`.../components/main-layout.tsx:302`），
//!   所以 Windows 产品里只有折叠态。
//!
//! **本函数只画折叠态**：展开态还带项目圆点、拖拽热区（宽 4、悬停 `primary/8`）、横向轮播
//! （`main-sidebar.tsx:689-708`、`sidebar-projects.tsx`），未实现。
//!
//! ## 盒宽算术
//!
//! - 左 rail 盒宽 = rail 宽 + `WORKBENCH_GAP` = 40 + 4 = **44**（改前 38 + 4 = 42）；右 rail
//!   盒宽 = **40**，外侧再补 4 ⇒ 两条 rail 的对外盒宽都是 **44**（维护者裁决 (a) 的直接结果，
//!   见 `BLOCKERS.md` B14）。
//! - **项盒几何照真源**（2026-10-04 换源，见 [`ITEM_SIZE`]）：30×30、圆角 6，装在 37×40 的槽里
//!   （`WorkbenchView.swift:11-20` 的 `ActivityBarMetrics`），槽之间无间隙 ⇒ 纵向节距 **40**。
//!   此前那套"左内衬 8 → 项盒 26、右项盒 28"出自**已作废**的 Windows 前端
//!   （`main-sidebar.tsx:610-611`、`plugin-activity-rail.tsx:39`），已删除。
//!   因此 `BLOCKERS.md` B12 里"要动项盒得先有真机观察"这条**前置不再需要** —— 真源源码已给足数值。
//! - 左 rail 比真源宽 4px（B14），所以左侧图标比真源偏右约 2px；这是那条决定的已知代价。
//!
//! ## 图标：真源优先，缺真源的保持 Lucide（逐项登记）
//!
//! 每个项的画法由 [`ActivityIcon`] 决定，有两条路：
//!
//! 1. **真源**（[`ActivityIcon::Idea`]）—— `lithe-gpui/assets/icons/idea/icons/**` 的 IntelliJ `expui` SVG，
//!    经 `lithe_gpui_shared::icons::idea_icon_svg` 按当前主题明暗挑路径渲染（单色，
//!    颜色取 `theme.foreground`/`muted_foreground`，与旧实现的 `IconName` 同口径）。
//! 2. **Lucide**（[`ActivityIcon::Lucide`]）—— 全量 Lucide 目录（1830 个字形）。
//!    留在这条路上的项都有明确理由，逐条写在 [`crate::workspace`] 的 `activity_items()` 调用处。
//!
//! | Windows 用途 | Windows 图标组件 | 现在用 | 真源文件 / 理由 |
//! | --- | --- | --- | --- |
//! | 项目 | `FilesIcon` | **真源** `idea::FILES_ICON` | `icons/idea/icons/ui/listFiles.svg`（+`_dark`） |
//! | 搜索 | `MagnifyingGlassIcon` | **真源** `idea::MAGNIFYING_GLASS_ICON` | `ui/search.svg` |
//! | 更改（Git） | `GitBranchIcon` | **真源** `idea::GIT_BRANCH_ICON` | `ui/branch.svg` |
//! | 提交记录 | `GitGraphIcon` | `Lucide` `GitGraph` | 真源是 `Nucleo.IconGitGraphOutline18` → Lucide，**无文件** |
//! | 终端 | `TerminalWindowIcon` | `Lucide` `SquareTerminal` | 同上（`Nucleo.IconSquareTerminalOutline18` → Lucide） |
//! | 诊断 | `WarningIcon` | **真源** `idea::WARNING_CIRCLE_ICON` | `ui/warningDialog.svg` |
//! | 运行 | `RunIcon` | **真源** `idea::PLAY_ICON` | `ui/run.svg`（+`_dark`）—— 真源 `run-icon.tsx` 只有内联 path，但包里本来就有这张彩色运行三角，2026-09-30 起改用它（此前误登记为 Lucide `Play`） |
//! | Maven | `MavenIcon` | **真源** `idea::MAVEN_ICON` | `files/maven.svg`（+`_dark`）—— Maven 蓝色标志（`#3574F0`），与 IDEA 工具窗同物；2026-09-30 改（此前用 Lucide `Package` 立方体） |
//! | 设置 | `GearIcon` | **真源** `idea::GEAR_ICON` | `ui/settings.svg` |
//! | 扩展（插件） | `PuzzlePieceIcon` | `Lucide` `Puzzle` | 真源经 `Nucleo` 代理解析到 Lucide `puzzle`，**已经是 1:1** |
//! | 通知 | `BellIcon` | **真源** `idea::BELL_ICON` | `ui/notifications.svg` |
//!
//! 出处与被否方案见 `lithe-gpui/research/icon-asset-inventory.md` 第 3.1 节。
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
use gpui_kit::component::badge::Badge;
use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Placement, Selectable as _};
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, Hsla, IntoElement, Length, ParentElement as _, Pixels,
    SharedString, Styled as _, Window, div, rems,
};
use lithe_gpui_shared::design;
use lithe_gpui_shared::icons::{idea, idea_icon_image};

/// 折叠态 rail 宽 = [`design::ACTIVITY_RAIL_WIDTH_SPEC`]（**40**；真源
/// `WorkbenchView.swift:7`（右）/`:12`（左））。
///
/// 保留这个具名常量只为**锚定调用点**（同 `CONTROL_RADIUS` 在 `workbench` 各处的别名口径）：
/// 数字本身只写在 owner 里，这里不再有第二份。展开态是 160（140–320），本函数不画。
const COLLAPSED_WIDTH: f32 = design::ACTIVITY_RAIL_WIDTH_SPEC;
/// 外壳里各栏之间的固定间隔 `--lithe-workbench-gap`
/// （`windows/tauri/src/styles/theme.css:125`；rails 用 `main-sidebar.tsx:588-590`、
/// `main-layout.tsx:299`）。只出现在左轨盒宽的算术里（见 `rail_width_spec`）。
///
/// ⚠️ 出处是**已作废**的 Windows 前端；真源 `WorkbenchView.swift:257` 那条
/// `HStack(spacing: 0)` **没有**这个 4px 间隔（真源侧 rail 只有它自己那 40）⇒ 左轨多出的这 4px
/// 是宿主的布局结构，**本轮 B14 不动它**（已记进 B14 的"仍未解决"）。
const WORKBENCH_GAP: f32 = 4.;
/// 图标项盒 **30×30**、圆角 **6**：真源 `WorkbenchView.swift:11-20` 的 `ActivityBarMetrics`
/// （`buttonWidth` / `buttonHeight` = 30）与 `:1283-1287`（左）/ `:1489-1493`（右）的
/// `.litheRowHover(cornerRadius: 6, activeBackground: LitheTheme.selection)`。
///
/// ⚠️ **2026-10-04 换源**：此前两条 rail 各有一套数字（左 26×24 / 圆角 4，右 28×28 / 圆角 4.8），
/// 出处是**已作废**的 Windows 前端（`main-sidebar.tsx:610-611`、`plugin-activity-rail.tsx:39,45`）。
/// 真源两条 rail 的项盒与圆角**完全相同**，所以这里收敛成一份，不再按 `side` 分叉。
/// 30 不在 gpui 的 rem 档位上（档位是 28 / 32），所以调用点写 `rems(30. / 16.)`；
/// 圆角走 [`crate::rem_px`]（`Button::rounded` 是**固有方法**、只吃 `Pixels`）。
const ITEM_SIZE: f32 = 30.;
/// 图标项圆角 = 6（真源 `WorkbenchView.swift:1285` / `:1491` 的 `cornerRadius: 6`）。
///
/// 6 与 `design::CONTROL_RADIUS`(6.4) 不是同一个值：后者是从作废 Windows 的 `--radius × k`
/// 阶梯推出来的。把真源那 4 条圆角（5 / 10 / 9 / 6）收敛成语义 role 是**同一批的后续项**，
/// 本处先按真源取值，不提前发明 role。
const ITEM_RADIUS: f32 = 6.;
/// 真源槽位 `slotWidth × slotHeight`（`WorkbenchView.swift:19-20`）：30×30 的按钮居中在槽里，
/// 槽之间是 `VStack(spacing: 0)`（`:1265-1266`）⇒ **项的纵向节距 = 40**，不另加 gap。
const SLOT_WIDTH: f32 = 37.;
const SLOT_HEIGHT: f32 = 40.;
// 右 rail 的 `rounded-r-xl`（`RIGHT_RAIL_RADIUS = 11.2`，真源 `plugin-activity-rail.tsx:34`）与
// 它那条 `border-r` **2026-09-30 一并删除**（维护者截图口径）：
//
// 1. 先只去掉圆角 —— rail 没有上/下边框，圆角在右边界两端各留一段与直线接不上的弧，
//    看上去像"线顶上多出来的一小截弯的"；
// 2. 那条右边界本身也去掉 —— 它通高、右边只剩 4px 间隔就是窗口边缘，像桌面上多出来的一根线。
//
// 现在右 rail 没有任何边框（底色本来就不画，见 `activity_bar` 里那段说明），
// 与左 rail 一致：图标条自己不画线，线由相邻面板的边框给。

/// 悬停底色不透明度：左 rail 是 `hover:bg-accent/70`（`ui/sidebar.tsx:241`）。
const HOVER_OPACITY: f32 = 0.7;

/// 活动栏所在的一侧。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivitySide {
    /// 左侧活动栏（Windows `SidebarActivityRail`）：透明底、无边框，图标列在 44 宽的盒内
    /// （40 rail + 4 `WORKBENCH_GAP`）。
    Left,
    /// 右侧活动栏（Windows `PluginActivityRail`）：40 宽 + 外侧 4 间隔、**无边框**
    /// （真源是 `rounded-r-xl` + `border-r`，两条本侧都不画 —— 见上面 `RIGHT_RAIL_RADIUS` 的墓碑注释）。
    Right,
}

/// 一条 rail 的**盒宽**（规格像素，**不含**右轨外侧那个 `outer_pad_end` 的 4px 间隔）。
///
/// 左 = owner + `WORKBENCH_GAP`、右 = owner —— 两侧都从 [`design::ACTIVITY_RAIL_WIDTH_SPEC`]
/// 派生（维护者 2026-10-03 裁决 B14 (a)：宿主左右同宽）。抽成函数只为让"左右两侧都来自同一个
/// owner"这条关系**可被单测钉住**（见本文件末尾的 `tests`），不引入新的宽度概念。
const fn rail_width_spec(side: ActivitySide) -> f32 {
    match side {
        ActivitySide::Left => COLLAPSED_WIDTH + WORKBENCH_GAP,
        ActivitySide::Right => COLLAPSED_WIDTH,
    }
}

/// 活动栏一项的图标来源。
///
/// 两个变体对应两条真源路线（见模块文档的对照表）：
///
/// - [`ActivityIcon::Lucide`]：全量 Lucide 目录（`gpui_kit::assets::IconName`）。
/// - [`ActivityIcon::Idea`]：`lithe-gpui/assets/icons/idea/icons/**` 的 IntelliJ `expui` SVG，
///   用 `lithe_gpui_shared::icons::idea::IdeaIcon` 常量引用（生成物，
///   见 `lithe-gpui/tools/generate-idea-icons.mjs`）。
///
/// **调用点只写图标名**：明暗选择、尺寸、颜色都由 [`ActivityIcon::apply`] 做掉 ——
/// expui 走 [`idea_icon_image`]（彩色 `img()`，原色保留），Lucide 走 `.icon(..)`（单色 + 主题前景色）。
/// 理由见 `shared::icons` 的模块文档：裸 `svg().path(..)` 缺 `text_color` 或尺寸就是**白板**，
/// 不报错也不 panic。
#[derive(Clone, Debug)]
pub enum ActivityIcon {
    /// Lucide 字形（真源本来就没有文件的那 4 项）。
    Lucide(IconName),
    /// IntelliJ `expui` 真源 SVG（`icons/idea/icons/**`）。
    Idea(&'static idea::IdeaIcon),
}

impl ActivityIcon {
    /// 把图标装进轨道项按钮（`Button::child`），**两条路各自用对的管线**：
    ///
    /// - [`Self::Idea`]（IntelliJ expui）：走 [`idea_icon_image`] 的 `img()` 管线 → **保留原色**。
    ///   这些资产本身是彩色的（157 个 SVG、0 个 `currentColor`、每个都有明暗两套），
    ///   走单色管线就会变成"图标全是灰的"（2026-09-30 维护者口径：图标上彩色）。
    /// - [`Self::Lucide`]：是**单色字形**，本来就没有颜色可保留 → 继续 `.icon(..)`，
    ///   照旧按按钮尺寸档位定大小、按主题前景色染色。
    ///
    /// ⚠️ 彩色那条**不跟随 `text_color`**：悬停 / 选中的反馈由**底色**承担
    /// （`ButtonCustomVariant::hover` / `Button::selected`），与 IDEA New UI 一致。
    /// 所以不要再给这一项加"图标变色表示选中"的样式 —— 那对 expui 图标不会生效。
    ///
    /// 尺寸：expui 那条**自己带尺寸**（`rems(1.25)` = 20，与 `Button` 按档位算出来的 16 一致），
    /// 因为 `Button::child` 不会像 `.icon(..)` 那样替它设尺寸。
    fn apply(&self, button: Button, cx: &App) -> Button {
        match self {
            Self::Lucide(name) => button.icon(*name),
            Self::Idea(icon) => button.child(idea_icon_image(icon, cx, rems(1.25))),
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
    /// 文案：中文逐字取自 `crates/shared/locales/source/locale.ts`（`workbench.project` = 项目 …），
    /// 同时用作 tooltip 文本与无障碍名称。
    label: SharedString,
    /// `true` = 归入**底部组**。Windows 的底部组是
    /// `SIDEBAR_BOTTOM_ACTIVITY_ITEM_IDS = [maven, run, terminal, diagnostics, gitLog, settings]`
    /// （`windows/tauri/src/features/layout/config/item-order.ts:12-19`），
    /// 顶部组只剩 files / git / search（`sidebar-pane-selector.tsx:311-319`）。
    bottom: bool,
    /// 右上角角点的颜色（`None` = 不画）。
    ///
    /// 只有铃铛用得上（`workspace::right_activity_items()` 里那一个），所以做成"一个可选
    /// 颜色"而不是"一个通知专用字段" —— 这样 `activity_bar` 不必知道通知中心存在。
    ///
    /// **颜色而不是数字**：IDEA 的铃铛角标是**一个点**，「蓝点 = 常规事件、红点 = 错误」
    /// （<https://www.jetbrains.com/help/idea/notifications.html>）。不画数字是因为数字要
    /// 一整套 read/unread 状态机，而那套状态机唯一的实际效果就是
    /// `lithe-gpui/docs/archive/ui-map-macos.md:96` 点名的缺陷（未读红点形同虚设）。决策见
    /// `.agents/notes/implemented/architecture/2026-09-27-gpui-notification-center.md`。
    badge: Option<Hsla>,
}

impl ActivityItem {
    /// 用 Lucide 字形的项（真源没有 SVG 文件的那几项，见模块文档的表）。
    pub fn new(icon: IconName, label: impl Into<SharedString>) -> Self {
        Self {
            icon: ActivityIcon::Lucide(icon),
            label: label.into(),
            bottom: false,
            badge: None,
        }
    }

    /// 用 `lithe-gpui/assets/icons/idea/icons/**` 真源 SVG 的项。
    ///
    /// 传的是 `idea::` 常量（生成物里的 `IdeaIcon`），**不要**手写资源路径字符串：
    /// 常量表由 `node lithe-gpui/tools/generate-idea-icons.mjs --check` 守着，路径写错会红。
    pub fn idea(icon: &'static idea::IdeaIcon, label: impl Into<SharedString>) -> Self {
        Self {
            icon: ActivityIcon::Idea(icon),
            label: label.into(),
            bottom: false,
            badge: None,
        }
    }

    /// 归入**底部组**（non-boolean builder 用 `with_`/语义动词；这里是布尔标记，
    /// 按《编码指南》词汇表用形容词式 `bottom`）。
    pub fn bottom(mut self, bottom: bool) -> Self {
        self.bottom = bottom;
        self
    }

    /// 在这一项右上角画一个角点（`color` = 主题色；传 `None` 的那支由调用方自己决定要不要画）。
    ///
    /// 画法用 gpui-kit 的 `Badge`：它的根是 `div().relative()`，角点自己绝对定位到
    /// `top_0().right_0()`（`gpui-component-0.6.6/src/badge.rs:114-163`），所以**必须由
    /// `Badge` 包住 `Button`**，不能反过来把 `Badge` 塞进 `Button` 的 children ——
    /// `Button` 的子元素落在 `h_flex().overflow_hidden()` 里（`button.rs:698-733`），会被裁掉。
    pub fn badge(mut self, color: Hsla) -> Self {
        self.badge = Some(color);
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
/// - `on_select(index, event, window, cx)` 在点击时回调，`index` 同样是 `items` 的下标，
///   `event` 原样传下去 —— 调用方要"这一次点击落在哪里"（诊断行的坐标，见
///   `right_tool_window::diagnose`）只能从这里拿：`ClickEvent::mouse_position()` 在键盘激活
///   （Enter / Space）时是 `None`，而 `Window::mouse_position()` 那种写法会给键盘激活配上一个
///   **上一次**指针位置的旧坐标，把"两行坐标相同 ⇒ 同一次派发"这条判据污染成假证据。
///
/// `window` 只用于取**运行时 rem 基准**（`window.rem_size()`）：两个 rail 项的圆角挂在
/// `Button::rounded` 上，而它是**固有方法**、只吃 `Pixels`（见 [`crate::rem_px`]）；
/// tooltip / 焦点环本身仍由 `Button` 内部处理。
pub fn activity_bar(
    side: ActivitySide,
    items: &[ActivityItem],
    is_active: impl Fn(usize) -> bool + 'static,
    on_select: impl Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    let rem = window.rem_size();
    let on_select: Rc<dyn Fn(usize, &ClickEvent, &mut Window, &mut App)> = Rc::new(on_select);
    // `is_active` 被每个项借用一次，而每个 `item_button` 只读借用，所以包一层 `Rc` 共享
    // （`Rc<dyn Fn>` 而不是 `&dyn Fn`：同一份谓词要在两个组里各用一遍）。
    let is_active: Rc<dyn Fn(usize) -> bool> = Rc::new(is_active);

    // 两个组各自保持 `items` 里的相对顺序。`cx` 只被 `item_button` 只读借用（取主题色）。
    let top_items: Vec<AnyElement> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| !item.bottom)
        .map(|(index, item)| {
            item_button(side, index, item, &*is_active, on_select.clone(), rem, cx)
                .into_any_element()
        })
        .collect();
    let bottom_items: Vec<AnyElement> = items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.bottom)
        .map(|(index, item)| {
            item_button(side, index, item, &*is_active, on_select.clone(), rem, cx)
                .into_any_element()
        })
        .collect();
    let has_bottom_group = !bottom_items.is_empty();

    // 槽位在 rail 里的横向内衬 =（rail 宽 − 槽宽）/ 2：左 rail 44 → 3.5，右 rail 40 → 1.5。
    // 真源 `WorkbenchView.swift:1342` 的左 rail 只有 40、槽 37 居中 ⇒ 内衬 1.5；本侧左 rail
    // 多出的 4px 是 **B14「左右对外盒宽同宽」** 的既定决定，所以左侧图标比真源偏右约 2px ——
    // 这条偏差归 B14，不在这里私自改算术。
    //
    // 内衬随 `side` 变化，套不了固定 rem 档位 helper，所以用 helper 底层的 `rems()`：
    // `rems(3.5 / 16.)` 在基准 16px 下与 `px(3.5)` 逐像素相等，且随主题字号缩放。
    let column_width = rail_width_spec(side);
    let slot_inset = rems(((column_width - SLOT_WIDTH) / 2.) / 16.);
    // 外侧 4px：左 rail 的 gap 记在它的盒宽里（见 `rail_width_spec`），右 rail 记在尾部内衬上，
    // 两者对外盒宽都是 44（B14）。
    let outer_pad_end = match side {
        ActivitySide::Left => rems(0.),
        ActivitySide::Right => rems(0.25),
    };

    // 顶部组：真源是 `VStack(spacing: 0)`（`WorkbenchView.swift:1265-1266`）—— 槽之间**无间隙**，
    // 也没有顶部内衬。真源 `ScrollView` 那一段（`:1310-1332`）在本侧换成 `overflow_hidden`：
    // gpui-kit 的滚动条要自己挂，本阶段不做。
    let top_group = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .w_full()
        .px(slot_inset)
        .overflow_hidden()
        .children(top_items);

    // 底部组：真源里紧随其后的是 `ScrollView`（贡献项，`minHeight: 280`）与末位的设置按钮，
    // 同样是 `spacing: 0`（`WorkbenchView.swift:1310-1341`）。本侧分组结构保持不变，只把已作废
    // Windows 前端的 `gap-1` / `pt-1` / `pb-1.5` 去掉 —— 否则项距会大于真源，与槽高 40 的节距冲突。
    let bottom_group = div()
        .flex()
        .flex_col()
        .flex_shrink_0()
        .w_full()
        .px(slot_inset)
        .children(bottom_items);

    let mut rail = div()
        .flex()
        .flex_col()
        .items_center()
        .h_full()
        // 44 = 40 + 4（右轨是 40 + 外侧 4）：两条 rail 的对外盒宽都是 44，但宽度随 `side`
        // 由常量算出来（见 `rail_width_spec`），套不了固定 helper；44 虽在档位表上（`w_11()`），
        // 仍按 helper 底层的 `rems(P / 16.)` 表达（基准 16px 时与 `px(44.)` 逐像素相等，
        // 且随主题字号缩放）。
        .w(rems(column_width / 16.))
        .flex_shrink_0()
        .overflow_hidden()
        .child(top_group);
    if has_bottom_group {
        rail = rail.child(bottom_group);
    }
    // 右 rail 的 `border-r` **不画**（真源 `plugin-activity-rail.tsx:34` 有 `border-border border-r`）：
    // 它是窗口最右边的一条通高竖线，右边只剩 4px 间隔就是窗口边缘，看起来像桌面上多出来的一根线
    // （2026-09-30 维护者截图圈出后去掉）。底色同样不画：真源 `bg-surface` 与外壳底色同色
    // （`main-layout.tsx:280`），这里保持透明由调用方决定外壳底色，效果一致。

    // 外侧 4px 间隔：Windows 放在父 flex row 的 `pr-(--lithe-workbench-gap)` 上
    // （`main-layout.tsx:299`），这里自带，保证两条 rail 的对外盒宽都是 44。
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
    on_select: Rc<dyn Fn(usize, &ClickEvent, &mut Window, &mut App)>,
    rem: Pixels,
    cx: &App,
) -> AnyElement {
    let theme = cx.theme();
    let accent = theme.accent;
    // 选中底 = **不透明的强调色**：真源 `LitheTheme.swift:143` 就是 `selection: accent`，
    // 而 gpui 的 `theme.accent` 语义是**悬停**底（UI-MAP §1.2 那条 token 陷阱：直搬会把强调色
    // 变成一层灰）。所以选中走 `theme.primary`（实心强调色），悬停才走 `accent`。
    let selected_background = theme.primary;

    let (foreground, hover_background) = match side {
        ActivitySide::Left => (theme.foreground, accent.opacity(HOVER_OPACITY)),
        ActivitySide::Right => (theme.muted_foreground, accent),
    };
    // tooltip 朝栏内：左 rail `tooltipSide="right"`、右 rail `"left"`
    // （`sidebar-pane-selector.tsx:106`、`plugin-activity-rail.tsx:42`）。
    // 宽度/高度是随 `side` 变化的运行时值，两边类型要一致，所以统一收敛到 `Length`：
    // 值本身走 helper 底层的 `rems(P / 16.)`（`Length: From<Rems>`，与 `_N()` 同一条口径）。
    // 圆角不能这样写 —— `Button::rounded` 是**固有方法**（只吃 `Pixels`），会遮蔽
    // `Styled::rounded`，所以经 [`crate::rem_px`] 按运行时基准换算。
    // 两条 rail 的项盒与圆角相同（真源 `WorkbenchView.swift:1279-1288` 左 / `:1485-1494` 右），
    // 只有 ElementId 前缀与 tooltip 方向按 `side` 变。
    let (id_name, placement): (&str, Placement) = match side {
        ActivitySide::Left => ("lithe-activity-bar-left", Placement::Right),
        ActivitySide::Right => ("lithe-activity-bar-right", Placement::Left),
    };
    let (width, height, radius): (Length, Length, Pixels) = (
        rems(ITEM_SIZE / 16.).into(),
        rems(ITEM_SIZE / 16.).into(),
        crate::rem_px(rem, ITEM_RADIUS),
    );

    let is_active = is_active(index);
    // 图标先装（expui 走彩色 `img()`、Lucide 走单色 `.icon(..)`，见 [`ActivityIcon::apply`]），
    // 后面的链是两侧共用的外观与交互。
    let button = item.icon.apply(
        Button::new(ElementId::named_usize(id_name, index))
            .custom(
                ButtonCustomVariant::new(cx)
                    .foreground(foreground)
                    .hover(hover_background)
                    // 自定义变体的**选中底**就是这里的 `active`（`button.rs:1283`）。
                    .active(selected_background),
            )
            // ⚠️ 仍未对齐：`ButtonCustomVariant` 只有一个前景色，所以选中态的文字/单色图标
            // 不会像真源那样切到 `toolWindowSelectedText`（`WorkbenchView.swift:1293`），
            // 彩色 expui 图标在实心强调色底上仍是原色。要完全对齐得给选中态单开一条
            // 前景/图标路径（缺口已登记，见 `UI-MAP.md` 的校正专章）。
            // `Selectable::selected` 出选中底色；`toggled` 只补 `aria-pressed`（`button.rs:477-486`）。
            .selected(is_active)
            .toggled(is_active),
        cx,
    );
    let button = button
        .tooltip(item.label.clone())
        .tooltip_placement(placement)
        .accessibility_label(item.label.clone())
        .rounded(radius)
        .w(width)
        .h(height)
        // ⚠️ 必须清掉"普通按钮"分支的内衬：expui 图标走 `Button::child`（见 [`ActivityIcon::apply`]），
        // 于是 `Button` 不再把它当"图标按钮"，按默认 `Size::Medium` 补 `px_2p5`（左右各 10，
        // `gpui-component-0.7.0/src/button/button.rs:646-671`）。项盒只有 26（左）/ 28（右）宽，
        // 补完内衬内容层只剩 6 / 8，而图标是 `rems(1.25)` = 20 宽 —— 被内容层的 `overflow_hidden()`
        // 左右各裁一刀，只剩中间一条窄缝（2026-09-30 维护者截图：Maven 标志只剩半个字形）。
        // `.px_0()` 由 `Styled` 记进 `instance_style`，渲染期 `refine_style` 压在库内内衬之上，
        // 所以盖得住（`button.rs:607,721`）。`changes_view` / `notifications` 那几处是同一条。
        .px_0()
        .on_click(move |event, window, cx| on_select(index, event, window, cx));

    // 真源里按钮外面还有一层 `.frame(slotWidth, slotHeight)`（`WorkbenchView.swift:1288` / `:1494`）：
    // 按钮在槽里居中、槽之间 `VStack(spacing: 0)` 无间隙 ⇒ 项的纵向节距由槽高（40）决定。
    // gpui 的 `Button` 自己就是那个可点区域，所以这里只补一层负责几何的槽，交互仍留在 `Button` 上。
    let slot = div()
        .w(rems(SLOT_WIDTH / 16.))
        .h(rems(SLOT_HEIGHT / 16.))
        .flex()
        .items_center()
        .justify_center();

    // 有角点时是 **`Badge` 包住 `Button`**，不能反过来：`Badge` 的根是 `div().relative()`
    // 而角点自己绝对定位到 `top_0().right_0()`（`badge.rs:114-163`）；而 `Button` 的子元素
    // 落在 `h_flex().overflow_hidden()` 里（`button.rs:698-733`），塞进去会被裁掉。
    match item.badge {
        None => slot.child(button).into_any_element(),
        Some(color) => slot
            .child(
                Badge::new()
                    .dot()
                    // `Badge` 的默认色是 `theme().red`（`badge.rs:125`），所以必须显式给色。
                    .color(color)
                    .child(button),
            )
            .into_any_element(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 两条 rail 的盒宽都来自 [`design::ACTIVITY_RAIL_WIDTH_SPEC`]（真源
    /// `WorkbenchView.swift:7`（右）/`:12`（左）的 40），而不是各自记一份数字。
    ///
    /// 断言守三件事：① owner 本身是 40；② 右轨盒宽 = owner；③ 左轨盒宽 = owner + `WORKBENCH_GAP`
    /// ——并且左轨盒宽与"右轨盒宽 + 外侧那个 4px 间隔"相等（对外盒宽都是 44，即维护者裁决 (a)
    /// 的"左右同宽"）。把任一侧写回字面量**不会**让这些断言变红；**改 owner** 会让它们一起红 ——
    /// 这正是"共同 owner"能给的保证：数字只有一份，跨 crate 的引用方跟着一起动。
    ///
    /// 纯常量算术：不开窗口、不碰主题、无后台任务、无外部依赖。
    #[test]
    fn both_rails_take_their_width_from_the_shared_owner() {
        assert_eq!(design::ACTIVITY_RAIL_WIDTH_SPEC, 40.);
        assert_eq!(
            COLLAPSED_WIDTH,
            design::ACTIVITY_RAIL_WIDTH_SPEC,
            "rail 宽只能来自 shared::design 的 owner，不得在本文件另写一份数字"
        );
        assert_eq!(rail_width_spec(ActivitySide::Right), 40.);
        assert_eq!(rail_width_spec(ActivitySide::Left), 44.);
        assert_eq!(
            rail_width_spec(ActivitySide::Left),
            rail_width_spec(ActivitySide::Right) + WORKBENCH_GAP
        );
    }
}
