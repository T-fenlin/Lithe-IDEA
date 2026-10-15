//! 键位数据层：预设、用户覆盖，以及「一条命令现在到底按哪个键」的合成。
//!
//! 范围**只有数据层**：设置页（画表格、录制按键）与重绑引擎（把结果写回 gpui 的
//! `Keymap`）都不在这里，属于二期。
//!
//! ## 四层合成
//!
//! 一条命令的有效键位按顺序取**第一层命中**，与 macOS 端
//! `LitheCommandCatalog.bindings(for:)` 同口径：
//!
//! 1. **当前预设下的用户覆盖**（[`KeySource::User`]）—— 含显式的空列表；
//! 2. **预设表**（[`KeySource::Preset`]）—— 只记「与默认不同」的那几条，所以 `Lithe`
//!    档是空表（它的键位就是默认值本身）；
//! 3. **注册表默认值**（[`KeySource::Default`]）—— [`DEFAULT_BINDINGS`]；
//! 4. 未绑定 —— 空列表。
//!
//! 空列表在第 1、2 两层是**有意义的值**（「这条命令在此预设下没有键位」），必须能被表达、
//! 不能被下一层回填。macOS 端 Eclipse 档里 `Find` 为空、不能回退成 `⌘F` 就是这条规则。
//!
//! ## 三条硬约束
//!
//! - **写 `secondary-`，不写 `cmd-`。** `gpui::Keystroke::parse` 把 `cmd` 解析成
//!   `modifiers.platform`，在 Windows 上就是 Win 键；`secondary` 才是 gpui 自己的平台
//!   归一化开关（macOS → `platform`，其他 → `control`）。`explorer` 那组 `cmd-a` / `ctrl-a`
//!   双写是历史形态（已在 B1 统一为 `secondary-a` / `secondary-c`），不抄。由
//!   `every_catalog_keystroke_parses_under_gpui` 与
//!   `no_keystroke_uses_the_cmd_dialect` 两条守着。
//! - **context 谓词原样带上。** 目录每行都留 [`PresetBinding::context`]，重绑时原样传回
//!   `KeyBinding::new`：组件库编辑器的键位是带 context `ElyEditor` 绑的，改成无 context 的
//!   全窗绑定会让 `secondary-z` 在编辑区之外也触发。今天这张表里九行都是全窗绑定（`None`），
//!   字段是为二期那条约束留的槽。
//! - **按 action 粒度整组替换。** 覆盖以「命令」为单位整体替换它的全部键位
//!   （[`KeyOverride::keystrokes`] 是列表），不留「一半能用一半不能用」的中间态。
//!
//! ## 已知缺口（登记，不掩盖）
//!
//! 三端命令 id 命名空间互不相同：gpui 侧 28 个 id 里只有 `toggle-terminal`、
//! `toggle-maven` 两个与 macOS 端同名，Windows 前端又是第三套（`file.save`、
//! `workbench.commandPalette`……）。预设表因此**无法从任何一端机械移植**，只能人工建显式
//! 映射。本轮只落「两端同名、且键位有真源」的条目：
//!
//! - [`IDEA_CLASSIC_BINDINGS`] 只有 `toggle-terminal` 一条：真源是
//!   `macos/Sources/Lithe/Models/Keymap/LitheCommandCatalog.swift` 的 `ideaClassicBindings`
//!   里的 `option-f12`，Windows 上 `option` 就是 `alt`。
//! - [`ECLIPSE_BINDINGS`] **空**：macOS 那两份 Eclipse 表（`eclipseBindings` /
//!   `eclipseParentBindings`）是 ⌘ 方言，Windows 上不成立，而 Windows 端没有对应的
//!   `Eclipse.xml` 真源。这是真源缺口，**不是**「Eclipse 没有键位」—— 没映射到的命令
//!   会落到第 3 层拿回默认值。
//!
//! 另：`toggle-maven` 虽与 macOS 端同名，但那边两份预设表里都没有它的条目，因此本轮
//! 不映射（没有键位可抄，不等于它可以不绑定）。

use serde::{Deserialize, Serialize};

use lithe_gpui_shared::Parsed;
use lithe_gpui_shared::commands::CommandId;

/// 键位设置文档的版本号（与 `shared::DOCUMENT_VERSION_KEY` 那一套对齐）。
pub const KEYMAP_SETTINGS_VERSION: u32 = 1;

/// 键位预设。
///
/// **不做 macOS 档**：⌘ 在 Windows 上没有意义。三个变体的对外 id 就是 serde 写进设置文件的
/// 字面量（`lithe` / `idea-classic` / `eclipse`），由 `preset_ids_match_the_serde_form` 钉死。
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum KeymapPreset {
    #[default]
    Lithe,
    IdeaClassic,
    Eclipse,
}

impl KeymapPreset {
    pub const ALL: [KeymapPreset; 3] = [Self::Lithe, Self::IdeaClassic, Self::Eclipse];

