---
url: /zh-CN/base/primitives/progress.md
description: 用于报告任务完成度的可组合轨道与指示器。
---

# Progress

用于报告任务完成度的可组合轨道与指示器。

和所有 GPUI Base 原语一样，Progress 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- progress
```

## 导入

```rust
use gpui_kit::base::{Progress, ProgressIndicator, ProgressTrack};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/progress.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/progress.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

应用提供规范化进度值；视觉宽度由该值派生。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use gpui::{IntoElement, ParentElement as _, Styled as _, div, px};
use gpui_base::{Progress, ProgressIndicator, ProgressTrack};

use super::super::BaseShowcase;

impl BaseShowcase {
    pub(in super::super) fn progress(&self) -> impl IntoElement {
        div()
            .w_64()
            .flex()
            .flex_col()
            .gap_2()
            .text_xs()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child("Uploading assets")
                    .child("68%"),
            )
            .child(
                Progress::new("example-progress").value(68.).child(
                    ProgressTrack::new()
                        .w_full()
                        .h(px(7.))
                        .border_1()
                        .border_color(super::example_rgb(0x171717))
                        .child(
                            ProgressIndicator::new()
                                .w(px(177.))
                                .h_full()
                                .bg(super::example_rgb(0x171717)),
                        ),
                ),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .text_sm()
                    .text_color(super::example_rgb(0x737373))
                    .child("Optimizing bundle")
                    .child("32%"),
            )
            .child(
                Progress::new("example-progress-secondary")
                    .value(32.)
                    .child(
                        ProgressTrack::new()
                            .w_full()
                            .h(px(6.))
                            .border_1()
                            .border_color(super::example_rgb(0xa3a3a3))
                            .child(
                                ProgressIndicator::new()
                                    .w(px(83.))
                                    .h_full()
                                    .bg(super::example_rgb(0x737373)),
                            ),
                    ),
            )
    }
}
```

## 可访问性

暴露进度角色、当前值、最小值、最大值及有意义的文本标签。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/progress）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
