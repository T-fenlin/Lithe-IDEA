use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _};
use gpui::prelude::FluentBuilder as _;
use gpui::{AnyElement, App, AppContext as _, ClickEvent, ClipboardItem, Context, Entity, FocusHandle, Focusable as _, InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, WeakEntity, Window, div, rems};
use gpui_kit::{Role};

use lithe_gpui_shared::icons::idea;
use lithe_gpui_shared::tr;
use lithe_ui::buttons::{Button as LitheButton, ButtonVariant};
use lithe_ui::lists::{
    PROJECT_ACTIONS, ProjectAction, ProjectAsset, ProjectLabels, ProjectTree, row_menu_entries,
};
use lithe_ui::menus::{DropdownMenu, Menu, MenuItem, OverflowMenu};
use lithe_ui::primitives::IconName;
use lithe_ui::theme::Size;

use crate::model::{Snapshot, icon_for, load_git_status, load_snapshot};

const SEARCH_FILES_KEY: &str = "lithe.fileExplorer.searchFiles";
const TREE_ARIA_LABEL_KEY: &str = "lithe.fileExplorer.ariaLabel";
const LOADING_FILES_KEY: &str = "lithe.quickOpen.loadingFiles";
const RETRY_KEY: &str = "lithe.ui.retry";
const NO_FOLDER_OPEN_KEY: &str = "lithe.fileExplorer.noFolderOpen";
const OPEN_FOLDER_KEY: &str = "lithe.titleProject.openFolder";
const NO_MATCHING_FILES_KEY: &str = "lithe.fileExplorer.noMatchingFiles";
const EMPTY_FOLDER_KEY: &str = "lithe.fileExplorer.folderIsEmpty";
const COLLAPSE_ALL_KEY: &str = "lithe.fileExplorer.collapseAll";
const HIDE_PANEL_KEY: &str = "lithe.fileExplorer.hidePanel";
const MORE_KEY: &str = "lithe.ui.more";
const PROJECT_VIEW_KEY: &str = "lithe.workbench.project";

const OPEN_KEY: &str = "lithe.files.open";
const REVEAL_KEY: &str = "lithe.files.reveal";
const COPY_PATH_KEY: &str = "lithe.files.copyPath";
const COPY_RELATIVE_PATH_KEY: &str = "lithe.files.copyRelativePath";
const REFRESH_KEY: &str = "lithe.files.refresh";

const TREE_ID: &str = "explorer-tree";

/// The panel header's height, as the macOS sidebar draws it (`ProjectSidebarView.swift:289`).
const HEADER_HEIGHT_SPEC: f32 = 39.;

pub type RevealInFileManagerHost = Arc<dyn Fn(&Path, &mut App) + 'static>;

/// Hides the tool window this tree lives in; the shell owns what that means.
pub type HidePanelHost = Arc<dyn Fn(&mut Window, &mut App) + 'static>;

enum LoadState {
    Loading,
    Ready,
    Failed(String),
}

impl LoadState {
    fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }
}

/// The left sidebar's project tree. The rows themselves — what is open, what is picked, the row
/// menu, the git letters — come from the component library; this view keeps the Core reads, the
/// ports (clipboard, reveal), the panel's own empty and loading states, and its words.
pub struct Explorer {
    root: PathBuf,
    snapshot: Option<Snapshot>,
    state: LoadState,
    tree: Entity<ProjectTree>,
    file_clipboard: Option<Box<dyn crate::FileClipboardPort>>,
    reveal_host: Option<RevealInFileManagerHost>,
    hide_host: Option<HidePanelHost>,
    on_open: Rc<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>,
    search: Entity<InputState>,
    #[allow(dead_code)]
    search_open: bool,
    query: SharedString,
    _search_events: Subscription,
}

