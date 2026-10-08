mod common;

use std::path::PathBuf;

use gpui::{AppContext as _, TestAppContext, px, size};
use gpui_kit::test::TestWindowExt as _;

use lithe_gpui_explorer::Explorer;

/// A workspace with one folder to collapse and room for one file to appear later.
fn workspace_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(".artifacts/tmp/explorer-header-test");
    std::fs::create_dir_all(root.join("src")).expect("建测试工作区");
    std::fs::write(root.join("a.rs"), "fn a() {}\n").expect("写 a.rs");
    std::fs::write(root.join("src/lib.rs"), "pub fn lib() {}\n").expect("写 lib.rs");
    let _ = std::fs::remove_file(root.join("added.rs"));
    root
}

/// The panel header's two actions — 「全部折叠」与「刷新」——through the same methods its buttons
/// call. The buttons themselves are only reachable by pointer, so this lane drives the actions.
#[gpui::test]
fn the_panel_actions_collapse_every_folder_and_refresh_the_snapshot(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    // 项目树是组件库的控件，渲染前要装组件库主题（与生产组合根同一入口）。
    cx.update(lithe_gpui_workbench::component_theme::install_component_theme);
    cx.update(|cx| lithe_gpui_workbench::install_actions(cx));
    let root = workspace_dir();

    let (handle, explorer) =
        common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
            cx.new(|cx| Explorer::new(root.clone(), Box::new(|_, _, _| {}), window, cx))
        });

    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
    })
    .unwrap();

    // 定位一个嵌套文件：祖先目录因此展开，"全部折叠"才有东西可折。
    cx.update(|cx| {
        let _ = explorer.update(cx, |explorer, cx| {
            explorer.reveal(&root.join("src/lib.rs"), cx)
        });
    });
    let expanded = cx.update(|cx| explorer.read(cx).tree_rows(cx));
    assert_eq!(
        expanded, 4,
        "根 / src / a.rs / src/lib.rs 四行，实际 {expanded}"
    );

    cx.update(|cx| explorer.update(cx, |explorer, cx| explorer.collapse_all(cx)));
    let collapsed = cx.update(|cx| explorer.read(cx).tree_rows(cx));
    assert_eq!(collapsed, 3, "折叠后只剩根与它的两个子项，实际 {collapsed}");

    // 刷新：磁盘上新出现的文件要进树。
    std::fs::write(root.join("added.rs"), "fn added() {}\n").expect("写 added.rs");
    cx.update_window(handle.into(), |_, window, cx| {
        let _ = explorer.update(cx, |explorer, cx| explorer.refresh(window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    let refreshed = cx.update(|cx| explorer.read(cx).tree_rows(cx));
    assert_eq!(
        refreshed,
        collapsed + 1,
        "刷新后多出 added.rs 一行，实际 {refreshed}"
    );
}
