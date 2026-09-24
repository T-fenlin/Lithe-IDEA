---
url: /zh-CN/base/primitives/number-input.md
description: 带可复用递增、递减和步进行为的数字输入。
---

# Number Input

带可复用递增、递减和步进行为的数字输入。

和所有 GPUI Base 原语一样，Number Input 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- number-input
```

## 导入

```rust
use gpui_kit::base::{Decrement, Increment, NumberInput, NumberInputText};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/number-input.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/number-input.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

父级或持久 entity 保存数值；按钮和文本输入共同更新同一状态。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement as _, Styled as _, div, px,
    relative,
};
use gpui_base::{Button, NumberInput};

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn number_input(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let valid = self.input.read(cx).value().parse::<f64>().is_ok();

        fn render_btn(this: Button, icon: AnyElement) -> Button {
            this.w(px(24.))
                .flex_1()
                .min_h_0()
                .line_height(relative(1.))
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::black())
                .text_color(gpui::white())
                .hover(|this| this.bg(gpui::black().opacity(0.8)))
                .child(icon)
        }

        fn minus_icon() -> AnyElement {
            div()
                .w(px(8.))
                .h(px(1.))
                .bg(gpui::white())
                .into_any_element()
        }

        fn plus_icon() -> AnyElement {
            div()
                .relative()
                .size(px(8.))
                .child(
                    div()
                        .absolute()
                        .top(px(3.5))
                        .left_0()
                        .w_full()
                        .h(px(1.))
                        .bg(gpui::white()),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(3.5))
                        .top_0()
                        .h_full()
                        .w(px(1.))
                        .bg(gpui::white()),
                )
                .into_any_element()
        }

        div()
            .w(px(200.))
            .flex()
            .flex_col()
            .gap_1()
            .text_xs()
            .child(div().text_xs().child("Quantity"))
            .child(
                NumberInput::new(&self.input)
                    .controls_right()
                    .w_full()
                    .h_7()
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(if valid {
                        super::example_rgb(0x171717)
                    } else {
                        super::example_rgb(0x737373)
                    })
                    .input(div().w_full().px_2().child(self.input.clone()))
                    .decrement_button(|button| render_btn(button, minus_icon()))
                    .increment_button(|button| render_btn(button, plus_icon())),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(super::example_rgb(0x737373))
                    .child(if valid { "Step: 1" } else { "Enter a number" }),
            )
    }
}
```

## 可访问性

提供标签、范围与当前值语义，并让增减操作可通过键盘完成。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
