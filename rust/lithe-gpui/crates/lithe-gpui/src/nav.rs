//! 面包屑与分页（第 7 章 Navigation）。

use std::rc::Rc;

use gpui_kit::TestSupportExt as _;
use gpui_kit::base::{Pagination as BasePagination, PaginationState};
use gpui_kit::prelude::*;
use gpui_kit::{App, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window, div, rems};

use crate::ActiveTheme;

/// 面包屑。条目由调用方给（名称 + 可选的点击回调）；分隔符为文本字符。
#[derive(IntoElement)]
pub struct Breadcrumb {
    id: ElementId,
    items: Vec<(SharedString, Option<Rc<dyn Fn(&mut Window, &mut App)>>)>,
}

impl Breadcrumb {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
        }
    }

    pub fn item(
        mut self,
        label: impl Into<SharedString>,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.items.push((label.into(), Some(Rc::new(on_click))));
        self
    }

    /// 当前（不可点）条目。
    pub fn current(mut self, label: impl Into<SharedString>) -> Self {
        self.items.push((label.into(), None));
        self
    }
}

impl RenderOnce for Breadcrumb {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let count = self.items.len();
        div()
            .id(self.id)
            .test_support()
            .flex()
            .items_center()
            .gap(rems(0.5))
            .children(
                self.items
                    .into_iter()
                    .enumerate()
                    .flat_map(move |(ix, (label, on_click))| {
                        let is_last = ix + 1 == count;
                        let crumb = match on_click {
                            Some(handler) => div()
                                .id(("crumb", ix))
                                .test_support()
                                .cursor_pointer()
                                .text_sm()
                                .text_color(theme.link)
                                .hover(move |style| style.text_color(theme.link_hover))
                                .child(label)
                                .on_click(move |_, window, cx| handler(window, cx))
                                .into_any_element(),
                            None => div()
                                .text_sm()
                                .font_weight(gpui_kit::FontWeight::MEDIUM)
                                .text_color(if is_last {
                                    theme.foreground
                                } else {
                                    theme.muted_foreground
                                })
                                .child(label)
                                .into_any_element(),
                        };
                        let separator = (!is_last).then(|| {
                            div()
                                .text_sm()
                                .text_color(theme.subtle_foreground)
                                .child("›")
                                .into_any_element()
                        });
                        [Some(crumb), separator]
                    })
                    .flatten(),
            )
    }
}

/// 分页。状态无快照保留：`on_change` 报告新页码，当前页由调用方持有。
#[derive(IntoElement)]
pub struct Pagination {
    id: ElementId,
    current_page: usize,
    total_pages: usize,
    on_change: Option<Rc<dyn Fn(usize, &mut Window, &mut App)>>,
}

impl Pagination {
    pub fn new(id: impl Into<ElementId>, current_page: usize, total_pages: usize) -> Self {
        Self {
            id: id.into(),
            current_page,
            total_pages,
            on_change: None,
        }
    }

    pub fn on_change(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Pagination {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let on_change = self.on_change.clone();
        let state = PaginationState::new(self.current_page, self.total_pages.max(1)).on_change(
            move |page, window, cx| {
                if let Some(handler) = on_change.as_ref() {
                    handler(page, window, cx);
                }
            },
        );
        div()
            .flex()
            .text_sm()
            .text_color(theme.foreground)
            .child(BasePagination::new(self.id, state))
    }
}
