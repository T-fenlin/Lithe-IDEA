//! 右侧工具窗（真机 `ResizablePane position="right"`，`main-layout.tsx:325-341`）。
//!
//! ## 这个区域在真机里是什么
//!
//! 右工具窗**不是一个常驻面板**：`main-layout.tsx:101-105` 拿
//! `isNotificationsVisible || isMavenVisible` 决定整个 `ResizablePane` 的 `hidden`，
//! 而这两个谓词都是 `isRightSidebarVisible && activeRightSidebarView === "<view>"`
//! （同文件 `:102,104`）。也就是**可见性**与**当前视图**是两个独立状态：
//!
//! - `isRightSidebarVisible` 默认 `false`
//!   （`features/window/stores/ui-state/panel-slice.ts:28`）；
//! - `activeRightSidebarView` 默认 `"outline"`
//!   （`stores/ui-state/view-slice.ts:26`），收起时**保留最后显示过的视图**
//!   —— 与底部窗的 `bottomPaneActiveTab` 同一条口径。
//!
//! 两个状态都归 [`crate::workspace::ShellWorkspace`]（外壳是唯一的布局状态所有者）；
//! 本模块**无状态**，只按传入的视图画内容，与 `activity_bar` / `status_bar` 同一口径。
//!
//! ## 宽度
//!
//! `rightToolWindowWidth: 400`（`features/settings/config/default-settings.ts:139`），
//! 取值区间 **140–600**（`features/settings/lib/settings-normalization.ts:114-115`，
//! 同文件 `:532-537` 用它夹 `rightToolWindowWidth`）。宽度由调用方的 `side_pane` 给
//! （`crate::workspace` 的 `RIGHT_TOOL_WINDOW_WIDTH`），本模块不碰。
//!
//! ## 视图清单与各自完成度
//!
//! | 视图 | 右活动栏项 | 真源 | 本侧完成度 |
//! | --- | --- | --- | --- |
//! | Maven | `MavenIcon`（`plugin-activity-rail.tsx:51-66`） | `MavenPane`（`features/maven/components/maven-pane.tsx:596-870`） | 头部（图标 + 标题 + 关闭）+ 空态（`maven.notDetected`）。**未做**：工具栏、模块树、生命周期、依赖树、Profiles、构建输出 —— 它们要 Maven 项目探测与 `maven.*` 数据源，属阶段 B |
//! | 通知 | `NotificationsTrigger`（`plugin-activity-rail.tsx:50`） | `NotificationsToolWindow`（`features/notifications/components/notifications-tool-window.tsx:340-475`） | 头部 + 面板（[`crate::notifications`]）：搜索 / 按严重性过滤 / 展开详情 / 全部清除。**未做**：按日期分组、复制按钮、右键菜单 · **有意不做**：跳转链接（[`lithe_gpui_notify::Target`] 只占位，见那个类型的文档） |
//! | 扩展 | `PuzzlePieceIcon`（`plugin-activity-rail.tsx:36-49`） | `openExtensionsBuffer` —— 真机里扩展是一个**编辑器缓冲区**（同文件 `:13,46`） | ⚠️ **不再是右栏视图**（2026-09-29）：曾按旧任务要求做成右栏空态（当时是有意偏离真源，理由写在已删除的 `PLAN.md` §11.3）；现已对齐真源 —— 拼图项打开**中央扩展页**（[`crate::extensions_page`]，由 `ShellWorkspace` 的中央列切换），右窗只剩通知与 Maven，下标映射见 [`RightToolWindowView::from_rail_index`] |
//!
//! ## 收起面板的三条出路（真源语义）
//!
//! 1. 再点右活动栏**同一项**：`applyRightToolWindowIntent(view, "toggle")` 在
//!    `isVisible = isRightSidebarVisible && activeRightSidebarView === view` 为真时只关可见性
//!    （`features/layout/actions/right-tool-window-actions.ts:17,26-31`）——见 [`resolve_click`]；
//! 2. 面板头部的关闭按钮：真机两个工具窗都自带
//!    （`maven-pane.tsx:606-616`、`notifications-tool-window.tsx:350-360`）；
//! 3. 点右活动栏**另一项**不收起，只换视图（同文件 `:33-36`）。
//!
//! ## 度量与配色
//!
//! | 规格 | 值 | 出处 | 本侧写法 |
//! | --- | --- | --- | --- |
//! | 头部高 | 32 | `h-8`（`notifications-tool-window.tsx:346`、`maven-pane.tsx:599`） | `h_8()` |
//! | 头部内衬 | 12 | `px-3`（同上） | `px_3()` |
//! | 头部图标与标题间距 | 8 | `gap-2`（`maven-pane.tsx:599`） | `gap_2()` |
//! | 标题字号 | 13（`--ui-text-sm`） | `notifications-tool-window.tsx:347` | `text_sm()`（14；13→14 是经维护者确认的有意改动，见 `crate::workspace` 模块头） |
//! | 关闭按钮 | 24×24、圆角 6.4 | `size="icon-xs"`（`notifications-tool-window.tsx:353`）、`rounded-md`（`ui/button.tsx:9`） | `size_6()` + `crate::rem_px(cx.theme().font_size, CLOSE_BUTTON_RADIUS_SPEC)`（`Button::rounded` 是固有方法，只吃 `Pixels`） |
//!
//! 颜色一律 `cx.theme()`；圆角走应用层具名规格常量，按运行时 rem 基准换算
//! （Lithe 的圆角阶梯不在 gpui 档位上：见符号式出处
//! [`lithe_gpui_shared::design::CONTROL_RADIUS`]（6.4，本文件的关闭按钮）与
//! `explorer_view.rs` 的 `ROW_RADIUS`（4，选中面那一族）—— **不再钉行号**：
//! `explorer` 的常量早已移到 `shared::design`，行号引用会随下一次编辑漂移）。

