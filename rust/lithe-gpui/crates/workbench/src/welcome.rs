use std::path::PathBuf;

use gpui::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, FontWeight, ImageSource,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, WeakEntity, Window, div, rems,
};
use gpui::{Resource, img};
use lithe_gpui_settings::{RecentProject, RecentProjects};
use lithe_gpui_shared::icons::{idea, idea_icon_image, idea_icon_mono};
use lithe_gpui_shared::{HOST_VERSION, tr, tr_args};
use lithe_ui::buttons::{Button, ButtonVariant};
use lithe_ui::forms::{Input, InputEvent, InputState};
use lithe_ui::layout::{h_stack, v_stack};
use lithe_ui::shell::{TitleBar, WindowRootExt as _};
use lithe_ui::theme::{ActiveTheme as _, Size};

use crate::project_menu::{ProjectBadge, badge_view};

pub const WELCOME_TAG: &str = "S1_WELCOME";

/// 欢迎窗口自画标题栏的高度。
///
/// 这条不是本 crate 自己的规格：窗口壳由库的 [`TitleBar`] 画，高度取它的
/// `Theme::titlebar_height()`（38pt）。本常量只是把那个值写出来给 UI 测试量，
/// 数值必须与库同步——改了库的标题栏高度就得跟着改这里。
pub const WELCOME_TITLE_BAR_HEIGHT_SPEC: f32 = 38.;

const SIDEBAR_WIDTH_SPEC: f32 = 224.;
const SEPARATOR_THICKNESS_SPEC: f32 = 1.;
const HEADER_PADDING_H_SPEC: f32 = 20.;
const HEADER_PADDING_TOP_SPEC: f32 = 26.;
const HEADER_PADDING_BOTTOM_SPEC: f32 = 38.;
const LOGO_SIZE_SPEC: f32 = 28.;
const HEADER_GAP_SPEC: f32 = 11.;
const SECTION_ICON_SIZE_SPEC: f32 = 15.;
const SECTION_ROW_HEIGHT_SPEC: f32 = 32.;
const SECTION_RADIUS_SPEC: f32 = 4.;
const SECTION_PADDING_H_SPEC: f32 = 14.;
const SETTINGS_ICON_SIZE_SPEC: f32 = 14.;
const SIDEBAR_BOTTOM_PADDING_SPEC: f32 = 14.;
const TOOLBAR_HEIGHT_SPEC: f32 = 68.;
const TOOLBAR_PADDING_H_SPEC: f32 = 14.;
const SEARCH_MAX_WIDTH_SPEC: f32 = 220.;
const TOOLBAR_SEPARATOR_PADDING_H_SPEC: f32 = 12.;
const LIST_PADDING_H_SPEC: f32 = 24.;
const LIST_PADDING_V_SPEC: f32 = 14.;
const LIST_GAP_SPEC: f32 = 2.;
const ROW_HEIGHT_SPEC: f32 = 68.;
const ROW_PADDING_H_SPEC: f32 = 12.;
const ROW_RADIUS_SPEC: f32 = 4.;
const ROW_GAP_SPEC: f32 = 12.;
const ROW_BADGE_SIZE_SPEC: f32 = 20.;
const EMPTY_ICON_SIZE_SPEC: f32 = 32.;
const EMPTY_GAP_SPEC: f32 = 10.;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WelcomeRow {
    pub name: String,
    pub path: String,
    pub key: String,
    pub missing: bool,
}

