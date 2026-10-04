use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::assets::IconName;
use gpui_kit::base::{TreeState, h_flex, v_flex};
use gpui_kit::component::button::Button;
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tree::{Tree, TreeEvent};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Sizable as _, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, ClipboardItem, Context, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, Role, ScrollStrategy,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, WeakEntity, Window,
    div, relative, rems,
};

use lithe_gpui_shared::design::{ROW_LINE_HEIGHT, ROW_RADIUS};
use lithe_gpui_shared::icons::{FileIcon, idea, idea_icon_image};
use lithe_gpui_shared::tr;

use crate::model::{
    BuiltTree, GitTreeStatus, RENDER_LIMIT, ROOT_ID, RowKind, Snapshot, TreeNode, build_tree_items,
    icon_for_file, load_git_status, load_snapshot,
};

const SEARCH_FILES_KEY: &str = "lithe.fileExplorer.searchFiles";
const TREE_ARIA_LABEL_KEY: &str = "lithe.fileExplorer.ariaLabel";
const LOADING_FILES_KEY: &str = "lithe.quickOpen.loadingFiles";
const RETRY_KEY: &str = "lithe.ui.retry";
const NO_FOLDER_OPEN_KEY: &str = "lithe.fileExplorer.noFolderOpen";
const OPEN_FOLDER_KEY: &str = "lithe.titleProject.openFolder";
const NO_MATCHING_FILES_KEY: &str = "lithe.fileExplorer.noMatchingFiles";

const OPEN_KEY: &str = "lithe.files.open";
const REVEAL_KEY: &str = "lithe.files.reveal";
const COPY_PATH_KEY: &str = "lithe.files.copyPath";
const COPY_RELATIVE_PATH_KEY: &str = "lithe.files.copyRelativePath";
const REFRESH_KEY: &str = "lithe.files.refresh";

const BASE_INDENT: f32 = 8.;
const INDENT_STEP: f32 = 14.;

pub type RevealInFileManagerHost = Arc<dyn Fn(&Path, &mut App) + 'static>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RowAction {
    Open,
    RevealInFileManager,
    CopyPath,
    CopyRelativePath,
    Refresh,
}

pub(crate) fn row_menu_entries(kind: RowKind) -> Vec<Option<RowAction>> {
    match kind {
        RowKind::EmptyPlaceholder => Vec::new(),
        RowKind::File => vec![
            Some(RowAction::Open),
            None,
            Some(RowAction::RevealInFileManager),
            Some(RowAction::CopyPath),
            Some(RowAction::CopyRelativePath),
        ],
        RowKind::Directory => vec![
            Some(RowAction::RevealInFileManager),
            Some(RowAction::CopyPath),
            Some(RowAction::CopyRelativePath),
            None,
            Some(RowAction::Refresh),
        ],
    }
}

fn row_action_label(action: RowAction) -> SharedString {
    match action {
        RowAction::Open => tr(OPEN_KEY),
        RowAction::RevealInFileManager => tr(REVEAL_KEY),
        RowAction::CopyPath => tr(COPY_PATH_KEY),
        RowAction::CopyRelativePath => tr(COPY_RELATIVE_PATH_KEY),
        RowAction::Refresh => tr(REFRESH_KEY),
    }
}

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

pub struct Explorer {
    root: PathBuf,
    snapshot: Option<Snapshot>,
    rows: usize,
    truncated: bool,
    state: LoadState,
    tree: Entity<TreeState>,
    expanded: BTreeSet<SharedString>,
    active: Option<SharedString>,
    selection: crate::selection::ProjectTreeSelection,
    focus_handle: gpui_kit::FocusHandle,
    file_clipboard: Option<Box<dyn crate::FileClipboardPort>>,
    decorations: GitTreeStatus,
    #[allow(dead_code)]
    search: Entity<InputState>,
    #[allow(dead_code)]
    search_open: bool,
    query: SharedString,
    on_open: Rc<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>,
    reveal_host: Option<RevealInFileManagerHost>,
    _tree_events: Subscription,
    _search_events: Subscription,
}

