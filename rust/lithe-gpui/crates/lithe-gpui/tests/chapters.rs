//! 第 2–20 章组件的 UI 集成测试：指针交互、回调通路、overlay 层。
//! 断言走 gpui-kit 的元素观测与状态快照，不依赖像素与墙钟。
//! 文本编辑语义属于 base::input 的自有测试域，这里聚焦包装层与状态回写。

mod common;

use std::{cell::Cell, rc::Rc};

use gpui_kit::prelude::*;
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{App, Entity, Render, TestAppContext, Window, div, point, px, rems};
use lithe_gpui::{
    Button, Checkbox, CommitList, CommitSummary, DiffLine, DiffLineKind, DiffStats, DiffView,
    Empty, FrameList, Heading, Input, InputState, Menu, MenuItem, Notification, Pagination,
    StackFrame, Switch, Tab, Tabs, Text, Variable, VariableList, init, open_menu,
    push_notification,
};

/// 观测夹具：所有要交互/断言的组件都在这一屏上。
struct Harness {
    clicks: Rc<Cell<u32>>,
    checked: Rc<Cell<bool>>,
    switched: Rc<Cell<bool>>,
    selected_tab: Rc<Cell<Option<usize>>>,
    selected_commit: Rc<Cell<Option<usize>>>,
    selected_frame: Rc<Cell<Option<usize>>>,
    page: Rc<Cell<usize>>,
    input: Entity<InputState>,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let clicks = self.clicks.clone();
        let checked = self.checked.clone();
        let switched = self.switched.clone();
        let selected_tab = self.selected_tab.clone();
        let selected_commit = self.selected_commit.clone();
        let selected_frame = self.selected_frame.clone();
        let page_cell = self.page.clone();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(Button::new("btn-ok").label("OK").on_click(move |_, _, _| {
                clicks.set(clicks.get() + 1);
            }))
            .child(Button::new("btn-locked").label("Locked").disabled(true))
            .child(
                Checkbox::new("checkbox-demo")
                    .label("Enabled")
                    .checked(self.checked.get())
                    .on_change(move |value, _, _| checked.set(*value)),
            )
            .child(
                Switch::new("switch-demo")
                    .checked(self.switched.get())
                    .on_change(move |value, _, _| switched.set(*value)),
            )
            .child(
                Tabs::new("tabs-demo")
                    .selected_index(0)
                    .child(Tab::new("General"))
                    .child(Tab::new("Editor"))
                    .on_select(move |ix, _, _| selected_tab.set(Some(ix))),
            )
            .child(Input::new(&self.input))
            .child(
                Pagination::new("pagination-demo", 1, 5).on_change(move |page_no, _, _| {
                    page_cell.set(page_no);
                }),
            )
            .child(Heading::new("heading-demo", "Heading"))
            .child(Text::new("text-demo", "Body"))
            .child(Empty::new("empty-demo", "No results"))
            .child(
                DiffView::new(
                    "diff-demo",
                    vec![
                        DiffLine::new(DiffLineKind::Context, Some(1), Some(1), "a"),
                        DiffLine::new(DiffLineKind::Added, None, Some(2), "b"),
                        DiffLine::new(DiffLineKind::Removed, Some(2), None, "c"),
                    ],
                )
                .stats(DiffStats {
                    additions: 1,
                    deletions: 1,
                }),
            )
            .child(
                CommitList::new(
                    "commit-list",
                    vec![CommitSummary::new("abc1234", "first", "a", "now")],
                )
                .on_select(move |ix, _, _| selected_commit.set(Some(ix))),
            )
            .child(
                FrameList::new(
                    "frame-list",
                    vec![StackFrame::new("Main.main", "Main.java:15")],
                )
                .on_select(move |ix, _, _| selected_frame.set(Some(ix))),
            )
            .child(VariableList::new(
                "variable-list",
                vec![Variable::new("count", "42").changed(true)],
            ))
    }
}

fn new_harness(
    clicks: Rc<Cell<u32>>,
    checked: Rc<Cell<bool>>,
    switched: Rc<Cell<bool>>,
    selected_tab: Rc<Cell<Option<usize>>>,
    selected_commit: Rc<Cell<Option<usize>>>,
    selected_frame: Rc<Cell<Option<usize>>>,
    page: Rc<Cell<usize>>,
) -> impl FnOnce(&mut Window, &mut App) -> Entity<Harness> {
    move |window, cx| {
        cx.new(|cx| Harness {
            clicks,
            checked,
            switched,
            selected_tab,
            selected_commit,
            selected_frame,
            page,
            input: cx.new(|cx| InputState::new(window, cx)),
        })
    }
}