pub fn welcome_rows(entries: &[RecentProject], filter: &str) -> Vec<WelcomeRow> {
    let needle = filter.trim().to_lowercase();
    entries
        .iter()
        .filter(|entry| {
            needle.is_empty()
                || entry.name.to_lowercase().contains(&needle)
                || entry.path.to_lowercase().contains(&needle)
        })
        .map(|entry| WelcomeRow {
            name: entry.name.clone(),
            path: abbreviate_home(&entry.path),
            key: entry.path.clone(),
            missing: entry.missing,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyState {
    NoRecentProjects,
    NoMatchingProjects,
}

pub fn empty_state(entries: &[RecentProject], filter: &str) -> Option<EmptyState> {
    if !welcome_rows(entries, filter).is_empty() {
        return None;
    }
    Some(if entries.is_empty() {
        EmptyState::NoRecentProjects
    } else {
        EmptyState::NoMatchingProjects
    })
}

fn abbreviate_home(path: &str) -> String {
    let Some(home) = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
    else {
        return path.to_string();
    };
    let home = home.to_string_lossy().to_string();
    if home.is_empty() {
        return path.to_string();
    }
    match path.strip_prefix(&home) {
        Some(rest) if rest.is_empty() || rest.starts_with(['\\', '/']) => format!("~{rest}"),
        _ => path.to_string(),
    }
}

pub trait ProjectOpener: 'static {
    fn open_project(&mut self, target: PathBuf, window: &mut Window, cx: &mut App);

    fn pick_folder(&mut self, _window: &mut Window, _cx: &mut Context<WelcomeView>) {}
}

pub struct WorkspaceOpener;

impl ProjectOpener for WorkspaceOpener {
    fn open_project(&mut self, target: PathBuf, window: &mut Window, cx: &mut App) {
        let path_text = target.to_string_lossy().to_string();
        if !target.is_dir() {
            eprintln!("{WELCOME_TAG} open_project state=missing path={path_text}");
            return;
        }
        eprintln!("{WELCOME_TAG} open_project state=opening path={path_text}");
        let Some(shell) = crate::workspace::replace_root_with_workspace(&target, window, cx) else {
            eprintln!("{WELCOME_TAG} open_project state=rebuild_failed reason=no_view");
            return;
        };
        let _ = shell.update(cx, |shell, cx| {
            shell.record_opened_project(&target, Some(false), cx)
        });
    }

    fn pick_folder(&mut self, window: &mut Window, cx: &mut Context<WelcomeView>) {
        if window.has_active_dialog(cx) {
            eprintln!("{WELCOME_TAG} pick_folder state=skipped reason=active_dialog");
            return;
        }
        eprintln!("{WELCOME_TAG} pick_folder state=prompt");
        let picked = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: None,
        });
        let me = cx.entity().downgrade();
        let task = cx.spawn_in(window, async move |_this, async_cx| {
            let target = match picked.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) => {
                    eprintln!("{WELCOME_TAG} pick_folder state=cancelled");
                    None
                }
                Ok(Err(error)) => {
                    eprintln!("{WELCOME_TAG} pick_folder state=failed error={error}");
                    None
                }
                Err(_) => {
                    eprintln!("{WELCOME_TAG} pick_folder state=cancelled reason=channel-closed");
                    None
                }
            };
            let Some(target) = target else {
                return;
            };
            eprintln!(
                "{WELCOME_TAG} pick_folder state=picked path={}",
                target.display()
            );
            let _ = async_cx.update(move |window, cx| {
                let _ = me.update(cx, |this, cx| {
                    this.opener.open_project(target.clone(), window, cx);
                });
            });
        });
        task.detach();
    }
}

pub struct WelcomeView {
    projects: RecentProjects,
    path: Option<PathBuf>,
    filter: SharedString,
    search: Entity<InputState>,
    hovered: Option<String>,
    _search_subscription: Subscription,
    opener: Box<dyn ProjectOpener>,
}

impl WelcomeView {
    pub fn new(
        projects: RecentProjects,
        path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::with_opener(projects, path, Box::new(WorkspaceOpener), window, cx)
    }

