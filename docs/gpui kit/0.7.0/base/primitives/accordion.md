---
url: /zh-CN/base/primitives/accordion.md
description: 由可独立设置样式的标题、触发器和面板组成的折叠组。
---

# Accordion

由可独立设置样式的标题、触发器和面板组成的折叠组。

和所有 GPUI Base 原语一样，Accordion 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- accordion
```

## 导入

```rust
use gpui_kit::base::{Accordion, AccordionHeader, AccordionItem, AccordionPanel, AccordionTrigger};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/accordion.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/accordion.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

管理每一项的展开状态，并在触发器激活时更新它。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;

impl BaseShowcase {
    pub(in super::super) fn accordion(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let items = [
            (
                "What is GPUI Base?",
                "Unstyled, accessible primitives for building native GPUI interfaces.",
            ),
            (
                "Can I bring my own theme?",
                "Yes. Every visual detail remains application-owned.",
            ),
            (
                "Does it support keyboard input?",
                "Focus, activation, and semantic state are built into the primitives.",
            ),
        ];

        Accordion::new("example-accordion")
            .w(px(270.))
            .border_t_1()
            .border_color(super::example_rgb(0xd4d4d4))
            .children(
                items
                    .into_iter()
                    .enumerate()
                    .map(|(index, (question, answer))| {
                        let open = self.accordion_items[index];
                        let entity = cx.entity().downgrade();
                        AccordionItem::new()
                            .open(open)
                            .header(AccordionHeader::new(
                                AccordionTrigger::new(format!("accordion-trigger-{index}"))
                                    .on_change(move |next, _, _, cx| {
                                        _ = entity.update(cx, |this, cx| {
                                            this.accordion_items[index] = next;
                                            cx.notify();
                                        });
                                    })
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .h_7()
                                    .border_b_1()
                                    .border_color(super::example_rgb(0xd4d4d4))
                                    .text_xs()
                                    .child(question)
                                    .child(
                                        div()
                                            .text_color(super::example_rgb(0x737373))
                                            .child(if open { "−" } else { "+" }),
                                    ),
                            ))
                            .panel(
                                AccordionPanel::new()
                                    .px_1()
                                    .py_1()
                                    .border_b_1()
                                    .border_color(super::example_rgb(0xd4d4d4))
                                    .text_xs()
                                    .text_color(super::example_rgb(0x525252))
                                    .child(answer),
                            )
                    }),
            )
    }
}
```

## 可访问性

让触发器可聚焦、可用键盘操作，并向辅助技术暴露展开状态。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/accordion）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
