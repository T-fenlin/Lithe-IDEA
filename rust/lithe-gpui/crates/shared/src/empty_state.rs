//! 「功能不可用 / 空态」的**共同 owner**：结构、度量与「关掉虚线」只在这里写一次。
//!
//! ## 真源
//!
//! macOS 的 `LitheUnavailableView`
//! （`macos/Sources/Lithe/Views/Components/LitheUnavailableView.swift:3-35`）：
//!
//! ```text
//! VStack(spacing: 8)
//!   ├─ Image(systemName:)  .font(size: 28, weight: .light)
//!   ├─ Text(title)         .font(size: 13, weight: .semibold)
//!   └─ description?        .font(size: 11.5) .opacity(0.72) .fixedSize(vertical)
//! .multilineTextAlignment(.center)
//! .padding(24)
//! .frame(maxWidth: .infinity, maxHeight: .infinity)
//! ```
//!
//! ## 为什么要有这个模块
//!
//! 此前有 **11 处 / 8 个文件**各自拼一遍 gpui-kit 的 `Empty`，而且每一处都重复写同一句
//! 「把 `Empty` 硬编码的虚线边框改成透明」（`gpui-component-0.7.0/src/empty.rs:74-75`），
//! 还各自决定图标尺寸（多为 `size_8()` = 32，真源 28）与标题档位（`text_sm()` 但**漏了字重**，
//! 真源是 semibold）—— 改一处不会带动其余十处。
//!
//! 放在 `shared` 的理由与 [`crate::icons`] 模块文档同一条：依赖方向是
//! `app → workbench → {editor, explorer, git, terminal} → shared`，feature 之间不能互相依赖，
//! 所以只有 `shared` 同时满足「被所有 feature 依赖」与「已经依赖 gpui-kit」。
//!
//! ## 两个取值必须说明（真源值 gpui 的档位表达不出来）
//!
//! - 真源标题 **13** 与 gpui 的档位 **12 / 14** 等距，这里取 **14**（`text_sm`）：不把已有
//!   11 个界面的标题改小；层级由**补上的 `font_semibold()`** 承担 —— 真源本来就是 semibold，
//!   此前每个调用点都漏了它，是这批里最实质的一处修正。
//! - 描述 **11.5** 取最近的 **12**（`text_xs`），透明度 **0.72** 照抄。
//! - 图标 **28**：尺寸无法由本函数代设（调用点给的是不同来源的元素 —— Lucide `Icon` 与
//!   `idea_icon_image` 两条路），所以导出 [`UNAVAILABLE_ICON_SIZE_SPEC`] 让调用点按它设，
//!   并把默认的 `size_8()` = 32 改掉。
//!
//! ⚠️ **迁移是分批的**：本模块先落地，`workbench` 的 5 处当批迁入；`editor`（批 5）、
//! `explorer`（批 4）、`git`（批 8）、`settings`（批 11/12）、`terminal`（批 9）各自在
//! 自己的批次里换成这里，避免一次性改 8 个文件把评审面摊开。

use gpui_kit::base::v_flex;
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
// `theme()` 来自 `ActiveTheme`、`font_semibold()` / `text_center()` 来自 `StyledExt`，两个 trait
// 都在 `gpui_kit::component` 下（与 `right_tool_window.rs` 同一取法）；`Styled` 本身在 crate 根。
use gpui_kit::component::{ActiveTheme as _, StyledExt as _};
use gpui_kit::{AnyElement, App, IntoElement, ParentElement as _, SharedString, Styled as _};

/// 空态图标的规格边长：真源 `LitheUnavailableView.swift:21` 的 `size: 28`。
///
/// 调用点按它设自己的图标（Lucide 走 `size_7()` = 28，`idea_icon_image` 走
/// `rems(UNAVAILABLE_ICON_SIZE_SPEC / 16.)`）。
pub const UNAVAILABLE_ICON_SIZE_SPEC: f32 = 28.;

/// 真源描述的透明度：`LitheUnavailableView.swift:27` 的 `.opacity(0.72)`。
pub const UNAVAILABLE_DESCRIPTION_OPACITY: f32 = 0.72;

/// 画一个「不可用 / 空」状态：真源 `LitheUnavailableView` 的结构与度量。
///
/// - `icon`：**已按 [`UNAVAILABLE_ICON_SIZE_SPEC`] 设好尺寸**的图标元素（来源由调用点决定）。
///   真源必给图标，但本仓已有两处调用点在无图标时也走同一条路径，所以这里收 `Option`。
/// - `title`：标题（真源 13 semibold → 见模块文档的档位取舍）；
/// - `description`：可选描述（真源 11.5 @ 0.72）；
/// - `extra`：可选的一块**额外内容**，渲染在空态下方（同一列）。真源组件没有这个槽，
///   但本仓已有一处调用点要在空态下再挂一行失败详情（`extensions_page::empty_state`）；
///   既然包裹层由本函数拥有，这个槽也归这里，免得调用点又各自拼一遍外层。
///
/// 返回的容器已经带 `w_full().flex_1().min_h_0()` —— 这是 11 处调用点此前各自重复的那层
/// 包裹（父级不是 flex 容器时 `Empty` 的 `flex_1` 不生效，见 `explorer_view.rs` 记过的坑），
/// 所以调用点直接 `.child(unavailable(..))` 即可。
pub fn unavailable(
    icon: Option<impl IntoElement>,
    title: impl Into<SharedString>,
    description: Option<SharedString>,
    extra: Option<AnyElement>,
    cx: &App,
) -> AnyElement {
    let mut header = EmptyHeader::new();
    if let Some(icon) = icon {
        header = header.media(EmptyMedia::new().child(icon));
    }
    header = header.title(
        EmptyTitle::new()
            .text_sm()
            .font_semibold()
            // `child` 收 `IntoElement`，所以先把 `impl Into<SharedString>` 落成 `SharedString`
            // 再交给它（`SharedString` 自己实现了 `IntoElement`）。
            .child(title.into()),
    );
    if let Some(description) = description {
        header = header.description(
            EmptyDescription::new()
                .text_xs()
                .opacity(UNAVAILABLE_DESCRIPTION_OPACITY)
                .child(description),
        );
    }

    v_flex()
        .w_full()
        .flex_1()
        .min_h_0()
        .child(
            Empty::new()
                // `Empty` 硬编码了 `.border_dashed()`（`gpui-component-0.7.0/src/empty.rs:74-75`），
                // 产品界面没有这圈虚线 —— 这一句此前在 11 处各写了一遍，现在只在这里写。
                .border_color(cx.theme().transparent)
                // 真源 `VStack(spacing: 8)`（`LitheUnavailableView.swift:19`）与 `padding(24)`
                // （`:33`）都落在 `Empty` 的根上。
                .gap_2()
                .p_6()
                .text_center()
                .header(header),
        )
        // `Option` 本身就是 `IntoIterator`，所以 `None` 时这一句不产生任何孩子。
        .children(extra)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 钉住 owner 的两条规格值，防止它们在别处被复制成第二份字面量。
    ///
    /// 纯常量断言：不开窗口、不碰主题、无后台任务。
    #[test]
    fn unavailable_specs_match_the_macos_source() {
        assert_eq!(UNAVAILABLE_ICON_SIZE_SPEC, 28.);
        assert_eq!(UNAVAILABLE_DESCRIPTION_OPACITY, 0.72);
    }
}
