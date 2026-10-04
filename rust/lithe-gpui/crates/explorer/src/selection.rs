#![allow(dead_code)]

use std::collections::BTreeSet;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ProjectTreeSelection {
    paths: BTreeSet<String>,
    anchor: Option<String>,
    focused: Option<String>,
}

impl ProjectTreeSelection {
    pub(crate) fn select(
        &mut self,
        path: &str,
        visible: &[String],
        extending: bool,
        toggling: bool,
    ) {
        self.focused = Some(path.to_string());
        let anchor_index = self
            .anchor
            .as_deref()
            .and_then(|anchor| visible.iter().position(|item| item == anchor));
        let target_index = visible.iter().position(|item| item == path);
        match (extending, anchor_index, target_index) {
            (true, Some(start), Some(end)) => {
                let (low, high) = if start <= end {
                    (start, end)
                } else {
                    (end, start)
                };
                let range = &visible[low..=high];
                if toggling {
                    self.paths.extend(range.iter().cloned());
                } else {
                    self.paths = range.iter().cloned().collect();
                }
            }
            _ if toggling => {
                if !self.paths.insert(path.to_string()) {
                    self.paths.remove(path);
                }
                self.anchor = Some(path.to_string());
            }
            _ => {
                self.paths = BTreeSet::from([path.to_string()]);
                self.anchor = Some(path.to_string());
            }
        }
    }

    pub(crate) fn covers(&self, path: &str) -> bool {
        let normalized = path.replace('\\', "/");
        let mut current = normalized.as_str();
        loop {
            if self.paths.contains(current) {
                return true;
            }
            match current.rfind('/') {
                Some(0) | None => return false,
                Some(index) => current = &current[..index],
            }
        }
    }

    pub(crate) fn select_for_context_menu(&mut self, path: &str) {
        self.focused = Some(path.to_string());
        if self.paths.contains(path) || (self.paths.len() > 1 && self.covers(path)) {
            return;
        }
        self.paths = BTreeSet::from([path.to_string()]);
        self.anchor = Some(path.to_string());
    }

    pub(crate) fn retain(&mut self, visible: &[String]) {
        let visible: BTreeSet<&String> = visible.iter().collect();
        self.paths.retain(|path| visible.contains(path));
        if self.anchor.as_ref().is_some_and(|a| !visible.contains(a)) {
            self.anchor = None;
        }
        if self.focused.as_ref().is_some_and(|f| !visible.contains(f)) {
            self.focused = None;
        }
    }

    pub(crate) fn select_all_in_siblings(&mut self, siblings: &[String]) {
        let Some(first) = siblings.first() else {
            return;
        };
        self.paths = siblings.iter().cloned().collect();
        self.anchor = Some(first.clone());
        if self
            .focused
            .as_ref()
            .is_none_or(|f| !self.paths.contains(f))
        {
            self.focused = Some(first.clone());
        }
    }

    pub(crate) fn paths(&self) -> &BTreeSet<String> {
        &self.paths
    }

    pub(crate) fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    pub(crate) fn anchor(&self) -> Option<&str> {
        self.anchor.as_deref()
    }

    pub(crate) fn clear(&mut self) {
        self.paths.clear();
        self.anchor = None;
        self.focused = None;
    }