use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Icon, StyledExt as _};
// `when` / `children` 收在 `FluentBuilder` 上（与 `explorer_view.rs` 同一取法）。
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ClickEvent, Entity, InteractiveElement as _, IntoElement, Length,
    ParentElement as _, Pixels, Point, SharedString, StatefulInteractiveElement as _, Styled as _,
    Window, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image, idea_icon_svg_px};

use lithe_gpui_shared::tr;

/// 关闭按钮圆角 6.4（`ui/button.tsx:9` 的 `rounded-md` = `--radius × 0.8`，`theme.css:7,134`）。
///
/// 6.4 不是 gpui 的 rem 档位（`rounded_md()` 是 6），且 Lithe 的圆角阶梯一律走应用层具名常量
/// —— 同一条阶梯的**符号式**出处是 [`lithe_gpui_shared::design::CONTROL_RADIUS`]
/// （`explorer_view.rs` 里那个 `ROW_RADIUS` 不是这一条，勿互换：它是 4，属选中面那一族）。
/// 这一处挂在 `Button::rounded` 上，它是**固有方法**（只吃 `Pixels`，`ButtonRounded` 只有
/// `From<Pixels>`），所以调用点经 [`crate::rem_px`] 按运行时 rem 基准换算；本函数拿不到
/// `Window`，基准取等价的 `cx.theme().font_size`（`Root::render` 每帧把它写进
/// `window.set_rem_size`）。
const CLOSE_BUTTON_RADIUS_SPEC: f32 = lithe_gpui_shared::design::CONTROL_RADIUS;

/// 工具窗头部的高度（真源 `LitheTheme.swift:479` 的 `toolWindowHeaderHeight = 30`，
/// 由 `LitheToolWindowHeader.swift:65` 与 `TerminalView.swift:127` 使用）。
///
/// ⚠️ **改前是 `h_8()` = 32**，出处是**已作废**的 Windows 前端
/// （`notifications-tool-window.tsx:346`、`maven-pane.tsx:599` 的 `h-8`）。
///
/// ⚠️ **暂不提升到 `shared::design`**：那个模块的准入判据是「已有清晰名称 + **两个以上**真实
/// 使用方」（见其模块文档），本条现在只有右工具窗一个调用方。批 9 让终端工具窗复用同一规格时，
/// 三条一起按该判据提升为共同 owner。
const TOOL_WINDOW_HEADER_HEIGHT_SPEC: f32 = 30.;
/// 头部前导图标边长 **13**（真源 `LitheToolWindowHeader.swift:34-36` 的 `size: 13`）。
/// 改前是 20（Windows 前端的值）。
const TOOL_WINDOW_HEADER_ICON_SPEC: f32 = 13.;
/// 头部**尾部**内衬 **7**：真源内衬是 leading 12 / trailing 7
/// （`LitheToolWindowHeader.swift:63-64`），不是左右各 12。前导侧的 12 直接用 `pl_3()`。
const TOOL_WINDOW_HEADER_TRAILING_INSET_SPEC: f32 = 7.;

/// 右侧工具窗当前显示哪一个视图。
///
/// 取值与真机 `activeRightSidebarView` 的子集对应（`SidebarView` 见
/// `features/layout/utils/sidebar-pane-utils.ts`；本侧实现右活动栏的通知与 Maven 两项）。
/// **扩展不是本枚举的取值**（2026-09-29 起）：右活动栏的拼图项打开中央扩展页
/// （[`crate::extensions_page`]），见 [`RightToolWindowView::from_rail_index`]。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightToolWindowView {
    /// 通知（`activeRightSidebarView === "notifications"`）。
    Notifications,
    /// Maven（`activeRightSidebarView === "maven"`）。
    Maven,
    /// Spring（**本侧新增**：真机 Windows 没有独立的 Spring 面板，Spring 能力留在语言服务里；
    /// 这里把 Core 的 `spring.index` 结论呈现出来，见 `crate::spring` 的模块文档）。
    Spring,
}

