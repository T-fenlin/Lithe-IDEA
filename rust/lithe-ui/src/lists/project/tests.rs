//! The project tree's own tests: the rows it materializes, what it picks, and what its rows offer.

use std::collections::HashSet;

use gpui::SharedString;

use super::{
    GitStatus, ProjectAction, ProjectLabels, ProjectNode, ProjectRowKind, fold_status,
    menu::row_menu_entries,
    node::{Arena, materialize},
    select::ProjectSelection,
};

fn keys(list: &[&'static str]) -> HashSet<SharedString> {
    list.iter().map(|key| SharedString::from(*key)).collect()
}

/// The workspace the tests walk: folders, files, and one folder that is empty.
fn forest() -> Vec<ProjectNode> {
    vec![
        ProjectNode::folder("src", "src")
            .child(
                ProjectNode::folder("src/main", "main")
                    .child(ProjectNode::file("src/main/App.java", "App.java")),
            )
            .child(ProjectNode::file("src/lib.rs", "lib.rs")),
        ProjectNode::folder("logs", "logs"),
        ProjectNode::file("a.rs", "a.rs"),
        ProjectNode::file("B.rs", "B.rs"),
    ]
}

fn arena() -> Arena {
    Arena::build("workspace".into(), forest())
}

/// The rows as `key:depth` pairs, so a test says exactly what is on screen.
fn shown(
    arena: &Arena,
    open: &[&'static str],
    query: &str,
    budget: usize,
) -> (Vec<String>, Vec<usize>, bool) {
    let (rows, truncated) = materialize(arena, &keys(open), query, budget);
    let keys = rows.iter().map(|row| row.key(arena).to_string()).collect();
    let depths = rows.iter().map(|row| row.depth).collect();
    (keys, depths, truncated)
}

#[test]
fn the_workspace_root_comes_first_and_folders_come_before_files() {
    let arena = arena();
    let (keys, depths, truncated) = shown(&arena, &["dir:"], "", 100);
    assert_eq!(
        keys,
        ["dir:", "dir:logs", "dir:src", "file:a.rs", "file:B.rs"],
        "shut folders cost one row each, and the root is open"
    );
    assert_eq!(depths, [0, 1, 1, 1, 1]);
    assert!(!truncated);
    let (keys, _, _) = shown(&arena, &[], "", 100);
    assert_eq!(keys, ["dir:"], "a shut root shows only itself");
}

#[test]
fn an_open_folder_shows_its_children_and_an_empty_one_shows_a_note() {
    let arena = arena();
    let (keys, depths, _) = shown(&arena, &["dir:", "dir:src", "dir:logs"], "", 100);
    assert_eq!(
        keys,
        [
            "dir:",
            "dir:logs",
            "empty:logs",
            "dir:src",
            "dir:src/main",
            "file:src/lib.rs",
            "file:a.rs",
            "file:B.rs"
        ],
        "each folder's children sit right under it, and an open-but-empty one says so"
    );
    assert_eq!(depths, [0, 1, 2, 1, 2, 2, 1, 1]);
    let (keys, _, _) = shown(&arena, &["dir:", "dir:src", "dir:src/main"], "", 100);
    assert!(
        keys.contains(&"file:src/main/App.java".to_string()),
        "the file inside the folder that was opened shows: {keys:?}"
    );
}

#[test]
fn a_filter_keeps_matches_and_the_folders_that_hold_them() {
    let arena = arena();
    let (keys, _, _) = shown(&arena, &[], "java", 100);
    assert_eq!(
        keys,
        ["dir:", "dir:src", "dir:src/main", "file:src/main/App.java"],
        "nothing has to be opened first: a filter opens what it keeps"
    );
    let (keys, _, _) = shown(&arena, &[], "nothing-matches", 100);
    assert_eq!(keys, ["dir:", "empty:"], "the root says it is empty");
}

#[test]
fn a_shallow_row_is_bought_before_a_deep_one() {
    // The root's first folder holds far more than the budget; its neighbours must still show.
    let deep = ProjectNode::folder("big", "big")
        .children((0..20).map(|ix| ProjectNode::file(format!("big/{ix}.rs"), format!("{ix}.rs"))));
    let arena = Arena::build(
        "workspace".into(),
        vec![deep, ProjectNode::folder("zeta", "zeta")],
    );
    let (keys, _, truncated) = shown(&arena, &["dir:", "dir:big"], "", 5);
    assert!(
        keys.contains(&"dir:zeta".to_string()),
        "the top-level folder next door survived: {keys:?}"
    );
    assert!(truncated, "the budget was spent");
    assert_eq!(keys.len(), 5);
}

#[test]
fn the_row_budget_counts_visible_rows_and_says_when_it_cut() {
    let children: Vec<ProjectNode> = (0..30)
        .map(|ix| ProjectNode::folder(format!("d{ix}"), format!("d{ix}")))
        .collect();
    let arena = Arena::build("workspace".into(), children);
    let (keys, _, truncated) = shown(&arena, &["dir:"], "", 10);
    assert_eq!(keys.len(), 10);
    assert!(truncated);
    let (keys, _, truncated) = shown(&arena, &["dir:"], "", 100);
    assert_eq!(keys.len(), 31, "the root and its thirty folders");
    assert!(!truncated);
}

#[test]
fn a_key_says_its_kind() {
    assert_eq!(
        ProjectRowKind::parse("dir:src/main"),
        (ProjectRowKind::Directory, "src/main")
    );
    assert_eq!(
        ProjectRowKind::parse("file:src/main/App.java"),
        (ProjectRowKind::File, "src/main/App.java")
    );
    assert_eq!(
        ProjectRowKind::parse("empty:logs"),
        (ProjectRowKind::EmptyFolder, "logs")
    );
    assert_eq!(
        ProjectRowKind::parse("dir:"),
        (ProjectRowKind::Directory, "")
    );
}

#[test]
fn a_row_key_is_the_node_key_or_the_empty_note() {
    let arena = arena();
    let (rows, _) = materialize(&arena, &keys(&["dir:", "dir:logs"]), "", 100);
    let keys: Vec<String> = rows.iter().map(|row| row.key(&arena).to_string()).collect();
    assert!(keys.contains(&"empty:logs".to_string()));
    let note = rows
        .iter()
        .find(|row| row.key(&arena).as_ref() == "empty:logs")
        .expect("the note row");
    assert_eq!(note.kind(&arena), ProjectRowKind::EmptyFolder);
    assert_eq!(note.depth, 2);
}

#[test]
fn the_menu_offers_the_reads_and_the_file_writes() {
    assert_eq!(
        row_menu_entries(ProjectRowKind::File),
        vec![
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
        ]
    );
    assert_eq!(
        row_menu_entries(ProjectRowKind::Directory),
        vec![
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
        ]
    );
    assert!(row_menu_entries(ProjectRowKind::EmptyFolder).is_empty());
    let mut actions: Vec<ProjectAction> = row_menu_entries(ProjectRowKind::File)
        .into_iter()
        .chain(row_menu_entries(ProjectRowKind::Directory))
        .flatten()
        .collect();
    actions.sort_by_key(|action| format!("{action:?}"));
    actions.dedup();
    let mut expected = super::PROJECT_ACTIONS.to_vec();
    expected.sort_by_key(|action| format!("{action:?}"));
    assert_eq!(
        actions, expected,
        "the two menus together carry every action and nothing else"
    );
}

#[test]
fn every_menu_entry_has_its_own_glyph() {
    use crate::primitives::IconName;

    // The loop touches every variant, so a new action with no mapping fails to compile.
    for action in super::PROJECT_ACTIONS {
        let _ = action.icon();
    }
    // The two copy entries share a glyph on purpose; the rest stay apart for scanning.
    assert_eq!(
        ProjectAction::CopyPath.icon(),
        ProjectAction::CopyRelativePath.icon()
    );
    assert_eq!(ProjectAction::Rename.icon(), IconName::Pencil);
    assert_eq!(ProjectAction::Delete.icon(), IconName::Trash2);
    assert_eq!(ProjectAction::NewFile.icon(), IconName::FilePlus);
    assert_eq!(ProjectAction::NewFolder.icon(), IconName::FolderPlus);
}

#[test]
fn a_rename_selects_the_main_name_not_the_extension() {
    assert_eq!(super::stem_len("App.java"), 3);
    assert_eq!(
        super::stem_len("foo.bar.java"),
        7,
        "only the last dot is split"
    );
    assert_eq!(super::stem_len("README"), 6, "no dot, the whole name");
    assert_eq!(
        super::stem_len(".gitignore"),
        10,
        "a leading dot hides the name, it does not open an extension"
    );
    assert_eq!(super::stem_len("a."), 1);
}

#[test]
fn every_menu_entry_has_its_own_word() {
    let labels = ProjectLabels {
        empty_folder: "Folder is empty".into(),
        open: "Open".into(),
        reveal: "Reveal in File Manager".into(),
        copy_path: "Copy Path".into(),
        copy_relative_path: "Copy Relative Path".into(),
        refresh: "Refresh".into(),
        rename: "Rename".into(),
        delete: "Delete".into(),
        new_file: "New File".into(),
        new_folder: "New Folder".into(),
    };
    let mut words: Vec<String> = super::PROJECT_ACTIONS
        .iter()
        .map(|action| action.label(&labels).to_string())
        .collect();
    let count = words.len();
    words.sort();
    words.dedup();
    assert_eq!(words.len(), count, "two entries must not read the same");
}

#[test]
fn git_state_folds_onto_the_folders_that_hold_it() {
    let (files, groups) = fold_status([
        ("src/App.java".into(), GitStatus::Modified),
        ("src/ui/Button.java".into(), GitStatus::Conflicted),
        ("readme.md".into(), GitStatus::Added),
    ]);
    assert_eq!(files.len(), 3);
    assert_eq!(files.get("src/App.java"), Some(&GitStatus::Modified));
    assert_eq!(
        groups.get("src"),
        Some(&GitStatus::Conflicted),
        "the most urgent descendant wins"
    );
    assert_eq!(groups.get("src/ui"), Some(&GitStatus::Conflicted));
    assert_eq!(
        groups.get(""),
        Some(&GitStatus::Conflicted),
        "the root row carries the workspace's worst state"
    );
    assert_eq!(groups.get("readme.md"), None, "a file is not a folder");
}

#[test]
fn the_first_state_of_a_path_wins_and_the_root_is_never_a_file() {
    let (files, groups) = fold_status([
        ("src/App.java".into(), GitStatus::Modified),
        ("src/App.java".into(), GitStatus::Deleted),
        (SharedString::default(), GitStatus::Conflicted),
    ]);
    assert_eq!(files.get("src/App.java"), Some(&GitStatus::Modified));
    assert_eq!(
        groups.get(""),
        Some(&GitStatus::Deleted),
        "a folder still takes the most urgent state reported under it, duplicates included"
    );
    assert_eq!(
        files.len(),
        1,
        "the workspace root never becomes a file row"
    );
}

// --- the selection, ported from the macOS truth ---------------------------------------------

fn visible() -> Vec<String> {
    ["src", "src/a.rs", "src/b.rs", "tests", "README.md"]
        .iter()
        .map(|item| (*item).to_string())
        .collect()
}

#[test]
fn a_plain_press_replaces_the_selection() {
    let mut selection = ProjectSelection::default();
    selection.select("src/a.rs", &visible(), false, false);
    assert_eq!(selection.paths().len(), 1);
    assert!(selection.paths().contains("src/a.rs"));
    assert_eq!(selection.anchor(), Some("src/a.rs"));
    assert_eq!(selection.focused(), Some("src/a.rs"));
    selection.select("src/b.rs", &visible(), false, false);
    assert_eq!(selection.paths().len(), 1);
    assert!(selection.paths().contains("src/b.rs"));
}

#[test]
fn a_widened_press_toggles_one_row_and_moves_the_anchor() {
    let mut selection = ProjectSelection::default();
    selection.select("src", &visible(), false, false);
    selection.select("README.md", &visible(), false, true);
    assert_eq!(selection.paths().len(), 2);
    assert_eq!(selection.anchor(), Some("README.md"));
    selection.select("README.md", &visible(), false, true);
    assert_eq!(selection.paths().len(), 1);
    assert!(!selection.paths().contains("README.md"));
}

#[test]
fn a_shift_press_takes_a_range_in_display_order() {
    let mut selection = ProjectSelection::default();
    selection.select("src", &visible(), false, false);
    selection.select("src/b.rs", &visible(), true, false);
    assert_eq!(
        selection.paths().iter().cloned().collect::<Vec<_>>(),
        vec!["src", "src/a.rs", "src/b.rs"]
    );
    assert_eq!(selection.anchor(), Some("src"), "Shift 不改锚点");

    let mut reversed = ProjectSelection::default();
    reversed.select("README.md", &visible(), false, false);
    reversed.select("tests", &visible(), true, false);
    assert_eq!(
        reversed.paths().iter().cloned().collect::<Vec<_>>(),
        vec!["README.md", "tests"]
    );
}

#[test]
fn shift_with_a_widened_press_unions_with_the_existing_selection() {
    let mut selection = ProjectSelection::default();
    selection.select("README.md", &visible(), false, false);
    selection.select("src", &visible(), false, true);
    selection.select("src/b.rs", &visible(), true, true);
    assert_eq!(
        selection.paths().iter().cloned().collect::<Vec<_>>(),
        vec!["README.md", "src", "src/a.rs", "src/b.rs"]
    );
}

#[test]
fn a_picked_folder_covers_its_descendants_only() {
    let mut selection = ProjectSelection::default();
    selection.select("src", &visible(), false, false);
    assert!(selection.covers("src"));
    assert!(selection.covers("src/a.rs"));
    assert!(selection.covers("src/deep/nested/File.java"));
    assert!(!selection.covers("tests"));
    assert!(!selection.covers("README.md"));
    assert!(!selection.covers("src2/a.rs"));
}

#[test]
fn a_right_press_keeps_a_selection_that_holds_the_row() {
    let mut single = ProjectSelection::default();
    single.select("src/a.rs", &visible(), false, false);
    single.select_for_context_menu("src/a.rs");
    assert_eq!(single.paths().len(), 1, "点自己不该丢选择");
    assert_eq!(single.focused(), Some("src/a.rs"));

    let mut folder = ProjectSelection::default();
    folder.select("src", &visible(), false, false);
    folder.select("README.md", &visible(), false, true);
    folder.select_for_context_menu("src/a.rs");
    assert_eq!(folder.paths().len(), 2, "多选时点子项保留整份选择");
    assert!(folder.paths().contains("src"));

    let mut other = ProjectSelection::default();
    other.select("src", &visible(), false, false);
    other.select("README.md", &visible(), false, true);
    other.select_for_context_menu("tests");
    assert_eq!(other.paths().len(), 1, "点未选中的无关行改成只选它");
    assert!(other.paths().contains("tests"));
    assert_eq!(other.anchor(), Some("tests"));
}

#[test]
fn what_is_no_longer_visible_leaves_the_selection() {
    let mut selection = ProjectSelection::default();
    selection.select("src", &visible(), false, false);
    selection.select("src/a.rs", &visible(), false, true);
    let collapsed = vec![
        "src".to_string(),
        "tests".to_string(),
        "README.md".to_string(),
    ];
    selection.retain(&collapsed);
    assert_eq!(
        selection.paths().iter().cloned().collect::<Vec<_>>(),
        vec!["src"]
    );
    assert_eq!(selection.focused(), None, "焦点不再可见 ⇒ 清掉");

    let mut hidden = ProjectSelection::default();
    hidden.select("src/a.rs", &visible(), false, false);
    hidden.retain(&collapsed);
    assert!(hidden.paths().is_empty());
    assert_eq!(hidden.anchor(), None);
    assert_eq!(hidden.focused(), None);
}

#[test]
fn select_all_takes_the_given_siblings() {
    let mut selection = ProjectSelection::default();
    selection.select("src/a.rs", &visible(), false, false);
    let siblings = vec!["src/a.rs".to_string(), "src/b.rs".to_string()];
    selection.select_all_in_siblings(&siblings);
    assert_eq!(selection.paths().len(), 2);
    assert_eq!(selection.anchor(), Some("src/a.rs"));
    assert_eq!(
        selection.focused(),
        Some("src/a.rs"),
        "焦点已在集合内则不动"
    );

    let mut elsewhere = ProjectSelection::default();
    elsewhere.select("README.md", &visible(), false, false);
    elsewhere.select_all_in_siblings(&siblings);
    assert_eq!(elsewhere.focused(), Some("src/a.rs"));

    let mut empty = ProjectSelection::default();
    empty.select("README.md", &visible(), false, false);
    empty.select_all_in_siblings(&[]);
    assert_eq!(empty.paths().len(), 1);
}

#[test]
fn a_single_selection_follows_the_active_document() {
    let mut empty = ProjectSelection::default();
    assert!(empty.follow_active_document(Some("src/a.rs")));
    assert_eq!(
        empty.paths().iter().cloned().collect::<Vec<_>>(),
        vec!["src/a.rs"]
    );
    assert_eq!(empty.anchor(), Some("src/a.rs"));
    assert_eq!(empty.focused(), Some("src/a.rs"));

    assert!(empty.follow_active_document(Some("src/b.rs")));
    assert_eq!(
        empty.paths().iter().cloned().collect::<Vec<_>>(),
        vec!["src/b.rs"]
    );

    let mut none = ProjectSelection::default();
    assert!(!none.follow_active_document(None));
    assert!(none.paths().is_empty());
}

#[test]
fn an_explicit_group_ignores_the_active_document() {
    let mut group = ProjectSelection::default();
    group.select("src", &visible(), false, false);
    group.select("README.md", &visible(), false, true);
    assert_eq!(group.paths().len(), 2);
    assert!(
        !group.follow_active_document(Some("src/a.rs")),
        "多选不该被跟随改掉"
    );
    assert_eq!(
        group.paths().iter().cloned().collect::<Vec<_>>(),
        vec!["README.md", "src"],
        "选择集原样保留"
    );
    assert_eq!(group.focused(), Some("README.md"));
}
