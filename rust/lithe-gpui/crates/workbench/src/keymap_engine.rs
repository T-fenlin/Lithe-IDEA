//! 重绑引擎：把 `settings::keymap` 解析出来的键位真正写回 gpui 的 `Keymap`。
//!
//! ## 为什么要有这一层
//!
//! `settings::keymap` 只回答「某条命令在当前预设下应该是哪串键」，它不碰 gpui。真按下键时
//! 走的是 gpui 的 `Keymap`，而那份 keymap 是各处 `install_actions` 里 `bind_keys` 累积出来
//! 的。用户改了键位，后者必须跟着变——这一层就是那个「跟着变」。
//!
//! ## 怎么变：快照 + 剔除 + 整批重放
//!
//! gpui 的 `Keymap` 只有 `add_bindings` 与 `clear`，**没有「删某一条」**。所以每次重放都是
//! 整批重建：
//!
//! 1. 快照：把当前全部绑定抄一份，剔除本引擎托管的那 11 条命令的绑定，剩下的记为
//!    `baseline_other`；
//! 2. 重放：`clear_key_bindings()` → `bind_keys(baseline_other + 生效的 11 条)`。
//!
//! `Keymap::clear()` 连 `binding_indices_by_action_id` 与 `disabled_binding_indices` 一起清
//! （`gpui-pre-0.3.8/src/keymap.rs:76`），所以整批重建是干净的，不会残留上一次的禁用标记。
//!
//! ## 快照点必须在窗口构造之后
//!
//! 托管的 11 条里有 6 条（`SaveBuffer`、`NavigateToDefinition`、`ToggleMenuBar`、
//! `OpenCommandPalette`、`OpenQuickOpen`、`OpenReplaceInFiles`）是 `ShellWorkspace::new` 里
//! 那 6 个 `install_actions` 装的（`workspace.rs:595-600`），**晚于 `main.rs` 的启动段**。
//! 快照若早于它就会漏掉这 6 条，第一次重放会把保存、命令面板、快速打开这些键位全清掉。
//! 所以快照点定在 `ShellWorkspace::new` 末尾，而不是 `main.rs`。
//!
//! 第二、第三个窗口再跑一次 `ShellWorkspace::new` 无害：它装的是同一批绑定，快照里已经有
//! 了，重放后反而从「两份」变「一份」。快照只做一次。
//!
//! ## 空键位 = 一条都不 add，不用 `Unbind`
//!
//! `gpui::Unbind(SharedString)` 的语义是「把某个 action 在某键位上的绑定标记为禁用」，前提
//! 是**那条绑定还在表里**；而这里是把托管绑定整个剔出基线，空覆盖就是**一条都不 add**，
//! `bindings_for_action` 直接拿不到——这才是「真解绑」。`NoAction` 是另一回事（吞掉按键），
//! 两者不能混。

use std::rc::Rc;

use gpui::{App, DummyKeyboardMapper, Global, KeyBinding, KeyBindingContextPredicate};
use lithe_gpui_shared::commands::CommandId;

use crate::menu_bar::MenuAction;

