---
url: /zh-CN/base/primitives/time-field.md
description: 分段编辑一天中的时间，提供完整的键盘模型，支持 24 或 12 小时制。
---

# Time Field

分段编辑一天中的时间，提供完整的键盘模型，支持 24 或 12 小时制。

和所有 GPUI Base 原语一样，Time Field 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- time-field
```

## 导入

```rust
use gpui_kit::base::{HourCycle, TimeField, TimeFieldEvent, TimeFieldState, TimePrecision};
```

## 结构与 API

示例在 `TimeFieldState` 之上组合 `TimeField`。字段为时、分以及可选的秒和上午/下午各渲染一个 `TimeFieldSegment`，数字段之间用 `:` 分隔。用 `Styled` 布局和装饰根元素，通过 `TimeField::render_segment` 装饰每一段；该插槽会收到 `TimeFieldSegmentState`，包含段类型、当前值以及是否选中。权威实现位于 [`components/time_field.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/time_field.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

`TimeFieldState` 保存时间、精度（`TimePrecision::Minute` 或 `Second`）和小时制（默认 `HourCycle::H23`，也可以是 `H12`）。`set_time` 替换值但不触发事件；用户编辑会触发 `TimeFieldEvent::Change`。

整个字段是一个 Tab 停靠点。Up/Down 调整当前选中的段，并在段内循环、不向上一级进位；Left/Right 和 Tab/Shift-Tab 在段之间移动；输入数字时使用两位缓冲，当前段无法再容纳更多数字时自动跳到下一段；`a`/`p` 切换上午或下午；Backspace/Delete 重置当前段。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use gpui::{
    Context, IntoElement, ParentElement as _, Styled as _, div, prelude::FluentBuilder as _,
};
use gpui_base::TimeField;

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn time_field(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let value = self.time_field.read(cx).time();

        div()
            .w_56()
            .flex()
            .flex_col()
            .gap_1()
            .text_xs()
            .child(div().child("Reminder time"))
            .child(
                TimeField::new("example-time-field", &self.time_field)
                    .flex()
                    .items_center()
                    .h_7()
                    .px_2()
                    .border_1()
                    .border_color(super::example_rgb(0xa3a3a3))
                    .bg(super::example_rgb(0xffffff))
                    .render_segment(|segment, state, _, _| {
                        segment
                            .px_0p5()
                            .when(state.is_selected(), |this| {
                                this.bg(super::example_rgb(0xdbeafe))
                            })
                            .into_any_element()
                    }),
            )
            .child(
                div()
                    .text_color(super::example_rgb(0x737373))
                    .child(format!("Selected {}", value.format("%H:%M:%S"))),
            )
    }
}
```

## 可访问性

根元素以 `Role::TimeInput` 暴露，值为格式化后的时间。请为字段提供标签，并让选中的段在视觉上与其他段明显区分。

## 注意事项

使用等宽数字或固定段宽，避免输入数字时字段宽度变化。在消费端设计系统中验证焦点、选中、禁用和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/time-field）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
