---
url: /zh-CN/base/primitives/sheet.md
description: 从边缘进入并管理关闭与焦点的模态界面。
---

# Sheet

从边缘进入并管理关闭与焦点的模态界面。

和所有 GPUI Base 原语一样，Sheet 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- sheet
```

## 导入

```rust
use gpui_kit::base::{Sheet};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/sheet.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/sheet.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

打开状态由应用或触发器控制；退出结束前内容可继续挂载。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use gpui::relative;

use super::*;

impl BaseShowcase {
    pub(in super::super) fn sheet(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let open = self.sheet_open;
        let entity = cx.entity().downgrade();
        let open_sheet = entity.clone();
        let trigger = Button::new("open-sheet")
            .h_7()
            .px_2()
            .text_xs()
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(super::example_rgb(0x171717))
            .bg(super::example_rgb(0xffffff))
            .child("Open settings")
            .on_click(move |_, _, cx| {
                _ = open_sheet.update(cx, |this, cx| {
                    this.sheet_open = true;
                    cx.notify();
                });
            });

        div()
            .size_full()
            .min_h_64()
            .text_xs()
            .flex()
            .items_center()
            .justify_center()
            .child(trigger)
            .when(open, |this| {
                this.child(
                    Sheet::new(cx)
                        .request_close({
                            let entity = entity.clone();
                            move |_, cx| {
                                _ = entity.update(cx, |this, cx| {
                                    this.sheet_open = false;
                                    cx.notify();
                                });
                            }
                        })
                        .overlay(
                            div()
                                .absolute()
                                .inset_0()
                                .bg(super::example_rgb(0x000000))
                                .opacity(0.15),
                        )
                        .surface(
                            div()
                                .absolute()
                                .right_0()
                                .top_0()
                                .h_full()
                                .w(px(210.))
                                .p_3()
                                .bg(super::example_rgb(0xffffff))
                                .border_1()
                                .border_color(super::example_rgb(0x171717))
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child("Settings"),
                                )
                                .child(
                                    div().mt_4().child("Workspace name").child(
                                        div()
                                            .mt_1()
                                            .h_7()
                                            .px_2()
                                            .flex()
                                            .items_center()
                                            .border_1()
                                            .border_color(super::example_rgb(0xa3a3a3))
                                            .child("Acme Studio"),
                                    ),
                                )
                                .child(
                                    div()
                                        .mt_2()
                                        .text_color(super::example_rgb(0x525252))
                                        .child("Update the workspace preferences for your team."),
                                )
                                .child(
                                    div()
                                        .mt_4()
                                        .py_1()
                                        .border_t_1()
                                        .border_color(super::example_rgb(0xd4d4d4))
                                        .child("Notifications  ·  Enabled"),
                                )
                                .child(
                                    div().mt_3().flex().justify_end().child(
                                        Button::new("close-sheet")
                                            .h_7()
                                            .line_height(relative(1.))
                                            .px_3()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .bg(gpui::black())
                                            .text_color(gpui::white())
                                            .child("Done")
                                            .on_click({
                                                let entity = entity.clone();
                                                move |_, _, cx| {
                                                    _ = entity.update(cx, |this, cx| {
                                                        this.sheet_open = false;
                                                        cx.notify();
                                                    });
                                                }
                                            }),
                                    ),
                                ),
                        ),
                )
            })
    }
}
```

## 可访问性

按模态层处理焦点与背景交互，提供标题和明确关闭方式。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。
