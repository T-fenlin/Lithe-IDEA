use gpui::{App, Entity, Window};

use crate::feedback::Toast;

use super::WindowRoot;

/// 宿主提供的**窗口级**入口。
///
/// 迁移期新旧两套窗口根并存，所以每个方法都返回"有没有做成"：调用方据此走回退路径，
/// **不许假装成功**（降级可见是本仓库既有纪律）。全部浮层入口都做成"返回 bool"而不是
/// 上游那种 `expect("window must have a Base Root")`——因为多窗口路径上确实存在还没挂宿主的
/// 窗口（`app/src/main.rs` 的"在新窗口打开工程"就是），照抄上游写法会在启动时直接 panic。
pub trait WindowRootExt {
    /// 该窗口的宿主；窗口没挂宿主时返回 `None`。
    fn window_root(&self) -> Option<Entity<WindowRoot>>;

    /// 推一条通知；窗口没挂宿主时返回 `false`，调用方须给出降级表现。
    fn push_notification(&mut self, toast: Toast, cx: &mut App) -> bool;

    /// 关掉最上面一层浮层；没挂宿主或没有浮层时返回 `false`。
    fn close_top_layer(&mut self, cx: &mut App) -> bool;
}

impl WindowRootExt for Window {
    fn window_root(&self) -> Option<Entity<WindowRoot>> {
        self.root::<WindowRoot>().flatten()
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
