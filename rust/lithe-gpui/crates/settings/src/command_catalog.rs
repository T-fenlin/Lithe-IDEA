//! 命令目录：`CommandId` → 展示文案（i18n key）。
//!
//! ## 为什么要有这一层
//!
//! `CommandId`（`crates/shared/src/commands.rs`）**只有 id，没有标题** —— 那个文件的头注释
//! 写得很清楚：这层不带 i18n 文案和图标，搬进来会把 i18n 与渲染拖进 `shared`。
//!
//! 而 `MenuAction::label_key()` 那套文案在 `workbench` 里，`MenuAction::command_id()` 是
//! **`MenuAction → CommandId`** 方向，**反查不出文案**。加上 `workbench` 依赖 `settings`，
//! `settings` 不能反过来依赖 `workbench`（会成环）。
//!
//! 所以目录只能人工建、放在 `settings` 这一层：
//! - `shared` 保持只有 id 的纯净形态；
//! - `settings` 的 keymap 页面能用；
//! - `workbench` 依赖 `settings`，也能用。
//!
//! ## 文案从哪来
//!
//! **一律复用既有 i18n key，不新造。** 22 条直接取自对应 `MenuAction::label_key()` 的取值
//! （两者语义同一个命令），只有 `OpenSettings` / `SwitchToLightTheme` / `SwitchToDarkTheme`
//! 这 3 条在 `MenuAction` 里没有对应项，各取一个既有 key：
//!
//! - `OpenSettings` → `lithe.menu.preferences`（`MenuAction::Preferences` 的文案；该动作的
//!   `command_id()` 是 `None`，但语义就是"打开设置"，且 `OpenSettings` 这个 gpui action
//!   确实存在并已绑 `secondary-,`）。
//! - 两个切主题 → `lithe.appearance.gpui.switchTheme{Light,Dark}`。这两个 key 在
//!   `locales/` 里**早已存在但代码从未引用过**，正是留给这个用途的。
//!
//! `lithe.` 前缀不写在这一层：`tr()` 自己会加。
//!
//! ## 「死行」这个词别再用
//!
//! 早期的说法是「23 个 id 里有 10 个死行，点了没反应」。**实测不成立**：`ShellWorkspace::
//! run_command_id`（`workbench/src/workspace.rs`）对 23 个 id **全部**有实现，命令面板里
//! 点哪条都有反应。
//!
//! 真正的区别是**能不能绑键盘快捷键**——那要有一个 gpui `Action` 类型，而 `MenuAction::
//! keybind_action()` 对其中一部分返回 `None`。这是「能否绑键」，不是「能否运行」，两者
//! 别混。本目录因此**不记** `wired` 标记：那是 `workbench` 侧的知识，写死在这会漂移。

use lithe_gpui_shared::commands::CommandId;

/// 命令在界面上的名字，取 i18n key（**不含** `lithe.` 前缀，`tr()` 自己会加）。
pub const fn label_key(command: CommandId) -> &'static str {
    match command {
        CommandId::OpenSettings => "menu.preferences",
        CommandId::OpenAppearanceSettings => "settings.tabs.appearance",
        CommandId::SwitchToLightTheme => "appearance.gpui.switchThemeLight",
        CommandId::SwitchToDarkTheme => "appearance.gpui.switchThemeDark",
        CommandId::ToggleTerminal => "menu.toggleTerminal",
        CommandId::ToggleMaven => "workbench.maven",
        CommandId::ToggleStatusBar => "settings.appearance.showStatusBar",
        CommandId::ToggleMenuBar => "menu.toggleMenuBar",
        CommandId::SaveBuffer => "menu.save",
        CommandId::OpenCommandPalette => "menu.commandPalette",
        CommandId::OpenQuickOpen => "menu.quickOpen",
        CommandId::OpenSearchEverywhere => "menu.globalSearch",
        CommandId::OpenReplaceInFiles => "search.gpui.replaceInFiles",
        CommandId::NavigateToDefinition => "menu.goToDefinition",
        CommandId::NavigateToImplementation => "menu.goToImplementation",
        CommandId::NavigateToTypeDefinition => "menu.goToTypeDefinition",
        CommandId::OpenFile => "outline.openFile",
        CommandId::CloseTab => "menu.closeTab",
        CommandId::CloseOtherTabs => "menu.closeOtherTabs",
        CommandId::CloseAllTabs => "menu.closeAllTabs",
        CommandId::CloseSavedTabs => "menu.closeSavedTabs",
        CommandId::CloseTabsToLeft => "menu.closeTabsToLeft",
        CommandId::CloseTabsToRight => "menu.closeTabsToRight",
        CommandId::ReopenClosedTab => "menu.reopenClosedTab",
        CommandId::NewWindow => "menu.newWindow",
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandId, label_key};

    /// 运行时读 i18n 文件，不 `include_str!`：两个 yml 各约 290 KB，嵌进测试二进制不划算。
    fn locale(name: &str) -> String {
        let path = format!("{}/../shared/locales/{}", env!("CARGO_MANIFEST_DIR"), name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("读不了 i18n 文件 {path}：{error}"))
    }

    /// key 在 yml 里的落法：`lithe.` 是根，其余是**扁平的完整路径**，
    /// 所以 `menu.save` 对应两空格缩进的 `  menu.save:`。
    fn has_key(source: &str, key: &str) -> bool {
        source
            .lines()
            .any(|line| line.trim_end() == format!("  {key}:").as_str())
    }

    #[test]
    fn every_command_has_a_label_and_the_label_exists_in_both_locales() {
        let zh = locale("lithe.zh-CN.yml");
        let en = locale("lithe.en.yml");
        for command in CommandId::ALL {
            let key = label_key(command);
            assert!(!key.is_empty(), "{command:?} 的文案是空的");
            assert!(
                !key.starts_with("lithe."),
                "{command:?} 的 key 不该带 `lithe.` 前缀（tr() 会加）：{key}"
            );
            assert!(
                has_key(&zh, key),
                "中文档案里没有 {command:?} 的 key：{key}"
            );
            assert!(
                has_key(&en, key),
                "英文档案里没有 {command:?} 的 key：{key}"
            );
        }
    }

    /// 两条命令共用一个文案，用户在快捷键页里就分不清谁是谁。
    #[test]
    fn no_two_commands_share_a_label() {
        let mut seen = std::collections::HashMap::new();
        for command in CommandId::ALL {
            let key = label_key(command);
            if let Some(first) = seen.insert(key, command) {
                panic!("{command:?} 与 {first:?} 共用文案 {key}");
            }
        }
        assert_eq!(seen.len(), CommandId::ALL.len());
    }

    /// 目录必须跟着 `CommandId` 一起长：加一个新变体而这里没登记，上面两条测试立刻会挂。
    #[test]
    fn the_catalog_covers_every_command() {
        assert_eq!(CommandId::ALL.len(), 25);
        for command in CommandId::ALL {
            // `label_key` 是穷举 match，落到这里即证明没漏；再断言一次非空。
            assert!(!label_key(command).is_empty());
        }
    }
}
