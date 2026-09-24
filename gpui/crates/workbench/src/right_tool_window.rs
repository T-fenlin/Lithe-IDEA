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
//! | 通知 | `NotificationsTrigger`（`plugin-activity-rail.tsx:50`） | `NotificationsToolWindow`（`features/notifications/components/notifications-tool-window.tsx:340-475`） | 头部 + 空态（`notifications.empty`）。**未做**：搜索 / 过滤 / 分组 / 详情 —— 没有通知数据源 |
//! | 扩展 | `PuzzlePieceIcon`（`plugin-activity-rail.tsx:36-49`） | ⚠️ **真机里这个按钮不是右栏视图**：它调 `openExtensionsBuffer`，扩展是一个**编辑器缓冲区**（同文件 `:13,46`） | 按本轮任务要求做成右栏视图（头部 + 空态）。**这是有意偏离真源**，理由与后续收敛路径写在 `PLAN.md` §11.3 |
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
//! | 关闭按钮 | 24×24、圆角 6.4 | `size="icon-xs"`（`notifications-tool-window.tsx:353`）、`rounded-md`（`ui/button.tsx:9`） | `size_6()` + `px(CLOSE_BUTTON_RADIUS)` |
//!
//! 颜色一律 `cx.theme()`；圆角保留 `px(...)`，遵循仓库既有约定
//! （Lithe 的圆角阶梯走应用层具名常量，见 `explorer_view.rs:87-93`）。

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::{ActiveTheme as _, Icon, StyledExt as _};
use gpui_kit::{
    AnyElement, App, ClickEvent, InteractiveElement as _, IntoElement, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};

use lithe_gpui_shared::tr;

/// 关闭按钮圆角 6.4（`ui/button.tsx:9` 的 `rounded-md` = `--radius × 0.8`，`theme.css:7,134`）。
///
/// ⚠️ **保留 `px(...)`**：6.4 不是 gpui 的 rem 档位（`rounded_md()` 是 6），且 Lithe 的圆角阶梯
/// 一律走应用层具名常量 —— 与 `explorer_view.rs:87-93` 的 `HEADER_BUTTON_RADIUS` 同一条理由。
const CLOSE_BUTTON_RADIUS: f32 = 6.4;

/// 右侧工具窗当前显示哪一个视图。
///
/// 取值与真机 `activeRightSidebarView` 的子集对应（`SidebarView` 见
/// `features/layout/utils/sidebar-pane-utils.ts`；本侧只实现右活动栏三项各自的那一个）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightToolWindowView {
    /// 扩展（真机是编辑器缓冲区，本侧按任务要求做成右栏视图 —— 见模块文档）。
    Extensions,
    /// 通知（`activeRightSidebarView === "notifications"`）。
    Notifications,
    /// Maven（`activeRightSidebarView === "maven"`）。
    Maven,
}

