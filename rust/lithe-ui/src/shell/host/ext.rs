use std::rc::Rc;

use gpui::{AnyElement, App, ElementId, Entity, Window};

use crate::feedback::Toast;

use super::{WindowRoot, priority};

/// 一层浮层被**宿主**移除时的收尾（还焦点、通知 owner）。
///
/// 注意它和 `overlays::Dialog::on_close` 不是同一件事：后者由组件自己（Esc / 遮罩 /
/// 关闭按钮）触发，前者只在外层主动 `close_dialog` 时跑。两边各管一段，不重复。
pub type Close = Rc<dyn Fn(&mut Window, &mut App)>;

/// 宿主提供的**窗口级**入口。
///
/// 迁移期新旧两套窗口根并存，所以每个方法都返回"有没有做成"：调用方据此走回退路径，
/// **不许假装成功**（降级可见是本仓库既有纪律）。全部浮层入口都做成"返回 bool"而不是
/// 上游那种 `expect("window must have a Base Root")`——因为多窗口路径上确实存在还没挂宿主的
/// 窗口（`app/src/main.rs` 的"在新窗口打开工程"就是），照抄上游写法会在启动时直接 panic。
///
/// 名字与上游 `gpui_kit::component::WindowExt` 的对应关系（迁移后 gpui-kit 那一套退场）：
///
/// | 上游 | 这里 |
/// | --- | --- |
/// | `open_dialog(cx, build)` | [`open_dialog`](Self::open_dialog) |
/// | `close_dialog(cx)` | [`close_dialog`](Self::close_dialog) |
/// | `has_active_dialog(cx)` | [`has_active_dialog`](Self::has_active_dialog) |
/// | `push_notification(note, cx)` | [`push_notification`](Self::push_notification) |
pub trait WindowRootExt: Sized {
    /// 该窗口的宿主；窗口没挂宿主时返回 `None`。
    fn window_root(&self) -> Option<Entity<WindowRoot>>;

    /// 压入一层浮层（对话框 / 抽屉 / 面板），返回是否压成功（窗口没挂宿主时 `false`）。
    ///
    /// `build` **每帧重新调用**，拿到本层的 id、产出元素；不做副作用（元素在 prepaint 时被消费，
    /// 缓存下来第二帧必 panic）。`on_close` 只在本层被**宿主**移除时跑一次（见 [`Close`]）。
    fn push_layer(
        &mut self,
        id: impl Into<ElementId>,
        priority: usize,
        build: impl Fn(ElementId, &mut Window, &mut App) -> AnyElement + 'static,
        on_close: Option<Close>,
        cx: &mut App,
    ) -> bool;

    /// 压入一层对话框浮层（`priority::DIALOG`）。等价于上游 `WindowExt::open_dialog`。
    fn open_dialog(
        &mut self,
        id: impl Into<ElementId>,
        build: impl Fn(ElementId, &mut Window, &mut App) -> AnyElement + 'static,
        cx: &mut App,
    ) -> bool;

    /// 关掉最上面一层浮层。等价于上游 `WindowExt::close_dialog`；返回是否真的关掉了一层。
    ///
    /// 它只摘层、不跑浮层组件自己的 `on_close`——组件自己那条路（Esc / 遮罩 / 关闭按钮）
    /// 由组件负责收尾，与上游语义一致。
    fn close_dialog(&mut self, cx: &mut App) -> bool;

    /// 当前有没有打开的浮层。等价于上游 `WindowExt::has_active_dialog`。
    fn has_active_dialog(&mut self, cx: &mut App) -> bool;

    /// 推一条通知；窗口没挂宿主时返回 `false`，调用方须给出降级表现。
    fn push_notification(&mut self, toast: Toast, cx: &mut App) -> bool;

    /// 关掉最上面一层浮层；没挂宿主或没有浮层时返回 `false`。
    fn close_top_layer(&mut self, cx: &mut App) -> bool;
}

impl WindowRootExt for Window {
    fn window_root(&self) -> Option<Entity<WindowRoot>> {
        self.root::<WindowRoot>().flatten()
    }

    fn push_layer(
        &mut self,
        id: impl Into<ElementId>,
        priority: usize,
        build: impl Fn(ElementId, &mut Window, &mut App) -> AnyElement + 'static,
        on_close: Option<Close>,
        cx: &mut App,
    ) -> bool {
        open(self, id, priority, false, build, on_close, cx)
    }

    fn open_dialog(
        &mut self,
        id: impl Into<ElementId>,
        build: impl Fn(ElementId, &mut Window, &mut App) -> AnyElement + 'static,
        cx: &mut App,
    ) -> bool {
        open(self, id, priority::DIALOG, true, build, None, cx)
    }

    fn close_dialog(&mut self, cx: &mut App) -> bool {
        self.close_top_layer(cx)
    }

    fn has_active_dialog(&mut self, cx: &mut App) -> bool {
        self.window_root()
            .map(|root| root.read(cx).has_layers())
            .unwrap_or(false)
    }

    fn push_notification(&mut self, toast: Toast, cx: &mut App) -> bool {
        let Some(root) = self.window_root() else {
            return false;
        };
        root.update(cx, |root, cx| {
            root.push_toast(toast, cx);
        });
        true
    }

    fn close_top_layer(&mut self, cx: &mut App) -> bool {
        let Some(root) = self.window_root() else {
            return false;
        };
        let window: &mut Window = self;
        root.update(cx, |root, cx| root.pop_layer(window, cx))
    }
}

/// `push_layer` / `open_dialog` 的共同实现；`dialog` 决定要不要走"对话框那一层"
/// （唯一的差别是移除时宿主把焦点交还给压入之前的那个人，理由见 `WindowRoot::push_dialog`）。
fn open(
    window: &mut Window,
    id: impl Into<ElementId>,
    priority: usize,
    dialog: bool,
    build: impl Fn(ElementId, &mut Window, &mut App) -> AnyElement + 'static,
    on_close: Option<Close>,
    cx: &mut App,
) -> bool {
    let Some(root) = window.window_root() else {
        return false;
    };
    // 在**压入之前**取焦点：组件第一次渲染时会把它抢走，那时再取就晚了。
    // `window.focused(cx)` 在渲染之外（事件回调里）也能读，与上游记 `previous_focused_handle`
    // 的时机一致。
    let previous_focus = if dialog {
        window.focused(cx).map(|handle| handle.downgrade())
    } else {
        None
    };
    let id = id.into();
    let build_id = id.clone();
    root.update(cx, move |root, cx| {
        let build = move |window: &mut Window, cx: &mut App| build(build_id.clone(), window, cx);
        if dialog {
            root.push_dialog(id, priority, build, on_close, previous_focus, cx);
        } else {
            root.push_layer(id, priority, build, on_close, cx);
        }
    });
    true
}