    /// 持久化用的 id。**与 serde 的 kebab-case 字面量同一个字符串**，改一处必须改两处。
    pub const fn id(self) -> &'static str {
        match self {
            Self::Lithe => "lithe",
            Self::IdeaClassic => "idea-classic",
            Self::Eclipse => "eclipse",
        }
    }

    /// 由 id 反查预设。查不到安静地返回 `None`，不猜一个。
    pub fn from_id(id: &str) -> Option<KeymapPreset> {
        match id {
            "lithe" => Some(Self::Lithe),
            "idea-classic" => Some(Self::IdeaClassic),
            "eclipse" => Some(Self::Eclipse),
            _ => None,
        }
    }
}

/// 一条键位的来源。
///
/// 三档，与 Windows 前端的四档（多一个 `extension`）**刻意不同**：gpui 这一侧没有扩展键位
/// 机制，加进去只会有一档永远没人填。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KeySource {
    /// 注册表默认值（[`DEFAULT_BINDINGS`]）。
    Default,
    /// 预设表（[`preset_bindings`]）。
    Preset,
    /// 当前预设下的用户覆盖。
    User,
}

/// 目录里的一行：一条命令在一张预设表里的键位。
///
/// `keystrokes` 是**整组**：一条命令的全部键位一起替换，不留半套。
/// `context` 是 gpui 的 context 谓词（如组件库编辑器的 `ElyEditor`），重绑时必须原样带回，
/// 否则带 context 的键位会退化成全窗绑定。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresetBinding {
    pub command: CommandId,
    pub keystrokes: &'static [&'static str],
    pub context: Option<&'static str>,
}

/// 用户覆盖：在**某一个预设**下，把某条命令的键位整组换成 `keystrokes`。
///
/// `command` 存的是 [`CommandId::id()`] 字符串而不是枚举本身：设置文件是长期兼容面，用户
/// 文件里可能留着这一侧已经删掉的命令 id，用 `String` 才能在 `normalize()` 里安静地丢掉它，
/// 而不是让整份文档解析失败。
///
/// `keystrokes` **空 vec 是有意义的值**：「这条命令在此预设下没有键位」。它必须能被表达、被
/// 存下来，不能被下一层（预设表 / 默认值）回填。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyOverride {
    pub preset: KeymapPreset,
    pub command: String,
    #[serde(default)]
    pub keystrokes: Vec<String>,
}

impl KeyOverride {
    pub fn new(preset: KeymapPreset, command: CommandId, keystrokes: Vec<String>) -> Self {
        Self {
            preset,
            command: command.id().to_string(),
            keystrokes,
        }
    }

    /// 这条覆盖指向的命令。文件里可能留着未知 id，所以是 `Option`。
    pub fn command_id(&self) -> Option<CommandId> {
        CommandId::from_id(&self.command)
    }
}

/// 键位设置：当前预设 + 全部预设的用户覆盖。
///
/// `overrides` **必须是 `Vec` 而不是 map**，理由见 `shared::document` 里 `preserve_unknown()`
/// 的文档注释：删除能否生效取决于键在 `next` 里存不存在，数组字段哪怕是空数组也会被序列化
/// 成 `[]`（键始终存在 ⇒ 整段被顶掉），map 则会被深合并回来、删除永远失效。
///
/// 覆盖按预设分桶（[`KeyOverride::preset`]）：分别改两套预设的同一条命令、切来切去与重启
/// 之后互不覆盖。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeymapSettings {
    pub preset: KeymapPreset,
    #[serde(default)]
    pub overrides: Vec<KeyOverride>,
}

impl KeymapSettings {
    pub fn new(preset: KeymapPreset) -> Self {
        Self {
            preset,
            overrides: Vec::new(),
        }
    }

    /// 取某预设下某条命令的覆盖。
    pub fn override_for(&self, preset: KeymapPreset, command: CommandId) -> Option<&KeyOverride> {
        let id = command.id();
        self.overrides
            .iter()
            .find(|entry| entry.preset == preset && entry.command == id)
    }

    /// 写入（或替换）一条覆盖，空列表表示「显式不绑定」。
    pub fn set_override(
        &mut self,
        preset: KeymapPreset,
        command: CommandId,
        keystrokes: Vec<String>,
    ) {
        let id = command.id();
        self.overrides
            .retain(|entry| !(entry.preset == preset && entry.command == id));
        self.overrides
            .push(KeyOverride::new(preset, command, keystrokes));
        self.normalize();
    }

    /// 删掉一条覆盖，让它回落到预设表 / 默认值。
    pub fn clear_override(&mut self, preset: KeymapPreset, command: CommandId) {
        let id = command.id();
        self.overrides
            .retain(|entry| !(entry.preset == preset && entry.command == id));
    }

    /// 合成一条命令的有效键位，顺序见模块文档那四层。
    pub fn resolve(&self, command: CommandId) -> ResolvedBinding {
        if let Some(entry) = self.override_for(self.preset, command) {
            return ResolvedBinding {
                command,
                keystrokes: entry.keystrokes.clone(),
                source: KeySource::User,
                context: context_for(command),
            };
        }
        if let Some(row) = find_row(preset_bindings(self.preset), command) {
            return ResolvedBinding {
                command,
                keystrokes: row
                    .keystrokes
                    .iter()
                    .map(|key| (*key).to_string())
                    .collect(),
                source: KeySource::Preset,
                context: row.context,
            };
        }
        if let Some(row) = find_row(DEFAULT_BINDINGS, command) {
            return ResolvedBinding {
                command,
                keystrokes: row
                    .keystrokes
                    .iter()
                    .map(|key| (*key).to_string())
                    .collect(),
                source: KeySource::Default,
                context: row.context,
            };
        }
        ResolvedBinding {
            command,
            keystrokes: Vec::new(),
            source: KeySource::Default,
            context: context_for(command),
        }
    }

