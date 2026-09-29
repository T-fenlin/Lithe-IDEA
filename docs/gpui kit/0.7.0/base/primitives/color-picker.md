---
url: /zh-CN/base/primitives/color-picker.md
description: 为自定义颜色选择器提供状态与交互基础。
---

# Color Picker

为自定义颜色选择器提供状态与交互基础。

和所有 GPUI Base 原语一样，Color Picker 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- color-picker
```

## 导入

```rust
use gpui_kit::base::{ColorPicker, ColorPickerEvent, ColorPickerState, ColorSwatch};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/color-picker.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/color-picker.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

ColorPickerState 保存颜色，ColorPickerEvent 报告用户变更。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;
use gpui::{Focusable as _, Hsla, MouseButton};

impl BaseShowcase {
    pub(in super::super) fn color_picker(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // A builder-supplied default cannot reach the hex field and the sliders
        // without a window, so flush it on the first render.
        self.color_picker
            .update(cx, |state, cx| state.sync_pending_value(window, cx));

        let picker = self.color_picker.read(cx);
        let open = picker.is_open();
        let selected = picker.value();
        let displayed = picker
            .displayed_color()
            .unwrap_or(super::example_rgb(0x171717).into());
        let hex = picker.hex_input().read(cx).value();
        let focus_handle = picker.focus_handle(cx);
        let hex_input = picker.hex_input().clone();
        let state = self.color_picker.clone();

        let trigger_state = state.clone();
        let trigger = div()
            .id("color-trigger")
            .w_full()
            .h_7()
            .px_2()
            .flex()
            .items_center()
            .gap_2()
            .border_1()
            .border_color(super::example_rgb(0x171717))
            .bg(super::example_rgb(0xffffff))
            .on_click(move |_, _, cx| {
                trigger_state.update(cx, |state, cx| state.toggle_open(cx));
            })
            .child(
                div()
                    .size(px(14.))
                    .bg(displayed)
                    .border_1()
                    .border_color(super::example_rgb(0x171717)),
            )
            .child(hex)
            .child(div().flex_1())
            .child(super::chevron(open));

        let swatches = div().flex().gap_1().children(
            [0xdc2626u32, 0xd97706, 0x16a34a, 0x2563eb, 0x7c3aed]
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    let color: Hsla = super::example_rgb(value).into();
                    let hover_state = state.clone();
                    let click_state = state.clone();
                    ColorSwatch::new(("swatch", index), color)
                        .selected(selected == Some(color))
                        .size(px(24.))
                        .bg(color)
                        .border_1()
                        .border_color(if selected == Some(color) {
                            super::example_rgb(0x171717)
                        } else {
                            super::example_rgb(0xffffff)
                        })
                        // Hovering previews without committing; leaving restores
                        // the committed color.
                        .on_hover(move |color, entered, window, cx| {
                            hover_state.update(cx, |state, cx| {
                                if entered {
                                    state.preview_color(color, window, cx);
                                } else {
                                    state.clear_preview(window, cx);
                                }
                            });
                        })
                        .on_click(move |color, _, window, cx| {
                            click_state
                                .update(cx, |state, cx| state.select_color(color, window, cx));
                        })
                }),
        );

        let content = div()
            .w(px(220.))
            .mt_1()
            .p_2()
            .flex()
            .flex_col()
            .gap_2()
            .border_1()
            .border_color(super::example_rgb(0x171717))
            .bg(super::example_rgb(0xffffff))
            .child(swatches)
            .child(
                InputBase::new("color-hex-input")
                    .w_full()
                    .h_7()
                    .px_2()
                    .border_1()
                    .border_color(super::example_rgb(0xd4d4d4))
                    .styles(|styles| {
                        styles.focused(|style| style.border_color(super::example_rgb(0x171717)))
                    })
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        hex_input.update(cx, |input, cx| input.focus(window, cx));
                    })
                    .child(picker.hex_input().clone()),
            );

        let open_state = state.clone();
        let root = ColorPicker::new("example-color-picker")
            .open(open)
            .track_focus(&focus_handle)
            .accessibility_label("Brand color")
            .on_open_change(move |open, _, cx| {
                open_state.update(cx, |state, cx| state.set_open(open, cx));
            })
            .w(px(220.))
            .text_xs()
            .child(trigger);

        Popup::new("example-color-picker-popup", root).when(open, |this| this.content(content))
    }
}
```

## 可访问性

为色样提供文本名称或数值，并确保键盘用户可以完成选择。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/color-picker）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