impl RightToolWindowView {
    /// 诊断行里的视图名（`S1_RIGHT_PANEL view=…`，可 grep）。
    ///
    /// 用真机的 `activeRightSidebarView` 取值口径（`"notifications"` / `"maven"`），
    /// 便于和真源对照。
    pub const fn id(self) -> &'static str {
        match self {
            Self::Notifications => "notifications",
            Self::Maven => "maven",
            Self::Spring => "spring",
        }
    }

    /// 视图名 → 视图；`None` = 没有这个视图（**不回落到任何默认项**）。
    ///
    /// 与 [`RightToolWindowView::id`] 双向一致：`from_id(v.id()) == Some(v)`。
    /// 命令行 `--right-view <id>` 就是靠它把"用户写的那串字符"变成状态机里的视图。
    ///
    /// `"extensions"` 在这里返回 `None`：扩展页不是右工具窗视图，`--right-view
    /// extensions` 由 `ShellStartup` 的独立开关承接（`crate::workspace`）。
    ///
    /// 为什么不复用 [`RightToolWindowView::from_rail_index`]：下标是**界面顺序**的坐标，
    /// 重排右活动栏就会改变含义；`id()` 是诊断行 `S1_RIGHT_PANEL view=…` 的取值，
    /// 是机器可验证的契约，命令行参数必须绑在契约上而不是绑在顺序上。
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "notifications" => Some(Self::Notifications),
            "maven" => Some(Self::Maven),
            "spring" => Some(Self::Spring),
            _ => None,
        }
    }

    /// 右活动栏下标 → 视图；`None` = 该下标没有对应视图。
    ///
    /// ⚠️ 下标必须与 `crate::workspace::right_activity_items()` 的顺序一致
    /// （0 扩展拼图 / 1 通知 / 2 Maven；照 `plugin-activity-rail.tsx:31-67`）。
    /// **下标 0 返回 `None`**：拼图项不是右工具窗视图，它开/关中央扩展页
    /// （`ShellWorkspace::toggle_extensions_page` 在派发前拦截下标 0）。
    ///
    /// 曾有的 `3 → Spring` 一支已删：2026-09-27 收敛为三项后活动栏本就不产生下标 3，
    /// 2026-09-29 扩展项改为中央页入口后，连"保留一个永不命中的下标"的对照意义也没有了
    /// —— Spring 视图仍由 `--right-view spring`（[`RightToolWindowView::from_id`]）到达。
    pub const fn from_rail_index(index: usize) -> Option<Self> {
        match index {
            1 => Some(Self::Notifications),
            2 => Some(Self::Maven),
            _ => None,
        }
    }

    /// 头部标题。
    ///
    /// 键取自真源：`maven.title`（`maven-pane.tsx:602`）、`notifications.title`
    /// （`notifications-tool-window.tsx:348`）。
    fn title(self) -> SharedString {
        match self {
            Self::Notifications => tr("lithe.notifications.title"),
            Self::Maven => tr("lithe.maven.title"),
            // Spring 在真源 catalog 里没有面板标题（Windows 没有独立面板），
            // 是本侧新增的键（理由写在 `extract-locale.mjs` 的 GPUI_ONLY_KEYS 里）。
            Self::Spring => tr("lithe.spring.title"),
        }
    }

    /// 头部 / 空态图标。与右活动栏逐项同字形（`activity_bar.rs` 的对照表）。
    ///
    /// `side` 由调用方给（头部 16、空态 32）。返回 `AnyElement` 而不是 `Icon`，因为 Maven 用的是
    /// **彩色**的蓝色标志（`files/maven.svg`，`img()` 管线）——它不吃 `text_color`，塞不进
    /// `Icon` 那个只收字形的槽位；通知与 Spring 仍是单色字形（`primary`）。
    fn icon(self, cx: &App, side: impl Into<Length>) -> AnyElement {
        let side: Length = side.into();
        match self {
            Self::Notifications => idea_icon_svg_px(&idea::BELL_ICON, cx, side)
                .text_color(cx.theme().primary)
                .into_any_element(),
            // 真源：`icons/idea/icons/files/maven.svg(+_dark)` —— 与 IDEA 工具窗图标同物
            // （2026-09-30 改：此前是 Lucide 的立方体）。
            Self::Maven => idea_icon_image(&idea::MAVEN_ICON, cx, side),
            // Spring 的真源字形是品牌绿叶（`spring-icon.tsx` 内联 path，没有 SVG 文件），
            // Lucide 里语义最近的是 `leaf`。
            Self::Spring => Icon::new(IconName::Leaf)
                .size(side)
                .text_color(cx.theme().primary)
                .into_any_element(),
        }
    }

    /// 空态标题。
    ///
    /// 两条都是真源**已有的**空态文案，不是新写的：
    /// - Maven：`maven.notDetected`（"未检测到 Maven 项目"，`maven-pane.tsx:806` 在
    ///   `projectStatus !== "loading"` 且没有项目时显示的那一句）；
    /// - 通知：`notifications.empty`（`notifications-tool-window.tsx:399` 的 `CommandEmpty`）。
    ///
    /// Maven 的空态是**如实**的：本侧还没有 Maven 项目探测（真机也只探测到 Maven 项目时才
    /// 渲染右栏那一项，`plugin-activity-rail.tsx:21-23,51`），所以"未检测到 Maven 项目"
    /// 就是当前的真实状态，不是占位文案。
    fn empty_title(self) -> SharedString {
        match self {
            Self::Notifications => tr("lithe.notifications.empty"),
            Self::Maven => tr("lithe.maven.notDetected"),
            // 判据是 Core 的 `spring.index` 没给出任何事实（响应 null 或集合全空），
            // 所以文案说「组件与端点」而不是「项目」——不把"没有 Spring 的 Java 项目"
            // 说成"没检测到项目"。键是本侧新增（同 `title`）。
            Self::Spring => tr("lithe.spring.notDetected"),
        }
    }
}

/// 点击右活动栏某一项之后的状态迁移，返回 `(视图, 是否可见)`。
///
/// 逐条照真机 `resolveRightToolWindowUpdate`
/// （`features/layout/actions/right-tool-window-actions.ts:11-37`）：
///
/// | 当前状态 | 点的项 | 结果 |
/// | --- | --- | --- |
/// | 可见且就是这一项 | 同一项 | **收起**，视图保持不变（`intent === "toggle" && isVisible`） |
/// | 可见但是别的项 | 另一项 | 切到那一项，**保持可见**（`:33-36`） |
/// | 不可见 | 任意项 | 切到那一项并显示（同 `:33-36`；`isVisible` 因为 `isRightSidebarVisible=false` 而为假） |
pub fn resolve_click(
    clicked: RightToolWindowView,
    current: RightToolWindowView,
    current_visible: bool,
) -> (RightToolWindowView, bool) {
    if current_visible && current == clicked {
        (current, false)
    } else {
        (clicked, true)
    }
}

/// 诊断行里的一次指针位置（**逻辑像素**，与 `Window::mouse_position()` 同一坐标空间）。
///
/// 坐标是"一次点击变成两行日志"这条判据的另一半：只有**同一个点**上的两次回调才可能是
/// 派发层重复，不同点必然是两次输入（判据全文见 [`diagnose`]）。
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ProbePoint {
    x: f32,
    y: f32,
}

