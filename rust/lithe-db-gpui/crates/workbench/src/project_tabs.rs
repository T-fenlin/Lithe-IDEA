//! 项目标签条（`ProjectTabBar`）—— GPUI 外壳的「项目标签条」区域。
//!
//! # 接口契约
//!
//! 本模块**无状态**：数据（`tabs` / `active`）与回调（`on_activate` / `on_close`）全部由调用方传入，
//! 交互只通过回调往外抛。它不持有 `Entity` / `Context`，渲染调用方式：
//!
//! ```ignore
//! project_tabs(
//!     &tabs,
//!     Some(active_index),
//!     move |index, window, cx| { /* 切到第 index 个项目 */ },
//!     move |index, window, cx| { /* 关闭第 index 个项目 */ },
//!     window,
//!     cx,
//! )
//! ```
//!
//! 两个回调都按 `Fn`（可多次调用）取，内部用 `Rc` 分发给每个标签。
//!
//! # 规格出处（Windows 前端是界面真源）
//!
//! 全部来自 `windows/tauri/src/features/window/components/project-tab-bar.tsx`：
//!
//! | 元素 | 源码行 | 尺寸 |
//! | --- | --- | --- |
//! | 容器 `flex h-8 ... border-border border-b bg-surface px-1.5` | `:41` | 高 **32**、px **6**、1px 下边框 |
//! | 内层 `flex min-w-max items-center gap-1` | `:46` | gap **4** |
//! | 标签 `h-7 min-w-36 max-w-60 gap-1.5 rounded-sm py-0 pr-8 pl-2.5` | `:64` | 高 **28**、min-w **144**、max-w **240**、gap **6**、pr **32**、pl **10** |
//! | 标签·选中 `border-transparent bg-accent text-foreground` | `:66` | 背景 `--accent` |
//! | 标签·未选中 `text-subtle-foreground hover:bg-accent/70 hover:text-foreground` | `:67` | 悬停底色 = accent @ 70% |
//! | 文件夹图标 `size-3.5`，选中 `text-primary` / 未选中 `text-subtle-foreground` | `:70-75` | **14×14** |
//! | 选中下划线 `absolute inset-x-1.5 bottom-0 h-0.5 rounded-t-sm bg-primary` | `:79-82` | 高 **2**、左右内缩 **6**、上圆角 4.8 |
//! | 关闭按钮容器 `absolute inset-y-0 right-1 z-10`，非选中 `opacity-0` + `group-hover` 显示 | `:86-91` | right **4** |
//! | 关闭按钮 `Button size=icon-xs variant=ghost` + tooltip `titleProject.closeProject` | `:93-104` | **24×24** |
//! | 标签文案 `getProjectDisplayLabel(project)`，`min-w-0 truncate` | `:48`、`:77` | 截断 |
//! | 显示条件 `shouldShowProjectTabBar(count, hideWhenSingle)` | `project-tab-bar-model.ts:12-13` | 单项目时整条隐藏 |
//!
//! ## ⚠️ 与旧文档的差异：高度是 32px，不是 38px
//!
//! 历史记录和本仓库旧笔记写「项目标签条高 38px」，**这是错的**。Windows 真源容器是 `h-8`
//! （Tailwind 1 单位 = 4px ⇒ **32px**，`project-tab-bar.tsx:41`），标签本体 `h-7` ⇒ **28px**（`:64`）。
//! 本模块按 **32 / 28** 实现。
//!
//! ## 圆角换算（来自 `windows/tauri/src/styles/theme.css`）
//!
//! `:root { --radius: 8px }`（`theme.css:134`），派生圆角定义在 `theme.css:6-11`：
//!
//! | Tailwind 档 | 计算式 | 实际 px | 用在哪 |
//! | --- | --- | --- | --- |
//! | `rounded-sm` | `calc(var(--radius) * 0.6)` = `8 × 0.6` | **4.8** | 标签本体（`:64`）、下划线上圆角（`:81`） |
//! | `rounded-md` | `calc(var(--radius) * 0.8)` = `8 × 0.8` | 6.4 | 本区域不用 |
//! | `rounded-xl` | `calc(var(--radius) * 1.4)` = `8 × 1.4` | 11.2 | 本区域不用 |
//!
//! ⚠️ **不要**用 gpui 的 `rounded_sm()`：gpui 的圆角梯度是 Tailwind 默认（`rems(0.25)` = 4px，
//! `gpui-pre-macros-0.3.6/src/styles.rs:1240-1244`），与 Lithe 的 `--radius-sm` 4.8 不是一回事。
//! 本模块一律显式写 `rems(4.8 / 16.)`（`Styled::rounded*` 收 `impl Into<AbsoluteLength>`）。
//!
//! ## 主题 token 映射（一律走 `cx.theme()`，不写裸色值）
//!
//! | Lithe（theme.css） | gpui-kit 0.6.6 | 说明 |
//! | --- | --- | --- |
//! | `--surface`（容器底） | `theme.background` | theme.css 把 surface 与 background 分开；gpui-kit 的 `popover` 默认等于 `background`，没有独立 surface token。深色主题下两者同黑 |
//! | `--accent`（选中底） | `theme.accent` | 0.6.6 默认值**不透明**：light `neutral-100` / dark `neutral-800`（`theme/default-theme.json:13`、`:218`） |
//! | `--accent/70`（悬停底） | `theme.accent.alpha(0.7)` | 与 Lithe 的 70% 一致 |
//! | `--border`（1px 下边框） | `theme.border` | — |
//! | `--primary`（图标/下划线） | `theme.primary` | — |
//! | `--foreground` | `theme.foreground` | — |
//! | `--subtle-foreground` | `theme.muted_foreground` | gpui-kit 无 `subtle-foreground` token（`UI-MAP.md:619` 已记录该缺口），取最近的「次要前景色」 |
//!
//! # 图标
//!
//! **有真源的用真源，没真源的保持 Lucide**（与活动栏 / 状态栏同一口径）：
//!
//! | 用途 | 采用 | 真源 / 理由 |
//! | --- | --- | --- |
//! | 标签左侧文件夹 | **真源** `idea::FOLDER_ICON` | `ui-icons/idea/expui/nodes/folder.svg`(+`_dark`) —— 真机 `FolderIcon`（`project-tab-bar.tsx:6,70`）。⚠️ 真源美术**自带颜色**（浅灰填充 + 灰描边），所以不再跟标签的 `text_color` 变（改前是 Lucide 单色、随选中态变色）—— 真机也是彩色图标，见 `icon_themes` 的取舍说明 |
//! | 关闭按钮 | `IconName::X` | 真源 `expui/general/close.svg` 存在，但关闭按钮的字形与**活动栏按钮的交互色**（悬停/选中变前景色）绑得紧，换成单色 expui 会丢掉那层反馈；且 `x` 本来就是 Windows `XIcon` 的 Lucide 字形，**已经是 1:1**。保持 Lucide |
//!
//! # 本模块未实现的部分（见交付报告）
//!
//! - **键盘焦点与 focus-visible 焦点环**：Windows 的标签是 `<button role="tab">`，带
//!   `focus-visible:ring-2 focus-visible:ring-primary/30`（`project-tab-bar.tsx:64`）。
//!   gpui 要做这件事必须 `track_focus(&handle)`，而句柄得跨帧稳定（重建会导致每帧焦点丢失），
//!   本模块无状态、没有地方放它。**未实现**；建议由主代理在宿主视图里为每个标签持有
//!   `FocusHandle` 后另行传入（契约未覆盖）。
//! - **`group-focus-within`**：未选中的关闭按钮在 Windows 里靠 `group-focus-within` 于键盘
//!   聚焦时显现（`project-tab-bar.tsx:90`）。gpui 的 `group_hover` 没有「祖先含焦点」对应物
//!   （`gpui-pre-0.3.6/src/elements/div.rs:3432-3450` 只判断 group hitbox 的 hover），
//!   本模块只做了悬停那一半。
//! - **标签条横向滚动**：Windows 容器是 `overflow-x-auto`（`:41`）。gpui 的横向滚动要配
//!   `ScrollHandle` 才有意义，本模块没有状态可放；超出宽度时标签会被裁掉。
//! - **`+` 新建 / 搜索按钮 / 溢出菜单**：Windows 的 `ProjectTabBar`（`:39-113`）里**没有**
//!   这些元素，所以不实现。
//! - **拖拽重排**：Windows 的 store 有 `reorderProjectTabs`（`utils/project-tab-order.ts`），
//!   但 `ProjectTabBar` 组件本身没有任何拖拽处理，本模块也不做。

