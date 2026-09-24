//! Dock 里的面板：`ShellPanel` 与它承载的内容类型 `PanelKind`。
//!
//! 一个面板同时实现 gpui-kit dock 要求的 `BasePanel`（持久化用的稳定名字）与
//! `Panel`（标题等表现信息），以及 `Focusable`、`EventEmitter<PanelEvent>` 和 `Render`。
//! 面板内容按 [`PanelKind`] 分支渲染：文件树、编辑器、终端占位、大纲占位、
//! 命令面板、项目统计表。
//!
//! 编辑区（[`PanelKind::Editor`]）自带一条**内部标签栏**（`← →` + 当前文件标签）和
//! 空状态，见 [`ShellPanel::render_editor`]：真机的编辑区就是"标签栏 + 正文"两层，
//! dock 只提供外层标签头，内层这条只能由面板自己渲染。
//!
//! 面板持有的 `TreeState` / `CommandState` / `TableState` / `EditorState` 是各内容
//! 自己的交互状态；跨模块（工作台）需要读写的字段标成 `pub(super)`。
//!
//! 项目树（[`PanelKind::Files`]）在"折叠箭头 + 文件图标 + 缩进 + 点击打开"之上补了两件事：
//!
//! 1. 文件行右侧的 **Git 状态字母**（9pt bold 等宽，数据来自 Core 的 `git.status`，
//!    见 [`load_git_marks`]）；目录行**不画字母**，只给文件名染色
//!    （`ProjectSidebarView.swift:435,479-485`）；
//! 2. 文件/目录行的**右键菜单**（菜单项、顺序与文案照 `ProjectSidebarView.swift:525-696`
//!    与 `macos/Resources/zh-Hans.lproj/Localizable.strings`，见 [`ShellPanel::render_files`]）。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gpui_kit::base::{TreeEntry, TreeState};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::component::dock::{BasePanel, Panel, PanelEvent};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::input::{Editor, EditorState};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::tab::{Tab, TabBar, TabVariant};
use gpui_kit::component::table::{DataTable, TableState};
use gpui_kit::component::tree::Tree;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, h_flex, v_flex,
};
use gpui_kit::{
    AnyElement, App, AppContext as _, ClipboardItem, Context, Entity, EventEmitter, FocusHandle,
    Focusable, FontWeight, Hsla, InteractiveElement as _, IntoElement, IsZero as _,
    ParentElement as _, Render, ScrollWheelEvent, SharedString, Styled as _, WeakEntity, Window,
    div, point, px, relative,
};

use super::stats::StatsDelegate;

/// 编辑器一次最多读入的字节数。超过这个大小的文件只在标签里保留名字，
/// 正文给出一条说明，避免把整个仓库里的巨型文件读进内存。
const MAX_EDITOR_BYTES: u64 = 2 * 1024 * 1024;

/// 项目树行高：macOS 默认 24pt、用户可调 20...32（`AppSettings.projectTreeRowHeight`，
/// `.artifacts/ui-map/03-explorer.md` §2.1）。
///
/// 树体是 `uniform_list`（`gpui-base-0.6.6/src/tree.rs:423`），行高只取**第 0 行**的测量值
/// 再按固定值铺满，所以所有行必须同高：行内容按这个值定高，行内纵向内边距一律 `py_0()`。
const TREE_ROW_HEIGHT: f32 = 24.;

/// 项目树每层的缩进步长，照 macOS 的 `depth * 14`（`ProjectSidebarView.swift:375,441,487`）。
const TREE_INDENT_STEP: f32 = 14.;

/// 右键菜单里"在访达/资源管理器中显示"的文案。
///
/// macOS 的中文是"在 Finder 中显示"（`macos/Resources/zh-Hans.lproj/Localizable.strings`
/// 的 `"Show in Finder"` 条目），但探针跑在 Windows 上、动作也是 `explorer.exe`，
/// 所以按平台等价物改成 Windows 的说法，动作与出处见 [`ShellPanel::tree_context_menu`]。
const REVEAL_IN_FILE_BROWSER_LABEL: &str = "在文件资源管理器中显示";

/// Git 变更的颜色角色。
///
/// 取值顺序就是 macOS 的紧急度优先级
/// （`macos/Sources/LitheGitModule/Models/GitModels.swift:856-865`：
/// modified < copied < moved < added < deleted < conflicted），
/// 所以 `derive(PartialOrd, Ord)` 的顺序就是"谁更紧急"，目录行投影直接比较即可。
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum GitMarkKind {
    Modified,
    Copied,
    Moved,
    Added,
    Deleted,
    Conflicted,
}

/// 文件行右侧的状态字母。
#[derive(Clone)]
struct GitMark {
    /// 冲突 `!`、未跟踪 `A`，否则 porcelain 的状态字符
    /// （`GitModels.swift:764-769` 的 `displayStatus`）。
    letter: SharedString,
    /// 字母与文件名的颜色角色。
    kind: GitMarkKind,
}

/// 项目树的 Git 标记。
///
/// **只有文件行有字母**：目录行不画徽标，只把文件名染成最紧急子文件的颜色
/// （`ProjectSidebarView.swift:435` 目录文件名的 `gitStatusColor`、`:479-485` 文件行徽标）。
#[derive(Default)]
struct GitMarks {
    /// 文件（工作区相对路径）→ 状态字母。
    files: HashMap<String, GitMark>,
    /// 目录（工作区相对路径）→ 最紧急的子文件状态，只用于染色。
    directories: HashMap<String, GitMarkKind>,
}

/// 状态字母与文件名的颜色。
///
/// macOS 的映射是 `LitheTheme.accent`/`success`/`error`/`skill`/`warning`
/// （`ProjectSidebarView.swift:698-709`），这里按项目既有约定落到 gpui 主题 token
/// （`gpui/UI-MAP.md` §1.2；同 `bottom_panel.rs:439-444` 的提交状态字配色）：
/// modified → `primary`（macOS `accent`）、added/copied → `success`、
/// deleted → `danger`（macOS `error`）、conflicted → `warning`；
/// macOS 的 `skill` 是紫色，gpui 主题没有紫色 token，取最接近的 `info`。
fn git_mark_color(kind: GitMarkKind, cx: &App) -> Hsla {
    match kind {
        GitMarkKind::Modified => cx.theme().primary,
        GitMarkKind::Added | GitMarkKind::Copied => cx.theme().success,
        GitMarkKind::Deleted => cx.theme().danger,
        GitMarkKind::Moved => cx.theme().info,
        GitMarkKind::Conflicted => cx.theme().warning,
    }
}

