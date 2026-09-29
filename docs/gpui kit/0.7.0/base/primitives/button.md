---
url: /zh-CN/base/primitives/button.md
description: 无样式、可访问且支持语义状态和键盘激活的按钮。
---

# Button

`Button` 提供按钮行为和语义结构，不强加产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- button
```

## 导入

```rust
use gpui_kit::base::Button;
```

## 结构与 API

示例组合了 `Button`。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/button.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/button.rs)。

## 状态与事件

激活使用 GPUI 点击处理。悬停、按下、焦点和禁用样式由应用负责。受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use gpui::relative;

use super::*;

impl BaseShowcase {
    pub(in super::super) fn button(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Button::new("primary-button")
                    .px_3()
                    .h_7()
                    .line_height(relative(1.))
                    .flex()
                    .items_center()
                    .text_xs()
                    .border_1()
                    .border_color(super::example_rgb(0x171717))
                    .bg(super::example_rgb(0x171717))
                    .text_color(super::example_rgb(0xffffff))
                    .hover(|style| style.bg(super::example_rgb(0x404040)))
                    .child("Save changes"),
            )
            .child(
                Button::new("secondary-button")
                    .px_3()
                    .h_7()
                    .line_height(relative(1.))
                    .flex()
                    .items_center()
                    .text_xs()
                    .border_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .bg(super::example_rgb(0xffffff))
                    .hover(|style| style.bg(super::example_rgb(0xf5f5f5)))
                    .child("Cancel"),
            )
    }
}
```

## 可访问性

提供可访问名称，保留键盘激活能力，并正确暴露禁用状态。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/button）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