    /// 按 `(preset, command)` 排序去重，**认不出来的命令 id 原样保留**。
    ///
    /// 全局设置文件（`schema::Settings`）走这一条而不是 [`Self::normalize`]，决定性理由是
    /// **调用频率不对称**：`SettingsStore::commit_global` 每提交一次就跑一次
    /// `Settings::normalize()`，也就是说挂在全局设置文件上的严格规则**不是只在读到坏数据时
    /// 生效，而是用户每改一个无关设置（调个字号、换个主题）都会触发一次**。那样的话用户改
    /// 字号、结果自己手写的那行未知 id 覆盖被悄悄删掉——这是**副作用，不是校验**。独立键位
    /// 文档没有这个问题：`parse_keymap` 只在明确解析键位文档时调用一次，那时做一次性的
    /// 严格清理是对的。
    ///
    /// 认不出的覆盖留着本身无害（`resolve` 只认已知 id），将来命令回来了它还在。
    ///
    /// 同一 `(preset, command)` 出现多次时**后写的赢**：设置页是「先删旧的再追加」，落盘前
    /// 再规整，后写的那一条代表用户最后一次的意图。**空 `keystrokes` 不在这条规则里** ——
    /// 它是「显式不绑定」，必须留下。
    pub fn dedup_overrides(&mut self) {
        let mut kept: Vec<KeyOverride> = Vec::with_capacity(self.overrides.len());
        for entry in self.overrides.drain(..) {
            match kept
                .iter_mut()
                .find(|known| known.preset == entry.preset && known.command == entry.command)
            {
                Some(known) => *known = entry,
                None => kept.push(entry),
            }
        }
        kept.sort_by(|left, right| {
            (left.preset, left.command.as_str()).cmp(&(right.preset, right.command.as_str()))
        });
        self.overrides = kept;
    }

    /// 在 [`Self::dedup_overrides`] 之上再丢掉认不出来的命令。
    ///
    /// 照 macOS 端那条「未知命令、重复绑定被忽略」：留着一行认不出来的覆盖，表现是「页面
    /// 上有这行、按下去也没反应」，比没有这行更难发现。只有**独立键位文档**（`parse_keymap`）
    /// 走这一条；全局设置文件走 `dedup_overrides`，理由见它自己的注释（调用频率不对称 ——
    /// `normalize` 会被 `commit_global` 反复触发，不能在那种频率下删用户数据）。
    pub fn normalize(&mut self) {
        self.dedup_overrides();
        self.overrides.retain(|entry| entry.command_id().is_some());
    }
}

/// 某条命令**能不能绑键**的判定口。由 workbench 在启动时注入。
///
/// ## 为什么是注入，不是一张表
///
/// 能不能绑键取决于这条命令有没有一个 gpui `Action` 类型，而 action 类型在 workbench
/// （`MenuAction::keybind_action()`）。依赖方向是 `workbench → settings`，`settings` **不能
/// 反向依赖**它。把「哪些命令能绑」抄成第二张表，就会有一份必然漂移的副本。
///
/// 所以这里只留一个查询口，宿主把解析器注进来——同 `identity::set_git_identity_host`
/// 那套宿主注入模式。
///
/// ## 没有注入时一律返回 `false`
///
/// 判定是 refinement，但缺省值必须选**错得响亮的那一侧**：返回 `true` 会放出一堆「改了也不
/// 生效」的编辑框，用户改完发现没反应，还找不到原因；返回 `false` 是整页变灰，一眼能看出
/// 解析器没注入。
pub type BindableResolver = std::rc::Rc<dyn Fn(CommandId) -> bool + 'static>;

thread_local! {
    static BINDABLE: std::cell::RefCell<Option<BindableResolver>> =
        const { std::cell::RefCell::new(None) };
}

pub fn set_bindable_resolver(resolver: Option<BindableResolver>) {
    BINDABLE.with(|slot| *slot.borrow_mut() = resolver);
}

pub fn is_bindable(command: CommandId) -> bool {
    BINDABLE.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|resolve| resolve(command))
    })
}

/// 一条命令的合成结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedBinding {
    pub command: CommandId,
    pub keystrokes: Vec<String>,
    pub source: KeySource,
    /// 重绑时要原样带回 `KeyBinding::new` 的 context 谓词。
    pub context: Option<&'static str>,
}

impl ResolvedBinding {
    /// 有没有键位。空列表（显式不绑定 / 未绑定）为 `false`。
    pub fn is_bound(&self) -> bool {
        !self.keystrokes.is_empty()
    }
}