impl ProbePoint {
    pub(crate) const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// gpui 的窗口坐标 → 诊断点。
///
/// 取值走 gpui 自己的 `From<Pixels> for f32`（`gpui-pre-0.3.6/src/geometry.rs:2921-2924`）：
/// `Pixels` 的字段是 `pub(crate)`，crate 外只能通过这个转换拿数值。
impl From<Point<Pixels>> for ProbePoint {
    fn from(point: Point<Pixels>) -> Self {
        Self::new(f32::from(point.x), f32::from(point.y))
    }
}

/// `S1_RIGHT_PANEL` 的进程内序号：每打一行 +1。
///
/// 三条入口（右活动栏点击 / 「视图 → Maven」菜单 / `--right-view` 探针）都在 UI 线程上，
/// 所以 `Relaxed` 足够 —— 这里要的只是"同一进程内单调且唯一"，不拿它同步别的内存。
static RIGHT_PANEL_SEQ: AtomicUsize = AtomicUsize::new(0);

/// `t_ms` 的单调原点。
///
/// `Instant` 不能在 `static` 里直接构造，所以取"第一次打这一族诊断"的时刻为原点。
/// 判据只看相邻两行的 **Δt**，原点落在哪里都不影响（所以不必去读系统时钟 ——
/// `Instant` 也不会被 NTP / 手动改表带偏，这一点比 `SystemTime` 重要）。
static RIGHT_PANEL_EPOCH: OnceLock<Instant> = OnceLock::new();

/// 组装一行右工具窗诊断（纯函数：格式本身可单测）。
///
/// 字段顺序是**机器契约**，验证脚本按名字取值，改它就是改契约。
/// 坐标是**可选**的：只有"这一行真的来自一次指针输入"时才打（见 [`diagnose`]）。
fn diagnose_line(
    view: RightToolWindowView,
    visible: bool,
    point: Option<ProbePoint>,
    seq: usize,
    t_ms: u128,
) -> String {
    let mut line = format!(
        "S1_RIGHT_PANEL seq={seq} t_ms={t_ms} view={} visible={visible}",
        view.id()
    );
    if let Some(point) = point {
        // 一位小数就够：判据是"两次的坐标**完全相同**"，多打位数不会增加信息，
        // 反而让"同一次点击的两行看起来不一样"这种噪声更容易被误读。
        line.push_str(&format!(" x={:.1} y={:.1}", point.x, point.y));
    }
    line
}

/// 打一行右工具窗诊断（可 grep，与其他 `S1_*` 同一口径）。
///
/// 每次状态迁移打一行，构造期也打一行 —— 这样"启动时面板是隐藏的"这件事本身有日志证据，
/// 不必只靠截图。
///
/// ## 为什么带 `seq` / `t_ms` / 坐标
///
/// 起因是"一次点击打出 `visible=true` 紧跟 `visible=false`"这个现象：在只有
/// `view=` / `visible=` 的日志里，"同一次派发栈里的重复回调"和"两次独立输入"**在证据上
/// 完全同构**（`lithe-gpui/research/click-double-trigger-dpi.md` §6 P0，那里的结论是派发层
/// 不会一发二，缺的是能证伪的仪表）。加上序号、单调时钟与点击坐标之后，这条判据变成可判定：
///
/// | 相邻两行 | 结论 |
/// | --- | --- |
/// | `Δt = t_ms(后) - t_ms(前) < 1ms` **且**两次坐标相同 | 两次回调落在**同一次派发**里 ⇒ 真·派发层重复派发 |
/// | `Δt` 几十~几百 ms，**或**坐标不同 | **两次独立输入**（环境杂散点击 / UIA `Action::Click` / 脚本点了两次 / 人手又点了一下） |
/// | 任一行没有 `x=` / `y=` | 这一行不是指针输入产生的（构造期 / `--right-view` 探针 / 菜单通道 / 键盘激活按钮），不能用于这条判据 |
///
/// 坐标只来自 `ClickEvent::mouse_position()`（右活动栏点击与面板关闭按钮两处调用点都是），
/// 所以"有坐标"本身就等于"这一次是鼠标 / 触摸点出来的"。
///
/// ⚠️ 这条判据只判"两次输入是不是同一个派发"，**不**改 toggle 语义、也**不**加防抖：
/// 真机就是一次 click 一个 toggle
/// （`windows/tauri/src/features/layout/actions/right-tool-window-actions.ts:26-31`、
/// `windows/tauri/src/features/layout/components/plugin-activity-rail.tsx:62`），
/// 快速点两次本来就该"开→关"。
///
/// ## 为什么走 stderr
///
/// stdout 在本仓库是**块缓冲**的：重定向到文件时进程还在跑就可能一行都看不到
/// （`lithe-gpui/crates/app/src/main.rs:504,548-550` 记了这条实测口径），而这一族诊断的用途恰恰是
/// "点击那一刻发生了什么"，延迟可见等于没有。与交互有关的
/// `S1_MENU_RUN` / `S1_TAB_MENU` / `S1_SOURCE_CONTROL` 同样走 stderr。
pub(crate) fn diagnose(view: RightToolWindowView, visible: bool, point: Option<ProbePoint>) {
    let seq = RIGHT_PANEL_SEQ.fetch_add(1, Ordering::Relaxed);
    let t_ms = RIGHT_PANEL_EPOCH
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis();
    eprintln!("{}", diagnose_line(view, visible, point, seq, t_ms));
}

/// 打一行中央扩展页诊断（`S1_EXTENSIONS_PAGE`，与 [`diagnose`] 同族、可 grep）。
///
/// 坐标纪律与 [`diagnose`] 相同：只有"这一次真的来自指针输入"才带 `x=` / `y=`
/// （构造期 / 探针路径没有坐标，编一个出来会污染"同一次派发"判据）。
pub(crate) fn diagnose_extensions_page(open: bool, point: Option<ProbePoint>) {
    if let Some(point) = point {
        eprintln!(
            "S1_EXTENSIONS_PAGE open={open} x={:.1} y={:.1}",
            point.x, point.y
        );
    } else {
        eprintln!("S1_EXTENSIONS_PAGE open={open}");
    }
}

/// 画右侧工具窗。
///
/// 结构照真机：`<section>` 头部（图标 + 标题 + 关闭按钮）+ 内容区。
///
/// `on_close` 由调用方给：可见性归外壳（`ShellWorkspace`），本函数不持有状态。
///
/// `notifications` 是通知中心面板。它是**必填**而不是 `Option`：唯一调用方
/// （`ShellWorkspace::render`）在构造期就建好了面板，通知视图不存在「拿不到面板」这种
/// 启动期形状 —— `--right-view notifications` 探针（`app/src/main.rs:263,870`）也是走
/// 同一个外壳。
pub fn right_tool_window(
    view: RightToolWindowView,
    maven: Option<&crate::maven::MavenProjectView>,
    spring: Option<&crate::spring::SpringIndexView>,
    notifications: &Entity<crate::notifications::NotificationPanel>,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let title = view.title();
    // 真源头部的副标题槽（`LitheToolWindowHeader.swift:51-55`，11.5 medium + `secondaryText`）。
    // 目前只有 Spring 面板用它：真源是 `"\(filteredEndpoints.count) routes"`
    // （`SpringEndpointsView.swift:12`）。本侧还没有筛选框，所以这里的 count 就是全部端点数；
    // 筛选行落地后改成"筛选后"的条数即可（真源取的就是 filtered 的那个）。
    let subtitle = match (view, spring) {
        (RightToolWindowView::Spring, Some(index)) => {
            let count = index.endpoints.len().to_string();
            Some(lithe_gpui_shared::tr_args(
                "lithe.spring.gpui.routeCount",
                &[("count", count.as_str())],
            ))
        }
        _ => None,
    };
    v_flex()
        .id("lithe-right-tool-window")
        // 真机是 `<section aria-label={t("maven.title")}>`（`maven-pane.tsx:597`）；
        // gpui 侧 `.aria_label()` 挂在 `StatefulInteractiveElement` 上，所以根元素带 `id`。
        .aria_label(title.clone())
        .size_full()
        .child(header(view, title, subtitle, on_close, cx))
        .child(match (view, maven, spring) {
            // 扫到了 Maven 项目就画项目结构；没扫到（或还没扫）保持真源的「未检测到 Maven 项目」。
            (RightToolWindowView::Maven, Some(project), _) => maven_content(project, cx),
            // Spring 同理：Core 的 `spring.index` 给到事实才画，否则空态（`is_empty` 也算没有事实）。
            (RightToolWindowView::Spring, _, Some(index)) if !index.is_empty() => {
                spring_content(index, cx)
            }
            // 通知中心：面板自己管搜索 / 筛选 / 展开，所以这里只转交实体。
            (RightToolWindowView::Notifications, _, _) => notifications.clone().into_any_element(),
            _ => empty_state(view, cx),
        })
        .into_any_element()
}

/// Spring 面板内容：**只呈现 `spring.index` 已经给出的事实**。
///
/// 三块，按"用户打开这一栏最想先看到什么"排：
/// 1. **端点**（`@RequestMapping` 家族的展开结果）：`GET/POST /api/users` 一行一条，
///    右边弱化显示 `控制器.方法`；这是 Spring 开发者最常核对的东西；
/// 2. **索引计数**（属性 / 配置值 / `@Value` 引用 / 注入点 / bean）：一眼看出索引覆盖到什么程度；
/// 3. 诊断条数单列 —— 它非 0 说明元数据或源码里有 `spring.index` 认不出的东西。
///
/// 真源那套（配置属性补全、注入链、`@Value` 引用跳转）要各自的 UI，本侧**不画假控件**；
/// 逐条渲染属于后续批次（`spring.rs` 的模块文档里登记了）。
fn spring_content(index: &crate::spring::SpringIndexView, cx: &App) -> AnyElement {
    // 索引到了项目、但**一个端点都没有**：真源这一档是居中的一句弱化文字
    // （`SpringEndpointsView.swift:27-31`：`Text("No Spring MVC endpoints found")` +
    // `LitheTheme.uiFont` + `secondaryText` + `frame(maxWidth/maxHeight: .infinity)`），
    // **不是** `LitheUnavailableView` 那套「图标 + 粗标题」，所以这里照它画裸文字，
    // 不套 `empty_state::unavailable`（套了就给真源没有的图标与字重）。
    // 它也与 `spring.notDetected`（根本没有可索引的事实）是两回事。
    if index.endpoints.is_empty() {
        return v_flex()
            .w_full()
            .flex_1()
            .min_h_0()
            .items_center()
            .justify_center()
            .child(
                div()
                    // `LitheTheme.uiFont` 在本侧就是默认 UI 字号（档位 14 = `text_sm`）。
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.spring.gpui.noEndpoints")),
            )
            .into_any_element();
    }

    // 真源把路由 / 方法 / 控制器都画成**等宽**（`SpringEndpointsView.swift:39,43`），
    // 本侧沿用仓库既有取法（`explorer_view.rs` / `git/log_view.rs`）。
    let mono = cx.theme().mono_font_family.clone();
    let mut column = v_flex()
        .id("spring-index")
        .w_full()
        .flex_1()
        .min_h_0()
        // 真源列表是 `LazyVStack(spacing: 1)` + `.padding(6)`
        // （`SpringEndpointsView.swift:34,62`）；1 与 6 都不在 gpui 的档位上，按 `rems(P / 16.)` 写。
        .gap(rems(1. / 16.))
        .p(rems(6. / 16.));

    for endpoint in &index.endpoints {
        // 真源行（`SpringEndpointsView.swift:37-56`）：`HStack(spacing: 9)`、方法列定宽 58 且左侧对齐、
        // 行内衬左右 10、最小高 30；方法色按**第一个** HTTP 方法取
        // （`:78-85`：GET success / POST accent / DELETE error / 其余 warning）。
        let method_color = match endpoint.http_methods.first().map(String::as_str) {
            Some("GET") => cx.theme().success,
            // 真源 `POST` 用的是 `LitheTheme.accent`（强调色）。gpui 的 `theme.accent` 语义是
            // **悬停底**（UI-MAP §1.2 记的 token 陷阱），强调色的前景对应 `theme.primary`。
            Some("POST") => cx.theme().primary,
            Some("DELETE") => cx.theme().danger,
            _ => cx.theme().warning,
        };
        column = column.child(
            h_flex()
                .w_full()
                .gap(rems(9. / 16.))
                .px(rems(10. / 16.))
                .min_h(rems(30. / 16.))
                .child(
                    // 方法列：真源 10.5 bold monospaced、宽 58、左对齐。
                    // 10.5 不在 gpui 的档位（12 / 14 / 16）上，取最近的 **12**。
                    div()
                        .flex_shrink_0()
                        .w(rems(58. / 16.))
                        .text_xs()
                        .font_bold()
                        .font_family(mono.clone())
                        .text_color(method_color)
                        .child(endpoint.methods_label()),
                )
                .child(
                    // 路由：真源 12.5 medium monospaced。12.5 与档位 12 / 14 等距，取 **14** ——
                    // 这样它与上一列的 12 之间仍保有真源那 2pt 的层级差。
                    div()
                        .flex_shrink_0()
                        .text_sm()
                        .font_medium()
                        .font_family(mono.clone())
                        .text_color(cx.theme().foreground)
                        .child(endpoint.route.clone()),
                )
                .child(
                    // 控制器.方法：真源 10.5 弱化（同上取 12）。
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{}.{}", endpoint.controller, endpoint.method)),
                )
                // 真源的 `Spacer()`：把相对路径推到行尾。
                .child(div().flex_1())
                .child(
                    // 相对路径：真源 10.5 弱化 + `lineLimit(1)`。`path` 是 Core 给的工作区相对路径。
                    div()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(endpoint.path.clone()),
                ),
        );
    }

    // 计数行：数字是事实，标签是**已有的** maven 面板同族风格（纯数字 + 弱化色），
    // 不引入新的文案键。
    column = column.child(
        div()
            .w_full()
            .mt_2()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(format!(
                "properties {} · values {} · refs {} · injections {} · beans {} · diagnostics {}",
                index.properties,
                index.values,
                index.property_references,
                index.injections,
                index.beans,
                index.diagnostics
            )),
    );

    column.into_any_element()
}

/// Maven 面板内容：**只呈现 `maven.scan` 已经给出的事实**，不画任何还没有数据源的控件。
///
/// 图上只有三样东西，每一样都能在 `MavenProjectView` 里找到出处：
/// 1. reactor 头（`artifactId` + `version` + 相对路径）；
/// 2. 模块树（递归缩进 —— 真机也是模块树，不是扁平列表）；
/// 3. profile 列表（默认激活的用主色 + 字重区分：**颜色之外还有字重**作为第二信号）。
///
/// 真源那一页还有工具栏 / 生命周期 / 依赖树 / 构建输出四块，它们要 `mvn` 执行与依赖解析的数据源，
/// 本侧**不画**（画了就是假控件，`07-settings-ui.md` §7.3-D 同一口径）。
fn maven_content(project: &crate::maven::MavenProjectView, cx: &App) -> AnyElement {
    let mut column = v_flex()
        .id("maven-project")
        .w_full()
        .flex_1()
        .min_h_0()
        .gap_1()
        .p_3();

    // 1) reactor 头：`artifactId`（正常色）+ 版本（弱化）。
    column = column.child(
        h_flex()
            .w_full()
            .gap_2()
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                    .text_color(cx.theme().foreground)
                    .child(project.artifact_id.clone()),
            )
            .children(project.version.clone().map(|version| {
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(version)
            })),
    );
    // 相对路径单独一行：reactor 不在工作区根时这一行才信息量最大。
    column = column.child(
        div()
            .w_full()
            .text_xs()
            .truncate()
            .text_color(cx.theme().muted_foreground)
            .child(project.relative_path.clone()),
    );

