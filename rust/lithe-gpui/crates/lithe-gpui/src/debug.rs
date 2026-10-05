//! 调试组件（第 20 章）：调用栈帧列表与变量列表。数据由调试引擎方提供。

use std::rc::Rc;

use gpui_kit::TestSupportExt as _;
use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window, div, rems};

use crate::ActiveTheme;

/// 一个栈帧。`location` 形如 "App.java:42"。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackFrame {
    pub name: SharedString,
    pub location: SharedString,
}

impl StackFrame {
    pub fn new(name: impl Into<SharedString>, location: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            location: location.into(),
        }
    }
}

/// 调用栈列表。`on_select` 报告帧下标；当前帧由调用方持有。
#[derive(IntoElement)]
pub struct FrameList {
    id: ElementId,
    frames: Vec<StackFrame>,
    selected_index: Option<usize>,
    on_select: Option<Rc<dyn Fn(usize, &mut Window, &mut App)>>,
    style: gpui_kit::StyleRefinement,
}

impl FrameList {
    pub fn new(id: impl Into<ElementId>, frames: Vec<StackFrame>) -> Self {
        Self {
            id: id.into(),
            frames,
            selected_index: None,
            on_select: None,
            style: Default::default(),
        }
    }

    pub fn selected_index(mut self, index: Option<usize>) -> Self {
        self.selected_index = index;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl Styled for FrameList {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for FrameList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let selected_bg = theme.accent;
        let hover_bg = theme.hover;
        div()
            .id(self.id)
            .test_support()
            .flex()
            .flex_col()
            .refine_style(&self.style)
            .children(self.frames.into_iter().enumerate().map(|(ix, frame)| {
                let selected = self.selected_index == Some(ix);
                let on_select = self.on_select.clone();
                div()
                    .id(("frame", ix))
                    .test_support()
                    .px(rems(0.75))
                    .py(rems(0.375))
                    .flex()
                    .items_baseline()
                    .gap(rems(0.5))
                    .when(selected, |el| el.bg(selected_bg))
                    .when(!selected, |el| el.hover(move |style| style.bg(hover_bg)))
                    .cursor_pointer()
                    .text_sm()
                    .on_click(move |_, window, cx| {
                        if let Some(handler) = on_select.as_ref() {
                            handler(ix, window, cx);
                        }
                    })
                    .child(div().text_color(theme.foreground).child(frame.name))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .font_family(theme.mono_font_family.clone())
                            .child(frame.location),
                    )
            }))
    }
}

/// 一个变量（或表达式求值结果）。`value` 已由调用方格式化；`changed`
/// 表示自上一次停止以来发生变化，用语义色提示。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variable {
    pub name: SharedString,
    pub value: SharedString,
    pub type_name: Option<SharedString>,
    pub changed: bool,
    pub depth: usize,
}

impl Variable {
    pub fn new(name: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            type_name: None,
            changed: false,
            depth: 0,
        }
    }

    pub fn type_name(mut self, type_name: impl Into<SharedString>) -> Self {
        self.type_name = Some(type_name.into());
        self
    }

    pub fn changed(mut self, changed: bool) -> Self {
        self.changed = changed;
        self
    }

    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }
}

/// 变量列表（Variables 视图）。缩进表达层级，changed 用 warning 色。
#[derive(IntoElement)]
pub struct VariableList {
    id: ElementId,
    variables: Vec<Variable>,
    style: gpui_kit::StyleRefinement,
}

impl VariableList {
    pub fn new(id: impl Into<ElementId>, variables: Vec<Variable>) -> Self {
        Self {
            id: id.into(),
            variables,
            style: Default::default(),
        }
    }
}

impl Styled for VariableList {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for VariableList {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(self.id)
            .test_support()
            .flex()
            .flex_col()
            .refine_style(&self.style)
            .children(self.variables.into_iter().enumerate().map(|(ix, var)| {
                div()
                    .id(("variable", ix))
                    .test_support()
                    .flex()
                    .items_baseline()
                    .gap(rems(0.375))
                    .pl(rems(0.375 * (var.depth as f32 + 1.0)))
                    .py(rems(0.125))
                    .text_sm()
                    .when(var.changed, |el| el.bg(theme.warning_subtle))
                    .child(
                        div()
                            .text_color(theme.foreground)
                            .font_family(theme.mono_font_family.clone())
                            .child(var.name),
                    )
                    .children(
                        var.type_name
                            .map(|t| div().text_color(theme.subtle_foreground).text_xs().child(t)),
                    )
                    .child(
                        div()
                            .text_color(if var.changed {
                                theme.warning
                            } else {
                                theme.muted_foreground
                            })
                            .font_family(theme.mono_font_family.clone())
                            .child(var.value),
                    )
            }))
    }
}
