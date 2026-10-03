//! 项目标签条（`ProjectTabBar`）—— GPUI 外壳的「项目标签条」区域。
//!
//! # 接口契约
//!
//! 本模块**无状态**：数据（`tabs` / `active` / `scroll`）与回调（`on_activate` / `on_close`）
//! 全部由调用方传入，交互只通过回调往外抛。它不持有 `Entity` / `Context`，渲染调用方式：
//!
//! ```ignore
//! project_tabs(
//!     &tabs,
//!     Some(active_index),
//!     &self.project_tabs_scroll,   // 调用方（ShellWorkspace）持有，滚动状态归它
//!     move |index, window, cx| { /* 切到第 index 个项目 */ },
//!     move |index, window, cx| { /* 关闭第 index 个项目 */ },
//!     window,
//!     cx,
//! )
//! ```
//!
//! 两个回调都按 `Fn`（可多次调用）取，内部用 `Rc` 分发给每个标签。
//!
//! # 规格出处（**macOS 前端**，2026-09-30 起）
//!
//! ⚠️ 本模块早先的规格是 `windows/tauri/src/features/window/components/project-tab-bar.tsx`。
//! 那份前端**已从仓库删除**、其数值（容器 `h-8` = 32、标签 `h-7` = 28、min-w 144 / max-w 240）
//! 全部作废；`develop-lithe` Skill 明确要求界面规格一律读 `macos/Sources/Lithe/Views/`。
//!
//! | 元素 | 值 | 出处（macOS） |
//! | --- | --- | --- |
//! | 容器高 | **38** | `WorkbenchView.swift:653`（`.frame(height: LitheTheme.Metrics.tabHeight + 4)`）；`tabHeight = 34` 在 `LitheTheme.swift:477` |
//! | 容器水平内衬（在**滚动区内部**） | **6** | `WorkbenchView.swift:620,637`（`horizontalPadding: CGFloat = 6` + `.padding(.horizontal, horizontalPadding)` 挂在 ScrollView 的**内容**上） |
//! | 标签间距 | **6** | `WorkbenchView.swift:621`（`tabSpacing: CGFloat = 6`） |
//! | 标签高 | **30** | `WorkbenchView.swift:705`（`.frame(width: width, height: 30)`） |
//! | 标签最小宽 / 按可用宽度**等分** | **180** / 等分 | `WorkbenchView.swift:622-627`（`minimumTabWidth = 180`、`max(minimumTabWidth, floor(availableWidth / projectCount))`） |
//! | 标签内左内衬 / 右内衬 | **38 / 3** | `WorkbenchView.swift:683`（`.padding(.horizontal, 38)`）、`:703`（`.padding(.trailing, 3)`） |
//! | 标签圆角 | **6** | `WorkbenchView.swift:707,715`（`RoundedRectangle(cornerRadius: 6, style: .continuous)`）= `LitheTheme.Metrics.controlCornerRadius`（`LitheTheme.swift:484`） |
//! | 文件夹图标 | **11** | `WorkbenchView.swift:666-667`（`Image(systemName: "folder.fill").font(.system(size: 11, weight: .medium))`） |
//! | 项目名字号 | **12.5**（选中 semibold） | `WorkbenchView.swift:671` |
//! | 选中下划线 | 宽 `min(56, max(28, w × 0.12))`、高 **2**、底距 **1**、胶囊 | `WorkbenchView.swift:723-727` |
//! | 关闭按钮 | **28×28**、`xmark` 9pt、仅选中或悬停可见 | `WorkbenchView.swift:691-703`；28×28 是 `LitheIconButtonStyle`（`LitheTheme.swift:547`） |
//! | 溢出处理 | **横向滚动**（`ScrollView(.horizontal, showsIndicators: false)`）、切换项目时滚到可见 | `WorkbenchView.swift:629-651` |
//! | 显示条件 | 单项目时整条隐藏 | `WorkbenchView.swift:253-255`（调用方判据） |
//!
//! ## 与旧实现的差异（本轮改的四处）
//!
//! 1. 容器高 **32 → 38**、标签高 **28 → 30**（旧值来自已删除的 `windows/tauri` 前端）；
//! 2. 标签宽度从"定宽 144–240"改成真源的 **等分 + 最小 180**；
//! 3. **横向滚动**接上（旧实现自陈"超出宽度时标签会被裁掉"）——滚动归调用方持有的
//!    [`ScrollHandle`]，内容内衬放在**滚动区内**（真源同样如此）；
//! 4. 圆角 4.8 → 6（真源的 `controlCornerRadius`）。
//!
//! ## ⚠️ 未接的两个"信息元素"（真源有、本侧没有生产方）
//!
//! 真源标签里除图标与项目名之外还有两格：
//!
//! - **当前文档名后缀** `"· \(documentName)"`（`WorkbenchView.swift:676-680`，11.5pt tertiary）；
//! - **`AgentAttentionIndicator`**（`WorkbenchView.swift:674`，6×6 accent 圆点，
//!   条件是 `model.agentConversationNeedsAttention`）。
//!
//! 两格都**不能**在本轮诚实接上：`ShellWorkspace::projects` 是**单个**工作区根
//! （`workspace.rs` 构造期只有 `vec![ProjectTab::new(project_name)]`），所以这一条标签条
//! 在当前宿主里**根本不渲染**（判据 `projects.len() > 1`），而"每个项目各自的当前文档名"
//! 这个数据在单项目外壳里没有维护者；`AgentAttentionIndicator` 的数据源（agent 会话）在
//! gpui 宿主里**不存在**。加一个永真的假字段只会变成死代码 —— 按《编码指南》的占位项纪律
//! 不画、不写（缺口登记在交付报告里，等"多项目 + agent"到位后一次接上）。
//!
//! # 主题 token 映射（一律走 `cx.theme()`，不写裸色值）
//!
//! | 用途 | 取值 |
//! | --- | --- |
//! | 容器底 | `theme.tab_bar`（真源 `LitheTheme.toolHeader`，`WorkbenchView.swift:654`） |
//! | 标签·选中底 | `theme.accent` |
//! | 标签·未选中底 | `theme.secondary` |
//! | 标签·悬停底 | `theme.accent` |
//! | 标签·选中描边 | `theme.ring` @ 0.7 |
//! | 标签·未选中描边 | `theme.border` |
//! | 下划线 | `theme.primary` |
//! | 项目名 / 次要前景 | `theme.foreground` / `theme.muted_foreground` |
//!
//! # 本模块仍未实现的部分（见交付报告）
//!
//! - **键盘焦点与 focus-visible 焦点环**：真源标签是 `Button`，gpui 要做这件事必须
//!   `track_focus(&handle)` 且句柄跨帧稳定，本模块无状态、没有地方放它。**未实现**。
//! - **`onHover` 高亮整条标签**：真源用 `@State hoveredProjectTabID` 让"悬停任一子元素"
//!   都高亮整条（`WorkbenchView.swift:242,729-731`）。gpui 的 `hover()` 只认自己的命中区，
//!   关闭按钮悬停时整条不会高亮 —— 需要 `group_hover` 之类的祖先感知，本模块只做了
//!   标签本体的悬停，关闭按钮的显现由 `group_hover` 负责。
//! - **拖拽重排**：真源没有（`ForEach` 无拖拽处理），本模块也不做。