impl Explorer {
    pub fn new(
        root: PathBuf,
        on_open: Box<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx));
        search.update(cx, |state, cx| {
            state.set_placeholder(tr(SEARCH_FILES_KEY), window, cx);
        });
        let search_events = cx.subscribe(
            &search,
            |this, _input, event: &InputEvent, cx: &mut Context<Self>| {
                if matches!(event, InputEvent::Change) {
                    let query = this.search.read(&**cx).value();
                    this.set_query(query, cx);
                }
            },
        );

        let explorer: WeakEntity<Self> = cx.entity().downgrade();
        let tree = cx.new(move |cx| {
            ProjectTree::new(TREE_ID, cx)
                .labels(row_labels())
                .caret(
                    ProjectAsset {
                        light: idea::CARET_RIGHT_ICON.light,
                        dark: idea::CARET_RIGHT_ICON.dark,
                    },
                    ProjectAsset {
                        light: idea::CARET_DOWN_ICON.light,
                        dark: idea::CARET_DOWN_ICON.dark,
                    },
                )
                .icon_for(icon_for)
                .on_action(move |action, path, window, cx| {
                    let _ = explorer.update(cx, |this, cx| {
                        this.run_row_action(action, path, window, cx);
                    });
                })
        });

        let mut explorer = Self {
            root,
            snapshot: None,
            state: LoadState::Loading,
            tree,
            file_clipboard: None,
            reveal_host: None,
            hide_host: None,
            on_open: Rc::from(on_open),
            search,
            search_open: false,
            query: SharedString::default(),
            _search_events: search_events,
        };
        explorer.load(cx);
        explorer
    }

    pub fn set_reveal_in_file_manager_host(&mut self, host: RevealInFileManagerHost) {
        self.reveal_host = Some(host);
    }

    pub fn set_hide_host(&mut self, host: HidePanelHost) {
        self.hide_host = Some(host);
    }

    /// How many rows the tree is showing; the whole window shares one tree, so this is also the
    /// number the `S1_EXPLORER` diagnostic reports.
    pub fn tree_rows(&self, cx: &App) -> usize {
        self.tree.read(cx).rows()
    }

    /// Shuts every folder but the workspace row: the panel header's「全部折叠」.
    pub fn collapse_all(&mut self, cx: &mut Context<Self>) {
        self.tree.update(cx, |tree, cx| tree.collapse_all(cx));
    }

    pub fn refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.load(cx);
    }

    /// Opens every folder above `path`, picks it, and scrolls it into view.
    pub fn reveal(&mut self, path: &Path, cx: &mut Context<Self>) {
        let Some(relative) = relative_to_root(&self.root, path) else {
            eprintln!(
                "S1_EXPLORER reveal=outside-root root={} path={}",
                self.root.display(),
                path.display()
            );
            return;
        };

        let found = self.tree.update(cx, |tree, cx| tree.reveal(&relative, cx));
        eprintln!("S1_EXPLORER reveal path={relative} found={found}");
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        self.state = LoadState::Loading;
        cx.notify();

        if self.root.as_os_str().is_empty() {
            self.snapshot = None;
            self.tree.update(cx, |tree, cx| {
                tree.set_root(SharedString::default(), Vec::new(), cx);
                tree.set_status(Vec::new(), cx);
                tree.set_active(None, cx);
            });
            self.state = LoadState::Ready;
            return;
        }

        let root = self.root.clone();
        cx.spawn(async move |this, cx| {
            let snapshot_root = root.clone();
            let (snapshot, status) = cx
                .background_spawn(async move {
                    (
                        load_snapshot(&snapshot_root),
                        load_git_status(&root),
                    )
                })
                .await;

            let _ = this.update(cx, |this, cx| {
                match snapshot {
                    Ok(snapshot) => {
                        let nodes = snapshot.root.count();
                        let missing = snapshot.files_missing_from_root();
                        let label = SharedString::from(root_label(&this.root));
                        let project = snapshot.project_nodes();
                        this.tree
                            .update(cx, |tree, cx| tree.set_root(label, project, cx));
                        this.snapshot = Some(snapshot);
                        this.state = LoadState::Ready;
                        let tree = this.tree.read(cx);
                        eprintln!(
                            "S1_EXPLORER root={} nodes={} rows={} limit={} truncated={} files_not_in_root={missing}",
                            this.root.display(),
                            nodes,
                            tree.rows(),
                            tree.limit(),
                            tree.truncated(),
                        );
                    }
                    Err(error) => {
                        eprintln!("S1_EXPLORER root={} error={error}", this.root.display());
                        this.snapshot = None;
                        this.tree.update(cx, |tree, cx| {
                            tree.set_root(SharedString::default(), Vec::new(), cx);
                        });
                        this.state = LoadState::Failed(error);
                    }
                }
                match status {
                    Ok(status) => {
                        this.tree
                            .update(cx, |tree, cx| tree.set_status(status, cx));
                        let (files, directories) = this.tree.read(cx).status_counts();
                        eprintln!(
                            "S1_EXPLORER_GIT root={} files={files} directories={directories} empty={}",
                            this.root.display(),
                            files == 0 && directories == 0
                        );
                    }
                    Err(error) => {
                        this.tree
                            .update(cx, |tree, cx| tree.set_status(Vec::new(), cx));
                        eprintln!("S1_EXPLORER_GIT root={} error={error}", this.root.display());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn select_all_in_tree(
        &mut self,
        _: &crate::SelectAllInTree,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tree
            .update(cx, |tree, cx| tree.select_all_in_siblings(cx));
    }

    pub fn selected_paths(&self, cx: &App) -> Vec<String> {
        self.tree
            .read(cx)
            .selected_paths()
            .iter()
            .map(|path| path.to_string())
            .collect()
    }

    pub fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.tree.read(cx).focus_handle(cx)
    }

    pub fn set_file_clipboard(&mut self, port: Box<dyn crate::FileClipboardPort>) {
        self.file_clipboard = Some(port);
    }

    /// The picked rows, in display order, to the clipboard port.
    fn copy_selection(
        &mut self,
        _: &crate::CopySelection,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(port) = self.file_clipboard.as_ref() else {
            return;
        };
        let payload = self.tree.read(cx).selected_in_display_order();
        if payload.is_empty() {
            return;
        }
        let absolute: Vec<PathBuf> = payload
            .iter()
            .map(|relative| join_relative(&self.root, relative.as_ref()))
            .collect();
        port.write_paths(&absolute);
    }

    pub fn follow_active_document(&mut self, path: Option<&str>, cx: &mut Context<Self>) {
        self.tree
            .update(cx, |tree, cx| tree.follow_active_document(path, cx));
    }

    pub fn follow_active_document_path(&mut self, path: Option<&Path>, cx: &mut Context<Self>) {
        let relative = path.and_then(|path| {
            path.strip_prefix(&self.root)
                .ok()
                .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        });
        self.follow_active_document(relative.as_deref(), cx);
    }

    fn has_entries(&self) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|snapshot| !snapshot.root.children.is_empty())
    }

    /// The tree keeps only the rows whose path holds this text.
    fn set_query(&mut self, query: SharedString, cx: &mut Context<Self>) {
        if self.query == query {
            return;
        }
        self.query = query;
        let query = self.query.to_string();
        self.tree.update(cx, |tree, cx| tree.set_query(&query, cx));
    }

    #[allow(dead_code)]
    fn toggle_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = !self.search_open;
        if self.search_open {
            let search = self.search.clone();
            search.update(cx, |state, cx| state.focus(window, cx));
        } else {
            self.clear_search(window, cx);
        }
        cx.notify();
    }

    #[allow(dead_code)]
    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let search = self.search.clone();
        search.update(cx, |state, cx| {
            state.set_value(SharedString::default(), window, cx);
        });
        self.set_query(SharedString::default(), cx);
    }

    /// One row-menu entry, on the row it was opened for.
    fn run_row_action(
        &mut self,
        action: ProjectAction,
        relative: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            ProjectAction::Open => {
                let path = join_relative(&self.root, relative);
                (self.on_open.clone())(path, window, cx);
            }
            ProjectAction::RevealInFileManager => {
                let path = join_relative(&self.root, relative);
                match self.reveal_host.clone() {
                    Some(host) => {
                        host(&path, cx);
                        eprintln!("S1_EXPLORER reveal_in_file_manager path={}", path.display());
                    }
                    None => eprintln!(
                        "S1_EXPLORER reveal_in_file_manager result=no-host path={}",
                        path.display()
                    ),
                }
            }
            ProjectAction::CopyPath => {
                let path = join_relative(&self.root, relative);
                let text = path.to_string_lossy().to_string();
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                eprintln!("S1_EXPLORER copyPath value={text}");
            }
            ProjectAction::CopyRelativePath => {
                let text = if relative.is_empty() {
                    ".".to_string()
                } else {
                    relative.to_string()
                };
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                eprintln!("S1_EXPLORER copyRelativePath value={text}");
            }
            ProjectAction::Refresh => self.load(cx),
        }
    }

    /// The panel's title row: the view selector on the left, and the tree's own actions on the
    /// right — refresh, collapse everything, the focused row's menu, and hide.
    fn render_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let explorer = cx.entity().downgrade();
        let (refresh, collapse, hide) = (explorer.clone(), explorer.clone(), explorer);
        let loading = self.state.is_loading();
        let options = self.options_menu(cx);
        let view = Menu::new().item(MenuItem::check(tr(PROJECT_VIEW_KEY), true));
        h_flex()
            .id("explorer-header")
            .h(rems(HEADER_HEIGHT_SPEC / 16.))
            .w_full()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .px_3()
            .child(
                DropdownMenu::new("explorer-header-view", tr(PROJECT_VIEW_KEY), view)
                    .variant(ButtonVariant::Ghost),
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_0p5()
                    .when(loading, |row| {
                        row.child(
                            div()
                                .size_6()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(Spinner::new().color(cx.theme().muted_foreground)),
                        )
                    })
                    .when(!loading, |row| {
                        row.child(testable(
                            "explorer-header-refresh",
                            LitheButton::new("explorer-header-refresh")
                                .icon(IconName::RefreshCw)
                                .variant(ButtonVariant::Ghost)
                                .size(Size::Small)
                                .tooltip(tr(REFRESH_KEY))
                                .on_click(move |_event: &ClickEvent, window, cx| {
                                    let _ = refresh.update(cx, |this, cx| this.refresh(window, cx));
                                }),
                        ))
                    })
                    .child(testable(
                        "explorer-header-collapse",
                        LitheButton::new("explorer-header-collapse")
                            .icon(IconName::ChevronsUpDown)
                            .variant(ButtonVariant::Ghost)
                            .size(Size::Small)
                            .tooltip(tr(COLLAPSE_ALL_KEY))
                            .on_click(move |_event: &ClickEvent, _window, cx| {
                                let _ = collapse.update(cx, |this, cx| this.collapse_all(cx));
                            }),
                    ))
                    .child(testable(
                        "explorer-header-options",
                        OverflowMenu::new("explorer-header-options", options).tooltip(tr(MORE_KEY)),
                    ))
                    .child(testable(
                        "explorer-header-hide",
                        LitheButton::new("explorer-header-hide")
                            .icon(IconName::PanelLeft)
                            .variant(ButtonVariant::Ghost)
                            .size(Size::Small)
                            .tooltip(tr(HIDE_PANEL_KEY))
                            .on_click(move |_event: &ClickEvent, window, cx| {
                                let host = hide
                                    .upgrade()
                                    .and_then(|explorer| explorer.read(cx).hide_host.clone());
                                if let Some(host) = host {
                                    host(window, cx);
                                }
                            }),
                    )),
            )
            .into_any_element()
    }

    /// The `⋮` menu: the focused row's own entries, or, with no row to act on, the same five
    /// actions greyed out — nothing is offered that has nowhere to land.
    fn options_menu(&self, cx: &mut Context<Self>) -> Menu {
        let labels = row_labels();
        let explorer = cx.entity().downgrade();
        let Some((kind, path)) = self.tree.read(cx).focused_row() else {
            let mut menu = Menu::new();
            for action in PROJECT_ACTIONS {
                menu = menu.item(MenuItem::new(action.label(&labels)).disabled(true));
            }
            return menu;
        };
        let mut menu = Menu::new();
        for entry in row_menu_entries(kind) {
            let Some(action) = entry else {
                menu = menu.separator();
                continue;
            };
            let explorer = explorer.clone();
            let path = path.clone();
            menu = menu.item(
                MenuItem::new(action.label(&labels)).on_click(move |window, cx| {
                    let _ = explorer.update(cx, |this, cx| {
                        this.run_row_action(action, path.as_ref(), window, cx);
                    });
                }),
            );
        }
        menu
    }

    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let content: AnyElement = if self.state.is_loading() && !self.has_entries() {
            div().flex_1().into_any_element()
        } else {
            match &self.state {
                LoadState::Failed(error) => self.empty_failed(error.clone(), cx),
                _ if !self.has_entries() => self.empty_no_rows(cx),
                _ => self.render_tree(cx),
            }
        };

        let loading = self.state.is_loading();
        v_flex()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .when(loading, |this| {
                this.child(
                    h_flex()
                        .absolute()
                        .top_3()
                        .left_0()
                        .right_0()
                        .justify_center()
                        .child(
                            h_flex()
                                .gap_2()
                                .px_3()
                                .py_1p5()
                                .rounded_full()
                                .bg(cx.theme().muted)
                                .border_1()
                                .border_color(cx.theme().border)
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(Spinner::new().color(cx.theme().muted_foreground))
                                .child(tr(LOADING_FILES_KEY)),
                        ),
                )
            })
            .child(content)
            .into_any_element()
    }

    fn render_tree(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .id("explorer-tree-viewport")
            .flex_1()
            .min_h_0()
            .w_full()
            .px_1p5()
            .role(Role::Tree)
            .aria_label(tr(TREE_ARIA_LABEL_KEY))
            .track_focus(&self.focus_handle(cx))
            .key_context(crate::EXPLORER_KEY_CONTEXT)
            .on_action(cx.listener(Self::select_all_in_tree))
            .on_action(cx.listener(Self::copy_selection))
            .child(self.tree.clone())
            .into_any_element()
    }

    fn empty_no_rows(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.root.as_os_str().is_empty() {
            return self.empty(
                tr(NO_FOLDER_OPEN_KEY),
                Some((tr(OPEN_FOLDER_KEY), true)),
                cx,
            );
        }

        let message = if self.query.is_empty() {
            tr(EMPTY_FOLDER_KEY)
        } else {
            tr(NO_MATCHING_FILES_KEY)
        };
        self.empty(message, None, cx)
    }

    fn empty_failed(&self, error: String, cx: &mut Context<Self>) -> AnyElement {
        self.empty(SharedString::from(error), Some((tr(RETRY_KEY), false)), cx)
    }

    fn empty(
        &self,
        message: SharedString,
        action: Option<(SharedString, bool)>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut empty = Empty::new().border_color(cx.theme().transparent).header(
            EmptyHeader::new().description(EmptyDescription::new().text_sm().child(message)),
        );

        if let Some((label, disabled)) = action {
            let button = Button::new("explorer-empty-action")
                .label(label)
                .small()
                .compact();
            let button = if disabled {
                button.disabled(true)
            } else {
                button.on_click(cx.listener(
                    |this: &mut Self,
                     _event: &ClickEvent,
                     _window: &mut Window,
                     cx: &mut Context<Self>| {
                        this.load(cx);
                    },
                ))
            };
            empty = empty.content(EmptyContent::new().child(button));
        }

        v_flex().size_full().child(empty).into_any_element()
    }
}

