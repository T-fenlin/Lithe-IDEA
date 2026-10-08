use std::rc::Rc;

use gpui::{
    AnyElement, AnyView, App, Context, ElementId, Entity, IntoElement, ParentElement, Render,
    Styled, Subscription, WeakFocusHandle, Window, div, prelude::*,
};

use crate::{
    feedback::{Toast, ToastViewport, Toaster},
    primitives::raise,
    theme::ActiveTheme as _,
};

use super::priority;

/// 一层浮层的搭建方式。**每帧重新调用**——元素会在 prepaint 时被消费，缓存必 panic。
type Build = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

struct Overlay {
    id: ElementId,
    priority: usize,
    /// 移除这一层时把焦点交还给谁（见 `WindowRoot::push_dialog`）。`None` 表示这一层不管焦点。
    previous_focus: Option<WeakFocusHandle>,
    build: Build,
    /// 关闭该层时的收尾（交还焦点、通知 owner）。宿主关闭它时调用一次。
    on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
}

/// 一个窗口的内容 **加上** 一层有序浮层栈。
pub struct WindowRoot {
    content: AnyView,
    overlays: Vec<Overlay>,
    toaster: Entity<Toaster>,
    /// 订阅 toaster，保证 toast 进出时宿主重渲染；`push` 之后只订阅、不在 render 里 notify。
    _toaster_changed: Subscription,
    next: usize,
}

impl WindowRoot {
    /// 包住窗口内容。`content` 通常是外壳（`ShellWorkspace`）或欢迎页。
    pub fn new(content: impl Into<AnyView>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // 新窗口继承当前缩放：尺寸只有一个锚点（主题基准字号 × 用户缩放）。
        cx.theme().apply_rem(window);
        let toaster = cx.new(|_| Toaster::default());
        let _toaster_changed = cx.observe(&toaster, |_, _, cx| cx.notify());
        Self {
            content: content.into(),
            overlays: Vec::new(),
            toaster,
            _toaster_changed,
            next: 0,
        }
    }

    /// 这个窗口是否已经挂了本宿主。迁移期两套根并存时，调用方据此选择走哪条路。
    pub fn is_rooted(window: &Window) -> bool {
        window.root::<WindowRoot>().flatten().is_some()
    }

    /// 压入一层浮层（对话框 / 抽屉）。返回它的 `ElementId`，可用于精确关闭。
    ///
    /// `build` 每帧被调用，所以它应当是**纯搭建**：读状态、产出元素，不做副作用。
    ///
    /// 本方法**不管焦点**：这一层是谁的元素、焦点归谁，由调用方自己收。元素是
    /// `overlays::Dialog` 那一类时用 [`Self::push_dialog`]。
    pub fn push_layer(
        &mut self,
        id: impl Into<ElementId>,
        priority: usize,
        build: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
        cx: &mut Context<Self>,
    ) -> ElementId {
        self.push(id, priority, None, build, on_close, cx)
    }

    /// 压入一层**对话框**浮层（`crate::overlays::Dialog` 那一类）。
    ///
    /// 与 [`Self::push_layer`] 的唯一区别：移除这一层时宿主把焦点交还给 `previous_focus`
    /// ——**压入之前**窗口上的焦点（`window.focused(cx)`，由调用方在压入时取；与上游
    /// `gpui-component` 的 `WindowState::open_dialog` 记 `previous_focused_handle` 是同一个做法）。
    ///
    /// 为什么非要宿主补这一手：两条关闭路径只有一条能自己交还焦点。
    ///
    /// - **组件自己关**（Esc / 遮罩 / 关闭按钮）→ 组件的 `close` 已经交还给同一个人，
    ///   宿主这次是幂等的第二次；
    /// - **程序化关**（`WindowRootExt::close_dialog`）→ 走不到组件内部的 `close`，
    ///   不补的话焦点会留在已经离树的那个 handle 上，之后的键盘输入没有落点
    ///   （真机症状：从快速打开里选中文件之后，编辑器收不到按键）。
    pub fn push_dialog(
        &mut self,
        id: impl Into<ElementId>,
        priority: usize,
        build: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
        previous_focus: Option<WeakFocusHandle>,
        cx: &mut Context<Self>,
    ) -> ElementId {
        self.push(id, priority, previous_focus, build, on_close, cx)
    }

    fn push(
        &mut self,
        id: impl Into<ElementId>,
        priority: usize,
        previous_focus: Option<WeakFocusHandle>,
        build: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
        cx: &mut Context<Self>,
    ) -> ElementId {
        let id = id.into();
        // 同类里"后压入的更高"：把档位放大 1000 倍再叠加序号，序号不会越过上一档。
        let seq = self.next % 1000;
        self.next = self.next.wrapping_add(1);
        let priority = priority * 1000 + seq;
        self.overlays.push(Overlay {
            id: id.clone(),
            priority,
            previous_focus,
            build: Rc::new(build),
            on_close,
        });
        cx.notify();
        id
    }

    /// 推一条通知；返回 toast id。
    pub fn push_toast(&mut self, toast: Toast, cx: &mut Context<Self>) -> u64 {
        self.toaster
            .update(cx, |toaster, cx| toaster.push(toast, cx))
    }

    /// 关掉最上面一层；返回是否真的关掉了一层（没有浮层时返回 `false`，调用方可据此降级）。
    pub fn pop_layer(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(index) = self.top_index() else {
            return false;
        };
        self.remove_at(index, window, cx);
        true
    }

    /// 关掉指定一层。
    pub fn close_layer(
        &mut self,
        id: &ElementId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(index) = self.overlays.iter().position(|overlay| &overlay.id == id) else {
            return false;
        };
        self.remove_at(index, window, cx);
        true
    }

    /// 当前是否有浮层。
    pub fn has_layers(&self) -> bool {
        !self.overlays.is_empty()
    }

    fn remove_at(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let overlay = self.overlays.remove(index);
        // 先交还焦点、再跑收尾，与 `overlays::Dialog` 自己那条 close 的顺序一致。
        if let Some(previous) = overlay
            .previous_focus
            .as_ref()
            .and_then(|handle| handle.upgrade())
        {
            window.focus(&previous, cx);
        }
        if let Some(on_close) = overlay.on_close {
            on_close(window, cx);
        }
        cx.notify();
    }

    fn top_index(&self) -> Option<usize> {
        self.overlays
            .iter()
            .enumerate()
            .max_by_key(|(_, overlay)| overlay.priority)
            .map(|(index, _)| index)
    }
}

impl Render for WindowRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let me = cx.entity();
        let mut root = div()
            .id("lithe-window-root")
            .size_full()
            // Esc 关最上面一层。gpui 没有窗口级"按键兜底"，所以挂在内容容器上；
            // 焦点在浮层内时事件先给浮层，浮层自己处理完不该再冒泡到这里。
            .on_key_down(move |event, window, cx| {
                if event.keystroke.key != "escape" {
                    return;
                }
                let me = me.clone();
                me.update(cx, |root, cx| {
                    root.pop_layer(window, cx);
                });
            })
            .child(self.content.clone());
        // 通知层：viewport 常挂，画不画由 Toaster 自己决定。
        root = root.child(
            raise(
                "lithe-toast-layer",
                ToastViewport::new("lithe-toasts", &self.toaster),
            )
            .with_priority(priority::NOTIFICATION),
        );
        // 浮层：每帧重建。
        for overlay in &self.overlays {
            let element = (overlay.build)(window, cx);
            root = root.child(raise(overlay.id.clone(), element).with_priority(overlay.priority));
        }
        root
    }
}