use std::rc::Rc;

// `h_flex()` 是函数、在 `gpui_kit::base`，**不在** `gpui_kit` 根。
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
// `Button` / `ButtonVariants` 在 `gpui_kit::component::button` **子模块**里。
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Sizable as _};
// `.when(cond, f)` 在 `FluentBuilder` trait 上，不 `use ... as _` 就会报
// `no method named when found for Div`。
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, ScrollHandle, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, rems,
};

use lithe_gpui_shared::icons::{idea, idea_icon_image};
use lithe_gpui_shared::tr_args;

// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`。真源的四个值里 6 / 30 / 38 / 180
// 只有 6 落在 gpui 的 4px 档位上，其余一律写 `rems(P / 16.)`（1 rem = 16px，**不是** 4px）。

/// 标签条容器高 **38**。
///
/// 真源：`WorkbenchView.swift:653` 的 `.frame(height: LitheTheme.Metrics.tabHeight + 4)`
/// = `34 + 4`（`LitheTheme.swift:477`）。原来本侧是 32（旧 `windows/tauri` 前端的 `h-8`）。
/// 38 不在 4px 档位上（`h_9()` = 36、`h_10()` = 40）→ 调用点写 `rems(STRIP_HEIGHT_SPEC / 16.)`。
const STRIP_HEIGHT_SPEC: f32 = 38.;

