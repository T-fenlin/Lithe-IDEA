//! The project tree: the workspace's folders and files as the IDE's left sidebar shows them.
//!
//! The component owns what the sidebar has always owned — which folders are open, which rows are
//! picked, the row budget, the row menu, and the git letter on each row — while the owner keeps
//! its Core reads, its ports (clipboard, reveal), its words, and its own icons. Rows are drawn a
//! row at a time, so a workspace of any size stays quick.

mod menu;
mod node;
mod row;
mod select;
#[cfg(test)]
mod tests;

use std::{
    collections::{BTreeMap, HashSet},
    rc::Rc,
};

use gpui::{
    App, Context, ElementId, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Pixels, Point, Render, ScrollStrategy, SharedString, Styled,
    UniformListScrollHandle, Window, div, uniform_list,
};

pub use menu::{PROJECT_ACTIONS, ProjectAction, ProjectLabels, row_menu_entries};
pub use node::{ProjectNode, ProjectRowKind};

use self::{
    node::{Arena, Row, materialize},
    row::RowData,
    select::ProjectSelection,
};
use crate::{
    forms::{Editing, OnCheck},
    lists::GitStatus,
    menus::{ContextMenu, Menu, MenuItem},
};

/// How many rows the tree materializes before it calls the rest truncated; the sidebar has always
/// stopped at 5000, and saying so is better than drawing a workspace of tens of thousands forever.
pub const ROW_LIMIT: usize = 5_000;

/// The root row's key: the workspace itself, which the arena names `dir:` with an empty path.
const ROOT_KEY: &str = "dir:";

type OnAction = Rc<dyn Fn(ProjectAction, &str, &mut Window, &mut App)>;
type OnRename = Rc<dyn Fn(&str, &str, &mut Window, &mut App)>;
type OnRenameCheck = Rc<dyn Fn(&str, &str) -> Result<(), SharedString>>;
pub(crate) type IconFor = Rc<dyn Fn(&ProjectIconRequest, &App) -> Option<ProjectIcon>>;

/// Where a row's icon comes from: a glyph the tree tints with the row's color, or an image
/// (a file-type icon) drawn as it is. Both name a path in the app's own asset source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectIcon {
    Glyph(SharedString),
    Image(SharedString),
}

/// An asset path the theme picks between: the app's light and dark files for the same drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectAsset {
    pub light: &'static str,
    pub dark: &'static str,
}

impl ProjectAsset {
    /// One file for both themes.
    pub const fn same(path: &'static str) -> Self {
        Self {
            light: path,
            dark: path,
        }
    }

    pub(crate) fn path(&self, dark: bool) -> &'static str {
        if dark { self.dark } else { self.light }
    }
}

/// What the owner is asked to find an icon for; the paths are the app's own asset paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectIconRequest {
    pub kind: ProjectRowKind,
    pub expanded: bool,
    pub path: SharedString,
    pub name: SharedString,
}

/// The workspace's tree: its nodes, what is open, what is picked, and the menu a row opens.
pub struct ProjectTree {
    id: ElementId,
    arena: Rc<Arena>,
    rows: Rc<Vec<Row>>,
    expanded: HashSet<SharedString>,
    selection: ProjectSelection,
    active: Option<SharedString>,
    query: String,
    cursor: Option<SharedString>,
    status: Rc<BTreeMap<SharedString, GitStatus>>,
    groups: Rc<BTreeMap<SharedString, GitStatus>>,
    truncated: bool,
    row_limit: usize,
    scroll: UniformListScrollHandle,
    focus: FocusHandle,
    open_menu: Option<(SharedString, u64, Point<Pixels>)>,
    menu_number: u64,
    pending_scroll: Option<usize>,
    labels: ProjectLabels,
    caret: Option<(ProjectAsset, ProjectAsset)>,
    icon_for: Option<IconFor>,
    on_action: Option<OnAction>,
    on_rename: Option<OnRename>,
    rename_check: Option<OnRenameCheck>,
    /// The row key being renamed in place, if any.
    renaming: Option<SharedString>,
}

