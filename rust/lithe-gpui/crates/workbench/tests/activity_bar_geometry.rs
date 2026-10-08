mod common;

use gpui::{
    AppContext as _, Context, ElementId, IntoElement, Render, TestAppContext, Window, px, size,
};
use gpui_kit::assets::IconName;
use gpui_kit::test::TestWindowExt as _;

use lithe_gpui_workbench::activity_bar::{ActivityItem, ActivitySide, activity_bar};

struct BarHost {
    items: Vec<ActivityItem>,
}

impl Render for BarHost {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        activity_bar(
            ActivitySide::Left,
            &self.items,
            |_| false,
            |_, _, _, _| {},
            window,
            cx,
        )
    }
}

#[gpui::test]
fn activity_bar_item_measures_the_macos_item_box(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);

    let (handle, _host) = common::open_window(cx, Some(size(px(400.), px(400.))), |_window, cx| {
        cx.new(|_cx| BarHost {
            items: vec![ActivityItem::new(IconName::Puzzle, "测试项")],
        })
    });

    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
    })
    .unwrap();

    cx.update_window(handle.into(), |_, window, _cx| {
        let item = window.find(ElementId::named_usize("lithe-activity-bar-left", 0));
        let bounds = item.bounds();
        assert!(
            (bounds.size.width.as_f32() - 30.).abs() < 0.5,
            "项盒宽照真源 30（实际 {}）",
            bounds.size.width.as_f32()
        );
        assert!(
            (bounds.size.height.as_f32() - 30.).abs() < 0.5,
            "项盒高照真源 30（实际 {}）",
            bounds.size.height.as_f32()
        );
    })
    .unwrap();
}
