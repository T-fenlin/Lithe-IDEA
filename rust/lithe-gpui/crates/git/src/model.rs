use std::collections::BTreeSet;
use std::path::Path;

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{App, ClickEvent, Hsla, SharedString, Window};

use lithe_gpui_shared::{CoreClient, CoreRequest, tr, tr_args};

pub(crate) const RADIUS_BASE: f32 = 8.;
pub(crate) const RADIUS_SM: f32 = RADIUS_BASE * 0.6;
pub(crate) const RADIUS_MD: f32 = RADIUS_BASE * 0.8;
pub(crate) const RADIUS_XL: f32 = RADIUS_BASE * 1.4;

pub(crate) const REFERENCE_PILL_MAX_WIDTH: f32 = 240.;
pub(crate) const REFERENCE_PILL_RADIUS: f32 = RADIUS_MD;
pub(crate) const ICON_BUTTON_RADIUS: f32 = RADIUS_MD;

pub(crate) const FILTER_INPUT_MIN_WIDTH: f32 = 144.;
pub(crate) const FILTER_INPUT_MAX_WIDTH: f32 = 288.;

pub(crate) const COMMIT_ROW_HEIGHT: f32 = 30.;
pub(crate) const AUTHOR_COLUMN_WIDTH: f32 = 112.;
pub(crate) const DATE_FONT_SIZE: f32 = 11.;
pub(crate) const COMMIT_CONTENT_MIN_WIDTH: f32 = 520.;
pub(crate) const LOAD_MORE_BUTTON_HEIGHT: f32 = 24.;

pub(crate) const GRAPH_LANE_GAP: f32 = 13.;
pub(crate) const GRAPH_MIN_WIDTH: f32 = 30.;
pub(crate) const GRAPH_LINE_WIDTH: f32 = 1.6;
pub(crate) const GRAPH_NODE_RADIUS: f32 = 4.3;
pub(crate) const LABEL_MAX_WIDTH: f32 = 112.;
pub(crate) const LABEL_FONT_SIZE: f32 = 10.;
pub(crate) const LABEL_RADIUS: f32 = RADIUS_MD;
pub(crate) const LABEL_PADDING_Y: f32 = 2.;

pub(crate) const REFERENCE_PANE_FRACTION: f32 = 0.19;
pub(crate) const COMMIT_PANE_FRACTION: f32 = 0.57;
pub(crate) const INSPECTOR_PANE_FRACTION: f32 = 0.24;
pub(crate) const REFERENCE_PANE_MIN_WIDTH: f32 = 140.;
pub(crate) const INSPECTOR_PANE_MIN_WIDTH: f32 = 220.;

pub(crate) const REFERENCE_TOOLBAR_BUTTON_RADIUS: f32 = RADIUS_SM;
pub(crate) const REFERENCE_HEAD_ROW_RADIUS: f32 = RADIUS_MD;
pub(crate) const REFERENCE_SECTION_RADIUS: f32 = RADIUS_MD;
pub(crate) const REFERENCE_ROW_RADIUS: f32 = RADIUS_SM;
pub(crate) const REFERENCE_INDENT_BASE: f32 = 10.;
pub(crate) const REFERENCE_INDENT_STEP: f32 = 14.;
pub(crate) const REFERENCE_BADGE_FONT_SIZE: f32 = 10.;
pub(crate) const TRACKING_COUNT_FONT_SIZE: f32 = 10.;
pub(crate) const TRACKING_COUNT_MAX: usize = 99;

pub(crate) const INSPECTOR_FILES_FRACTION: f32 = 0.62;
pub(crate) const INSPECTOR_DETAILS_FRACTION: f32 = 0.38;
pub(crate) const INSPECTOR_FILES_MIN_HEIGHT: f32 = 90.;
pub(crate) const INSPECTOR_MONO_FONT_SIZE: f32 = 11.;
pub(crate) const INSPECTOR_HASH_FONT_SIZE: f32 = 10.;
pub(crate) const COMMIT_FILE_INDENT_BASE: f32 = 10.;
pub(crate) const COMMIT_FILE_INDENT_STEP: f32 = 14.;
pub(crate) const COMMIT_FILE_STATUS_FONT_SIZE: f32 = 10.;