    // 2) 模块树。
    for module in &project.modules {
        column = column.child(maven_module_row(module, cx));
    }

    // 3) profile：id 直接列出来，默认激活的加主色 + 中粗（两种信号，不只靠颜色）。
    if !project.profiles.is_empty() {
        let active = &project.active_profiles;
        column = column.child(v_flex().w_full().mt_2().gap_1().children(
            project.profiles.iter().map(|id| {
                let is_active = active.contains(id);
                div()
                    .w_full()
                    .truncate()
                    .text_xs()
                    .when(is_active, |this| {
                        this.text_color(cx.theme().primary)
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                    })
                    .when(!is_active, |this| {
                        this.text_color(cx.theme().muted_foreground)
                    })
                    .child(id.clone())
            }),
        ));
    }

    column.into_any_element()
}

/// 一行模块：`artifactId` + （有的话）版本；子模块缩进一层。
///
/// 用嵌套 `v_flex` + 每层 `pl_3()` 表达层级，而不是手算像素缩进 ——
/// 缩进因此跟着 rem 档位走（界面字号变了不会错位）。
fn maven_module_row(module: &crate::maven::MavenModuleView, cx: &App) -> AnyElement {
    let mut column = v_flex().w_full().gap_1();
    column = column.child(
        h_flex()
            .w_full()
            .gap_2()
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_xs()
                    .text_color(cx.theme().foreground)
                    .child(module.label().to_string()),
            )
            .children(module.version.clone().map(|version| {
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(version)
            })),
    );
    // 源码根：Maven 模型给的权威事实（不猜目录）。`kind` 是语义分档，原样显示；
    // 生成源根（`generatedMain` 一类）也列出来 —— 用户在补全里看到生成代码时，
    // 这一行就是它从哪来的答案。
    for root in &module.source_roots {
        column = column.child(
            h_flex()
                .w_full()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(root.path.clone()),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(root.kind.clone()),
                ),
        );
    }
    if !module.modules.is_empty() {
        column = column.child(
            v_flex().w_full().pl_3().gap_1().children(
                module
                    .modules
                    .iter()
                    .map(|child| maven_module_row(child, cx)),
            ),
        );
    }
    column.into_any_element()
}