/// 从 porcelain v1 的两字符状态码推断颜色角色，
/// 判定顺序照 `GitModels.swift:748-757` 的 `GitChange.kind`：
/// 先判冲突（`AA`/`DD` 否则会被下面的 added/deleted 吃掉），再 A/D/R/C，最后退回 modified。
fn git_mark_kind(status: &str, untracked: bool) -> GitMarkKind {
    let mut chars = status.chars();
    let index = chars.next().unwrap_or(' ');
    let worktree = chars.next().unwrap_or(' ');

    let conflicted = index == 'U'
        || worktree == 'U'
        || (index == 'A' && worktree == 'A')
        || (index == 'D' && worktree == 'D');

    if conflicted {
        GitMarkKind::Conflicted
    } else if untracked || index == 'A' || worktree == 'A' {
        GitMarkKind::Added
    } else if index == 'D' || worktree == 'D' {
        GitMarkKind::Deleted
    } else if index == 'R' || worktree == 'R' {
        GitMarkKind::Moved
    } else if index == 'C' || worktree == 'C' {
        GitMarkKind::Copied
    } else {
        GitMarkKind::Modified
    }
}

/// 文件行显示的字母，照 `GitModels.swift:764-769` 的 `displayStatus`：
/// 冲突 `!`、未跟踪 `A`，否则优先工作区状态字符、退化到索引状态字符。
fn git_mark_letter(status: &str, untracked: bool) -> SharedString {
    let mut chars = status.chars();
    let index = chars.next().unwrap_or(' ');
    let worktree = chars.next().unwrap_or(' ');

    if git_mark_kind(status, untracked) == GitMarkKind::Conflicted {
        return SharedString::from("!");
    }
    if untracked {
        return SharedString::from("A");
    }
    if worktree != ' ' {
        return SharedString::from(worktree.to_string());
    }
    SharedString::from(index.to_string())
}

/// 把仓库相对路径换算成**工作区相对路径**（树节点 id 里用的那条）。
///
/// `git.status.repositoryRoot` 的三种形态（`shared/contracts/rust-core-api.md:283-287`）：
/// 工作区就是仓库根时是 `.`；工作区在仓库子目录里时是**绝对路径**（只能靠 `strip_prefix`
/// 判断变更是否落在工作区内）；仓库在工作区下面时是相对工作区的路径（把它当前缀拼回去）。
fn workspace_relative_path(root: &Path, repository_root: &str, path: &str) -> Option<String> {
    let repository_root = Path::new(repository_root);

    if repository_root.is_absolute() {
        let absolute = repository_root.join(path);
        let relative = absolute.strip_prefix(root).ok()?;
        return Some(relative.to_string_lossy().replace('\\', "/"));
    }
    if repository_root.as_os_str().is_empty() || repository_root == Path::new(".") {
        return Some(path.to_string());
    }

    Some(format!(
        "{}/{}",
        repository_root
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/'),
        path
    ))
}

/// 把工作区相对路径（Core 约定用 `/` 分隔，`shared/contracts/rust-core-api.md:283`）
/// 拼成宿主绝对路径；逐段 `join` 是为了让结果用平台自己的分隔符
/// （Windows 的 `explorer.exe /select,` 不认正斜杠）。
fn absolute_path(root: &Path, relative: &str) -> PathBuf {
    relative
        .split('/')
        .filter(|segment| !segment.is_empty())
        .fold(root.to_path_buf(), |path, segment| path.join(segment))
}

/// 调 Core 的 `git.status`，把仓库相对路径换成工作区相对路径后投影到树节点上。
///
/// 请求信封与 [`super::files::load_files`] 的 `workspace.snapshot` 完全一致
/// （`id` / `operationId` / `timeoutMilliseconds` / `command` / `payload`），payload 只有
/// `root` 一个字段（`GitStatusRequest`，`rust/lithe-core/src/git/mod.rs:91-96`；
/// 命令名登记在 `rust/lithe-core/src/protocol/command.rs:401`）。
/// 响应取 `/data/...`（`GitStatusResponse`，`rust/lithe-core/src/protocol/contracts.rs:529-538`）：
/// `repositoryRoot` / `branch` / `ahead` / `behind` / `changes[]`，每条变更含
/// `path` / `originalPath` / `status` / `staged` / `worktree` / `untracked`（`:515-527`）。
///
/// **失败时的行为**：工作区不是 Git 仓库（Core 对非仓库返回 `repositoryRoot: null`，
/// `git/mod.rs:6536-6544`）、命令失败（`ok: false`）、响应缺字段、JSON 解析失败、
/// 路径不是合法 UTF-8 —— 一律返回**空标记**：文件行不显示字母、目录行不着色，
/// 也**不报错、不弹窗**（macOS 同样"没有 Git 状态就不画徽标"，`ProjectSidebarView.swift:480`）。
fn load_git_marks(root: &Path) -> GitMarks {
    let mut marks = GitMarks::default();
    let Some(root_text) = root.to_str() else {
        return marks;
    };

    let request = serde_json::json!({
        "id": "shell-git-status",
        "operationId": "shell-git-status",
        "timeoutMilliseconds": 120_000,
        "command": "git.status",
        "payload": { "root": root_text },
    })
    .to_string();

    let raw = lithe_core::execute_json(&request);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return marks;
    };
    if value.get("ok").and_then(serde_json::Value::as_bool) != Some(true) {
        return marks;
    }

    let Some(repository_root) = value
        .pointer("/data/repositoryRoot")
        .and_then(serde_json::Value::as_str)
    else {
        return marks;
    };
    let Some(changes) = value
        .pointer("/data/changes")
        .and_then(serde_json::Value::as_array)
    else {
        return marks;
    };

    for change in changes {
        let Some(path) = change.get("path").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(status) = change.get("status").and_then(serde_json::Value::as_str) else {
            continue;
        };
        // 老响应可能没有 `untracked`：porcelain 的 `??` 就是未跟踪。
        let untracked = change
            .get("untracked")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(status == "??");
        let Some(relative) = workspace_relative_path(root, repository_root, path) else {
            continue;
        };

        let kind = git_mark_kind(status, untracked);
        marks.files.insert(
            relative.clone(),
            GitMark {
                letter: git_mark_letter(status, untracked),
                kind,
            },
        );

        // 目录行只染色：把子文件的状态投影到每一级祖先，取最紧急的那个
        // （`GitModels.swift:806-871` 的 `GitTreeStatusProjection`，优先级见 `GitMarkKind`）。
        let mut ancestor = relative.as_str();
        while let Some(slash) = ancestor.rfind('/') {
            ancestor = &ancestor[..slash];
            if ancestor.is_empty() {
                break;
            }
            let current = marks
                .directories
                .entry(ancestor.to_string())
                .or_insert(kind);
            if kind > *current {
                *current = kind;
            }
        }
    }

    marks
}

