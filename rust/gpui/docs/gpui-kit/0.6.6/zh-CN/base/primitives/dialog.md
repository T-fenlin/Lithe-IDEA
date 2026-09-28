---
url: /zh-CN/base/primitives/dialog.md
description: 带焦点管理、遮罩、标题和关闭部件的可组合模态界面。
---

# Dialog

带焦点管理、遮罩、标题和关闭部件的可组合模态界面。

和所有 GPUI Base 原语一样，Dialog 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- dialog
```

## 导入

```rust
use gpui_kit::base::{Dialog, DialogBackdrop, DialogClose, DialogDescription, DialogPopup, DialogTitle, DialogTrigger};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/dialog.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/dialog.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

打开状态可以受控；触发器、关闭按钮与遮罩根据产品策略更新它。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;
use gpui::{MouseButton, relative};

impl BaseShowcase {
    pub(in super::super) fn dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.dialog_open;
        let entity = cx.entity().downgrade();
        let open_entity = entity.clone();

        div()
            .child(
                Button::new("open-dialog")
                    .h_7()
                    .line_height(relative(1.))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(gpui::black())
                    .text_color(gpui::white())
                    .on_click(move |_, _, cx| {
                        _ = open_entity.update(cx, |this, cx| {
                            this.dialog_open = true;
                            cx.notify();
                        });
                    })
                    .child("Edit profile"),
            )
            .child(
                Dialog::new(cx)
                    .open(open)
                    .on_open_change(move |open, _, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.dialog_open = open;
                            cx.notify();
                        });
                    })
                    .backdrop(
                        DialogBackdrop::new()
                            .absolute()
                            .inset_0()
                            .bg(super::example_rgb(0x000000))
                            .opacity(0.2),
                    )
                    .popup(
                        DialogPopup::new()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w_72()
                                    .p_3()
                                    .flex()
                                    .flex_col()
                                    .items_stretch()
                                    .text_xs()
                                    .bg(super::example_rgb(0xffffff))
                                    .border_1()
                                    .border_color(super::example_rgb(0xd4d4d4))
                                    .child(
                                        DialogTitle::new()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child("Edit profile"),
                                    )
                                    .child(
                                        DialogDescription::new()
                                            .mt_2()
                                            .text_color(super::example_rgb(0x737373))
                                            .child(
                                                "Update the public details shown on your profile.",
                                            ),
                                    )
                                    .child(div().mt_3().text_sm().child("Display name"))
                                    .child(
                                        InputBase::new("dialog-name")
                                            .mt_2()
                                            .w_full()
                                            .h_7()
                                            .px_2()
                                            .border_1()
                                            .border_color(super::example_rgb(0xd4d4d4))
                                            .on_mouse_down(MouseButton::Left, {
                                                let input = self.input.clone();
                                                move |_, window, cx| {
                                                    input.update(cx, |state, cx| {
                                                        state.focus(window, cx)
                                                    });
                                                }
                                            })
                                            .child(self.input.clone()),
                                    )
                                    .child(
                                        div()
                                            .mt_3()
                                            .flex()
                                            .justify_end()
                                            .gap_2()
                                            .child(
                                                gpui_base::DialogClose::new().child(
                                                    Button::new("dialog-cancel")
                                                        .h_7()
                                                        .line_height(relative(1.))
                                                        .px_3()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .border_1()
                                                        .border_color(super::example_rgb(0xd4d4d4))
                                                        .child("Cancel"),
                                                ),
                                            )
                                            .child(
                                                Button::new("dialog-save")
                                                    .h_7()
                                                    .line_height(relative(1.))
                                                    .px_3()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .bg(super::example_rgb(0x171717))
                                                    .text_color(super::example_rgb(0xffffff))
                                                    .on_click({
                                                        let entity = cx.entity().downgrade();
                                                        move |_, _, cx| {
                                                            _ = entity.update(cx, |this, cx| {
                                                                this.dialog_open = false;
                                                                cx.notify();
                                                            });
                                                        }
                                                    })
                                                    .child("Save changes"),
                                            ),
                                    ),
                            ),
                    ),
            )
    }
}
```

## 可访问性

打开后管理焦点，关联标题与说明，并提供键盘可达的关闭方式。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
