---
url: /zh-CN/base/primitives/tabs.md
description: 带受控选择的标签列表和可访问标签控件。
---

# Tabs

带受控选择的标签列表和可访问标签控件。

和所有 GPUI Base 原语一样，Tabs 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- tabs
```

## 导入

```rust
use gpui_kit::base::{Tab, Tabs};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/tabs.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/tabs.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

父级保存活动标签，激活标签后更新对应面板。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;

impl BaseShowcase {
    pub(in super::super) fn tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected_tab;
        div()
            .w_72()
            .text_xs()
            .border_1()
            .border_color(super::example_rgb(0xd4d4d4))
            .child(
                Tabs::new("example-tabs")
                    .flex()
                    .px_2()
                    .pt_1()
                    .border_b_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .children(
                        ["Overview", "Activity", "Settings"]
                            .into_iter()
                            .enumerate()
                            .map(|(index, label)| {
                                let entity = cx.entity().downgrade();
                                Tab::new(index)
                                    .selected(self.selected_tab == index)
                                    .px_2()
                                    .h_7()
                                    .flex()
                                    .items_center()
                                    .border_b_2()
                                    .border_color(if self.selected_tab == index {
                                        super::example_rgb(0x171717)
                                    } else {
                                        super::example_rgb(0xffffff)
                                    })
                                    .when(self.selected_tab == index, |this| {
                                        this.font_weight(gpui::FontWeight::SEMIBOLD)
                                    })
                                    .on_click(move |_, _, cx| {
                                        _ = entity.update(cx, |this, cx| {
                                            this.selected_tab = index;
                                            cx.notify();
                                        });
                                    })
                                    .child(label)
                            }),
                    ),
            )
            .child(
                div().min_h_20().p_3().child(match selected {
                    0 => div().child("Workspace overview").child(
                        div()
                            .mt_1()
                            .text_color(super::example_rgb(0x737373))
                            .child("12 components · 4 contributors · updated today"),
                    ),
                    1 => div().child("Recent activity").child(
                        div()
                            .mt_1()
                            .text_color(super::example_rgb(0x737373))
                            .child("Button example was updated 8 minutes ago."),
                    ),
                    _ => div().child("Project settings").child(
                        div()
                            .mt_1()
                            .text_color(super::example_rgb(0x737373))
                            .child("Manage notifications and member access."),
                    ),
                }),
            )
    }
}
```

## 可访问性

保留标签列表、标签与面板关系，以及方向键和焦点行为。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
