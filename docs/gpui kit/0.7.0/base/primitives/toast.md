---
url: /zh-CN/base/primitives/toast.md
description: 受管理、带动画的临时状态消息栈。
---

# Toast

受管理、带动画的临时状态消息栈。

和所有 GPUI Base 原语一样，Toast 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- toast
```

## 导入

```rust
use gpui_kit::base::{Toast, ToastManager, ToastOptions, ToastStack};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/toast.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/toast.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

ToastManager 管理消息的加入、超时和移除；应用选择持续时间与操作。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;

impl BaseShowcase {
    pub(in super::super) fn toast(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.toast_visible;
        let entity = cx.entity().downgrade();
        div()
            .w_72()
            .h(px(158.))
            .text_xs()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .child(
                Button::new("show-toast")
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(super::example_rgb(0x171717))
                    .bg(super::example_rgb(0xffffff))
                    .child("Save changes")
                    .on_click({
                        let show_entity = entity.clone();
                        move |_, _, cx| {
                            _ = show_entity.update(cx, |this, cx| {
                                this.toast_visible = true;
                                cx.notify();
                            });
                        }
                    }),
            )
            .when(visible, |this| {
                this.child(
                    Toast::new("example-toast")
                        .transition_status(ToastTransitionStatus::Present)
                        .absolute()
                        .right_0()
                        .bottom_0()
                        .w_64()
                        .p_2()
                        .border_1()
                        .border_color(super::example_rgb(0x171717))
                        .bg(super::example_rgb(0xffffff))
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child("Changes saved"),
                                )
                                .child(
                                    Button::new("dismiss-toast")
                                        .size_6()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child("×")
                                        .on_click({
                                            let entity = entity.clone();
                                            move |_, _, cx| {
                                                _ = entity.update(cx, |this, cx| {
                                                    this.toast_visible = false;
                                                    cx.notify();
                                                });
                                            }
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .mt_1()
                                .text_color(super::example_rgb(0x737373))
                                .child("Your preferences are now up to date."),
                        ),
                )
            })
    }
}
```

## 可访问性

根据紧急程度使用合适的实时区域；重要内容不能只依赖自动消失的消息。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/toast）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
