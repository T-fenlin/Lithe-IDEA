use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{Align, ElementExt as _, Placement, Positioner, h_flex, v_flex};
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Bounds, ClickEvent, Context, DismissEvent, Entity,
    FocusHandle, Focusable as _, FontWeight, Hsla, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Pixels, SharedString, StatefulInteractiveElement as _,
    Styled as _, Subscription, WeakEntity, Window, deferred, div, radians, rems, rgb,
};
use lithe_gpui_shared::icons::{idea, idea_icon_mono};

use lithe_gpui_settings::RecentProject;
use lithe_gpui_shared::{tr, tr_args};

use crate::menu_bar::Missing;
use crate::workspace::ShellWorkspace;

const PANEL_WIDTH_SPEC: f32 = 384.;

const PANEL_MAX_HEIGHT_SPEC: f32 = 520.;

const TRIGGER_MAX_WIDTH_SPEC: f32 = 224.;

const TRIGGER_HEIGHT_SPEC: f32 = 30.;

const TRIGGER_LEADING_INSET_SPEC: f32 = 6.;

const TRIGGER_TRAILING_INSET_SPEC: f32 = 10.;

const BADGE_FONT_SIZE_SPEC: f32 = 10.;

const PANEL_OFFSET_SPEC: f32 = 4.;

const PANEL_WINDOW_MARGIN_SPEC: f32 = 8.;

const ROW_RADIUS_SPEC: f32 = lithe_gpui_shared::design::CONTROL_RADIUS;

const ACTION_ROW_HEIGHT_SPEC: f32 = 32.;

const PROJECT_ROW_HEIGHT_SPEC: f32 = 44.;

const ROW_BADGE_SIZE_SPEC: f32 = 28.;

const TRIGGER_BADGE_SIZE_SPEC: f32 = 20.;

const BADGE_TONES: [u32; 5] = [0x0284_c7, 0x0596_69, 0xea58_0c, 0x7c3a_ed, 0xe11d_48];

const BADGE_FOREGROUND: u32 = 0x00ff_ffff;

const BADGE_FALLBACK: &str = "LI";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectBadge {
    initials: SharedString,
    tone_index: usize,
}

impl ProjectBadge {
    pub fn initials(&self) -> &SharedString {
        &self.initials
    }

    pub fn tone_index(&self) -> usize {
        self.tone_index
    }

    pub fn tone(&self) -> Hsla {
        Hsla::from(rgb(BADGE_TONES[self.tone_index]))
    }

