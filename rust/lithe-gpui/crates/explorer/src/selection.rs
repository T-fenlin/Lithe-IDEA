//! 项目树的**多选模型**：真源 `ProjectTreeSelection` 的逐条移植。
//!
//! 真源：`macos/Sources/Lithe/Views/Workspace/ProjectTreeSelection.swift`（78 行）。
//! 本模块只做纯数据变换（标识别一律是**工作区相对路径**、`/` 分隔），不依赖 gpui 组件、
//! 不开窗口，所以每条语义都能被确定性单测钉住。
//!
//! ## 为什么先做模型再接线
//!
//! 上游 `TreeState` 只有单个 `selected_ix`（`gpui-base-0.7.0/src/tree.rs:195`），而且
//! `Tree::new` 的 wrapper 会在 `render_item` 返回后**覆盖**调用方设的 `.selected(...)`
//! （`gpui-component-0.7.0/src/tree.rs:89-92`）⇒ 行的选中态只能由我们自己画。
//! 于是"选择集 + 锚点 + 焦点"必须自持；先把这套语义钉死，接线（行底色、修饰键、键盘动作）
//! 是下一步 —— 与 `explorer/src/lib.rs` 模块文档第 11 条登记的路线一致。
//!
//! ⚠️ 本文件落地时**还没有调用方**（接线在下一步），所以整模块 `allow(dead_code)`；
//! 这与 `workspace.rs` 里 `clear_breakpoints` 的处置同类（实现了、入口未接）。
#![allow(dead_code)]

use std::collections::BTreeSet;

/// 选择集 + Shift 区间锚点 + 键盘焦点。
///
/// 用 `BTreeSet` 而不是 `HashSet`：`paths()` 会被用于"批量操作要处理哪些条目"，
/// 结果必须**确定性有序**（仓库的契约口径：列表与序列化结果确定性排序）。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ProjectTreeSelection {
    paths: BTreeSet<String>,
    anchor: Option<String>,
    focused: Option<String>,
}

impl ProjectTreeSelection {
    /// 真源 `:11-28` 的 `select`。
    ///
    /// - `visible`：**显示顺序**的可见路径表（真源 `visibleNodes`，`:72-77`）—— 区间选择按它切片，
    ///   不是按路径字典序；
    /// - `extending`：Shift；
    /// - `toggling`：⌘（单独 ⌘ 是切换，Shift+⌘ 是与已有集求并）。
    ///
    /// 三种分支与真源逐条一致；`focused` 恒被设为 `path`。
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
                    // Shift+⌘：并入已有选择（锚点保持不动）。
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

    /// 真源 `:32-41` 的 `covers`：选中的**文件夹连带其子孙**都算选中。
    ///
    /// 沿**文件系统父级**上溯（真源用 `deletingLastPathComponent`）；路径按 `/` 分段，
    /// 所以既处理 `/` 也处理 `\`（后者先归一）。
    pub(crate) fn covers(&self, path: &str) -> bool {
        let normalized = path.replace('\\', "/");
        let mut current = normalized.as_str();
        loop {
            if self.paths.contains(current) {
                return true;
            }
            match current.rfind('/') {
                // 父级为空（顶层项）或与当前相同：到顶了。
                Some(0) | None => return false,
                Some(index) => current = &current[..index],
            }
        }
    }

    /// 真源 `:43-49` 的 `selectForContextMenu`。
    ///
    /// 右键时：该行**已在选中集内**，或（选中多于一条且 `covers(path)`）⇒ **保留**当前选择
    /// —— 单项菜单作用于被点的那一行；否则改成只选该行并把锚点移到它。
    pub(crate) fn select_for_context_menu(&mut self, path: &str) {
        self.focused = Some(path.to_string());
        if self.paths.contains(path) || (self.paths.len() > 1 && self.covers(path)) {
            return;
        }
        self.paths = BTreeSet::from([path.to_string()]);
        self.anchor = Some(path.to_string());
    }

    /// 真源 `:51-56` 的 `retain`：折叠 / 刷新后把选择集与**当前可见**求交，
    /// 锚点与焦点一旦不可见就清掉。
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

    /// 真源 `:59-70` 的 `selectAll`。真源在类型内部按**显示树**求焦点行的同级兄弟；
    /// 本侧把那份计算留在调用方（它手里才有显示树），这里只接收兄弟表：
    /// 语义完全一致 —— 选中的是"同级兄弟"，**不是**整棵可见树（文件夹的子孙由 [`Self::covers`] 覆盖）。
    ///
    /// 锚点落在第一个兄弟；焦点若不在选中集里就移到第一个兄弟（与真源同）。
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

    /// 当前选中集（确定性有序）。
    pub(crate) fn paths(&self) -> &BTreeSet<String> {
        &self.paths
    }

    /// 键盘焦点（最近一次点击 / 键盘导航落点）。
    pub(crate) fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// Shift 区间锚点。
    pub(crate) fn anchor(&self) -> Option<&str> {
        self.anchor.as_deref()
    }