/// 编辑区当前打开的文档。
///
/// 标签上显示**文件名**而不是整条路径（真机就是文件名，名字长了截断），
/// `relative_path` 留着给后续的面包屑 / 同名文件区分 / 保存用。
struct OpenDocument {
    /// 文件名，显示在标签上。
    name: SharedString,
    /// 相对工作区根的路径，用于读回文件内容与后续显示面包屑。
    relative_path: SharedString,
    /// 标签左侧的文件类型图标。
    icon: IconName,
}

/// 按文件名后缀挑一个默认图标集里**确实存在**的字形。
///
/// 应用注册的是 `gpui_kit::assets::Assets`（只含 `default-icons.txt` 里那 101 个字形），
/// 所以这里只能从 `FileText` / `File` / `GalleryVerticalEnd` 三个里挑；
/// 语言级的专属字形（`FileCode` 等）不在默认集里，会被画成空白。
fn icon_for_file(name: &str) -> IconName {
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());

    match extension.as_deref() {
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "svg" | "bmp") => {
            IconName::GalleryVerticalEnd
        }
        Some(
            "rs" | "ts" | "tsx" | "js" | "jsx" | "json" | "md" | "toml" | "yaml" | "yml" | "css"
            | "html" | "sh" | "py" | "swift" | "vue" | "txt" | "lock",
        ) => IconName::FileText,
        _ => IconName::File,
    }
}

/// 面板承载的内容类型。
pub(super) enum PanelKind {
    /// 文件列表，对应 Windows 的 `MainSidebar` / 项目树。
    Files,
    /// 代码编辑器，对应 Windows 的编辑区与标签页。
    Editor,
    /// 大纲，对应 Windows 右侧工具窗。
    Outline,
    /// 命令面板，对应 Windows 的双击 Shift 命令面板（目前是浮层的占位实体）。
    Command,
    /// 项目统计表，用来验证 `DataTable` 组件；对应 Windows 右侧工具窗里的列表视图。
    Stats,
}

/// 一个 Dock 面板。
///
/// `BasePanel` 提供身份与持久化所需的稳定名字；`Panel` 提供标题等表现信息；
/// 两者都由 gpui-kit 的 dock 要求实现。
pub(super) struct ShellPanel {
    /// 稳定名字，用于布局持久化与恢复注册。
    name: &'static str,
    /// 标签与标题上显示的文字。文件面板加载完成后会更新它（`项目 · <总数>`）。
    pub(super) title: SharedString,
    kind: PanelKind,
    /// 项目树的层次状态；只有 Files 面板会用到。
    pub(super) tree: Entity<TreeState>,
    /// 命令面板的交互状态；只有 Command 面板会用到。
    palette: Entity<CommandState>,
    /// 统计表状态；只有 Stats 面板会用到。
    pub(super) stats: Entity<TableState<StatsDelegate>>,
    editor: Entity<EditorState>,
    /// 编辑区打开的文档；`None` 表示没有打开文件，编辑区显示空状态。
    document: Option<OpenDocument>,
    /// 工作区根目录。Editor 面板用它把相对路径解析成绝对路径并读盘；
    /// Files 面板用它调 `git.status` 与拼右键菜单动作（复制路径 / 在资源管理器中显示）的绝对路径。
    root: PathBuf,
    /// 项目树的 Git 标记；只有 Files 面板会用到。
    ///
    /// 启动时由后台的 `git.status` 填一次（见 [`load_git_marks`]），拿不到仓库或命令失败时保持空。
    /// 渲染闭包要求 `'static` 且每帧都要取一份，所以用 `Rc` 共享而不是克隆整个 `HashMap`。
    git_marks: Rc<GitMarks>,
    /// 点击文件时要打开到的编辑区面板；只有 Files 面板会用到。
    ///
    /// 用弱引用而不是强引用：两个面板都由 Dock 持有，互相强引用会形成环。
    open_target: Option<WeakEntity<ShellPanel>>,
    focus_handle: FocusHandle,
}

