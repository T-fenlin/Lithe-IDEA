---
url: /zh-CN/base/primitives/calendar.md
description: 支持选择匹配器和自定义日期项渲染的状态驱动日期网格。
---

# Calendar

支持选择匹配器和自定义日期项渲染的状态驱动日期网格。

和所有 GPUI Base 原语一样，Calendar 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- calendar
```

## 导入

```rust
use gpui_kit::base::{Calendar, CalendarState};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/calendar.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/calendar.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

CalendarState 保存可见月份和选择；回调负责同步受控值。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;

impl BaseShowcase {
    pub(in super::super) fn calendar(&self) -> impl IntoElement {
        Calendar::new("example-calendar", &self.calendar)
            // 7 × 32px cells + 12px padding on each side + 1px borders.
            .w(px(250.))
            .p_3()
            .border_1()
            .border_color(super::example_rgb(0xd4d4d4))
            .item(|item, state, _, _| {
                match state.kind() {
                    CalendarItemKind::Previous | CalendarItemKind::Next => item
                        .size_7()
                        .flex()
                        .items_center()
                        .justify_center()
                        .hover(|s| s.bg(super::example_rgb(0xf5f5f5))),
                    CalendarItemKind::MonthToggle | CalendarItemKind::YearToggle => item
                        .px_1()
                        .h_7()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .hover(|s| s.bg(super::example_rgb(0xf5f5f5))),
                    CalendarItemKind::Weekday => item
                        .size_8()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .text_color(super::example_rgb(0x737373)),
                    CalendarItemKind::Day => item
                        .size_8()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .when(state.is_muted(), |s| {
                            s.text_color(super::example_rgb(0xa3a3a3))
                        })
                        .when(state.is_today() && !state.is_active(), |s| {
                            s.border_1().border_color(super::example_rgb(0xd4d4d4))
                        })
                        .when(state.is_active(), |s| {
                            s.bg(super::example_rgb(0x171717))
                                .text_color(super::example_rgb(0xffffff))
                        })
                        .when(!state.is_disabled() && !state.is_active(), |s| {
                            s.hover(|s| s.bg(super::example_rgb(0xf5f5f5)))
                        }),
                    CalendarItemKind::Month | CalendarItemKind::Year => item
                        .w(px(74.))
                        .h_7()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_xs()
                        .when(state.is_active(), |s| {
                            s.bg(super::example_rgb(0x171717))
                                .text_color(super::example_rgb(0xffffff))
                        })
                        .when(!state.is_active(), |s| {
                            s.hover(|s| s.bg(super::example_rgb(0xf5f5f5)))
                        }),
                }
                .into_any_element()
            })
            .label(|kind, value| match kind {
                CalendarItemKind::Previous => "‹".into(),
                CalendarItemKind::Next => "›".into(),
                CalendarItemKind::MonthToggle | CalendarItemKind::Month => [
                    "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
                    "Nov", "Dec",
                ][value as usize]
                    .into(),
                CalendarItemKind::Weekday => {
                    ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"][value as usize].into()
                }
                _ => value.to_string().into(),
            })
    }
}
```

## 可访问性

保留日期网格语义、方向键导航、焦点与选中状态。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