use std::rc::Rc;

// `h_flex()` 是函数、在 `gpui_kit::base`，**不在** `gpui_kit` 根
// （`gpui-base-0.6.6/src/styled.rs:38`；`gpui-kit-0.6.6/src/lib.rs:106` 只有 `pub use ::gpui_base as base;`，
// 没有 `pub use gpui_base::*`）。
use gpui_kit::base::h_flex;
// `.when(cond, f)` 在 `FluentBuilder` trait 上（`gpui-pre-0.3.6/src/util.rs:21`），
// 不 `use ... as _` 就会报 `no method named when found for Div`。
use gpui_kit::assets::IconName;
use gpui_kit::prelude::FluentBuilder as _;
// `Button` / `ButtonVariants` 在 `gpui_kit::component::button` **子模块**里，
// 不在 `gpui_kit::component` 根（`gpui-component-0.6.6/src/lib.rs:32` 是 `pub mod button;`，
// 根层没有 `pub use button::*`）。
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, Sizable as _};
use gpui_kit::{
    App, InteractiveElement as _, IntoElement, ParentElement as _, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, rems,
};

use lithe_db_gpui_shared::icons::{idea, idea_icon_svg_px};
use lithe_db_gpui_shared::tr_args;

// ---------------------------------------------------------------------------
// 度量常量（全部来自 project-tab-bar.tsx，见模块头表格）
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`，与 Windows 规格逐像素相等
// （规格全部来自 `project-tab-bar.tsx`，见模块头表格）：
//
// | 规格 | 值 | 用到的 helper | 出处 |
// | --- | --- | --- | --- |
// | `h-8` 容器高 | 32 | `h_8()` | `:41` |
// | `px-1.5` 容器内边距 | 6 | `px_1p5()` | `:41` |
// | `gap-1` 标签间距 | 4 | `gap_1()` | `:46` |
// | `h-7` 标签高 | 28 | `h_7()` | `:64` |
// | `pl-2.5` 标签左内边距 | 10 | `pl_2p5()` | `:64` |
// | `pr-8` 标签右内边距 | 32 | `pr_8()` | `:64` |
// | `gap-1.5` 标签内间距 | 6 | `gap_1p5()` | `:64` |
// | `size-3.5` 文件夹图标 | 14 | `size_3p5()` | `:70` |
// | `h-0.5` / `inset-x-1.5` 下划线 | 2 / 6 | `h_0p5()` / `left_1p5()` / `right_1p5()` | `:81` |
// | `right-1` 关闭按钮容器 | 4 | `right_1()` | `:87` |
//
// 字号：`--ui-text-chrome` 是 **13px**，gpui 的档位只有 `text_xs()`(12) / `text_sm()`(14)，
// 13 不在档位上。按《编码指南》用 **`text_sm()`（14px）**——13 → 14 是经维护者确认的
// **有意**视觉改动，不是等价换算。

/// 标签最小宽度：`min-w-36` = 144px（`project-tab-bar.tsx:64`）。
///
/// 144 不在 gpui 的固定 rem 档位上（档位后缀是 `…_8()`=32、`_9()`=36、`_10()`=40 →
/// `min_w_32()`=128 / `min_w_40()`=160；没有 `min_w_36()`）—— 档位外就写 helper 底层的
/// `rems(TAB_MIN_WIDTH_SPEC / 16.)`（默认 16px 基准下与 144 逐像素相等，且随主题字号缩放）。
const TAB_MIN_WIDTH_SPEC: f32 = 144.;

/// 标签最大宽度：`max-w-60` = 240px（`project-tab-bar.tsx:64`）。
///
/// 240 不在 gpui 的固定 rem 档位上（档位里 56 → 224、64 → 256，
/// `gpui-pre-macros-0.3.6/src/styles.rs:1043-1052`）。Tailwind v4 的任意整数档
/// （`max-w-60`）在 gpui 里没有对应 helper，不能自己发明一个 —— 同样写
/// `rems(TAB_MAX_WIDTH_SPEC / 16.)`。
const TAB_MAX_WIDTH_SPEC: f32 = 240.;

/// 标签圆角：`rounded-sm` = `calc(var(--radius) * 0.6)` = `8px × 0.6` = **4.8px**
/// （`styles/theme.css:6`、`:134`）。
///
/// 两条理由都可核对：
/// 1. 4.8 **不是** gpui 的 rem 档位（gpui 的 `rounded_sm()` 是 4px，`styles.rs:1240-1244`）；
/// 2. 也不能从主题读：`ThemeConfig.radius` 是 `usize`（`gpui-component-0.6.6/src/theme/schema.rs:67-68`），
///    装不下 4.8 / 6.4；而把主题半径调成 8 会让**所有** gpui-kit 组件的圆角从 6 变成 8，
///    离 Lithe 的 `rounded-md`(6.4) 反而更远。
///
/// 所以 Lithe 的圆角阶梯只能作为应用层命名常量（见模块头「圆角换算」），调用点写
/// `rems(TAB_RADIUS_SPEC / 16.)`。
const TAB_RADIUS_SPEC: f32 = 4.8;

/// 关闭按钮尺寸：24（`project-tab-bar.tsx:95` 的 `size=icon-xs`；gpui-kit 的 `Size::XSmall`
/// 图标按钮默认只有 20×20，`gpui-component-0.6.6/src/button/button.rs:620`，所以显式传值）。
///
/// ⚠️ 这里**必须**经 [`crate::rem_px`]：走的是 `Sizable::with_size(impl Into<Size>)`，`Size`
/// 只有 `From<Pixels>`（`gpui-component-0.6.6/src/sizing.rs:169-183`），**没有** `From<Rems>`；
/// 换成 `Size::XSmall` 会连带改掉内边距与图标尺寸，不是逐像素等价。基准取当帧的
/// `window.rem_size()`。
const CLOSE_BUTTON_SIZE_SPEC: f32 = 24.;

/// 悬停底色透明度：Lithe 写 `hover:bg-accent/70`（`project-tab-bar.tsx:67`）。
const HOVER_BG_ALPHA: f32 = 0.7;

/// 悬停分组名（`group` / `group_hover` 的配对键）。
/// `InteractiveElement::group` 见 `gpui-pre-0.3.6/src/elements/div.rs:773`；
/// `group_hover` 见 `:852`；命中判定见 `:3432-3450`。
const CLOSE_GROUP: &str = "project-tab-group";

// ---------------------------------------------------------------------------
// 接口契约
// ---------------------------------------------------------------------------

/// 项目标签条里的一个项目标签。
///
/// 只承载渲染必需的字段（契约只给了一个）。路径 / 自定义图标 / 显示别名等 Windows 侧字段
/// （`ProjectTab`，`windows/tauri/src/features/window/stores/workspace-tabs.store.ts:21`）
/// 该由调用方先折算成 `name` 再传进来 —— 本模块不做 `getProjectDisplayLabel` 那层逻辑。
///
/// 字段私有 + [`ProjectTab::new`]：跨 crate 之后结构体字面量不再是合法构造方式
/// （《编码指南》「公共 API 设计」）。
#[non_exhaustive]
pub struct ProjectTab {
    /// 标签上显示的项目名（Windows 是 `getProjectDisplayLabel(project)` 的结果，
    /// `project-tab-bar.tsx:48`）。截断由本模块负责（`max-w-60` + `truncate`）。
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
/// - `on_activate(index, window, cx)`：点击某个**未选中**的标签时调用
///   （点已选中标签是 no-op，`project-tab-bar.tsx:60`）；
/// - `on_close(index, window, cx)`：点击某个标签的关闭按钮时调用；
/// - `window` / `cx`：宿主窗口与应用上下文。`window` 用于取**运行时 rem 基准**
///   （`window.rem_size()`）：关闭按钮的 `Sizable::with_size` 只吃 `Pixels`（见
///   [`CLOSE_BUTTON_SIZE_SPEC`]），其余度量直接写 `rems(P / 16.)`。
///
/// 返回元素**不含**「单项目时隐藏」的判断 —— 那是调用方的条件（`project-tab-bar.tsx:37`、
/// `project-tab-bar-model.ts:12-13`），本模块只负责画。
pub fn project_tabs(
    tabs: &[ProjectTab],
    active: Option<usize>,
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
    let surface = cx.theme().background;
    let border = cx.theme().border;
    let accent = cx.theme().accent;
    let hover_bg = accent.alpha(HOVER_BG_ALPHA);
    let foreground = cx.theme().foreground;
    let subtle_foreground = cx.theme().muted_foreground;
    let primary = cx.theme().primary;

    h_flex()
        .h_8()
        .flex_shrink_0()
        .items_center()
        .gap_1()
        .px_1p5()
        .bg(surface)
        .border_b_1()
        .border_color(border)
        .children(tabs.iter().enumerate().map(|(index, tab)| {
            let is_active = active == Some(index);
            let name = tab.name().clone();
            let tooltip = tab.name().clone();

            let activate = on_activate.clone();
            let close = on_close.clone();

            // 每个标签是一个 `relative` 包装：标签本体（可点，也是 hover group 的宿主）
            // + 绝对定位在右侧的关闭按钮。与 Windows 的 DOM 结构一致
            // （`project-tab-bar.tsx:52-108`：外层 `group relative`，内层 `<button>` 与关闭容器是兄弟）。
            div()
                .relative()
                .flex_shrink_0()
                .child({
                    let mut label = h_flex()
                        .id(("project-tab", index))
                        .h_7()
                        .min_w(rems(TAB_MIN_WIDTH_SPEC / 16.))
                        .max_w(rems(TAB_MAX_WIDTH_SPEC / 16.))
                        .items_center()
                        .gap_1p5()
                        .pl_2p5()
                        .pr_8()
                        .rounded(rems(TAB_RADIUS_SPEC / 16.))
                        .text_sm()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .cursor_pointer()
                        // 悬停分组：让关闭按钮在鼠标进入标签时显现，
                        // 对应 Windows 的 `group-hover`（`project-tab-bar.tsx:90`）。
                        // `group()` 会给本元素装 hitbox（`gpui-pre-0.3.6/src/elements/div.rs:2405`），
                        // 子元素的 `group_hover(CLOSE_GROUP, ..)` 靠它判断。
                        .group(CLOSE_GROUP)
                        .on_click(move |_, window, cx| {
                            // Windows 点已选中的标签是 no-op（`project-tab-bar.tsx:60`）。
                            if !is_active {
                                activate(index, window, cx);
                            }
                        })
                        .child(
                            // 真源：`ui-icons/idea/expui/nodes/folder.svg(+_dark)` —— 真机
                            // `FolderIcon`（`project-tab-bar.tsx:6,70`）。14 = `rems(14. / 16.)`
                            // （与原来的 `size_3p5()` 同值；14 不在 rem 档位上，所以自己换算）。
                            // 颜色由真源 SVG 自带（浅灰填充 + 灰描边），不再随选中态变 ——
                            // 真机同样是彩色图标，理由见模块文档的图标表。
                            idea_icon_svg_px(&idea::FOLDER_ICON, cx, rems(14. / 16.)),
                        )
                        .child(
                            // 文案容器：`min-w-0 truncate`（`project-tab-bar.tsx:77`）。
                            // `min_w_0` 是 flex 子项能被压缩的前提，否则 `text_ellipsis` 不会生效。
                            div()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .text_color(if is_active {
                                    foreground
                                } else {
                                    subtle_foreground
                                })
                                .child(name),
                        );

                    // 选中态 / 悬停态底色。`hover()` 的样式在 `compute_style` 里 refin 在基础样式
                    // 之上（`gpui-pre-0.3.6/src/elements/div.rs:3452-3469`）。
                    if is_active {
                        label = label.bg(accent);
                    } else {
                        label = label.hover(move |this| this.bg(hover_bg).text_color(foreground));
                    }

                    // 选中下划线：`absolute inset-x-1.5 bottom-0 h-0.5 rounded-t-sm bg-primary`
                    // （`project-tab-bar.tsx:79-82`）。
                    if is_active {
                        label = label.child(
                            div()
                                .absolute()
                                .bottom_0()
                                .left_1p5()
                                .right_1p5()
                                .h_0p5()
                                .rounded_t(rems(TAB_RADIUS_SPEC / 16.))
                                .bg(primary),
                        );
                    }

                    label
                })
                .child(
                    // 关闭按钮容器：`absolute inset-y-0 right-1 z-10 flex items-center`
                    // （`project-tab-bar.tsx:86-91`）。选中时常显（`opacity-100`），
                    // 未选中时 `opacity-0` + 悬停标签才显现。
                    h_flex()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .right_1()
                        // 源码是 `z-10`，但 gpui **没有 z-index**（元素按绘制顺序决定叠放，
                        // `gpui-pre-0.3.6/src/styled.rs` 里没有 `z_*` 方法）。关闭按钮是本行的
                        // 后一个 child，天然画在上层，所以这里直接省略。
                        .items_center()
                        .when(!is_active, |this| {
                            this.opacity(0.)
                                .group_hover(CLOSE_GROUP, |this| this.opacity(1.))
                        })
                        .child(
                            Button::new(("project-tab-close", index))
                                // Windows 的 `XIcon`（lucide `x`）就是 `icons/x.svg`。
                                .icon(IconName::X)
                                .ghost()
                                // `Sizable::with_size(impl Into<Size>)`；`Pixels` 经
                                // `From<Pixels> for Size` 变成 `Size::Size(24)`。`Size` 没有
                                // `From<Rems>`，所以这一处按当帧 rem 基准换算（见常量注释）。
                                .with_size(crate::rem_px(rem, CLOSE_BUTTON_SIZE_SPEC))
                                // 文案逐字取下 Windows 中文包：`titleProject.closeProject`
                                // = "关闭项目 {name}"（`windows/tauri/src/i18n/locale.ts:6188`）。
                                .tooltip(tr_args(
                                    "lithe.titleProject.closeProject",
                                    &[("name", tooltip.as_ref())],
                                ))
                                .on_click(move |_, window, cx| {
                                    close(index, window, cx);
                                }),
                        ),
                )
        }))
}
