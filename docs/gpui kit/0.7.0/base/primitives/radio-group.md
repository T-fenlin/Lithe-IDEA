---
url: /zh-CN/base/primitives/radio-group.md
description: 将单选项分组，并为单项选择提供键盘导航。
---

# Radio Group

将单选项分组，并为单项选择提供键盘导航。

和所有 GPUI Base 原语一样，Radio Group 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- radio-group
```

## 导入

```rust
use gpui_kit::base::{Radio, RadioGroup};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/radio-group.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/radio-group.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

组持有唯一选中值，并协调各项的焦点与选择。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use gpui::{
    Context, IntoElement, ParentElement as _, Styled as _, div, prelude::FluentBuilder as _, px,
};
use gpui_base::{Radio, RadioGroup};

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn radio_group(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity().downgrade();
        RadioGroup::new("example-radio-group")
            .w_56()
            .text_xs()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.radio(cx))
            .child(
                Radio::new("express-radio")
                    .checked(self.radio_selected == 1)
                    .on_change(move |next, _, _, cx| {
                        if next {
                            _ = entity.update(cx, |this, cx| {
                                this.radio_selected = 1;
                                cx.notify();
                            });
                        }
                    })
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div()
                            .mt(px(2.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .size(px(14.))
                            .border_1()
                            .border_color(super::example_rgb(0x171717))
                            .when(self.radio_selected == 1, |this| {
                                this.child(div().size(px(6.)).bg(super::example_rgb(0x171717)))
                            }),
                    )
                    .child(
                        div().child("Express").child(
                            div()
                                .text_xs()
                                .text_color(super::example_rgb(0x737373))
                                .child("Next business day"),
                        ),
                    ),
            )
            .child(
                Radio::new("pickup-radio")
                    .disabled(true)
                    .flex()
                    .items_start()
                    .gap_2()
                    .opacity(0.45)
                    .child(
                        div()
                            .mt(px(2.))
                            .size(px(14.))
                            .border_1()
                            .border_color(super::example_rgb(0x171717)),
                    )
                    .child(
                        div()
                            .child("Local pickup")
                            .child(div().text_xs().child("Currently unavailable")),
                    ),
            )
    }
}
```

## 可访问性

提供组标签，保留方向键导航以及各项的选中、禁用语义。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/radio-group）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