impl ProjectTree {
    pub fn new(id: impl Into<ElementId>, cx: &mut Context<Self>) -> Self {
        Self {
            id: id.into(),
            arena: Rc::new(Arena::default()),
            rows: Rc::new(Vec::new()),
            expanded: HashSet::new(),
            selection: ProjectSelection::default(),
            active: None,
            query: String::new(),
            cursor: None,
            status: Rc::new(BTreeMap::new()),
            groups: Rc::new(BTreeMap::new()),
            truncated: false,
            row_limit: ROW_LIMIT,
            scroll: UniformListScrollHandle::default(),
            focus: cx.focus_handle(),
            open_menu: None,
            menu_number: 0,
            pending_scroll: None,
            labels: ProjectLabels::default(),
            caret: None,
            icon_for: None,
            on_action: None,
            on_rename: None,
            rename_check: None,
            renaming: None,
        }
    }

    /// The owner's words for what the tree says on its own.
    pub fn labels(mut self, labels: ProjectLabels) -> Self {
        self.labels = labels;
        self
    }

    /// The shut and open caret assets, so a folder row opens the way the rest of the sidebar does.
    pub fn caret(mut self, shut: ProjectAsset, open: ProjectAsset) -> Self {
        self.caret = Some((shut, open));
        self
    }

    /// Finds the icon for a row in the app's asset paths; without one, rows carry no icon.
    pub fn icon_for(
        mut self,
        find: impl Fn(&ProjectIconRequest, &App) -> Option<ProjectIcon> + 'static,
    ) -> Self {
        self.icon_for = Some(Rc::new(find));
        self
    }

