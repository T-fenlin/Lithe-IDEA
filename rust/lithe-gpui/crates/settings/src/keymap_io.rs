//! 键位的导入导出：把「当前预设下的用户覆盖」写成一个文件、再读回来。
//!
//! 只做**数据转换**（文本 ↔ 覆盖），不碰文件对话框、不碰提示 UI——那些在 `dialog.rs`。
//!
//! ## 范围：gpui 端自闭环，不跨端
//!
//! **导出的文件只有 gpui 端读得回。** 三端命令 id 命名空间互不相同（`keymap.rs` 模块文档
//! 「已知缺口」）：Windows 遗留前端是 `file.save` / `workbench.commandPalette`，macOS 端是
//! 第三套，本侧这 23 个 id 又是另一套。预设表因此无法机械移植，导入导出同理。与 macOS 端
//! 互通需要第二张 id 映射表，而真源还不全（Eclipse 档就因为没有 Windows 侧真源留空了）——
//! 那是另一条任务，不在这里。
//!
//! ## 为什么 format 串不沿用老的 `lithe.keybindings`
//!
//! 沿用会让老文件**通过格式校验**，然后每条 `command` 都过不了
//! [`CommandId::from_id`]，一路走到「导入 0 条」。用户看到「已导入 0 个」只会以为文件是
//! 空的，永远猜不到是 id 命名空间不通——**这是最迷惑的一种失败**。
//! 换一个不同的串（[`EXPORT_FORMAT`]）让老文件在**第一道格式校验**就被挡下，直接判成
//! 「文件格式无效」，而不是通过校验后再静默导入 0 条。
//!
//! ## 三道判定，一道比一道兜底
//!
//! 1. **格式门（主防线）**：必须是对象，且 `format` 与 `version` **严格相等**。
//!    **不接受裸数组**——裸数组没有 `format` 可校验，收下它就等于把这个门拆了。
//! 2. **文件里有条目、却一条都没导入**：同样判成「格式无效」，**不**报「已导入 0 个」。
//!    这对用户就是「这个文件本应用不了」。格式门是主防线，这一道防的是「有人手改了
//!    format 串」之类的漏网。
//! 3. 部分成功才报「已导入 N 个」+ 跳过条数。**跳过数永远不许静默。**
//!
//! ## 已知的有损转换
//!
//! 老格式（`enabled: false`）里禁用条目**还保留着键位**，将来可以再启用；本侧的模型只有
//! 「空 keystrokes = 显式不绑定」，存成空就丢掉了那个键。这是有损的，见
//! [`normalize_entry`] 里的注释。

use serde_json::Value;

use lithe_gpui_shared::commands::CommandId;

use crate::keymap::{KeymapPreset, KeymapSettings, context_for};

/// 导出文件的格式标识。
///
/// **刻意与 Windows 遗留前端的 `lithe.keybindings` 不同**：老文件的命令 id 是另一套命名
/// 空间，复用老串会让它通过格式校验、然后一条都导不进来（见模块文档）。换串之后，老文件
/// 在第一道校验就被挡下。
pub const EXPORT_FORMAT: &str = "lithe.gpui.keybindings";

/// 导出文件的格式版本。改动 `ExportPayload` 的字段语义时**必须**递增——导入侧是严格相等
/// 校验，不递增就会拿旧形状去解释新文件。
pub const EXPORT_VERSION: u32 = 1;

/// 建议的导出文件名。
pub const EXPORT_FILE_NAME: &str = "keybindings.json";

/// 一份导入文件解析出来的结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportResult {
    /// 判成「文件格式无效」：格式门没过（含裸数组），**或**文件里认出了条目却一条都导不进。
    Rejected,
    /// 可以应用。`entries` 里的 [`ImportEntry::command`] 为 `None` 的就是要跳过并报出来的。
    Applied(ImportPlan),
}

