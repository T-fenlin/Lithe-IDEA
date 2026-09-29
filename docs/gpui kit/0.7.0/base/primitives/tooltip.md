---
url: /zh-CN/base/primitives/tooltip.md
description: 与触发元素关联、延迟显示且可定位的说明。
---

# Tooltip

与触发元素关联、延迟显示且可定位的说明。

和所有 GPUI Base 原语一样，Tooltip 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- tooltip
```

## 导入

```rust
use gpui_kit::base::{Tooltip};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/tooltip.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/tooltip.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

指针悬停或键盘聚焦后延迟显示，离开或失焦后关闭。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;

impl BaseShowcase {
    pub(in super::super) fn tooltip(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let visible = self.tooltip_visible;
        let entity = cx.entity().downgrade();
        let trigger = div()
            .id("tooltip-trigger")
            .on_hover(move |hovered, _, cx| {
                _ = entity.update(cx, |this, cx| {
                    this.tooltip_visible = *hovered;
                    cx.notify();
                });
            })
            .child(
                Button::new("tooltip-anchor")
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(super::example_rgb(0x171717))
                    .bg(super::example_rgb(0xffffff))
                    .child("Command menu"),
            );

        Popup::new("example-tooltip-popup", trigger)
            .text_xs()
            .when(visible, |this| {
                this.content(
                    Tooltip::new("example-tooltip")
                        .px_2()
                        .h_7()
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_1()
                        .border_color(super::example_rgb(0x171717))
                        .bg(super::example_rgb(0x171717))
                        .text_color(super::example_rgb(0xffffff))
                        .child("Open command menu · ⌘K"),
                )
            })
    }
}
```

## 可访问性

Tooltip 只补充说明，不能承载完成任务所必需的信息；触发器必须可聚焦。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/tooltip）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