/// 标签高 **30**（真源 `WorkbenchView.swift:705`）。
///
/// 30 不在 4px 档位上（`h_7()` = 28、`h_8()` = 32）→ 调用点写 `rems(TAB_HEIGHT_SPEC / 16.)`。
const TAB_HEIGHT_SPEC: f32 = 30.;

/// 标签最小宽 **180**（真源 `WorkbenchView.swift:622` 的 `minimumTabWidth`）。
///
/// 真源的宽度是 `max(180, floor(可用宽度 / 项目数))` —— 本侧用 flex 表达同一件事：
/// 每个标签 `flex_1()`（等分）+ `min_w(180)`（下限），容器装不下时内容溢出、由滚动区接管。
/// 180 不在 4px 档位上 → 调用点写 `rems(TAB_MIN_WIDTH_SPEC / 16.)`。
const TAB_MIN_WIDTH_SPEC: f32 = 180.;

/// 标签左右内衬 **38**（真源 `WorkbenchView.swift:683` 的 `.padding(.horizontal, 38)`）。
///
/// ⚠️ 这个 38 是**给关闭按钮让位**的：关闭按钮绝对定位在右端（`:691-703`，`.trailing 3`
/// + 28 宽的图标按钮），38 > 3 + 28 = 31，所以文字永远不会压到按钮上。
const TAB_PADDING_SPEC: f32 = 38.;

/// 关闭按钮距标签右边缘 **3**（真源 `WorkbenchView.swift:703` 的 `.padding(.trailing, 3)`）。
const CLOSE_BUTTON_TRAILING_SPEC: f32 = 3.;

/// 文件夹图标 **11**（真源 `WorkbenchView.swift:666-667` 的 `size: 11`）。
///
/// 11 不在 gpui 的 rem 档位上（档位步长是 2px：`size_2p5()` = 10、`size_3()` = 12），
/// 也不能自己发明 helper → 调用点写 `rems(FOLDER_ICON_SIZE_SPEC / 16.)`。
const FOLDER_ICON_SIZE_SPEC: f32 = 11.;

/// 选中下划线（胶囊）宽 **28**、高 **2**、距标签底 **1**。
///
/// 真源是 `min(56, max(28, width × 0.12))`（`WorkbenchView.swift:726`）—— 它**依赖标签的
/// 实际宽度**，而本侧标签宽度是 flex 求出来的、`render` 期拿不到。取公式**下界 28**：
/// 在真源的最小宽 180 上 `180 × 0.12 = 21.6 → max(28, ..) = 28`，也就是"标签越窄越准、
/// 越宽真源越宽"的那一端。不发明动态宽度（那需要几何回调，超出本区域契约）。
const TAB_UNDERLINE_WIDTH_SPEC: f32 = 28.;
/// 下划线高 2（真源 `WorkbenchView.swift:726` 的 `height: 2`）→ `h_0p5()`。
/// 下划线距标签底 1（真源 `:727` 的 `.padding(.bottom, 1)`）。
const TAB_UNDERLINE_BOTTOM_SPEC: f32 = 1.;