impl RightToolWindowView {
    /// 诊断行里的视图名（`S1_RIGHT_PANEL view=…`，可 grep）。
    ///
    /// 用真机的 `activeRightSidebarView` 取值口径（`"notifications"` / `"maven"`），
    /// 便于和真源对照；扩展没有对应的真机取值，取 `extensions`。
    pub const fn id(self) -> &'static str {
        match self {
            Self::Extensions => "extensions",
            Self::Notifications => "notifications",
            Self::Maven => "maven",
        }
    }

    /// 右活动栏下标 → 视图；`None` = 该下标没有对应视图。
    ///
    /// ⚠️ 下标必须与 [`crate::workspace::right_activity_items`] 的顺序一致
    /// （0 扩展 / 1 通知 / 2 Maven，照 `plugin-activity-rail.tsx:31-67`）。
    pub const fn from_rail_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(Self::Extensions),
            1 => Some(Self::Notifications),
            2 => Some(Self::Maven),
            _ => None,
        }
    }

    /// 头部标题。
    ///
    /// 键取自真源：`maven.title`（`maven-pane.tsx:602`）、`notifications.title`
    /// （`notifications-tool-window.tsx:348`）、`extensions.title`
    /// （扩展在真机是缓冲区标题，`plugin-activity-rail.tsx:28` 的 `extensionsLabel` 是同一个键）。
    fn title(self) -> SharedString {
        match self {
            Self::Extensions => tr("lithe.extensions.title"),
            Self::Notifications => tr("lithe.notifications.title"),
            Self::Maven => tr("lithe.maven.title"),
        }
    }

    /// 头部图标。与右活动栏逐项同字形（`activity_bar.rs` 的对照表）。
    fn icon(self) -> IconName {
        match self {
            Self::Extensions => IconName::Puzzle,
            Self::Notifications => IconName::Bell,
            // Lucide 没有 Maven 字形，取「包 / 构建产物」语义的 `package`（与活动栏一致）。
            Self::Maven => IconName::Package,
        }
    }

    /// 空态标题。
    ///
    /// 三条都是真源**已有的**空态文案，不是新写的：
    /// - Maven：`maven.notDetected`（"未检测到 Maven 项目"，`maven-pane.tsx:806` 在
    ///   `projectStatus !== "loading"` 且没有项目时显示的那一句）；
    /// - 通知：`notifications.empty`（`notifications-tool-window.tsx:399` 的 `CommandEmpty`）；
    /// - 扩展：`extensions.noneFound`（`extensions.noneFound` = "未找到扩展。"，
    ///   `locale.ts:7620`，扩展面板搜不到任何扩展时的空态）。
    ///
    /// Maven 的空态是**如实**的：本侧还没有 Maven 项目探测（真机也只探测到 Maven 项目时才
    /// 渲染右栏那一项，`plugin-activity-rail.tsx:21-23,51`），所以"未检测到 Maven 项目"
    /// 就是当前的真实状态，不是占位文案。
    fn empty_title(self) -> SharedString {
        match self {
            Self::Extensions => tr("lithe.extensions.noneFound"),
            Self::Notifications => tr("lithe.notifications.empty"),
            Self::Maven => tr("lithe.maven.notDetected"),
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

/// 打一行右工具窗诊断（可 grep，与其他 `S1_*` 同一口径）。
///
/// 每次状态迁移打一行，构造期也打一行 —— 这样"启动时面板是隐藏的"这件事本身有日志证据，
/// 不必只靠截图。
pub fn diagnose(view: RightToolWindowView, visible: bool) {
    println!("S1_RIGHT_PANEL view={} visible={visible}", view.id());
}

/// 画右侧工具窗。
///
/// 结构照真机：`<section>` 头部（图标 + 标题 + 关闭按钮）+ 内容区。本轮的三个视图都只有
/// 空态内容（见模块文档的完成度表）。
///
/// `on_close` 由调用方给：可见性归外壳（`ShellWorkspace`），本函数不持有状态。
pub fn right_tool_window(
    view: RightToolWindowView,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let title = view.title();
    v_flex()
        .id("lithe-right-tool-window")
        // 真机是 `<section aria-label={t("maven.title")}>`（`maven-pane.tsx:597`）；
        // gpui 侧 `.aria_label()` 挂在 `StatefulInteractiveElement` 上，所以根元素带 `id`。
        .aria_label(title.clone())
        .size_full()
        .child(header(view, title, on_close, cx))
        .child(empty_state(view))
        .into_any_element()
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
        .h_8()
        .gap_2()
        .px_3()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            Icon::new(view.icon())
                .size_4()
                .flex_shrink_0()
                .text_color(cx.theme().primary),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_sm()
                // 真机标题字重：通知是 `font-semibold`（`notifications-tool-window.tsx:347`）、
                // Maven 是 `font-medium`（`maven-pane.tsx:601`）。取较粗的那一档统一。
                .font_semibold()
                .text_color(cx.theme().foreground)
                .child(title),
        )
        .child(
            // `Button variant=ghost size=icon-xs`：24×24（`ui/button.tsx:27`）。
            // `ghost()` 与 `explorer_view.rs` 的 `header_button` 同一取法（前景 `muted_foreground`）。
            Button::new("right-tool-window-close")
                .ghost()
                .icon(IconName::X)
                .tab_stop(false)
                .size_6()
                .rounded(px(CLOSE_BUTTON_RADIUS))
                .tooltip(close_label.clone())
                .accessibility_label(close_label)
                .on_click(on_close),
        )
        .into_any_element()
}

/// 视图内容区：本轮三个视图都是空态。
///
/// 用 gpui-kit 的 `Empty`（`gpui/docs/gpui-kit/0.6.6/zh-CN/component/empty.md`）——
/// 成熟组件已经负责居中、间距与标题层级，与 `terminal_view.rs:402-435` 的空态同一用法。
///
/// ⚠️ 外面必须套一层 `v_flex().flex_1()`：`Empty` 的根是 `v_flex().flex_1()`
/// （`gpui-component-0.6.6/src/empty.rs:63-82`），父级不是 flex 容器时它不生效
/// （`explorer_view.rs:765-768` 记过同一个坑）。
fn empty_state(view: RightToolWindowView) -> AnyElement {
    v_flex()
        .w_full()
        .flex_1()
        .min_h_0()
        .child(
            Empty::new().header(
                EmptyHeader::new()
                    .media(EmptyMedia::new().child(Icon::new(view.icon()).size_8()))
                    .title(EmptyTitle::new().text_sm().child(view.empty_title())),
            ),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{RightToolWindowView, resolve_click};

    /// 右活动栏下标 → 视图必须与 `workspace::right_activity_items()` 的顺序一一对应
    /// （0 扩展 / 1 通知 / 2 Maven），越界返回 `None` 而不是回落到某一项。
    #[test]
    fn rail_index_maps_to_view() {
        assert_eq!(
            RightToolWindowView::from_rail_index(0),
            Some(RightToolWindowView::Extensions)
        );
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
        assert_eq!(RightToolWindowView::Extensions.id(), "extensions");
        assert_eq!(RightToolWindowView::Notifications.id(), "notifications");
        assert_eq!(RightToolWindowView::Maven.id(), "maven");
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
            RightToolWindowView::Extensions,
            RightToolWindowView::Maven,
            false,
        );
        assert_eq!(view, RightToolWindowView::Extensions);
        assert!(visible);
    }
}