/// 注册表默认值：**今天代码里真的绑了的那批**，按 [`CommandId::ALL`] 的顺序排。
///
/// 每一行都能指到一处 `install_actions`（`ctrl-` 已按本模块的约定改写为 `secondary-`，
/// Windows 上两者等价）。**改了任何一处 `bind_keys` 就必须同步改这张表**，否则设置页显示的
/// 与实际能按出来的不是同一套。
pub const DEFAULT_BINDINGS: &[PresetBinding] = &[
    // settings/src/dialog.rs
    PresetBinding {
        command: CommandId::OpenSettings,
        keystrokes: &["secondary-,"],
        context: None,
    },
    // workbench/src/menu_bar.rs:KEY_TOGGLE_TERMINAL
    PresetBinding {
        command: CommandId::ToggleTerminal,
        keystrokes: &["secondary-j"],
        context: None,
    },
    // workbench/src/command_palette.rs
    PresetBinding {
        command: CommandId::ToggleMenuBar,
        keystrokes: &["secondary-m"],
        context: None,
    },
    // editor/src/lib.rs
    PresetBinding {
        command: CommandId::SaveBuffer,
        keystrokes: &["secondary-s"],
        context: None,
    },
    // workbench/src/command_palette.rs
    PresetBinding {
        command: CommandId::OpenCommandPalette,
        keystrokes: &["secondary-shift-p"],
        context: None,
    },
    // workbench/src/quick_open.rs
    PresetBinding {
        command: CommandId::OpenQuickOpen,
        keystrokes: &["secondary-p"],
        context: None,
    },
    // workbench/src/replace_in_files.rs:REPLACE_KEY
    PresetBinding {
        command: CommandId::OpenReplaceInFiles,
        keystrokes: &["secondary-shift-r"],
        context: None,
    },
    // editor/src/lib.rs
    PresetBinding {
        command: CommandId::NavigateToDefinition,
        keystrokes: &["f12"],
        context: None,
    },
    // workbench/src/menu_bar.rs:KEY_GO_TO_REFERENCES（真源那句「使用 Shift+F12 查找引用」）
    PresetBinding {
        command: CommandId::GoToReferences,
        keystrokes: &["shift-f12"],
        context: None,
    },
    // workbench/src/menu_bar.rs:KEY_REOPEN_CLOSED_TAB
    PresetBinding {
        command: CommandId::ReopenClosedTab,
        keystrokes: &["secondary-shift-t"],
        context: None,
    },
];

/// IntelliJ IDEA Classic 档。**只映射两端同名、且键位有真源的那一条**（模块文档「已知缺口」）。
pub const IDEA_CLASSIC_BINDINGS: &[PresetBinding] = &[
    // 真源 `LitheCommandCatalog.swift` 的 `ideaClassicBindings`：`option-f12`。
    // macOS 的 `option` 在 Windows 键盘上就是 `alt`，与 IntelliJ Windows 端一致。
    PresetBinding {
        command: CommandId::ToggleTerminal,
        keystrokes: &["alt-f12"],
        context: None,
    },
];

/// Eclipse 档。**真源缺口，本轮留空**：macOS 那两份 Eclipse 表是 ⌘ 方言，Windows 上不成立。
///
/// 留空不等于「Eclipse 档没键位」—— 这里的空表表示「不覆盖任何默认值」，命中的命令全部
/// 落到第 3 层拿回 [`DEFAULT_BINDINGS`]。拿到 Windows 端 `Eclipse.xml` 真源后再填。
pub const ECLIPSE_BINDINGS: &[PresetBinding] = &[];

/// 某预设**显式声明**的键位表。它是覆盖层，不是全量表：没写进来的命令回落
/// [`DEFAULT_BINDINGS`]。所以 `Lithe` 档是空表（它的键位就是默认值本身）。
pub fn preset_bindings(preset: KeymapPreset) -> &'static [PresetBinding] {
    match preset {
        KeymapPreset::Lithe => &[],
        KeymapPreset::IdeaClassic => IDEA_CLASSIC_BINDINGS,
        KeymapPreset::Eclipse => ECLIPSE_BINDINGS,
    }
}

/// 重绑一条命令时要带的 context 谓词：先用预设表那一行，再用默认值那一行。
pub fn context_for(command: CommandId) -> Option<&'static str> {
    preset_bindings(KeymapPreset::Lithe)
        .iter()
        .chain(DEFAULT_BINDINGS.iter())
        .find(|row| row.command == command)
        .and_then(|row| row.context)
}

