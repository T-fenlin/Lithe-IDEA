---
url: /zh-CN/base/primitives/tree.md
description: 显式管理展开与选择状态的虚拟化层级列表。
---

# Tree

显式管理展开与选择状态的虚拟化层级列表。

和所有 GPUI Base 原语一样，Tree 只提供行为和语义结构，不规定产品视觉语言。请使用 GPUI 样式并组合导出的部件，使其符合你的设计系统。

## 示例

原生示例和页面上方的 WASM 预览共用同一份实现：

```bash
cargo run -p gpui-base-examples -- tree
```

## 导入

```rust
use gpui_kit::base::{Tree, TreeItem, TreeState};
```

## 结构与 API

示例组合上述公开类型。GPUI 的标准样式和事件 trait 负责表现，Base 类型负责交互结构。权威实现位于 [`components/tree.rs`](https://github.com/longbridge/gpui-kit/blob/main/crates/base/examples/showcase/components/tree.rs)，原生与浏览器预览编译的是同一文件。

## 状态与事件

TreeState 保存展开、选择和虚拟滚动状态；稳定节点 ID 用于跨渲染保留身份。

受控状态应保存在父渲染类型或 GPUI entity 中；在回调中更新并调用 `cx.notify()`，不要在每次渲染时重建持久 entity。

## 完整 Rust 示例

```rust
use super::*;
use gpui::{Image, ImageFormat, StyleRefinement, img};
use std::sync::Arc;

const CHEVRON_RIGHT_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none"><path d="m6 3.5 4.5 4.5L6 12.5" stroke="#171717" stroke-width="1.5" stroke-linecap="square" stroke-linejoin="miter"/></svg>"##;
const CHEVRON_DOWN_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none"><path d="m3.5 6 4.5 4.5L12.5 6" stroke="#171717" stroke-width="1.5" stroke-linecap="square" stroke-linejoin="miter"/></svg>"##;

impl BaseShowcase {
    pub(in super::super) fn tree(&self) -> impl IntoElement {
        Tree::new(&self.tree)
            .w_64()
            .h_48()
            .list_style(StyleRefinement::default().flex_grow_1().size_full())
            .relative()
            .text_sm()
            .border_1()
            .border_color(super::example_rgb(0xd4d4d4))
            .py_1()
            .item(|_, entry, state, _, _| {
                let depth = entry.depth();
                let icon = entry.is_folder().then(|| {
                    let bytes = if entry.is_expanded() {
                        CHEVRON_DOWN_SVG
                    } else {
                        CHEVRON_RIGHT_SVG
                    };
                    img(Arc::new(Image::from_bytes(
                        ImageFormat::Svg,
                        bytes.to_vec(),
                    )))
                    .size_3()
                    .flex_none()
                });
                div()
                    .h_8()
                    .mx_1()
                    .px_2()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(state.is_selected(), |this| {
                        this.bg(super::example_rgb(0xf0f0f0))
                    })
                    .when(depth > 0, |this| {
                        this.child(div().flex_none().w(px(depth as f32 * 12.)))
                    })
                    .child(
                        div()
                            .size_3()
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .children(icon),
                    )
                    .child(entry.item().label.clone())
                    .into_any_element()
            })
    }
}
```

## 可访问性

暴露层级、展开与选中状态，并保留方向键、Home/End 和类型导航。

## 注意事项

在支持的位置使用稳定元素 ID，并在消费端设计系统中验证焦点、悬停、按下、选中、禁用、减少动态效果和高对比度状态。

> 文档许可：GPUI Kit 有权授权的原创正文与图示另以 CC BY 4.0 提供。复制或改编时请署名 GPUI Kit，链接原文（https://gpui-kit.com/zh-CN/base/primitives/tree）及 https://creativecommons.org/licenses/by/4.0/，并注明修改。代码示例与软件源码采用 Apache-2.0；第三方内容保留原许可；既有 Apache-2.0 使用权不受影响。
