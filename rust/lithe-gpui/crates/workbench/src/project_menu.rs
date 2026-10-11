use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    AnyElement, App, AppContext as _, Context, Entity, FontWeight, Hsla, InteractiveElement as _,
    IntoElement, Length, ParentElement as _, SharedString, Styled as _, WeakEntity, div, radians,
    rems, rgb,
};
use lithe_gpui_shared::icons::{idea, idea_icon_mono};
use lithe_ui::layout::{h_stack, v_stack};
use lithe_ui::menus::{Menu, MenuItem, TriggerMenu};
use lithe_ui::theme::{ActiveTheme as _, IconSize};

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

/// 项目徽标：两个字母 + 一层底色。
///
/// 尺寸收 [`Length`] 而不是 `Pixels`：菜单行是「绘制期才拿得到 `&App`」的地方
/// （见 [`MenuItem::element`]），那里只能说 `rems`；欢迎页那边手上有 `Window`，
/// 说像素更方便。两种写法都进得来。
pub(crate) fn badge_view(badge: &ProjectBadge, size: impl Into<Length>) -> impl IntoElement {
    let size: Length = size.into();
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

    /// 本仓自己的图标常量，深浅由画的时候再问主题（同 [`idea_icon_mono`] 的口径）。
    fn icon(self) -> &'static idea::IdeaIcon {
        match self {
            Self::NewProject => &idea::PLUS_ICON,
            Self::OpenFolder => &idea::FOLDER_ICON,
            Self::CloneRepository => &idea::GIT_BRANCH_ICON,
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

/// 面板顶上那三条动作：图标 + 一句话。
///
/// 行外壳（内边距、圆角、hover 底、整行可点）由库给，这里只画内容。
fn action_row(action: PanelAction, shell: Option<WeakEntity<ShellWorkspace>>) -> MenuItem {
    let wire = action.wire().zip(shell);
    let clickable = wire.is_some();
    let item = MenuItem::element(move |cx| {
        let colors = &cx.theme().colors;
        let color = if clickable { colors.fg } else { colors.fg_muted };
        h_stack()
            .w_full()
            .h(rems(ACTION_ROW_HEIGHT_SPEC / 16.))
            .gap_2()
            .child(
                idea_icon_mono(action.icon(), cx.theme().is_dark())
                    .size(IconSize::Md)
                    .color(color),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_sm()
                    .text_color(color)
                    .child(tr(action.label_key())),
            )
            .into_any_element()
    });

    if !clickable {
        return item.disabled(true);
    }

    let Some((wire, shell)) = wire else {
        return item.disabled(true);
    };
    item.on_click(move |window, cx| match wire.missing() {
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

/// 「已打开项目」里的一行：徽标 + 名字 + 路径，当前那个带对勾。
///
/// 一行装不下「徽标 + 两行字」，所以整行自画（[`MenuItem::element`]）。
/// 行仍然是**条目**而不是可点的动作：切换项目走标签页，这里只是状态陈列。
fn project_row(entry: &ProjectEntry) -> MenuItem {
    let badge = ProjectBadge::for_name(&entry.name);
    let name = entry.name.clone();
    let path = entry.path.clone();
    let active = entry.active;

    MenuItem::element(move |cx| {
        let colors = &cx.theme().colors;
        h_stack()
            .w_full()
            .min_h(rems(PROJECT_ROW_HEIGHT_SPEC / 16.))
            .gap_2p5()
            // 库的行壳自带 `px_2`；负外边距 + 同量内边距让高亮铺满整条面板。
            .mx_neg_2()
            .px_2()
            .py_1p5()
            .rounded(rems(ROW_RADIUS_SPEC / 16.))
            // 行画在浮层面板（`overlay`）上，用 chrome 那一档：`hover` 在浅色里与浮层同色。
            .when(active, |row| row.bg(colors.chrome_active))
            .child(badge_view(&badge, rems(ROW_BADGE_SIZE_SPEC / 16.)))
            .child(
                v_stack()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg)
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(colors.fg_muted)
                            .child(path.clone()),
                    ),
            )
            .when(active, |row| {
                row.child(
                    idea_icon_mono(&idea::CHECK_ICON, cx.theme().is_dark())
                        .size(IconSize::Md)
                        .color(colors.primary),
                )
            })
            .into_any_element()
    })
    .disabled(true)
}

fn recent_empty_row() -> MenuItem {
    MenuItem::element(|cx| {
        div()
            .w_full()
            .mx_neg_2()
            .px_2()
            .py_3()
            .text_xs()
            .text_color(cx.theme().colors.fg_muted)
            .child(tr("lithe.titleProject.noRecentProjects"))
            .into_any_element()
    })
    .disabled(true)
}

/// 「最近项目」里的一行：和 [`project_row`] 同一张脸，但点了会去开它。
fn recent_row(entry: &RecentProject, shell: Option<WeakEntity<ShellWorkspace>>) -> MenuItem {
    let badge = ProjectBadge::for_name(&entry.name);
    let name: SharedString = entry.name.clone().into();
    let path_text = entry.path.clone();
    let path = std::path::PathBuf::from(entry.path.clone());
    let item = MenuItem::element(move |cx| {
        let colors = &cx.theme().colors;
        h_stack()
            .w_full()
            .min_h(rems(PROJECT_ROW_HEIGHT_SPEC / 16.))
            .gap_2p5()
            .mx_neg_2()
            .px_2()
            .py_1p5()
            .rounded(rems(ROW_RADIUS_SPEC / 16.))
            .child(badge_view(&badge, rems(ROW_BADGE_SIZE_SPEC / 16.)))
            .child(
                v_stack()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(colors.fg)
                            .child(name.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(colors.fg_muted)
                            .child(path_text.clone()),
                    ),
            )
            .into_any_element()
    });

    match shell {
        Some(shell) => item.on_click(move |window, cx| {
            let path = path.clone();
            let _ = shell.update(cx, |shell, cx| {
                shell.request_open_project(path.clone(), None, window, cx)
            });
        }),
        None => item.disabled(true),
    }
}

fn build_panel(
    entries: &[ProjectEntry],
    recent: &[RecentProject],
    shell: Option<WeakEntity<ShellWorkspace>>,
) -> Menu {
    let mut menu = Menu::new()
        .items(
            PanelAction::ALL
                .into_iter()
                .map(|action| action_row(action, shell.clone())),
        )
        // 定宽 + 限高：行里是「徽标 + 两行字」，宽度随内容变会让面板每帧都不一样宽。
        .width(rems(PANEL_WIDTH_SPEC / 16.))
        .max_height(rems(PANEL_MAX_HEIGHT_SPEC / 16.));

    menu = menu.group(
        tr("lithe.titleProject.openProjects"),
        entries.iter().map(project_row),
    );

    let recent_items: Vec<MenuItem> = if recent.is_empty() {
        vec![recent_empty_row()]
    } else {
        recent
            .iter()
            .map(|entry| recent_row(entry, shell.clone()))
            .collect()
    };
    menu.group(tr("lithe.titleProject.recentProjects"), recent_items)
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

/// 标题栏上那块「项目下拉」在壳里的状态。
///
/// 面板自己的开合由库的 [`TriggerMenu`] 管（`Open` 就在窗口的键控状态里），
/// 这个实体只剩壳要交给它的东西：往哪开项目、当前项目的徽标，以及探针的请求编号。
pub struct ProjectMenu {
    /// 当前项目的徽标。项目名在构造时就定死了，不必每帧从名字重算一遍。
    badge: ProjectBadge,
    shell: Option<WeakEntity<ShellWorkspace>>,
    /// 探针请求打开的编号；库按「编号变了就开一次」处理。`None` = 还没人请求过。
    probe: Option<u64>,
    /// 下一个编号。
    asked_by_probe: u64,
    /// 上一帧面板开着没有。开合诊断只在变的那一刻报一次 —— 面板每帧都在重画，
    /// 跟着它刷屏没有意义，而开合本身现在归库管，抓不到「刚打开」那个点。
    open: Rc<Cell<bool>>,
    /// 结构诊断报过没有。那是**静态规格**（面板宽度、段数、行高），报一次就够。
    structure_reported: Cell<bool>,
}

impl ProjectMenu {
    pub fn new(current: &str, recent_count: usize, cx: &mut App) -> Entity<Self> {
        diagnose(false, current, recent_count);
        cx.new(|_| Self {
            badge: ProjectBadge::for_name(current),
            shell: None,
            probe: None,
            asked_by_probe: 0,
            open: Rc::new(Cell::new(false)),
            structure_reported: Cell::new(false),
        })
    }

    pub(crate) fn set_shell(&mut self, shell: WeakEntity<ShellWorkspace>) {
        self.shell = Some(shell);
    }

    pub fn open_by_probe(&mut self, cx: &mut Context<Self>) {
        eprintln!("S1_PROJECT_MENU_PROBE open=true");
        self.asked_by_probe += 1;
        self.probe = Some(self.asked_by_probe);
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

/// 标题栏上那块触发器：徽标 + 名字 + 展开箭头。
///
/// 点击与「挂在触发器下面」由库的 [`TriggerMenu`] 管，这里只画长相 ——
/// `open` 决定箭头朝哪边。
fn trigger(name: &SharedString, badge: &ProjectBadge, open: bool, cx: &App) -> AnyElement {
    let colors = &cx.theme().colors;
    // 触发器在标题栏（`title_bar`）上，悬停取 chrome 那一档。
    let (idle, hover_bg, hover_fg) = (colors.fg_muted, colors.chrome_hover, colors.fg);

    h_stack()
        .flex_shrink_0()
        .h(rems(TRIGGER_HEIGHT_SPEC / 16.))
        .pl(rems(TRIGGER_LEADING_INSET_SPEC / 16.))
        .pr(rems(TRIGGER_TRAILING_INSET_SPEC / 16.))
        .gap_1p5()
        .max_w(rems(TRIGGER_MAX_WIDTH_SPEC / 16.))
        .rounded(rems(ROW_RADIUS_SPEC / 16.))
        .text_sm()
        .text_color(idle)
        .hover(move |style| style.bg(hover_bg).text_color(hover_fg))
        .child(badge_view(badge, rems(TRIGGER_BADGE_SIZE_SPEC / 16.)))
        .child(div().min_w_0().truncate().child(name.clone()))
        .child(
            // 本仓只有朝下的插入符（`CARET_DOWN_ICON`）；朝上那张由它转半圈。
            idea_icon_mono(&idea::CARET_DOWN_ICON, cx.theme().is_dark())
                .size(IconSize::Sm)
                .color(idle)
                .rotate(match open {
                    true => radians(std::f32::consts::PI),
                    false => radians(0.),
                }),
        )
        .into_any_element()
}

pub(crate) fn render(
    menu: &Entity<ProjectMenu>,
    entries: &[ProjectEntry],
    recent: &[RecentProject],
    cx: &App,
) -> Option<AnyElement> {
    if entries.is_empty() {
        return None;
    }
    Some(project_menu(menu.read(cx), entries, recent))
}

fn project_menu(
    menu: &ProjectMenu,
    entries: &[ProjectEntry],
    recent: &[RecentProject],
) -> AnyElement {
    let name = entries[0].name.clone();
    let badge = menu.badge.clone();
    let recent_count = recent.len();
    let aria_label = tr_args("lithe.titleProject.trigger", &[("project", name.as_ref())]);
    let panel = build_panel(entries, recent, menu.shell.clone());

    // 结构诊断是静态规格，报一次就够（面板每帧都在重画）。
    if !menu.structure_reported.replace(true) {
        diagnose_structure(entries.len(), recent_count);
        diagnose_actions();
        diagnose_recent(recent);
    }

    let was_open = menu.open.clone();
    TriggerMenu::new("lithe-title-project-menu", panel, move |open, _window, cx| {
        if was_open.replace(open) != open {
            diagnose(open, name.as_ref(), recent_count);
        }
        trigger(&name, &badge, open, cx)
    })
    .aria_label(aria_label)
    .manual(menu.probe)
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
