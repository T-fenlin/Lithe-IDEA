//! 项目树 `⌘A` 的**行为级**测试：把键盘真的按下去，看选择集变成什么。
//!
//! 为什么值得单独一条：批 4 的键位链路跨三处（app 登记绑定 → 视图 `key_context` → 处理器），
//! 任何一处断了都"编译通过但按键无效"。此前只有编译与纯函数单测，覆盖不到这条链。
//!
//! 判据：真源 `ProjectTreeSelection.selectAll`（`ProjectTreeSelection.swift:59-70`）——
//! **没有焦点时**取**根的子项**（`?? root`）。所以"首帧后直接按 ⌘A"的正确结果是顶层项全选。
//!
//! ⚠️ 覆盖边界（不假装更全）：本测试**不**覆盖"行点击把焦点交给树"那一步 ——
//! 测试读不到行的元素 id（`explorer_row_element_id` 是私有的），所以焦点是直接给的。
//! 也不覆盖 Shift 区间：`simulate_mouse_down(position, button)` 在这版测试装置里
//! **没有修饰键参数**（`gpui-pre-0.3.7/src/app/test_app.rs:421`）。

mod common;

use std::path::PathBuf;

use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AppContext as _, TestAppContext, px, size};

use lithe_gpui_explorer::Explorer;

/// 造一个确定形状的工作区：`src/`（目录）+ 两个顶层文件。
///
/// 放在工作区内的临时目录（不是系统 tmp）：Core 的调用在测试进程里对系统 tmp 有过
/// `PermissionDenied` 的历史（见本仓既有的 `TEMP`/`TMP` 约定），用一个显式路径更稳。
fn workspace_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(".artifacts/tmp/explorer-selection-test");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("建测试工作区");
    std::fs::write(root.join("a.rs"), "fn a() {}\n").expect("写 a.rs");
    std::fs::write(root.join("b.rs"), "fn b() {}\n").expect("写 b.rs");
    std::fs::write(root.join("src/lib.rs"), "pub fn lib() {}\n").expect("写 lib.rs");
    root
}

/// 首帧后按 `⌘A`：没有焦点 ⇒ 选中**根的子项**（目录在前、再按名字小写升序）。
#[gpui_kit::test]
fn command_a_without_focus_selects_the_top_level_items(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    // 键位**登记**：真机由 app 调 `lithe_gpui_workbench::install_actions`（见 `app/src/main.rs`
    // 里那一行）；app 是二进制、测试不会跑它，所以这里显式调一次 —— 这样测的才是
    // "登记 → key context → 处理器"整条链，而不是"处理器单独能不能跑"。
    cx.update(|cx| lithe_gpui_workbench::install_actions(cx));
    let root = workspace_dir();

    let (handle, explorer) =
        common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
            cx.new(|cx| Explorer::new(root.clone(), Box::new(|_, _, _| {}), window, cx))
        });

    // 快照是**异步**加载的（`explorer_view.rs:488` 的 `cx.spawn`）：等它跑完再断言。
    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
    })
    .unwrap();

    // 焦点直接给树（真机路径是行点击 —— 那一步本测试覆盖不到，见文件头的边界说明）。
    // 先把句柄取出来：`explorer.read(cx)` 与 `window.focus(.., cx)` 会同时借 `cx`。
    let focus = cx.update(|cx| explorer.read(cx).focus_handle().clone());
    cx.update_window(handle.into(), |_, window, cx| {
        window.focus(&focus, cx);
    })
    .unwrap();

    // 按下 ⌘A：绑定登记在 `lithe_gpui_explorer::install_actions`、处理在树的 key context 上。
    cx.simulate_keystrokes(handle.into(), "cmd-a");

    cx.update(|cx| {
        assert_eq!(
            explorer.read(cx).selected_paths(),
            ["a.rs", "b.rs", "src"],
            "⌘A 没有焦点时应当选中根的子项（真源 `?? root`）"
        );
    });
}
