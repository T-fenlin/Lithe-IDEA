---
url: /zh-CN/base/primitives/popup.md
description: 底层触发器与锚定浮动内容宿主。
---

# Popup

底层触发器与锚定浮动内容宿主。

和所有 GPUI Base 原语一样，Popup 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- popup
```

## 导入

```rust
use gpui_kit::base::{Popup};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/popup.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/popup.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

应用负责打开状态和关闭策略；Popup 负责锚定及浮层结构。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use gpui::{
    Context, IntoElement, ParentElement as _, Styled as _, div, prelude::FluentBuilder as _,
    relative,
};
use gpui_base::{Button, Popup};

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn popup(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.popup_open;
        let entity = cx.entity().downgrade();
        Popup::new(
            "example-popup",
            Button::new("popup-trigger")
                .h_7()
                .line_height(relative(1.))
                .px_3()
                .flex()
                .items_center()
                .justify_center()
                .bg(gpui::black())
                .text_color(gpui::white())
                .on_click(move |_, _, cx| {
                    _ = entity.update(cx, |this, cx| {
                        this.popup_open = !this.popup_open;
                        cx.notify();
                    });
                })
                .child(if open { "Close popup" } else { "Open popup" }),
        )
        .when(open, |this| {
            this.content(
                div()
                    .w_64()
                    .p_2()
                    .text_xs()
                    .bg(super::example_rgb(0xffffff))
                    .border_1()
                    .border_color(super::example_rgb(0x171717))
                    .child("Anchored surface")
                    .child(
                        div()
                            .mt_1()
                            .text_sm()
                            .text_color(super::example_rgb(0x737373))
                            .child("Popup positions content relative to its trigger."),
                    ),
            )
        })
    }
}
```

## 可访问性

根据所组合控件补充正确角色、名称、焦点管理与 Escape 关闭行为。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/popup）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
