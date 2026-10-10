mod common;

use gpui::{AppContext as _, Context, IntoElement, Render, TestAppContext, Window, px, size};
use lithe_gpui_shared::icons::idea;

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
    let (handle, _host) = common::open_window(cx, Some(size(px(400.), px(400.))), |_window, cx| {
        cx.new(|_cx| BarHost {
            items: vec![ActivityItem::idea(&idea::PLUGIN_ICON, "测试项")],
        })
    });

    // 断言改从 gpui 自己的 debug selector 读（`.debug_selector` 记进 `rendered_frame.debug_bounds`）：
    // 上游那套 `TestSupportExt` 观察包装随 gpui-kit 一并退场，补一帧用 gpui 原生的
    // `refresh` + `draw(..).clear(..)`（正是 `TestWindowExt::render_frame` 的实现）。
    let mut window = gpui::VisualTestContext::from_window(handle.into(), cx);
    window.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    let bounds = window
        .debug_bounds("lithe-activity-bar-item-0")
        .expect("活动栏项必须带 debug selector（lithe-activity-bar-item-<index>），UI 测试就是靠它量盒子的");
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
}