fn find_row(rows: &'static [PresetBinding], command: CommandId) -> Option<&'static PresetBinding> {
    rows.iter().find(|row| row.command == command)
}

/// 解析一份键位设置文档：认不出的字段保留在 `previous` 里由调用方原样写回。
///
/// 与 `settings::persistence::parse` 同形态：坏 JSON 不会让整个键位系统失效，只退化成默认值
/// 并留一条诊断。
pub fn parse_keymap(text: &str) -> Parsed<KeymapSettings> {
    let mut parsed: Parsed<KeymapSettings> =
        lithe_gpui_shared::parse(text, KEYMAP_SETTINGS_VERSION);
    parsed.value.normalize();
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn keystrokes_of(settings: &KeymapSettings, command: CommandId) -> Vec<String> {
        settings.resolve(command).keystrokes
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn preset_ids_match_the_serde_form() {
        // `id()` 与 serde 写进文件的字面量必须是同一个字符串：前者给诊断和前端看，后者是
        // 持久化的兼容面，两边漂移就会出现「存进去读不出来」。
        let mut seen = std::collections::BTreeSet::new();
        for preset in KeymapPreset::ALL {
            let id = preset.id();
            assert!(!id.is_empty(), "{preset:?} 的 id 是空的");
            assert!(seen.insert(id), "id 重复：{id}");
            assert_eq!(KeymapPreset::from_id(id), Some(preset), "{id} 反查不回自己");

            let value = serde_json::to_value(preset).expect("序列化失败");
            assert_eq!(
                value,
                Value::from(id),
                "{preset:?} 的 serde 形态与 id() 不一致"
            );
            assert_eq!(
                serde_json::from_value::<KeymapPreset>(value).expect("反序列化失败"),
                preset
            );
        }
        assert_eq!(seen.len(), KeymapPreset::ALL.len());
        assert_eq!(seen.len(), 3, "预设恰好三档");
    }

    #[test]
    fn unknown_preset_ids_are_rejected() {
        for id in [
            "",
            "Lithe",
            "idea_classic",
            "idea-classic ",
            "macos",
            "日蚀",
        ] {
            assert_eq!(KeymapPreset::from_id(id), None, "{id:?} 不该被认出来");
        }
    }

    #[test]
    fn no_keystroke_uses_the_cmd_dialect() {
        // `cmd-` 在 Windows 上是 Win 键（gpui 的 `Keystroke::parse` 把它解析成
        // `modifiers.platform`），键位串必须写 `secondary-`。`explorer` 那组 `cmd-a`/`ctrl-a`
        // 双写是历史形态（已在 B1 统一为 `secondary-`），不许往这里抄。
        const PREFIXES: [&str; 5] = ["secondary", "ctrl", "alt", "shift", "fn"];
        for (table_name, table) in [
            ("DEFAULT_BINDINGS", DEFAULT_BINDINGS),
            ("IDEA_CLASSIC_BINDINGS", IDEA_CLASSIC_BINDINGS),
            ("ECLIPSE_BINDINGS", ECLIPSE_BINDINGS),
        ] {
            for row in table {
                for keystroke in row.keystrokes {
                    let prefix = keystroke.split_once('-').map(|(head, _)| head);
                    assert!(
                        !keystroke
                            .split('-')
                            .any(|part| part.eq_ignore_ascii_case("cmd")
                                || part.eq_ignore_ascii_case("super")
                                || part.eq_ignore_ascii_case("win")),
                        "{table_name} 里 {keystroke:?} 用了平台方言（写 cmd/super/win 在 Windows 上就是 Win 键）"
                    );
                    if let Some(prefix) = prefix {
                        assert!(
                            PREFIXES.contains(&prefix),
                            "{table_name} 里 {keystroke:?} 的修饰键 {prefix:?} 不在允许表里"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_catalog_keystroke_parses_under_gpui() {
        // 键位串写错（多一个连字符、拼错键名）在 gpui 里要等按键按下才发现，而且表现是
        // 「静默没反应」。这里用 gpui 自己的解析器过一遍整张目录。
        for (table_name, table) in [
            ("DEFAULT_BINDINGS", DEFAULT_BINDINGS),
            ("IDEA_CLASSIC_BINDINGS", IDEA_CLASSIC_BINDINGS),
            ("ECLIPSE_BINDINGS", ECLIPSE_BINDINGS),
        ] {
            for row in table {
                for keystroke in row.keystrokes {
                    assert!(
                        gpui::Keystroke::parse(keystroke).is_ok(),
                        "{table_name} 里 {keystroke:?} 过不了 gpui 的 Keystroke::parse"
                    );
                }
            }
        }
    }

    #[test]
    fn default_bindings_follow_the_command_catalog_order_and_are_unique() {
        let mut previous = None;
        let mut seen = std::collections::BTreeSet::new();
        for row in DEFAULT_BINDINGS {
            assert!(
                seen.insert(row.command),
                "{:?} 在默认表里出现两次",
                row.command
            );
            assert!(
                !row.keystrokes.is_empty(),
                "{:?} 的默认行是空列表：没有键位的命令不进目录",
                row.command
            );
            if let Some(previous) = previous {
                assert!(
                    previous < row.command,
                    "默认表要按 CommandId::ALL 的顺序排：{previous:?} 在 {:?} 之后",
                    row.command
                );
            }
            previous = Some(row.command);
        }
    }

    #[test]
    fn the_default_table_is_the_keys_that_are_actually_bound_today() {
        // 真源快照：左侧是本仓 `install_actions` 里真的绑的那些键（`ctrl-` 已改写成
        // `secondary-`），右侧是别的 crate 里的常量。改了任何一处 `bind_keys` 都要同步改这里。
        let expected: Vec<(CommandId, Vec<&str>)> = vec![
            (CommandId::OpenSettings, vec!["secondary-,"]),
            (CommandId::ToggleTerminal, vec!["secondary-j"]),
            (CommandId::ToggleMenuBar, vec!["secondary-m"]),
            (CommandId::SaveBuffer, vec!["secondary-s"]),
            (CommandId::OpenCommandPalette, vec!["secondary-shift-p"]),
            (CommandId::OpenQuickOpen, vec!["secondary-p"]),
            (CommandId::OpenReplaceInFiles, vec!["secondary-shift-r"]),
            (CommandId::NavigateToDefinition, vec!["f12"]),
            // 真源 `references.findReferencesHint` 写着「使用 Shift+F12 查找引用」，
            // 绑定登记在 `lithe_gpui_editor::install_actions`。
            (CommandId::GoToReferences, vec!["shift-f12"]),
            (CommandId::ReopenClosedTab, vec!["secondary-shift-t"]),
        ];
        let actual: Vec<(CommandId, Vec<String>)> = DEFAULT_BINDINGS
            .iter()
            .map(|row| {
                (
                    row.command,
                    row.keystrokes
                        .iter()
                        .map(|key| (*key).to_string())
                        .collect(),
                )
            })
            .collect();
        let expected: Vec<(CommandId, Vec<String>)> = expected
            .into_iter()
            .map(|(command, keys)| (command, strings(&keys)))
            .collect();
        assert_eq!(actual, expected, "默认表与本仓实际绑定的键位不一致");
    }

    #[test]
    fn an_unmapped_command_resolves_to_nothing_rather_than_a_guess() {
        let settings = KeymapSettings::default();
        // `OpenFile` 今天没有可派发的 action（见 2026-10-10 决策十），所以它没有键位，
        // 也不能拿别的命令的键来凑。
        let resolved = settings.resolve(CommandId::OpenFile);
        assert!(resolved.keystrokes.is_empty());
        assert!(!resolved.is_bound());
    }

    #[test]
    fn the_lithe_preset_reads_the_default_table() {
        let settings = KeymapSettings::new(KeymapPreset::Lithe);
        let resolved = settings.resolve(CommandId::SaveBuffer);
        assert_eq!(resolved.keystrokes, strings(&["secondary-s"]));
        assert_eq!(resolved.source, KeySource::Default, "Lithe 档不覆盖默认值");
    }

    #[test]
    fn the_preset_beats_the_default_and_reports_its_source() {
        let settings = KeymapSettings::new(KeymapPreset::IdeaClassic);
        let resolved = settings.resolve(CommandId::ToggleTerminal);
        assert_eq!(resolved.keystrokes, strings(&["alt-f12"]));
        assert_eq!(resolved.source, KeySource::Preset);

        // 预设表里没有的命令仍然回落默认值，不会被预设档「清空」。
        let resolved = settings.resolve(CommandId::SaveBuffer);
        assert_eq!(resolved.keystrokes, strings(&["secondary-s"]));
        assert_eq!(resolved.source, KeySource::Default);
    }

    #[test]
    fn the_eclipse_preset_has_no_true_source_yet() {
        // 这不是断言「Eclipse 档应该没键位」，而是把**当前的真源缺口**钉住：等拿到
        // Windows 端 Eclipse 真源、往 ECLIPSE_BINDINGS 里填了东西，这条会失败并提醒补断言。
        assert!(
            ECLIPSE_BINDINGS.is_empty(),
            "ECLIPSE_BINDINGS 不再是空的 —— 真源缺口已补上，本条断言与模块文档要一起改"
        );
        let settings = KeymapSettings::new(KeymapPreset::Eclipse);
        let resolved = settings.resolve(CommandId::ToggleTerminal);
        assert_eq!(
            resolved.keystrokes,
            strings(&["secondary-j"]),
            "没映射到的命令必须回落默认值，而不是变成未绑定"
        );
        assert_eq!(resolved.source, KeySource::Default);
    }

    #[test]
    fn a_user_override_beats_the_preset_and_the_default() {
        let mut settings = KeymapSettings::new(KeymapPreset::IdeaClassic);
        settings.set_override(
            KeymapPreset::IdeaClassic,
            CommandId::ToggleTerminal,
            strings(&["secondary-k"]),
        );
        let resolved = settings.resolve(CommandId::ToggleTerminal);
        assert_eq!(resolved.keystrokes, strings(&["secondary-k"]));
        assert_eq!(resolved.source, KeySource::User);
    }

    #[test]
    fn an_empty_override_disables_the_command_instead_of_falling_back() {
        // 硬约束：空 vec 是「这条命令在此预设下没有键位」，不能被下一层（预设表 / 默认值）
        // 回填。macOS 端 Eclipse 档里 `Find` 为空、不能回退成 ⌘F 就是同一条规则。
        let mut settings = KeymapSettings::new(KeymapPreset::Lithe);
        settings.set_override(KeymapPreset::Lithe, CommandId::SaveBuffer, Vec::new());

        let resolved = settings.resolve(CommandId::SaveBuffer);
        assert!(resolved.keystrokes.is_empty(), "不许回落到 secondary-s");
        assert!(!resolved.is_bound());
        assert_eq!(resolved.source, KeySource::User, "空覆盖也是用户意图");

        // 而且它必须能原样存下来、读回来还是空的。
        let json = serde_json::to_string(&settings).expect("序列化失败");
        assert!(
            json.contains(r#""keystrokes":[]"#),
            "空列表必须写成 [] 而不是被省略：{json}"
        );
        let parsed = parse_keymap(&json);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(
            keystrokes_of(&parsed.value, CommandId::SaveBuffer),
            Vec::<String>::new()
        );
    }

    #[test]
    fn overrides_are_scoped_to_their_preset() {
        let mut settings = KeymapSettings::new(KeymapPreset::Lithe);
        settings.set_override(
            KeymapPreset::Lithe,
            CommandId::ToggleTerminal,
            strings(&["secondary-k"]),
        );
        settings.set_override(
            KeymapPreset::Eclipse,
            CommandId::ToggleTerminal,
            strings(&["secondary-l"]),
        );

        assert_eq!(
            keystrokes_of(&settings, CommandId::ToggleTerminal),
            strings(&["secondary-k"])
        );
        settings.preset = KeymapPreset::Eclipse;
        assert_eq!(
            keystrokes_of(&settings, CommandId::ToggleTerminal),
            strings(&["secondary-l"]),
            "切换预设要换桶，不能两套覆盖互相盖"
        );
        settings.preset = KeymapPreset::IdeaClassic;
        assert_eq!(
            keystrokes_of(&settings, CommandId::ToggleTerminal),
            strings(&["alt-f12"]),
            "没覆盖过的预设看它自己的预设表"
        );
    }

    #[test]
    fn clearing_an_override_falls_back_to_the_layer_below() {
        let mut settings = KeymapSettings::new(KeymapPreset::IdeaClassic);
        settings.set_override(
            KeymapPreset::IdeaClassic,
            CommandId::ToggleTerminal,
            strings(&["secondary-k"]),
        );
        assert_eq!(
            settings.resolve(CommandId::ToggleTerminal).source,
            KeySource::User
        );

        settings.clear_override(KeymapPreset::IdeaClassic, CommandId::ToggleTerminal);
        let resolved = settings.resolve(CommandId::ToggleTerminal);
        assert_eq!(resolved.keystrokes, strings(&["alt-f12"]));
        assert_eq!(
            resolved.source,
            KeySource::Preset,
            "清掉覆盖 → 回落到预设表"
        );
    }

    #[test]
    fn normalize_drops_unknown_commands_and_duplicates() {
        let mut settings = KeymapSettings::new(KeymapPreset::Lithe);
        settings.overrides = vec![
            KeyOverride {
                preset: KeymapPreset::Lithe,
                command: "not-a-command".to_string(),
                keystrokes: strings(&["secondary-x"]),
            },
            KeyOverride {
                preset: KeymapPreset::IdeaClassic,
                command: "save-buffer".to_string(),
                keystrokes: strings(&["secondary-a"]),
            },
            KeyOverride {
                preset: KeymapPreset::IdeaClassic,
                command: "save-buffer".to_string(),
                keystrokes: strings(&["secondary-b"]),
            },
        ];
        settings.normalize();

        assert_eq!(
            settings.overrides,
            vec![KeyOverride {
                preset: KeymapPreset::IdeaClassic,
                command: "save-buffer".to_string(),
                keystrokes: strings(&["secondary-b"]),
            }],
            "未知命令要丢、重复条目后写的赢"
        );
    }

    #[test]
    fn normalize_keeps_the_sorted_order() {
        let mut settings = KeymapSettings::new(KeymapPreset::Lithe);
        settings.set_override(
            KeymapPreset::Eclipse,
            CommandId::SaveBuffer,
            strings(&["secondary-a"]),
        );
        settings.set_override(
            KeymapPreset::IdeaClassic,
            CommandId::CloseTab,
            strings(&["secondary-b"]),
        );
        settings.set_override(
            KeymapPreset::IdeaClassic,
            CommandId::SaveBuffer,
            strings(&["secondary-c"]),
        );
        assert_eq!(
            settings
                .overrides
                .iter()
                .map(|entry| (entry.preset, entry.command.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (KeymapPreset::IdeaClassic, "close-tab"),
                (KeymapPreset::IdeaClassic, "save-buffer"),
                (KeymapPreset::Eclipse, "save-buffer"),
            ]
        );
    }

    #[test]
    fn the_overrides_key_is_always_written_so_deletion_survives() {
        // 这是「必须存数组」那条约束的**行为级**证明，不是靠注释：`preserve_unknown` 的判据
        // 是「键在 next 里存不存在」，数组字段哪怕是空数组也序列化成 `[]`，于是整段被顶掉；
        // 存成 map 会被深合并回来，删除永远失效。
        let previous: Value = serde_json::from_str(
            r#"{
              "version": 1,
              "preset": "lithe",
              "overrides": [
                {"preset": "lithe", "command": "save-buffer", "keystrokes": ["secondary-s"]},
                {"preset": "lithe", "command": "close-tab", "keystrokes": ["secondary-w"]}
              ]
            }"#,
        )
        .expect("旧文档");

        // 用户删掉了 save-buffer 那条覆盖。
        let mut next = KeymapSettings::new(KeymapPreset::Lithe);
        next.set_override(
            KeymapPreset::Lithe,
            CommandId::CloseTab,
            strings(&["secondary-w"]),
        );
        let typed = serde_json::to_value(&next).expect("序列化失败");
        assert!(
            typed.get("overrides").is_some(),
            "overrides 必须每次都出现在文档里：{typed}"
        );

        let merged = lithe_gpui_shared::preserve_unknown(Some(&previous), &typed);
        assert_eq!(
            merged.get("overrides"),
            Some(&serde_json::json!([
                {"preset": "lithe", "command": "close-tab", "keystrokes": ["secondary-w"]}
            ])),
            "删掉的那条不许被旧文档合并回来：{merged}"
        );
        assert_eq!(merged.get("version"), Some(&Value::from(1)), "未知键要保留");
    }

    #[test]
    fn parsing_a_document_drops_unknown_commands_and_keeps_unknown_keys() {
        let parsed = parse_keymap(
            r#"{
              "version": 1,
              "preset": "idea-classic",
              "overrides": [
                {"preset": "lithe", "command": "save-buffer", "keystrokes": ["secondary-x"]},
                {"preset": "lithe", "command": "ghost-command", "keystrokes": ["secondary-y"]}
              ],
              "thirdParty": {"keepMe": true}
            }"#,
        );
        assert!(!parsed.read_only);
        assert_eq!(parsed.value.preset, KeymapPreset::IdeaClassic);
        assert_eq!(
            parsed.value.overrides,
            vec![KeyOverride {
                preset: KeymapPreset::Lithe,
                command: "save-buffer".to_string(),
                keystrokes: strings(&["secondary-x"]),
            }],
            "认不出来的命令要被丢掉"
        );
        assert_eq!(
            parsed
                .previous
                .as_ref()
                .and_then(|value| value.get("thirdParty")),
            Some(&serde_json::json!({"keepMe": true})),
            "未知键留在 previous 里由调用方原样写回"
        );
    }

    #[test]
    fn a_broken_document_falls_back_to_the_default() {
        let parsed = parse_keymap("{ not json ");
        assert_eq!(parsed.value, KeymapSettings::default());
        assert!(!parsed.diagnostics.is_empty());

        let parsed = parse_keymap(r#"{"preset": "no-such-preset"}"#);
        assert_eq!(
            parsed.value.preset,
            KeymapPreset::Lithe,
            "认不出的预设回落 Lithe，不是让整份文档失效"
        );
    }

    #[test]
    fn a_newer_document_version_is_read_only() {
        let parsed = lithe_gpui_shared::parse::<KeymapSettings>(
            r#"{"version": 99, "preset": "eclipse"}"#,
            KEYMAP_SETTINGS_VERSION,
        );
        assert!(parsed.read_only);
        assert_eq!(parsed.value.preset, KeymapPreset::Eclipse);
    }

    #[test]
    fn the_context_predicate_is_carried_through_resolution() {
        // 今天目录里九行都是全窗绑定（`None`），这条先把「context 跟着命令走」这条管道钉住：
        // 二期往目录里填带 context 的行（组件库编辑器的 `ElyEditor`）时，重绑必须原样带回它，
        // 否则 `secondary-z` 会在编辑区之外也触发。
        for row in DEFAULT_BINDINGS.iter().chain(IDEA_CLASSIC_BINDINGS.iter()) {
            if let Some(context) = row.context {
                assert!(!context.is_empty(), "{:?} 的 context 是空串", row.command);
            }
            let resolved = KeymapSettings::new(KeymapPreset::Lithe).resolve(row.command);
            assert_eq!(
                resolved.context,
                context_for(row.command),
                "{:?} 的 context 在合成时丢了",
                row.command
            );
        }
        assert_eq!(context_for(CommandId::OpenFile), None);
    }

    #[test]
    fn resolution_never_hands_out_a_keystroke_that_gpui_cannot_parse() {
        // 用户覆盖里的键位串同样要能过 gpui 的解析器 —— 二期重绑时会直接喂给它。
        for keystroke in ["secondary-s", "secondary-shift-p", "alt-f12", "f12"] {
            assert!(gpui::Keystroke::parse(keystroke).is_ok(), "{keystroke}");
        }
    }

    /// 没注入解析器时一律 `false`。
    ///
    /// 这个缺省方向是刻意的：宁可整页变灰（一眼看得出解析器没注入），也不能放出一堆
    /// 「改了也不生效」的编辑框。
    #[test]
    fn nothing_is_bindable_without_a_resolver() {
        // `BINDABLE` 是 thread_local，而测试各跑各的线程，所以这里不会串到别的用例。
        set_bindable_resolver(None);
        for command in CommandId::ALL {
            assert!(
                !is_bindable(command),
                "{command:?} 在没有注入解析器时不该被判成可绑"
            );
        }
    }

    /// 注入之后由解析器说了算，且**逐条**生效。
    #[test]
    fn a_resolver_decides_what_is_bindable_command_by_command() {
        set_bindable_resolver(Some(std::rc::Rc::new(|command| {
            matches!(command, CommandId::SaveBuffer | CommandId::OpenQuickOpen)
        })));
        assert!(is_bindable(CommandId::SaveBuffer));
        assert!(is_bindable(CommandId::OpenQuickOpen));
        assert!(!is_bindable(CommandId::NewWindow));
        assert!(!is_bindable(CommandId::OpenSearchEverywhere));
        // 恢复缺省，免得同线程里后来的用例被影响。
        set_bindable_resolver(None);
    }
}