/// The sidebar's own words, in the host's catalogue: the note an empty folder shows and the five
/// row-menu entries.
fn row_labels() -> ProjectLabels {
    ProjectLabels {
        empty_folder: tr(EMPTY_FOLDER_KEY),
        open: tr(OPEN_KEY),
        reveal: tr(REVEAL_KEY),
        copy_path: tr(COPY_PATH_KEY),
        copy_relative_path: tr(COPY_RELATIVE_PATH_KEY),
        refresh: tr(REFRESH_KEY),
    }
}

/// Marks a header control so the UI test lane can find and click it: the gpui-kit test harness
/// looks elements up by debug selector, which an `id` alone does not register.
fn testable(id: &'static str, control: impl IntoElement) -> impl IntoElement {
    div()
        .debug_selector(move || id.to_string())
        .flex()
        .items_center()
        .child(control)
}

fn root_label(root: &Path) -> String {
    root.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| root.to_string_lossy().to_string())
}

fn join_relative(root: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .filter(|segment| !segment.is_empty())
        .fold(root.to_path_buf(), |path, segment| path.join(segment))
}

fn relative_to_root(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let text = relative.to_string_lossy().replace('\\', "/");
    let text = text.trim_matches('/').to_string();
    if text.is_empty() { None } else { Some(text) }
}

