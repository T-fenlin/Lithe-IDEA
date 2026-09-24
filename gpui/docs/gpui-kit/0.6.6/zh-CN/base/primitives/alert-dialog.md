---
url: /zh-CN/base/primitives/alert-dialog.md
description: 用于需要用户明确决定之操作的模态确认界面。
---

# Alert Dialog

用于需要用户明确决定之操作的模态确认界面。

和所有 GPUI Base 原语一样，Alert Dialog 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- alert-dialog
```

## 导入

```rust
use gpui_kit::base::{AlertDialog, AlertDialogAction, AlertDialogCancel, AlertDialogDescription, AlertDialogPopup, AlertDialogTitle, AlertDialogTrigger};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/alert-dialog.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/alert-dialog.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

打开状态和确认/取消结果由应用管理。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use gpui::relative;

use super::*;

impl BaseShowcase {
    pub(in super::super) fn alert_dialog(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.alert_dialog_open;
        let entity = cx.entity().downgrade();
        let open_entity = entity.clone();
        let ok_entity = entity.clone();
        let cancel_entity = entity.clone();
        let action_entity = entity.clone();

        div()
            .child(
                Button::new("open-alert-dialog")
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
                            this.alert_dialog_open = true;
                            cx.notify();
                        });
                    })
                    .child("Delete project"),
            )
            .child(
                AlertDialog::new(cx)
                    .open(open)
                    .on_open_change(move |open, _, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.alert_dialog_open = open;
                            cx.notify();
                        });
                    })
                    .on_ok(move |_, _, cx| {
                        _ = ok_entity.update(cx, |this, cx| {
                            this.alert_dialog_open = false;
                            cx.notify();
                        });
                        true
                    })
                    .backdrop(
                        AlertDialogBackdrop::new()
                            .absolute()
                            .inset_0()
                            .bg(super::example_rgb(0x000000))
                            .opacity(0.18),
                    )
                    .popup(
                        AlertDialogPopup::new()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .w_72()
                                    .p_3()
                                    .bg(super::example_rgb(0xffffff))
                                    .border_1()
                                    .border_color(super::example_rgb(0x171717))
                                    .child(
                                        AlertDialogTitle::new()
                                            .child("Delete project?"),
                                    )
                                    .child(
                                        AlertDialogDescription::new()
                                            .mt_2()
                                            .text_xs()
                                            .text_color(super::example_rgb(0x525252))
                                            .child(
                                                "This permanently deletes Acme Studio and all of its data.",
                                            ),
                                    )
                                    .child(
                                        div()
                                            .mt_3()
                                            .flex()
                                            .justify_end()
                                            .gap_2()
                                            .child(AlertDialogCancel::new().child(
                                                Button::new("cancel-delete")
                                                    .px_3()
                                                    .h_7()
                                                    .flex()
                                                    .items_center()
                                                    .text_xs()
                                                    .border_1()
                                                    .border_color(super::example_rgb(0xd4d4d4))
                                                    .on_click(move |_, _, cx| {
                                                        _ = cancel_entity.update(cx, |this, cx| {
                                                            this.alert_dialog_open = false;
                                                            cx.notify();
                                                        });
                                                    })
                                                    .child("Cancel"),
                                            ))
                                            .child(AlertDialogAction::new().child(
                                                Button::new("confirm-delete")
                                                    .px_3()
                                                    .h_7()
                                                    .flex()
                                                    .items_center()
                                                    .text_xs()
                                                    .border_1()
                                                    .border_color(super::example_rgb(0x171717))
                                                    .bg(super::example_rgb(0x171717))
                                                    .text_color(super::example_rgb(0xffffff))
                                                    .on_click(move |_, _, cx| {
                                                        _ = action_entity.update(cx, |this, cx| {
                                                            this.alert_dialog_open = false;
                                                            cx.notify();
                                                        });
                                                    })
                                                    .child("Delete"),
                                            )),
                                    ),
                            ),
                    ),
            )
    }
}
```

## 可访问性

将焦点限制在模态层内，提供标题与说明，并确保取消操作始终可用。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
