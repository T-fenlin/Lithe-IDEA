//! 设置界面的两个行/分组零件：`SettingsGroup` 与 `SettingsRow`。
//!
//! ## 真源
//!
//! 逐值照 `windows/tauri/src/features/settings/components/macos-settings-panels.tsx:40-73`
//! （真实对话框用的那一份，**不是** `components/settings-section.tsx` 里那份无边框的
//! `Section`/`SettingRow`；`07-settings-ui.md` §1.5 最后一段明确建议用带外框的这一份）：
//!
//! | 元素 | Windows 类名 | 值 | 这里怎么落 |
//! | --- | --- | --- | --- |
//! | 分组外框 | `rounded-md border border-border bg-surface/35` | 圆角 6.4 / 1px / 半透明 surface | `radius()` + `border_1()` + `popover.opacity(0.35)` |
//! | 分组标题 | `border-b px-3 py-2 ui-text-sm font-medium text-subtle-foreground` | 12/8 内边距 | `px_3()` + `py_2()` + `text_sm()` |
//! | 分组内容 | `flex flex-col gap-3 p-3` | 12 间隔 / 12 内边距 | `gap_3()` + `p_3()` |
//! | 行 | `min-h-8 items-center gap-4` | 最小高 32 / 16 间隔 | `min_h_8()` + `gap_4()` |
//! | 行标签 | `ui-text-sm text-foreground` | 14px | `text_sm()` |
//! | 行描述 | `mt-1 ui-text-caption text-subtle-foreground` | 12–13px | `text_xs()` + `muted_foreground` |
//! | 控件宽度 | `w-36`(默认) / `w-28`(数字) | 144 / 112 | [`ControlWidth`]（`rems(P / 16.)`） |
//!
//! 颜色全部走 `cx.theme()`：Windows 的 `bg-surface` 对应 gpui 的语义 surface 角色，
//! 也就是 `Theme::semantic_tokens()` 映射的 `popover`（`gpui-component-0.6.6/src/theme/mod.rs:418`）；
//! `text-subtle-foreground` 对应 `muted_foreground`（弱化文字角色）。
//! 圆角用主题的 `radius()`（设计指南「所有应用拥有的控件圆角都应来自主题」）。
//!
//! ## 行可点即激活主控件
//!
//! `SettingsRow` 在 Windows 里"整行可点即激活主控件"（`settings-section.tsx:134-186`）。
//! 这里只对**开关行**实现（[`settings_row`] 的 `on_activate`）：开关本身会吃掉点击并
//! 自己切换，所以行级回调只在点标签/空白处触发，两者不会重复触发。
//! **下拉行与数字行不做行级转发** —— gpui 的 `Button::dropdown_menu` 没有"以编程方式展开"的
//! 入口，而把点击转发进状态化控件（`InputState` 聚焦）需要另一套机制；这两行的控件自身可点、
//! 可键盘到达，所以「要点得动」这条满足。这是有意的取舍，不是遗漏。

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{
    AnyElement, App, ClickEvent, Div, ElementId, InteractiveElement as _, IntoElement,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, Window, div,
    prelude::FluentBuilder as _, rems,
};

/// 行内控件的宽度档。真源 `windows/tauri/src/features/settings/components/settings-section.tsx:40-49`。
///
/// 两档都**不在 gpui 的 rem 档位表**上（档位表是
/// `gpui-pre-macros-0.3.6/src/styles.rs` 的 `box_style_suffixes()`，只有 32→128px 那几档），
/// 所以用 helper 底层的 `rems(P / 16.)` 表达同一个长度：**1rem = 16px**（主题的 rem 基准，
/// `Root::render` 每帧写入，`gpui-component-0.6.6/src/root.rs:582`），写成 `/ 4.` 就错了。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlWidth {
    /// `w-36` = 144px。下拉与文本控件的默认档。
    Default,
    /// `w-28` = 112px。数字档。
    Number,
}

impl ControlWidth {
    /// 长度表达式（rem 值本身）。
    pub fn rems(self) -> gpui_kit::Rems {
        match self {
            // 144px = 9rem；112px = 7rem。
            Self::Default => rems(144. / 16.),
            Self::Number => rems(112. / 16.),
        }
    }

    /// 交给 `Styled::w` 的 `Length`（布局期按窗口 rem 基准求值）。
    pub fn length(self) -> gpui_kit::Length {
        self.rems().into()
    }
}