pub(crate) const HISTORY_PAGE_LIMIT: usize = 100;
pub(crate) const REPOSITORY_PROBE_LIMIT: usize = 6;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Panel {
    Log,
    Console,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoadState {
    Loading,
    Ready,
    Stale,
    Failed,
    NoRepository,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterScope {
    Text,
    Author,
    Branch,
}

impl FilterScope {
    fn label_key(self) -> &'static str {
        match self {
            FilterScope::Text => "lithe.git.log.filterText",
            FilterScope::Author => "lithe.git.log.filterAuthor",
            FilterScope::Branch => "lithe.git.log.filterBranch",
        }
    }

    pub(crate) fn label(self) -> SharedString {
        tr(self.label_key())
    }

    pub(crate) fn placeholder(self) -> SharedString {
        tr_args(
            "lithe.git.log.filterPlaceholder",
            &[("field", self.label().as_ref())],
        )
    }

    pub(crate) fn all() -> [FilterScope; 3] {
        [FilterScope::Text, FilterScope::Author, FilterScope::Branch]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefKind {
    Local,
    Remote,
    Tag,
}

impl RefKind {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "local" => Some(RefKind::Local),
            "remote" => Some(RefKind::Remote),
            "tag" => Some(RefKind::Tag),
            _ => None,
        }
    }

    pub(crate) fn id(self) -> &'static str {
        match self {
            RefKind::Local => "local",
            RefKind::Remote => "remote",
            RefKind::Tag => "tag",
        }
    }

    pub(crate) fn title(self) -> SharedString {
        tr(match self {
            RefKind::Local => "lithe.git.log.local",
            RefKind::Remote => "lithe.git.log.remote",
            RefKind::Tag => "lithe.git.log.tags",
        })
    }

    pub(crate) fn sections() -> [RefKind; 3] {
        [RefKind::Local, RefKind::Remote, RefKind::Tag]
    }
}

#[derive(Clone)]
pub(crate) struct Reference {
    pub(crate) full_name: SharedString,
    pub(crate) short_name: SharedString,
    pub(crate) kind: RefKind,
    pub(crate) is_current: bool,
    pub(crate) upstream_short_name: Option<SharedString>,
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LabelKind {
    Head,
    Remote,
    Tag,
    Branch,
}

#[derive(Clone)]
pub(crate) struct Label {
    pub(crate) title: SharedString,
    pub(crate) kind: LabelKind,
}

#[derive(Clone)]
pub(crate) struct Commit {
    pub(crate) hash: SharedString,
    pub(crate) short_hash: SharedString,
    pub(crate) parent_hashes: Vec<SharedString>,
    pub(crate) subject: SharedString,
    pub(crate) author: SharedString,
    pub(crate) email: SharedString,
    pub(crate) date: SharedString,
    pub(crate) labels: Vec<Label>,
}

pub(crate) struct GraphRow {
    pub(crate) lanes: Vec<Option<usize>>,
    pub(crate) lane: usize,
    pub(crate) node_color: usize,
    pub(crate) edges: Vec<(usize, usize, bool)>,
}

#[derive(Clone)]
pub(crate) struct CommitFileRow {
    pub(crate) depth: usize,
    pub(crate) name: SharedString,
    pub(crate) is_folder: bool,
    pub(crate) file_count: usize,
    pub(crate) status: SharedString,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilesState {
    Idle,
    Loading,
    Ready,
    Failed,
}

pub(crate) struct Detail {
    pub(crate) subject: SharedString,
    pub(crate) short_hash: SharedString,
    pub(crate) author: SharedString,
    pub(crate) email: SharedString,
    pub(crate) date: SharedString,
    pub(crate) decorations: SharedString,
    pub(crate) hash: SharedString,
}

pub(crate) struct HistoryCursor {
    pub(crate) root: String,
    pub(crate) cursor: String,
}

impl HistoryCursor {
    pub(crate) fn close(self) {
        let _ = execute_core(
            "git.historyCursorClose",
            serde_json::json!({ "root": self.root, "cursor": self.cursor }),
        );
    }
}

pub(crate) struct FirstLoad {
    pub(crate) repository_root: Option<String>,
    pub(crate) branch: Option<SharedString>,
    pub(crate) references: Vec<Reference>,
    pub(crate) commits: Vec<Commit>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) has_more: bool,
    pub(crate) failures: Vec<String>,
}

pub(crate) struct MoreLoad {
    pub(crate) commits: Vec<Commit>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) has_more: bool,
    pub(crate) failure: Option<String>,
}