    pub fn for_name(name: &str) -> Self {
        let words: Vec<&str> = name
            .split(|character: char| !character.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .collect();
        let mut initials: String = words
            .iter()
            .take(2)
            .filter_map(|word| word.chars().next())
            .collect();
        initials = initials.to_uppercase();
        if initials.is_empty() {
            initials = BADGE_FALLBACK.to_string();
        }

        let hash = name.chars().fold(0i64, |value: i64, character: char| {
            (value * 31 + character as i64) & 0x7fff_ffff
        });

        Self {
            initials: initials.into(),
            tone_index: (hash % BADGE_TONES.len() as i64) as usize,
        }
    }
}

pub(crate) fn badge_view(badge: &ProjectBadge, size: Pixels) -> impl IntoElement {
    div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .size(size)
        .rounded(rems(ROW_RADIUS_SPEC / 16.))
        .bg(badge.tone())
        .text_color(Hsla::from(rgb(BADGE_FOREGROUND)))
        .text_size(rems(BADGE_FONT_SIZE_SPEC / 16.))
        .font_weight(FontWeight::BOLD)
        .child(badge.initials.clone())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanelAction {
    NewProject,
    OpenFolder,
    CloneRepository,
}

impl PanelAction {
    const ALL: [Self; 3] = [Self::NewProject, Self::OpenFolder, Self::CloneRepository];

    fn icon(self, is_dark: bool) -> gpui_kit::component::Icon {
        match self {
            Self::NewProject => idea_icon_mono(&idea::PLUS_ICON, is_dark),
            Self::OpenFolder => idea_icon_mono(&idea::FOLDER_ICON, is_dark),
            Self::CloneRepository => idea_icon_mono(&idea::GIT_BRANCH_ICON, is_dark),
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Self::NewProject => "lithe.titleProject.newProject",
            Self::OpenFolder => "lithe.titleProject.open",
            Self::CloneRepository => "lithe.titleProject.cloneRepository",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::NewProject => "newProject",
            Self::OpenFolder => "open",
            Self::CloneRepository => "cloneRepository",
        }
    }

    fn precondition(self) -> Option<&'static str> {
        match self {
            Self::NewProject => Some("no_scaffolding"),
            Self::OpenFolder => None,
            Self::CloneRepository => Some("no_git_clone_call_site"),
        }
    }

    fn wire(self) -> Option<PanelWire> {
        match self {
            Self::NewProject => Some(PanelWire::NewProjectNotWired),
            Self::OpenFolder => Some(PanelWire::OpenFolder),
            Self::CloneRepository => Some(PanelWire::CloneNotWired),
        }
    }

    fn enabled(self) -> bool {
        self.wire().is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanelWire {
    OpenFolder,
    CloneNotWired,
    NewProjectNotWired,
}

impl PanelWire {
    fn missing(self) -> Option<Missing> {
        match self {
            Self::OpenFolder => None,
            Self::CloneNotWired => Some(crate::menu_bar::MISSING_CLONE_UI),
            Self::NewProjectNotWired => Some(crate::menu_bar::MISSING_NEW_PROJECT_SCAFFOLDING),
        }
    }
}

fn action_row(action: PanelAction, shell: Option<WeakEntity<ShellWorkspace>>) -> PopupMenuItem {
    let wire = action.wire().zip(shell);
    let clickable = wire.is_some();
    let item = PopupMenuItem::element(move |_window, cx| {
        let color = if clickable {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };
        h_flex()
            .w_full()
            .h(rems(ACTION_ROW_HEIGHT_SPEC / 16.))
            .gap_2()
            .mx_neg_2()
            .px_2()
            .rounded(rems(ROW_RADIUS_SPEC / 16.))
            .child(action.icon(cx.theme().is_dark()).size_4().text_color(color))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .text_color(color)
                    .child(tr(action.label_key())),
            )
    });

    if !clickable {
        return item.disabled(true);
    }

    let Some((wire, shell)) = wire else {
        return item.disabled(true);
    };
    item.on_click(move |_event, window, cx| match wire.missing() {
        Some(missing) => {
            let _ = shell.update(cx, |shell, cx| {
                shell.report_panel_action_not_wired(action.id(), missing, cx)
            });
        }
        None => {
            let _ = shell.update(cx, |shell, cx| shell.open_project_picker(window, cx));
        }
    })
}

#[derive(Clone, Debug)]
pub(crate) struct ProjectEntry {
    pub name: SharedString,
    pub path: SharedString,
    pub active: bool,
}

fn project_row(entry: &ProjectEntry) -> PopupMenuItem {
    let badge = ProjectBadge::for_name(&entry.name);
    let name = entry.name.clone();
    let path = entry.path.clone();
    let active = entry.active;

    PopupMenuItem::element(move |window, cx| {
        let theme = cx.theme();
        let rem = window.rem_size();
        let accent = theme.accent;
        let foreground = theme.foreground;
        let muted = theme.muted_foreground;
        let primary = theme.primary;

        h_flex()
            .w_full()
            .min_h(rems(PROJECT_ROW_HEIGHT_SPEC / 16.))
            .gap_2p5()
            .mx_neg_2()
            .px_2()
            .py_1p5()
            .rounded(rems(ROW_RADIUS_SPEC / 16.))
            .when(active, |this| this.bg(accent))
            .child(badge_view(&badge, crate::rem_px(rem, ROW_BADGE_SIZE_SPEC)))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(foreground)
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(muted)
                            .child(path.clone()),
                    ),
            )
            .when(active, |this| {
                this.child(Icon::new(IconName::Check).size_4().text_color(primary))
            })
    })
    .disabled(true)
}

fn group_label(key: &'static str) -> PopupMenuItem {
    PopupMenuItem::element(move |_window, cx| {
        div()
            .w_full()
            .mx_neg_2()
            .px_2()
            .pt_1p5()
            .pb_1()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(tr(key))
    })
    .disabled(true)
}

fn recent_empty_row() -> PopupMenuItem {
    PopupMenuItem::element(move |_window, cx| {
        div()
            .w_full()
            .mx_neg_2()
            .px_2()
            .py_3()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(tr("lithe.titleProject.noRecentProjects"))
    })
    .disabled(true)
}

