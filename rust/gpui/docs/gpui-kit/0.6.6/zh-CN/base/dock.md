---
url: /zh-CN/base/dock.md
description: 用持久布局树、面板生命周期和可替换渲染器构建停靠工作区。
---

# Dock

Dock 是 `gpui-base` 的无样式停靠布局基础。它把可持久化布局模型、面板生命周期、拖放与查询行为放在 Base，把标签、边框、图标、空状态和其他产品视觉交给应用提供的 renderer。

## 模型

布局是一棵由分栏、标签组、面板和边缘 dock 组成的树。稳定面板 ID 是移动、恢复、查询和事件关联的基础；不要用当前位置作为身份。

### 关键类型

`DockArea` 承载工作区和交互状态；布局节点描述结构；面板 trait/工厂负责由持久数据创建内容；renderer trait 把模型映射为应用外观。

## 开始使用

先创建持久 `DockArea` entity，注册面板工厂和 renderer，再安装初始布局。渲染期间只读取模型；添加、移动、关闭、缩放和锁定操作放在回调中执行并通知 GPUI。

## 描述布局

用嵌套节点声明水平/垂直分栏、标签组和面板。分栏尺寸属于相邻槽位的约束数据，不属于面板视觉样式。载入外部或旧版本布局后先规范化：移除空节点、折叠无意义嵌套并修正非法比例，同时保留稳定 ID。

## 面板与生命周期

面板提供身份、标题/元数据、渲染内容以及可选的激活、停用、关闭、保存和恢复 hook。hook 可能发生在拖放、布局替换、窗口关闭或恢复期间，必须保持幂等，不能依赖一次 render 创建的临时状态。资源与订阅跟随持久 entity 生命周期。

## DockArea 操作

可以整体安装布局，也可以向指定标签组添加面板、在节点间移动面板、打开或关闭边缘 dock、缩放当前面板、锁定结构，以及按 ID 查询面板或节点。修改前验证目标仍存在，因为事件发生后布局可能已经变化。

## 直接编辑布局树

直接编辑适合批量迁移和恢复，不适合普通交互。编辑后必须规范化并通过 `DockArea` 安装，使索引、焦点、事件和渲染状态同步更新；不要绕过宿主只修改一份外部副本。

## 提供外观

`DockAreaRenderer` 负责工作区边框、边缘 dock 和空状态；`TabGroupRenderer` 负责标签、活动态、关闭入口和标签拖动反馈。renderer 接收只读上下文和明确回调，不应拥有领域状态，也不应在 render 中修改布局。

## 拖放

拖动数据使用稳定面板身份。命中测试决定插入标签、分割方向或边缘 dock，提交前再次验证源与目标。

## 事件与持久化

订阅布局、活动面板、面板关闭和拖放事件来同步应用状态。持久化稳定 ID、节点种类、分栏比例、标签顺序、活动项、边缘 dock 与面板自有数据，不要序列化 GPUI entity、焦点句柄或 renderer。恢复时容忍未知面板类型，并对版本化数据做迁移和规范化。

## 架构取舍

Dock 的数据模型比单一分栏组件更重，但换来可查询、可移动、可持久化和可替换表现。Base 使用中性的 tree/panel/tab 命名，产品可以在 renderer 和面板工厂层映射成自己的术语。

## 可运行示例

```bash
cargo run -p gpui-base-examples -- dock
```