/// 本引擎托管的命令 → 产出它 action 的那个 `MenuAction`。
///
/// ## 为什么是「薄表」：只写枚举变体对，一个 action 类型都不写
///
/// action 类型全仓只有一处，就是 `MenuAction::keybind_action()`。这里再抄一遍
/// `Box::new(lithe_gpui_editor::SaveBuffer)` 就是第二份，必然漂移。所以本表只记
/// `CommandId ↔ MenuAction` 这对枚举变体，action 由 `keybind_action()` 派生。
///
/// 附带好处：菜单上的键位胶囊也走 `keybind_action()` + `highest_precedence_binding_for_action`
/// （见 `menu_bar.rs:473` 的注释），所以「菜单上显示的键位」与「真按得出的键位」物理上不可能
/// 不一致。
///
/// ## 为什么必须人工建：`CommandId → Action` 全仓不存在
///
/// - `CommandId` 只有 `id()` / `from_id()`（`shared/src/commands.rs:39-126`）；
/// - `ShellWorkspace::run_command_id`（`workspace.rs:2011`）直接调宿主方法，手里没有 action；
/// - `MenuAction::keybind_action()` 与 `MenuAction::command_id()` 都**从 `MenuAction` 出发**，
///   没有反向；
/// - `menu_bar::lookup_action`（`menu_bar.rs:1364`）匹配的是 i18n 文案键，不是 `CommandId`；
/// - `MenuAction` 不可枚举（没有 `MenuAction::ALL`，全仓无 `strum` / `EnumIter`），
///   所以连「遍历反推」都做不到。
pub fn managed_menu_action(command: CommandId) -> Option<MenuAction> {
    Some(match command {
        // 唯一的刻意例外：`MenuAction::Preferences.command_id()` 是 `None`
        // （`menu_bar.rs:561-564`），这一对反推不出来，只能手写。
        //
        // 它不是漏登记：`apply_menu_action` 开头若查到 `command_id()` 就委托并 `return`
        // （`workspace.rs:1822-1826`），而 `Preferences` 有自己的本地分支
        // （`workspace.rs:1844`）——两者同时存在时本地分支就是死代码。仓库规则是「有本地
        // 分支就不给 CommandId」，`Preferences` 因此被列进 `NON_COMMAND` 白名单
        // （`menu_bar.rs:2337-2369`）。别把它"修好"。
        CommandId::OpenSettings => MenuAction::Preferences,
        CommandId::SaveBuffer => MenuAction::Save,
        CommandId::OpenCommandPalette => MenuAction::CommandPalette,
        CommandId::OpenQuickOpen => MenuAction::QuickOpen,
        CommandId::OpenReplaceInFiles => MenuAction::ReplaceInFiles,
        CommandId::NavigateToDefinition => MenuAction::GoToDefinition,
        CommandId::NavigateToImplementation => MenuAction::GoToImplementation,
        CommandId::NavigateToTypeDefinition => MenuAction::GoToTypeDefinition,
        CommandId::ReopenClosedTab => MenuAction::ReopenClosedTab,
        CommandId::ToggleTerminal => MenuAction::ToggleTerminal,
        CommandId::ToggleMenuBar => MenuAction::ToggleMenuBar,
        _ => return None,
    })
}

/// 托管命令的 action 名，快照剔除时拿它做判据。
fn managed_action_names() -> Vec<&'static str> {
    CommandId::ALL
        .iter()
        .filter_map(|command| {
            managed_menu_action(*command)
                .and_then(MenuAction::keybind_action)
                .map(|action| action.name())
        })
        .collect()
}

/// 引擎状态。挂在 gpui 的 `Global` 上，所以是**每 App 一份**，不会跨测试泄漏。
#[derive(Default)]
pub struct KeymapEngine {
    /// 剔除托管绑定之后的基线。
    baseline_other: Vec<KeyBinding>,
    /// 快照只做一次。
    snapshotted: bool,
    /// 上一次真正重放用的设置（已去重），用来短路。
    applied: Option<lithe_gpui_settings::KeymapSettings>,
}

/// `Global` 要求 `'static`，`KeymapEngine` 满足；包一层只为避免把 `Global` 实现到
/// 公开类型上、让外部也能 `global_mut` 改它。
#[derive(Default)]
struct EngineGlobal(KeymapEngine);

impl Global for EngineGlobal {}

/// 把所有绑定抄一份，剔除本引擎托管的那 11 条命令的绑定。
fn capture_baseline(cx: &App) -> Vec<KeyBinding> {
    let managed = managed_action_names();
    let keymap = cx.key_bindings();
    let bindings = keymap.borrow();
    let total = bindings.bindings().len();
    let other: Vec<KeyBinding> = bindings
        .bindings()
        .filter(|binding| !managed.contains(&binding.action().name()))
        .cloned()
        .collect();
    // 这行是生产路径唯一的自检手段：快照时机错了（跑在某个 `install_actions` 之前）会漏掉
    // 一批绑定，而单测测不出来——单测只认它自己装的那批。`total` 应该是几十条量级，
    // 个位数甚至 0 就说明快照跑早了。
    eprintln!(
        "S1_KEYMAP snapshot total={total} managed={} other={}",
        total.saturating_sub(other.len()),
        other.len()
    );
    other
}