impl Explorer {
    pub fn new(
        root: PathBuf,
        on_open: Box<dyn Fn(PathBuf, &mut Window, &mut App) + 'static>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let tree = cx.new(|cx| TreeState::new(cx));
        let search = cx.new(|cx| InputState::new(window, cx));
        search.update(cx, |state, cx| {
            state.set_placeholder(tr(SEARCH_FILES_KEY), window, cx);
        });

        let tree_events = cx.subscribe(
            &tree,
            |this, _tree, event: &TreeEvent, cx: &mut Context<Self>| match event {
                TreeEvent::Expanded(id) => {
                    this.expanded.insert(id.clone());
                }
                TreeEvent::Collapsed(id) => {
                    this.expanded.remove(id);
                    let visible = this.visible_paths(&**cx);
                    this.selection.retain(&visible);
                    cx.notify();
                }
            },
        );

        let search_events = cx.subscribe(
            &search,
            |this, _input, event: &InputEvent, cx: &mut Context<Self>| {
                if matches!(event, InputEvent::Change) {
                    let query = this.search.read(&**cx).value();
                    this.set_query(query, cx);
                }
            },
        );

        let mut explorer = Self {
            root,
            snapshot: None,
            rows: 0,
            truncated: false,
            state: LoadState::Loading,
            tree,
            expanded: BTreeSet::new(),
            active: None,
            selection: crate::selection::ProjectTreeSelection::default(),
            focus_handle: cx.focus_handle(),
            file_clipboard: None,
            decorations: GitTreeStatus::default(),
            search,
            search_open: false,
            query: SharedString::default(),
            on_open: Rc::from(on_open),
            reveal_host: None,
            _tree_events: tree_events,
            _search_events: search_events,
        };
        explorer.expanded.insert(SharedString::new_static(ROOT_ID));
        explorer.load(cx);
        explorer
    }

    pub fn set_reveal_in_file_manager_host(&mut self, host: RevealInFileManagerHost) {
        self.reveal_host = Some(host);
    }

    pub fn refresh(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.load(cx);
    }