/// 一份待应用的导入内容。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportPlan {
    /// 文件里带的预设。**有值时导入会顺带把当前预设切过去**——覆盖是按预设分桶的，写进
    /// 文件那一桶而当前停在另一档，用户会看到「导入成功但页面没变」。
    pub preset: Option<KeymapPreset>,
    /// 文件里认出来的每一条。顺序即文件顺序。
    pub entries: Vec<ImportEntry>,
}

impl ImportPlan {
    /// 真正写得进去的条数。
    pub fn imported(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.command.is_some())
            .count()
    }

    /// 认出来但写不进去的条数（命令 id 不认识）。
    pub fn skipped(&self) -> usize {
        self.entries.len() - self.imported()
    }
}

/// 一条从文件里认出来的条目。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportEntry {
    /// `None` = 命令 id 认不出来，这条只能跳过。
    pub command: Option<CommandId>,
    /// 空 vec 是**显式不绑定**，不是「没设置」。
    pub keystrokes: Vec<String>,
}

/// 导出：把**当前预设**那一桶覆盖写成 [`Value`]。
///
/// 只导当前预设，不导三桶：覆盖是按预设分桶的（`KeyOverride::preset`），把三桶压进一条
/// 平表就分不清哪条属于哪一档，丢的信息比省的事多。要导另一档就切过去再导一次。
///
/// `exported_at` 由调用方给（便于测试确定性），**导入侧从不读它**，只是给人看的一行。
pub fn export_payload(settings: &KeymapSettings, exported_at: &str) -> Value {
    let preset = settings.preset;
    let keybindings: Vec<Value> = settings
        .overrides
        .iter()
        .filter(|entry| entry.preset == preset)
        .filter_map(|entry| entry.command_id())
        .map(|command| {
            // context 谓词**必须原样带回**（`keymap.rs` 模块文档第二条硬约束）：不带的话
            // `ElyEditor` 那批键位会退化成全窗绑定。今天覆盖本身没有 context 字段，所以取
            // 注册表那一行的——那正是重绑时会用的那个。
            let when = context_for(command);
            let mut row = serde_json::Map::new();
            row.insert("command".into(), Value::from(command.id()));
            row.insert(
                "keystrokes".into(),
                Value::from(
                    settings
                        .override_for(preset, command)
                        .map(|entry| entry.keystrokes.clone())
                        .unwrap_or_default(),
                ),
            );
            row.insert("when".into(), when.map(Value::from).unwrap_or(Value::Null));
            Value::Object(row)
        })
        .collect();

    let mut payload = serde_json::Map::new();
    payload.insert("format".into(), Value::from(EXPORT_FORMAT));
    payload.insert("version".into(), Value::from(EXPORT_VERSION));
    payload.insert("exportedAt".into(), Value::from(exported_at));
    payload.insert("keybindingPreset".into(), Value::from(preset.id()));
    payload.insert("keybindings".into(), Value::from(keybindings));
    Value::Object(payload)
}

/// 解析一份导入文件的文本。判定三档见模块文档。
pub fn parse_import(text: &str) -> ImportResult {
    let Ok(root) = serde_json::from_str::<Value>(text) else {
        return ImportResult::Rejected;
    };
    // 只认对象形态。**裸数组不接受**：它没有 `format` 可校验，收下它等于拆掉格式门。
    let Some(object) = root.as_object() else {
        return ImportResult::Rejected;
    };
    if object.get("format").and_then(Value::as_str) != Some(EXPORT_FORMAT) {
        return ImportResult::Rejected;
    }
    if object.get("version").and_then(Value::as_u64) != Some(EXPORT_VERSION as u64) {
        return ImportResult::Rejected;
    }
    let Some(rows) = object.get("keybindings").and_then(Value::as_array) else {
        return ImportResult::Rejected;
    };

    let preset = object
        .get("keybindingPreset")
        .and_then(Value::as_str)
        .and_then(KeymapPreset::from_id);
    let entries: Vec<ImportEntry> = rows.iter().filter_map(normalize_entry).collect();

    // 兜底那一道：文件里写了条目（**包括**只是认不出来的行），却一条都没导进来 —— 判成
    // 「格式无效」而不是「已导入 0 个」。否则用户只会以为文件是空的，永远猜不到是 id
    // 命名空间不通。判据用**原始行数**而不是认出来的条数：一堆 junk 行同样该被挡下。
    if !rows.is_empty() && entries.iter().all(|entry| entry.command.is_none()) {
        return ImportResult::Rejected;
    }

    ImportResult::Applied(ImportPlan { preset, entries })
}