/// 关闭按钮尺寸 **28**（真源走 `LitheIconButtonStyle`：`LitheTheme.swift:547` 的
/// `.frame(width: 28, height: 28)`）。
///
/// ⚠️ 这里**必须**经 [`crate::rem_px`]：走的是 `Sizable::with_size(impl Into<Size>)`，`Size`
/// 只有 `From<Pixels>`，**没有** `From<Rems>`。基准取当帧的 `window.rem_size()`。
const CLOSE_BUTTON_SIZE_SPEC: f32 = 28.;

/// 悬停分组名（`group` / `group_hover` 的配对键）。
const CLOSE_GROUP: &str = "project-tab-group";

// ---------------------------------------------------------------------------
// 接口契约
// ---------------------------------------------------------------------------

/// 项目标签条里的一个项目标签。
///
/// 只承载渲染必需的字段（契约只给了一个：项目名）。路径 / 自定义图标 / 显示别名等
/// Windows 侧字段该由调用方先折算成 `name` 再传进来 —— 本模块不做那层逻辑。
///
/// 字段私有 + [`ProjectTab::new`]：跨 crate 之后结构体字面量不再是合法构造方式
/// （《编码指南》「公共 API 设计」）。
#[non_exhaustive]
pub struct ProjectTab {
    /// 标签上显示的项目名。截断由本模块负责。
    name: SharedString,
}

impl ProjectTab {
    /// 一个项目标签（展示名由调用方先折算好）。
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self { name: name.into() }
    }

    /// 标签上显示的项目名。
    pub fn name(&self) -> &SharedString {
        &self.name
    }
}

