//! Dock 布局（第 3/4 章）：可拖拽分隔的左右（或上下）面板。
//!
//! 行为层完全复用 gpui-base 的 `resizable`：拖动事件由局部布局容器承载、
//! 尺寸经 min/max 钳制、高频事件内部合并——符合
//! `2026-09-13-resizable-ui-performance-boundaries` Note 的边界要求。
//! 本层只提供 Ely 外观（分隔条 hairline/悬停加深/拖动主色）与包装 API。
//! 尺寸持久化归调用方：只在 `on_resize` 里按需写布局存储，不要在拖动中写。

use std::rc::Rc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::base::{
    ResizablePanel as BasePanel, ResizablePanelGroup, ResizableState, ResizeHandleContext,
    ResizeHandleRenderer, ResizeHandleState, TestSupportExt as _, h_resizable, resizable_panel,
    v_resizable,
};
use gpui_kit::prelude::*;
use gpui_kit::{
    App, Axis, ElementId, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px,
};
use std::ops::Range;

use crate::ActiveTheme;

/// 分隔条的 Ely 外观：静止为 1px hairline，悬停加深，拖动用主色。
pub fn resize_handle_appearance() -> ResizeHandleRenderer {
    Rc::new(|ctx: &ResizeHandleContext, _, cx: &mut App| {
        let theme = cx.theme();
        let color = match ctx.state() {
            ResizeHandleState::Idle => theme.border,
            ResizeHandleState::Hovered => theme.border_strong,
            // 按下未动与拖动中都保持主色，给出"握住了"的反馈。
            ResizeHandleState::Pressed | ResizeHandleState::Dragging => theme.primary,
        };
        // Horizontal 组的面板左右排布 → 分隔条是竖条；反之是横条。
        let bar = match ctx.axis() {
            Axis::Horizontal => div().w(px(1.)).h_full(),
            Axis::Vertical => div().h(px(1.)).w_full(),
        };
        Some(
            div()
                .id("dock-handle")
                .test_support()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .cursor_col_resize()
                .child(bar.bg(color))
                .into_any_element(),
        )
    })
}

/// 水平 Dock：面板左右排布，分隔条可左右拖动。
pub fn h_dock(id: impl Into<ElementId>, state: &Entity<ResizableState>) -> Dock {
    Dock {
        group: h_resizable(id)
            .with_state(state)
            .with_handle_appearance(resize_handle_appearance()),
    }
}

/// 垂直 Dock：面板上下排布，分隔条可上下拖动。
pub fn v_dock(id: impl Into<ElementId>, state: &Entity<ResizableState>) -> Dock {
    Dock {
        group: v_resizable(id)
            .with_state(state)
            .with_handle_appearance(resize_handle_appearance()),
    }
}

/// Dock 容器。`panel` 依序加入面板；`on_resize` 在尺寸变化后回调
/// （节流归 base，可安全用于状态回写）。
#[derive(IntoElement)]
pub struct Dock {
    group: ResizablePanelGroup,
}

impl Dock {
    pub fn panel(mut self, panel: DockPanel) -> Self {
        // base 的 group.child 只收 ResizablePanel/AnyElement；这里在装配期
        // 把包装面板转成 BasePanel（样式 refine 是纯数据操作，可提前做）。
        self.group = self.group.child(panel.into_base());
        self
    }

    pub fn on_resize(
        mut self,
        handler: impl Fn(&Entity<ResizableState>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.group = self.group.on_resize(handler);
        self
    }
}

impl RenderOnce for Dock {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.group
    }
}

/// Dock 面板。默认最小宽 `PANEL_MIN_SIZE`；内容经 `child` 传入。
/// 这是纯 builder（不自己渲染），加入 [`Dock`] 时转成 base 面板。
pub struct DockPanel {
    panel: BasePanel,
    style: gpui_kit::StyleRefinement,
}

impl DockPanel {
    pub fn new() -> Self {
        Self {
            panel: resizable_panel(),
            style: Default::default(),
        }
    }

    /// 初始尺寸。
    pub fn size(mut self, size: gpui_kit::Pixels) -> Self {
        self.panel = self.panel.size(size);
        self
    }

    /// 尺寸钳制范围（拖动不会越过）。
    pub fn size_range(mut self, range: Range<gpui_kit::Pixels>) -> Self {
        self.panel = self.panel.size_range(range);
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.panel = self.panel.visible(visible);
        self
    }

    fn into_base(self) -> BasePanel {
        let mut panel = self.panel;
        panel = panel.refine_style(&self.style);
        panel
    }
}

impl Styled for DockPanel {
    fn style(&mut self) -> &mut gpui_kit::StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for DockPanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui_kit::AnyElement>) {
        self.panel.extend(elements);
    }
}
