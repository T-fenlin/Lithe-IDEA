//! `.lithe/session.local.json`：会话状态（**项目本机层，可丢弃**）。
//!
//! ## 这一版只存"今天真的有宿主"的四类字段
//!
//! 设计 Note 第八节把"会话恢复"整块列进了 `.lithe/session.local.json`，但那一节列的是
//! **目标**形状（打开的文件、光标、滚动、展开节点、面板尺寸、断点）。落地前先按今天的
//! 实现核对了一遍，只有下面这些值存在**唯一的持有者**：
//!
//! | 字段 | 今天的持有者 | 备注 |
//! | --- | --- | --- |
//! | `openFiles` | `EditorPane::buffers`（`gpui/crates/editor`） | 顺序 = 标签栏顺序 |
//! | `activeFile` | `EditorPane::active` | 存**下标**，见下面的"为什么是下标" |
//! | `leftSidebarVisible` | `ShellWorkspace::left_sidebar_visible` | B2 的侧栏收起态 |
//! | `topActivityView` / `bottomKind` / `bottomVisible` / `rightView` / `rightVisible` | `ShellWorkspace` 的五个字段 | 当前视图 id + 可见性 |
//!
//! **没有落点的**（因此本模块**不建模**，也不给它们编字段）：光标与滚动位置（只在
//! `EditorState` 内部，没有读出口）、展开的树节点（资源管理器不记录展开态）、面板 / 分栏
//! 尺寸（gpui 侧没有可拖动分隔条那套状态）、断点与监视表达式（gpui 侧没有调试器）。
//! 给没有消费方的字段建模只会造出假契约：写进去、没人读、还得为它写测试。
//!
//! ## 为什么 `openFiles` 存工作区相对路径而不是绝对路径
//!
//! 与本目录其余成员同一条硬规则（见 [`super::paths::workspace_relative`] 的文档）：
//! 会话文件虽然是本机层、不跨机器共享，但它同样**不许出现机器路径**——同一份文档在
//! "根被移动 / 挂载点变了"之后仍然要能用，而且绝对路径会让这个文件在日志里泄露用户目录。
//! 不在工作区根之下的文件（仓库外的文件）**直接不存**：它不是"这个项目里的会话状态"。
//!
//! ## 为什么 `activeFile` 是下标
//!
//! 存文件路径会与 `openFiles` 重复一份数据，两份一旦不一致就没有仲裁者。存下标则有一个
//! 明确的三步判据（见 [`session_from_paths`]）：先按"根内的文件"过滤 → 再按上限截断 →
//! **最后**算下标。写进去的下标一定指向 `openFiles` 里真实存在的那一项。
//!
//! ## 容错：坏路径在**恢复阶段**跳过，不在读取阶段报错
//!
//! 读取（[`load_session`]）只做一件事：把文档解析成 [`SessionState`]。文件被删 / 改名 /
//! 被移到工作区外都是**恢复阶段**才知道的事（要查磁盘），所以由
//! [`crate::workspace_config::resolve_session_files`] 逐条判定并给出诊断；读取阶段对
//! "这条路径还存不存在"一无所知，也不该装作知道。
//!
//! 坏 JSON / 非对象退化成"没有会话"（[`SessionState::default`]）并留一条诊断 —— 这也是
//! [`crate::document::parse`] 的既有语义，本模块不另外实现一份。
//!
//! ## 上界
//!
//! 见 [`MAX_OPEN_FILES`]：截断规则与理由写在那个常量上。

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::document;
use crate::workspace_config::paths::{self, WorkspaceConfigPaths};
use crate::workspace_config::sharing;

/// `session.local.json` 当前的文档版本。
pub const SESSION_DOCUMENT_VERSION: u32 = 1;