    pub fn reveal(&mut self, path: &Path, cx: &mut Context<Self>) {
        let Some(relative) = relative_to_root(&self.root, path) else {
            eprintln!(
                "S1_EXPLORER reveal=outside-root root={} path={}",
                self.root.display(),
                path.display()
            );
            return;
        };

        for ancestor in ancestor_dir_ids(&relative) {
            self.expanded.insert(SharedString::from(ancestor));
        }

        let id = SharedString::from(format!("file:{relative}"));
        self.active = Some(SharedString::from(relative.clone()));
        self.rebuild(cx);
        let mut found = false;
        self.tree.update(cx, |state, cx| {
            state.reveal_item(&id, ScrollStrategy::Center, cx);
            found = state.index_of(&id).is_some();
            let index = state.index_of(&id);
            state.set_selected_index(index, cx);
        });
        eprintln!("S1_EXPLORER reveal path={relative} found={found}");
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        self.state = LoadState::Loading;
        cx.notify();

        if self.root.as_os_str().is_empty() {
            self.snapshot = None;
            self.decorations = GitTreeStatus::default();
            self.state = LoadState::Ready;
            self.rebuild(cx);
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
                        this.snapshot = Some(snapshot);
                        this.state = LoadState::Ready;
                        this.rebuild(cx);
                        eprintln!(
                            "S1_EXPLORER root={} nodes={} rows={} limit={} truncated={} files_not_in_root={missing}",
                            this.root.display(),
                            nodes,
                            this.rows,
                            RENDER_LIMIT,
                            this.truncated,
                        );
                    }
                    Err(error) => {
                        eprintln!("S1_EXPLORER root={} error={error}", this.root.display());
                        this.snapshot = None;
                        this.decorations = GitTreeStatus::default();
                        this.state = LoadState::Failed(error);
                        this.rebuild(cx);
                    }
                }
                match status {
                    Ok(status) => {
                        let (files, directories) = status.counts();
                        let empty = status.is_empty();
                        this.decorations = status;
                        eprintln!(
                            "S1_EXPLORER_GIT root={} files={files} directories={directories} empty={empty}",
                            this.root.display()
                        );
                    }
                    Err(error) => {
                        this.decorations = GitTreeStatus::default();
                        eprintln!("S1_EXPLORER_GIT root={} error={error}", this.root.display());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn built_tree(&self) -> BuiltTree {
        let filter: Option<String> = if self.query.is_empty() {
            None
        } else {
            Some(self.query.to_lowercase())
        };
        let label = root_label(&self.root);
        let empty = TreeNode {
            path: String::new(),
            name: label.clone(),
            is_directory: true,
            children: Vec::new(),
        };
        let root = self
            .snapshot
            .as_ref()
            .map(|snapshot| &snapshot.root)
            .unwrap_or(&empty);
        build_tree_items(&label, root, &self.expanded, filter.as_deref())
    }

    fn visible_paths(&self, cx: &App) -> Vec<String> {
        let state = self.tree.read(cx);
        let mut paths = Vec::new();
        let mut ix = 0;
        while let Some(entry) = state.entry(ix) {
            let (_, relative_path) = RowKind::parse(entry.item().id.as_ref());
            paths.push(relative_path);
            ix += 1;
        }
        paths
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let built = self.built_tree();
        self.rows = built.rows;
        self.truncated = built.truncated;
        self.tree
            .update(cx, |state, cx| state.set_items(built.items, cx));
    }

    fn select_all_in_tree(
        &mut self,
        _: &crate::SelectAllInTree,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(snapshot) = self.snapshot.as_ref() else {
            return;
        };
        let siblings = crate::model::displayed_siblings(&snapshot.root, self.selection.focused());
        self.selection.select_all_in_siblings(&siblings);
        cx.notify();
    }

    pub fn selected_paths(&self) -> Vec<String> {
        self.selection.paths().iter().cloned().collect()
    }

    pub fn focus_handle(&self) -> &gpui_kit::FocusHandle {
        &self.focus_handle
    }

    pub fn set_file_clipboard(&mut self, port: Box<dyn crate::FileClipboardPort>) {
        self.file_clipboard = Some(port);
    }

    fn copy_selection(
        &mut self,
        _: &crate::CopySelection,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        let (Some(port), Some(snapshot)) = (self.file_clipboard.as_ref(), self.snapshot.as_ref())
        else {
            return;
        };
        let payload = crate::model::selected_in_display_order(
            &snapshot.root,
            &self.expanded,
            self.selection.paths(),
        );
        if payload.is_empty() {
            return;
        }
        let absolute: Vec<PathBuf> = payload
            .iter()
            .map(|relative| join_relative(&self.root, relative))
            .collect();
        port.write_paths(&absolute);
    }

    pub fn follow_active_document(&mut self, path: Option<&str>, cx: &mut Context<Self>) {
        let next = path.map(SharedString::from);
        if self.active == next {
            return;
        }
        self.active = next;
        if self.selection.follow_active_document(path) {
            cx.notify();
        }
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

    #[allow(dead_code)]
    fn set_query(&mut self, query: SharedString, cx: &mut Context<Self>) {
        if self.query == query {
            return;
        }
        self.query = query;
        self.rebuild(cx);
        cx.notify();
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

    fn run_row_action(
        &mut self,
        action: RowAction,
        relative: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            RowAction::Open => {
                let path = join_relative(&self.root, relative);
                (self.on_open.clone())(path, window, cx);
                self.active = Some(SharedString::from(relative.to_string()));
                cx.notify();
            }
            RowAction::RevealInFileManager => {
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
            RowAction::CopyPath => {
                let path = join_relative(&self.root, relative);
                let text = path.to_string_lossy().to_string();
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                eprintln!("S1_EXPLORER copyPath value={text}");
            }
            RowAction::CopyRelativePath => {
                let text = if relative.is_empty() {
                    ".".to_string()
                } else {
                    relative.to_string()
                };
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
                eprintln!("S1_EXPLORER copyRelativePath value={text}");
            }
            RowAction::Refresh => self.load(cx),
        }
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
        let on_open = self.on_open.clone();
        let active = self.active.clone();
        let root = self.root.clone();
        let decorations = Rc::new(self.decorations.clone());
        let selection = self.selection.clone();
        let explorer: WeakEntity<Self> = cx.entity().downgrade();
        let menu_explorer = explorer.clone();
        let focus_handle = self.focus_handle.clone();

        let tree = Tree::new(&self.tree, move |_index, entry, selected, _window, cx| {
            let id = entry.item().id.clone();
            let label = entry.item().label.clone();
            let is_folder = entry.is_folder();
            let expanded = entry.is_expanded();
            let (kind, relative_path) = RowKind::parse(id.as_ref());
            let is_active =
                kind == RowKind::File && active.as_deref() == Some(relative_path.as_str());

            let caret: AnyElement = if is_folder {
                h_flex()
                    .size_4()
                    .mr_0p5()
                    .flex_shrink_0()
                    .justify_center()
                    .child(idea_icon_image(
                        if expanded {
                            &idea::CARET_DOWN_ICON
                        } else {
                            &idea::CARET_RIGHT_ICON
                        },
                        cx,
                        rems(0.75),
                    ))
                    .into_any_element()
            } else {
                div().size_4().mr_0p5().flex_shrink_0().into_any_element()
            };

            let icon = match kind {
                RowKind::EmptyPlaceholder => FileIcon::lucide(IconName::File),
                RowKind::Directory => FileIcon::lucide(if expanded {
                    IconName::FolderOpen
                } else {
                    IconName::FolderClosed
                })
                .with_theme_folder(&label, expanded, cx),
                RowKind::File => icon_for_file(&label, cx),
            };

            let decoration = decorations.file(&relative_path);
            let tone = if kind == RowKind::File {
                decoration.map(|decoration| decoration.tone)
            } else {
                decorations.directory(&relative_path)
            };

            let mut label_el = div().flex_1().min_w_0().truncate().child(label);
            if entry.is_root() {
                label_el = label_el.font_semibold();
            }
            if let Some(tone) = tone {
                label_el = label_el.text_color(tone.color(cx));
            }

            let mut row = h_flex()
                .w_full()
                .h_6()
                .gap_1()
                .pl(rems(
                    (BASE_INDENT + entry.depth() as f32 * INDENT_STEP) / 16.,
                ))
                .pr_1p5()
                .rounded(rems(ROW_RADIUS / 16.))
                .child(caret)
                .child(icon.render(rems(1.), cx))
                .child(label_el);

            if let Some(decoration) = decoration {
                row = row.child(
                    div()
                        .flex_shrink_0()
                        .text_size(rems(9. / 16.))
                        .font_weight(FontWeight::BOLD)
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_color(decoration.tone.color(cx))
                        .child(decoration.letter.clone()),
                );
            }

            if is_active || selection.covers(&relative_path) {
                row = row.bg(cx.theme().list_active);
            }

            let opening: Option<PathBuf> = if kind == RowKind::File {
                Some(join_relative(&root, &relative_path))
            } else {
                None
            };

            let row_on_open = on_open.clone();
            let row_explorer = explorer.clone();
            let row_focus = focus_handle.clone();
            let row_relative_path = relative_path;
            let row_path = opening;

            ListItem::new(SharedString::from(explorer_row_element_id(id.as_ref())))
                .selected(selected)
                .p_0()
                .h_6()
                .text_sm()
                .line_height(relative(ROW_LINE_HEIGHT))
                .whitespace_nowrap()
                .overflow_hidden()
                .child(row)
                .on_click(
                    move |event: &ClickEvent, window: &mut Window, cx: &mut App| {
                        let extending = event.modifiers().shift;
                        let toggling = event.modifiers().secondary();
                        window.focus(&row_focus, cx);
                        let is_file_row = row_path.is_some();
                        let _ = row_explorer.update(cx, |this, cx| {
                            let visible = this.visible_paths(&**cx);
                            this.selection.select(
                                row_relative_path.as_str(),
                                &visible,
                                extending,
                                toggling,
                            );
                            if is_file_row {
                                this.active = Some(SharedString::from(row_relative_path.clone()));
                            }
                            cx.notify();
                        });
                        if let Some(path) = row_path.clone() {
                            row_on_open(path, window, cx);
                        }
                    },
                )
        })
        .size_full()
        .context_menu(move |_index, entry, menu, _window, cx| {
            let (kind, relative_path) = RowKind::parse(entry.item().id.as_ref());
            let _ = menu_explorer.update(&mut *cx, |this, cx| {
                this.selection.select_for_context_menu(&relative_path);
                cx.notify();
            });
            let mut menu: PopupMenu = menu;
            for action in row_menu_entries(kind) {
                let Some(action) = action else {
                    menu = menu.separator();
                    continue;
                };
                let menu_explorer = menu_explorer.clone();
                let relative_path = relative_path.clone();
                menu = menu.item(PopupMenuItem::new(row_action_label(action)).on_click(
                    move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
                        let _ = menu_explorer.update(cx, |this, cx| {
                            this.run_row_action(action, &relative_path, window, cx);
                        });
                    },
                ));
            }
            menu
        });

        v_flex()
            .id("explorer-tree-viewport")
            .flex_1()
            .min_h_0()
            .w_full()
            .px_1p5()
            .role(Role::Tree)
            .aria_label(tr(TREE_ARIA_LABEL_KEY))
            .track_focus(&self.focus_handle)
            .key_context(crate::EXPLORER_KEY_CONTEXT)
            .on_action(cx.listener(Self::select_all_in_tree))
            .on_action(cx.listener(Self::copy_selection))
            .child(tree)
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
            tr("lithe.fileExplorer.folderIsEmpty")
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

fn root_label(root: &Path) -> String {
    root.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| root.to_string_lossy().to_string())
}

fn explorer_row_element_id(row_id: &str) -> String {
    format!("explorer-row:{row_id}")
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

fn ancestor_dir_ids(relative: &str) -> Vec<String> {
    let segments: Vec<&str> = relative
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let mut ids = vec![ROOT_ID.to_string()];
    let mut prefix = String::new();
    for segment in segments.iter().take(segments.len().saturating_sub(1)) {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(segment);
        ids.push(format!("dir:{prefix}"));
    }
    ids
}

impl Render for Explorer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = self.render_body(cx);

        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_element_ids_do_not_depend_on_the_visible_row_order() {
        let nodes = [
            "dir:",
            "dir:src",
            "file:src/main.rs",
            "empty:src/empty-folder",
        ];

        let forward: Vec<String> = nodes.iter().map(|id| explorer_row_element_id(id)).collect();
        let reversed: Vec<String> = nodes
            .iter()
            .rev()
            .map(|id| explorer_row_element_id(id))
            .collect();

        assert_eq!(forward[0], reversed[3], "根行的 id 必须与行号无关");
        assert_eq!(forward[3], reversed[0], "空目录占位行的 id 必须与行号无关");
        assert_eq!(forward[1], "explorer-row:dir:src", "id 必须由节点 id 构成");
        for left in 0..nodes.len() {
            for right in (left + 1)..nodes.len() {
                assert_ne!(forward[left], forward[right]);
            }
        }
    }

    #[test]
    fn row_menu_entries_match_the_macos_context_menus() {
        assert_eq!(
            row_menu_entries(RowKind::File),
            vec![
                Some(RowAction::Open),
                None,
                Some(RowAction::RevealInFileManager),
                Some(RowAction::CopyPath),
                Some(RowAction::CopyRelativePath),
            ],
            "文件行照真源 fileContextMenuItems（ProjectSidebarView.swift:704-745）"
        );
        assert_eq!(
            row_menu_entries(RowKind::Directory),
            vec![
                Some(RowAction::RevealInFileManager),
                Some(RowAction::CopyPath),
                Some(RowAction::CopyRelativePath),
                None,
                Some(RowAction::Refresh),
            ],
            "目录行照真源 directoryContextMenuItems（同文件 :574-648），末尾那条刷新是真的重取快照"
        );
        assert!(
            row_menu_entries(RowKind::EmptyPlaceholder).is_empty(),
            "空目录占位行是禁用行，不该有右键菜单"
        );
    }

    #[test]
    fn row_menu_has_no_write_actions() {
        let mut actions: Vec<RowAction> = row_menu_entries(RowKind::File)
            .into_iter()
            .chain(row_menu_entries(RowKind::Directory))
            .flatten()
            .collect();
        actions.sort_by_key(|action| format!("{action:?}"));
        actions.dedup();
        assert_eq!(
            actions,
            vec![
                RowAction::CopyPath,
                RowAction::CopyRelativePath,
                RowAction::Open,
                RowAction::Refresh,
                RowAction::RevealInFileManager,
            ],
            "菜单里的动作集合必须正好是这 5 条（新增变体前先确认真源有落点）"
        );
    }

    #[test]
    fn row_action_labels_are_distinct_catalog_keys() {
        let labels: Vec<String> = [
            RowAction::Open,
            RowAction::RevealInFileManager,
            RowAction::CopyPath,
            RowAction::CopyRelativePath,
            RowAction::Refresh,
        ]
        .iter()
        .map(|action| row_action_label(*action).to_string())
        .collect();
        for label in &labels {
            assert!(
                !label.starts_with("lithe."),
                "文案必须是译文而不是键名（键漏配时 rust-i18n 会原样回显），实际 = {label}"
            );
        }
        for left in 0..labels.len() {
            for right in (left + 1)..labels.len() {
                assert_ne!(labels[left], labels[right], "两条动作的文案不许重复");
            }
        }
    }

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
    fn ancestor_dir_ids_walk_from_the_root_row_to_the_parent_directory() {
        assert_eq!(
            ancestor_dir_ids("src/main/java/App.java"),
            vec![
                "dir:".to_string(),
                "dir:src".to_string(),
                "dir:src/main".to_string(),
                "dir:src/main/java".to_string(),
            ]
        );
        assert_eq!(
            ancestor_dir_ids("main.rs"),
            vec!["dir:".to_string()],
            "根下的文件只需要展开根行"
        );
    }

    #[test]
    fn indentation_follows_the_macos_metrics() {
        assert_eq!(BASE_INDENT, 8.);
        assert_eq!(INDENT_STEP, 14.);
        assert_eq!(BASE_INDENT + 2. * INDENT_STEP, 36.);
    }
}
