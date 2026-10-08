use std::cell::RefCell;

use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::command::{Command, CommandItem, CommandState};
use gpui::{App, AppContext as _, Context, Entity, IntoElement, KeyBinding, ParentElement as _, Render, SharedString, Styled as _, WeakEntity, Window, div, rems};
use gpui::{DefiniteLength};

use lithe_gpui_shared::tr;
use lithe_ui::overlays::Dialog;
use lithe_ui::shell::{WindowRootExt as _, dismiss_hosted};

use crate::workspace::ShellWorkspace;

thread_local! {
    static SHELL: RefCell<Option<WeakEntity<ShellWorkspace>>> = const { RefCell::new(None) };
}

pub(crate) fn set_shell(shell: WeakEntity<ShellWorkspace>) {
    SHELL.with(|slot| *slot.borrow_mut() = Some(shell));
}

pub(crate) fn shell() -> Option<WeakEntity<ShellWorkspace>> {
    SHELL.with(|slot| slot.borrow().clone())
}

thread_local! {
    static SHELL_FOCUS: RefCell<Option<gpui_kit::FocusHandle>> = const { RefCell::new(None) };
}

pub(crate) fn set_shell_focus(focus: gpui_kit::FocusHandle) {
    SHELL_FOCUS.with(|slot| *slot.borrow_mut() = Some(focus));
}

pub fn shell_focus() -> Option<gpui_kit::FocusHandle> {
    SHELL_FOCUS.with(|slot| slot.borrow().clone())
}

thread_local! {
    static PENDING_FOCUS: RefCell<bool> = const { RefCell::new(false) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandId {
    OpenSettings,
    OpenAppearanceSettings,
    SwitchToLightTheme,
    SwitchToDarkTheme,
    ToggleTerminal,
    ToggleMaven,
    ToggleStatusBar,
    ToggleMenuBar,
    SaveBuffer,
    OpenCommandPalette,
    OpenQuickOpen,
    OpenSearchEverywhere,
    OpenReplaceInFiles,
    NavigateToDefinition,
    OpenFile,
    CloseTab,
    CloseOtherTabs,
    CloseAllTabs,
    CloseSavedTabs,
    CloseTabsToLeft,
    CloseTabsToRight,
    ReopenClosedTab,
    NewWindow,
}

impl CommandId {
    pub const ALL: [CommandId; 23] = [
        Self::OpenSettings,
        Self::OpenAppearanceSettings,
        Self::SwitchToLightTheme,
        Self::SwitchToDarkTheme,
        Self::ToggleTerminal,
        Self::ToggleMaven,
        Self::ToggleStatusBar,
        Self::ToggleMenuBar,
        Self::SaveBuffer,
        Self::OpenCommandPalette,
        Self::OpenQuickOpen,
        Self::OpenSearchEverywhere,
        Self::OpenReplaceInFiles,
        Self::NavigateToDefinition,
        Self::OpenFile,
        Self::CloseTab,
        Self::CloseOtherTabs,
        Self::CloseAllTabs,
        Self::CloseSavedTabs,
        Self::CloseTabsToLeft,
        Self::CloseTabsToRight,
        Self::ReopenClosedTab,
        Self::NewWindow,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::OpenSettings => "open-settings",
            Self::OpenAppearanceSettings => "open-appearance-settings",
            Self::SwitchToLightTheme => "switch-theme-light",
            Self::SwitchToDarkTheme => "switch-theme-dark",
            Self::ToggleTerminal => "toggle-terminal",
            Self::ToggleMaven => "toggle-maven",
            Self::ToggleStatusBar => "toggle-status-bar",
            Self::ToggleMenuBar => "toggle-menu-bar",
            Self::SaveBuffer => "save-buffer",
            Self::OpenCommandPalette => "open-command-palette",
            Self::OpenQuickOpen => "open-quick-open",
            Self::OpenSearchEverywhere => "open-search-everywhere",
            Self::OpenReplaceInFiles => "open-replace-in-files",
            Self::NavigateToDefinition => "navigate-to-definition",
            Self::OpenFile => "open-file",
            Self::CloseTab => "close-tab",
            Self::CloseOtherTabs => "close-other-tabs",
            Self::CloseAllTabs => "close-all-tabs",
            Self::CloseSavedTabs => "close-saved-tabs",
            Self::CloseTabsToLeft => "close-tabs-to-left",
            Self::CloseTabsToRight => "close-tabs-to-right",
            Self::ReopenClosedTab => "reopen-closed-tab",
            Self::NewWindow => "new-window",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Category {
    Settings,
    View,
}

impl Category {
    pub(crate) fn label(self) -> SharedString {
        match self {
            Self::Settings => tr("lithe.commandPalette.categories.Settings"),
            Self::View => tr("lithe.commandPalette.categories.View"),
        }
    }
}

pub(crate) struct CommandAction {
    pub(crate) id: CommandId,
    pub(crate) category: Category,
    pub(crate) label: SharedString,
    pub(crate) description: SharedString,
    pub(crate) icon: gpui_kit::component::Icon,
}

impl CommandAction {
    pub(crate) fn category_label(&self) -> SharedString {
        self.category.label()
    }
}

pub fn install_actions(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-shift-p", OpenCommandPalette, None),
        KeyBinding::new("ctrl-m", ToggleMenuBar, None),
    ]);
    cx.on_action(|_: &OpenCommandPalette, cx: &mut App| {
        let Some(window_handle) = cx.active_window() else {
            return;
        };
        let _ = window_handle.update(cx, |_, window, cx| open_command_palette(window, cx));
    });
    cx.on_action(|_: &ToggleMenuBar, cx: &mut App| crate::menu_bar::toggle_menu_bar(cx));
}

gpui_kit::actions!(lithe_workbench, [OpenCommandPalette, ToggleMenuBar]);

const PANEL_WIDTH: f32 = 704.;
const PANEL_MAX_HEIGHT: f32 = 512.;
const PANEL_TOP_INSET: f32 = 16.;

pub fn open_command_palette(window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        return;
    }
    let action_count = shell()
        .and_then(|shell| shell.upgrade())
        .map(|shell| shell.read(cx).command_actions(cx).len())
        .unwrap_or(0);
    println!("S1_COMMAND_PALETTE opened=true actions={action_count}");

    let palette = cx.new(|cx| CommandPalette::new(window, cx));
    PENDING_FOCUS.with(|slot| *slot.borrow_mut() = true);
    window.open_dialog(
        "lithe-command-palette",
        move |id, window, _cx| {
            let rem = window.rem_size();
            // 面板态：顶部对齐 + 自定义宽度与内衬，等价于上游 Dialog 的
            // `width(..).margin_top(..).p_0()` 与 `content(|c| c.p_0().gap_0())`。
            Dialog::new(id)
                .panel()
                .on_close(dismiss_hosted)
                .w(crate::rem_px(rem, PANEL_WIDTH))
                .p_0()
                .gap_0()
                .top_inset(crate::rem_px(rem, PANEL_TOP_INSET))
                .child(palette.clone())
                .into_any_element()
        },
        cx,
    );
}

pub struct CommandPalette {
    command: Entity<CommandState>,
}

impl CommandPalette {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            command: cx.new(|cx| CommandState::new(window, cx)),
        }
    }

    fn confirm(&mut self, row: usize, window: &mut Window, cx: &mut Context<Self>) {
        window.close_dialog(cx);
        let Some(shell) = shell().and_then(|shell| shell.upgrade()) else {
            return;
        };
        shell.update(cx, |shell, cx| shell.run_command(row, window, cx));
    }
}

