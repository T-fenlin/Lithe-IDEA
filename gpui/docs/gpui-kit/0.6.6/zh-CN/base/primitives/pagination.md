---
url: /zh-CN/base/primitives/pagination.md
description: 显式管理当前页和总页数的受控分页导航。
---

# Pagination

显式管理当前页和总页数的受控分页导航。

和所有 GPUI Base 原语一样，Pagination 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- pagination
```

## 导入

```rust
use gpui_kit::base::{Pagination, PaginationState};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/pagination.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/pagination.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

PaginationState 保存页数边界与当前页，应用处理页码变化。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use gpui::{
    Context, IntoElement, ParentElement as _, Styled as _, div, prelude::FluentBuilder as _, px,
};
use gpui_base::{Button, Pagination, PaginationItem, PaginationState};

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn pagination(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        let state = PaginationState::new(self.page, 8).on_change(move |page, _, cx| {
            _ = entity.update(cx, |this, cx| {
                this.page = page;
                cx.notify();
            });
        });
        let items = state.items();
        Pagination::new("example-pagination", state.clone())
            .flex()
            .items_center()
            .gap_2()
            .text_xs()
            .children(items.into_iter().map(move |item| {
                match item {
                    PaginationItem::Page(page) => {
                        let state = state.clone();
                        Button::new(("page", page))
                            .size_7()
                            .p_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_1()
                            .border_color(super::example_rgb(0xd4d4d4))
                            .when(page == state.current_page(), |this| {
                                this.bg(super::example_rgb(0x171717))
                                    .text_color(super::example_rgb(0xffffff))
                            })
                            .on_click(move |_, window, cx| state.request_page(page, window, cx))
                            .child(page.to_string())
                            .into_any_element()
                    }
                    PaginationItem::Ellipsis(_) => div()
                        .w(px(20.))
                        .h_7()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child("…")
                        .into_any_element(),
                }
            }))
    }
}
```

## 可访问性

标记当前页，为上一页、下一页和具体页码提供可访问名称。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
