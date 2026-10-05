//! 文本输入（第 6 章 Forms）。状态与编辑行为完全复用 gpui-base 的
//! `InputState`/`Input`；base 的 `Input` 是无样式的文本编辑面，本层画外框。
//! 多行 `Textarea` 同源（后续章节接入）。

use gpui_kit::base::StyledExt as _;
use gpui_kit::base::input::{Input as BaseInput, InputBaseState, InputMode};
use gpui_kit::prelude::*;
use gpui_kit::{App, Entity, IntoElement, RenderOnce, Styled, Window, div, rems};

use crate::ActiveTheme;

pub use gpui_kit::base::input::InputEvent;

/// 输入状态类型。base 的别名经私有模块再导出不可达，这里给出公开别名。
pub type InputState = InputBaseState<InputMode>;

/// 单行文本输入。状态由调用方持有：`Entity<InputState>`。
#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    disabled: bool,
    style: gpui_kit::StyleRefinement,
}

impl Input {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            state: state.clone(),
            disabled: false,
            style: Default::default(),
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for Input {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Input {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let border = if self.disabled {
            theme.border
        } else {
            theme.input
        };
        div()
            .h(rems(1.75))
            .px(rems(0.5))
            .rounded(theme.radius)
            .bg(theme.surface)
            .text_sm()
            .text_color(theme.foreground)
            .border_1()
            .border_color(border)
            .when(self.disabled, |el| {
                el.text_color(theme.disabled_foreground).bg(theme.sunken)
            })
            .refine_style(&self.style)
            .child(BaseInput::new(&self.state))
    }
}