impl Render for Explorer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let header = self.render_header(cx);
        let body = self.render_body(cx);

        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(header)
            .child(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_to_root_normalizes_and_rejects_outside_paths() {
        let root = Path::new("workspace");
        assert_eq!(
            relative_to_root(root, &Path::new("workspace").join("src").join("main.rs")),
            Some("src/main.rs".to_string())
        );
        assert_eq!(
            relative_to_root(root, Path::new("workspace")),
            None,
            "工作区根自己没有可定位的行"
        );
        assert_eq!(
            relative_to_root(root, &Path::new("elsewhere").join("a.txt")),
            None
        );
        assert_eq!(
            relative_to_root(root, &Path::new("workspace-other").join("a.txt")),
            None
        );
    }

    #[test]
    fn row_labels_are_distinct_catalog_keys() {
        let labels = row_labels();
        let words = [
            labels.open.to_string(),
            labels.reveal.to_string(),
            labels.copy_path.to_string(),
            labels.copy_relative_path.to_string(),
            labels.refresh.to_string(),
        ];
        for word in &words {
            assert!(
                !word.starts_with("lithe."),
                "文案必须是译文而不是键名（键漏配时 rust-i18n 会原样回显），实际 = {word}"
            );
        }
        for left in 0..words.len() {
            for right in (left + 1)..words.len() {
                assert_ne!(words[left], words[right], "两条动作的文案不许重复");
            }
        }
        assert!(
            !labels.empty_folder.starts_with("lithe."),
            "空目录那一行也必须是译文，实际 = {}",
            labels.empty_folder
        );
    }
}