/// 行级激活回调（"点整行 = 激活主控件"）。
pub type RowActivation = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// 一个带外框的设置分组：标题 + 若干行。
pub fn settings_group(title: SharedString, rows: Vec<AnyElement>, cx: &App) -> impl IntoElement {
    v_flex()
        .w_full()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(cx.theme().border)
        // `bg-surface/35`：surface 角色 = `popover`，35% 透明。
        .bg(cx.theme().popover.opacity(0.35))
        .overflow_hidden()
        .child(
            h_flex()
                .w_full()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(cx.theme().border)
                .text_sm()
                .font_weight(gpui_kit::FontWeight::MEDIUM)
                .text_color(cx.theme().muted_foreground)
                .child(title),
        )
        .child(v_flex().w_full().gap_3().p_3().children(rows))
}

/// 一行设置：左「标签（+ 可选描述）」、右「控件」。
///
/// `on_activate` 见模块文档的「行可点即激活主控件」。
/// 返回 `AnyElement`（而不是 `impl IntoElement`）：调用方要把若干行收进 `Vec<AnyElement>`
/// 再交给 [`settings_group`]，类型擦除在这里做一次比每个调用点都 `.into_any_element()` 清楚。
pub fn settings_row(
    id: impl Into<ElementId>,
    label: SharedString,
    description: Option<SharedString>,
    control: AnyElement,
    on_activate: Option<RowActivation>,
    cx: &App,
) -> AnyElement {
    settings_row_with_note(id, label, description, None, control, on_activate, cx)
}

/// 与 [`settings_row`] 相同，但在标签与描述之下多一行**补充说明**（外观来源标注）。
///
/// 它存在的理由只有一个：外观键可以被工作区覆盖（`.lithe/settings.json` /
/// `settings.local.json`），而用户必须能看出"这一刻这个值来自哪一层"，
/// 并有一个动作把它改回全局（见 `workspace.rs` 的三条缓解措施）。
/// 没有覆盖时调用方传 `None`，行与 [`settings_row`] 逐像素相同（默认静默）。
pub fn settings_row_with_note(
    id: impl Into<ElementId>,
    label: SharedString,
    description: Option<SharedString>,
    note: Option<AnyElement>,
    control: AnyElement,
    on_activate: Option<RowActivation>,
    cx: &App,
) -> AnyElement {
    h_flex()
        // 行本身要有 `id` 才能挂 `on_click`（`InteractiveElement` / `StatefulInteractiveElement`
        // 的约束，与 `lithe-gpui/crates/git/src/log_view.rs:1050` 的同一处结论）。
        .id(id)
        .w_full()
        .min_h_8()
        .items_center()
        .gap_4()
        .when_some(on_activate, |this, on_activate| {
            this.on_click(move |event, window, cx| on_activate(event, window, cx))
        })
        .child(
            v_flex()
                .min_w_0()
                .flex_1()
                .gap_1()
                .child(div().text_sm().text_color(cx.theme().foreground).child(label))
                .children(description.map(|description| {
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(description)
                }))
                .children(note),
        )
        .child(div().flex_shrink_0().child(control))
        .into_any_element()
}

/// 外观来源标注的容器：一句来源说明 + 一个「改回我的全局外观」按钮。
///
/// 配色走主题的 `muted_foreground`（弱化文字角色，与行描述同一档），按钮用 ghost 小按钮：
/// 它是"纠正一个意外状态"的动作，不该比主控件更抢眼。
pub fn appearance_source_note(
    text: SharedString,
    revert: AnyElement,
    cx: &App,
) -> AnyElement {
    h_flex()
        .w_full()
        .min_w_0()
        .items_center()
        .gap_2()
        .child(
            div()
                .min_w_0()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(text),
        )
        .child(div().flex_shrink_0().child(revert))
        .into_any_element()
}

/// 页面标题：`mb-5 text-xl font-semibold`（`settings-dialog.tsx:152`）。
pub fn page_title(title: SharedString) -> impl IntoElement {
    div()
        .w_full()
        .mb_5()
        .text_xl()
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .child(title)
}

/// 页面内容区：分组之间 `gap-4`（`macos-settings-panels.tsx:136`）。
pub fn page_stack() -> Div {
    v_flex().w_full().gap_4()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 宽度档必须是 144 / 112 **px**（写成 `/ 4.` 会得到 36 / 28rem，肉眼立刻错）。
    #[test]
    fn control_widths_are_the_windows_values() {
        let to_px = |width: ControlWidth, rem: f32| {
            gpui_kit::AbsoluteLength::from(width.rems()).to_pixels(gpui_kit::px(rem))
        };

        assert_eq!(to_px(ControlWidth::Default, 16.), gpui_kit::px(144.));
        assert_eq!(to_px(ControlWidth::Number, 16.), gpui_kit::px(112.));
        // rem 基准翻倍时长度一起翻倍（这就是用 rem 而不是 px 的意义）。
        assert_eq!(to_px(ControlWidth::Default, 32.), gpui_kit::px(288.));
    }
}
