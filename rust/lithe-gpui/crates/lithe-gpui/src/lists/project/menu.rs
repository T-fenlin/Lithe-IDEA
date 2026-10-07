//! The menu a project row opens, and the words the tree shows around it.

use gpui::SharedString;

use super::ProjectRowKind;

/// What a row's menu can do. The five entries are exactly the macOS truth's row menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectAction {
    Open,
    RevealInFileManager,
    CopyPath,
    CopyRelativePath,
    Refresh,
}

/// The menu of a row kind, `None` for a separator; nothing is offered for a note row.
pub fn row_menu_entries(kind: ProjectRowKind) -> Vec<Option<ProjectAction>> {
    match kind {
        ProjectRowKind::EmptyFolder => Vec::new(),
        ProjectRowKind::File => vec![
            Some(ProjectAction::Open),
            None,
            Some(ProjectAction::RevealInFileManager),
            Some(ProjectAction::CopyPath),
            Some(ProjectAction::CopyRelativePath),
        ],
        ProjectRowKind::Directory => vec![
            Some(ProjectAction::RevealInFileManager),
            Some(ProjectAction::CopyPath),
            Some(ProjectAction::CopyRelativePath),
            None,
            Some(ProjectAction::Refresh),
        ],
    }
}

/// Every action the menu can carry, so a caller can be exhaustive about its own wiring.
pub const PROJECT_ACTIONS: [ProjectAction; 5] = [
    ProjectAction::Open,
    ProjectAction::RevealInFileManager,
    ProjectAction::CopyPath,
    ProjectAction::CopyRelativePath,
    ProjectAction::Refresh,
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
}
