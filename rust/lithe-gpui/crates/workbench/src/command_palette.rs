use std::cell::RefCell;

use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, KeyBinding, ParentElement as _, Render,
    SharedString, Styled as _, Subscription, WeakEntity, Window, div, rems,
};
use lithe_gpui_shared::tr;
use lithe_ui::forms::{InputEvent, InputState};
use lithe_ui::layout::v_stack;
use lithe_ui::navigation::{Palette, PaletteRow};
use lithe_ui::overlays::Dialog;
use lithe_ui::primitives::IconElement;
use lithe_ui::shell::{WindowRootExt as _, dismiss_hosted};
use lithe_ui::theme::ActiveTheme as _;

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
    static SHELL_FOCUS: RefCell<Option<gpui::FocusHandle>> = const { RefCell::new(None) };
}

pub(crate) fn set_shell_focus(focus: gpui::FocusHandle) {
    SHELL_FOCUS.with(|slot| *slot.borrow_mut() = Some(focus));
}

pub fn shell_focus() -> Option<gpui::FocusHandle> {
    SHELL_FOCUS.with(|slot| slot.borrow().clone())
}

thread_local! {
    static PENDING_FOCUS: RefCell<bool> = const { RefCell::new(false) };
}

/// 命令目录的真源在 `lithe_gpui_shared`：设置页要枚举全部命令，而 `settings` 够不到
/// `workbench`（依赖方向 `settings → shared`、`workbench → settings + shared`）。
///
/// 这里原样再导出，`crate::command_palette::CommandId` 这个路径对所有引用方保持有效。
/// 带 i18n 文案和图标的那一层（[`CommandAction`]）留在本地，不要跟着搬。
pub use lithe_gpui_shared::commands::CommandId;

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
    /// 应用图标，取自本仓 `assets/icons/idea/**`（见 `shared::icons` 的图标口径）。
    ///
    /// 是**现取**的工厂而不是元素：`is_dark` 要等画的时候问主题。没有对应图标的那几条留空。
    pub(crate) icon: Option<IconElement>,
}

impl CommandAction {
    pub(crate) fn category_label(&self) -> SharedString {
        self.category.label()
    }
}

pub fn install_actions(cx: &mut App) {
    // `secondary-` 是 gpui 的平台归一化口径（macOS ⌘⇧P / ⌘M，其他 Ctrl+Shift+P / Ctrl+M），
    // 与 `settings/src/keymap.rs::DEFAULT_BINDINGS` 报的一致；旧写的 `ctrl-` 在 macOS 上错位（F2）。
    cx.bind_keys([
        KeyBinding::new("secondary-shift-p", OpenCommandPalette, None),
        KeyBinding::new("secondary-m", ToggleMenuBar, None),
    ]);
    cx.on_action(|_: &OpenCommandPalette, cx: &mut App| {
        let Some(window_handle) = cx.active_window() else {
            return;
        };
        let _ = window_handle.update(cx, |_, window, cx| open_command_palette(window, cx));
    });
    cx.on_action(|_: &ToggleMenuBar, cx: &mut App| crate::menu_bar::toggle_menu_bar(cx));
}

gpui::actions!(lithe_workbench, [OpenCommandPalette, ToggleMenuBar]);

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
    query: Entity<InputState>,
    /// 查询框一动就重排一次 —— 行是本面板自己算的（库的 `Palette` 不做过滤）。
    _query_changed: Subscription,
}

impl CommandPalette {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr("lithe.commandPalette.placeholder"))
        });
        let query_changed = cx.subscribe_in(
            &query,
            window,
            |_, _: &Entity<InputState>, event: &InputEvent, _, cx| {
                if *event == InputEvent::Changed {
                    cx.notify();
                }
            },
        );
        Self {
            query,
            _query_changed: query_changed,
        }
    }

    fn confirm(&mut self, id: CommandId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(shell) = shell().and_then(|shell| shell.upgrade()) else {
            return;
        };
        shell.update(cx, |shell, cx| shell.run_command_id(id, window, cx));
    }
}

/// 命令表按 `search_model` 的同一套匹配裁一遍，**保持目录顺序**。
///
/// 原来是上游 `Command` 自己过滤：它排序、按 label/关键字匹配，而 `run_command(row)`
/// 却拿的是**未过滤目录**的位次 ⇒ 带查询时按下的那一行与跑起来的不是同一条。
/// 这里改成「过滤后的行 → 行上的 id」，索引与动作合一，匹配口径也与「随处搜索」一致。
fn visible_rows(query: &str, cx: &App) -> Vec<CommandAction> {
    let catalog = shell()
        .and_then(|shell| shell.upgrade())
        .map(|shell| shell.read(cx).action_catalog(cx))
        .unwrap_or_default();
    crate::search_model::action_rows(catalog, query)
}

fn row(action: CommandAction, cx: &App) -> PaletteRow {
    let theme = cx.theme();
    let (id, label, description, icon) = (action.id, action.label, action.description, action.icon);
    let mut row = PaletteRow::new(id.id(), label.clone()).label(
        v_stack()
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
                    .text_color(theme.colors.fg)
                    .child(label),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_xs()
                    .text_color(theme.colors.fg_muted)
                    .child(description),
            ),
    );
    if let Some(icon) = icon {
        row = row.icon_element(icon.build(cx));
    }
    row
}

impl Render for CommandPalette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let needs_focus = PENDING_FOCUS.with(|slot| {
            let pending = *slot.borrow();
            *slot.borrow_mut() = false;
            pending
        });
        if needs_focus {
            window.focus(&self.query.read(cx).focus().clone(), cx);
            println!("S1_COMMAND_PALETTE focus=search");
        }

        let query = self.query.read(cx).text().to_string();
        let rows = visible_rows(&query, cx);
        let ids: Vec<CommandId> = rows.iter().map(|action| action.id).collect();
        let close = |window: &mut Window, cx: &mut App| dismiss_hosted(window, cx);
        let pick = {
            let palette = cx.entity();
            move |row: usize, _: &SharedString, window: &mut Window, cx: &mut App| {
                let Some(id) = ids.get(row).copied() else {
                    return;
                };
                palette.update(cx, |palette, cx| palette.confirm(id, window, cx));
            }
        };

        Palette::new("lithe-command-palette", self.query.clone(), close)
            .rows(rows.into_iter().map(|action| row(action, cx)))
            .empty(tr("lithe.commandPalette.noCommands"))
            .on_pick(pick)
            .w_full()
            .h_auto()
            .max_h(rems(PANEL_MAX_HEIGHT / 16.))
            .card(window, cx)
    }
}
