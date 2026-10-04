mod buffer;
mod code_actions;
mod completion;
mod diagnostics;
mod editor_view;
mod go_to_line;
mod hover;
mod languages;
mod navigation;
mod problems;

pub use editor_view::{
    BufferWriteHost, CursorPosition, DebugGutterHost, EditorPane, GutterBreakpoint, SessionRestore,
    TabMenuHostActions,
};
pub use problems::ProblemsPane;

use gpui_kit::{App, KeyBinding};

gpui_kit::actions!(
    lithe_editor,
    [SaveBuffer, NavigateToDefinition, CloseActiveTab]
);

pub fn install_actions(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-s", SaveBuffer, None),
        KeyBinding::new("f12", NavigateToDefinition, None),
        KeyBinding::new("ctrl-w", CloseActiveTab, None),
    ]);
}