    pub(crate) fn follow_active_document(&mut self, path: Option<&str>) -> bool {
        let Some(path) = path else {
            return false;
        };
        if self.paths.len() > 1 {
            return false;
        }
        self.select(path, &[], false, false);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visible() -> Vec<String> {
        ["src", "src/a.rs", "src/b.rs", "tests", "README.md"]
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    }

    #[test]
    fn a_plain_click_replaces_the_selection() {
        let mut selection = ProjectTreeSelection::default();
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
    fn command_click_toggles_one_row_and_moves_the_anchor() {
        let mut selection = ProjectTreeSelection::default();
        selection.select("src", &visible(), false, false);
        selection.select("README.md", &visible(), false, true);
        assert_eq!(selection.paths().len(), 2);
        assert_eq!(selection.anchor(), Some("README.md"));
        selection.select("README.md", &visible(), false, true);
        assert_eq!(selection.paths().len(), 1);
        assert!(!selection.paths().contains("README.md"));
    }

    #[test]
    fn shift_click_selects_a_range_in_display_order() {
        let mut selection = ProjectTreeSelection::default();
        selection.select("src", &visible(), false, false);
        selection.select("src/b.rs", &visible(), true, false);
        assert_eq!(
            selection.paths().iter().cloned().collect::<Vec<_>>(),
            vec!["src", "src/a.rs", "src/b.rs"]
        );
        assert_eq!(selection.anchor(), Some("src"), "Shift 不改锚点");

        let mut reversed = ProjectTreeSelection::default();
        reversed.select("README.md", &visible(), false, false);
        reversed.select("tests", &visible(), true, false);
        assert_eq!(
            reversed.paths().iter().cloned().collect::<Vec<_>>(),
            vec!["README.md", "tests"]
        );
    }

    #[test]
    fn shift_command_click_unions_with_the_existing_selection() {
        let mut selection = ProjectTreeSelection::default();
        selection.select("README.md", &visible(), false, false);
        selection.select("src", &visible(), false, true);
        selection.select("src/b.rs", &visible(), true, true);
        assert_eq!(
            selection.paths().iter().cloned().collect::<Vec<_>>(),
            vec!["README.md", "src", "src/a.rs", "src/b.rs"]
        );
    }

    #[test]
    fn a_selected_folder_covers_its_descendants_only() {
        let mut selection = ProjectTreeSelection::default();
        selection.select("src", &visible(), false, false);
        assert!(selection.covers("src"));
        assert!(selection.covers("src/a.rs"));
        assert!(selection.covers("src/deep/nested/File.java"));
        assert!(!selection.covers("tests"));
        assert!(!selection.covers("README.md"));
        assert!(!selection.covers("src2/a.rs"));
    }

    #[test]
    fn context_menu_keeps_an_existing_selection() {
        let mut single = ProjectTreeSelection::default();
        single.select("src/a.rs", &visible(), false, false);
        single.select_for_context_menu("src/a.rs");
        assert_eq!(single.paths().len(), 1, "点自己不该丢选择");
        assert_eq!(single.focused(), Some("src/a.rs"));

        let mut folder = ProjectTreeSelection::default();
        folder.select("src", &visible(), false, false);
        folder.select("README.md", &visible(), false, true);
        folder.select_for_context_menu("src/a.rs");
        assert_eq!(folder.paths().len(), 2, "多选时点子项保留整份选择");
        assert!(folder.paths().contains("src"));

        let mut other = ProjectTreeSelection::default();
        other.select("src", &visible(), false, false);
        other.select("README.md", &visible(), false, true);
        other.select_for_context_menu("tests");
        assert_eq!(other.paths().len(), 1, "点未选中的无关行改成只选它");
        assert!(other.paths().contains("tests"));
        assert_eq!(other.anchor(), Some("tests"));
    }

    #[test]
    fn retain_drops_paths_that_are_no_longer_visible() {
        let mut selection = ProjectTreeSelection::default();
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

        let mut hidden = ProjectTreeSelection::default();
        hidden.select("src/a.rs", &visible(), false, false);
        hidden.retain(&collapsed);
        assert!(hidden.paths().is_empty());
        assert_eq!(hidden.anchor(), None);
        assert_eq!(hidden.focused(), None);
    }

    #[test]
    fn select_all_takes_the_given_siblings() {
        let mut selection = ProjectTreeSelection::default();
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

        let mut elsewhere = ProjectTreeSelection::default();
        elsewhere.select("README.md", &visible(), false, false);
        elsewhere.select_all_in_siblings(&siblings);
        assert_eq!(elsewhere.focused(), Some("src/a.rs"));

        let mut empty = ProjectTreeSelection::default();
        empty.select("README.md", &visible(), false, false);
        empty.select_all_in_siblings(&[]);
        assert_eq!(empty.paths().len(), 1);
    }

    #[test]
    fn a_single_selection_follows_the_active_document() {
        let mut empty = ProjectTreeSelection::default();
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

        let mut none = ProjectTreeSelection::default();
        assert!(!none.follow_active_document(None));
        assert!(none.paths().is_empty());
    }

    #[test]
    fn an_explicit_group_ignores_the_active_document() {
        let mut group = ProjectTreeSelection::default();
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
}