/// 一次会话最多记住多少个打开的文件（超出的按标签顺序截断）。
///
/// **40 的来历与理由**：它对应"一次会话里打开的标签数"这一档量级 —— 40 之后的标签在实践中
/// 基本都是"顺手点开、当前用不到"的东西。给上界的目的是**让这份文档有界**（一个长会话
/// 不该让它无限增长），不是省字节：40 条相对路径最坏也就几 KB，落盘代价可以忽略。
///
/// 上界的截断规则（顺序不能改，判据见 [`session_from_paths`]）：
///
/// 1. 按标签顺序留下**前 `MAX_OPEN_FILES - 1` 个**；
/// 2. 当前活动文件若不在那一段里，**追加到最后**。
///
/// 于是文档里的条数是 `MAX_OPEN_FILES - 1`（活动文件本来就在前一段里）或
/// `MAX_OPEN_FILES`（活动文件被追加回来）—— 两种都不会超过 [`MAX_OPEN_FILES`]，
/// 而且当前文件**任何情况下都不会被丢掉**：否则"只开了一个第 40 号标签"的会话恢复之后
/// 当前文件会消失，而那正是会话恢复最该保住的那一项。
pub const MAX_OPEN_FILES: usize = 40;

/// 会话文档：`openFiles` 是工作区相对路径（`/` 分隔），`activeFile` 是它在其中的下标。
///
/// 字段语义见模块文档。**不要**给字段加 `skip_serializing_if`：那会让该键退出
/// [`crate::document::known_keys`] 派生出来的键集，于是它在"另一个键坏掉"时被当成未知键
/// 读不回来（同一个陷阱在 [`super::project::ProjectManifest`] 的文档里记过一次）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionState {
    /// 打开的文件，**工作区相对路径 + `/` 分隔**，顺序 = 标签顺序。
    pub open_files: Vec<String>,
    /// 当前激活文件在 [`Self::open_files`] 里的下标；`None` = 没有激活文件。
    pub active_file: Option<usize>,
    /// 左侧栏（活动栏 + 面板整块）是否可见。
    pub left_sidebar_visible: Option<bool>,
    /// 左活动栏顶部组选中的视图下标（资源管理器 / 更改 / 搜索）。
    pub top_activity_view: Option<usize>,
    /// 底部工具窗当前页签的稳定 id（`terminal` / `git` / `run` / `diagnostics`）。
    pub bottom_kind: Option<String>,
    /// 底部工具窗是否可见。
    pub bottom_visible: Option<bool>,
    /// 右工具窗当前视图的稳定 id（`extensions` / `notifications` / `maven` / `spring`）。
    pub right_view: Option<String>,
    /// 右工具窗是否可见。
    pub right_visible: Option<bool>,
}

/// 一次读取的结论。
#[derive(Debug, Clone, PartialEq)]
pub struct SessionLoaded {
    /// 解析出来的会话状态。任何失败路径都给可用的值（最差是全默认）。
    pub session: SessionState,
    /// 文件里的原始对象；写回时用它保留未知键。
    pub previous: Option<Value>,
    /// 文件声明的版本高于本程序支持：能读，**不要覆盖**（与其余文档同一条口径）。
    pub read_only: bool,
    /// 文件是否真的存在且被读到了。
    pub file_existed: bool,
    /// 逐键诊断，调用方负责打印。
    pub diagnostics: Vec<String>,
}

/// 一次落盘的结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSaved {
    /// 写入的字节数。
    pub bytes: usize,
    /// 写之前"确保不共享"的结论（非 Git 项目是 [`sharing::EnsureExcluded::NotARepository`]）。
    pub exclusion: sharing::EnsureExcluded,
}

/// 会话文档的失败。
#[derive(Debug)]
pub enum SessionError {
    /// 写 `.lithe/` 之前"确保本机排除"失败（非 Git 仓库那一类已经被静默跳过）。
    Exclude(sharing::SharingError),
    /// 读写会话文件失败。
    Io(io::Error),
    /// 生成文档 JSON 失败。
    Json(serde_json::Error),
    /// 磁盘上的文档声明了比本程序更高的版本：拒绝写入，别把它降级覆盖。
    ReadOnly {
        /// 被拒绝的文件（用于诊断）。
        path: String,
        /// 文件里声明的版本。
        declared: u64,
        /// 本程序支持的版本。
        supported: u32,
    },
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Exclude(error) => write!(formatter, "确保本机排除失败：{error}"),
            Self::Io(error) => write!(formatter, "会话文件读写失败：{error}"),
            Self::Json(error) => write!(formatter, "会话 JSON 生成失败：{error}"),
            Self::ReadOnly {
                path,
                declared,
                supported,
            } => write!(
                formatter,
                "文档版本 {declared} 高于本程序支持的 {supported}，拒绝覆盖：{path}"
            ),
        }
    }
}

