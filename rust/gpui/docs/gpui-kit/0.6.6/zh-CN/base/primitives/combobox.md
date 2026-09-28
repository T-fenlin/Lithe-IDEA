---
url: /zh-CN/base/primitives/combobox.md
description: 结合文本输入、键盘导航建议和选择行为的组合框。
---

# Combobox

结合文本输入、键盘导航建议和选择行为的组合框。

和所有 GPUI Base 原语一样，Combobox 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- combobox
```

## 导入

```rust
use gpui_kit::base::{Combobox};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/combobox.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/combobox.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

持久状态保存查询、候选项、焦点项和选择；输入与选择事件由应用处理。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;
use gpui::MouseButton;

impl BaseShowcase {
    pub(in super::super) fn combobox(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = self.combobox_open;
        let query = self.combobox_query.read(cx).value().to_lowercase();
        let selected = self.combobox_selection.clone();
        let entity = cx.entity().downgrade();
        let query_state = self.combobox_query.clone();
        let open_query_state = self.combobox_query.clone();
        let trigger_entity = cx.entity().downgrade();
        let trigger_query_state = self.combobox_query.clone();

        let combobox = Combobox::new("example-combobox")
            .open(open)
            .on_open_change(move |open, window, cx| {
                _ = entity.update(cx, |this, cx| {
                    this.combobox_open = open;
                    cx.notify();
                });
                if open {
                    open_query_state.update(cx, |state, cx| state.focus(window, cx));
                }
            })
            .w_56()
            .child(
                div()
                    .id("combobox-trigger")
                    .w_full()
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .text_xs()
                    .bg(super::example_rgb(0xffffff))
                    .on_click(move |_, window, cx| {
                        _ = trigger_entity.update(cx, |this, cx| {
                            this.combobox_open = !open;
                            cx.notify();
                        });
                        if !open {
                            trigger_query_state.update(cx, |state, cx| state.focus(window, cx));
                        }
                    })
                    .child(selected)
                    .child(div().text_color(super::example_rgb(0x737373)).child("⌄")),
            );
        let popup = div()
            .w_56()
            .p_1()
            .border_1()
            .border_color(super::example_rgb(0xd4d4d4))
            .bg(super::example_rgb(0xffffff))
            .child(
                InputBase::new("combobox-search")
                    .w_full()
                    .h_7()
                    .px_2()
                    .border_1()
                    .border_color(super::example_rgb(0xe5e5e5))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        query_state.update(cx, |state, cx| state.focus(window, cx));
                    })
                    .child(self.combobox_query.clone()),
            )
            .child(
                div().mt_1().children(
                    ["GPUI", "React", "SwiftUI", "Vue"]
                        .into_iter()
                        .filter(|label| query.is_empty() || label.to_lowercase().contains(&query))
                        .map(|label| {
                            let entity = cx.entity().downgrade();
                            div()
                                .id(format!("combobox-{label}"))
                                .px_2()
                                .h_7()
                                .flex()
                                .items_center()
                                .text_xs()
                                .hover(|s| s.bg(super::example_rgb(0xf5f5f5)))
                                .on_click(move |_, _, cx| {
                                    _ = entity.update(cx, |this, cx| {
                                        this.combobox_selection = label.into();
                                        this.combobox_open = false;
                                        cx.notify();
                                    });
                                })
                                .child(label)
                        }),
                ),
            );

        Popup::new("example-combobox-popup", combobox).when(open, |this| this.content(popup))
    }
}
```

## 可访问性

保留组合框语义、活动后代关系以及上下键、Enter 和 Escape 操作。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
