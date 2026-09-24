//! 编辑区的**表现层**：标签栏（`← →` 导航组 + 各文件的标签 + 关闭按钮）与正文 / 空状态。
//!
//! 从 `shell_probe/editor.rs` 原样拆出（逐字搬迁，只调整可见性与 import）。
//! 本文件只负责画与交互；读盘、图标与标签显示名在 `buffer.rs`。

use std::path::Path;

use crate::buffer::{Buffer, display_names, icon_for_file, read_body};
use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::input::{Editor, EditorState};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::tab::{Tab, TabBar, TabVariant};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, Sizable as _};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollWheelEvent, SharedString, Styled as _, Window, div, point,
    px, relative,
};
use lithe_gpui_shared::tr;

// ---------------------------------------------------------------------------
// 度量：一律用 gpui 的 rem-based helper，不再直接写 `px(...)`
// ---------------------------------------------------------------------------
//
// rem base = 主题字号 16px，所以 helper 后缀 `N` = `N × 4px`，与 Windows 规格逐像素相等：
//
// | 规格（Windows 真源） | 值 | 用到的 helper |
// | --- | --- | --- |
// | 标签栏高 `--lithe-tab-bar-height: 2.25rem`（`styles/theme.css:120-121`、`ui/tab-bar.tsx:248`） | 36 | `h_9()` |
// | 标签栏左右内边距 `--lithe-chrome-padding-inline`（`theme.css:132`） | 8 | `px_2()` |
// | 标签栏段间距 `--lithe-chrome-gap`（`theme.css:130`） | 4 | `mr_1()` |
// | 标签 `pl-2` / `pr-6`（`ui/tab-bar.tsx:260`） | 8 / 24 | `pl_2()` / `pr_6()` |
// | 标签图标↔文字间距 `--lithe-chrome-gap-loose`（`theme.css:131`） | 6 | `gap_1p5()` |
// | 标签图标槽 `size-3`（`tab-bar-item.tsx:183`） | 12 | `size_3()` |
// | 导航组 `gap-0.5`（`tab-bar.tsx:633`） | 2 | `gap_0p5()` |
// | 关闭按钮 `absolute right-1`（`tab-bar-item.tsx:164-166`） | 4 | `right_1()` |
// | 脏标记圆点 `size-2`（`tab-bar-item.tsx:284-291`） | 8 | `size_2()` |
// | 空状态 `px-6 py-8` / `gap-3`（`empty-editor-state.tsx:15-16`） | 24 / 32 / 12 | `px_6()` / `py_8()` / `gap_3()` |
// | 空状态内容块 `max-w-md`（`empty-editor-state.tsx:16`） | 448 | `max_w_112()` |
// | 图标块 `size-12` / 主图标 `size-10` / 放大镜 `size-5`（`empty-editor-state.tsx:17-22`） | 48 / 40 / 20 | `size_12()` / `size_10()` / `size_5()` |
//
// 字号：`--ui-text-base` / `--ui-text-sm` 在这里都是 **13px**（`theme.css:116-117`），不在 gpui 的
// 档位（`text_xs()`=12 / `text_sm()`=14）上，按《编码指南》用 **`text_sm()`（14px）**——
// 13 → 14 是经维护者确认的**有意**视觉改动。

/// 单个标签宽度上限 200px：`--lithe-tab-max-width`（12.5rem，`windows/tauri/src/styles/theme.css:123`，
/// 用在 `windows/tauri/src/ui/tab-bar.tsx:260`）。**这是标签文字能截断的前提**。
///
/// ⚠️ **保留 `px(...)`**：200 不在 gpui 的固定 rem 档位上（档位里 48 → 192、56 → 224，
/// `gpui-pre-macros-0.3.6/src/styles.rs:1039-1047`），没有 `max_w_50()`。
const TAB_MAX_WIDTH: f32 = 200.;

