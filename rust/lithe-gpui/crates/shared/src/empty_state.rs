use gpui::{AnyElement, App, IntoElement, ParentElement as _, SharedString, Styled as _};
use lithe_ui::feedback::EmptyState;
use lithe_ui::layout::v_stack;

pub const UNAVAILABLE_ICON_SIZE_SPEC: f32 = 28.;

/// 宿主那种「不可用 / 空目录」提示：一个图标、一句标题、可选的一行说明，`extra` 另起一块贴在下面。
///
/// 版面交给组件库的 [`EmptyState`]：槽位式组合，头槽位收任意元素（调用方自己定图标尺寸，
/// 见 [`UNAVAILABLE_ICON_SIZE_SPEC`]），标题与说明走库内的字号档位。原先这里自己拼
/// 上游的 `Empty`/`EmptyHeader`/`EmptyMedia`/`EmptyTitle`/`EmptyDescription`，
/// 2026-10-08 迁到组件库后由库决定字号与间距——**说明不再叠 `.opacity(0.72)`**，
/// 库的说明槽位本就取 `colors.fg_muted`。
///
/// `_cx` 现在没有读点（版面全在库里），留着是为了调用点签名不变；等 `workbench` 那一片
/// 迁完再连同参数一起收掉。
///
/// 另注：库里的 [`EmptyState`] 会做一次「升起」动画（上游 `Empty` 没有），动画按元素 id 去重，
/// 而这里 5 个调用点共用 `"shared-unavailable"` 一个 id —— 没改签名是因为签名一改要动 5 个
/// `workbench` 调用点（属下一片）。**同一窗口里两处同时渲染、且祖先元素 id 路径也相同时，
/// 它们的升起动画会共用一份状态**（可见后果只是同时升起；元素不是可交互元素，不涉及 id 撞车
/// 断言）。要彻底分开，得让调用点自己传 id。
pub fn unavailable(
    icon: Option<impl IntoElement>,
    title: impl Into<SharedString>,
    description: Option<SharedString>,
    extra: Option<AnyElement>,
    _cx: &App,
) -> AnyElement {
    let mut state = EmptyState::slots("shared-unavailable").title(title);
    if let Some(icon) = icon {
        state = state.media_element(icon);
    }
    if let Some(description) = description {
        state = state.body(description);
    }

    v_stack()
        .w_full()
        .flex_1()
        .min_h_0()
        .child(state)
        .children(extra)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_specs_match_the_macos_source() {
        assert_eq!(UNAVAILABLE_ICON_SIZE_SPEC, 28.);
    }
}
