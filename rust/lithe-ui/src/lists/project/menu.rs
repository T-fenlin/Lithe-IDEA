//! The menu a project row opens, and the words the tree shows around it.

use gpui::SharedString;

use super::ProjectRowKind;
use crate::primitives::IconName;

/// What a row's menu can do: the reads the macOS truth's row menus have always had, plus the
/// rename, delete and new entries the IDE's project view offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectAction {
    Open,
    RevealInFileManager,
    CopyPath,
    CopyRelativePath,
    Refresh,
    Rename,
    Delete,
    NewFile,
    NewFolder,
}

/// The menu of a row kind, `None` for a separator; nothing is offered for a note row.
pub fn row_menu_entries(kind: ProjectRowKind) -> Vec<Option<ProjectAction>> {
    match kind {
        ProjectRowKind::EmptyFolder => Vec::new(),
        ProjectRowKind::File => vec![
            Some(ProjectAction::Open),
            None,
            Some(ProjectAction::NewFile),
            None,
            Some(ProjectAction::Rename),
            Some(ProjectAction::Delete),
            None,
            Some(ProjectAction::RevealInFileManager),
            Some(ProjectAction::CopyPath),
            Some(ProjectAction::CopyRelativePath),
        ],
        ProjectRowKind::Directory => vec![
            Some(ProjectAction::NewFile),
            Some(ProjectAction::NewFolder),
            None,
            Some(ProjectAction::Rename),
            Some(ProjectAction::Delete),
            None,
            Some(ProjectAction::RevealInFileManager),
            Some(ProjectAction::CopyPath),
            Some(ProjectAction::CopyRelativePath),
            None,
            Some(ProjectAction::Refresh),
        ],
    }
}

/// Every action the menu can carry, so a caller can be exhaustive about its own wiring.
pub const PROJECT_ACTIONS: [ProjectAction; 9] = [
    ProjectAction::Open,
    ProjectAction::RevealInFileManager,
    ProjectAction::CopyPath,
    ProjectAction::CopyRelativePath,
    ProjectAction::Refresh,
    ProjectAction::Rename,
    ProjectAction::Delete,
    ProjectAction::NewFile,
    ProjectAction::NewFolder,
];

impl ProjectAction {
    /// The entry's word in the given labels.
    pub fn label(self, labels: &ProjectLabels) -> SharedString {
        match self {
            Self::Open => labels.open.clone(),
            Self::RevealInFileManager => labels.reveal.clone(),
            Self::CopyPath => labels.copy_path.clone(),
            Self::CopyRelativePath => labels.copy_relative_path.clone(),
            Self::Refresh => labels.refresh.clone(),
            Self::Rename => labels.rename.clone(),
            Self::Delete => labels.delete.clone(),
            Self::NewFile => labels.new_file.clone(),
            Self::NewFolder => labels.new_folder.clone(),
        }
    }

    /// The glyph the entry leads with, so every menu that shows one action reads the same mapping.
    pub fn icon(self) -> IconName {
        match self {
            Self::Open => IconName::ExternalLink,
            Self::RevealInFileManager => IconName::FolderOpen,
            Self::CopyPath | Self::CopyRelativePath => IconName::Copy,
            Self::Refresh => IconName::RefreshCw,
            Self::Rename => IconName::Pencil,
            Self::Delete => IconName::Trash2,
            Self::NewFile => IconName::FilePlus,
            Self::NewFolder => IconName::FolderPlus,
        }
    }
}

/// The owner's words for what the tree says on its own: the note an empty folder shows, and the
/// entries of a row's menu. The owner keeps them in its own catalogue.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectLabels {
    pub empty_folder: SharedString,
    pub open: SharedString,
    pub reveal: SharedString,
    pub copy_path: SharedString,
    pub copy_relative_path: SharedString,
    pub refresh: SharedString,
    pub rename: SharedString,
    pub delete: SharedString,
    pub new_file: SharedString,
    pub new_folder: SharedString,
}