impl ShellPanel {
    /// 文件面板。`editor` 是点击文件后要打开到的编辑区面板。
    pub(super) fn files(
        window: &mut Window,
        cx: &mut Context<Self>,
        editor: WeakEntity<ShellPanel>,
    ) -> Self {
        // 工作区根的持有者是编辑区面板（`ShellPanel::editor` 的 `root` 参数），文件面板的
        // 构造签名里没有它，所以开工前先读一次：这是 `open_target` 之外唯一现成的来源。
        // （改 `files()` 的签名去收 `root` 要同步 `workspace.rs`，不在本次改动范围内。）
        let root = editor
            .read_with(cx, |panel, _| panel.root.clone())
            .unwrap_or_default();

        // Git 状态是后台 Core 命令，回前台写进面板再重画：调用形态照
        // `workspace.rs:142-174` 的工作区快照加载（`cx.spawn` + `cx.background_spawn`）。
        // 拿不到字母时保持空标记，不报错也不弹窗（见 [`load_git_marks`]）。
        if !root.as_os_str().is_empty() {
            let git_root = root.clone();
            cx.spawn(async move |panel, cx| {
                let marks = cx
                    .background_spawn(async move { load_git_marks(&git_root) })
                    .await;
                let _ = panel.update(cx, |panel, cx| {
                    panel.git_marks = Rc::new(marks);
                    cx.notify();
                });
            })
            .detach();
        }

        Self {
            name: "FilesPanel",
            title: "项目".into(),
            kind: PanelKind::Files,
            tree: cx.new(|cx| TreeState::new(cx)),
            palette: cx.new(|cx| CommandState::new(window, cx)),
            stats: cx.new(|cx| {
                TableState::new(
                    StatsDelegate { rows: Vec::new() },
                    window,
                    cx,
                )
            }),
            editor: cx.new(|cx| EditorState::new(window, cx)),
            document: None,
            root,
            git_marks: Rc::new(GitMarks::default()),
            open_target: Some(editor),
            focus_handle: cx.focus_handle(),
        }
    }

    pub(super) fn editor(window: &mut Window, cx: &mut Context<Self>, root: PathBuf) -> Self {
        Self {
            name: "EditorPanel",
            // Dock 的标签头必须写点什么。文件名已经由面板**内部**的标签栏承担
            // （真机也只有那一条），所以这里只留面板级名字，避免两行都写文件名。
            title: "编辑器".into(),
            kind: PanelKind::Editor,
            tree: cx.new(|cx| TreeState::new(cx)),
            palette: cx.new(|cx| CommandState::new(window, cx)),
            stats: cx.new(|cx| {
                TableState::new(
                    StatsDelegate { rows: Vec::new() },
                    window,
                    cx,
                )
            }),
            editor: cx.new(|cx| EditorState::new(window, cx)),
            // 一开始没有打开任何文件：真机启动时编辑区就是空状态，打开动作由用户触发。
            document: None,
            root,
            git_marks: Rc::new(GitMarks::default()),
            open_target: None,
            focus_handle: cx.focus_handle(),
        }
    }