fn recent_row(entry: &RecentProject, shell: Option<WeakEntity<ShellWorkspace>>) -> PopupMenuItem {
    let badge = ProjectBadge::for_name(&entry.name);
    let name: SharedString = entry.name.clone().into();
    let path_text = entry.path.clone();
    let path = std::path::PathBuf::from(entry.path.clone());
    let item = PopupMenuItem::element(move |window, cx| {
        let theme = cx.theme();
        let foreground = theme.foreground;
        let muted = theme.muted_foreground;
        let rem = window.rem_size();

        h_flex()
            .w_full()
            .min_h(rems(PROJECT_ROW_HEIGHT_SPEC / 16.))
            .gap_2p5()
            .mx_neg_2()
            .px_2()
            .py_1p5()
            .rounded(rems(ROW_RADIUS_SPEC / 16.))
            .child(badge_view(&badge, crate::rem_px(rem, ROW_BADGE_SIZE_SPEC)))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(foreground)
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(muted)
                            .child(path_text.clone()),
                    ),
            )
    });

    match shell {
        Some(shell) => item.on_click(move |_event, window, cx| {
            let path = path.clone();
            let _ = shell.update(cx, |shell, cx| {
                shell.request_open_project(path.clone(), None, window, cx)
            });
        }),
        None => item.disabled(true),
    }
}

fn build_popup(
    entries: &[ProjectEntry],
    recent: &[RecentProject],
    shell: Option<WeakEntity<ShellWorkspace>>,
    action_context: FocusHandle,
    window: &mut Window,
    cx: &mut App,
) -> Entity<PopupMenu> {
    let entries = entries.to_vec();
    let recent = recent.to_vec();
    PopupMenu::build(window, cx, move |menu, window, _cx| {
        let rem = window.rem_size();
        let mut menu = menu
            .action_context(action_context)
            .min_w(crate::rem_px(rem, PANEL_WIDTH_SPEC))
            .max_w(crate::rem_px(rem, PANEL_WIDTH_SPEC))
            .max_h(crate::rem_px(rem, PANEL_MAX_HEIGHT_SPEC))
            .scrollable(true);

        for action in PanelAction::ALL {
            menu = menu.item(action_row(action, shell.clone()));
        }

        menu = menu
            .separator()
            .item(group_label("lithe.titleProject.openProjects"));
        for entry in &entries {
            menu = menu.item(project_row(entry));
        }

        menu = menu
            .separator()
            .item(group_label("lithe.titleProject.recentProjects"));
        if recent.is_empty() {
            menu = menu.item(recent_empty_row());
        } else {
            for entry in &recent {
                menu = menu.item(recent_row(entry, shell.clone()));
            }
        }
        menu
    })
}

fn popup_for(
    popup: Entity<PopupMenu>,
    trigger_bounds: Bounds<Pixels>,
    rem: Pixels,
) -> impl IntoElement {
    deferred(
        Positioner::side(trigger_bounds)
            .placement(Placement::Bottom)
            .align(Align::Start)
            .offset(crate::rem_px(rem, PANEL_OFFSET_SPEC))
            .margin(crate::rem_px(rem, PANEL_WINDOW_MARGIN_SPEC))
            .occlude()
            .child(popup),
    )
    .with_priority(1)
}

fn diagnose(opened: bool, current: &str, recent: usize) {
    eprintln!("S1_PROJECT_MENU opened={opened} recent={recent} current={current}");
}

fn diagnose_structure(projects: usize, recent: usize) {
    eprintln!(
        "S1_PROJECT_MENU panel_w={} max_h={} sections=3 actions={} actions_disabled={} \
         open_projects={} recent={} action_h={} row_h={} badge={}",
        PANEL_WIDTH_SPEC as i32,
        PANEL_MAX_HEIGHT_SPEC as i32,
        PanelAction::ALL.len(),
        PanelAction::ALL
            .iter()
            .filter(|action| !action.enabled())
            .count(),
        projects,
        recent,
        ACTION_ROW_HEIGHT_SPEC as i32,
        PROJECT_ROW_HEIGHT_SPEC as i32,
        ROW_BADGE_SIZE_SPEC as i32,
    );
}