impl std::error::Error for SessionError {}

impl From<io::Error> for SessionError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for SessionError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// 读会话（**不创建文件、不 panic**）。
pub fn session_path(root: &Path) -> PathBuf {
    WorkspaceConfigPaths::new(root).session_local()
}

/// 读会话（**不创建文件、不 panic**）。
///
/// 文件不存在 → [`SessionState::default`] 且**不落盘**（"读取不得创建文件"是设计 Note
/// 第六节的硬要求）。
pub fn load_session(root: &Path) -> SessionLoaded {
    let path = session_path(root);
    match document::read_document_text(&path) {
        Ok(Some(text)) => {
            let parsed = document::parse::<SessionState>(&text, SESSION_DOCUMENT_VERSION);
            SessionLoaded {
                session: parsed.value,
                previous: parsed.previous,
                read_only: parsed.read_only,
                file_existed: true,
                diagnostics: parsed.diagnostics,
            }
        }
        Ok(None) => SessionLoaded {
            session: SessionState::default(),
            previous: None,
            read_only: false,
            file_existed: false,
            diagnostics: Vec::new(),
        },
        Err(error) => SessionLoaded {
            session: SessionState::default(),
            previous: None,
            read_only: false,
            file_existed: false,
            diagnostics: vec![format!("read_failed error={error}")],
        },
    }
}

/// 写会话：**先确保 `.lithe/` 不被共享，再原子落盘**。
///
/// 顺序由本函数保证（与 [`super::project::save_project_manifest`] 同一条口径）：
/// 调用方不需要记得先调守卫。未知键靠 [`document::previous_object`] 从磁盘现读，
/// 所以这里不需要调用方传上一版文档。
///
/// ⚠️ **写完**才检查版本：版本过新的文件在被覆盖之前就要拦住，所以判据放在写之前
/// （先读一次 previous，同一次读结果同时用于判只读与保留未知键）。
pub fn save_session(root: &Path, session: &SessionState) -> Result<SessionSaved, SessionError> {
    let path = session_path(root);
    let previous = document::previous_object(&path);
    if let Some(Value::Object(object)) = previous.as_ref() {
        let declared = document::declared_version(object);
        let supported = u64::from(SESSION_DOCUMENT_VERSION);
        if declared.is_some_and(|version| version > supported) {
            return Err(SessionError::ReadOnly {
                path: path.display().to_string(),
                declared: declared.unwrap_or_default(),
                supported: SESSION_DOCUMENT_VERSION,
            });
        }
    }

    let exclusion =
        sharing::ensure_project_dir_excluded(root).map_err(SessionError::Exclude)?;
    let bytes = document::save_document(
        &path,
        session,
        previous.as_ref(),
        SESSION_DOCUMENT_VERSION,
    )?;
    // 第二道闸（`.lithe/.gitignore`）跟着维护：与清单同一条理由 —— 它要生效的时刻正是
    // 第一道闸被"共享此项目的配置"移除的那一刻，不能等到会话真的被共享时才补。
    let _ = sharing::ensure_local_ignore_file(root);
    Ok(SessionSaved { bytes, exclusion })
}

/// 恢复阶段对一条会话路径的判定结论。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionFileOutcome {
    /// 存下来了：绝对路径可用（在磁盘上）。
    Restored(PathBuf),
    /// 没有存下来：原因（用于诊断），`String` 是那条原始相对路径。
    Skipped {
        /// 会话文档里那条原始路径。
        relative: String,
        /// 跳过的原因（稳定 token：`outside_root` / `missing`）。
        reason: &'static str,
    },
}

