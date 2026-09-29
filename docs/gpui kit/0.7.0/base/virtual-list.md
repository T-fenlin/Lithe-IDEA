---
url: /zh-CN/base/virtual-list.md
description: 只绘制屏幕内项目，流畅呈现十万条不同尺寸的行。
---

# VirtualList

Virtual List 只绘制当前屏幕内的项目，因此可处理任意长度的列表。不同于 `gpui_kit::uniform_list`，每一项可以有不同尺寸，适合可变行高表格、聊天记录和大纲树。它属于基础设施而不是带外观的组件：你预先提供尺寸，再提供渲染范围的闭包。

## 为什么要预先提供尺寸

虚拟化必须在不渲染项目的情况下知道总范围和可见项。`uniform_list` 用统一尺寸换取零逐项数据；边滚动边测量会造成滚动条跳动；`VirtualList` 用预先提供的逐项尺寸换取精确偏移和稳定滚动条。无法预先精确测量时可使用合理估算，或固定行高并裁剪内容。

## 开始使用

```rust
use std::rc::Rc;
use gpui_kit::base::{v_virtual_list, VirtualListScrollHandle};
use gpui_kit::{px, size};

let sizes = Rc::new(vec![size(px(280.), px(32.)); 100_000]);
v_virtual_list(cx.entity(), "customers", sizes, |_this, range, _window, _cx| {
    range.map(|ix| div().h_8().px_2().child(format!("Customer {ix}"))).collect()
})
.track_scroll(&self.scroll_handle)
.size_full()
```

闭包只收到可见范围及少量超绘制，并为每个索引返回一个元素。横向列表使用 `h_virtual_list`。

## 尺寸契约

- 纵向列表只读取 `height`，横向列表只读取 `width`。
- 交叉轴通过布局一个项目测量，默认使用第 0 项；不具代表性时调用 `.with_item_to_measure_index(3)`。
- `item_sizes.len()` 就是项目数，必须与数据一致。

尺寸表使用 `Rc<Vec<Size<Pixels>>>`，应保存在 entity 中并只克隆句柄，不要在 `render` 中重建。

## 滚动与滚动条

`VirtualListScrollHandle` 持有跨渲染的滚动位置，支持 `scroll_to_item(index, ScrollStrategy::Top)`、`scroll_to_bottom()` 和 `base_handle()`。它实现了 `ScrollbarHandle`，可直接传给 `Scrollbar::vertical(&self.scroll)`；外层容器需要 `relative()`。

## 尺寸行为与热路径

`ListSizingBehavior::Auto`（默认）使用父级提供的空间；`Infer` 根据项目推导尺寸。虚拟列表必须放在有边界的父级中。可见范围变化的每一帧都会运行渲染闭包，因此其中不要做 I/O、排序或过滤，也不要为每行创建持久 GPUI entity。元素 ID 应来自稳定数据键或项目索引，状态更新放在回调中并调用 `cx.notify()`。

每帧工作量与可见项目数而非总数成正比；只有尺寸表随总数增长。大量统一高度项目更适合 `gpui_kit::uniform_list`。

## 完整 Rust 示例

```bash
cargo run -p gpui-base-examples -- virtual-list
```

```rust
use super::*;

const ITEM_COUNT: usize = 100_000;

impl BaseShowcase {
    pub(in super::super) fn virtual_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let sizes = Rc::new(vec![size(px(280.), px(32.)); ITEM_COUNT]);
        div()
            .relative()
            .w_72()
            .h_48()
            .overflow_hidden()
            .border_1()
            .border_color(super::example_rgb(0x171717))
            .child(
                v_virtual_list(
                    cx.entity(),
                    "example-virtual-list",
                    sizes,
                    |_, range, _, _| {
                        range
                            .map(|ix| {
                                div()
                                    .w_full()
                                    .h_8()
                                    .px_2()
                                    .text_xs()
                                    .flex()
                                    .items_center()
                                    .border_b_1()
                                    .border_color(gpui::black())
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .size(px(18.))
                                                    .flex_none()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .line_height(px(18.))
                                                    .border_1()
                                                    .border_color(gpui::black())
                                                    .child(format!("{}", (ix % 9) + 1)),
                                            )
                                            .child(format!("Customer {:06}", ix + 1)),
                                    )
                                    .child(format!("ID-{:06}", 100_000 + ix))
                            })
                            .collect()
                    },
                )
                .track_scroll(&self.virtual_scroll)
                .size_full(),
            )
            .child(Scrollbar::vertical(&self.virtual_scroll).mode(ScrollbarMode::Always))
    }
}
```

## 检查清单

- 在 entity 中保存尺寸表和滚动句柄。
- 保持尺寸表与数据长度一致，并选择有代表性的测量项。
- 提供有边界的父级；添加滚动条时父级使用 `relative()`。
- 保持逻辑顺序、项目数和稳定身份，让辅助技术获得连贯列表。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/virtual-list）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