    /// Runs a row's menu action; the owner keeps the ports and the Core work behind it.
    pub fn on_action(
        mut self,
        handler: impl Fn(ProjectAction, &str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(Rc::new(handler));
        self
    }

    /// Commits a row's in-place rename: the node's path and the name the user kept.
    pub fn on_rename(
        mut self,
        handler: impl Fn(&str, &str, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_rename = Some(Rc::new(handler));
        self
    }

    /// Asked before an in-place rename commits, with `(path, new name)`; an `Err` holds the field
    /// open and its reason is drawn beside it. The owner reads the file system here.
    pub fn on_rename_check(
        mut self,
        check: impl Fn(&str, &str) -> Result<(), SharedString> + 'static,
    ) -> Self {
        self.rename_check = Some(Rc::new(check));
        self
    }

    /// How many rows to materialize at most.
    pub fn row_limit(mut self, limit: usize) -> Self {
        self.row_limit = limit;
        self
    }
    /// The workspace and everything under it, already in display order; the root row is open.
    pub fn set_root(
        &mut self,
        label: impl Into<SharedString>,
        nodes: impl IntoIterator<Item = ProjectNode>,
        cx: &mut Context<Self>,
    ) {
        self.arena = Rc::new(Arena::build(label.into(), nodes));
        self.expanded.insert(SharedString::from(ROOT_KEY));
        self.cursor = Some(SharedString::from(ROOT_KEY));
        self.rebuild(cx);
    }

    /// The git state of the workspace's files, by workspace-relative path; every folder takes the
    /// most urgent state under it, and so does the root row.
    pub fn set_status(
        &mut self,
        statuses: impl IntoIterator<Item = (SharedString, GitStatus)>,
        cx: &mut Context<Self>,
    ) {
        let (files, groups) = fold_status(statuses);
        self.status = Rc::new(files);
        self.groups = Rc::new(groups);
        cx.notify();
    }

    /// The row that follows the open document; nothing changes when it is already that row.
    pub fn set_active(&mut self, path: Option<&str>, cx: &mut Context<Self>) {
        let next = path.map(SharedString::from);
        if self.active == next {
            return;
        }
        self.active = next;
        cx.notify();
    }

    /// One selection follows the editor; an explicit group of rows stays as the user left it.
    /// `true` when the selection moved too.
    pub fn follow_active_document(&mut self, path: Option<&str>, cx: &mut Context<Self>) -> bool {
        let next = path.map(SharedString::from);
        if self.active == next {
            return false;
        }
        self.active = next;
        let followed = self.selection.follow_active_document(path);
        cx.notify();
        followed
    }

    /// Opens every folder above `path`, picks it, and scrolls it into view; `false` when the
    /// workspace does not hold that path.
    pub fn reveal(&mut self, path: &str, cx: &mut Context<Self>) -> bool {
        let Some(node) = self.arena.find(path) else {
            return false;
        };
        let key = self.arena.key(node);
        let mut parent = self.arena.parent(node);
        while let Some(ix) = parent {
            self.expanded.insert(self.arena.key(ix));
            parent = self.arena.parent(ix);
        }
        self.active = Some(path.into());
        self.selection.select(path, &[], false, false);
        self.cursor = Some(key.clone());
        self.rebuild(cx);
        self.pending_scroll = self
            .rows
            .iter()
            .position(|row| !row.placeholder && self.arena.key(row.node) == key);
        cx.notify();
        true
    }

    /// Keeps only the rows whose path contains `query`, opening the folders that hold them.
    pub fn set_query(&mut self, query: &str, cx: &mut Context<Self>) {
        if self.query == query {
            return;
        }
        self.query = query.to_string();
        self.rebuild(cx);
    }

    /// The picked paths, in path order.
    pub fn selected_paths(&self) -> Vec<SharedString> {
        self.selection
            .paths()
            .iter()
            .map(SharedString::from)
            .collect()
    }

    /// The picked paths in the order they are shown, the workspace root left out.
    pub fn selected_in_display_order(&self) -> Vec<SharedString> {
        self.rows
            .iter()
            .filter(|row| !self.arena.is_root(row.node))
            .map(|row| self.arena.entry(row.node).path.clone())
            .filter(|path| self.selection.paths().contains(path.as_ref()))
            .collect()
    }

    /// ⌘A: every sibling of the focused row, or the root's children when nothing is focused.
    pub fn select_all_in_siblings(&mut self, cx: &mut Context<Self>) {
        let focused = self.selection.focused().map(str::to_string);
        let parent = focused
            .as_deref()
            .and_then(|path| self.arena.find(path))
            .and_then(|ix| self.arena.parent(ix))
            .unwrap_or(0);
        let siblings: Vec<String> = self
            .arena
            .children(parent)
            .iter()
            .map(|child| self.arena.entry(*child).path.to_string())
            .collect();
        self.selection.select_all_in_siblings(&siblings);
        cx.notify();
    }

    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        self.selection.clear();
        cx.notify();
    }

    /// Shuts every folder under the root row; the root row itself stays open, so the workspace's
    /// own children are still there to walk.
    pub fn collapse_all(&mut self, cx: &mut Context<Self>) {
        self.expanded.retain(|key| key.as_ref() == ROOT_KEY);
        self.cursor = Some(SharedString::from(ROOT_KEY));
        self.rebuild(cx);
    }

    /// The row the keyboard is on, if any: what the panel's own actions act on.
    pub fn focused_row(&self) -> Option<(ProjectRowKind, SharedString)> {
        let ix = self.cursor_row()?;
        let row = self.rows.get(ix)?;
        Some((
            row.kind(&self.arena),
            self.arena.entry(row.node).path.clone(),
        ))
    }

    /// How many rows are now shown. `limit` is how many it will show at most.
    pub fn rows(&self) -> usize {
        self.rows.len()
    }

    /// Whether the row limit cut the tree short.
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// How many files carry a state, and how many folders summarize one.
    pub fn status_counts(&self) -> (usize, usize) {
        (self.status.len(), self.groups.len())
    }

    pub fn limit(&self) -> usize {
        self.row_limit
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let (rows, truncated) =
            materialize(&self.arena, &self.expanded, &self.query, self.row_limit);
        self.rows = Rc::new(rows);
        self.truncated = truncated;
        let visible: Vec<String> = self.visible_paths();
        self.selection.retain(&visible);
        cx.notify();
    }

    fn visible_paths(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|row| self.arena.entry(row.node).path.to_string())
            .collect()
    }

    fn row_of(&self, key: &str) -> Option<usize> {
        self.rows
            .iter()
            .position(|row| row.key(&self.arena).as_ref() == key)
    }

    fn cursor_row(&self) -> Option<usize> {
        let cursor = self.cursor.as_ref()?;
        self.row_of(cursor.as_ref())
    }

    /// A caret was pressed: open a shut folder or shut an open one.
    pub(crate) fn toggle_key(&mut self, key: &SharedString, cx: &mut Context<Self>) {
        if self.expanded.contains(key) {
            self.expanded.remove(key);
        } else {
            self.expanded.insert(key.clone());
        }
        self.cursor = Some(key.clone());
        self.rebuild(cx);
    }