fn diagnose_actions() {
    for action in PanelAction::ALL {
        let state = match action.wire() {
            Some(PanelWire::OpenFolder) => "wired",
            Some(PanelWire::CloneNotWired | PanelWire::NewProjectNotWired) => "not_wired",
            None => "disabled",
        };
        eprintln!(
            "S1_PROJECT_MENU action={} state={} precondition={}",
            action.id(),
            state,
            action.precondition().unwrap_or("none")
        );
    }
}

fn diagnose_recent(recent: &[RecentProject]) {
    for (index, entry) in recent.iter().enumerate() {
        eprintln!(
            "S1_PROJECT_MENU recent_project index={index} path={} missing={} pinned={} lastOpenedAt={}",
            entry.path, entry.missing, entry.pinned, entry.last_opened_at
        );
    }
}

pub struct ProjectMenu {
    open: bool,
    popup: Option<Entity<PopupMenu>>,
    trigger_bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
    _dismiss_subscription: Option<Subscription>,
    action_context: FocusHandle,
    shell: Option<WeakEntity<ShellWorkspace>>,
}

impl ProjectMenu {
    pub fn new(
        current: &str,
        recent_count: usize,
        action_context: FocusHandle,
        cx: &mut App,
    ) -> Entity<Self> {
        diagnose(false, current, recent_count);
        cx.new(|_| Self {
            open: false,
            popup: None,
            trigger_bounds: Rc::new(Cell::new(None)),
            _dismiss_subscription: None,
            action_context,
            shell: None,
        })
    }

    pub(crate) fn set_shell(&mut self, shell: WeakEntity<ShellWorkspace>) {
        self.shell = Some(shell);
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.open = false;
            self.popup = None;
            self._dismiss_subscription = None;
            window.focus(&self.action_context, cx);
        } else {
            self.open = true;
        }
        cx.notify();
    }

    fn handle_dismiss(
        &mut self,
        _popup: &Entity<PopupMenu>,
        _event: &DismissEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open = false;
        self.popup = None;
        self._dismiss_subscription = None;
        cx.notify();
    }

    pub fn open_by_probe(&mut self, cx: &mut Context<Self>) {
        eprintln!("S1_PROJECT_MENU_PROBE open=true");
        self.open = true;
        cx.notify();
    }
}

thread_local! {
    static PROJECT_MENU: RefCell<Option<WeakEntity<ProjectMenu>>> = const { RefCell::new(None) };
}

pub(crate) fn set_project_menu(menu: WeakEntity<ProjectMenu>) {
    PROJECT_MENU.with(|slot| *slot.borrow_mut() = Some(menu));
}

pub fn handle() -> WeakEntity<ProjectMenu> {
    PROJECT_MENU.with(|slot| slot.borrow().clone()).expect(
        "项目下拉句柄没有登记：`ShellWorkspace::new` 必须在 `render` 之前调 `set_project_menu`",
    )
}

fn trigger(
    entry: &ProjectEntry,
    open: bool,
    handle: WeakEntity<ProjectMenu>,
    rem: Pixels,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let badge = ProjectBadge::for_name(&entry.name);
    let name = entry.name.clone();
    let idle = theme.muted_foreground;
    let hover_bg = theme.accent;
    let hover_fg = theme.foreground;

    div()
        .id("lithe-title-project-trigger")
        .flex()
        .flex_shrink_0()
        .items_center()
        .h(rems(TRIGGER_HEIGHT_SPEC / 16.))
        .pl(rems(TRIGGER_LEADING_INSET_SPEC / 16.))
        .pr(rems(TRIGGER_TRAILING_INSET_SPEC / 16.))
        .gap_1p5()
        .max_w(rems(TRIGGER_MAX_WIDTH_SPEC / 16.))
        .rounded(rems(ROW_RADIUS_SPEC / 16.))
        .text_sm()
        .text_color(idle)
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        .on_mouse_down(MouseButton::Left, |_event, window, cx| {
            window.prevent_default();
            cx.stop_propagation();
        })
        .on_click(
            move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
                let _ = handle.update(cx, |menu, cx| menu.toggle(window, cx));
            },
        )
        .child(badge_view(
            &badge,
            crate::rem_px(rem, TRIGGER_BADGE_SIZE_SPEC),
        ))
        .child(div().min_w_0().truncate().child(name))
        .child(
            Icon::new(match open {
                true => IconName::ChevronUp,
                false => IconName::ChevronDown,
            })
            .small()
            .text_color(idle)
            .rotate(match open {
                true => radians(std::f32::consts::PI),
                false => radians(0.),
            }),
        )
        .aria_label(tr_args(
            "lithe.titleProject.trigger",
            &[("project", entry.name.as_ref())],
        ))
}