    /// 清空（Esc；真源没有这一步 —— 它是本侧键盘收尾的常规做法，语义上是"取消选择"）。
    pub(crate) fn clear(&mut self) {
        self.paths.clear();
        self.anchor = None;
        self.focused = None;
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

    /// 普通点击：只选该条，锚点与焦点都落到它。
    #[test]
    fn a_plain_click_replaces_the_selection() {
        let mut selection = ProjectTreeSelection::default();
        selection.select("src/a.rs", &visible(), false, false);
        assert_eq!(selection.paths().len(), 1);
        assert!(selection.paths().contains("src/a.rs"));
        assert_eq!(selection.anchor(), Some("src/a.rs"));
        assert_eq!(selection.focused(), Some("src/a.rs"));
        // 再点另一条：仍然只有一条。
        selection.select("src/b.rs", &visible(), false, false);
        assert_eq!(selection.paths().len(), 1);
        assert!(selection.paths().contains("src/b.rs"));
    }

    /// ⌘：切换该条的存在性，锚点跟着走。
    #[test]
    fn command_click_toggles_one_row_and_moves_the_anchor() {
        let mut selection = ProjectTreeSelection::default();
        selection.select("src", &visible(), false, false);
        selection.select("README.md", &visible(), false, true);
        assert_eq!(selection.paths().len(), 2);
        assert_eq!(selection.anchor(), Some("README.md"));
        // 再 ⌘ 一次把它去掉。
        selection.select("README.md", &visible(), false, true);
        assert_eq!(selection.paths().len(), 1);
        assert!(!selection.paths().contains("README.md"));
    }

    /// Shift：区间按**显示顺序**取，锚点不动。含"从下往上选"的逆序区间。
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

        // 逆序：锚点在后面、目标是前面。
        let mut reversed = ProjectTreeSelection::default();
        reversed.select("README.md", &visible(), false, false);
        reversed.select("tests", &visible(), true, false);
        assert_eq!(
            reversed.paths().iter().cloned().collect::<Vec<_>>(),
            vec!["README.md", "tests"]
        );
    }

    /// Shift+⌘：与已有集**求并**，不是替换。
    #[test]
    fn shift_command_click_unions_with_the_existing_selection() {
        let mut selection = ProjectTreeSelection::default();
        selection.select("README.md", &visible(), false, false);
        selection.select("src", &visible(), false, true);
        selection.select("src/b.rs", &visible(), true, true);
        // README.md 仍在（并集），区间 src..src/b.rs 也进来了。
        assert_eq!(
            selection.paths().iter().cloned().collect::<Vec<_>>(),
            vec!["README.md", "src", "src/a.rs", "src/b.rs"]
        );
    }

    /// `covers`：选中文件夹连带子孙；同级不算、兄弟前缀不算。
    #[test]
    fn a_selected_folder_covers_its_descendants_only() {
        let mut selection = ProjectTreeSelection::default();
        selection.select("src", &visible(), false, false);
        assert!(selection.covers("src"));
        assert!(selection.covers("src/a.rs"));
        assert!(selection.covers("src/deep/nested/File.java"));
        assert!(!selection.covers("tests"));
        assert!(!selection.covers("README.md"));
        // 前缀像但不同级：`src2` 不是 `src` 的子孙。
        assert!(!selection.covers("src2/a.rs"));
    }

    /// 右键：已选中的行保留当前选择；多选且被 `covers` 也保留；否则改成只选该行。
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

    /// 折叠 / 刷新：选择集与可见集求交，锚点与焦点不可见就清掉。
    #[test]
    fn retain_drops_paths_that_are_no_longer_visible() {
        let mut selection = ProjectTreeSelection::default();
        selection.select("src", &visible(), false, false);
        selection.select("src/a.rs", &visible(), false, true);
        // 折叠 `src` 之后只剩顶层可见。
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

        // 只选了"随折叠消失的子项"：retain 之后选择集清空、锚点与焦点都没了。
        let mut hidden = ProjectTreeSelection::default();
        hidden.select("src/a.rs", &visible(), false, false);
        hidden.retain(&collapsed);
        assert!(hidden.paths().is_empty());
        assert_eq!(hidden.anchor(), None);
        assert_eq!(hidden.focused(), None);
    }

    /// ⌘A：选中的是**同级兄弟**（本侧由调用方算好兄弟表），锚点落在第一个，
    /// 焦点不在集合里就移到第一个。
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

        // 焦点不在兄弟集合里时移到第一个。
        let mut elsewhere = ProjectTreeSelection::default();
        elsewhere.select("README.md", &visible(), false, false);
        elsewhere.select_all_in_siblings(&siblings);
        assert_eq!(elsewhere.focused(), Some("src/a.rs"));

        // 空兄弟表：什么都不做（真源 `guard let first` 同）。
        let mut empty = ProjectTreeSelection::default();
        empty.select("README.md", &visible(), false, false);
        empty.select_all_in_siblings(&[]);
        assert_eq!(empty.paths().len(), 1);
    }
}