/// 把会话里记的**工作区相对路径**逐条还原成绝对路径，**只留下磁盘上真的存在的**。
///
/// 这是设计里"文件被删 / 改名 / 在仓库外时恢复阶段跳过它，不报错、不崩溃"那一条的落点：
/// 判定逐条给出（[`SessionFileOutcome`]），调用方打一行汇总诊断即可，不需要自己再判一遍。
///
/// 三条规则：
///
/// - 不在工作区根之下（`../` 逃逸、盘符路径）→ 跳过，`outside_root`；
/// - 在根之下但磁盘上不存在（被删 / 改名 / 目录被移走）→ 跳过，`missing`；
/// - 其余 → [`SessionFileOutcome::Restored`]，**保持文档里的顺序**。
pub fn resolve_session_files(root: &Path, session: &SessionState) -> Vec<SessionFileOutcome> {
    session
        .open_files
        .iter()
        .map(|relative| match resolve_one(root, relative) {
            Some(path) => SessionFileOutcome::Restored(path),
            None if is_plain_relative(relative) => SessionFileOutcome::Skipped {
                relative: relative.clone(),
                reason: "missing",
            },
            None => SessionFileOutcome::Skipped {
                relative: relative.clone(),
                reason: "outside_root",
            },
        })
        .collect()
}

/// 一条相对路径的解析：不在根之下或磁盘上没有 → `None`。
fn resolve_one(root: &Path, relative: &str) -> Option<PathBuf> {
    if !is_plain_relative(relative) {
        return None;
    }
    // 文档里是 `/` 分隔（共享契约口径）；Windows 上 `Path::join` 两种分隔都认，
    // 但显式替换一次让判据在任何平台都一致。
    let path = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    path.is_file().then_some(path)
}

/// 这条路径是不是"普通的、在根之内的相对路径"。
///
/// 拒绝三类：空串、绝对路径（盘符 / 前导分隔符）、含 `..` 的路径（逃出根）。
/// 单独一个 `..` 也会被挡掉 —— 会话文档是**可丢弃的**，容忍不了"读到一条路径就跑到根外面去"。
fn is_plain_relative(relative: &str) -> bool {
    let normalized = relative.replace('\\', "/");
    if normalized.is_empty() {
        return false;
    }
    if normalized.starts_with('/') {
        return false;
    }
    // `C:foo` 这种"有盘符但没分隔符"的写法同样是逃逸，所以看第二个字符。
    let bytes = normalized.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        return false;
    }
    !normalized.split('/').any(|segment| segment == "..")
}