```rust
use super::*;

use gpui::{
    AnyElement, Axis, Div, Entity, EventEmitter, FocusHandle, Focusable, MouseButton,
    MouseMoveEvent, MouseUpEvent, SharedString, Stateful, rgba,
};
use std::cell::RefCell;

const SURFACE: u32 = 0xffffff;
const CHROME: u32 = 0xf4f4f5;
const BORDER: u32 = 0xd4d4d8;
const MUTED: u32 = 0x71717a;
const ACCENT: u32 = 0x2563eb;
const DROP_TARGET: u32 = 0x2563eb33;

const TAB_BAR_HEIGHT: gpui::Pixels = px(26.);
const RESIZE_STRIP: gpui::Pixels = px(4.);

/// One dockable view. Its only obligation to base is a stable name; the title
/// and body are this example's own, and reach the skin through a downcast of
/// the handle base was given.
struct ShowcasePanel {
    name: &'static str,
    title: SharedString,
    body: SharedString,
    focus_handle: FocusHandle,
}

impl ShowcasePanel {
    fn new(
        name: &'static str,
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self {
            name,
            title: title.into(),
            body: body.into(),
            focus_handle: cx.focus_handle(),
        })
    }
}

impl Panel for ShowcasePanel {
    fn panel_name(&self) -> &'static str {
        self.name
    }
}

impl EventEmitter<PanelEvent> for ShowcasePanel {}

impl Focusable for ShowcasePanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ShowcasePanel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .text_xs()
            .child(div().child(self.title.clone()))
            .child(
                div()
                    .text_color(super::example_rgb(MUTED))
                    .child(self.body.clone()),
            )
    }
}

/// The preview that follows the cursor while a tab is dragged.
///
/// Base's own `DragPanel` renders nothing, because a preview is appearance.
struct DragPreview {
    title: SharedString,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .text_xs()
            .bg(super::example_rgb(SURFACE))
            .text_color(super::example_rgb(ACCENT))
            .border_1()
            .border_color(super::example_rgb(ACCENT))
            .child(self.title.clone())
    }
}

/// A panel's title, recovered across the renderer seam.
///
/// Base carries every panel as `Arc<dyn PanelView>`, which knows its
/// `panel_name` and nothing else — a title is presentation, and base has no
/// opinion about it. A skin gets one back by downcasting to the concrete
/// handle base was handed.
fn panel_title(panel: &Arc<dyn PanelView>, cx: &App) -> SharedString {
    panel
        .as_any()
        .downcast_ref::<Entity<ShowcasePanel>>()
        .map(|panel| panel.read(cx).title.clone())
        .unwrap_or_else(|| panel.panel_name(cx).into())
}

/// Everything this example draws. Base draws none of it.
#[derive(Clone, Default)]
struct ShowcaseDockSkin {
    /// The dock a resize drag is currently sizing, captured on mouse down.
    ///
    /// A resize follows the pointer anywhere in the area, not only over the
    /// strip, so the listener that tracks it sits on the area frame — which is
    /// not handed a `DockContext`. The strip stashes its own here instead.
    resizing: Rc<RefCell<Option<DockContext>>>,
}

impl ShowcaseDockSkin {
    /// The strip on a dock's inner edge that resizes it: a wide hit area with
    /// a hairline inside, so the edge reads as a line rather than a bar.
    fn render_resize_strip(&self, dock: &DockContext) -> impl IntoElement {
        let placement = dock.placement();
        let dock = dock.clone();
        let resizing = self.resizing.clone();

        div()
            .absolute()
            .flex()
            .items_center()
            .justify_center()
            .map(|this| match placement {
                DockPlacement::Left => this
                    .top_0()
                    .right_0()
                    .h_full()
                    .w(RESIZE_STRIP)
                    .cursor_col_resize(),
                DockPlacement::Bottom => this
                    .top_0()
                    .left_0()
                    .w_full()
                    .h(RESIZE_STRIP)
                    .cursor_row_resize(),
                _ => this
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(RESIZE_STRIP)
                    .cursor_col_resize(),
            })
            .child(
                div()
                    .bg(super::example_rgb(BORDER))
                    .map(|line| match placement {
                        DockPlacement::Bottom => line.h(px(1.)).w_full(),
                        _ => line.w(px(1.)).h_full(),
                    }),
            )
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                cx.stop_propagation();
                *resizing.borrow_mut() = Some(dock.clone());
            })
    }
}

impl DockAreaRenderer for ShowcaseDockSkin {
    fn frame(&self, _: &mut Window, _: &mut App) -> Stateful<Div> {
        let dragging = self.resizing.clone();
        let finished = self.resizing.clone();

        div()
            .id("showcase-dock")
            .size_full()
            .flex()
            .flex_row()
            .overflow_hidden()
            .bg(super::example_rgb(CHROME))
            .on_mouse_move(move |event: &MouseMoveEvent, window, cx| {
                // Cloned out before the call, so the borrow is released before
                // resizing reaches back into another frame reading this cell.
                let dock = dragging.borrow().clone();
                let Some(dock) = dock else {
                    return;
                };
                dock.resize_to(event.position, window, cx);
            })
            .on_mouse_up(MouseButton::Left, move |_: &MouseUpEvent, _, _| {
                finished.borrow_mut().take();
            })
    }

    fn center_frame(&self, _: &mut Window, _: &mut App) -> Stateful<Div> {
        div()
            .id("showcase-dock-center")
            .flex()
            .flex_1()
            .flex_col()
            .overflow_hidden()
    }

    fn split_frame(&self, node: NodeId, _: Axis, _: &mut Window, _: &mut App) -> Stateful<Div> {
        div()
            .id(("showcase-dock-split", node.as_u64()))
            .size_full()
            .flex_1()
            .min_h(px(0.))
            .overflow_hidden()
    }

    /// Only the paint: base keeps the hit area, the cursor and the drag.
    fn render_split_handle(
        &self,
        handle: &ResizeHandleContext,
        _: &mut Window,
        _: &mut App,
    ) -> Option<AnyElement> {
        Some(
            div()
                .bg(super::example_rgb(if handle.is_active() {
                    ACCENT
                } else {
                    BORDER
                }))
                .map(|line| match handle.axis() {
                    Axis::Horizontal => line.w(px(1.)).h_full(),
                    Axis::Vertical => line.h(px(1.)).w_full(),
                })
                .into_any_element(),
        )
    }

    fn render_dock(
        &self,
        dock: &DockContext,
        content: AnyElement,
        _: &mut Window,
        _: &mut App,
    ) -> AnyElement {
        // A closed dock takes no space; the toolbar is what brings it back.
        if !dock.is_open() {
            return div().into_any_element();
        }

        div()
            .flex()
            .flex_none()
            .relative()
            .overflow_hidden()
            .map(|this| match dock.placement() {
                DockPlacement::Bottom => this.w_full().h(dock.size()).flex_col(),
                _ => this.h_full().w(dock.size()).flex_row(),
            })
            .child(content)
            .child(self.render_resize_strip(dock))
            .into_any_element()
    }

    fn tab_group_renderer(&self) -> Rc<dyn TabGroupRenderer> {
        Rc::new(self.clone())
    }
}

impl TabGroupRenderer for ShowcaseDockSkin {
    fn frame(&self, _: &TabGroupContext, _: &mut Window, _: &mut App) -> Stateful<Div> {
        div()
            .id("showcase-tab-group")
            .size_full()
            .flex()
            .flex_col()
            .min_h(px(0.))
            .overflow_hidden()
            .bg(super::example_rgb(SURFACE))
    }

    fn content_frame(&self, _: &TabGroupContext, _: &mut Window, _: &mut App) -> Stateful<Div> {
        // Relative, because the drop indicator is positioned against it.
        div()
            .id("showcase-tab-content")
            .relative()
            .flex_1()
            .min_h(px(0.))
            .overflow_hidden()
    }

    fn render_tab_bar(&self, group: &TabGroupContext, _: &mut Window, cx: &mut App) -> AnyElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .h(TAB_BAR_HEIGHT)
            .flex_none()
            .overflow_hidden()
            .bg(super::example_rgb(CHROME))
            .border_b_1()
            .border_color(super::example_rgb(BORDER))
            .children(
                group
                    .panels()
                    .iter()
                    .enumerate()
                    // A hidden panel keeps its place in the tree and its tab
                    // slot; it is the skin that leaves it undrawn.
                    .filter(|(_, panel)| panel.visible(cx))
                    .map(|(ix, panel)| {
                        let selected = ix == group.active_ix();
                        let title = panel_title(panel, cx);
                        div()
                            .id(("showcase-tab", ix))
                            .px_2()
                            .h_full()
                            .flex()
                            .items_center()
                            .text_xs()
                            .cursor_pointer()
                            .map(|this| match selected {
                                true => this
                                    .bg(super::example_rgb(SURFACE))
                                    .text_color(super::example_rgb(ACCENT)),
                                false => this.text_color(super::example_rgb(MUTED)),
                            })
                            .child(title.clone())
                            .on_click({
                                let group = group.clone();
                                move |_, window, cx| group.select_tab(ix, window, cx)
                            })
                            .when_some(group.drag_panel(ix, cx), |this, drag| {
                                this.on_drag(drag, move |_, _, _, cx| {
                                    cx.new(|_| DragPreview {
                                        title: title.clone(),
                                    })
                                })
                            })
                    })
                    .collect::<Vec<_>>(),
            )
            .into_any_element()
    }

    /// Base resolves where a drop would land; painting it is all that is left.
    fn render_drop_indicator(
        &self,
        indicator: DropIndicator,
        _: &mut Window,
        _: &mut App,
    ) -> Option<AnyElement> {
        let to = indicator.to();
        Some(
            div()
                .absolute()
                .left(to.origin().x)
                .top(to.origin().y)
                .w(to.size().width)
                .h(to.size().height)
                .bg(rgba(DROP_TARGET))
                .into_any_element(),
        )
    }
}

/// Build the area once, at showcase construction: a `DockArea` is an entity,
/// and rebuilding it every frame would discard the layout the viewer arranged.
pub(in super::super) fn build_dock(window: &mut Window, cx: &mut App) -> Entity<DockArea> {
    let explorer = ShowcasePanel::new(
        "Explorer",
        "Explorer",
        "Drag this tab into the other group to move it there.",
        cx,
    );
    let search = ShowcasePanel::new(
        "Search",
        "Search",
        "Two panels share this tab group. Click a tab to switch.",
        cx,
    );
    let editor = ShowcasePanel::new(
        "Editor",
        "Editor",
        "Drag a tab towards an edge of this group to split there.",
        cx,
    );
    let terminal = ShowcasePanel::new(
        "Terminal",
        "Terminal",
        "The bottom dock shares the column with the center region.",
        cx,
    );
    let problems = ShowcasePanel::new("Problems", "Problems", "Nothing here.", cx);

    let area = cx.new(|cx| {
        DockArea::new("showcase-dock", Some(1), window, cx)
            .with_renderer(Rc::new(ShowcaseDockSkin::default()))
    });

    area.update(cx, |area, cx| {
        area.set_center(
            DockLayout::h_split()
                .child(
                    DockLayout::tabs().panel(explorer).panel(search),
                    Some(px(200.)),
                )
                .child(DockLayout::tabs().panel(editor), None),
            window,
            cx,
        );
        area.set_dock(
            DockPlacement::Bottom,
            DockLayout::tabs().panel(terminal).panel(problems),
            window,
            cx,
        );
        area.set_dock_size(DockPlacement::Bottom, px(140.), window, cx);
    });

    area
}

impl BaseShowcase {
    /// A toggle for one dock, so a closed dock can be brought back.
    fn dock_toggle(
        &self,
        placement: DockPlacement,
        label: &'static str,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let open = self.dock.read(cx).is_dock_open(placement);
        let area = self.dock.clone();

        div()
            .id(label)
            .px_2()
            .py_1()
            .text_xs()
            .cursor_pointer()
            .border_1()
            .border_color(super::example_rgb(BORDER))
            .map(|this| match open {
                true => this
                    .bg(super::example_rgb(SURFACE))
                    .text_color(super::example_rgb(ACCENT)),
                false => this.text_color(super::example_rgb(MUTED)),
            })
            .child(label)
            .on_click(move |_, window, cx| {
                area.update(cx, |area, cx| area.toggle_dock(placement, window, cx));
            })
    }

    pub(in super::super) fn dock(&self, cx: &Context<Self>) -> impl IntoElement {
        // Fills whatever the showcase gives it — the surrounding container
        // opts this example out of the centered, intrinsically-sized box the
        // smaller parts use, so a percentage size resolves here.
        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .border_1()
            .border_color(super::example_rgb(BORDER))
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_2()
                    .p_2()
                    .bg(super::example_rgb(CHROME))
                    .border_b_1()
                    .border_color(super::example_rgb(BORDER))
                    .child(self.dock_toggle(DockPlacement::Bottom, "Bottom", cx))
                    .child(div().text_xs().text_color(super::example_rgb(MUTED)).child(
                        "Drag a tab onto another group to merge it, or towards an edge to split",
                    )),
            )
            .child(div().flex_1().min_h(px(0.)).child(self.dock.clone()))
    }
}
```

## 集成检查清单

- 面板、标签组和需要持久化的节点使用稳定 ID。
- 领域状态保存在面板或应用 entity，renderer 只负责表现。
- 布局修改只发生在事件回调，并在安装/恢复后规范化。
- 处理未知面板、空布局、关闭否决、拖放目标失效和版本迁移。
- 验证键盘焦点、标签顺序、锁定、缩放、边缘 dock、减少动态效果和高对比度。