/// 滚轮增量换算用的行高兜底值：真机默认行高就是 **20**
/// （`windows/tauri/src/features/editor/config/constants.ts:5` 的 `DEFAULT_LINE_HEIGHT: 20`；
/// 算法是 `ceil(fontSize × 1.4)`，`windows/tauri/src/features/editor/utils/lines.ts:8-15`）。
/// 编辑器还没完成首次布局时 `line_height()` 是 `None`，用这个值兜底，
/// 否则 `ScrollDelta::Lines` 换算出 0，整段滚不动。
///
/// 它是滚轮增量换算里的 `Pixels`（与 `line_height()` 的返回值同类型做算术：
/// `event.delta.pixel_delta(line_height)`、`if delta == px(0.)`），不是布局样式槽，
/// 套不了 `Styled` 的 `line_height` 系列 helper，所以保留 `px(...)`。
const FALLBACK_LINE_HEIGHT: f32 = 20.;

/// 编辑区视图：标签栏 + 正文（正文没有活动 buffer 时是空状态）。
pub struct EditorPane {
    /// 打开的 buffer，顺序就是标签栏里的顺序。
    buffers: Vec<Buffer>,
    /// 活动 buffer 在 [`Self::buffers`] 里的下标；`None` = 没有活动 buffer → 空状态。
    active: Option<usize>,
}

