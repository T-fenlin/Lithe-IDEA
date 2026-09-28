---
url: /zh-CN/base/text-selection.md
description: 在多个自绘文本参与者之间实现窗口级文本选择。
---

# 文本选择

`gpui-base` 的文本选择基础设施允许一次拖拽跨越多个独立绘制的文本块，同时保留每个参与者自己的布局与绘制逻辑。它适合文档、消息流和其他由多个自定义元素组成、但用户期望像连续文本一样选择的界面。

## 开始使用

核心由窗口级 `TextSelectionState`、稳定的 `SelectableTextHandle`、prepaint 阶段注册的几何信息，以及把全局选择投影到各文本 run 的辅助 API 组成。

## 工作方式

每个参与者在 prepaint 时报告文本、屏幕几何和逻辑顺序。窗口状态根据指针锚点与当前点计算跨参与者的选择区间；绘制时，各参与者只查询落在自身范围内的切片并绘制选中背景。状态更新发生在事件回调中，不在 `render` 中写入。

## 安装窗口元素

在窗口内容的稳定外层安装选择宿主，使它能够接收拖拽、释放和复制操作。宿主应覆盖所有需要共同选择的参与者，但不应改变它们的布局或样式。

## 创建稳定句柄

每个参与者的 `SelectableTextHandle` 应保存在 entity 中或由稳定数据键派生。不要在每次渲染时创建新身份，否则拖拽途中重绘会丢失锚点、逻辑顺序或选择投影。

## 在 prepaint 注册几何

文本最终布局完成后注册边界、行与字符位置。注册顺序必须与用户看到和辅助技术读取的逻辑顺序一致。动态插入、删除或移动参与者时，用稳定身份更新对应记录。

## 把选择投影到文本 run

绘制各 run 前查询当前窗口选择，取得与本地文本相交的字节或字符范围，再把范围转换为文本系统需要的高亮几何。注意 UTF-8 边界，不要把字节偏移当作字符索引。

## 完整 Rust 示例

```bash
cargo run -p gpui-base-examples -- text-selection
```

```rust
use gpui::{Context, IntoElement, ParentElement as _, Styled as _, Window};
#[cfg(test)]
use gpui_base::ElementExt as _;
use gpui_base::{SelectableText, TextSelection};

use super::*;

const PRODUCT_PARAGRAPH: &str = "Selection should feel like a natural part of reading a product brief. Start in this paragraph, continue into the next renderer, and GPUI preserves the document order while every frame supplies fresh geometry for the same stable selection handle.";
const IMPLEMENTATION_PARAGRAPH: &str = "This second paragraph is deliberately long enough to wrap in the showcase. Drag across the boundary to see one continuous highlight, then use the platform copy shortcut to confirm that the copied result follows the visible reading order rather than renderer ownership.";
const INTERNATIONAL_PARAGRAPH: &str = "International text should remain predictable when a line mixes café, déjà vu, Kraków, naïve, and résumé. Resize the window or drag across several wrapped lines; UTF-8 byte ranges still map back to the correct glyphs without splitting a character.";

impl BaseShowcase {
    pub(in super::super) fn text_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.text_selection_active = TextSelection::has_selection(window, cx);
        self.text_selection_text = TextSelection::selected_text(window, cx);

        let active = self.text_selection_active;
        let selected_text = if active {
            self.text_selection_text.clone()
        } else {
            "Drag across any paragraphs to select text.".to_owned()
        };
        let entity = cx.entity().downgrade();

        let footer = div()
            .id("text-selection-footer")
            .h(px(150.))
            .flex_none()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(super::example_rgb(0xf5f5f5))
            .border_1()
            .border_color(super::example_rgb(0xe5e5e5))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(if active {
                        "Selection active"
                    } else {
                        "No selection"
                    }),
            )
            .child(
                div()
                    .id("text-selection-preview")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .text_color(super::example_rgb(0x525252))
                    .child(selected_text),
            )
            .child(
                Button::new("clear-text-selection")
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_center()
                    .self_start()
                    .border_1()
                    .border_color(super::example_rgb(0x171717))
                    .child("Clear selection")
                    .on_click(move |_, window, cx| {
                        TextSelection::clear(window, cx);
                        _ = entity.update(cx, |this, cx| {
                            this.text_selection_active = false;
                            this.text_selection_text.clear();
                            cx.notify();
                        });
                    }),
            );
        #[cfg(test)]
        let footer = {
            let bounds = self.text_selection_footer_bounds.clone();
            footer.on_prepaint(move |value, _, _| *bounds.borrow_mut() = Some(value))
        };

        div()
            .id("text-selection-example")
            .w(px(620.))
            .max_w_full()
            .h(px(520.))
            .max_h_full()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .id("text-selection-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.text_selection_scroll)
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(
                                SelectableText::with_handle(
                                    "selection-heading",
                                    self.text_selection_handles[0].clone(),
                                    "Text selection across renderers",
                                )
                                .document_order(0),
                            ),
                    )
                    .child(
                        div()
                            .text_color(super::example_rgb(0x525252))
                            .line_height(px(22.))
                            .child(
                                SelectableText::with_handle(
                                    "selection-product",
                                    self.text_selection_handles[1].clone(),
                                    PRODUCT_PARAGRAPH,
                                )
                                .document_order(1),
                            ),
                    )
                    .child(
                        div()
                            .text_color(super::example_rgb(0x525252))
                            .line_height(px(22.))
                            .child(
                                SelectableText::with_handle(
                                    "selection-implementation",
                                    self.text_selection_handles[2].clone(),
                                    IMPLEMENTATION_PARAGRAPH,
                                )
                                .document_order(2),
                            ),
                    )
                    .child(
                        div()
                            .text_color(super::example_rgb(0x525252))
                            .line_height(px(22.))
                            .child(
                                SelectableText::with_handle(
                                    "selection-international",
                                    self.text_selection_handles[3].clone(),
                                    INTERNATIONAL_PARAGRAPH,
                                )
                                .document_order(3),
                            ),
                    ),
            )
            .child(footer)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{TestAppContext, point, px};

    use crate::showcase::BaseShowcase;

    #[gpui::test]
    fn text_selection_footer_stays_fixed_when_document_scrolls(cx: &mut TestAppContext) {
        let (view, window) =
            cx.add_window_view(|window, cx| BaseShowcase::new("text-selection", window, cx));
        window.update(|window, cx| window.draw(cx).clear(cx));

        let (footer_bounds, scroll) = view.read_with(window, |view, _| {
            (
                view.text_selection_footer_bounds
                    .borrow()
                    .expect("footer should be painted"),
                view.text_selection_scroll.clone(),
            )
        });
        scroll.set_offset(point(px(0.), px(-80.)));
        view.update(window, |_, cx| cx.notify());
        window.update(|window, cx| window.draw(cx).clear(cx));

        let scrolled_footer_bounds = view.read_with(window, |view, _| {
            view.text_selection_footer_bounds
                .borrow()
                .expect("footer should be painted after scrolling")
        });
        assert_eq!(scrolled_footer_bounds, footer_bounds);
    }
}
```

