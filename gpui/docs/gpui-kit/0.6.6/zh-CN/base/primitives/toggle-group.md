---
url: /zh-CN/base/primitives/toggle-group.md
description: 将多个 Toggle 协调为单选或多选组。
---

# Toggle Group

将多个 Toggle 协调为单选或多选组。

和所有 GPUI Base 原语一样，Toggle Group 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- toggle-group
```

## 导入

```rust
use gpui_kit::base::{Toggle, ToggleGroup};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/toggle-group.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/toggle-group.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

组模式决定只保留一个值还是一组值；变更由父级持久化。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;

impl BaseShowcase {
    pub(in super::super) fn toggle_group(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let italic = self.toggle_group_selection & 1 != 0;
        let underline = self.toggle_group_selection & 2 != 0;
        let entity = cx.entity().downgrade();
        ToggleGroup::new("example-toggle-group")
            .flex()
            .text_xs()
            .gap_0()
            .child(self.toggle(cx))
            .child(
                Toggle::new("italic-toggle")
                    .pressed(italic)
                    .size_7()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_l_0()
                    .border_color(super::example_rgb(0x171717))
                    .when(italic, |this| {
                        this.bg(super::example_rgb(0x171717))
                            .text_color(super::example_rgb(0xffffff))
                    })
                    .accessibility_label("Italic")
                    .child("I")
                    .on_change({
                        let entity = entity.clone();
                        move |next, _, _, cx| {
                            _ = entity.update(cx, |this, cx| {
                                if next {
                                    this.toggle_group_selection |= 1
                                } else {
                                    this.toggle_group_selection &= !1
                                };
                                cx.notify();
                            });
                        }
                    }),
            )
            .child(
                Toggle::new("underline-toggle")
                    .pressed(underline)
                    .size_7()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_l_0()
                    .border_color(super::example_rgb(0x171717))
                    .when(underline, |this| {
                        this.bg(super::example_rgb(0x171717))
                            .text_color(super::example_rgb(0xffffff))
                    })
                    .accessibility_label("Underline")
                    .child("U")
                    .on_change(move |next, _, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            if next {
                                this.toggle_group_selection |= 2
                            } else {
                                this.toggle_group_selection &= !2
                            };
                            cx.notify();
                        });
                    }),
            )
    }
}
```

## 可访问性

提供组名称，清楚表达每项的按下状态，并支持一致的键盘导航。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
