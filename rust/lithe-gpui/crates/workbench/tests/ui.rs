//! Lithe 的 UI 集成测试层：把**真实视图**渲染进无头窗口，断言它真的画出来了。
//!
//! 这是任务表 `D26` 的落点，也是本仓库**第一处**能在窗口里跑交互的测试
//! （此前 `#[cfg(test)]` 全是纯函数单测，注释多处写明"本 crate 拿不到 `TestAppContext`"）。
//! 形状照上游 `gpui-kit-0.7.0/tests/ui.rs` + `tests/common/mod.rs` 抄；
//! 判据与可用的断言接口见 `docs/gpui kit/0.7.0/docs/test.md`。
//!
//! ⚠️ 关于"要不要给生产视图加 `.test_support()`"：`TestSupportExt::test_support()` 对
//! 原生 div 只注册观察、不增加布局盒子，**启用与否都能调用**（上游 `test.md` 的说明）。
//! 本测试**不需要**它 —— 断言走的是 `EditorPane` 自己的公开访问器 + 窗口真的渲染成功，
//! 所以不必为了让测试好写而改动生产渲染树（那正是 `test.md` 明确劝阻的做法：
//! "不要仅为让视觉断言通过而添加 aria_label"。）

mod common;

use std::path::PathBuf;

// ⚠️ 这两个 trait 都必须显式引入，缺一个就编译不过（实测）：
// `AppContext` 提供 `cx.new(..)` 与 `cx.update_window(..)`（**不是**
// `TestAppContextExt` —— 那个在 0.7.0 里只提供 `wait_for`，本测试是同步的用不到）；
// `TestWindowExt` 提供 `window.render_frame(..)`。
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AppContext as _, TestAppContext, px, size};

/// 建一个临时文件并在测试结束时删掉它（断言失败也要删，所以用 `Drop` 而不是结尾手动 remove）。
struct TempFile {
    path: PathBuf,
}

impl TempFile {
    /// 在系统临时目录下写一个内容确定的文件（文件名带 pid，避免并发测试互相覆盖）。
    fn new(name: &str, contents: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("lithe-ui-test-{}-{name}", std::process::id()));
        std::fs::write(&path, contents).expect("写临时文件");
        Self { path }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// 打开一个真实文件 → 视图真的渲染出来 → 缩进宽度真的传到了视图里。
///
/// 这条同时钉住三件事，而这三件事此前都**只有真机手测**：
/// 1. `EditorPane` 能在无头窗口里渲染（渲染树不 panic、不依赖真实窗口）；
/// 2. 打开文件后活动 buffer 是"可写"的（`active_buffer_is_readonly()` 为 `false`），
///    以及 `tab_size()` 的默认档就是 2；
/// 3. `set_tab_size(n)` 立刻被视图接受（D13 的状态栏缩进格读的就是同一个值）。
#[gpui_kit::test]
fn editor_pane_renders_a_real_file_and_reports_its_indent(cx: &mut TestAppContext) {
    let file = TempFile::new("indent.md", "# 标题\n\n正文\n");

    // 组件库得先初始化（主题、字体、组件注册），否则渲染期取主题会 panic。
    cx.update(gpui_kit::init);

    let (handle, pane) = common::open_window(cx, Some(size(px(640.), px(480.))), |window, cx| {
        cx.new(|cx| lithe_gpui_editor::EditorPane::new(None, window, cx))
    });

    // ① 打开文件（需要 `&mut Window`，所以写在 update_content 里）。
    let path = file.path.clone();
    common::update_content(handle, &pane, cx, |pane, window, cx| {
        pane.open(&path, window, cx);
    })
    .unwrap();

    // ② 真的画出一帧（首帧之前查 `window.find` 会 panic）。
    cx.update_window(handle.into(), |_, window, cx| {
        window.render_frame(cx);
    })
    .unwrap();

    // ③ 视图的公开真值：刚打开的文件是可写的，缩进是默认档 2。
    cx.update(|cx| {
        let pane = pane.read(cx);
        assert_eq!(
            pane.tab_size(),
            2,
            "新视图的缩进宽度应当是默认档 2（设置页 tabSize 的默认值）"
        );
        assert!(
            !pane.active_buffer_is_readonly(),
            "刚打开的普通 markdown 文件必须是可写的（只读格的判据取 Buffer::writable）"
        );
        assert_eq!(
            pane.active_buffer_name(),
            "lithe-ui-test-".to_string() + &std::process::id().to_string() + "-indent.md",
            "活动标签应当是刚打开的那个文件"
        );
    });

    // ④ 改缩进宽度：视图立刻接受（真机路径是设置变化 → 外壳 → set_tab_size）。
    common::update_content(handle, &pane, cx, |pane, _window, cx| {
        pane.set_tab_size(8, cx);
    })
    .unwrap();
    cx.update(|cx| {
        assert_eq!(
            pane.read(cx).tab_size(),
            8,
            "set_tab_size 之后访问器必须返回新值（状态栏缩进格读的就是它）"
        );
    });
}