/// 从"绝对路径 + 界面状态"构造要落盘的会话文档：过滤 → 去重 → 截断 → 算下标。
///
/// ## 三步判据（顺序不能改）
///
/// 1. **过滤**：只留工作区根之内、且磁盘上存在的文件；
/// 2. **去重**：同一路径只留第一次出现的那条（否则下标失去意义）；
/// 3. **截断 + 算下标**：见 [`MAX_OPEN_FILES`] 的文档，截断之后**再**算活动文件的下标 ——
///    反过来的话，被截掉的那些项会把下标顶到列表之外。
///
/// `top_activity_view` 等界面值由调用方直接给（它们在 `ShellWorkspace` 上是私有字段，
/// 只有外壳自己读得到）。`bottom_kind` / `right_view` 传稳定 id 字符串（`None` = 不存）。
///
/// 返回的文档保证：`active_file` 一定指向 `open_files` 里的一项（或两组都是空 / `None`）。
pub fn session_from_paths(
    root: &Path,
    open_paths: &[PathBuf],
    active_path: Option<&Path>,
    ui: SessionUiState,
) -> SessionState {
    // 过滤 + 去重一步做完：`editor` 侧同一路径不会开两次（同路径只切活动标签），但本函数是
    // 公开的纯函数，不能依赖调用方保证这一点。
    let mut kept: Vec<(String, PathBuf)> = Vec::new();
    for path in open_paths {
        if !path.is_file() {
            continue;
        }
        let Some(relative) = paths::workspace_relative(root, path) else {
            // 根之外的文件不是"这个项目里的会话状态"，不存（本机层同样不许出现机器路径）。
            continue;
        };
        if kept.iter().any(|(_, existing)| existing == path) {
            continue;
        }
        kept.push((relative, path.clone()));
    }

    // 截断：先取前 N-1 项，活动文件无条件保留（见 `MAX_OPEN_FILES`）。
    // ⚠️ 被过滤掉的活动文件（删了 / 在根外）**不补位**：它本来就不该出现在文档里。
    let active_relative = active_path
        .filter(|active| kept.iter().any(|(_, path)| path == active))
        .and_then(|active| paths::workspace_relative(root, active));

    let mut open_files: Vec<String> = if kept.len() <= MAX_OPEN_FILES {
        kept.iter().map(|(relative, _)| relative.clone()).collect()
    } else {
        let mut bounded: Vec<String> = kept
            .iter()
            .take(MAX_OPEN_FILES - 1)
            .map(|(relative, _)| relative.clone())
            .collect();
        if let Some(active) = active_relative.as_ref() {
            if !bounded.contains(active) {
                bounded.push(active.clone());
            }
        }
        bounded
    };

    // 下标在**截断之后**的列表里算：截断前算的话，被截掉的那些项会把下标顶到列表之外。
    let active_file = active_relative
        .and_then(|active| open_files.iter().position(|relative| *relative == active));
    open_files.shrink_to_fit();

    SessionState {
        open_files,
        active_file,
        left_sidebar_visible: ui.left_sidebar_visible,
        top_activity_view: ui.top_activity_view,
        bottom_kind: ui.bottom_kind,
        bottom_visible: ui.bottom_visible,
        right_view: ui.right_view,
        right_visible: ui.right_visible,
    }
}

