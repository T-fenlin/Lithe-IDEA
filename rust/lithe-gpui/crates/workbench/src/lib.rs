pub mod activity_bar;
pub mod branch_panel;
pub mod command_palette;
pub mod component_theme;
pub mod debug;
pub mod debug_session;
pub mod debug_transport;
pub mod extensions_page;
pub mod maven;
pub mod menu_bar;
pub mod notifications;
pub mod project_menu;
pub mod project_tabs;
pub mod quick_open;
pub mod replace_in_files;
pub mod right_tool_window;
pub mod search_everywhere;
pub mod search_index;
pub mod search_model;
pub mod search_panel;
pub mod session;
pub mod spring;
pub mod spring_panel;
pub mod spring_paths;
pub mod status_bar;
pub mod title_bar;
pub mod welcome;
pub mod workspace;

pub use welcome::{ProjectOpener, WelcomeRow, WelcomeView};
pub use workspace::{
    OpenDestination, ShellWorkspace, install_open_project_action, left_activity_index,
    set_shell_startup,
};

pub fn install_actions(cx: &mut gpui_kit::App) {
    lithe_gpui_explorer::install_actions(cx);
}

pub(crate) use lithe_gpui_shared::rem_px;