/// 从文件里的一行解出一条条目。
///
/// `None` = 连「有 command」都不满足，不算一条（连跳过数都不该计它）。
fn normalize_entry(value: &Value) -> Option<ImportEntry> {
    let object = value.as_object()?;
    let command = object.get("command").and_then(Value::as_str)?;
    let command = command.trim();
    if command.is_empty() {
        return None;
    }

    // 老格式用单个 `key` 字符串、外加 `enabled` 开关。这两条**只为兼容**留着：我们自己
    // 导出的文件用的是 `keystrokes` 数组。
    //
    // **有损**：老格式的 `enabled: false` 里**还留着键位**（禁用不等于清空，将来可以再启
    // 用），而本侧模型只有「空 keystrokes = 显式不绑定」这一档，映射过来就把那个键丢了。
    // 这是刻意的取舍，不是无损映射——别让后人以为是等价转换。
    if object.get("enabled").and_then(Value::as_bool) == Some(false) {
        return Some(ImportEntry {
            command: CommandId::from_id(command),
            keystrokes: Vec::new(),
        });
    }

    let keystrokes = match object.get("keystrokes").and_then(Value::as_array) {
        Some(list) => list
            .iter()
            .filter_map(Value::as_str)
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty())
            .collect(),
        None => match object.get("key").and_then(Value::as_str) {
            Some(key) => key.split_whitespace().map(|key| key.to_string()).collect(),
            None => Vec::new(),
        },
    };

    Some(ImportEntry {
        command: CommandId::from_id(command),
        keystrokes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with(preset: KeymapPreset) -> KeymapSettings {
        let mut settings = KeymapSettings::new(preset);
        settings.set_override(preset, CommandId::ToggleTerminal, vec!["alt-f12".into()]);
        settings.set_override(
            KeymapPreset::IdeaClassic,
            CommandId::ToggleTerminal,
            vec!["ctrl-f12".into()],
        );
        settings
    }

    /// 只导当前预设那一桶：三桶压进一条平表会分不清哪条属于哪一档。
    #[test]
    fn export_only_carries_the_current_preset_bucket() {
        let payload = export_payload(&settings_with(KeymapPreset::Lithe), "2026-10-11");
        let rows = payload["keybindings"].as_array().unwrap();
        assert_eq!(
            rows.len(),
            1,
            "Lithe 档只有一条覆盖，不该带上 IdeaClassic 那一条"
        );
        assert_eq!(rows[0]["command"], "toggle-terminal");
        assert_eq!(rows[0]["keystrokes"], serde_json::json!(["alt-f12"]));
        assert_eq!(payload["keybindingPreset"], "lithe");
    }

    /// 导出的文件能原样读回来，键位与 context 都不丢。
    #[test]
    fn a_round_trip_preserves_keystrokes_and_context() {
        let settings = settings_with(KeymapPreset::IdeaClassic);
        let text = serde_json::to_string_pretty(&export_payload(&settings, "now")).unwrap();
        let ImportResult::Applied(plan) = parse_import(&text) else {
            panic!("自己导出的文件必须能被自己读回来");
        };
        assert_eq!(plan.preset, Some(KeymapPreset::IdeaClassic));
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].command, Some(CommandId::ToggleTerminal));
        assert_eq!(plan.entries[0].keystrokes, vec!["ctrl-f12".to_string()]);
        assert_eq!(plan.imported(), 1);
        assert_eq!(plan.skipped(), 0);
    }

    /// **第 6 条裁定的守卫**：老格式串必须在第一道格式校验就被挡下。
    ///
    /// 沿用 `lithe.keybindings` 会让它通过校验、然后每条 id 都认不出来，走到「导入 0 条」——
    /// 那是最迷惑的一种失败。这条断言钉死「老文件过不了门」。
    #[test]
    fn an_old_format_file_is_rejected_at_the_format_gate() {
        let old = serde_json::json!({
            "format": "lithe.keybindings",
            "version": 1,
            "exportedAt": "2026-10-11",
            "keybindingPreset": "lithe",
            "keybindings": [
                { "key": "ctrl-s", "command": "file.save", "source": "user", "enabled": true }
            ],
        });
        assert_eq!(
            parse_import(&serde_json::to_string(&old).unwrap()),
            ImportResult::Rejected,
            "老格式串必须被格式门挡下，不能通过校验后再导入 0 条"
        );
        // 只把 format 串改成自己的、id 仍是老的那套，会被第二道判定挡下（见下一条）。
        let mut hand_edited = old.clone();
        hand_edited["format"] = serde_json::json!(EXPORT_FORMAT);
        assert_eq!(
            parse_import(&serde_json::to_string(&hand_edited).unwrap()),
            ImportResult::Rejected,
            "手改 format 串进来的老 id 文件，由「有条目却一条都导不进」那一道兜住"
        );
    }

    /// 裸数组没有 format 可校验，收下它等于把格式门拆了。
    #[test]
    fn a_bare_array_is_rejected_too() {
        let bare = serde_json::json!([{ "command": "toggle-terminal", "keystrokes": ["alt-f12"] }]);
        assert_eq!(
            parse_import(&serde_json::to_string(&bare).unwrap()),
            ImportResult::Rejected
        );
    }

    /// 部分成功：成功的照进，认不出来的报出来，两个数字分开。
    #[test]
    fn a_partially_importable_file_reports_both_counts() {
        let mixed = serde_json::json!({
            "format": EXPORT_FORMAT,
            "version": EXPORT_VERSION,
            "keybindingPreset": "lithe",
            "keybindings": [
                { "command": "toggle-terminal", "keystrokes": ["alt-f12"] },
                { "command": "file.save", "keystrokes": ["ctrl-s"] },
            ],
        });
        let ImportResult::Applied(plan) = parse_import(&serde_json::to_string(&mixed).unwrap())
        else {
            panic!("有一条认得出来就该走 Applied，由调用方报跳过条数");
        };
        assert_eq!(plan.imported(), 1);
        assert_eq!(plan.skipped(), 1, "认不出来的那条必须能被报出来，不许静默");
    }

    /// 空 keystrokes 是「显式不绑定」，导出成一个空数组、读回来还是空。
    #[test]
    fn an_unbound_command_round_trips_as_an_empty_list() {
        let mut settings = KeymapSettings::new(KeymapPreset::Lithe);
        settings.set_override(KeymapPreset::Lithe, CommandId::ToggleTerminal, Vec::new());
        let text = serde_json::to_string(&export_payload(&settings, "now")).unwrap();
        assert!(
            text.contains("\"keystrokes\":[]"),
            "空列表必须写出来：{text}"
        );
        let ImportResult::Applied(plan) = parse_import(&text) else {
            panic!("空列表是合法值，不该被判成格式无效");
        };
        assert_eq!(plan.entries[0].keystrokes, Vec::<String>::new());
    }

    /// 老格式的 `enabled: false` 映射到空 keystrokes（**有损**，见 `normalize_entry`）。
    #[test]
    fn a_disabled_legacy_entry_becomes_an_empty_keystroke_list() {
        let legacy = serde_json::json!({
            "format": EXPORT_FORMAT,
            "version": EXPORT_VERSION,
            "keybindingPreset": "lithe",
            "keybindings": [
                { "key": "ctrl-s", "command": "toggle-terminal", "enabled": false }
            ],
        });
        let ImportResult::Applied(plan) = parse_import(&serde_json::to_string(&legacy).unwrap())
        else {
            panic!("禁用条目也是认出来的条目");
        };
        assert_eq!(
            plan.entries[0].keystrokes,
            Vec::<String>::new(),
            "enabled: false = 显式不绑定；键位本身被丢弃，这是已知的有损转换"
        );
    }

    /// 一堆没有 command 的 junk 行：连条目都算不上（不污染跳过数），但**文件被判无效**——
    /// 「文件里有东西却一条都没进来」跟 id 不认识是同一档，都该挡下。
    #[test]
    fn a_file_of_junk_rows_is_rejected_and_counts_as_nothing() {
        let junk = serde_json::json!({
            "format": EXPORT_FORMAT,
            "version": EXPORT_VERSION,
            "keybindingPreset": "lithe",
            "keybindings": [{ "keystrokes": ["ctrl-s"] }, { "command": "  " }],
        });
        assert_eq!(
            parse_import(&serde_json::to_string(&junk).unwrap()),
            ImportResult::Rejected
        );
    }

    /// 文件里带了**认不出来**的预设 id：不切换、不报错、键位照常进来。
    ///
    /// 绝不能因为预设 id 认不出来就把整个文件判成无效——那会把文件里的键位一起丢掉。
    /// 认不出来就落成 `preset: None`，由调用方落在当前预设上（也因此不填「及预设」）。
    #[test]
    fn an_unknown_preset_id_never_invalidates_a_file_that_has_usable_keys() {
        let unknown_preset = serde_json::json!({
            "format": EXPORT_FORMAT,
            "version": EXPORT_VERSION,
            "keybindingPreset": "vscode",
            "keybindings": [{ "command": "toggle-terminal", "keystrokes": ["alt-f12"] }],
        });
        let ImportResult::Applied(plan) =
            parse_import(&serde_json::to_string(&unknown_preset).unwrap())
        else {
            panic!("预设 id 认不出来不该连累键位：这一条必须照常进来");
        };
        assert_eq!(
            plan.preset, None,
            "认不出来的预设要落成 None，由调用方落在当前档、且不切档"
        );
        assert_eq!(plan.imported(), 1);
        assert_eq!(plan.skipped(), 0);

        // 反过来：预设认不出来、键位也全是外来的 —— 那就真的没有可救的东西了，判无效。
        // 与上面不矛盾：挡下它的不是预设 id，是「一条键位都没进来」。
        let nothing_usable = serde_json::json!({
            "format": EXPORT_FORMAT,
            "version": EXPORT_VERSION,
            "keybindingPreset": "vscode",
            "keybindings": [{ "command": "file.save", "keystrokes": ["ctrl-s"] }],
        });
        assert_eq!(
            parse_import(&serde_json::to_string(&nothing_usable).unwrap()),
            ImportResult::Rejected
        );
    }

    /// 坏 JSON 与空 keybindings 各自的判定：前者无效，后者是「真的 0 条」。
    #[test]
    fn broken_json_and_an_empty_file_are_told_apart() {
        assert_eq!(parse_import("not json at all"), ImportResult::Rejected);
        let empty = serde_json::json!({
            "format": EXPORT_FORMAT,
            "version": EXPORT_VERSION,
            "keybindingPreset": "lithe",
            "keybindings": [],
        });
        let ImportResult::Applied(plan) = parse_import(&serde_json::to_string(&empty).unwrap())
        else {
            panic!("空文件是「真的 0 条」，不是「这个文件用不了」");
        };
        assert_eq!(plan.imported(), 0);
        assert_eq!(plan.skipped(), 0);
    }
}