    pub fn with_opener(
        projects: RecentProjects,
        path: Option<PathBuf>,
        opener: Box<dyn ProjectOpener>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx
            .new(|cx| InputState::new(window, cx).placeholder(tr("lithe.welcome.searchProjects")));
        let search_subscription = cx.subscribe_in(
            &search,
            window,
            |this: &mut Self, state: &Entity<InputState>, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Changed) {
                    this.filter = SharedString::from(state.read(cx).text().to_string());
                    cx.notify();
                }
            },
        );
        eprintln!(
            "{WELCOME_TAG} view_open count={} recent_file={}",
            projects.len(),
            path.as_deref()
                .map_or_else(|| "<none>".to_string(), |path| path.display().to_string())
        );
        Self {
            projects,
            path,
            filter: SharedString::default(),
            search,
            hovered: None,
            _search_subscription: search_subscription,
            opener,
        }
    }

    pub fn rows(&self) -> Vec<WelcomeRow> {
        welcome_rows(self.projects.entries(), self.filter.as_ref())
    }

    pub fn empty_state(&self) -> Option<EmptyState> {
        empty_state(self.projects.entries(), self.filter.as_ref())
    }

    fn activate_row(&mut self, key: String, window: &mut Window, cx: &mut Context<Self>) {
        eprintln!("{WELCOME_TAG} row_activate path={key}");
        self.opener.open_project(PathBuf::from(key), window, cx);
    }

    fn remove_row(&mut self, key: &str, cx: &mut Context<Self>) {
        if !self.projects.remove(key) {
            eprintln!("{WELCOME_TAG} row_remove state=not_found path={key}");
            return;
        }
        eprintln!(
            "{WELCOME_TAG} row_remove state=removed path={key} count={}",
            self.projects.len()
        );
        self.save();
        cx.notify();
    }

    fn save(&self) {
        let Some(path) = self.path.as_deref() else {
            eprintln!("{WELCOME_TAG} save=skipped reason=no_path");
            return;
        };
        match lithe_gpui_settings::save_recent_projects(path, &self.projects) {
            Ok(bytes) => println!(
                "{WELCOME_TAG} saved path={} bytes={bytes} count={}",
                path.display(),
                self.projects.len()
            ),
            Err(error) => eprintln!("{WELCOME_TAG} save_failed error={error}"),
        }
    }

    fn sidebar(&self, cx: &App) -> AnyElement {
        let text = cx.theme().colors.fg;
        let muted = cx.theme().colors.fg_muted;
        v_stack()
            .w(rems(SIDEBAR_WIDTH_SPEC / 16.))
            .h_full()
            .flex_shrink_0()
            // 左侧栏是比页面更"沉"的一层：库的槽位表里对应的就是 `sunken`
            // （组件库没有上游那套 `sidebar` / `sidebar_accent` 专名）。
            .bg(cx.theme().colors.sunken)
            .child(
                h_stack()
                    .gap(rems(HEADER_GAP_SPEC / 16.))
                    .items_center()
                    .px(rems(HEADER_PADDING_H_SPEC / 16.))
                    .pt(rems(HEADER_PADDING_TOP_SPEC / 16.))
                    .pb(rems(HEADER_PADDING_BOTTOM_SPEC / 16.))
                    .child(
                        img(ImageSource::Resource(Resource::Embedded(
                            "images/logo.png".into(),
                        )))
                        .w(rems(LOGO_SIZE_SPEC / 16.))
                        .h(rems(LOGO_SIZE_SPEC / 16.))
                        .flex_shrink_0(),
                    )
                    .child(
                        v_stack()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(text)
                                    .child("Lithe"),
                            )
                            .child(div().text_xs().text_color(muted).child(HOST_VERSION)),
                    ),
            )
            .child(
                h_stack()
                    .gap(rems(9. / 16.))
                    .items_center()
                    .h(rems(SECTION_ROW_HEIGHT_SPEC / 16.))
                    .px(rems(SECTION_PADDING_H_SPEC / 16.))
                    .mx(rems(SECTION_PADDING_H_SPEC / 16.))
                    .rounded(rems(SECTION_RADIUS_SPEC / 16.))
                    // 选中行的底：上游用 `sidebar_accent`，库这边安静选中的那一档是 `hover`。
                    .bg(cx.theme().colors.hover)
                    .text_color(text)
                    .child(idea_icon_image(
                        &idea::FOLDER_ICON,
                        cx,
                        rems(SECTION_ICON_SIZE_SPEC / 16.),
                    ))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(tr("lithe.welcome.projects")),
                    ),
            )
            .child(div().flex_1())
            .child(
                h_stack()
                    .px(rems(SECTION_PADDING_H_SPEC / 16.))
                    .pb(rems(SIDEBAR_BOTTOM_PADDING_SPEC / 16.))
                    .child(
                        // 库的 `Button` 不吃 `.child()`：图标走 `.icon_element()`，
                        // 内边距由按钮自己按 `Size::Small` 收，不再手写 `.px_0()`。
                        Button::new("lithe-welcome-settings")
                            .variant(ButtonVariant::Ghost)
                            .size(Size::Small)
                            .icon_element(idea_icon_image(
                                &idea::GEAR_ICON,
                                cx,
                                rems(SETTINGS_ICON_SIZE_SPEC / 16.),
                            ))
                            .on_click(|_, window, cx| {
                                lithe_gpui_settings::open_settings_dialog(window, cx);
                            }),
                    ),
            )
            .into_any_element()
    }

    fn projects_content(&self, me: &WeakEntity<Self>, window: &Window, cx: &App) -> AnyElement {
        let mut column = v_stack()
            .flex_1()
            .h_full()
            .min_w_0()
            .bg(cx.theme().colors.bg)
            .child(self.toolbar(me, cx))
            .child(
                div()
                    .px(rems(TOOLBAR_SEPARATOR_PADDING_H_SPEC / 16.))
                    .child(
                        div()
                            .h(rems(SEPARATOR_THICKNESS_SPEC / 16.))
                            .w_full()
                            .bg(cx.theme().colors.border),
                    ),
            );

        let rows = self.rows();
        column = match self.empty_state() {
            Some(state) => column.child(self.empty_state_view(state, cx)),
            None => column.child(
                v_stack()
                    // `.overflow_y_scroll()` 属 `StatefulInteractiveElement`，只长在
                    // `Stateful<Div>` 上：先 `.id()` 再滚动。
                    .id("lithe-welcome-projects")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .gap(rems(LIST_GAP_SPEC / 16.))
                    .px(rems(LIST_PADDING_H_SPEC / 16.))
                    .py(rems(LIST_PADDING_V_SPEC / 16.))
                    .children(
                        rows.iter()
                            .enumerate()
                            .map(|(index, row)| self.row(index, row, me, window, cx)),
                    ),
            ),
        };
        column.into_any_element()
    }

    fn toolbar(&self, me: &WeakEntity<Self>, cx: &App) -> AnyElement {
        let me_for_pick = me.clone();
        h_stack()
            .w_full()
            .flex_shrink_0()
            .gap(rems(10. / 16.))
            .px(rems(TOOLBAR_PADDING_H_SPEC / 16.))
            .h(rems(TOOLBAR_HEIGHT_SPEC / 16.))
            .child(
                // 库的 `Input` 是 `IntoElement`，不吃 `.max_w()`；宽度上限外包一层给。
                div().max_w(rems(SEARCH_MAX_WIDTH_SPEC / 16.)).child(
                    Input::new(&self.search)
                        .size(Size::Small)
                        .clearable()
                        .prefix(idea_icon_image(
                            &idea::MAGNIFYING_GLASS_ICON,
                            cx,
                            rems(12.5 / 16.),
                        )),
                ),
            )
            .child(div().flex_1())
            .child(
                Button::new("lithe-welcome-open-folder")
                    .variant(ButtonVariant::Outline)
                    .size(Size::Small)
                    .label(tr("lithe.welcome.open"))
                    .on_click(move |_, window, cx| {
                        let _ =
                            me_for_pick.update(cx, |this, cx| this.opener.pick_folder(window, cx));
                    }),
            )
            .into_any_element()
    }

    fn row(
        &self,
        index: usize,
        row: &WelcomeRow,
        me: &WeakEntity<Self>,
        window: &Window,
        cx: &App,
    ) -> AnyElement {
        let id = welcome_row_element_id(&row.key);
        let hovered = self.hovered.as_deref() == Some(row.key.as_str());
        let foreground = if row.missing {
            cx.theme().colors.fg_muted
        } else {
            cx.theme().colors.fg
        };
        let muted = cx.theme().colors.fg_muted;
        let hover_bg = cx.theme().colors.hover;
        let badge = ProjectBadge::for_name(&row.name);

        let me_for_click = me.clone();
        let key_for_click = row.key.clone();
        let me_for_hover = me.clone();
        let key_for_hover = row.key.clone();

        let mut element = h_stack()
            .id(id)
            // 量盒子走 gpui 原生的 debug selector（按行号，UI 测试可预期）。
            .debug_selector(move || format!("lithe-welcome-row-{index}"))
            .w_full()
            .flex_shrink_0()
            .h(rems(ROW_HEIGHT_SPEC / 16.))
            .gap(rems(ROW_GAP_SPEC / 16.))
            .items_center()
            .px(rems(ROW_PADDING_H_SPEC / 16.))
            .rounded(rems(ROW_RADIUS_SPEC / 16.))
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg))
            .on_hover(move |is_hovered: &bool, _window, cx| {
                let next = is_hovered.then(|| key_for_hover.clone());
                let _ = me_for_hover.update(cx, |this, cx| {
                    if this.hovered != next {
                        this.hovered = next;
                        cx.notify();
                    }
                });
            })
            .on_click(move |_event, window, cx| {
                let key = key_for_click.clone();
                let _ = me_for_click.update(cx, |this, cx| this.activate_row(key, window, cx));
            })
            .child(badge_view(
                &badge,
                crate::rem_px(window.rem_size(), ROW_BADGE_SIZE_SPEC),
            ))
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
                            .text_color(foreground)
                            .child(row.name.clone()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_xs()
                            .text_color(muted)
                            .child(row.path.clone()),
                    ),
            );

        if hovered {
            let me_for_remove = me.clone();
            let key_for_remove = row.key.clone();
            let name_for_remove = row.name.clone();
            element = element.child(
                Button::new(welcome_remove_element_id(&row.key))
                    .variant(ButtonVariant::Ghost)
                    .size(Size::Small)
                    .icon_element(idea_icon_mono(&idea::X_ICON, cx.theme().is_dark()))
                    .tooltip(tr_args(
                        "lithe.welcome.removeRecent",
                        &[("name", name_for_remove.as_str())],
                    ))
                    .on_click(move |_, _window, cx| {
                        cx.stop_propagation();
                        let key = key_for_remove.clone();
                        let _ = me_for_remove.update(cx, |this, cx| this.remove_row(&key, cx));
                    }),
            );
        }

        element.into_any_element()
    }

    fn empty_state_view(&self, state: EmptyState, cx: &App) -> AnyElement {
        let title = match state {
            EmptyState::NoRecentProjects => tr("lithe.welcome.noRecentProjects"),
            EmptyState::NoMatchingProjects => tr("lithe.welcome.gpui.noMatchingProjects"),
        };
        v_stack()
            .w_full()
            .flex_1()
            .items_center()
            .justify_center()
            .gap(rems(EMPTY_GAP_SPEC / 16.))
            .child(idea_icon_image(
                &idea::FOLDER_PLUS_ICON,
                cx,
                rems(EMPTY_ICON_SIZE_SPEC / 16.),
            ))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().colors.fg)
                    .child(title),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(tr("lithe.welcome.openFolderHint")),
            )
            .into_any_element()
    }
}

