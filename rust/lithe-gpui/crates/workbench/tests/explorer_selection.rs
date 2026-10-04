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
/// ⚠️ **幂等、不删目录**：`#[gpui_kit::test]` 之间是**并行**跑的，先 `remove_dir_all` 再建会让
/// 另一条测试正在用的目录消失（实测踩到过：`create_dir_all` 直接 expect 失败）。
/// 两条测试写的是同一份内容，所以并发写同一个目录是安全的。
///
/// 放在工作区内的临时目录（不是系统 tmp）：Core 的调用在测试进程里对系统 tmp 有过
/// `PermissionDenied` 的历史（见本仓既有的 `TEMP`/`TMP` 约定），用一个显式路径更稳。
fn workspace_dir() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(".artifacts/tmp/explorer-selection-test");
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

/// 记录"写进剪贴板的是什么"的测试替身 —— 平台端口在 explorer 侧是 trait，
/// 所以这里不必碰系统剪贴板（那是**全局状态**，按本仓测试规范不该在测试里读写）。
#[derive(Clone)]
struct RecordingClipboard {
    written: std::sync::Arc<std::sync::Mutex<Vec<PathBuf>>>,
}

impl lithe_gpui_explorer::FileClipboardPort for RecordingClipboard {
    fn write_paths(&self, paths: &[PathBuf]) -> bool {
        self.written
            .lock()
            .expect("记录锁没中毒")
            .extend_from_slice(paths);
        true
    }
}

/// `⌘C`：把选择集按真源口径（**显示顺序**、排除根、选中文件夹只算文件夹本身）
/// 写成**绝对路径**交给平台端口。
///
/// 这条同时验三件事：① `⌘A` 的选择确实传到了 `⌘C` 的处理器；② 载荷用的是**显示顺序**
/// （目录在前：`src, a.rs, b.rs`），不是选择集的字典序（那是 `a.rs, b.rs, src`）；
/// ③ 交给端口的是**绝对路径**。平台侧真正写系统剪贴板的那段不在本测试范围内。
#[gpui_kit::test]
fn command_c_writes_the_selected_items_to_the_clipboard_port(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(|cx| lithe_gpui_workbench::install_actions(cx));
    let root = workspace_dir();
    let written = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let recorded = written.clone();

    let (handle, explorer) =
        common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
            cx.new(|cx| {
                let mut explorer = Explorer::new(root.clone(), Box::new(|_, _, _| {}), window, cx);
                explorer.set_file_clipboard(Box::new(RecordingClipboard { written: recorded }));
                explorer
            })
        });

    cx.run_until_parked();
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
    })
    .unwrap();
    let focus = cx.update(|cx| explorer.read(cx).focus_handle().clone());
    cx.update_window(handle.into(), |_, window, cx| {
        window.focus(&focus, cx);
    })
    .unwrap();

    cx.simulate_keystrokes(handle.into(), "cmd-a");
    cx.simulate_keystrokes(handle.into(), "cmd-c");

    let absolute = written.lock().expect("记录锁没中毒").clone();
    assert_eq!(
        absolute,
        vec![root.join("src"), root.join("a.rs"), root.join("b.rs")],
        "载荷应当是**显示顺序**、排除根、且为绝对路径"
    );
}