pub(crate) fn render(
    menu: &Entity<ProjectMenu>,
    entries: &[ProjectEntry],
    recent: &[RecentProject],
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    if entries.is_empty() {
        return None;
    }
    menu.downgrade()
        .update(cx, |menu, cx| {
            project_menu(menu, entries, recent, window, cx)
        })
        .ok()
}

fn project_menu(
    menu: &mut ProjectMenu,
    entries: &[ProjectEntry],
    recent: &[RecentProject],
    window: &mut Window,
    cx: &mut Context<ProjectMenu>,
) -> AnyElement {
    let open = menu.open;

    if open && menu.popup.is_none() {
        let popup = build_popup(
            entries,
            recent,
            menu.shell.clone(),
            menu.action_context.clone(),
            window,
            cx,
        );
        menu._dismiss_subscription =
            Some(cx.subscribe_in(&popup, window, ProjectMenu::handle_dismiss));
        let focus = popup.read(cx).focus_handle(cx);
        if !focus.contains_focused(window, cx) {
            focus.focus(window, cx);
        }
        menu.popup = Some(popup);
        diagnose(true, entries[0].name.as_ref(), recent.len());
        diagnose_structure(entries.len(), recent.len());
        diagnose_actions();
        diagnose_recent(recent);
    }

    let rem = window.rem_size();
    let trigger = trigger(&entries[0], open, handle(), rem, cx).into_any_element();
    let panel = match (menu.popup.clone(), menu.trigger_bounds.get()) {
        (Some(popup), Some(trigger_bounds)) => Some(popup_for(popup, trigger_bounds, rem)),
        _ => None,
    };

    let bounds_cell = menu.trigger_bounds.clone();
    div()
        .id("lithe-title-project-menu")
        .relative()
        .flex_shrink_0()
        .on_prepaint(move |bounds, window, _cx| {
            let first = bounds_cell.get().is_none();
            bounds_cell.set(Some(bounds));
            if first {
                window.request_animation_frame();
            }
        })
        .child(trigger)
        .children(panel)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{
        BADGE_FALLBACK, BADGE_TONES, PANEL_MAX_HEIGHT_SPEC, PANEL_WIDTH_SPEC, PanelAction,
        PanelWire, ProjectBadge,
    };

    #[test]
    fn badge_initials_match_the_windows_algorithm() {
        assert_eq!(ProjectBadge::for_name("jzwg").initials().as_ref(), "J");
        assert_eq!(ProjectBadge::for_name("qzjk").initials().as_ref(), "Q");
        assert_eq!(
            ProjectBadge::for_name("Lithe-IDEA").initials().as_ref(),
            "LI"
        );
        assert_eq!(
            ProjectBadge::for_name("Lithe-IDEA-issue-35-ci")
                .initials()
                .as_ref(),
            "LI"
        );
        assert_eq!(ProjectBadge::for_name("文档项目").initials().as_ref(), "文");
        assert_eq!(
            ProjectBadge::for_name("").initials().as_ref(),
            BADGE_FALLBACK
        );
        assert_eq!(
            ProjectBadge::for_name("---").initials().as_ref(),
            BADGE_FALLBACK
        );
    }

    #[test]
    fn badge_tone_index_is_the_hashed_palette_slot() {
        assert_eq!(ProjectBadge::for_name("jzwg").tone_index(), 0);
        assert_eq!(ProjectBadge::for_name("qzjk").tone_index(), 3);
        assert_eq!(ProjectBadge::for_name("Lithe-IDEA").tone_index(), 1);
        assert_eq!(
            ProjectBadge::for_name("Lithe-IDEA-issue-35-ci"),
            ProjectBadge::for_name("Lithe-IDEA-issue-35-ci")
        );
    }

    #[test]
    fn badge_palette_is_the_tailwind_palette() {
        assert_eq!(
            BADGE_TONES,
            [0x0284_c7, 0x0596_69, 0xea58_0c, 0x7c3a_ed, 0xe11d_48],
            "sky-600 / emerald-600 / orange-600 / violet-600 / rose-600"
        );
    }

    #[test]
    fn every_tone_slot_is_reachable() {
        let mut seen = [false; BADGE_TONES.len()];
        for index in 0..2_000 {
            seen[ProjectBadge::for_name(&format!("project-{index}")).tone_index()] = true;
        }
        assert!(seen.iter().all(|hit| *hit), "有颜色永远取不到：{seen:?}");
    }

    #[test]
    fn action_rows_match_the_source_order() {
        assert_eq!(PanelAction::ALL.len(), 3);
        assert_eq!(
            PanelAction::ALL.iter().map(|a| a.id()).collect::<Vec<_>>(),
            vec!["newProject", "open", "cloneRepository"]
        );
        let keys: Vec<&str> = PanelAction::ALL.iter().map(|a| a.label_key()).collect();
        assert_eq!(
            keys,
            vec![
                "lithe.titleProject.newProject",
                "lithe.titleProject.open",
                "lithe.titleProject.cloneRepository"
            ]
        );
        let mut preconditions: Vec<&str> = PanelAction::ALL
            .iter()
            .map(|action| action.precondition().unwrap_or("none"))
            .collect();
        preconditions.sort_unstable();
        preconditions.dedup();
        assert_eq!(
            preconditions,
            vec!["no_git_clone_call_site", "no_scaffolding", "none"],
            "三条动作的前置条件必须各说各的（真接线的那条是 none）"
        );
    }

    #[test]
    fn every_action_row_is_clickable_and_says_what_is_missing() {
        let clickable: Vec<PanelAction> = PanelAction::ALL
            .into_iter()
            .filter(|action| action.enabled())
            .collect();
        assert_eq!(
            clickable,
            vec![
                PanelAction::NewProject,
                PanelAction::OpenFolder,
                PanelAction::CloneRepository
            ],
            "三条都可点：「打开…」真接线，新建 / 克隆是占位项（点了给提示）"
        );
        assert_eq!(
            PanelAction::OpenFolder.precondition(),
            None,
            "真接线的那条不该再报前置条件"
        );
        assert_eq!(PanelAction::OpenFolder.wire(), Some(PanelWire::OpenFolder));
        assert_eq!(
            PanelAction::CloneRepository.wire(),
            Some(PanelWire::CloneNotWired)
        );
        assert_eq!(
            PanelAction::NewProject.wire(),
            Some(PanelWire::NewProjectNotWired),
            "「新建项目…」缺脚手架生成 —— 那句话说得出来，所以与克隆仓库一致做成占位项"
        );
        for action in [PanelAction::NewProject, PanelAction::CloneRepository] {
            assert!(
                action.precondition().is_some(),
                "{action:?} 没真的做出来就必须说清缺什么"
            );
        }
        assert_eq!(
            PanelAction::NewProject.precondition(),
            Some("no_scaffolding"),
            "占位项保留它**自己**那条前置条件，不复用克隆仓库的"
        );
        assert_eq!(
            PanelAction::CloneRepository.precondition(),
            Some("no_git_clone_call_site"),
            "占位项也要留前置条件：那句话说的是'缺 URL / 凭据 / 进度 UI'，不是'什么都有'"
        );
        assert_eq!(
            PanelWire::NewProjectNotWired.missing(),
            Some(crate::menu_bar::MISSING_NEW_PROJECT_SCAFFOLDING)
        );
        assert_eq!(
            PanelWire::CloneNotWired.missing(),
            Some(crate::menu_bar::MISSING_CLONE_UI)
        );
        assert_ne!(
            PanelWire::NewProjectNotWired.missing(),
            PanelWire::CloneNotWired.missing(),
            "两条占位项缺的不是同一件事，不许共用同一句能力话"
        );
        assert_eq!(
            PanelWire::OpenFolder.missing(),
            None,
            "真接线的那条不该带能力提示"
        );
    }

    #[test]
    fn recent_rows_render_without_a_shell_handle() {
        use lithe_gpui_settings::RecentProject;

        let entry = RecentProject {
            name: "alpha".to_string(),
            path: r"D:\proj\alpha".to_string(),
            last_opened_at: 1_700_000_000_000,
            pinned: false,
            missing: true,
            open_in_new_window: None,
        };
        let _item = super::recent_row(&entry, None);
    }

    #[test]
    fn panel_width_and_max_height_match_the_source() {
        assert_eq!(PANEL_WIDTH_SPEC, 384.);
        assert_eq!(PANEL_MAX_HEIGHT_SPEC, 520.);
    }
}