    /// 终端与大纲面板目前只有标题，用来验证 Dock 的分割与标签。
    pub(super) fn placeholder(
        name: &'static str,
        title: &'static str,
        kind: PanelKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            name,
            title: title.into(),
            kind,
            tree: cx.new(|cx| TreeState::new(cx)),
            palette: cx.new(|cx| CommandState::new(window, cx)),
            stats: cx.new(|cx| {
                TableState::new(
                    StatsDelegate { rows: Vec::new() },
                    window,
                    cx,
                )
            }),
            editor: cx.new(|cx| EditorState::new(window, cx)),
            document: None,
            root: PathBuf::new(),
            git_marks: Rc::new(GitMarks::default()),
            open_target: None,
            focus_handle: cx.focus_handle(),
        }
    }

    /// 把工作区里的一个文件读进编辑区。
    ///
    /// `relative_path` 是相对工作区根的路径（文件树节点的 id 里带的就是它）。
    /// 读文件在这里是同步的，只在用户点击时发生，读的是普通源文件尺寸；
    /// 三种情况不把内容交给编辑器，而是在正文里留一条说明：
    /// 读不到、超过 [`MAX_EDITOR_BYTES`]、含 NUL 字节（二进制）。
    pub(super) fn open_document(
        &mut self,
        relative_path: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 已经打开的文件不重复读盘（后续接入 Core 的文件监听后，内容更新由事件驱动）。
        if self
            .document
            .as_ref()
            .is_some_and(|document| document.relative_path.as_ref() == relative_path)
        {
            return;
        }

        let absolute = self.root.join(relative_path);
        let name: SharedString = absolute
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| relative_path.to_string())
            .into();

        let notice = match std::fs::metadata(&absolute) {
            Err(error) => Some(format!("读取失败：{error}")),
            Ok(metadata) if metadata.len() > MAX_EDITOR_BYTES => Some(format!(
                "文件超过 {} MiB，暂不在编辑器中打开。",
                MAX_EDITOR_BYTES / (1024 * 1024)
            )),
            Ok(_) => match std::fs::read(&absolute) {
                Err(error) => Some(format!("读取失败：{error}")),
                Ok(bytes) if bytes.contains(&0) => {
                    Some("二进制文件不在编辑器中打开。".to_string())
                }
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes).to_string();
                    self.set_document(name, relative_path, text, window, cx);
                    return;
                }
            },
        };

        // 打不开的文件也占一个标签（真机就是这样：标签在、正文给出原因），
        // 正文里的 `//` 前缀表示这行是我们补的说明，不是文件内容。
        let body = format!("// {}", notice.unwrap_or_default());
        self.set_document(name, relative_path, body, window, cx);
    }

    /// 写入当前文档并刷新编辑区。
    fn set_document(
        &mut self,
        name: SharedString,
        relative_path: &str,
        body: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.document = Some(OpenDocument {
            icon: icon_for_file(&name),
            name,
            relative_path: SharedString::from(relative_path.to_string()),
        });
        self.editor
            .update(cx, |state, cx| state.set_value(&body, window, cx));
        cx.notify();
    }

    /// 编辑区：内部标签栏 + 正文；没有打开文件时正文位置是空状态。
    ///
    /// 结构照 Windows 源码 `features/panes/components/pane-container.tsx:1094-1100`：
    /// `<TabBar/>` 在上、正文在 `relative min-h-0 flex-1 overflow-hidden` 里，
    /// 没有活动 buffer 时正文位置换成 `EmptyEditorState`（标签栏仍在），
    /// 所以这里标签栏两种情况都渲染，只有正文区切换。
    fn render_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let body = if self.document.is_some() {
            // ⚠️ **必须显式给高度**：多行输入（代码编辑器就是多行）在组件里走的是
            // `.h_auto()`（`gpui-component-0.6.6/src/input/input.rs:706-709`），
            // 高度 = **内容高度**，而内容高度来自 `LayoutMode::CodeEditor { rows }`
            // （默认 `rows: 2`，`gpui-base-0.6.6/src/input/base/mode.rs:83-85`）。
            // 不给高度时编辑区就只有两行高、下面整片空白 —— 这正是"编辑器没铺满、
            // 只有一行"的原因。`relative(1.)` 等价于 `Input::full_height()`
            // （`input.rs:250-252`），让它填满外层那格。
            // ⚠️ **滚轮要自己接**：编辑器元素（`gpui-base-0.6.6/src/input/base/element.rs:202-213`）
            // 的样式只有 `position: absolute` + `100%`，**没有** `overflow: scroll`，
            // 而 gpui 只在"命中的元素样式带 `Overflow::Scroll`"时才把滚轮交给它
            // （`gpui-pre-0.3.6/src/elements/div.rs:3332-3370`）。结果是内容多了滚不动。
            // 这里在**包裹层**接滚轮，把增量交给编辑器自己的滚动偏移
            // （`InputBaseState::{scroll_offset, set_scroll_offset}`，
            // `input/base/state.rs:2806-2817`；后者会自己 clamp、下一帧生效）。
            div()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .on_scroll_wheel(cx.listener(|panel, event: &ScrollWheelEvent, _window, cx| {
                    // 行高拿不到（还没首次布局）时退到 20px，避免整段滚不动。
                    let line_height = panel.editor.read(cx).line_height().unwrap_or(px(20.));
                    let delta = event.delta.pixel_delta(line_height).y;
                    if delta.is_zero() {
                        return;
                    }
                    let current = panel.editor.read(cx).scroll_offset();
                    panel.editor.update(cx, |state, cx| {
                        state.set_scroll_offset(point(current.x, current.y + delta), cx);
                    });
                }))
                .child(Editor::new(&self.editor).h(relative(1.)))
                .into_any_element()
        } else {
            Self::render_empty_editor(cx)
        };

        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .child(self.render_editor_tabs(cx))
            .child(body)
            .into_any_element()
    }

    /// 编辑区内部的标签栏：左侧 `← →`，然后是当前文件标签。
    ///
    /// 结构与文案照 Windows 源码，观感用 gpui-kit 的组件与主题 token 实现
    /// （不逐层翻译 Tailwind class）：
    ///
    /// - 元素组成见 `features/tabs/components/tab-bar.tsx:633-660`（左侧前进/后退）、
    ///   `tab-bar-item.tsx:132-292`（标签：图标 + 文件名 + 关闭按钮）；
    /// - 标签条底色 `bg-tab-bar` + 底边框、高度 `--lithe-tab-bar-height: 2.25rem = 36px`、
    ///   标签宽度上限 `--lithe-tab-max-width: 12.5rem = 200px`、条内边距
    ///   `--lithe-chrome-padding-inline: 8px`（`ui/tab-bar.tsx:244-255`、
    ///   `styles/theme.css:120-133`）；
    /// - 活动标签在 Windows 里是**透明底 + 底边 3px 主色条**
    ///   （`ui/tab-bar.tsx:204-210`，注释写明是 IntelliJ 风格），
    ///   对应 gpui-kit 的 `TabVariant::Underline`（`tab/tab.rs:253-262`：
    ///   透明底 + 底边 2px `primary`）；Underline 的标签条底色是透明的，
    ///   所以再用 `.bg(cx.theme().tab_bar)` 补回 Windows 的 `bg-tab-bar`。
    ///   它的标签条高度正好也是 36px（`tab/tab.rs:38-41`）。
    /// - Underline 变体的条内边距与标签内边距都是 0（`tab/tab.rs:72-89`），
    ///   所以 8px 的条内边距、8px 的标签内边距、6px 的图标-文字间距
    ///   （`--lithe-chrome-gap-loose`）由这里显式给。
    /// - `max_width` 让**标签文字**在空间不够时让位（图标与 `✕` 保持原尺寸），
    ///   也就是真机那种 `OrderChargeService.j…` 的截断。
    fn render_editor_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs = TabBar::new("editor-tabs")
            .with_variant(TabVariant::Underline)
            .bg(cx.theme().tab_bar)
            .px(px(8.))
            .max_width(px(200.))
            .prefix(
                h_flex()
                    .items_center()
                    .gap(px(2.))
                    .child(Self::nav_button(
                        "editor-back",
                        IconName::ArrowLeft,
                        "后退",
                        "后退到上一个位置",
                    ))
                    .child(Self::nav_button(
                        "editor-forward",
                        IconName::ArrowRight,
                        "前进",
                        "前进到下一个位置",
                    )),
            );

        match self.document.as_ref() {
            Some(document) => tabs.selected_index(0).child(
                Tab::new()
                    .child(
                        h_flex()
                            .items_center()
                            .px(px(8.))
                            .gap(px(6.))
                            .min_w_0()
                            // Windows 里图标是 `text-subtle-foreground` 的文件类型图标
                            // （`tab-bar-item.tsx:246-251`）；0.6.6 的 `component::IconName`
                            // 只派生 `Clone`（不是 `Copy`），从借用里取要 clone。
                            .child(
                                Icon::new(document.icon.clone())
                                    .size_4()
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(div().min_w_0().truncate().child(document.name.clone())),
                    )
                    .suffix(Self::tab_close_button(cx)),
            ),
            None => tabs,
        }
    }

    /// 标签栏左侧的前进/后退按钮。
    ///
    /// Windows（`tab-bar.tsx:633-660`）里它们是 `variant="ghost" size="icon-xs"`，
    /// `disabled={!canGoBack}`（jump list），`aria-label` 取 `tabs.goBack` /
    /// `tabs.goForward`，tooltip 取 `tabs.goBackShort` / `tabs.goForwardShort`
    /// （中文见 `i18n/locale.ts:7850-7855`）。
    /// **只有外观，没有历史栈**：探针里没有 jump list，所以两个按钮恒为禁用态
    /// （禁用 ghost = 透明底 + 灰图标，`button/button.rs:1273-1306`），
    /// 而不是"点了没反应"的假按钮。接上历史栈后改成
    /// `disabled(!can_go_back)` + 对应的 `on_click` 即可。
    fn nav_button(
        id: &'static str,
        icon: IconName,
        tooltip: &'static str,
        label: &'static str,
    ) -> Button {
        Button::new(id)
            .icon(icon)
            .ghost()
            .xsmall()
            .disabled(true)
            .tab_stop(false)
            .tooltip(tooltip)
            .accessibility_label(label)
    }

    /// 标签右侧的关闭按钮。
    ///
    /// Windows（`tab-bar-item.tsx:150-180`）里它是 ghost 图标按钮，`tooltip=tabs.close`
    /// （"关闭"），位置是标签内绝对定位的右上（`right-1`），默认 `opacity-0`、悬停才出现。
    /// 这里让它常显（gpui 侧还没有按父级悬停的条件），但**点击是有行为的**：
    /// 结果对齐 Windows 关掉最后一个标签的状态 —— 没有活动 buffer 就渲染
    /// `EmptyEditorState`（`pane-container.tsx:1100`）。
    fn tab_close_button(cx: &mut Context<Self>) -> Button {
        Button::new("editor-tab-close")
            .icon(IconName::Close)
            .ghost()
            .xsmall()
            .tab_stop(false)
            .tooltip("关闭")
            .accessibility_label("关闭")
            .on_click(cx.listener(|panel, _, window, cx| {
                panel.document = None;
                // 关掉文档后不要把上一个文件的内容留在编辑器里。
                panel
                    .editor
                    .update(cx, |state, cx| state.set_value("", window, cx));
                cx.notify();
            }))
    }

    /// 没有打开文件时的空状态，照 Windows
    /// `features/panes/components/empty-editor-state.tsx`：
    /// 居中一列 = 图标块（48px 的 `FileText` + 右下角 20px 放大镜）+ 标题 + 说明，
    /// 文案取**中文 i18n 原文**（`i18n/locale.ts:6150-6151`），不自编。
    fn render_empty_editor(cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex_1()
            .min_h_0()
            .child(
                Empty::new()
                    // Windows 的空状态没有边框；`Empty` 默认那圈虚线是给卡片里的空列表用的。
                    .border_color(cx.theme().transparent)
                    .header(
                        EmptyHeader::new()
                            .media(
                                // 图标块：`relative flex size-12 items-center justify-center`
                                // + `FileTextIcon size-10` + 右下角 `MagnifyingGlassIcon size-5`，
                                // 颜色 `text-subtle-foreground`。
                                EmptyMedia::new().child(
                                    div()
                                        .relative()
                                        .items_center()
                                        .justify_center()
                                        .w(px(48.))
                                        .h(px(48.))
                                        .child(
                                            Icon::new(IconName::FileText)
                                                .w(px(40.))
                                                .h(px(40.))
                                                .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(
                                            Icon::new(IconName::Search)
                                                .absolute()
                                                .right_0()
                                                .bottom_0()
                                                .w(px(20.))
                                                .h(px(20.))
                                                .text_color(cx.theme().muted_foreground),
                                        ),
                                ),
                            )
                            // Windows：`ui-text-base font-medium text-foreground`。
                            .title(EmptyTitle::new().text_base().child("选择文件以查看"))
                            // Windows：`ui-text-sm text-subtle-foreground`。
                            .description(
                                EmptyDescription::new().child("外部工具产生的更改会自动显示。"),
                            ),
                    ),
            )
            .into_any_element()
    }

    /// 项目树：折叠箭头 + 文件/目录图标 + 名字（文件行再加 Git 状态字母），每行挂右键菜单。
    ///
    /// 行内容照真机：`HStack(spacing: 6)`、缩进 `depth * 14`、树图标 16、缩进步长 14
    /// （`ProjectSidebarView.swift:426-444,469-490`）；图标只能取默认图标集里的字形
    /// （`File` / `FolderClosed` / `FolderOpen`），语言级字形不在清单里。
    fn render_files(&self) -> AnyElement {
        // 点击文件行 -> 把内容读进编辑区面板。渲染闭包拿到的是 `&mut App`
        // 而不是 `Context<Self>`，所以这里先取出编辑区的弱引用再移动进去。
        // 右键菜单里的"打开"要用同一个弱引用，两个闭包都要 `'static`，各拿一份。
        let open_target = self.open_target.clone();
        let menu_open_target = self.open_target.clone();
        // 渲染闭包要求 `'static`：Git 标记用 `Rc` 共享（不复制 `HashMap`），
        // 工作区根是小的 `PathBuf`。
        let git_marks = self.git_marks.clone();
        let root = self.root.clone();
        // 活动文件 = 编辑区当前打开的文档，整行用**强调底**而不是 macOS 的灰底
        // （`gpui/UI-MAP.md` §2.3「macOS 做得不好」第 2 条：`ProjectSidebarView.swift:446-451,492-499`
        // 把悬停与活动文件都设成 `subtleSelection`，两者在真机上不可区分）。
        let active = self
            .document
            .as_ref()
            .map(|document| document.relative_path.clone());

        Tree::new(&self.tree, move |index, entry, selected, _window, cx| {
            let is_folder = entry.is_folder();
            let expanded = entry.is_expanded();
            // 展开箭头只在文件夹行显示；文件行留一个**等宽占位**，同层的文件夹与文件图标才能左对齐
            // （macOS 的文件行是 `Color.clear.frame(width: 10)`，`ProjectSidebarView.swift:470`）。
            let caret: AnyElement = if is_folder {
                Icon::new(if expanded {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .size_3()
                .text_color(cx.theme().muted_foreground)
                .into_any_element()
            } else {
                div().w(px(12.)).flex_shrink_0().into_any_element()
            };
            let icon = if is_folder {
                if expanded {
                    IconName::FolderOpen
                } else {
                    IconName::FolderClosed
                }
            } else {
                IconName::File
            };

            // 树节点的 id 里带的就是相对路径（见 `files.rs` 的 `build_tree`）：
            // `file:<相对路径>` 是文件，`dir:<相对路径>` 是目录。
            let id = entry.item().id.clone();
            let relative = id.split_once(':').map_or(id.as_ref(), |(_, rest)| rest);
            let open_target = open_target.clone();

            // 字母只有文件行有；目录行只用子文件里最紧急的那个状态给名字染色。
            let mark = if is_folder {
                None
            } else {
                git_marks.files.get(relative).cloned()
            };
            let name_kind = if is_folder {
                git_marks.directories.get(relative).copied()
            } else {
                mark.as_ref().map(|mark| mark.kind)
            };
            // macOS：`foregroundStyle(gitStatusColor ?? LitheTheme.primaryText)`。
            let name_color = name_kind
                .map(|kind| git_mark_color(kind, cx))
                .unwrap_or_else(|| cx.theme().foreground);

            let row = h_flex()
                .w_full()
                .h(px(TREE_ROW_HEIGHT))
                .items_center()
                .gap_1()
                .pl(px(4. + entry.depth() as f32 * TREE_INDENT_STEP))
                .child(caret)
                .child(
                    Icon::new(icon)
                        .size_4()
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(name_color)
                        .child(entry.item().label.clone()),
                )
                // 字母贴行尾：macOS 是 `Spacer(minLength: 4)` 之后接徽标（`:479-485`）。
                .child(div().flex_1())
                .children(mark.map(|mark| {
                    div()
                        .flex_shrink_0()
                        .text_size(px(9.))
                        .font_weight(FontWeight::BOLD)
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_color(git_mark_color(mark.kind, cx))
                        .child(mark.letter)
                }));

            // 活动文件用强调底（`list_active`，与树自己的选中底同一个 token）。
            // **不能**在 `ListItem` 上写 `.selected(...)`：`component::tree::Tree` 的包装层
            // 紧接着用 `entry_state.is_selected()` 覆盖它
            // （`gpui-component-0.6.6/src/tree.rs:87-90`），所以这里自己画一层底。
            let row = if active.as_deref() == Some(relative) {
                row.bg(cx.theme().list_active).rounded(px(4.))
            } else {
                row
            };

            ListItem::new(SharedString::from(format!("tree-node-{index}")))
                .selected(selected)
                // 行高必须可控：树内部用 `uniform_list`（均匀行高），
                // 内容高于基准高度就会溢出到相邻行上。
                // `py_0()` 去掉 `ListItem` 默认的纵向内边距，24px 的行高才装得下内容。
                .py_0()
                .h(px(TREE_ROW_HEIGHT))
                .text_sm()
                // 必须不换行：折行会让实际行高超过基准，相邻行叠在一起。
                .whitespace_nowrap()
                .overflow_hidden()
                // **必须自己套一层 `h_flex()`**：`ListItem` 内部装 children 的是
                // 一个普通 `div`（块级），直接把多个 child 挂上去会**竖着排**
                // （箭头、图标、名字各占一行），行高随之失控。
                .child(row)
                .on_click(move |_event, window, cx| {
                    let Some(editor) = open_target.as_ref() else {
                        return;
                    };
                    let Some(relative) = id.as_ref().strip_prefix("file:") else {
                        // 目录行的展开/折叠由 `Tree` 自己处理。
                        return;
                    };
                    let relative = relative.to_string();
                    let _ = editor.update(cx, |editor, cx| {
                        editor.open_document(&relative, window, cx);
                    });
                })
        })
        .context_menu(move |_index, entry, menu, window, cx| {
            Self::tree_context_menu(entry, menu, window, cx, menu_open_target.clone(), &root)
        })
        .size_full()
        .into_any_element()
    }

    /// 一行的右键菜单，项、顺序与文案照 macOS `ProjectSidebarView.swift:525-696`
    /// 与 `macos/Resources/zh-Hans.lproj/Localizable.strings` 的中文原文。
    ///
    /// **真生效的动作只有三个**：`打开`、`复制路径` / `复制相对路径`（写剪贴板）、
    /// `在文件资源管理器中显示`（起 `explorer.exe`）。新建 / 重命名 / 删除在 macOS 走的是
    /// 项目文件命令（`WorkspaceFeatureModel.requestCreateFile/requestRenameProjectItem/
    /// requestDeleteProjectItem`，`WorkspaceFeatureModel.swift:476-491,619-628`），
    /// 这些命令**不在 Core 契约表里**，本步**待 Core 文件写命令接线**，所以渲染成
    /// `disabled(true)` 的占位，而不是"点了没反应"的假按钮。
    fn tree_context_menu(
        entry: &TreeEntry,
        menu: PopupMenu,
        window: &mut Window,
        cx: &mut Context<TreeState>,
        open_target: Option<WeakEntity<ShellPanel>>,
        root: &Path,
    ) -> PopupMenu {
        let is_folder = entry.is_folder();
        // 树节点的 id 里带的就是相对路径（见 `files.rs` 的 `build_tree`）。
        let id = entry.item().id.clone();
        let relative = id
            .split_once(':')
            .map_or(id.as_ref(), |(_, rest)| rest)
            .to_string();
        let absolute = absolute_path(root, &relative);

        // 删除：`PopupMenuItem` 没有 destructive 角色，危险色只能自绘
        // （`gpui/UI-MAP.md` §1.3「危险菜单项」；主题 token `danger_foreground`）。
        // 文案 `移到废纸篓` 是 macOS `Move to Trash` 的中文原文。
        // 代价：`ElementItem` 没有 `label`，无障碍名会缺（`menu/popup_menu.rs:272-279`），
        // 换成自带 label 的 `Item` 才有，但那样拿不到危险色。
        let trash = PopupMenuItem::element(|_window, cx| {
            div()
                .text_color(cx.theme().danger_foreground)
                .child("移到废纸篓")
        })
        // macOS 用 SF Symbol `trash`，默认图标集里没有，取最接近的 `delete`
        // （`gpui-kit-assets-0.6.6/default-icons.txt`）。
        .icon(IconName::Delete)
        .disabled(true);
        let rename = PopupMenuItem::new("重命名…").disabled(true);

        // `Copy Path`（`ProjectSidebarView.swift:688-690`）：macOS 复制绝对路径并弹
        // "Copied path"（`AppModel.swift:1093-1098`）；这里只写剪贴板（没有通知层）。
        // 文案与路径都在起 `explorer.exe` 之前算好：那个闭包会把 `absolute` 移走。
        let absolute_text = absolute.display().to_string();
        let copy_path = PopupMenuItem::new("复制路径")
            .icon(IconName::Copy)
            .on_click(move |_event, _window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(absolute_text.clone()));
            });

        // `Show in Finder`：macOS 走 `platformUI.revealInFileBrowser`
        // （`AppModel.swift:1089-1091`，即 Finder 的 `activateFileViewerSelecting`）；
        // Windows 的等价动作是 `explorer.exe /select,<路径>`。**真生效**。
        // 路径用平台分隔符（见 [`absolute_path`]）；失败时静默退出 —— 探针没有通知层
        // （`gpui/UI-MAP.md` §1.3），也不该为一次系统调用失败弹窗。
        let reveal = PopupMenuItem::new(REVEAL_IN_FILE_BROWSER_LABEL)
            .icon(IconName::Folder)
            .on_click(move |_event, _window, _cx| {
                let _ = std::process::Command::new("explorer.exe")
                    .arg(format!("/select,{}", absolute.display()))
                    .spawn();
            });

        // `Copy Relative Path`（`:691-693`）：相对工作区根、`/` 分隔
        // （`shared/contracts/rust-core-api.md:283`）。
        let copy_relative_path = PopupMenuItem::new("复制相对路径").on_click({
            let text = relative.clone();
            move |_event, _window, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
            }
        });

        if is_folder {
            // 目录行第一项是 `New` 子菜单（`:529-536`）。子菜单标题 `New` 在
            // `zh-Hans.lproj/Localizable.strings` 里没有条目，用两个子项共有的前缀"新建"。
            let create = PopupMenu::build(window, cx, |menu, _window, _cx| {
                menu.item(
                    PopupMenuItem::new("新建文件…")
                        .icon(IconName::FileText)
                        .disabled(true),
                )
                .item(
                    PopupMenuItem::new("新建目录…")
                        .icon(IconName::Folder)
                        .disabled(true),
                )
            });

            menu.item(PopupMenuItem::submenu("新建", create))
                .separator()
                .item(rename)
                .item(trash)
                .separator()
                .item(reveal)
                .item(copy_path)
                .item(copy_relative_path)
        } else {
            // 文件行第一项是 `Open`（`:657-659`）：真生效，走和左键单击同一条路。
            let open = PopupMenuItem::new("打开").on_click(move |_event, window, cx| {
                let Some(editor) = open_target.as_ref() else {
                    return;
                };
                let _ = editor.update(cx, |editor, cx| {
                    editor.open_document(&relative, window, cx);
                });
            });

            menu.item(open)
                .separator()
                .item(rename)
                .item(trash)
                .separator()
                .item(reveal)
                .item(copy_path)
                .item(copy_relative_path)
        }
    }
}

