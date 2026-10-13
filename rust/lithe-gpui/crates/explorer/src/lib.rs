mod explorer_view;
mod model;

pub use explorer_view::{
    DeleteHost, Explorer, HidePanelHost, NewHost, RenameHost, RevealInFileManagerHost,
    is_windows_reserved,
};

gpui::actions!(
    lithe_gpui_explorer,
    [SelectAllInTree, CopySelection, RenameSelection]
);

pub const EXPLORER_KEY_CONTEXT: &str = "Explorer";

pub fn install_actions(cx: &mut gpui::App) {
    // `secondary-` is gpui's platform switch (Command on macOS, Control elsewhere), which is what
    // the keymap's `DEFAULT_BINDINGS` already assumes; the old `cmd-a` / `ctrl-a` double write was
    // the legacy dialect that table says not to copy.
    cx.bind_keys([
        gpui::KeyBinding::new("secondary-a", SelectAllInTree, Some(EXPLORER_KEY_CONTEXT)),
        gpui::KeyBinding::new("secondary-c", CopySelection, Some(EXPLORER_KEY_CONTEXT)),
        // IDEA's in-place rename: the row the keyboard is on starts editing. The bare `Delete` key
        // is deliberately left unbound (deleting has no recycle bin yet).
        gpui::KeyBinding::new("f2", RenameSelection, Some(EXPLORER_KEY_CONTEXT)),
    ]);
}

pub trait FileClipboardPort: 'static {
    fn write_paths(&self, paths: &[std::path::PathBuf]) -> bool;
}
