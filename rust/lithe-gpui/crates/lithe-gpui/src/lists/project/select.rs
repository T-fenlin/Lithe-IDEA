//! The project tree's selection: one row, a ⌘-widened set, or a Shift range in display order.

use std::collections::BTreeSet;

/// The rows picked, where a Shift range starts, and the row the owner last acted on.
/// Ported from the macOS truth (`ProjectTreeSelection.swift`); the paths are workspace-relative.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ProjectSelection {
    paths: BTreeSet<String>,
    anchor: Option<String>,
    focused: Option<String>,
}

impl ProjectSelection {
    /// Picks `path`: a plain press replaces, a Shift press takes the range from the anchor in
    /// display order, and a widened press adds or drops one and moves the anchor.
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

    /// Whether the selection holds this path or a folder above it.
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

    /// A right press: the row it lands on, keeping a selection that already holds it.
    pub(crate) fn select_for_context_menu(&mut self, path: &str) {
        self.focused = Some(path.to_string());
        if self.paths.contains(path) || (self.paths.len() > 1 && self.covers(path)) {
            return;
        }
        self.paths = BTreeSet::from([path.to_string()]);
        self.anchor = Some(path.to_string());
    }

    /// Drops what is no longer visible, and forgets an anchor or focus that went with it.
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

    /// ⌘A: the focused row's siblings, or the root's children when nothing is focused.
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

    /// Only the tests read the anchor back; the selection uses it on every widening press.
    #[cfg(test)]
    pub(crate) fn anchor(&self) -> Option<&str> {
        self.anchor.as_deref()
    }

    pub(crate) fn clear(&mut self) {
        self.paths.clear();
        self.anchor = None;
        self.focused = None;
    }

    /// A single selection follows the editor; an explicit group stays until the user changes it.
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