impl EventEmitter<PanelEvent> for ShellPanel {}

impl Focusable for ShellPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl BasePanel for ShellPanel {
    fn panel_name(&self) -> &'static str {
        self.name
    }
}

impl Panel for ShellPanel {
    fn title(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // 真机的资源管理器头部**只有两个图标**（搜索 + 视图设置），左边的面板名不显示；
        // 文件名数量显示在底部状态栏右侧（见 `workspace.rs` 的 `StatusBar`）。
        // 其它面板在 Dock 里需要有名字，否则标签无法辨认。
        if matches!(self.kind, PanelKind::Files) {
            return h_flex()
                .gap_2()
                .child(Icon::new(IconName::Search))
                .child(Icon::new(IconName::LayoutDashboard))
                .into_any_element();
        }

        h_flex().gap_2().child(self.title.clone()).into_any_element()
    }

    /// `false`：编辑器面板自己画标签栏（真机编辑区顶部只有文件标签，没有工具窗标题），
    /// Dock 不要再画一条。
    fn title_bar(&self, _: &App) -> bool {
        !matches!(self.kind, PanelKind::Editor)
    }

    /// `false`：每个面板都自己控制内边距（树要贴边、编辑器要占满），
    /// Dock 不要在内容外再加一层留白。
    fn inner_padding(&self, _: &App) -> bool {
        false
    }
}

