//! 标签页（第 7 章 Navigation）。容器与项复用 gpui-base 的 headless
//! `Tabs`/`Tab`，选中态走主题；`on_select` 报告索引，选中值由调用方持有。

use std::rc::Rc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::base::{Tab as BaseTab, Tabs as BaseTabs};
use gpui_kit::prelude::*;
use gpui_kit::{
    App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, rems,
};

use crate::ActiveTheme;

/// 标签页容器。`children` 应全部是 [`Tab`]。
#[derive(IntoElement)]
pub struct Tabs {
    id: ElementId,
    children: Vec<Tab>,
    selected_index: usize,
    on_select: Option<Rc<dyn Fn(usize, &mut Window, &mut App)>>,
    style: gpui_kit::StyleRefinement,
}

impl Tabs {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            children: Vec::new(),
            selected_index: 0,
            on_select: None,
            style: Default::default(),
        }
    }

    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = index;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    pub fn child(mut self, tab: Tab) -> Self {
        self.children.push(tab);
        self
    }

    pub fn children(mut self, tabs: impl IntoIterator<Item = Tab>) -> Self {
        self.children.extend(tabs);
        self
    }
}

impl Styled for Tabs {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

/// 单个标签。
pub struct Tab {
    label: SharedString,
}

impl Tab {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

impl RenderOnce for Tabs {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let selected_color = theme.foreground;
        let muted_color = theme.muted_foreground;
        let hover_color = theme.foreground;
        let selected_border = theme.foreground;

        let tabs = self.children.into_iter().enumerate().map(|(ix, tab)| {
            let selected = ix == self.selected_index;
            let on_select = self.on_select.clone();
            BaseTab::new(ix)
                .selected(selected)
                .text_sm()
                .text_color(if selected {
                    selected_color
                } else {
                    muted_color
                })
                .when(!selected, |el| {
                    el.hover(move |style| style.text_color(hover_color))
                })
                .styles(|styles| {
                    styles.selected(move |style| {
                        style
                            .text_color(selected_color)
                            .border_color(selected_border)
                    })
                })
                .child(tab.label)
                .on_click(move |_, window, cx| {
                    if let Some(handler) = on_select.as_ref() {
                        handler(ix, window, cx);
                    }
                })
        });

        BaseTabs::new(self.id)
            .h(rems(2.0))
            .gap(rems(1.0))
            .border_b_1()
            .border_color(theme.border)
            .refine_style(&self.style)
            .children(tabs)
    }
}
