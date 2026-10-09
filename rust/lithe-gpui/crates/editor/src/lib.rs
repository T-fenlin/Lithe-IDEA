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
    GutterBreakpoint, SessionRestore, TabMenuHostActions,
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

pub fn install_actions(cx: &mut App) {
    let mut bindings = vec![
        KeyBinding::new("ctrl-s", SaveBuffer, None),
        KeyBinding::new("f12", NavigateToDefinition, None),
        KeyBinding::new("ctrl-w", CloseActiveTab, None),
        // 代码动作：与上游 `gpui-base` 的 `ToggleCodeActions` 同一个键（`state.rs` 里
        // `cmd-.` / `ctrl-.` 绑在编辑器的上下文上）。组件库没有"请求代码动作"的事件 ——
        // 它不认识语言服务 —— 所以这个入口由宿主自己绑、自己取数（见 `src/code_actions.rs`）。
        KeyBinding::new("ctrl-.", ShowCodeActions, None),
    ];
    if cfg!(target_os = "macos") {
        bindings.push(KeyBinding::new("cmd-.", ShowCodeActions, None));
    }
    cx.bind_keys(bindings);
}
