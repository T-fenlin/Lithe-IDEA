---
url: /zh-CN/base/primitives/avatar.md
description: 带可组合后备内容的人物或实体图像。
---

# Avatar

带可组合后备内容的人物或实体图像。

和所有 GPUI Base 原语一样，Avatar 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- avatar
```

## 导入

```rust
use gpui_kit::base::{Avatar, AvatarFallback, AvatarImage};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/avatar.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/avatar.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

图像加载失败时显示后备内容。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;

impl BaseShowcase {
    pub(in super::super) fn avatar(&self) -> impl IntoElement {
        div().flex().items_start().gap_2().children(
            [
                ("AM", 0xf5f5f5),
                ("JL", 0xe5e5e5),
                ("SK", 0xd4d4d4),
                ("+3", 0xffffff),
            ]
            .into_iter()
            .map(|(initials, background)| {
                Avatar::new()
                    .size(px(34.))
                    .overflow_hidden()
                    .border_1()
                    .border_color(super::example_rgb(0xa3a3a3))
                    .fallback(
                        AvatarFallback::new()
                            .flex()
                            .size_8()
                            .items_center()
                            .justify_center()
                            .bg(super::example_rgb(background))
                            .text_xs()
                            .text_color(super::example_rgb(0x262626))
                            .child(initials),
                    )
            }),
        )
    }
}
```

## 可访问性

为有信息含义的图像提供替代文本；纯装饰图像应从可访问树中隐藏。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/avatar）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
