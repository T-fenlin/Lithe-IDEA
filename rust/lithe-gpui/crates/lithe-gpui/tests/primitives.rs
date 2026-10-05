//! 第 1 章 Primitives 的 UI 集成测试：真实渲染、指针交互、回调通路。
//! 断言走 gpui-kit 的元素观测（ElementId + 状态），不依赖像素与墙钟。

mod common;

use std::{cell::Cell, rc::Rc};

use gpui_kit::prelude::*;
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{Render, TestAppContext, Window, div, px};
use lithe_gpui::{
    Avatar, Badge, BadgeVariant, Icon, Kbd, Label, Progress, Separator, Spinner, Tag, init,
};

struct Harness {
    removed: Rc<Cell<u32>>,
}

impl Render for Harness {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let removed = self.removed.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .child(Icon::new("icon-check", "icons/check.svg"))
            .child(Avatar::new("avatar-zk", "ZK"))
            .child(Badge::new("badge-success", "OK").variant(BadgeVariant::Success))
            .child(
                Tag::new("tag-closable", "Java")
                    .on_remove(move |_, _| removed.set(removed.get() + 1)),
            )
            .child(Label::new("label-name", "Project name").required(true))
            .child(Kbd::new("kbd-cmd", "⌘K"))
            .child(Spinner::new("spinner"))
            .child(Progress::new("progress").value(0.6))
            .child(Separator::new("separator-h"))
            .child(
                div()
                    .h_8()
                    .flex()
                    .child(Separator::new("separator-v").vertical()),
            )
    }
}

#[gpui_kit::test]
fn chapter_one_components_render_with_stable_ids(cx: &mut TestAppContext) {
    cx.update(init);
    let handle = common::open_window(cx, px(640.), px(480.), |_, cx| {
        cx.new(|_| Harness {
            removed: Rc::new(Cell::new(0)),
        })
    });
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        for id in [
            "icon-check",
            "avatar-zk",
            "badge-success",
            "tag-closable",
            "label-name",
            "kbd-cmd",
            "spinner",
            "progress",
            "separator-h",
            "separator-v",
        ] {
            assert!(window.try_find(id).is_some(), "missing element {id}");
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn tag_remove_reports_intent_exactly_once_per_click(cx: &mut TestAppContext) {
    cx.update(init);
    let removed = Rc::new(Cell::new(0));
    let flag = removed.clone();
    let handle = common::open_window(cx, px(640.), px(480.), |_, cx| {
        cx.new(|_| Harness { removed: flag })
    });
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.within("tag-closable").click("remove", cx);
        window.within("tag-closable").click("remove", cx);
    })
    .unwrap();
    assert_eq!(removed.get(), 2, "每次点击恰好上报一次移除意图");
}
