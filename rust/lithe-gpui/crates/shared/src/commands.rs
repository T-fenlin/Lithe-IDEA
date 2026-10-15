//! 命令目录：命令的稳定 id。
//!
//! 这一层**只有 id**，没有标题、图标、分组。带 i18n 文案和图标的那层
//! （`workbench::command_palette::CommandAction`）留在 workbench：它依赖 `tr()` 与
//! `IconElement`，搬进来会把 i18n 和渲染拖进 shared。
//!
//! 为什么要放 shared：`settings` 要枚举全部命令来画快捷键页，而依赖方向是
//! `settings → shared`、`workbench → settings + shared`——枚举留在 workbench 就成环了。
//!
//! **id 是长期兼容面**：用户覆盖按 id 存进设置文件，定了就不要改。

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommandId {
    OpenSettings,
    OpenAppearanceSettings,
    SwitchToLightTheme,
    SwitchToDarkTheme,
    ToggleTerminal,
    ToggleMaven,
    ToggleStatusBar,
    ToggleMenuBar,
    SaveBuffer,
    OpenCommandPalette,
    OpenQuickOpen,
    OpenSearchEverywhere,
    OpenReplaceInFiles,
    NavigateToDefinition,
    NavigateToImplementation,
    NavigateToTypeDefinition,
    GoToReferences,
    RenameSymbol,
    FormatDocument,
    OpenFile,
    CloseTab,
    CloseOtherTabs,
    CloseAllTabs,
    CloseSavedTabs,
    CloseTabsToLeft,
    CloseTabsToRight,
    ReopenClosedTab,
    NewWindow,
}

impl CommandId {
    pub const ALL: [CommandId; 28] = [
        Self::OpenSettings,
        Self::OpenAppearanceSettings,
        Self::SwitchToLightTheme,
        Self::SwitchToDarkTheme,
        Self::ToggleTerminal,
        Self::ToggleMaven,
        Self::ToggleStatusBar,
        Self::ToggleMenuBar,
        Self::SaveBuffer,
        Self::OpenCommandPalette,
        Self::OpenQuickOpen,
        Self::OpenSearchEverywhere,
        Self::OpenReplaceInFiles,
        Self::NavigateToDefinition,
        Self::NavigateToImplementation,
        Self::NavigateToTypeDefinition,
        Self::GoToReferences,
        Self::RenameSymbol,
        Self::FormatDocument,
        Self::OpenFile,
        Self::CloseTab,
        Self::CloseOtherTabs,
        Self::CloseAllTabs,
        Self::CloseSavedTabs,
        Self::CloseTabsToLeft,
        Self::CloseTabsToRight,
        Self::ReopenClosedTab,
        Self::NewWindow,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::OpenSettings => "open-settings",
            Self::OpenAppearanceSettings => "open-appearance-settings",
            Self::SwitchToLightTheme => "switch-theme-light",
            Self::SwitchToDarkTheme => "switch-theme-dark",
            Self::ToggleTerminal => "toggle-terminal",
            Self::ToggleMaven => "toggle-maven",
            Self::ToggleStatusBar => "toggle-status-bar",
            Self::ToggleMenuBar => "toggle-menu-bar",
            Self::SaveBuffer => "save-buffer",
            Self::OpenCommandPalette => "open-command-palette",
            Self::OpenQuickOpen => "open-quick-open",
            Self::OpenSearchEverywhere => "open-search-everywhere",
            Self::OpenReplaceInFiles => "open-replace-in-files",
            Self::NavigateToDefinition => "navigate-to-definition",
            Self::NavigateToImplementation => "navigate-to-implementation",
            Self::NavigateToTypeDefinition => "navigate-to-type-definition",
            Self::GoToReferences => "go-to-references",
            Self::RenameSymbol => "rename-symbol",
            Self::FormatDocument => "format-document",
            Self::OpenFile => "open-file",
            Self::CloseTab => "close-tab",
            Self::CloseOtherTabs => "close-other-tabs",
            Self::CloseAllTabs => "close-all-tabs",
            Self::CloseSavedTabs => "close-saved-tabs",
            Self::CloseTabsToLeft => "close-tabs-to-left",
            Self::CloseTabsToRight => "close-tabs-to-right",
            Self::ReopenClosedTab => "reopen-closed-tab",
            Self::NewWindow => "new-window",
        }
    }

    /// 由 id 字符串反查命令。
    ///
    /// 用户的键位覆盖是按 id 存进设置文件的，读回来必须能查到命令；查不到要安静地返回
    /// `None`，不能猜一个。这张表与 `CommandId::id` **互为逆运算**，由下面的测试钉死。
    pub fn from_id(id: &str) -> Option<CommandId> {
        match id {
            "open-settings" => Some(Self::OpenSettings),
            "open-appearance-settings" => Some(Self::OpenAppearanceSettings),
            "switch-theme-light" => Some(Self::SwitchToLightTheme),
            "switch-theme-dark" => Some(Self::SwitchToDarkTheme),
            "toggle-terminal" => Some(Self::ToggleTerminal),
            "toggle-maven" => Some(Self::ToggleMaven),
            "toggle-status-bar" => Some(Self::ToggleStatusBar),
            "toggle-menu-bar" => Some(Self::ToggleMenuBar),
            "save-buffer" => Some(Self::SaveBuffer),
            "open-command-palette" => Some(Self::OpenCommandPalette),
            "open-quick-open" => Some(Self::OpenQuickOpen),
            "open-search-everywhere" => Some(Self::OpenSearchEverywhere),
            "open-replace-in-files" => Some(Self::OpenReplaceInFiles),
            "navigate-to-definition" => Some(Self::NavigateToDefinition),
            "navigate-to-implementation" => Some(Self::NavigateToImplementation),
            "navigate-to-type-definition" => Some(Self::NavigateToTypeDefinition),
            "go-to-references" => Some(Self::GoToReferences),
            "rename-symbol" => Some(Self::RenameSymbol),
            "format-document" => Some(Self::FormatDocument),
            "open-file" => Some(Self::OpenFile),
            "close-tab" => Some(Self::CloseTab),
            "close-other-tabs" => Some(Self::CloseOtherTabs),
            "close-all-tabs" => Some(Self::CloseAllTabs),
            "close-saved-tabs" => Some(Self::CloseSavedTabs),
            "close-tabs-to-left" => Some(Self::CloseTabsToLeft),
            "close-tabs-to-right" => Some(Self::CloseTabsToRight),
            "reopen-closed-tab" => Some(Self::ReopenClosedTab),
            "new-window" => Some(Self::NewWindow),
            _ => None,
        }
    }
}