#[gpui_kit::test]
fn buttons_fire_clicks_and_disabled_rejects(cx: &mut TestAppContext) {
    cx.update(init);
    let clicks = Rc::new(Cell::new(0));
    let handle = common::open_window(
        cx,
        px(640.),
        px(640.),
        new_harness(
            clicks.clone(),
            Rc::new(Cell::new(false)),
            Rc::new(Cell::new(false)),
            Rc::new(Cell::new(None)),
            Rc::new(Cell::new(None)),
            Rc::new(Cell::new(None)),
            Rc::new(Cell::new(0)),
        ),
    );
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("btn-ok", cx);
        window.click("btn-ok", cx);
        window.click("btn-locked", cx);
    })
    .unwrap();
    assert_eq!(clicks.get(), 2, "两次点击各触发一次；禁用按钮不得触发回调");
}

#[gpui_kit::test]
fn forms_report_controlled_intent(cx: &mut TestAppContext) {
    cx.update(init);
    let checked = Rc::new(Cell::new(false));
    let switched = Rc::new(Cell::new(false));
    let handle = common::open_window(
        cx,
        px(640.),
        px(640.),
        new_harness(
            Rc::new(Cell::new(0)),
            checked.clone(),
            switched.clone(),
            Rc::new(Cell::new(None)),
            Rc::new(Cell::new(None)),
            Rc::new(Cell::new(None)),
            Rc::new(Cell::new(0)),
        ),
    );
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("checkbox-demo", cx);
        window.click("switch-demo", cx);
    })
    .unwrap();
    assert!(checked.get(), "Checkbox 上报开启意图");
    assert!(switched.get(), "Switch 上报开启意图");
}

#[gpui_kit::test]
fn navigation_and_data_lists_report_selection(cx: &mut TestAppContext) {
    cx.update(init);
    let selected_tab = Rc::new(Cell::new(None));
    let selected_commit = Rc::new(Cell::new(None));
    let selected_frame = Rc::new(Cell::new(None));
    let handle = common::open_window(
        cx,
        px(640.),
        px(640.),
        new_harness(
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(false)),
            Rc::new(Cell::new(false)),
            selected_tab.clone(),
            selected_commit.clone(),
            selected_frame.clone(),
            Rc::new(Cell::new(0)),
        ),
    );
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.within("tabs-demo").click(1usize, cx);
        window.within("commit-list").click(("commit", 0usize), cx);
        window.within("frame-list").click(("frame", 0usize), cx);
    })
    .unwrap();
    assert_eq!(selected_tab.get(), Some(1), "Tabs 报告选中的下标");
    assert_eq!(selected_commit.get(), Some(0), "CommitList 报告选中的提交");
    assert_eq!(selected_frame.get(), Some(0), "FrameList 报告选中的帧");
}

#[gpui_kit::test]
fn chapter_surfaces_render_with_stable_ids(cx: &mut TestAppContext) {
    cx.update(init);
    let handle = common::open_window(
        cx,
        px(720.),
        px(720.),
        new_harness(
            Rc::new(Cell::new(0)),
            Rc::new(Cell::new(false)),
            Rc::new(Cell::new(false)),
            Rc::new(Cell::new(None)),
            Rc::new(Cell::new(None)),
            Rc::new(Cell::new(None)),
            Rc::new(Cell::new(0)),
        ),
    );
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        for id in [
            "heading-demo",
            "text-demo",
            "empty-demo",
            "diff-demo",
            "variable-list",
        ] {
            assert!(window.try_find(id).is_some(), "missing element {id}");
        }
        assert!(
            window.try_find(("line", 0usize)).is_some(),
            "missing diff line 0"
        );
        assert!(
            window.try_find(("line", 2usize)).is_some(),
            "missing diff line 2"
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn menu_and_notification_layers_render_and_dismiss(cx: &mut TestAppContext) {
    cx.update(init);
    let handle = common::open_window(cx, px(480.), px(480.), |_, cx| cx.new(|_| OverlayHarness));
    // 副作用在测试体内做一次，而不是在 render 里（render 必须无副作用）。
    cx.update_window(handle, |_, window, cx| {
        open_menu(
            Menu::new()
                .item(MenuItem::new("First").on_click(|_, _| {}))
                .item(MenuItem::new("Danger").danger(true)),
            point(px(40.), px(40.)),
            window,
            cx,
        );
        push_notification(Notification::new("Hello").message("world"), window, cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(
            window.try_find("menu-capture").is_some(),
            "menu capture missing"
        );
        assert!(
            window.try_find(("menu-item", 0usize)).is_some(),
            "menu item missing"
        );
        assert!(
            window.try_find(("toast", 0usize)).is_some(),
            "toast missing"
        );
        window.click("menu-capture", cx);
        window.render_frame(cx);
        assert!(
            window.try_find("menu-capture").is_none(),
            "点击外部后菜单层应关闭"
        );
        window
            .within(("toast", 0usize))
            .click(("toast-close", 0usize), cx);
        window.render_frame(cx);
        assert!(
            window.try_find(("toast", 0usize)).is_none(),
            "关闭后通知应消失"
        );
    })
    .unwrap();
}

struct OverlayHarness;

impl Render for OverlayHarness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size(rems(8.0))
    }
}