/// 外壳提供的界面状态（会话文档里除打开文件之外的那几项）。
///
/// 单独一个结构体而不是把六个参数摊在 [`session_from_paths`] 的签名上：调用点只有一处，
/// 但六个裸参数读起来无法分辨谁是谁（`Some(true)` 出现两次就该有人搞反）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionUiState {
    /// 左侧栏（活动栏 + 面板整块）是否可见。
    pub left_sidebar_visible: Option<bool>,
    /// 左活动栏顶部组选中的视图下标。
    pub top_activity_view: Option<usize>,
    /// 底部工具窗当前页签的稳定 id。
    pub bottom_kind: Option<String>,
    /// 底部工具窗是否可见。
    pub bottom_visible: Option<bool>,
    /// 右工具窗当前视图的稳定 id。
    pub right_view: Option<String>,
    /// 右工具窗是否可见。
    pub right_visible: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_config::test_repo::TempRepo;

    /// 一个**不含 Git 仓库**的临时目录：读路径与纯函数用它（不触发排除守卫）。
    ///
    /// 任何调用 `save_session` 的用例必须用 [`TempRepo`] —— 理由见 `test_repo` 的模块文档
    /// （`TEMP` 落在仓库内时守卫会经 `git rev-parse` 找到外层真实仓库）。
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lithe-session-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        dir
    }

    /// 写出一个 fixture 项目：给定相对路径各建一个文件。
    fn write_files(root: &Path, relatives: &[&str]) -> Vec<PathBuf> {
        relatives
            .iter()
            .map(|relative| {
                let path = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
                std::fs::create_dir_all(path.parent().expect("有父目录")).expect("建父目录");
                std::fs::write(&path, "// fixture\n").expect("写 fixture 文件");
                path
            })
            .collect()
    }

    fn ui_hidden() -> SessionUiState {
        SessionUiState {
            left_sidebar_visible: Some(false),
            top_activity_view: Some(1),
            bottom_kind: Some("terminal".to_string()),
            bottom_visible: Some(false),
            right_view: Some("maven".to_string()),
            right_visible: Some(false),
        }
    }

    /// 读不存在的会话：给默认值、**不创建文件**。
    #[test]
    fn loading_a_missing_session_does_not_create_it() {
        let dir = temp_dir("missing");
        let path = session_path(&dir);

        let loaded = load_session(&dir);
        assert_eq!(loaded.session, SessionState::default());
        assert!(!loaded.file_existed);
        assert!(loaded.diagnostics.is_empty());
        assert!(!path.exists(), "读取不得创建文件");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 坏 JSON 退化成"没有会话" + 一条诊断，**不 panic**。
    #[test]
    fn broken_json_degrades_to_no_session_with_a_diagnostic() {
        let dir = temp_dir("broken");
        let path = session_path(&dir);
        std::fs::create_dir_all(path.parent().expect("有父目录")).expect("建配置目录");
        std::fs::write(&path, "{ this is not json").expect("写坏文件");

        let loaded = load_session(&dir);
        assert_eq!(loaded.session, SessionState::default());
        assert!(loaded.file_existed, "文件读到了，只是内容坏");
        assert!(
            loaded
                .diagnostics
                .iter()
                .any(|line| line.contains("invalid_json")),
            "{:?}",
            loaded.diagnostics
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 落盘：先让"默认不共享"落到磁盘上，再写出带 `version` 的会话；未知键原样保留。
    #[test]
    fn saving_a_session_lands_under_the_excluded_directory() {
        // `tag` 必须**全局唯一**：`TempRepo::new` 会先删同名目录，两个用例共用一个 tag 时
        // 并行跑会互相删掉对方的 `.git`（本轮实测：`save` 与 project.rs 的用例撞过一次）。
        let repo = TempRepo::new("session-save");
        repo.write("alpha.txt", "a\n");
        let session = session_from_paths(
            repo.root(),
            &[repo.root().join("alpha.txt")],
            Some(&repo.root().join("alpha.txt")),
            ui_hidden(),
        );

        let saved = save_session(repo.root(), &session).expect("写会话失败");
        assert!(saved.bytes > 0);
        assert!(!repo.status().contains(".lithe"), "{}", repo.status());

        let text = repo.read(".lithe/session.local.json");
        assert!(text.contains("\"version\": 1"), "{text}");
        assert!(text.contains("\"openFiles\""), "{text}");
        assert!(text.contains("\"alpha.txt\""), "{text}");

        // 另一份生产者写的字段必须逐字保留（round-trip）。
        let mut object: Value = serde_json::from_str(&text).expect("应是 JSON");
        object
            .as_object_mut()
            .expect("应是对象")
            .insert("futureField".to_string(), serde_json::json!({ "deep": [1, 2] }));
        std::fs::write(
            crate::workspace_config::session::session_path(repo.root()),
            serde_json::to_string_pretty(&object).expect("序列化"),
        )
        .expect("写回");

        save_session(repo.root(), &session).expect("第二次写会话失败");
        let text = repo.read(".lithe/session.local.json");
        assert!(
            text.contains("futureField"),
            "未知键必须原样保留，否则下一次落盘会吃掉别的生产者写的东西：{text}"
        );
    }

    /// 文档版本比本程序新 → **拒绝覆盖**。
    #[test]
    fn a_newer_document_version_is_refused_instead_of_downgraded() {
        let repo = TempRepo::new("newer");
        repo.write(".lithe/session.local.json", r#"{"version":99,"openFiles":["x.txt"]}"#);

        let error = save_session(repo.root(), &SessionState::default())
            .expect_err("更高版本必须被拒绝");
        assert!(matches!(error, SessionError::ReadOnly { .. }), "{error:?}");
        assert_eq!(
            repo.read(".lithe/session.local.json"),
            r#"{"version":99,"openFiles":["x.txt"]}"#,
            "被拒绝的写入不得改动文件"
        );
        assert!(
            !repo.exclude_lines().iter().any(|line| line == ".lithe/"),
            "被拒绝的写入不该动排除文件"
        );
    }

    /// 恢复阶段跳过：文件被删 / 在根之外，其余照常还原，顺序保持。
    #[test]
    fn resolving_files_skips_missing_and_outside_paths_in_order() {
        let dir = temp_dir("resolve");
        write_files(&dir, &["alpha.txt", "gamma.txt"]);
        let session = SessionState {
            open_files: vec![
                "alpha.txt".to_string(),
                "deleted.txt".to_string(),
                "gamma.txt".to_string(),
                "../outside.txt".to_string(),
                "C:/windows/win.ini".to_string(),
                "".to_string(),
            ],
            ..SessionState::default()
        };

        let resolved = resolve_session_files(&dir, &session);
        assert_eq!(resolved.len(), 6, "逐条都要给出结论");
        assert_eq!(
            resolved[0],
            SessionFileOutcome::Restored(dir.join("alpha.txt"))
        );
        assert_eq!(
            resolved[1],
            SessionFileOutcome::Skipped {
                relative: "deleted.txt".to_string(),
                reason: "missing",
            }
        );
        assert_eq!(
            resolved[2],
            SessionFileOutcome::Restored(dir.join("gamma.txt"))
        );
        for outcome in &resolved[3..] {
            assert!(
                matches!(
                    outcome,
                    SessionFileOutcome::Skipped {
                        reason: "outside_root",
                        ..
                    }
                ),
                "逃出根 / 绝对路径必须按 outside_root 跳过：{outcome:?}"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 根之外的打开文件**不写进文档**（本机层同样不许出现机器路径）。
    #[test]
    fn files_outside_the_root_are_not_recorded() {
        let dir = temp_dir("outside");
        let root = dir.join("project");
        std::fs::create_dir_all(&root).expect("建工作区根");
        let inside = write_files(&root, &["inside.txt"]);
        let outside_dir = temp_dir("outside-file");
        let outside = write_files(&outside_dir, &["outside.txt"]);

        let mut paths = inside.clone();
        paths.extend(outside.iter().cloned());
        let session = session_from_paths(&root, &paths, Some(&inside[0]), ui_hidden());

        assert_eq!(session.open_files, vec!["inside.txt".to_string()]);
        assert_eq!(session.active_file, Some(0));

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&outside_dir);
    }

    /// 被删的文件当时就不写进文档（写的是"现在能存什么"）。
    #[test]
    fn missing_files_are_not_recorded() {
        let dir = temp_dir("missing-file");
        let present = write_files(&dir, &["present.txt"]);
        let gone = dir.join("gone.txt");

        let session = session_from_paths(
            &dir,
            &[present[0].clone(), gone.clone()],
            Some(&gone),
            SessionUiState::default(),
        );
        assert_eq!(session.open_files, vec!["present.txt".to_string()]);
        assert_eq!(
            session.active_file, None,
            "活动文件被过滤掉时不能留下指向别人的下标"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 上界：超出的按标签顺序截断，**当前活动文件无条件保留**。
    #[test]
    fn the_open_file_list_is_bounded_and_keeps_the_active_file() {
        let dir = temp_dir("bounded");
        let relatives: Vec<String> = (0..(MAX_OPEN_FILES + 5))
            .map(|index| format!("file-{index:03}.txt"))
            .collect();
        let borrowed: Vec<&str> = relatives.iter().map(String::as_str).collect();
        let paths = write_files(&dir, &borrowed);

        // ① 活动文件在前 N-1 项之内：截断后正好 N-1 项（活动文件已经在那一段里，不重复追加）。
        let session = session_from_paths(&dir, &paths, Some(&paths[1]), ui_hidden());
        assert_eq!(session.open_files.len(), MAX_OPEN_FILES - 1);
        assert_eq!(session.active_file, Some(1));
        assert_eq!(session.open_files[0], "file-000.txt");
        let expected_tail = format!("file-{:03}.txt", MAX_OPEN_FILES - 2);
        assert_eq!(
            session.open_files.last().map(String::as_str),
            Some(expected_tail.as_str())
        );

        // ② 活动文件在窗口之外：它被追加回来 → 正好 N 项，且下标指向它自己。
        let last = paths.len() - 1;
        let session = session_from_paths(&dir, &paths, Some(&paths[last]), ui_hidden());
        assert_eq!(session.open_files.len(), MAX_OPEN_FILES);
        assert_eq!(
            session.open_files.last().map(String::as_str),
            Some(relatives[last].as_str()),
            "活动文件必须无条件保留"
        );
        assert_eq!(session.active_file, Some(MAX_OPEN_FILES - 1));
        let active_index = session.active_file.expect("有活动文件");
        assert_eq!(
            session.open_files[active_index], relatives[last],
            "下标必须指向活动文件自己"
        );

        // ③ 同一路径重复出现只留一条（下标才有意义）。
        let duplicated = vec![paths[0].clone(), paths[0].clone(), paths[1].clone()];
        let session = session_from_paths(&dir, &duplicated, Some(&paths[1]), ui_hidden());
        assert_eq!(
            session.open_files,
            vec!["file-000.txt".to_string(), "file-001.txt".to_string()]
        );
        assert_eq!(session.active_file, Some(1));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 落盘 → 读回的往返：界面七值逐条回来。
    #[test]
    fn a_saved_session_round_trips_through_disk() {
        let repo = TempRepo::new("round-trip");
        repo.write("src/main.java", "class Main {}\n");
        repo.write("README.md", "# readme\n");
        let paths = vec![
            repo.root().join("src").join("main.java"),
            repo.root().join("README.md"),
        ];
        let written = session_from_paths(repo.root(), &paths, Some(&paths[1]), ui_hidden());
        save_session(repo.root(), &written).expect("写会话失败");

        let loaded = load_session(repo.root());
        assert!(loaded.file_existed);
        assert!(!loaded.read_only);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(loaded.session, written);
        assert_eq!(
            loaded.session.open_files,
            vec!["src/main.java".to_string(), "README.md".to_string()],
            "路径用工作区相对 + `/` 分隔"
        );
        assert_eq!(loaded.session.active_file, Some(1));
        assert_eq!(loaded.session.left_sidebar_visible, Some(false));
        assert_eq!(loaded.session.top_activity_view, Some(1));
        assert_eq!(loaded.session.bottom_kind.as_deref(), Some("terminal"));
    }

    /// 只读文档（版本过新）照样能读回来 —— 恢复不因为"版本新"就丢掉整个会话。
    #[test]
    fn a_newer_document_is_still_readable() {
        let dir = temp_dir("read-newer");
        let path = session_path(&dir);
        std::fs::create_dir_all(path.parent().expect("有父目录")).expect("建配置目录");
        std::fs::write(
            &path,
            r#"{"version":99,"openFiles":["kept.txt"],"activeFile":0}"#,
        )
        .expect("写文件");

        let loaded = load_session(&dir);
        assert!(loaded.read_only, "更高版本必须标成只读");
        assert_eq!(loaded.session.open_files, vec!["kept.txt".to_string()]);
        assert!(
            loaded
                .diagnostics
                .iter()
                .any(|line| line.contains("document_version_newer")),
            "{:?}",
            loaded.diagnostics
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 相对路径判定：空串 / 绝对路径 / `..` 逃逸一律拒绝。
    #[test]
    fn plain_relative_paths_reject_escapes() {
        for accepted in ["a.txt", "src/main.java", "deep/nested/x/y.txt"] {
            assert!(is_plain_relative(accepted), "{accepted} 应当被接受");
        }
        for rejected in ["", "/abs.txt", "\\abs.txt", "../up.txt", "a/../../b.txt", "C:/x.txt"] {
            assert!(!is_plain_relative(rejected), "{rejected} 应当被拒绝");
        }
    }

    /// 文档里的路径用 `/`，回到绝对路径时按平台分隔符还原（Windows 上必须能找到文件）。
    #[test]
    fn resolving_a_nested_relative_path_uses_the_platform_separator() {
        let dir = temp_dir("separator");
        write_files(&dir, &["src/main/java/App.java"]);
        let session = SessionState {
            open_files: vec!["src/main/java/App.java".to_string()],
            ..SessionState::default()
        };
        let resolved = resolve_session_files(&dir, &session);
        assert_eq!(
            resolved,
            vec![SessionFileOutcome::Restored(
                dir.join("src")
                    .join("main")
                    .join("java")
                    .join("App.java")
            )]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
