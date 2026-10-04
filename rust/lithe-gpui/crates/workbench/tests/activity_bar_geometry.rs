//! 活动栏的**几何**回归测试：把批 1 换源到 macOS 规格的项盒尺寸钉成可复现证据。
//!
//! 为什么不用截图：`lithe-gpui/UI-MAP.md` 记过"非 DPI 感知的截图会拍出误导画面"，而本仓库
//! 没有截图工具（`docs/assets/screenshots/` 里是成品图，不是工具产物）。上游 `gpui-kit` 的测试
//! 装置能直接给出**元素的布局盒**（`window.find(id).bounds()`，`ui.rs:347,352` 已在用），
//! 于是"30×30 还是 32×32"这种问题可以在无头窗口里确定性地判掉 —— 判据比截图更硬、可进 CI。
//!
//! 断言走**生产代码里已有的元素 id**（`ElementId::named_usize("lithe-activity-bar-left", index)`，
//! `activity_bar.rs:441-455`），不为测试改动渲染树。

mod common;

// ⚠️ `IconName` 有两个同名类型：`ActivityItem::new` 收的是 **assets 里生成的那个**
// （`gpui_kit::assets::IconName`，见 `activity_bar.rs:78`），不是 `component::IconName`（Lucide）。
use gpui_kit::assets::IconName;
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AppContext as _, Context, ElementId, IntoElement, Render, TestAppContext, Window, px, size,
};

use lithe_gpui_workbench::activity_bar::{ActivityItem, ActivitySide, activity_bar};

/// 测试宿主：只把活动栏渲染出来，别的什么都不画。
///
/// ⚠️ `items` 必须是**宿主自己的字段**：`activity_bar(..)` 返回的元素借用了那份切片
/// （生产里传的是 `&self.items`，见 `workspace.rs:6030` 一带），放成 `render` 里的局部量会
/// 借不过生命周期。
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

/// 项盒是 **30×30**（真源 `WorkbenchView.swift:1279-1288`，本仓常量 `ITEM_SIZE`）。
///
/// 容差 0.5px：`rems(30./16.)` 在 16px 基准上就是 30，但布局可能按设备像素取整，
/// 所以判"约等于 30"而不是要求逐位相等（避免测试因取整而假红）。
#[gpui_kit::test]
fn activity_bar_item_measures_the_macos_item_box(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);

    let (handle, _host) = common::open_window(cx, Some(size(px(400.), px(400.))), |_window, cx| {
        cx.new(|_cx| BarHost {
            items: vec![ActivityItem::new(IconName::Puzzle, "测试项")],
        })
    });

    // 首帧之前查 `window.find` 会 panic（`ui.rs:71` 同）。
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