/// 面板头部：图标 + 标题 + 关闭按钮。
///
/// 真机两个工具窗的头部都是 `flex h-8 items-center border-b px-3`，
/// 标题 `flex-1 truncate`（`notifications-tool-window.tsx:346-349`、
/// `maven-pane.tsx:599-604`）。
///
/// ⚠️ **有意偏离**：通知工具窗的头部**没有**前导图标（`:346-349` 只有标题），
/// 而 Maven 有（`maven-pane.tsx:600` 的 `MavenIcon text-primary`）。这里给三个视图统一加图标，
/// 免得同一个面板在切换视图时头部结构跳变；图标用 `theme.primary`，与 Maven 真源一致。
fn header(
    view: RightToolWindowView,
    title: SharedString,
    // 真源的副标题槽（`LitheToolWindowHeader.swift:51-55`）：`nil` 时不画。
    subtitle: Option<SharedString>,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    // 关闭按钮的无障碍名与 tooltip。真源两个工具窗都用 `commandPalette.close`
    // （`maven-pane.tsx:606`、`notifications-tool-window.tsx:355`），文案是「关闭命令面板」
    // （`locale.ts:7938`）—— 这是真源自身的键复用，本侧照抄，不改写成新键。
    let close_label = tr("lithe.commandPalette.close");
    h_flex()
        .w_full()
        .flex_shrink_0()
        .h(rems(TOOL_WINDOW_HEADER_HEIGHT_SPEC / 16.))
        .gap_2()
        // 真源内衬是 leading 12 / trailing 7（`LitheToolWindowHeader.swift:63-64`），
        // 不是左右各 12；前导侧的 12 就是 `pl_3()`。
        .pl_3()
        .pr(rems(TOOL_WINDOW_HEADER_TRAILING_INSET_SPEC / 16.))
        // ⚠️ 真源头部的表面是 `litheWorkbenchSurface(LitheTheme.toolHeader)`
        // （`LitheToolWindowHeader.swift:66`），本侧**没有** `toolHeader` 这条语义 role，
        // 所以仍用一条下边框分隔 —— 缺 role 时不在调用点拼颜色（UI-MAP §1.2）。
        // 这条缺口登记进 UI-MAP 校正专章，等主题 role 收敛那一批一起做。
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            // 尺寸与前景色由 `icon()` 自己按字形决定（彩色标志不吃 `text_color`），
            // 这里只给边长：真源头部图标 13。
            view.icon(cx, rems(TOOL_WINDOW_HEADER_ICON_SPEC / 16.)),
        )
        .child(
            // 标题 + 副标题同一段：真源是 `Text(title)` 之后紧跟可选的 `Text(subtitle)`
            // （`LitheToolWindowHeader.swift:48-55`），两者都在 `Spacer(minLength: 12)` 之前。
            // 所以这里用一层 `h_flex`：标题可截断、副标题 `shrink_0` 且单行。
            h_flex()
                .flex_1()
                .min_w_0()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        // 真源标题是 12.5 semibold（`LitheToolWindowHeader.swift:48-50`）。gpui 的字号档位
                        // 只有 12 / 14 / 16（`text_xs` / `text_sm` / `text_base`），12.5 取最近的 **12**。
                        .text_xs()
                        .font_semibold()
                        .text_color(cx.theme().foreground)
                        .child(title),
                )
                .children(subtitle.map(|subtitle| {
                    // 真源副标题：11.5 medium + `secondaryText`、`lineLimit(1)`
                    // （`LitheToolWindowHeader.swift:51-55`）；11.5 同样取最近的 **12**，
                    // 与标题的差别由字重（medium vs semibold）与颜色（弱化 vs 正常）承担。
                    div()
                        .flex_shrink_0()
                        .truncate()
                        .text_xs()
                        .font_medium()
                        .text_color(cx.theme().muted_foreground)
                        .child(subtitle)
                })),
        )
        .child(
            // `Button variant=ghost size=icon-xs`：24×24（`ui/button.tsx:27`）。
            // `ghost()` 取法与原 `explorer_view.rs` 的 `header_button` 相同（前景 `muted_foreground`）
            // —— 那个辅助函数 2026-09-27 随「项目」树头一起删除，这里只留"同一取法"的说明。
            Button::new("right-tool-window-close")
                .ghost()
                .icon(IconName::X)
                .tab_stop(false)
                .size_6()
                .rounded(crate::rem_px(
                    cx.theme().font_size,
                    CLOSE_BUTTON_RADIUS_SPEC,
                ))
                .tooltip(close_label.clone())
                .accessibility_label(close_label)
                .on_click(on_close),
        )
        .into_any_element()
}