impl Render for CommandPalette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let needs_focus = PENDING_FOCUS.with(|slot| {
            let pending = *slot.borrow();
            *slot.borrow_mut() = false;
            pending
        });
        if needs_focus {
            self.command
                .update(cx, |command, cx| command.focus(window, cx));
            println!("S1_COMMAND_PALETTE focus=search");
        }

        let actions = shell()
            .and_then(|shell| shell.upgrade())
            .map(|shell| shell.read(cx).command_actions(cx))
            .unwrap_or_default();

        Command::new(&self.command)
            .bordered(false)
            .w_full()
            .placeholder(tr("lithe.commandPalette.placeholder"))
            .max_h(DefiniteLength::from(rems(PANEL_MAX_HEIGHT / 16.)))
            .items(actions.into_iter().map(IntoCommandItem::into_command_item))
            .empty(|_, _, cx| {
                div()
                    .py_6()
                    .w_full()
                    .text_center()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr("lithe.commandPalette.noCommands"))
            })
            .on_confirm({
                let palette = cx.entity();
                move |index, window, cx| {
                    palette.update(cx, |palette, cx| {
                        palette.confirm(index.row, window, cx);
                    });
                }
            })
    }
}

trait IntoCommandItem {
    fn into_command_item(self) -> CommandItem;
}

impl IntoCommandItem for CommandAction {
    fn into_command_item(self) -> CommandItem {
        let search_category = self.category_label();
        let CommandAction {
            id,
            label,
            description,
            icon,
            ..
        } = self;
        let search_key: SharedString = id.id().into();
        let item_label = label.clone();
        let item_description = description.clone();
        CommandItem::new()
            .label(label)
            .keywords([description, search_key, search_category])
            .icon(icon)
            .child(move |_, cx| {
                v_flex()
                    .w_full()
                    .min_w_0()
                    .min_h_8()
                    .justify_center()
                    .gap_0p5()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(item_label.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(item_description.clone()),
                    )
                    .into_any_element()
            })
    }
}