pub fn welcome_row_element_id(key: &str) -> ElementId {
    ElementId::Name(SharedString::from(format!("lithe-welcome-project:{key}")))
}

pub fn welcome_remove_element_id(key: &str) -> ElementId {
    ElementId::Name(SharedString::from(format!("lithe-welcome-remove:{key}")))
}

impl Render for WelcomeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let me = cx.entity().downgrade();
        let border = cx.theme().colors.border;
        v_stack()
            .w_full()
            .h_full()
            .bg(cx.theme().colors.bg)
            .child(welcome_title_bar())
            .child(
                h_stack()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .child(self.sidebar(cx))
                    .child(
                        div()
                            .w(rems(SEPARATOR_THICKNESS_SPEC / 16.))
                            .h_full()
                            .flex_shrink_0()
                            .bg(border),
                    )
                    .child(self.projects_content(&me, _window, cx)),
            )
    }
}

/// 欢迎窗口的窗口壳：gpui 的窗口在 Windows 上没有原生标题栏，少了这一条既没有窗口键
/// （min/max/close）也没有拖拽区，用户只能靠 Alt+F4。高度由库的 [`TitleBar`] 自己定。
fn welcome_title_bar() -> impl IntoElement {
    div()
        .id("lithe-welcome-title-bar")
        // UI 测试从 gpui 原生的 debug selector 量这条标题栏。
        .debug_selector(|| "lithe-welcome-title-bar".into())
        .w_full()
        .flex_shrink_0()
        .child(TitleBar::new("lithe-welcome-window-bar"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lithe_gpui_settings::RecentProjects;

    fn projects_at(paths: &[&str]) -> RecentProjects {
        let mut projects = RecentProjects::new();
        for (index, path) in paths.iter().enumerate() {
            projects.record_open(path, 10_000 - index as u64, None);
        }
        projects
    }

    #[test]
    fn empty_list_reports_no_recent_projects() {
        let projects = RecentProjects::new();
        assert_eq!(
            empty_state(projects.entries(), ""),
            Some(EmptyState::NoRecentProjects)
        );
        assert_eq!(
            empty_state(projects.entries(), "anything"),
            Some(EmptyState::NoRecentProjects),
            "列表本来就空时，即使有过滤串也还是'没有最近项目'（真源先看 filteredProjects 再判 recentProjects）"
        );
    }

    #[test]
    fn filter_without_match_reports_no_matching_projects() {
        let projects = projects_at(&[r"D:\proj\alpha", r"D:\proj\beta"]);
        assert_eq!(
            empty_state(projects.entries(), "gamma"),
            Some(EmptyState::NoMatchingProjects)
        );
        assert_eq!(empty_state(projects.entries(), "alpha"), None);
    }

    #[test]
    fn filter_is_case_insensitive_and_matches_name_or_path() {
        let projects = projects_at(&[r"D:\proj\Alpha", r"D:\other\beta"]);

        let all = welcome_rows(projects.entries(), "");
        assert_eq!(all.len(), 2, "空过滤串必须原样返回全部");
        let data_layer: Vec<&str> = projects
            .entries()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        let projected: Vec<&str> = all.iter().map(|row| row.name.as_str()).collect();
        assert_eq!(projected, data_layer, "行顺序必须与数据层逐条一致");

        let by_name = welcome_rows(projects.entries(), "ALPHA");
        assert_eq!(by_name.len(), 1, "大小写不敏感地按名字匹配");
        assert_eq!(by_name[0].key, r"D:\proj\Alpha");

        let by_path = welcome_rows(projects.entries(), r"d:\proj\alpha");
        assert_eq!(by_path.len(), 1, "路径也参与匹配");
        assert_eq!(by_path[0].key, r"D:\proj\Alpha");

        assert!(welcome_rows(projects.entries(), "gamma").is_empty());
    }

    #[test]
    fn rows_carry_the_data_layer_key_and_flags() {
        let mut projects = RecentProjects::new();
        projects.record_open(r"D:\proj\alpha", 1, None);
        projects.set_missing(r"D:\proj\alpha", true);

        let rows = welcome_rows(projects.entries(), "");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, r"D:\proj\alpha");
        assert_eq!(rows[0].name, "alpha");
        assert!(
            rows[0].missing,
            "数据层的 missing 要透到行上（真源按 fileExists 置灰）"
        );
    }

    #[test]
    fn home_abbreviation_respects_segment_boundaries() {
        let path = r"D:\proj\alpha";
        assert_eq!(abbreviate_home(path), path);
    }

    #[test]
    fn element_ids_use_the_project_path() {
        let row = welcome_row_element_id(r"D:\proj\alpha").to_string();
        let remove = welcome_remove_element_id(r"D:\proj\alpha").to_string();
        assert!(
            row.contains(r"D:\proj\alpha"),
            "行 id 必须含业务身份：{row}"
        );
        assert!(
            remove.contains(r"D:\proj\alpha"),
            "移除按钮 id 必须含业务身份：{remove}"
        );
        assert_ne!(row, remove);
    }
}