/// 视图内容区：本轮三个视图都是空态，结构与度量由 [`lithe_gpui_shared::empty_state`] 这个
/// **共同 owner** 提供（真源 `LitheUnavailableView.swift:19-35`）—— 它已经负责外层包裹、
/// 居中、间距与标题层级，所以这里只给图标与标题。
///
/// 图标从改前的 `rems(2.)` = 32 收到真源的 **28**，标题补上真源本来就有的 `font_semibold()`。
fn empty_state(view: RightToolWindowView, cx: &App) -> AnyElement {
    lithe_gpui_shared::empty_state::unavailable(
        Some(view.icon(
            cx,
            rems(lithe_gpui_shared::empty_state::UNAVAILABLE_ICON_SIZE_SPEC / 16.),
        )),
        view.empty_title(),
        None,
        // 本视图没有空态下方的额外内容。
        None,
        cx,
    )
}

#[cfg(test)]
mod tests {
    use super::{ProbePoint, RightToolWindowView, diagnose_line, resolve_click};

    /// 右活动栏下标 → 视图必须与 `workspace::right_activity_items()` 的顺序对应：
    /// 下标 0 是**扩展拼图项**（开中央扩展页，不是右工具窗视图，所以是 `None`），
    /// 1 通知 / 2 Maven；越界返回 `None` 而不是回落到某一项。
    #[test]
    fn rail_index_maps_to_view() {
        assert_eq!(RightToolWindowView::from_rail_index(0), None);
        assert_eq!(
            RightToolWindowView::from_rail_index(1),
            Some(RightToolWindowView::Notifications)
        );
        assert_eq!(
            RightToolWindowView::from_rail_index(2),
            Some(RightToolWindowView::Maven)
        );
        assert_eq!(RightToolWindowView::from_rail_index(3), None);
    }