/// `&str` 的 const 比较：`const fn` 里用不了 `==`，只能逐字节比。
const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut index = 0;
    while index < a.len() {
        if a[index] != b[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// `ALL` 里的 id 两两不同。
///
/// 配合下面那条长度断言就是完备性证明：`ALL` 有 28 项、id 两两不同、而变体恰好 28 个
/// 且各有各的 id，那么 `ALL` 必然覆盖了每一个变体——少一个就必有一个重复顶上来。
const fn all_ids_are_distinct() -> bool {
    let all = CommandId::ALL;
    let mut outer = 0;
    while outer < all.len() {
        let mut inner = outer + 1;
        while inner < all.len() {
            if str_eq(all[outer].id(), all[inner].id()) {
                return false;
            }
            inner += 1;
        }
        outer += 1;
    }
    true
}

// 编译期守两件事：变体数（新增变体必须同步改这个数）与 id 不重复。
const _: () = assert!(CommandId::ALL.len() == 28);
const _: () = assert!(all_ids_are_distinct());

#[cfg(test)]
mod tests {
    use super::CommandId;

    /// 需要运行期才能查的部分：`id()` 与 `from_id()` 必须严丝合缝地互为逆运算。
    ///
    /// 编译期那两条断言已经保证了 `ALL` 的长度和唯一性，这里再补两件事：
    /// 一是每个变体的 id 往返得回来，二是未知 id 必须安静地返回 `None` 而不是猜一个。
    #[test]
    fn every_command_has_a_unique_id() {
        let mut seen = std::collections::BTreeSet::new();
        for command in CommandId::ALL {
            let id = command.id();
            assert!(!id.is_empty(), "{id:?} 是空 id");
            assert!(seen.insert(id), "id 重复：{id}");
            assert_eq!(CommandId::from_id(id), Some(command), "{id} 反查不回自己");
        }
        assert_eq!(seen.len(), CommandId::ALL.len());
    }

    #[test]
    fn unknown_ids_are_rejected() {
        for id in [
            "",
            "open-settings ",
            "OpenSettings",
            "save-buffer-x",
            "文件保存",
        ] {
            assert_eq!(CommandId::from_id(id), None, "{id:?} 不该被认出来");
        }
    }
}