/// 由设置合成生效的绑定。**空键位一律不 add** —— 那就是「真解绑」。
fn effective_bindings(settings: &lithe_gpui_settings::KeymapSettings) -> Vec<KeyBinding> {
    let mut out = Vec::new();
    for command in CommandId::ALL {
        let resolved = settings.resolve(command);
        if !resolved.is_bound() {
            continue;
        }
        let Some(action) = managed_menu_action(command).and_then(MenuAction::keybind_action) else {
            // 这条命令没有可派发的 action（如 `OpenSearchEverywhere`），绑不了也就不绑，
            // 不拿别的动作顶替。
            continue;
        };
        // context 谓词必须原样带回，否则带 context 的键位会退化成全窗绑定。
        let predicate = match resolved.context {
            Some(source) => match KeyBindingContextPredicate::parse(source) {
                Ok(parsed) => Some(Rc::new(parsed)),
                Err(error) => {
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "S1_KEYMAP predicate_failed command={} context={source} error={error}",
                        command.id()
                    );
                    continue;
                }
            },
            None => None,
        };
        for keystrokes in &resolved.keystrokes {
            // `load` 是唯一接受 `Box<dyn Action>` 的入口：`new` 要具体类型，而这里只有
            // `keybind_action()` 给的 trait object。
            match KeyBinding::load(
                keystrokes,
                action.boxed_clone(),
                predicate.clone(),
                false,
                None,
                &DummyKeyboardMapper,
            ) {
                Ok(binding) => out.push(binding),
                Err(error) => {
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "S1_KEYMAP keystroke_failed command={} keystrokes={keystrokes} error={error}",
                        command.id()
                    );
                }
            }
        }
    }
    out
}

/// 把 `settings` 写回 gpui 的 `Keymap`。
///
/// 首次调用顺带快照；之后每次都整批重放。**设置没变就直接返回**——`bind_keys` 每次都会推
/// `Effect::RefreshWindows`，没必要在改字号、换主题这种高频改动上也重放一遍。
pub fn apply(settings: &lithe_gpui_settings::KeymapSettings, cx: &mut App) {
    // 指纹用**去重后**的 overrides：`commit_global` 会跑 `dedup_overrides`（排序 + 后写的
    // 赢），原始 Vec 顺序变了但语义没变时不该触发重放。
    let mut fingerprint = settings.clone();
    fingerprint.dedup_overrides();

    // `capture_baseline` 要借 `cx`，不能和下面的 `default_global` 可变借用叠在一起。
    let captured = if cx.default_global::<EngineGlobal>().0.snapshotted {
        None
    } else {
        Some(capture_baseline(cx))
    };

    let replay = {
        let engine = &mut cx.default_global::<EngineGlobal>().0;
        if let Some(baseline) = captured {
            engine.baseline_other = baseline;
            engine.snapshotted = true;
        }
        if engine.applied.as_ref() == Some(&fingerprint) {
            false
        } else {
            engine.applied = Some(fingerprint.clone());
            true
        }
    };
    if !replay {
        return;
    }

    let mut all = cx.default_global::<EngineGlobal>().0.baseline_other.clone();
    all.extend(effective_bindings(&fingerprint));
    cx.clear_key_bindings();
    cx.bind_keys(all);
}

/// 从设置仓库取键位设置并重放。没有仓库（组合根还没建 / 无头测试）就什么都不做。
///
/// 装好对仓库的订阅之后要**主动调一次**：`observe` 只在变化时触发，而启动时用户可能已经
/// 存过覆盖，那时 keymap 还是基线默认值。
/// 把「这条命令能不能绑键」的判定注给 `settings` 的键位层。
///
/// 判定依据是薄表 + `keybind_action()` 有没有值——那是 workbench 的知识，`settings` 不能
/// 反向依赖，所以走宿主注入（同 `lithe_gpui_settings::set_git_identity_host`）。设置页拿它
/// 决定哪几行要灰掉。
///
/// 幂等，重复调用只是换掉同一个解析器；但**必须调**——不调的话 `is_bindable` 一律 false，
/// 整页变灰。
pub fn register_bindable_resolver() {
    lithe_gpui_settings::set_bindable_resolver(Some(std::rc::Rc::new(|command| {
        managed_menu_action(command)
            .and_then(MenuAction::keybind_action)
            .is_some()
    })));
}