    /// 视图 id 是诊断行 `S1_RIGHT_PANEL view=…` 的取值，改它就是改机器可验证的契约。
    #[test]
    fn view_ids_are_probe_tokens() {
        assert_eq!(RightToolWindowView::Notifications.id(), "notifications");
        assert_eq!(RightToolWindowView::Maven.id(), "maven");
        assert_eq!(RightToolWindowView::Spring.id(), "spring");
    }

    /// `--right-view <id>` 的入口必须与 `id()` 双向一致：正解、反解各走一遍，
    /// 少一遍就会出现"日志打得出来、命令行解不回去"这种只在一侧成立的假契约。
    #[test]
    fn id_round_trips_through_from_id() {
        for view in [
            RightToolWindowView::Notifications,
            RightToolWindowView::Maven,
            RightToolWindowView::Spring,
        ] {
            assert_eq!(RightToolWindowView::from_id(view.id()), Some(view));
        }
        assert_eq!(
            RightToolWindowView::from_id("notifications"),
            Some(RightToolWindowView::Notifications)
        );
        assert_eq!(
            RightToolWindowView::from_id("maven"),
            Some(RightToolWindowView::Maven)
        );
        assert_eq!(
            RightToolWindowView::from_id("spring"),
            Some(RightToolWindowView::Spring)
        );
    }

    /// 未知 id 返回 `None`（**不回落**到某一项）：命令行拼错时必须能被发现，
    /// 否则 `--right-view mavne` 会静默打开一个用户没点名的面板。
    ///
    /// `"extensions"` 也是 `None`：扩展页不是右工具窗视图，`--right-view extensions`
    /// 由 `ShellStartup` 的独立开关承接（`crate::workspace`），不走这个枚举。
    #[test]
    fn unknown_id_has_no_view() {
        assert_eq!(RightToolWindowView::from_id(""), None);
        assert_eq!(RightToolWindowView::from_id("extensions"), None);
        assert_eq!(RightToolWindowView::from_id("Maven"), None);
        assert_eq!(RightToolWindowView::from_id("outline"), None);
        assert_eq!(RightToolWindowView::from_id("mavne"), None);
        assert_eq!(RightToolWindowView::from_id("spring "), None);
    }

    /// 再点同一项 → 收起，且**视图保持不变**（收起时记住最后显示过的视图，
    /// 与底部窗的 `bottomPaneActiveTab` 同一条口径）。对应真机
    /// `right-tool-window-actions.test.ts` 的 "toggles the active view closed"。
    #[test]
    fn clicking_the_visible_view_closes_it() {
        let (view, visible) =
            resolve_click(RightToolWindowView::Maven, RightToolWindowView::Maven, true);
        assert_eq!(view, RightToolWindowView::Maven);
        assert!(!visible);
    }

    /// 隐藏状态下点同一项 → 显示（不是"再收一次"）。
    #[test]
    fn clicking_a_hidden_view_shows_it() {
        let (view, visible) = resolve_click(
            RightToolWindowView::Maven,
            RightToolWindowView::Maven,
            false,
        );
        assert_eq!(view, RightToolWindowView::Maven);
        assert!(visible);
    }

    /// 可见时点**另一项** → 切视图并保持可见（真机 `:33-36`）；隐藏时点另一项 → 显示那一项。
    #[test]
    fn clicking_another_view_switches_and_keeps_visible() {
        let (view, visible) = resolve_click(
            RightToolWindowView::Notifications,
            RightToolWindowView::Maven,
            true,
        );
        assert_eq!(view, RightToolWindowView::Notifications);
        assert!(visible);

        let (view, visible) = resolve_click(
            RightToolWindowView::Maven,
            RightToolWindowView::Notifications,
            false,
        );
        assert_eq!(view, RightToolWindowView::Maven);
        assert!(visible);
    }

    /// 诊断行的**格式**是机器契约：验证脚本按 `seq=` / `t_ms=` / `view=` / `visible=` /
    /// `x=` / `y=` 取值，所以逐字段钉住；带坐标与不带坐标两种形态各一条。
    ///
    /// 这条也是"两行日志可判定"的前提：没有 `seq` / `t_ms` 时，"同一次派发的重复回调"和
    /// "两次独立输入"在证据上无法区分（`lithe-gpui/research/click-double-trigger-dpi.md` §6 P0）。
    #[test]
    fn diagnose_line_carries_seq_timestamp_and_optional_point() {
        assert_eq!(
            diagnose_line(
                RightToolWindowView::Spring,
                true,
                Some(ProbePoint::new(1421.6, 123.2)),
                7,
                8123,
            ),
            "S1_RIGHT_PANEL seq=7 t_ms=8123 view=spring visible=true x=1421.6 y=123.2"
        );
        // 没有指针位置的行（构造期 / `--right-view` 探针 / 菜单通道）：坐标字段**整段不出现**，
        // 不留 `x=0 y=0` 这种会被读成"真的点在原点"的假证据。
        assert_eq!(
            diagnose_line(RightToolWindowView::Maven, false, None, 8, 9001),
            "S1_RIGHT_PANEL seq=8 t_ms=9001 view=maven visible=false"
        );
    }

    /// 坐标**只被除一次**：gpui 给的窗口坐标本来就是逻辑像素（
    /// `gpui-pre-windows-0.3.6/src/util.rs:150-156` 的 `logical_point` 已经除过 `scale_factor`），
    /// 诊断这一层再缩放一次就会造出"125% DPI 下坐标不对"的假象。
    #[test]
    fn probe_point_keeps_logical_pixels_unscaled() {
        use gpui_kit::{Point, px};

        let point = ProbePoint::from(Point {
            x: px(1421.6),
            y: px(123.2),
        });
        assert_eq!(point, ProbePoint::new(1421.6, 123.2));
    }
}
