---
url: /zh-CN/base/primitives/hover-card.md
description: 与指针或键盘触发器关联的延迟浮动卡片。
---

# Hover Card

与指针或键盘触发器关联的延迟浮动卡片。

和所有 GPUI Base 原语一样，Hover Card 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

iOS 和 Android 上，点击触发元素切换卡片开关，点击外部关闭；忽略悬停及其打开、关闭延迟。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- hover-card
```

## 导入

```rust
use gpui_kit::base::{HoverCard};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/hover-card.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/hover-card.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

悬停或聚焦触发器后延迟打开，离开后关闭。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;

impl BaseShowcase {
    pub(in super::super) fn hover_card(&self) -> impl IntoElement {
        HoverCard::new("example-hover-card")
            .trigger(
                div()
                    .id("hover-trigger")
                    .px_3()
                    .py_1()
                    .text_xs()
                    .text_color(super::example_rgb(0x171717))
                    .underline()
                    .child("Hover over gpui-base"),
            )
            .content(|_, _, _| {
                div()
                    .id("hover-content")
                    .w(px(210.))
                    .p_2()
                    .text_xs()
                    .bg(super::example_rgb(0xffffff))
                    .border_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .size_7()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .border_1()
                                    .border_color(super::example_rgb(0x171717))
                                    .text_sm()
                                    .child("G"),
                            )
                            .child(
                                div().text_sm().child("gpui-base").child(
                                    div()
                                        .text_sm()
                                        .text_color(super::example_rgb(0x737373))
                                        .child("@gpui-base"),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_sm()
                            .text_color(super::example_rgb(0x737373))
                            .child("Unstyled primitives for GPUI."),
                    )
            })
    }
}
```

## 可访问性

不要把完成任务所必需的操作只放在 Hover Card 中；键盘焦点也应能触发。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/hover-card）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