pub(crate) type ButtonHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

pub(crate) fn handler(f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> ButtonHandler {
    Box::new(f)
}

pub(crate) const GIT_TIMEOUT_MILLIS: u64 = 60_000;

pub(crate) fn execute_core(
    command: &str,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    CoreClient::new()
        .execute(
            &CoreRequest::command(command)
                .with_payload(payload)
                .with_timeout_millis(GIT_TIMEOUT_MILLIS),
        )
        .map(|data| data.unwrap_or(serde_json::Value::Null))
        .map_err(|error| match error.code() {
            Some(code) => code.to_string(),
            None => error.to_string(),
        })
}

pub(crate) fn resolve_repository_root(root: &Path, repository_root: &str) -> String {
    let path = Path::new(repository_root);
    if path.is_absolute() {
        repository_root.to_string()
    } else {
        root.join(path).to_string_lossy().to_string()
    }
}

pub(crate) fn repository_root_of(data: &serde_json::Value) -> Option<String> {
    data.get("repositoryRoot")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

pub(crate) fn read_status(root: &str) -> Result<(Option<String>, Option<SharedString>), String> {
    let data = execute_core("git.status", serde_json::json!({ "root": root }))?;
    let branch = data
        .get("branch")
        .and_then(serde_json::Value::as_str)
        .map(SharedString::from);
    Ok((repository_root_of(&data), branch))
}

pub(crate) fn discover_repository_root(root: &Path) -> Option<String> {
    let data = execute_core(
        "workspace.snapshot",
        serde_json::json!({ "root": root.to_string_lossy() }),
    )
    .ok()?;

    let children = data.pointer("/root/children")?.as_array()?;
    let directories = children.iter().filter_map(|child| {
        let is_directory = child
            .get("isDirectory")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let path = child.get("path").and_then(serde_json::Value::as_str)?;
        is_directory.then(|| root.join(path).to_string_lossy().to_string())
    });

    for candidate in directories.take(REPOSITORY_PROBE_LIMIT) {
        if let Ok(data) = execute_core("git.status", serde_json::json!({ "root": candidate })) {
            if let Some(found) = repository_root_of(&data) {
                return Some(resolve_repository_root(root, &found));
            }
        }
    }

    None
}

pub(crate) fn parse_references(data: &serde_json::Value) -> Vec<Reference> {
    data.get("references")
        .and_then(serde_json::Value::as_array)
        .map(|references| {
            references
                .iter()
                .filter_map(|reference| {
                    Some(Reference {
                        full_name: SharedString::from(reference.get("fullName")?.as_str()?),
                        short_name: SharedString::from(reference.get("shortName")?.as_str()?),
                        kind: RefKind::parse(reference.get("kind")?.as_str()?)?,
                        is_current: reference
                            .get("isCurrent")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false),
                        upstream_short_name: reference
                            .get("upstreamShortName")
                            .and_then(serde_json::Value::as_str)
                            .map(SharedString::from),
                        ahead: reference
                            .get("ahead")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0) as usize,
                        behind: reference
                            .get("behind")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or(0) as usize,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn parse_decorations(value: &str) -> Vec<Label> {
    let mut labels = Vec::new();

    for part in value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        if let Some(branch) = part.strip_prefix("HEAD -> ") {
            labels.push(Label {
                title: SharedString::from("HEAD"),
                kind: LabelKind::Head,
            });
            labels.push(Label {
                title: SharedString::from(branch.to_string()),
                kind: LabelKind::Branch,
            });
            continue;
        }

        if let Some(tag) = part.strip_prefix("tag: ") {
            labels.push(Label {
                title: SharedString::from(tag.to_string()),
                kind: LabelKind::Tag,
            });
            continue;
        }

        let (title, kind) = if part == "HEAD" {
            (part, LabelKind::Head)
        } else if part.contains('/') {
            (part, LabelKind::Remote)
        } else {
            (part, LabelKind::Branch)
        };
        labels.push(Label {
            title: SharedString::from(title.to_string()),
            kind,
        });
    }

    labels
}

pub(crate) fn parse_commits(data: &serde_json::Value) -> Vec<Commit> {
    data.get("commits")
        .and_then(serde_json::Value::as_array)
        .map(|commits| {
            commits
                .iter()
                .filter_map(|commit| {
                    Some(Commit {
                        hash: SharedString::from(commit.get("hash")?.as_str()?),
                        short_hash: SharedString::from(
                            commit
                                .get("shortHash")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        parent_hashes: commit
                            .get("parentHashes")
                            .and_then(serde_json::Value::as_array)
                            .map(|parents| {
                                parents
                                    .iter()
                                    .filter_map(serde_json::Value::as_str)
                                    .map(SharedString::from)
                                    .collect()
                            })
                            .unwrap_or_default(),
                        subject: SharedString::from(
                            commit
                                .get("subject")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        author: SharedString::from(
                            commit
                                .get("authorName")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        email: SharedString::from(
                            commit
                                .get("authorEmail")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        date: SharedString::from(
                            commit
                                .get("date")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                        labels: parse_decorations(
                            commit
                                .get("decorations")
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or_default(),
                        ),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn parse_history_page(data: &serde_json::Value) -> (Vec<Commit>, Option<String>, bool) {
    let commits = parse_commits(data);
    let next_cursor = data
        .get("nextCursor")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let has_more = data
        .get("hasMore")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(next_cursor.is_some());
    (commits, next_cursor, has_more)
}

pub(crate) fn read_history_page(
    root: &str,
    reference: Option<&str>,
    cursor: Option<&str>,
) -> Result<(Vec<Commit>, Option<String>, bool), String> {
    let data = execute_core(
        "git.historyPage",
        serde_json::json!({
            "root": root,
            "reference": reference,
            "order": "date",
            "cursor": cursor,
            "limit": HISTORY_PAGE_LIMIT,
        }),
    )?;
    Ok(parse_history_page(&data))
}

pub(crate) fn load_first(
    root: &Path,
    reference: Option<String>,
    old_cursor: Option<HistoryCursor>,
) -> FirstLoad {
    if let Some(cursor) = old_cursor {
        cursor.close();
    }

    let mut load = FirstLoad {
        repository_root: None,
        branch: None,
        references: Vec::new(),
        commits: Vec::new(),
        next_cursor: None,
        has_more: false,
        failures: Vec::new(),
    };

    let root_text = root.to_string_lossy().to_string();

    match read_status(&root_text) {
        Ok((repository_root, branch)) => {
            load.repository_root =
                repository_root.map(|found| resolve_repository_root(root, &found));
            load.branch = branch;
        }
        Err(code) => load.failures.push(format!("git.status（{code}）")),
    }

    if load.repository_root.is_none() {
        load.repository_root = discover_repository_root(root);
    }

    let Some(repository_root) = load.repository_root.clone() else {
        return load;
    };

    match execute_core(
        "git.references",
        serde_json::json!({ "root": repository_root.clone() }),
    ) {
        Ok(data) => load.references = parse_references(&data),
        Err(code) => load.failures.push(format!("git.references（{code}）")),
    }

    match read_history_page(&repository_root, reference.as_deref(), None) {
        Ok((commits, next_cursor, has_more)) => {
            load.commits = commits;
            load.next_cursor = next_cursor;
            load.has_more = has_more;
        }
        Err(code) => load.failures.push(format!("git.historyPage（{code}）")),
    }

    load
}

pub(crate) fn load_more(root: &str, reference: Option<String>, cursor: &str) -> MoreLoad {
    match read_history_page(root, reference.as_deref(), Some(cursor)) {
        Ok((commits, next_cursor, has_more)) => MoreLoad {
            commits,
            next_cursor,
            has_more,
            failure: None,
        },
        Err(code) => MoreLoad {
            commits: Vec::new(),
            next_cursor: None,
            has_more: false,
            failure: Some(code),
        },
    }
}

pub(crate) fn load_commit_files(root: &str, commit: &str) -> Result<Vec<(String, String)>, String> {
    let data = execute_core(
        "git.commitFiles",
        serde_json::json!({ "root": root, "commit": commit }),
    )?;

    let files = data
        .get("files")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "响应缺少 files".to_string())?;

    Ok(files
        .iter()
        .filter_map(|file| {
            let path = file.get("path")?.as_str()?;
            let status = file
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("M");
            (!path.is_empty()).then(|| (status.to_string(), path.to_string()))
        })
        .collect())
}

pub(crate) fn lane_color(index: usize, cx: &App) -> Hsla {
    let theme = cx.theme();
    match index % 6 {
        0 => theme.success,
        1 => theme.blue_light,
        2 => theme.magenta_light,
        3 => theme.warning,
        4 => theme.danger,
        _ => theme.cyan_light,
    }
}

pub(crate) fn label_color(kind: LabelKind, cx: &App) -> Hsla {
    let theme = cx.theme();
    match kind {
        LabelKind::Head => theme.cyan_light,
        LabelKind::Remote => theme.blue_light,
        LabelKind::Tag => theme.yellow_light,
        LabelKind::Branch => theme.green_light,
    }
}

pub(crate) fn layout_graph(commits: &[Commit]) -> Vec<GraphRow> {
    let known: BTreeSet<&SharedString> = commits.iter().map(|commit| &commit.hash).collect();
    let mut lanes: Vec<Option<SharedString>> = Vec::new();
    let mut rows = Vec::with_capacity(commits.len());

    for commit in commits {
        let lane = match lanes
            .iter()
            .position(|expected| expected.as_ref() == Some(&commit.hash))
        {
            Some(lane) => lane,
            None => {
                lanes.push(None);
                lanes.len() - 1
            }
        };

        let mut lane_colors: Vec<Option<usize>> = lanes
            .iter()
            .enumerate()
            .map(|(index, expected)| expected.as_ref().map(|_| index % 6))
            .collect();
        let node_color = lane_colors.get(lane).copied().flatten().unwrap_or(lane % 6);

        let mut edges: Vec<(usize, usize, bool)> = Vec::new();
        for (position, parent) in commit.parent_hashes.iter().enumerate() {
            let target = if position == 0 {
                lane
            } else if let Some(existing) = lanes
                .iter()
                .position(|expected| expected.as_ref() == Some(parent))
            {
                existing
            } else if let Some(free) = lanes.iter().position(Option::is_none) {
                free
            } else {
                lanes.push(None);
                lanes.len() - 1
            };

            let missing = !known.contains(parent);
            edges.push((target, target % 6, missing));
            lanes[target] = Some(parent.clone());
            while lane_colors.len() <= target {
                lane_colors.push(None);
            }
        }
        if commit.parent_hashes.is_empty() {
            lanes[lane] = None;
        }

        if let Some((widest, _, _)) = edges.iter().max_by_key(|(lane, _, _)| *lane) {
            while lane_colors.len() <= *widest {
                lane_colors.push(None);
            }
        }

        rows.push(GraphRow {
            lanes: lane_colors,
            lane,
            node_color,
            edges,
        });

        while lanes.last().is_some_and(|lane| lane.is_none()) {
            lanes.pop();
        }
    }

    rows
}

struct RefNode {
    name: SharedString,
    id: String,
    reference: Option<usize>,
    children: Vec<RefNode>,
}

pub(crate) struct RefRow {
    pub(crate) depth: usize,
    pub(crate) name: SharedString,
    pub(crate) id: String,
    pub(crate) reference: Option<usize>,
}

pub(crate) fn build_reference_rows(references: &[Reference], kind: RefKind) -> Vec<RefRow> {
    let mut roots: Vec<RefNode> = Vec::new();

    for (index, reference) in references.iter().enumerate() {
        if reference.kind != kind {
            continue;
        }

        let segments: Vec<&str> = reference
            .short_name
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();
        let Some((leaf, directories)) = segments.split_last() else {
            continue;
        };

        let mut path = String::new();
        let mut level = &mut roots;
        for directory in directories {
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(directory);
            let id = format!("{}/{}", kind.id(), path);
            let position = match level.iter().position(|node| node.id == id) {
                Some(position) => position,
                None => {
                    level.push(RefNode {
                        name: SharedString::from((*directory).to_string()),
                        id: id.clone(),
                        reference: None,
                        children: Vec::new(),
                    });
                    level.len() - 1
                }
            };
            level = &mut level[position].children;
        }

        let mut leaf_path = path.clone();
        if !leaf_path.is_empty() {
            leaf_path.push('/');
        }
        leaf_path.push_str(leaf);
        level.push(RefNode {
            name: SharedString::from((*leaf).to_string()),
            id: format!("{}/{}", kind.id(), leaf_path),
            reference: Some(index),
            children: Vec::new(),
        });
    }

    let mut rows = Vec::new();
    flatten_reference_nodes(&roots, 0, &mut rows);
    rows
}

fn flatten_reference_nodes(nodes: &[RefNode], depth: usize, rows: &mut Vec<RefRow>) {
    for node in nodes {
        rows.push(RefRow {
            depth,
            name: node.name.clone(),
            id: node.id.clone(),
            reference: node.reference,
        });
        flatten_reference_nodes(&node.children, depth + 1, rows);
    }
}

#[derive(Default)]
struct FileNode {
    dirs: Vec<(String, FileNode)>,
    files: Vec<(String, String)>,
}

impl FileNode {
    fn insert(&mut self, path: &str, status: &str) {
        let segments: Vec<&str> = path.split('/').collect();
        let Some((name, directories)) = segments.split_last() else {
            return;
        };

        let mut node = self;
        for directory in directories {
            let position = match node.dirs.iter().position(|(key, _)| key == directory) {
                Some(position) => position,
                None => {
                    node.dirs
                        .push(((*directory).to_string(), FileNode::default()));
                    node.dirs.len() - 1
                }
            };
            node = &mut node.dirs[position].1;
        }
        node.files.push(((*name).to_string(), status.to_string()));
    }

    fn file_count(&self) -> usize {
        self.files.len()
            + self
                .dirs
                .iter()
                .map(|(_, child)| child.file_count())
                .sum::<usize>()
    }

    fn flatten(&self, depth: usize, rows: &mut Vec<CommitFileRow>) {
        for (name, child) in &self.dirs {
            rows.push(CommitFileRow {
                depth,
                name: SharedString::from(name.clone()),
                is_folder: true,
                file_count: child.file_count(),
                status: SharedString::from(""),
            });
            child.flatten(depth + 1, rows);
        }

        for (name, status) in &self.files {
            rows.push(CommitFileRow {
                depth,
                name: SharedString::from(name.clone()),
                is_folder: false,
                file_count: 0,
                status: SharedString::from(status.clone()),
            });
        }
    }
}

pub(crate) fn build_commit_files(files: &[(String, String)]) -> Vec<CommitFileRow> {
    let mut root = FileNode::default();
    for (status, path) in files {
        root.insert(path, status);
    }

    let mut rows = Vec::new();
    root.flatten(0, &mut rows);
    rows
}

pub(crate) fn matches_filter(commit: &Commit, query: &str, scope: FilterScope) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }

    match scope {
        FilterScope::Author => contains(&commit.author, &query) || contains(&commit.email, &query),
        FilterScope::Branch => commit
            .labels
            .iter()
            .any(|label| contains(&label.title, &query)),
        FilterScope::Text => {
            contains(&commit.subject, &query)
                || contains(&commit.hash, &query)
                || contains(&commit.short_hash, &query)
        }
    }
}

pub(crate) fn contains(value: &SharedString, query: &str) -> bool {
    value.to_lowercase().contains(query)
}

pub(crate) fn tracking_count(value: usize) -> SharedString {
    if value > TRACKING_COUNT_MAX {
        SharedString::from(format!("{TRACKING_COUNT_MAX}+"))
    } else {
        SharedString::from(value.to_string())
    }
}
