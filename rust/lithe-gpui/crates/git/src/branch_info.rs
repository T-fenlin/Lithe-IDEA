use std::path::Path;

use gpui_kit::SharedString;

use crate::model::{
    RefKind, Reference, discover_repository_root, execute_core, parse_references,
    repository_root_of, resolve_repository_root,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchInfo {
    pub name: SharedString,
    pub is_current: bool,
    pub upstream: Option<SharedString>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackingCounts {
    pub ahead: usize,
    pub behind: usize,
}

impl TrackingCounts {
    pub fn is_empty(self) -> bool {
        self.ahead == 0 && self.behind == 0
    }
}

#[derive(Clone, Debug, Default)]
pub struct BranchSnapshot {
    pub repository_root: Option<SharedString>,
    pub branch: Option<SharedString>,
    pub tracking: TrackingCounts,
    pub branches: Vec<BranchInfo>,
    pub failures: Vec<String>,
}

impl BranchSnapshot {
    pub fn current_index(&self) -> Option<usize> {
        self.branches.iter().position(|branch| branch.is_current)
    }

    pub fn load(root: &Path) -> Self {
        let mut snapshot = Self::default();

        let root_text = root.to_string_lossy().to_string();
        match read_status_detailed(&root_text) {
            Ok((repository_root, branch, tracking)) => {
                snapshot.repository_root =
                    repository_root.map(|found| resolve_repository_root(root, &found).into());
                snapshot.branch = branch;
                snapshot.tracking = tracking;
            }
            Err(code) => snapshot.failures.push(format!("git.status（{code}）")),
        }

        if snapshot.repository_root.is_none() {
            snapshot.repository_root = discover_repository_root(root).map(SharedString::from);
        }

        let Some(repository_root) = snapshot.repository_root.clone() else {
            return snapshot;
        };

        match execute_core(
            "git.references",
            serde_json::json!({ "root": repository_root.as_ref() }),
        ) {
            Ok(data) => {
                let references = parse_references(&data);
                let has_current = references.iter().any(|reference| reference.is_current);
                snapshot.branches = local_branches(&references, &snapshot.branch, has_current);
            }
            Err(code) => snapshot.failures.push(format!("git.references（{code}）")),
        }

        snapshot
    }
}

fn read_status_detailed(
    root: &str,
) -> Result<(Option<String>, Option<SharedString>, TrackingCounts), String> {
    let data = execute_core("git.status", serde_json::json!({ "root": root }))?;
    let branch = data
        .get("branch")
        .and_then(serde_json::Value::as_str)
        .map(SharedString::from);
    let tracking = TrackingCounts {
        ahead: data
            .get("ahead")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize,
        behind: data
            .get("behind")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize,
    };
    Ok((repository_root_of(&data), branch, tracking))
}

fn local_branches(
    references: &[Reference],
    status_branch: &Option<SharedString>,
    has_current_flag: bool,
) -> Vec<BranchInfo> {
    let mut branches: Vec<BranchInfo> = references
        .iter()
        .filter(|reference| reference.kind == RefKind::Local)
        .map(|reference| BranchInfo {
            name: reference.short_name.clone(),
            is_current: if has_current_flag {
                reference.is_current
            } else {
                status_branch
                    .as_ref()
                    .is_some_and(|branch| branch == &reference.short_name)
            },
            upstream: reference.upstream_short_name.clone(),
        })
        .collect();

    branches.sort_by(|left, right| {
        right
            .is_current
            .cmp(&left.is_current)
            .then_with(|| left.name.cmp(&right.name))
    });
    branches
}

#[cfg(test)]
mod tests {
    use super::{BranchInfo, BranchSnapshot, RefKind, TrackingCounts, local_branches};
    use crate::model::Reference;
    use gpui_kit::SharedString;

    fn reference(name: &str, kind: RefKind, is_current: bool) -> Reference {
        Reference {
            full_name: SharedString::from(format!("refs/heads/{name}")),
            short_name: SharedString::from(name),
            kind,
            is_current,
            upstream_short_name: None,
            ahead: 0,
            behind: 0,
        }
    }

    #[test]
    fn only_local_references_become_branch_rows() {
        let references = vec![
            reference("main", RefKind::Local, true),
            reference("origin/main", RefKind::Remote, false),
            reference("v1.0.0", RefKind::Tag, false),
            reference("feature/login", RefKind::Local, false),
        ];
        let branches = local_branches(&references, &Some(SharedString::from("main")), true);
        let names: Vec<&str> = branches.iter().map(|b| b.name.as_ref()).collect();
        assert_eq!(names, vec!["main", "feature/login"]);
    }

    #[test]
    fn current_branch_comes_first_then_alphabetical() {
        let references = vec![
            reference("zeta", RefKind::Local, false),
            reference("main", RefKind::Local, true),
            reference("alpha", RefKind::Local, false),
        ];
        let branches = local_branches(&references, &Some(SharedString::from("main")), true);
        let names: Vec<&str> = branches.iter().map(|b| b.name.as_ref()).collect();
        assert_eq!(names, vec!["main", "alpha", "zeta"]);
    }

    #[test]
    fn current_flag_falls_back_to_the_status_branch_name() {
        let references = vec![
            reference("main", RefKind::Local, false),
            reference("dev", RefKind::Local, false),
        ];
        let branches = local_branches(&references, &Some(SharedString::from("dev")), false);
        let current: Vec<&str> = branches
            .iter()
            .filter(|branch| branch.is_current)
            .map(|branch| branch.name.as_ref())
            .collect();
        assert_eq!(current, vec!["dev"]);
        assert_eq!(branches[0].name.as_ref(), "dev");
    }

    #[test]
    fn empty_references_give_an_empty_branch_list() {
        assert!(local_branches(&[], &None, false).is_empty());
        let snapshot = BranchSnapshot::default();
        assert_eq!(snapshot.current_index(), None);
        assert!(snapshot.failures.is_empty());
    }

    #[test]
    fn current_index_points_at_the_pinned_row() {
        let snapshot = BranchSnapshot {
            branches: vec![
                BranchInfo {
                    name: SharedString::from("main"),
                    is_current: true,
                    upstream: None,
                },
                BranchInfo {
                    name: SharedString::from("dev"),
                    is_current: false,
                    upstream: Some(SharedString::from("origin/dev")),
                },
            ],
            ..BranchSnapshot::default()
        };
        assert_eq!(snapshot.current_index(), Some(0));
    }

    #[test]
    fn tracking_counts_are_empty_only_when_both_are_zero() {
        assert!(
            TrackingCounts {
                ahead: 0,
                behind: 0
            }
            .is_empty()
        );
        assert!(
            !TrackingCounts {
                ahead: 2,
                behind: 0
            }
            .is_empty()
        );
        assert!(
            !TrackingCounts {
                ahead: 0,
                behind: 1
            }
            .is_empty()
        );
    }
}
