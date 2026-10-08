//! 窗口根宿主的集成测试：证明"压一层浮层 → Esc / 显式关闭 → 移出"这条路**真的通**，
//! 而不是只编译得过。
//!
//! 测试用真实视图进无头窗口（`add_window_view` + `Window::replace_root`，与生产组合根
//! `workbench/src/workspace.rs` 的换根路径同形），键事件用 gpui 的确定性时钟推进。

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    Context, Entity, FocusHandle, IntoElement, KeyUpEvent, Keystroke, ParentElement, Render,
    Styled, TestAppContext, VisualTestContext, Window, div,
};

use super::{WindowRoot, WindowRootExt as _, dismiss_hosted, priority};
use crate::{overlays::Dialog, primitives::FocusScope, theme::Theme};

/// 宿主包住的最小内容视图；只有一个焦点锚点，够测"根是不是装上了"。
struct Content {
    root: FocusHandle,
}

impl Content {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            root: cx.focus_handle(),
        }
    }
}

impl Render for Content {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        FocusScope::new(&self.root)
            .size_full()
            .child(div().child("content"))
    }
}

fn settle(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

fn press(key: &str, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes(key);
    settle(cx);
    cx.simulate_event(KeyUpEvent {
        keystroke: Keystroke::parse(key).expect("a key"),
    });
    settle(cx);
}

/// 开一个窗口并把它换成"内容 + 窗口根宿主"，与生产组合根的换根路径同形。
fn rooted_window(cx: &mut TestAppContext) -> (Entity<Content>, &mut VisualTestContext) {
    cx.update(Theme::init);
    let (content, cx) = cx.add_window_view(|_window, cx| Content::new(cx));
    cx.update(|window, cx| {
        window.replace_root(cx, |window, cx| {
            WindowRoot::new(content.clone(), window, cx)
        });
    });
    settle(cx);
    (content, cx)
}

#[gpui::test]
fn the_host_is_the_only_root_of_the_window(cx: &mut TestAppContext) {
    let (_content, cx) = rooted_window(cx);
    let rooted = cx.update(|window, _| WindowRoot::is_rooted(window));
    assert!(rooted, "换根后窗口的根必须是 WindowRoot");
    let has_layers = cx.update(|window, cx| {
        window
            .window_root()
            .map(|root| root.read(cx).has_layers())
            .unwrap_or(false)
    });
    assert!(!has_layers, "刚装上的宿主必须能被取到，且一开始没有浮层");
}

#[gpui::test]
fn escape_puts_the_dialog_layer_back_down(cx: &mut TestAppContext) {
    let (_content, cx) = rooted_window(cx);

    let opened = cx.update(|window, cx| {
        window.open_dialog(
            "host-test-dialog",
            |id, _window, _cx| {
                Dialog::new(id)
                    .title("probe")
                    .on_close(dismiss_hosted)
                    .child(div().child("body"))
                    .into_any_element()
            },
            cx,
        )
    });
    assert!(opened, "装了宿主的窗口必须能压入一层对话框");
    settle(cx);

    let active = cx.update(|window, cx| window.has_active_dialog(cx));
    assert!(active, "压入之后 `has_active_dialog` 必须为真");

    press("escape", cx);
    let active = cx.update(|window, cx| window.has_active_dialog(cx));
    assert!(
        !active,
        "Esc 之后那一层必须真的从栈里移出，而不是只把动画走完了"
    );
}

#[gpui::test]
fn closing_explicitly_runs_the_layer_on_close_once(cx: &mut TestAppContext) {
    let (_content, cx) = rooted_window(cx);

    let closed: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let hook = closed.clone();
    let pushed = cx.update(|window, cx| {
        window.push_layer(
            "host-test-layer",
            priority::DRAWER,
            |_id, _window, _cx| div().child("drawer").into_any_element(),
            Some(Rc::new(move |_window, _cx| hook.set(hook.get() + 1))),
            cx,
        )
    });
    assert!(pushed, "装了宿主的窗口必须能压入一层抽屉");
    settle(cx);
    assert_eq!(closed.get(), 0, "压入本身不该触发 on_close");

    let popped = cx.update(|window, cx| window.close_dialog(cx));
    assert!(popped, "有浮层时 close_dialog 必须报告关掉了一层");
    assert_eq!(closed.get(), 1, "宿主移除这一层时必须跑一次 on_close");

    let again = cx.update(|window, cx| window.close_dialog(cx));
    assert!(
        !again,
        "栈空了之后 close_dialog 必须如实返回 false，不许假装成功"
    );
    assert_eq!(closed.get(), 1, "on_close 不许被重复触发");
}

#[gpui::test]
fn a_window_without_the_host_reports_instead_of_panicking(cx: &mut TestAppContext) {
    cx.update(Theme::init);
    let (_content, cx) = cx.add_window_view(|_window, cx| Content::new(cx));
    settle(cx);

    let opened = cx.update(|window, cx| {
        window.open_dialog("no-host", |_id, _w, _c| div().into_any_element(), cx)
    });
    assert!(
        !opened,
        "没装宿主的窗口必须如实返回 false（多窗口兜底路径就靠这条不 panic）"
    );
    let active = cx.update(|window, cx| window.has_active_dialog(cx));
    assert!(!active, "没宿主时不该报告有浮层");
}

/// 程序化关闭（不是 Esc / 遮罩）也必须把焦点交还给打开对话框的那个人。
///
/// 这条是真机可见的：焦点留在已经离树的那个 handle 上，之后的键盘输入就没有落点——
/// 例如从快速打开里选中文件之后，编辑器收不到按键。上游的 `WindowState::close_dialog`
/// 会恢复 `previous_focused_handle`，换根后这条得由宿主补上。
#[gpui::test]
fn closing_a_dialog_programmatically_hands_focus_back(cx: &mut TestAppContext) {
    let (content, cx) = rooted_window(cx);

    let previous = cx.update(|_window, cx| content.read(cx).root.clone());
    cx.update(|window, cx| window.focus(&previous, cx));
    settle(cx);
    assert!(
        cx.update(|window, _| previous.is_focused(window)),
        "前置：打开对话框之前，焦点在内容上"
    );

    cx.update(|window, cx| {
        window.open_dialog(
            "focus-probe",
            |id, _window, _cx| {
                Dialog::new(id)
                    .title("focus")
                    .on_close(dismiss_hosted)
                    .child(div().child("body"))
                    .into_any_element()
            },
            cx,
        )
    });
    settle(cx);
    assert!(
        !cx.update(|window, _| previous.is_focused(window)),
        "对话框打开时焦点被它接管"
    );

    cx.update(|window, cx| {
        window.close_dialog(cx);
    });
    settle(cx);
    assert!(
        cx.update(|window, _| previous.is_focused(window)),
        "程序化关闭之后焦点必须回到打开对话框之前的位置"
    );
}