## 查询与控制窗口选择

应用可以读取当前选中文本、主动清除选择，并把复制命令连接到窗口状态。程序化更新后调用 `cx.notify()`，让所有受影响参与者重绘。

### 触摸选择

在参与者上长按会选中手指下的单词；抬起手指后，这个选区成为*触摸选区*：两端各带一个拖动 handle，并附带编辑菜单。手势和拖动由 Base 负责；展示层根据 `TextSelectionSnapshot` 之外的 `TouchSelectionSnapshot` 绘制 handle 和菜单，它以窗口坐标提供选区两端的光标行框。

通过 `TextSelection::touch_selection` 读取快照，用 `TextSelection::observe_touch_selection` 在快照变化时重绘；`begin_edge_drag`、`update_edge_drag`、`end_edge_drag` 拖动其中一端，`select_all` 全选被按下的参与者，`close_edit_menu` 在菜单自身的命令执行后关闭菜单。handle 由参与者自己绘制，位于它在绘制顺序中的位置，因此盖住文字的东西也会盖住 handle：在 prepaint 调用 `TextSelectionHandle::prepaint_touch_handles`（插入手指可按的 hitbox），在 paint 末尾、`register` 之后调用 `TextSelectionHandle::paint_touch_handles` 并传入选区颜色；`TextView` 已完成这两步。参与者通过 `TextSelectionRegistration::with_selection_edges` 上报其选区两端的绘制位置。绘制菜单的一方需要在每帧 paint 时调用 `TextSelection::register_touch_ui` 登记菜单 bounds，这样落在菜单上的按压不会清除它所属的选区；GPUI Component 的 `Root` 负责绘制菜单。

## 高级参与者适配器

自定义文本布局可以实现参与者接口，提供命中测试、范围投影和文本提取。适配器应只桥接已有布局数据，避免在指针移动热路径中重新排版或分配大型缓冲区。

## 用 scope 隔离模态内容

Dialog、Sheet 等模态内容应使用独立 scope，避免一次选择跨越背景与前景。模态关闭后恢复原 scope，不要让已卸载参与者留在窗口注册表中。

## 集成检查清单

- 窗口只安装一个相应 scope 的选择宿主。
- 参与者身份和逻辑顺序跨渲染稳定。
- 只在 prepaint 注册最终几何，并正确处理 UTF-8。
- 拖拽更新有界，不在 render 中修改状态。
- 验证跨块拖拽、反向选择、复制、动态内容、滚动和模态隔离。
