//! 代码块（第 17 章 Editor）：只读代码展示，行号栏 + 等宽字体。
//! 语法高亮随 base 的 tree-sitter 集成（需上游 feature）另行接入。

use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window, div, rems};

use crate::ActiveTheme;

/// 只读代码块。输入是已格式化的代码文本；按 `\n` 切行展示。
#[derive(IntoElement)]
pub struct CodeBlock {
    id: ElementId,
    code: SharedString,
    style: gpui_kit::StyleRefinement,
}

impl CodeBlock {
    pub fn new(id: impl Into<ElementId>, code: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            code: code.into(),
            style: Default::default(),
        }
    }
}

impl Styled for CodeBlock {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for CodeBlock {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let lines: Vec<SharedString> = self.code.lines().map(SharedString::from).collect();
        let gutter_width = rems(0.5 * (lines.len().to_string().len() as f32 + 1.0));
        div()
            .id(self.id)
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .bg(theme.sunken)
            .overflow_hidden()
            .flex()
            .items_stretch()
            .refine_style(&self.style)
            .child(
                // 行号栏
                div()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .items_end()
                    .px(rems(0.375))
                    .py(rems(0.5))
                    .gap_0()
                    .bg(theme.sunken)
                    .border_r_1()
                    .border_color(theme.border)
                    .font_family(theme.mono_font_family.clone())
                    .text_xs()
                    .text_color(theme.subtle_foreground)
                    .children((1..=lines.len()).map(|no| {
                        div()
                            .w(gutter_width)
                            .text_align(gpui_kit::TextAlign::Right)
                            .child(no.to_string())
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .px(rems(0.5))
                    .py(rems(0.5))
                    .font_family(theme.mono_font_family.clone())
                    .text_xs()
                    .text_color(theme.foreground)
                    .children(lines.into_iter().map(|line| {
                        div().h(rems(1.0)).child(if line.is_empty() {
                            SharedString::from(" ")
                        } else {
                            line
                        })
                    })),
            )
    }
}