pub fn sync(cx: &mut App) {
    let Some(store) = lithe_gpui_settings::try_store(cx) else {
        return;
    };
    let settings = store.read(cx).keymap().clone();
    apply(&settings, cx);
}

#[cfg(test)]
mod tests {
    use super::{EngineGlobal, managed_action_names, managed_menu_action};
    use gpui::{KeyBinding, TestAppContext};
    use lithe_gpui_settings::{KeyOverride, KeymapPreset, KeymapSettings};
    use lithe_gpui_shared::commands::CommandId;

    use crate::menu_bar::MenuAction;

    /// lithe-ui 编辑器键位的 context。生产侧是 `lithe-ui/src/editor/keys.rs` 的
    /// `pub(crate) const CONTEXT`，这里照抄字面量。
    const ELY_EDITOR: &str = "ElyEditor";

    /// 一条绑定的可比形状：action 名 + 键位 + context 谓词。
    ///
    /// `KeyBinding` 的字段全是 `pub(crate)`，外部只能走 `action()` / `keystrokes()` /
    /// `predicate()`，所以逐元素比较只能这么比。
    fn shapes(cx: &gpui::App) -> Vec<(String, String, String)> {
        let keymap = cx.key_bindings();
        let bindings = keymap.borrow();
        bindings
            .bindings()
            .map(|binding| {
                (
                    binding.action().name().to_string(),
                    format!("{:?}", binding.keystrokes()),
                    binding
                        .predicate()
                        .map(|predicate| predicate.to_string())
                        .unwrap_or_default(),
                )
            })
            .collect()
    }

    fn names(cx: &gpui::App) -> Vec<String> {
        let keymap = cx.key_bindings();
        let bindings = keymap.borrow();
        bindings
            .bindings()
            .map(|binding| binding.action().name().to_string())
            .collect()
    }

    /// 装一批 lithe-ui 的编辑器键位。
    ///
    /// 生产侧由 `lithe_ui::init`（`lithe-ui/src/lib.rs:58`）装载，但它第一件事就校验资产源，
    /// 无头测试里必然 panic（gpui 的测试上下文把 `asset_source` 设成 `Arc::new(())`，见
    /// `gpui-pre-0.3.8/src/app/test_context.rs:171`）。所以这里用**真实的动作类型 + 真实的
    /// context 字面量**自己装——与生产的差别只有「谁调的 `bind_keys`」。
    fn install_lithe_ui_keys(cx: &mut gpui::App) {
        use lithe_ui::editor::{Copy, Cut, Paste, Redo, SelectAll, Undo};
        cx.bind_keys([
            KeyBinding::new("secondary-z", Undo, Some(ELY_EDITOR)),
            KeyBinding::new("secondary-y", Redo, Some(ELY_EDITOR)),
            KeyBinding::new("secondary-c", Copy, Some(ELY_EDITOR)),
            KeyBinding::new("secondary-v", Paste, Some(ELY_EDITOR)),
            KeyBinding::new("secondary-x", Cut, Some(ELY_EDITOR)),
            KeyBinding::new("secondary-a", SelectAll, Some(ELY_EDITOR)),
        ]);
    }

    /// 托管集合之外的绑定，一条都不能少。
    #[gpui::test]
    fn bindings_outside_the_managed_commands_survive_a_replay(cx: &mut TestAppContext) {
        cx.update(|cx| {
            install_lithe_ui_keys(cx);

            super::apply(&KeymapSettings::default(), cx);

            // 非空断言：基线里真有这批绑定，否则下面那条「都还在」会空过。
            let before: Vec<(String, String, String)> = shapes(cx)
                .into_iter()
                .filter(|(action, _, _)| action.starts_with("ely_editor::"))
                .collect();
            assert_eq!(before.len(), 6, "基线里应当有 6 条 lithe-ui 绑定");

            // 换预设 + 加覆盖，逼出一次真正的重放。
            let mut changed = KeymapSettings::new(KeymapPreset::IdeaClassic);
            changed.overrides.push(KeyOverride::new(
                KeymapPreset::IdeaClassic,
                CommandId::SaveBuffer,
                vec!["f2".to_string()],
            ));
            super::apply(&changed, cx);

            let after: Vec<(String, String, String)> = shapes(cx)
                .into_iter()
                .filter(|(action, _, _)| action.starts_with("ely_editor::"))
                .collect();
            assert_eq!(after, before, "重放之后 lithe-ui 的绑定一条都不能少");
        });
    }