/// 渲染项目标签条。
///
/// - `tabs`：要显示的标签，**顺序即显示顺序**；
/// - `active`：当前选中项的索引（`None` 表示一个都没选中）；
/// - `scroll`：横向滚动的 [`ScrollHandle`]，由调用方（`ShellWorkspace`）持有 ——
///   它要跨帧活着，也需要在切换项目后由回调 `scroll_to_item` 把活动项滚进可视区
///   （真源 `WorkbenchView.swift:640-650` 的行为）；
/// - `on_activate(index, window, cx)`：点击某个**未选中**的标签时调用；
/// - `on_close(index, window, cx)`：点击某个标签的关闭按钮时调用；
/// - `window` / `cx`：宿主窗口与应用上下文。`window` 用于取**运行时 rem 基准**
///   （`window.rem_size()`）：关闭按钮的 `Sizable::with_size` 只吃 `Pixels`（见
///   [`CLOSE_BUTTON_SIZE_SPEC`]），其余度量直接写 `rems(P / 16.)`。
///
/// 返回元素**不含**「单项目时隐藏」的判断 —— 那是调用方的条件
/// （`WorkbenchView.swift:253-255`），本模块只负责画。
pub fn project_tabs(
    tabs: &[ProjectTab],
    active: Option<usize>,
    scroll: &ScrollHandle,
    on_activate: impl Fn(usize, &mut Window, &mut App) + 'static,
    on_close: impl Fn(usize, &mut Window, &mut App) + 'static,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    // 回调用 `Rc` 分发：契约给的是 `Fn`（可重复调用），而 `Div::on_click` 要的是 `'static`
    // 的 `Fn`，每个标签都要自己的一份所有权。
    let rem = window.rem_size();
    let on_activate: Rc<dyn Fn(usize, &mut Window, &mut App)> = Rc::new(on_activate);
    let on_close: Rc<dyn Fn(usize, &mut Window, &mut App)> = Rc::new(on_close);

    // 先把要用的 token 全部取出来：`cx.theme()` 借 `cx`，而下面的闭包也要 `cx`。
    // 容器底色用 `tab_bar`（真源 `LitheTheme.toolHeader`）。
    let strip_bg = cx.theme().tab_bar;
    let border = cx.theme().border;
    let accent = cx.theme().accent;
    let inactive_bg = cx.theme().secondary;
    let foreground = cx.theme().foreground;
    let subtle_foreground = cx.theme().muted_foreground;
    let primary = cx.theme().primary;
    let focus_border = cx.theme().ring;

    // 容器：高 38、贴住整幅宽度、不参与压缩；**8px 水平内衬不在这里**
    // （真源把内衬放在滚动区的内容上，见模块头表格）。
    h_flex()
        .w_full()
        .h(rems(STRIP_HEIGHT_SPEC / 16.))
        .flex_shrink_0()
        .min_w_0()
        .items_center()
        .bg(strip_bg)
        .border_b_1()
        .border_color(border)
        // 真正的滚动区域：`flex_1()` + 横向滚动 + 跟踪调用方的 `ScrollHandle`。
        // ⚠️ 滚动条槽位与内容内衬都在**滚动区内部**（《设计指南》
        // `docs/gpui kit/0.7.0/docs/design-guides.md:197`：「滚动条属于实际滚动的区域」、
        // 「内容内边距可以缩进文字与行，但不能把滚动条推到界面中间」）。
        .child(
            h_flex()
                .id("project-tabs-scroll")
                .flex_1()
                .min_w_0()
                .h_full()
                .items_center()
                .overflow_x_scroll()
                .track_scroll(scroll)
                .child(
                    // 内容行：间距 6、左右内衬 6（两者都在滚动区内）。
                    h_flex()
                        .h_full()
                        .items_center()
                        .gap_1p5()
                        .px_1p5()
                        .children(tabs.iter().enumerate().map(|(index, tab)| {
                            let is_active = active == Some(index);
                            let name = tab.name().clone();
                            let tooltip = tab.name().clone();

                            let activate = on_activate.clone();
                            let close = on_close.clone();

                            // 每个标签是一个 `relative` 包装：标签本体（可点，也是 hover group
                            // 的宿主）+ 绝对定位在右侧的关闭按钮。
                            //
                            // 宽度：`flex_1()` + `min_w(180)` + `flex_shrink_0()`。三项合起来就是
                            // 真源的 `max(minimumTabWidth, floor(availableWidth / projectCount))`：
                            // 容器装得下时等分（flex_grow），装不下时各自保持 180 并把内容撑出
                            // 容器，由上面的滚动区接管（flex_shrink_0 保证不被压回去）。
                            div()
                                .relative()
                                .flex_1()
                                .flex_shrink_0()
                                .min_w(rems(TAB_MIN_WIDTH_SPEC / 16.))
                                .h(rems(TAB_HEIGHT_SPEC / 16.))
                                .child({
                                    let mut label = h_flex()
                                        // `ElementId` 用**业务身份**（标签所属的项目名），
                                        // 不用下标：关掉一个项目后下标会整体前移，下标当 id
                                        // 会让 hover / 关闭态串到邻居身上。
                                        .id(SharedString::from(format!("project-tab-{name}")))
                                        .relative()
                                        .size_full()
                                        .items_center()
                                        // 组内间距 7（真源 `WorkbenchView.swift:665` 的
                                        // `HStack(spacing: 7)`）。
                                        .gap_1p5()
                                        // 真源是 `.padding(.horizontal, 38)`（`:683`）。
                                        .px(rems(TAB_PADDING_SPEC / 16.))
                                        .rounded_md()
                                        .text_sm()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .cursor_pointer()
                                        // 悬停分组：让关闭按钮在鼠标进入标签时显现。
                                        .group(CLOSE_GROUP)
                                        .on_click(move |_, window, cx| {
                                            // 点已选中的标签是 no-op（真源同）。
                                            if !is_active {
                                                activate(index, window, cx);
                                            }
                                        })
                                        .child(
                                            // 真源 `Image(systemName: "folder.fill")`（11pt，
                                            // `WorkbenchView.swift:666`）。有真源资源就用真源
                                            // （`idea::FOLDER_ICON`，与文件树同一族），颜色由
                                            // SVG 自带，不随选中态变。
                                            idea_icon_image(
                                                &idea::FOLDER_ICON,
                                                cx,
                                                rems(FOLDER_ICON_SIZE_SPEC / 16.),
                                            ),
                                        )
                                        .child(
                                            // 文案容器：`min-w-0 truncate`。
                                            // `min_w_0` 是 flex 子项能被压缩的前提，否则
                                            // `text_ellipsis` 不会生效。
                                            div()
                                                .min_w_0()
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .text_ellipsis()
                                                // 字号：真源 12.5（`:671`）；gpui 档位只有
                                                // 12（`text_xs()`）/ 14（`text_sm()`），按本仓
                                                // 既有口径（chrome 文字 13 → 14）取 `text_sm()`。
                                                .text_color(if is_active {
                                                    foreground
                                                } else {
                                                    subtle_foreground
                                                })
                                                .child(name),
                                        );

                                    // 选中态 / 悬停态底色（真源 `:706-713`）。
                                    if is_active {
                                        label = label.bg(accent);
                                    } else {
                                        label = label
                                            .bg(inactive_bg)
                                            .hover(move |this| this.bg(accent));
                                    }

                                    // 选中下划线：真源是**居中的胶囊**
                                    // `min(56, max(28, w × 0.12)) × 2`、底距 1（`:723-727`）。
                                    // 宽度取公式下界 28（见 [`TAB_UNDERLINE_WIDTH_SPEC`]）。
                                    if is_active {
                                        label = label.child(
                                            div()
                                                .absolute()
                                                .bottom(rems(TAB_UNDERLINE_BOTTOM_SPEC / 16.))
                                                .left_0()
                                                .right_0()
                                                .flex()
                                                .justify_center()
                                                .child(
                                                    div()
                                                        .w(rems(TAB_UNDERLINE_WIDTH_SPEC / 16.))
                                                        .h_0p5()
                                                        .rounded_full()
                                                        .bg(primary),
                                                ),
                                        );
                                    }

                                    label
                                })
                                // 选中描边（真源 `:714-722`）。
                                .when(is_active, |this| {
                                    this.child(
                                        div()
                                            .absolute()
                                            .inset_0()
                                            .rounded_md()
                                            .border_1()
                                            .border_color(focus_border.opacity(0.7)),
                                    )
                                })
                                .child(
                                    // 关闭按钮容器：绝对定位在右端，选中时常显，
                                    // 未选中时 `opacity-0` + 悬停标签才显现（真源 `:691-703`）。
                                    h_flex()
                                        .absolute()
                                        .top_0()
                                        .bottom_0()
                                        .right(rems(CLOSE_BUTTON_TRAILING_SPEC / 16.))
                                        .items_center()
                                        .when(!is_active, |this| {
                                            this.opacity(0.)
                                                .group_hover(CLOSE_GROUP, |this| this.opacity(1.))
                                        })
                                        .child(
                                            // `ElementId` 同上：业务身份 = 项目名。
                                            Button::new(SharedString::from(format!(
                                                "project-tab-close-{tooltip}"
                                            )))
                                            .icon(IconName::X)
                                            .ghost()
                                            // `Sizable::with_size(impl Into<Size>)`；
                                            // `Pixels` 经 `From<Pixels> for Size` 变成
                                            // `Size::Size(28)`。`Size` 没有 `From<Rems>`，
                                            // 所以这一处按当帧 rem 基准换算。
                                            .with_size(crate::rem_px(rem, CLOSE_BUTTON_SIZE_SPEC))
                                            .tooltip(tr_args(
                                                "lithe.titleProject.closeProject",
                                                &[("name", tooltip.as_ref())],
                                            ))
                                            .on_click(move |_, window, cx| {
                                                close(index, window, cx);
                                            }),
                                        ),
                                )
                        })),
                ),
        )
}
