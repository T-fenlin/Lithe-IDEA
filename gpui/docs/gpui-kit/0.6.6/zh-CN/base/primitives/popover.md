---
url: /zh-CN/base/primitives/popover.md
description: 支持受控或内部开关状态的锚定浮层。
---

# Popover

支持受控或内部开关状态的锚定浮层。

和所有 GPUI Base 原语一样，Popover 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- popover
```

## 导入

```rust
use gpui_kit::base::{Popover};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/popover.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/popover.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

触发器切换打开状态；点击外部或 Escape 可按配置关闭。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use gpui::{InteractiveElement as _, IntoElement, ParentElement as _, Styled as _, div, relative};
use gpui_base::{Button, Popover};

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn popover(&self) -> impl IntoElement {
        Popover::new("example-popover")
            .trigger(
                Button::new("popover-trigger")
                    .h_7()
                    .line_height(relative(1.))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(gpui::black())
                    .text_color(gpui::white())
                    .child("Open Popover"),
            )
            .content(|_, _, cx| {
                let state = cx.entity().downgrade();
                div()
                    .id("popover-content")
                    .w_64()
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .text_xs()
                    .bg(super::example_rgb(0xffffff))
                    .border_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .child("Workspace access")
                    .child(
                        div()
                            .text_xs()
                            .text_color(super::example_rgb(0x737373))
                            .child("Anyone with the link can view."),
                    )
                    .child(
                        div().mt_1().flex().justify_end().child(
                            Button::new("popover-done")
                                .h_7()
                                .line_height(relative(1.))
                                .px_3()
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(gpui::black())
                                .text_color(gpui::white())
                                .on_click(move |_, window, cx| {
                                    _ = state.update(cx, |state, cx| state.dismiss(window, cx));
                                })
                                .child("Done"),
                        ),
                    )
            })
    }
}
```

## 可访问性

管理触发器与内容的关系和焦点，不要让关闭后焦点丢失。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
