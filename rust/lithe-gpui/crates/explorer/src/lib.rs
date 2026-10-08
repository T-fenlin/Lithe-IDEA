mod explorer_view;
mod model;

pub use explorer_view::{Explorer, HidePanelHost, RevealInFileManagerHost};

gpui::actions!(lithe_gpui_explorer, [SelectAllInTree, CopySelection]);

pub const EXPLORER_KEY_CONTEXT: &str = "Explorer";

pub fn install_actions(cx: &mut gpui::App) {
    cx.bind_keys([
        gpui::KeyBinding::new("cmd-a", SelectAllInTree, Some(EXPLORER_KEY_CONTEXT)),
        gpui::KeyBinding::new("ctrl-a", SelectAllInTree, Some(EXPLORER_KEY_CONTEXT)),
        gpui::KeyBinding::new("cmd-c", CopySelection, Some(EXPLORER_KEY_CONTEXT)),
        gpui::KeyBinding::new("ctrl-c", CopySelection, Some(EXPLORER_KEY_CONTEXT)),
    ]);
}

pub trait FileClipboardPort: 'static {
    fn write_paths(&self, paths: &[std::path::PathBuf]) -> bool;
}