    /// A row was pressed: pick it, and open the file when it is one. `Some` carries the action the
    /// owner has to run, to be run after this update returns (the owner must not be called while
    /// this tree is borrowed).
    pub(crate) fn click_key(
        &mut self,
        key: &SharedString,
        extending: bool,
        toggling: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(ProjectAction, SharedString)> {
        let ix = self.row_of(key)?;
        if self.rows[ix].placeholder {
            return None;
        }
        let node = self.rows[ix].node;
        let path = self.arena.entry(node).path.clone();
        let visible = self.visible_paths();
        self.selection.select(&path, &visible, extending, toggling);
        self.cursor = Some(key.clone());
        self.open_menu = None;
        window.focus(&self.focus, cx);
        let action = match self.arena.entry(node).is_dir {
            true => None,
            false => {
                self.active = Some(path.clone());
                Some((ProjectAction::Open, path))
            }
        };
        cx.notify();
        action
    }

    /// A row was right-pressed: keep a selection that holds it, and ask for its menu.
    pub(crate) fn context_key(
        &mut self,
        key: &SharedString,
        at: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let Some(ix) = self.row_of(key) else {
            return;
        };
        if self.rows[ix].placeholder {
            return;
        }
        let node = self.rows[ix].node;
        let path = self.arena.entry(node).path.to_string();
        self.selection.select_for_context_menu(&path);
        self.cursor = Some(key.clone());
        self.menu_number += 1;
        self.open_menu = Some((key.clone(), self.menu_number, at));
        cx.notify();
    }

    /// Starts an in-place rename on a row, selecting the main name — not the extension, so typing
    /// `Bar` turns `Foo.java` into `Bar.java`. Nothing happens for a note row, the workspace root,
    /// or a row already being renamed. Committing goes through [`Self::on_rename`]; a refusal from
    /// [`Self::on_rename_check`] holds the field open with its reason drawn in the row.
    pub(crate) fn begin_rename(
        &mut self,
        key: &SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.renaming.is_some() {
            return;
        }
        let Some(ix) = self.row_of(key.as_ref()) else {
            return;
        };
        if self.rows[ix].placeholder || self.arena.is_root(self.rows[ix].node) {
            return;
        }
        let entry = self.arena.entry(self.rows[ix].node);
        let (path, name) = (entry.path.to_string(), entry.name.to_string());
        let stem = stem_len(&name);
        self.renaming = Some(key.clone());
        self.open_menu = None;
        // The same keyed state `render` reads, so the field and the row agree on which row edits.
        let editing =
            window.use_keyed_state((self.id.clone(), "rename"), cx, |_, _| Editing::default());
        if let Some(on_rename) = self.on_rename.clone() {
            let path = path.clone();
            editing.update(cx, |editing, _| {
                editing.on_commit = Some(Rc::new(move |name, window, cx| {
                    on_rename(&path, name, window, cx)
                }));
            });
        }
        let check: Option<OnCheck> = self.rename_check.clone().map(|check| {
            let path = path.clone();
            Rc::new(move |name: &SharedString| check(&path, name)) as OnCheck
        });
        Editing::open(&editing, name, Some(0..stem), check, window, cx);
        cx.notify();
    }

    /// Starts an in-place rename on the row the keyboard is on: the `F2` fast path, the same entry
    /// the row menu's Rename uses. Nothing happens for a note row, the workspace root, no row at
    /// all, or a row already being renamed.
    pub fn rename_focused(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = self.cursor.clone() else {
            return;
        };
        self.begin_rename(&key, window, cx);
    }

    /// The rows the keyboard walks, passed over notes.
    fn walk(&self, from: usize, forward: bool) -> Option<usize> {
        let range: Box<dyn Iterator<Item = usize>> = match forward {
            true => Box::new(from + 1..self.rows.len()),
            false => Box::new((0..from).rev()),
        };
        range.into_iter().find(|ix| !self.rows[*ix].placeholder)
    }

    fn pick_row(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let at = self.cursor_row().unwrap_or(0);
        let node = self.rows[ix].node;
        let path = self.arena.entry(node).path.to_string();
        let visible = self.visible_paths();
        self.selection.select(&path, &visible, false, false);
        self.cursor = Some(self.rows[ix].key(&self.arena));
        // The walk keeps the row it landed on in view, from the edge it came from.
        let strategy = if ix < at {
            ScrollStrategy::Top
        } else {
            ScrollStrategy::Bottom
        };
        self.scroll.scroll_to_item(ix, strategy);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // While a row is being renamed its field owns the keyboard; the tree must not also walk.
        if self.rows.is_empty() || self.renaming.is_some() {
            return;
        }
        let at = self.cursor_row().unwrap_or(0);
        let mut handled = true;
        match event.keystroke.key.as_str() {
            "down" => match self.walk(at, true) {
                Some(to) => self.pick_row(to, window, cx),
                None => handled = false,
            },
            "up" => match self.walk(at, false) {
                Some(to) => self.pick_row(to, window, cx),
                None => handled = false,
            },
            "home" => self.pick_row(0, window, cx),
            "end" => self.pick_row(self.rows.len() - 1, window, cx),
            "left" => self.go_left(at, window, cx),
            "right" => self.go_right(at, window, cx),
            "enter" => self.activate(at, window, cx),
            _ => handled = false,
        }
        if handled {
            cx.stop_propagation();
        }
    }

    /// Left shuts an open folder, or climbs to the row above it.
    fn go_left(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let node = self.rows[at].node;
        let key = self.rows[at].key(&self.arena);
        if self.arena.entry(node).is_dir && self.expanded.contains(&key) {
            self.toggle_key(&key, cx);
            return;
        }
        match self.rows[at].parent {
            Some(parent) => self.pick_row(parent, window, cx),
            None => {}
        }
    }

    /// Right opens a shut folder, or steps into an open one.
    fn go_right(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let node = self.rows[at].node;
        let key = self.rows[at].key(&self.arena);
        if !self.arena.entry(node).is_dir {
            return;
        }
        if !self.expanded.contains(&key) {
            self.toggle_key(&key, cx);
            return;
        }
        if let Some(child) = self.walk(at, true)
            && self.rows[child].parent == Some(at)
        {
            self.pick_row(child, window, cx);
        }
    }

    /// Enter opens a file or opens and shuts a folder.
    fn activate(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let node = self.rows[at].node;
        let key = self.rows[at].key(&self.arena);
        if self.arena.entry(node).is_dir {
            self.selection
                .select(&self.arena.entry(node).path.clone(), &[], false, false);
            self.toggle_key(&key, cx);
            return;
        }
        let path = self.arena.entry(node).path.clone();
        self.active = Some(path.clone());
        self.selection.select(&path, &[], false, false);
        let handler = self.on_action.clone();
        cx.notify();
        if let Some(handler) = handler {
            window.defer(cx, move |window, cx| {
                handler(ProjectAction::Open, &path, window, cx)
            });
        }
    }

    /// The menu a row opens, if it has any entries.
    fn row_menu(&self, key: &SharedString, tree: &Entity<Self>) -> Option<Menu> {
        let ix = self.row_of(key.as_ref())?;
        let kind = self.rows[ix].kind(&self.arena);
        let path = self.arena.entry(self.rows[ix].node).path.clone();
        // Renaming or deleting the workspace root itself means nothing.
        let root = self.arena.is_root(self.rows[ix].node);
        let entries = row_menu_entries(kind);
        if entries.is_empty() {
            return None;
        }
        let mut menu = Menu::new();
        for entry in entries {
            let Some(action) = entry else {
                menu = menu.separator();
                continue;
            };
            let label = action.label(&self.labels);
            let disabled = root && matches!(action, ProjectAction::Rename | ProjectAction::Delete);
            let tree = tree.clone();
            let path = path.clone();
            let row = key.clone();
            menu = menu.item(
                MenuItem::new(label)
                    .icon(action.icon())
                    .disabled(disabled)
                    .on_click(move |window, cx| {
                        if action == ProjectAction::Rename {
                            let _ = tree.update(cx, |tree, cx| tree.begin_rename(&row, window, cx));
                            return;
                        }
                        let _ = tree.update(cx, |tree, cx| {
                            tree.open_menu = None;
                            if action == ProjectAction::Open {
                                tree.active = Some(path.clone());
                            }
                            cx.notify();
                        });
                        let handler = tree.read(cx).on_action.clone();
                        if let Some(handler) = handler {
                            handler(action, path.as_ref(), window, cx);
                        }
                    }),
            );
        }
        Some(menu)
    }
}

/// The length of a name's main part: everything before the last dot, except a leading dot that
/// only hides the name (`.gitignore` is all main part). A rename selects this, not the extension.
fn stem_len(name: &str) -> usize {
    match name.rfind('.') {
        None | Some(0) => name.len(),
        Some(dot) => dot,
    }
}

/// Gives a folder the most urgent state seen under it.
fn bump(groups: &mut BTreeMap<SharedString, GitStatus>, folder: &str, status: GitStatus) {
    let key: SharedString = folder.to_string().into();
    match groups.get(&key).copied() {
        Some(known) => {
            if let Some(dominant) = GitStatus::dominant([known, status]) {
                groups.insert(key, dominant);
            }
        }
        None => {
            groups.insert(key, status);
        }
    }
}

/// The file states by path (the first one wins for a path that repeats), and every folder's most
/// urgent state — including the workspace root under `""`, which is what the root row shows.
fn fold_status(
    statuses: impl IntoIterator<Item = (SharedString, GitStatus)>,
) -> (
    BTreeMap<SharedString, GitStatus>,
    BTreeMap<SharedString, GitStatus>,
) {
    let mut files: BTreeMap<SharedString, GitStatus> = BTreeMap::new();
    let mut groups: BTreeMap<SharedString, GitStatus> = BTreeMap::new();
    for (path, status) in statuses {
        if path.is_empty() {
            continue;
        }
        files.entry(path.clone()).or_insert(status);
        let text = path.to_string();
        let mut current: &str = &text;
        loop {
            match current.rfind('/') {
                Some(index) => {
                    current = &current[..index];
                    bump(&mut groups, current, status);
                }
                None => {
                    bump(&mut groups, "", status);
                    break;
                }
            }
        }
    }
    (files, groups)
}

impl Focusable for ProjectTree {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ProjectTree {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(ix) = self.pending_scroll.take() {
            self.scroll.scroll_to_item(ix, ScrollStrategy::Center);
        }
        // The in-place rename's field lives in a keyed state, shared with `begin_rename`. Once the
        // field is gone (Enter, Escape, a leaving field, a committed check) the row stops editing
        // and focus returns to the tree.
        let editing =
            window.use_keyed_state((self.id.clone(), "rename"), cx, |_, _| Editing::default());
        if self.renaming.is_some() && editing.read(cx).field().is_none() {
            self.renaming = None;
            window.focus(&self.focus, cx);
        }
        let rename_refusal = editing.read(cx).refusal();
        let tree = cx.entity();
        let data = Rc::new(RowData {
            id: self.id.clone(),
            arena: self.arena.clone(),
            rows: self.rows.clone(),
            expanded: Rc::new(self.expanded.clone()),
            status: self.status.clone(),
            groups: self.groups.clone(),
            selection: Rc::new(self.selection.clone()),
            active: self.active.clone(),
            caret: self.caret,
            empty_folder: self.labels.empty_folder.clone(),
            icon_for: self.icon_for.clone(),
            tree: tree.clone(),
            editing: editing.clone(),
            renaming: self.renaming.clone(),
            rename_refusal,
        });
        let count = self.rows.len();
        let list = uniform_list(
            (self.id.clone(), "rows"),
            count,
            move |range, window, cx| row::draw(&data, range, window, cx),
        )
        .track_scroll(&self.scroll)
        .size_full();
        let menu = self
            .open_menu
            .as_ref()
            .and_then(|(key, number, at)| Some((self.row_menu(key, &tree)?, *number, *at)));
        let base = div()
            .id(self.id.clone())
            .size_full()
            .flex()
            .flex_col()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key_down));
        // The menu is an overlay, not a wrapper: `ContextMenu`'s own box is a plain div (auto
        // height), so putting the row list inside it collapses the list to nothing — which is
        // exactly what "the menu opened and every row vanished" was. It hangs off a zero-height
        // sibling instead, and `Spot::At` still opens it at the pointer.
        match menu {
            Some((menu, number, at)) => base
                .child(list)
                .child(ContextMenu::new((self.id.clone(), "menu"), menu).manual(Some((number, at))))
                .into_any_element(),
            None => base.child(list).into_any_element(),
        }
    }
}
