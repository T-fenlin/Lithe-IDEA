mod common;

use std::path::PathBuf;

use gpui::{AppContext as _, TestAppContext, px, size};

use lithe_gpui_explorer::Explorer;

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

#[gpui::test]
fn command_a_without_focus_selects_the_top_level_items(cx: &mut TestAppContext) {
    // 项目树现在是组件库的控件，渲染前必须由组合根那一套把组件库主题装上
    // （生产路径是 `crates/app/src/main.rs`；测试里走同一个入口）。
    cx.update(|cx| lithe_gpui_workbench::install_actions(cx));
    let root = workspace_dir();

    let (handle, explorer) =
        common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
            cx.new(|cx| Explorer::new(root.clone(), Box::new(|_, _, _| {}), window, cx))
        });

    cx.run_until_parked();
    // 补一帧：上游那套观察包装退场后，帧由 gpui 自己的 `refresh` + `draw` 出。
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();

    let focus = cx.update(|cx| explorer.read(cx).focus_handle(cx));
    cx.update_window(handle.into(), |_, window, cx| {
        window.focus(&focus, cx);
    })
    .unwrap();

    cx.simulate_keystrokes(handle.into(), "cmd-a");

    cx.update(|cx| {
        assert_eq!(
            explorer.read(cx).selected_paths(cx),
            ["a.rs", "b.rs", "src"],
            "⌘A 没有焦点时应当选中根的子项（真源 `?? root`）"
        );
    });
}

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

#[gpui::test]
fn command_c_writes_the_selected_items_to_the_clipboard_port(cx: &mut TestAppContext) {
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
    // 补一帧：上游那套观察包装退场后，帧由 gpui 自己的 `refresh` + `draw` 出。
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
    let focus = cx.update(|cx| explorer.read(cx).focus_handle(cx));
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
