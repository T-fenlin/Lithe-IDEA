mod buffer;
mod code_actions;
mod completion;
mod diagnostics;
mod editor_view;
mod go_to_line;
mod hover;
mod navigation;
mod problems;

pub use editor_view::{
    BufferWriteHost, CursorPosition, DebugGutterHost, EDITOR_HOST_ELEMENT_ID, EditorPane,
    FIND_BAR_KEY_CONTEXT, GutterBreakpoint, SessionRestore, TabMenuHostActions,
};
pub use problems::ProblemsPane;

use gpui::{App, KeyBinding};

gpui::actions!(
    lithe_editor,
    [
        SaveBuffer,
        NavigateToDefinition,
        CloseActiveTab,
        ShowCodeActions
    ]
);

// 查找 / 替换的动作。声明在 editor（渲染方持有查找条与其 key context），workbench 绑键时引用。
// `Escape` 的 handler 挂在查找条容器上，`F3` / `Shift+F3` 用全窗绑定，所以四个动作都在这里。
gpui::actions!(
    lithe_editor,
    [OpenFind, OpenFindReplace, FindNext, FindPrevious, CloseFind]
);

pub fn install_actions(cx: &mut App) {
    // 字面量一律 `secondary-`：它是 gpui 的平台归一化开关（macOS → Command，其他 → Control），
    // 也是 `settings/src/keymap.rs::DEFAULT_BINDINGS` 已经假设的形态。旧写的 `ctrl-` 在 macOS 上
    // 让「设置页显示的（⌘S）」与「真能按出来的（Ctrl+S）」分成了两套（F2）。
    let mut bindings = vec![
        KeyBinding::new("secondary-s", SaveBuffer, None),
        KeyBinding::new("f12", NavigateToDefinition, None),
        KeyBinding::new("secondary-w", CloseActiveTab, None),
        // 代码动作：与上游 `gpui-base` 的 `ToggleCodeActions` 同一个键（`state.rs` 里
        // `secondary-.` 绑在编辑器的上下文上）。组件库没有"请求代码动作"的事件 ——
        // 它不认识语言服务 —— 所以这个入口由宿主自己绑、自己取数（见 `src/code_actions.rs`）。
        KeyBinding::new("secondary-.", ShowCodeActions, None),
    ];
    if cfg!(target_os = "macos") {
        bindings.push(KeyBinding::new("cmd-.", ShowCodeActions, None));
    }
    cx.bind_keys(bindings);
}
