pub mod account;
pub mod agent;
mod assets;

pub mod buttons;
pub mod calendar;
pub mod canvas;
pub mod charts;
pub mod chat;
pub mod collab;
pub mod dashboard;
pub mod data_display;
pub mod debug;
pub mod devtools;
pub mod documents;
pub mod editor;
pub mod feedback;
pub mod files;
pub mod finance;
pub mod forms;
pub mod generative;
pub mod git;
rust_i18n::i18n!("locales", fallback = "en-US");

pub mod i18n;
pub mod interaction;
pub mod language;
pub mod layout;
pub mod lists;
pub mod mail;
pub mod maps;
pub mod media;
pub mod menus;
pub mod messaging;
pub mod misc;
pub mod motion;
pub mod navigation;
pub mod onboarding;
pub mod overlays;
pub mod primitives;
pub mod project;
pub mod rendering;
pub mod settings;
pub mod shell;
pub mod tables;
pub mod terminal;
pub mod theme;
pub mod tooling;
pub mod typography;

pub use assets::Assets;

use gpui::{App, KeyBinding};

use crate::primitives::{FocusNext, FocusPrev, IconName};

/// Loads fonts and the theme, binds Tab and text keys. Call once, first.
pub fn init(cx: &mut App) {
    match cx.asset_source().load(IconName::Check.path()) {
        Ok(Some(_)) => {}
        Ok(None) => panic!("lithe-ui: pass `lithe_ui::Assets` to `Application::with_assets`"),
        Err(error) => panic!("lithe-ui: asset source failed: {error:#}"),
    }
    assets::load_fonts(cx).expect("lithe-ui: embedded fonts failed to register");
    theme::Theme::init(cx);
    install_key_bindings(cx);
    #[cfg(not(target_family = "wasm"))]
    terminal::bind_keys(cx);
    documents::bind_keys(cx);
}

/// Binds the keys of the components a host is expected to use: Tab focus, the text input and the
/// code editor. It deliberately does **not** load assets, fonts or the theme.
///
/// ## Why this entry point exists
///
/// [`init`] opens by loading `IconName::Check.path()` (`icons/check.svg`) and panics when the host's
/// asset source does not contain it. A host that ships its own asset folder, fonts and theme
/// pipeline — such as `lithe-gpui`, which registers `LitheAssets` and calls `Theme::init` itself —
/// cannot call [`init`] without either duplicating lithe-ui's assets or crashing at startup. What
/// such a host still needs is the key bindings: without them the component's `on_action` handlers
/// are never reached, because gpui only dispatches actions that are in the keymap.
///
/// `terminal::bind_keys` and `documents::bind_keys` are **not** part of it: a host that owns its own
/// terminal or document widgets should bind those itself (or not at all). Binding them here would
/// register actions nothing in the host handles.
pub fn install_key_bindings(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", FocusNext, None),
        KeyBinding::new("shift-tab", FocusPrev, None),
    ]);
    forms::bind_keys(cx);
    editor::bind_keys(cx);
}

#[cfg(test)]
mod tests {
    use super::install_key_bindings;
    use gpui::{App, TestAppContext};

    /// `install_key_bindings` 不碰资产源，所以能在无头测试里跑——这正是它存在的理由
    /// （`init` 第一件事就是 `load(IconName::Check.path())`，在这里必 panic）。
    ///
    /// 断言按「三组各取一条」写：焦点组无 context、输入组带 `ElyInput`、编辑器组带
    /// `ElyEditor`。只断言总数会放过「装了但 context 丢了」这种退化。
    #[gpui::test]
    fn install_key_bindings_works_without_an_asset_source(cx: &mut TestAppContext) {
        cx.update(|cx| {
            install_key_bindings(cx);

            let keymap = cx.key_bindings();
            let bindings = keymap.borrow();
            let total = bindings.bindings().len();
            assert!(total > 30, "键位数量少得可疑：只装了 {total} 条");

            let focus = bindings
                .bindings()
                .any(|binding| binding.action().name() == "lithe::FocusNext");
            assert!(focus, "Tab 焦点切换的动作没装上");

            let input = bindings
                .bindings()
                .any(|binding| binding.action().name().starts_with("ely_input::"));
            assert!(input, "输入框那批键位没装上");

            let editor = bindings
                .bindings()
                .any(|binding| binding.action().name().starts_with("ely_editor::"));
            assert!(editor, "编辑器那批键位没装上");

            // context 必须跟着一起装：带 context 的键位若丢了谓词，会退化成全窗绑定。
            for context in ["ElyInput", "ElyEditor"] {
                let scoped = bindings.bindings().any(|binding| {
                    binding
                        .predicate()
                        .is_some_and(|predicate| predicate.to_string() == context)
                });
                assert!(scoped, "{context} 这组键位丢了 context 谓词");
            }

            // terminal / documents 不在这个入口里：宿主有自己的终端，装了没人接。
            let stray = bindings.bindings().any(|binding| {
                let name = binding.action().name();
                name.starts_with("ely_terminal::") || name.starts_with("ely_documents::")
            });
            assert!(!stray, "这个入口不该装终端 / 文档那两组键位");
        });
    }
}