    /// 同一份设置重放两次，逐元素完全一样。
    #[gpui::test]
    fn replaying_is_idempotent_element_by_element(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let first = KeymapSettings::default();
            super::apply(&first, cx);
            let after_first = shapes(cx);
            assert!(!after_first.is_empty(), "默认档应当装出一批绑定");

            // 中间插一次别的重放，否则「指纹没变直接返回」会让这条测试空过。
            let mut other = KeymapSettings::new(KeymapPreset::IdeaClassic);
            other.overrides.push(KeyOverride::new(
                KeymapPreset::IdeaClassic,
                CommandId::SaveBuffer,
                vec!["f2".to_string()],
            ));
            super::apply(&other, cx);
            assert_ne!(shapes(cx), after_first, "换档之后 keymap 应当真的变了");

            super::apply(&first, cx);
            assert_eq!(
                shapes(cx),
                after_first,
                "同一份设置重放两次，逐元素必须完全一样"
            );
        });
    }

    /// 空键位 = 真解绑：keymap 里一条都不剩，且不是靠 `Unbind` 标记压住的。
    #[gpui::test]
    fn an_empty_override_really_unbinds_the_command(cx: &mut TestAppContext) {
        cx.update(|cx| {
            super::apply(&KeymapSettings::default(), cx);

            let save = managed_menu_action(CommandId::SaveBuffer)
                .and_then(MenuAction::keybind_action)
                .expect("SaveBuffer 必须有可派发的 action")
                .name();
            let reopen = managed_menu_action(CommandId::ReopenClosedTab)
                .and_then(MenuAction::keybind_action)
                .expect("ReopenClosedTab 必须有可派发的 action")
                .name();

            let bound = names(cx);
            assert_eq!(
                bound.iter().filter(|name| *name == save).count(),
                1,
                "默认档下 SaveBuffer 应当有键位"
            );

            let mut cleared = KeymapSettings::default();
            cleared.set_override(KeymapPreset::Lithe, CommandId::SaveBuffer, Vec::new());
            super::apply(&cleared, cx);

            let after = names(cx);
            assert_eq!(
                after.iter().filter(|name| *name == save).count(),
                0,
                "空键位必须是真解绑：keymap 里一条都不该剩"
            );
            assert!(
                after.iter().any(|name| name == reopen),
                "解绑一条不该牵连别的命令"
            );
        });
    }

    /// 薄表本身：每对全部自洽，且 `Preferences` 那对的例外是被白名单认可的，不是漏登记。
    #[test]
    fn the_managed_table_is_consistent_with_command_id() {
        let mut checked = 0;
        for command in CommandId::ALL {
            let Some(action) = managed_menu_action(command) else {
                continue;
            };
            checked += 1;

            let keybind = action
                .keybind_action()
                .unwrap_or_else(|| panic!("{command:?} 在表里，却拿不到 action"));

            match action.command_id() {
                Some(id) => assert_eq!(
                    id, command,
                    "表里 {command:?} ↔ {action:?} 与 `command_id()` 不一致"
                ),
                // 唯一的例外：`Preferences` 有本地执行分支，按仓库规则不该有 CommandId。
                None => assert_eq!(
                    (command, action),
                    (CommandId::OpenSettings, MenuAction::Preferences),
                    "只有 OpenSettings ↔ Preferences 允许 `command_id()` 为 None"
                ),
            }

            assert!(!keybind.name().is_empty(), "{command:?} 的 action 名是空的");
        }
        // 方向是**单向**的：`DEFAULT_BINDINGS` 里的每一条都必须被托管（否则重放会把它的
        // 键位当基线漏掉），但托管可以更宽 —— 「转到实现 / 转到类型定义」有真 Action、
        // 用户可绑键，只是出厂没有默认键位，因此不在默认表里。
        for row in lithe_gpui_settings::DEFAULT_BINDINGS {
            assert!(
                managed_menu_action(row.command).is_some(),
                "{:?} 有默认键位却没被托管：重放会漏掉它",
                row.command
            );
        }
        assert!(
            checked >= lithe_gpui_settings::DEFAULT_BINDINGS.len(),
            "托管表比默认表还小，说明有命令漏登记了"
        );
    }

    /// 托管的 action 名两两不同——重名会让「剔除托管绑定」误伤到别人的绑定。
    #[test]
    fn managed_action_names_are_distinct() {
        let names = managed_action_names();
        let mut seen = std::collections::BTreeSet::new();
        for name in &names {
            assert!(seen.insert(*name), "托管的 action 名重复：{name}");
        }
        assert_eq!(seen.len(), names.len());
    }

    /// `Preferences` 的 `command_id()` 为 None 是**白名单内的刻意例外**，不是漏登记。
    ///
    /// 这条钉住 `menu_bar.rs` 的 `NON_COMMAND`：一旦有人把它"修好"（给 Preferences 补上
    /// `Some(OpenSettings)`），`apply_menu_action` 里那条本地分支就变成死代码。
    #[test]
    fn the_open_settings_pair_is_a_declared_exception() {
        assert_eq!(
            managed_menu_action(CommandId::OpenSettings),
            Some(MenuAction::Preferences)
        );
        assert_eq!(
            MenuAction::Preferences.command_id(),
            None,
            "Preferences 的执行分支在 `apply_menu_action` 里，不该有 CommandId"
        );
        assert!(
            menu_bar_non_command_contains(MenuAction::Preferences),
            "Preferences 必须留在 NON_COMMAND 白名单里，否则它就是一条死项"
        );
    }

    /// `NON_COMMAND` 是 `menu_bar` 测试里的私有常量，这里按同样的成员复刻一份来查。
    fn menu_bar_non_command_contains(action: MenuAction) -> bool {
        const NON_COMMAND: [MenuAction; 36] = [
            MenuAction::OpenFolder,
            MenuAction::NewTerminalTab,
            MenuAction::CloseTerminalTab,
            MenuAction::Preferences,
            MenuAction::Minimize,
            MenuAction::Maximize,
            MenuAction::ToggleFullscreen,
            MenuAction::Undo,
            MenuAction::Redo,
            MenuAction::Cut,
            MenuAction::Copy,
            MenuAction::Paste,
            MenuAction::SelectAll,
            MenuAction::Find,
            MenuAction::FindAndReplace,
            MenuAction::QuickFix,
            MenuAction::ToggleActivitySidebar,
            MenuAction::ToggleSecondarySidebar,
            MenuAction::ShowDiagnostics,
            MenuAction::ShowFileExplorer,
            MenuAction::ShowSourceControl,
            MenuAction::ShowRunAndDebug,
            MenuAction::StartDebugging,
            MenuAction::StopDebugging,
            MenuAction::ToggleBreakpoint,
            MenuAction::GoToLine,
            MenuAction::ToggleWordWrap,
            MenuAction::ShowHover,
            MenuAction::ZoomIn,
            MenuAction::ZoomOut,
            MenuAction::ResetZoom,
            // 文件生命周期五个：执行分支在 `apply_menu_action`，都没有 `CommandId`。
            MenuAction::NewTab,
            MenuAction::NewFile,
            MenuAction::SaveAs,
            MenuAction::SaveAll,
            MenuAction::RevertFile,
        ];
        NON_COMMAND.contains(&action)
    }

    /// `Global` 按 App 隔离：两次 `apply` 之间状态不会串到别的 App 上。
    #[gpui::test]
    fn the_engine_state_is_per_app(cx: &mut TestAppContext) {
        cx.update(|cx| {
            assert!(
                cx.try_global::<EngineGlobal>().is_none(),
                "新 App 里不该已经有引擎状态"
            );
            super::apply(&KeymapSettings::default(), cx);
            assert!(
                cx.try_global::<EngineGlobal>().is_some(),
                "apply 之后引擎状态应当挂上"
            );
        });
    }
}