impl EditorPane {
    /// 建一个没有打开任何文件的编辑区。
    ///
    /// 真机启动时编辑区就是空状态（标签栏在、正文是空状态），
    /// 打开动作由用户触发（`panes/components/pane-container.tsx:1100`）。
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            buffers: Vec::new(),
            active: None,
        }
    }

    /// 打开一个文件：读盘、判定类型、更新标签栏与正文。
    ///
    /// **同一路径重复打开只切换活动标签、不再读盘**（沿用上一轮已验证实现的约定，
    /// git HEAD `panels.rs::open_document`）：正文已经在这个 [`Buffer`] 的
    /// `EditorState` 里，重读会白白丢掉撤销栈与光标。
    /// 注意"同路径"是按传入的 `Path` 字面比较；调用方要保证同一个文件每次传同一种写法
    /// （绝对路径就一直绝对路径）。
    ///
    /// 关闭标签会把该 buffer 的 `EditorState`（以及缓存正文）一起丢掉 —— 真机同样会释放
    /// 对应 model，所以"重复打开不读盘"只对**仍然打开着**的标签成立。
    pub fn open(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.to_path_buf();

        if let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) {
            self.active = Some(index);
            cx.notify();
            return;
        }

        let name: SharedString = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
            .into();
        let body = read_body(&path, &name);

        // 一个标签一个 `EditorState`：先建状态再灌正文，然后才入列。
        let editor = cx.new(|cx| EditorState::new(window, cx));
        editor.update(cx, |state, cx| state.set_value(body, window, cx));

        // 实参从左到右求值，图标要在 `name` 被移进构造函数之前算好。
        let icon = icon_for_file(&name);
        self.buffers.push(Buffer::new(path, name, icon, editor));
        self.active = Some(self.buffers.len() - 1);
        cx.notify();
    }

    /// 关闭一个标签。
    ///
    /// 关掉最后一个标签后编辑区回到空状态（`windows/tauri/src/features/panes/components/pane-container.tsx:1100`）；
    /// 关掉活动标签时顺位接上它的邻居（真机就是"关掉后激活相邻标签"）。
    fn close(&mut self, index: usize) {
        if index >= self.buffers.len() {
            return;
        }

        // 被关掉的标签之后的标签整体左移一位，所以旧下标大于 index 的要减一。
        let shift = |active: usize| if active > index { active - 1 } else { active };
        self.buffers.remove(index);

        self.active = if self.buffers.is_empty() {
            None
        } else {
            Some(
                self.active
                    .map(shift)
                    .unwrap_or(index)
                    .min(self.buffers.len() - 1),
            )
        };
    }

    /// 标签栏：左侧 `← →` 导航组 + 各文件的标签。
    ///
    /// 组件的选型与为什么这么用：
    ///
    /// - `TabVariant::Underline` 是 gpui-kit 里唯一"活动标签 = 透明底 + 底边主色条"的变体
    ///   （`tab/tab.rs:253-262`：`bg` 透明、`border_b` 2px `primary`），与 Windows 的
    ///   IntelliJ 风格一致（`windows/tauri/src/ui/tab-bar.tsx:204-209`：`bg-transparent` +
    ///   `before:h-[3px] before:bg-primary`）；
    /// - Underline 变体的标签条自带 1px 下边框（`tab/tab_bar.rs:501-514`，
    ///   `border_b_1()` + `theme.border`），正好是 Windows 的 `border-b border-border`；
    /// - Underline 变体的条底色是**透明**、外层 padding 是 0
    ///   （`tab/tab_bar.rs:393-403`），所以这里显式补 `.bg(theme.tab_bar)` 与
    ///   `.px_2()`（8px），把 Windows 的 `bg-tab-bar` + `px-(--lithe-chrome-padding-inline)` 找回来；
    /// - `.h_9()`（36px）是**必须**的：标签条自身没有高度，没有标签时会被压成 0；
    ///   真机里没有活动 buffer 时标签栏也照常占着 36px（`windows/tauri/src/features/panes/components/pane-container.tsx:1094-1100`）。
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let names = display_names(&self.buffers);

        let mut bar = TabBar::new("editor-tab-bar")
            .with_variant(TabVariant::Underline)
            .bg(cx.theme().tab_bar)
            .h_9()
            .px_2()
            // `max_width` 让**标签文字**在空间不够时让位（图标与关闭按钮保持原尺寸），
            // 也就是真机那种 `OrderChargeService.j…` 的截断。
            .max_width(TAB_MAX_WIDTH)
            .prefix(Self::render_nav_group())
            .on_click(cx.listener(|pane, index: &usize, _window, cx| {
                // `TabBar::on_click` 给的是被点标签的下标
                // （`tab/tab_bar.rs:168-177`），切换活动 buffer 就是切标签。
                if *index < pane.buffers.len() {
                    pane.active = Some(*index);
                    cx.notify();
                }
            }));

        for (index, (buffer, name)) in self.buffers.iter().zip(names).enumerate() {
            bar = bar.child(self.render_tab(index, buffer, name, cx));
        }

        if let Some(active) = self.active {
            bar = bar.selected_index(active);
        }

        bar.into_any_element()
    }

    /// 标签栏左侧的后退/前进按钮组。
    ///
    /// 真机是一个 `h-8` 的行（`windows/tauri/src/features/tabs/components/tab-bar.tsx:633-660`），
    /// 与后面的标签区之间有标签栏自己的 4px gap；`TabBar` 的 `Underline` 变体不给
    /// 外层容器设 gap，所以这里用右外边距补上同样的 4px。
    fn render_nav_group() -> impl IntoElement {
        h_flex()
            .items_center()
            .gap_0p5()
            .mr_1()
            // 文案取中文 i18n 原文：tooltip = `tabs.goBackShort` / `tabs.goForwardShort`
            // （"后退" / "前进"，`windows/tauri/src/i18n/locale.ts:7854-7855`），
            // 无障碍名 = `tabs.goBack` / `tabs.goForward`
            // （"后退到上一个位置" / "前进到下一个位置"，`windows/tauri/src/i18n/locale.ts:7850-7851`）。
            .child(Self::nav_button(
                "editor-nav-back",
                IconName::ArrowLeft,
                tr("lithe.tabs.goBackShort"),
                tr("lithe.tabs.goBack"),
            ))
            .child(Self::nav_button(
                "editor-nav-forward",
                IconName::ArrowRight,
                tr("lithe.tabs.goForwardShort"),
                tr("lithe.tabs.goForward"),
            ))
    }

    /// 单个导航按钮。
    ///
    /// 真机：`variant="ghost" size="icon-xs"`、`disabled={!canGoBack}`、
    /// tooltip `tooltipSide="bottom"`（`windows/tauri/src/features/tabs/components/tab-bar.tsx:634-659`）。
    ///
    /// ⚠️ **尺寸陷阱**：真机的 `icon-xs` 是 **24×24**（`windows/tauri/src/ui/button.tsx:27`
    /// 的 `icon-xs size-6`），而 gpui-kit 的 `Sizable::xsmall()` 给图标按钮是 **20×20**、
    /// `small()` 才是 24×24（`button/button.rs:618-623`）。这里对齐的是**尺寸值**，
    /// 所以用 `.small()`，不要被变体名带偏。
    ///
    /// ⚠️ **本轮只有外观 + 禁用态，没有历史栈**：探针没有 jump list，
    /// 所以两个按钮恒为 `disabled(true)`（禁用态 ghost = 灰图标，`button/button.rs:1273-1306`），
    /// 而不是"点了没反应"的假按钮。接上历史栈后改成 `disabled(!can_go_back)`
    /// 并在 `on_click` 里跳转即可。
    fn nav_button(
        id: &'static str,
        icon: IconName,
        tooltip: SharedString,
        label: SharedString,
    ) -> Button {
        Button::new(id)
            .icon(icon)
            .ghost()
            .small()
            .disabled(true)
            .tab_stop(false)
            .tooltip(tooltip)
            .accessibility_label(label)
    }

    /// 一个标签：文件类型图标 + 显示名（+ 未保存圆点）+ 关闭按钮。
    ///
    /// ⚠️ **图标 + 文件名必须走 `Tab::child(...)`**：`Tab::icon()` 只渲染图标，
    /// label 与 children 在图标分支里被整个丢掉（`tab/tab.rs:719-744`，
    /// 图标分支没有 `.children(self.children)`）。
    ///
    /// 颜色照 Windows（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:276`：活动 `--foreground`、非活动
    /// `--subtle-foreground`；`:250`：图标恒为 `--subtle-foreground`）。
    /// 这里必须**自己给文字设色**：`TabBar` 会把标签的 `text_color` 换成自己的
    /// `tab_foreground`/`tab_active_foreground`（`tab/tab.rs:253-264`），
    /// 而 Windows 的非活动标签要的是更暗的 `subtle-foreground`。
    ///
    /// 关闭按钮用**绝对定位**放在标签里，不走 `Tab::suffix`：
    /// suffix 是 flex 项，会算进标签宽度，切换标签时标签宽度会跳；
    /// 绝对定位后宽度只由 `pr(24px)` 决定，与真机一致（`right-1` + `pr-6`）。
    fn render_tab(
        &self,
        index: usize,
        buffer: &Buffer,
        name: SharedString,
        cx: &mut Context<Self>,
    ) -> Tab {
        let is_active = self.active == Some(index);
        let label_color = if is_active {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };

        // 无障碍名照真机拼「名称 +（未保存）」（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:137-141`、
        // `windows/tauri/src/i18n/locale.ts:7860` 的 `tabs.ariaUnsavedSuffix`）；
        // 固定/预览后缀本轮没有这两种状态。必须在 `name` 被移进正文之前算好。
        let aria_label = if buffer.is_dirty {
            SharedString::from(format!("{name}（未保存）"))
        } else {
            name.clone()
        };

        // 未保存圆点：8×8、`rounded-full`、`primary`（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:284-291`）。
        // `is_dirty` 本轮恒为 false，所以画不出来；位置留在这里，
        // 接上编辑事件后把开关改成真实值即可。
        let dirty_dot = buffer.is_dirty.then(|| {
            div()
                .flex_shrink_0()
                .size_2()
                .rounded_full()
                .bg(cx.theme().primary)
        });

        // 关闭按钮：只有活动标签显示。真机默认设置是
        // `tabCloseButtonVisibility: "active"`（`windows/tauri/src/features/settings/config/default-settings.ts:96`），
        // 规则见 `windows/tauri/src/features/settings/lib/ui-preferences.ts:13-19`
        // （固定标签恒显 / `always` 全显 / `active` 只有活动标签 / 否则悬停才显）。
        // 悬停那一档没做：`TabBar` 不暴露每个标签的悬停状态，见报告的"未能实现"一节。
        //
        // 绝对定位而不是 `Tab::suffix`：suffix 是 flex 项，会算进标签宽度，
        // 切换标签时标签宽度会跳；绝对定位后宽度只由正文常驻的 `pr(24px)` 决定，
        // 与真机一致（关掉按钮绝对定位在 `right-1` 上，见 `windows/tauri/src/features/tabs/components/tab-bar-item.tsx:164-166`）。
        let close = is_active.then(|| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right_1()
                .flex()
                .items_center()
                .child(Self::close_button(index, cx))
        });

        let content = h_flex()
            .items_center()
            .pl_2()
            .pr_6()
            .gap_1p5()
            // 没有 `min_w_0` 的话 flex 项的最小尺寸会按内容算，文字截断不了。
            .min_w_0()
            .child(
                // 全量目录的 `IconName` 是 `Copy`（`gpui-kit-assets-0.6.6/build.rs:52-53`
                // 的 `#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, IntoElement)]`），
                // 从借用里直接取出即可。
                Icon::new(buffer.icon)
                    .size_3()
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(label_color)
                    .child(name),
            )
            .children(dirty_dot)
            .children(close);

        Tab::new().aria_label(aria_label).child(content)
    }

    /// 标签上的关闭按钮。
    ///
    /// 真机：ghost 图标按钮 `icon-xs`（24×24），tooltip `tabs.close`（"关闭"，
    /// `windows/tauri/src/i18n/locale.ts:7845`），点击关闭该标签
    /// （`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:150-180`）。
    /// gpui-kit 的 `Button` 点击时会 `stop_propagation`（`button/button.rs:797-808`），
    /// 所以点关闭不会顺带把标签激活。
    /// 尺寸用 `.small()` 的理由（gpui-kit 的 `small()` = 24×24、`xsmall()` = 20×20）
    /// 见 [`TAB_CLOSE_INSET`] 的注释。
    fn close_button(index: usize, cx: &mut Context<Self>) -> Button {
        Button::new(format!("editor-tab-close-{index}"))
            // Windows 的关闭字形是 lucide `x`，`icons/x.svg` 在全量目录里确有该字形。
            .icon(IconName::X)
            .ghost()
            .small()
            .tab_stop(false)
            .tooltip(tr("lithe.tabs.close"))
            .accessibility_label(tr("lithe.tabs.close"))
            .on_click(cx.listener(move |pane, _event, _window, cx| {
                pane.close(index);
                cx.notify();
            }))
    }

    /// 正文区：有活动 buffer 时是代码编辑器，没有时是空状态。
    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(buffer) = self.active.and_then(|index| self.buffers.get(index)) else {
            return Self::render_empty_state(cx);
        };
        // 渲染闭包要 `'static`，先把活动编辑器的句柄取出来（`&Buffer` 借用到此结束）。
        // 两份：滚轮闭包会把它 move 走，正文还要再渲染一次同一个编辑器。
        let editor = buffer.editor.clone();
        let scroll_editor = editor.clone();

        div()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            // ⚠️ **滚轮必须自己接**：编辑器元素的样式只有 `position: absolute` + `100%`，
            // **没有** `overflow: scroll`（`gpui-base-0.6.6/src/input/base/element.rs:202-213`），
            // 而 gpui 只把滚轮交给"命中元素样式里带 `Overflow::Scroll`"的那个
            // （`gpui-pre-0.3.6/src/elements/div.rs:3332-3370`）。
            // 做法是在包裹层接，把增量交给编辑器自己的滚动偏移
            // （`InputBaseState::{scroll_offset, set_scroll_offset}`，
            // `gpui-base-0.6.6/src/input/base/state.rs:2806-2817`；后者会自己 clamp、
            // 下一帧生效）。
            .on_scroll_wheel(
                cx.listener(move |_pane, event: &ScrollWheelEvent, _window, cx| {
                    let line_height = scroll_editor
                        .read(cx)
                        .line_height()
                        // 兜底行高 20：滚轮增量换算里的 `Pixels`（不是样式槽），见常量注释。
                        .unwrap_or(px(FALLBACK_LINE_HEIGHT));
                    let delta = event.delta.pixel_delta(line_height).y;
                    if delta == px(0.) {
                        return;
                    }
                    let current = scroll_editor.read(cx).scroll_offset();
                    scroll_editor.update(cx, |state, cx| {
                        state.set_scroll_offset(point(current.x, current.y + delta), cx);
                    });
                }),
            )
            // ⚠️ **多行编辑器必须显式给高度**：多行走 `.h_auto()`，高度 = **内容高度**，
            // 而内容高度来自 `LayoutMode::CodeEditor { rows }`，`rows` 默认只有 2
            // （`gpui-component-0.6.6/src/input/input.rs:706-709`、
            // `gpui-base-0.6.6/src/input/base/mode.rs:83-85`）—— 不给高度时编辑区只有两行高、
            // 下面整片空白。`relative(1.)` 让它填满外层这一格
            // （等价于组件自带的 `Input::full_height()`，`input.rs:250-252`）。
            //
            // `.bordered(false)`：真机的编辑面是**无边框**的纯表面
            // （`windows/tauri/src/features/editor/components/code-editor.tsx:650,731`：
            // `absolute inset-0 bg-background`，没有 border），而 `Editor` 默认
            // `bordered: true`（`gpui-component-0.6.6/src/input/editor.rs:46`）会画出
            // 输入框那种 1px `theme.input` 描边（`input/input.rs:713-715`）。
            // 焦点环不用管：`Editor` 已经把 `focus_bordered(false)` 写死了
            // （`input/editor.rs:146`）。
            .child(Editor::new(&editor).h(relative(1.)).bordered(false))
            .into_any_element()
    }

    /// 没有活动 buffer 时的空状态。
    ///
    /// 结构照 `windows/tauri/src/features/panes/components/empty-editor-state.tsx`：
    /// 居中一列 = 图标块（48×48 的 `FileText` 40×40 + 右下角 20×20 放大镜）
    /// + 标题 + 说明（`:15-30`）。文案取**中文 i18n 原文，逐字**：
    ///
    /// - 标题 `workbench.emptyEditorTitle` = "选择文件以查看"（`windows/tauri/src/i18n/locale.ts:6150`）；
    /// - 说明 `workbench.emptyEditorDescription` = "外部工具产生的更改会自动显示。"
    ///   （`windows/tauri/src/i18n/locale.ts:6151`）；
    /// - 右键菜单只有一项禁用项 `ui.noActionsHere` = "此处无任何内容"
    ///   （`windows/tauri/src/i18n/locale.ts:4630`；空态菜单在
    ///   `windows/tauri/src/features/panes/components/empty-editor-state.tsx:32-34`）。
    fn render_empty_state(cx: &App) -> AnyElement {
        // 外层用 `v_flex` 而不是裸 `div`：`Empty` 自己带 `flex_1`，
        // 但"flex 项"只在 flex 容器里才成立 —— 裸 div 里它会塌成内容高度，
        // 垂直居中就没了。
        v_flex()
            .flex_1()
            .min_h_0()
            .context_menu(|menu, _window, _cx| {
                menu.item(PopupMenuItem::new(tr("lithe.ui.noActionsHere")).disabled(true))
            })
            .child(
                Empty::new()
                    // ⚠️ 真机空状态**没有边框**；`Empty` 的 render 里硬编码了
                    // `.border_dashed().border_color(cx.theme().border)`
                    // （`gpui-component-0.6.6/src/empty.rs:74-75`），
                    // 所以把边框色改成透明来关掉那圈虚线。
                    .border_color(cx.theme().transparent)
                    .gap_3()
                    .px_6()
                    .py_8()
                    .header(
                        EmptyHeader::new()
                            .max_w_112()
                            .gap_3()
                            .media(
                                EmptyMedia::new().child(
                                    div()
                                        .relative()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .size_12()
                                        .child(
                                            Icon::new(IconName::FileText)
                                                .size_10()
                                                .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(
                                            Icon::new(IconName::Search)
                                                .absolute()
                                                .right_0()
                                                .bottom_0()
                                                .size_5()
                                                .text_color(cx.theme().muted_foreground),
                                        ),
                                ),
                            )
                            // 字号：Windows 的 `ui-text-base` / `ui-text-sm` 都是 13px
                            // （`windows/tauri/src/styles/theme.css:116-117`），不在 gpui 的档位上，
                            // 按《编码指南》用 `text_sm()`（14px，经维护者确认的有意改动；
                            // 正好等于组件默认值，写出来是为了标明"这就是规格值"）。
                            // 颜色用组件默认：标题 `foreground`、说明 `muted_foreground`（`empty.rs:264-324`）。
                            .title(
                                EmptyTitle::new()
                                    .text_sm()
                                    .child(tr("lithe.workbench.emptyEditorTitle")),
                            )
                            .description(
                                EmptyDescription::new()
                                    .text_sm()
                                    .child(tr("lithe.workbench.emptyEditorDescription")),
                            ),
                    ),
            )
            .into_any_element()
    }
}

impl Render for EditorPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 两层：标签栏在上，正文占剩下的高度。容器要 `min_h_0` + `overflow_hidden`，
        // 否则正文里的编辑器会把整个视图撑出父容器。
        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .child(self.render_tab_bar(cx))
            .child(self.render_body(cx))
    }
}
