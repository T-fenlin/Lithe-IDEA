---
url: /zh-CN/base/primitives/slider.md
description: 轨道、已选区和滑块可独立设置样式的状态驱动范围输入。
---

# Slider

轨道、已选区和滑块可独立设置样式的状态驱动范围输入。

和所有 GPUI Base 原语一样，Slider 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- slider
```

## 导入

```rust
use gpui_kit::base::{Slider, SliderState};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/slider.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/slider.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

SliderState 持久保存值和范围，拖动及键盘操作更新它。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;
use gpui::relative;

impl BaseShowcase {
    pub(in super::super) fn slider(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let percentage = self.slider.read(cx).percentage().end;
        let thumb_size = 14.;
        div()
            .w_56()
            .text_xs()
            .child(
                div()
                    .mb_2()
                    .flex()
                    .justify_between()
                    .child("Volume")
                    .child("Drag to adjust"),
            )
            .child(
                Slider::new(&self.slider).w_full().h_7().child(
                    SliderTrack::new(&self.slider)
                        .relative()
                        .w_full()
                        .h_full()
                        .child(
                            div()
                                .absolute()
                                .top(px(13.))
                                .left_0()
                                .w_full()
                                .h(px(2.))
                                .bg(super::example_rgb(0xd4d4d4)),
                        )
                        .child(
                            SliderIndicator::new(&self.slider)
                                .absolute()
                                .top(px(13.))
                                .left_0()
                                .w_full()
                                .h(px(2.))
                                .child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .bottom_0()
                                        .left_0()
                                        .right(relative(1. - percentage))
                                        .bg(super::example_rgb(0x171717)),
                                ),
                        )
                        .child(
                            SliderThumb::new(&self.slider)
                                .absolute()
                                .top(px(7.))
                                .left(relative(percentage))
                                .ml(px(-thumb_size / 2.))
                                .size(px(thumb_size))
                                .bg(super::example_rgb(0xffffff))
                                .border_1()
                                .border_color(super::example_rgb(0x171717)),
                        ),
                ),
            )
    }
}
```

## 可访问性

暴露范围、当前值和方向，并保留方向键、Page Up/Down 等键盘操作。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