impl Render for ShellPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.kind {
            // 编辑区自带内部标签栏与空状态，见 `render_editor`。
            PanelKind::Editor => self.render_editor(cx),

            // 项目树：`Tree` 自带折叠、选中与滚动，行内容与右键菜单见 `render_files`。
            PanelKind::Files => self.render_files(),

            PanelKind::Outline => v_flex()
                .size_full()
                .p_3()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(SharedString::from("大纲（占位：验证右侧面板）"))
                .into_any_element(),

            // 命令面板：分组 + 可搜索，对应 Windows 的命令面板。
            PanelKind::Command => Command::new(&self.palette)
                .searchable(true)
                .group(
                    CommandGroup::new().label("文件").items([
                        CommandItem::new().label("打开文件…"),
                        CommandItem::new().label("保存"),
                        CommandItem::new().label("关闭编辑器"),
                    ]),
                )
                .group(
                    CommandGroup::new().label("导航").items([
                        CommandItem::new().label("转到文件…"),
                        CommandItem::new().label("项目内搜索"),
                        CommandItem::new().label("转到符号…"),
                    ]),
                )
                .group(
                    CommandGroup::new().label("Git").items([
                        CommandItem::new().label("提交更改"),
                        CommandItem::new().label("查看历史"),
                    ]),
                )
                .size_full()
                .into_any_element(),

            // 项目统计：用真实快照数据填充 `DataTable`。
            // `DataTable` 实现的是 `Sizable`（`.small()/.large()`），不是 `Styled`，
            // 所以要靠外层 `div` 给它尺寸。
            PanelKind::Stats => div()
                .size_full()
                .child(DataTable::new(&self.stats))
                .into_any_element(),
        }
    }
}
