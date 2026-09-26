//! 工作区外观覆盖层：`.lithe/settings.json`（共享）与 `.lithe/settings.local.json`（本机）。
//!
//! ## 覆盖顺序（唯一一份实现）
//!
//! ```text
//! 内置默认 < 全局 settings.json < 工作区 .lithe/settings.json < 工作区 .lithe/settings.local.json
//! ```
//!
//! 合并的**唯一**落点是 [`resolve_effective`]：设置页显示的"生效值"、主题应用、字号转发
//! 都走它算出来的那一份 [`Settings`]。不要在任何别处再写一遍"项目优先"的判断 —— 两份实现
//! 漂移的表现是"界面显示 A、实际生效 B"，这是最难查的一类错（同 `project.rs` 的
//! `ToolchainPaths::resolve` 那条口径）。
//!
//! ## 只有外观类键允许被工作区覆盖
//!
//! [`AppearanceKey`] 就是这份白名单，**九个键**：主题四键（`theme` / `syncSystemTheme` /
//! `autoThemeLight` / `autoThemeDark`）+ 五个外观/字体键（`uiFontSize` / `fontSize` /
//! `fontFamily` / `monoFontFamily` / `terminalFontSize`）。
//! 语言、终端 shell、缩进、工具链、Git 相关键**只有全局层**：工作区文件里写了它们也
//! 不生效（它们会被当作未知键逐字保留，但不会改变生效值）。
//! 为什么这样切：维护者已经决定**允许**外观被工作区覆盖（团队统一主题与字号是真实需求），
//! 决策与验证证据见
//! `.agents/notes/implemented/feature/2026-09-27-appearance-workspace-overlay.md`；
//! 其余键的"项目层"是另一批工作，设计意图见
//! `.agents/notes/proposed/architecture/2026-09-26-workspace-configuration-layers.md` 第四节。
//!
//! ## 三条缓解措施（都是硬要求，不是可选项）
//!
//! 1. **独立文件**：覆盖只存在于 `.lithe/settings.json` / `settings.local.json`，
//!    全局设置文件里不出现任何项目字段（写入方见 `store.rs` 的 `commit_appearance`）。
//! 2. **来源可见**：[`appearance_sources`] 逐键给出这一刻的来源三态
//!    （团队设置 / 本项目的设置 / 你的个人覆盖），设置页据此画标注。
//! 3. **一键改回**：`SettingsStore::revert_appearance_to_global` 把键从**两层**工作区文件里
//!    删掉，于是生效值真的回落到全局值（不是留一个空值）。
//!
//! ## 文档语义照抄全局设置文件
//!
//! 读用 `lithe_gpui_shared::parse`（逐键容错 + 未知键保留 + 版本判定），写用
//! `preserve_unknown` + `save_json`（合并未知键 → 临时文件 → rename）。本模块**不**自己写
//! 第二版 version / 未知键 / 原子写逻辑。工作区设置文档的版本从 [`WORKSPACE_SETTINGS_VERSION`]
//! （= 1）开始。
//!
//! **未设的键不落盘**（不是写 `null`）：`null` 不是这份文档的形状，用户手改时也读不出
//! "这个键被清掉了"与"这个键没写过"的差别。所以写回时要把没设的键从文档对象里删掉 ——
//! 见 [`write_overlay`]。

use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::schema::Settings;

/// 工作区设置文档的版本。与全局设置文件**各自独立**（两处都是 1 只是巧合，不是共享常量）。
pub const WORKSPACE_SETTINGS_VERSION: u32 = 1;

/// 允许被工作区覆盖的外观键（白名单，**九键**）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppearanceKey {
    /// `theme`：配色主题 id。
    Theme,
    /// `syncSystemTheme`：是否跟随系统外观。
    SyncSystemTheme,
    /// `autoThemeLight`：跟随系统时的浅色主题。
    AutoThemeLight,
    /// `autoThemeDark`：跟随系统时的深色主题。
    AutoThemeDark,
    /// `uiFontSize`：界面字号。
    UiFontSize,
    /// `fontSize`：编辑器字号。
    EditorFontSize,
    /// `fontFamily`：界面字体族。
    FontFamily,
    /// `monoFontFamily`：代码字体族。
    MonoFontFamily,
    /// `terminalFontSize`：终端字号。
    TerminalFontSize,
}

impl AppearanceKey {
    /// 全部外观键（顺序 = 界面上的出现顺序，也是诊断行的顺序）。
    pub const ALL: [AppearanceKey; 9] = [
        AppearanceKey::Theme,
        AppearanceKey::SyncSystemTheme,
        AppearanceKey::AutoThemeLight,
        AppearanceKey::AutoThemeDark,
        AppearanceKey::UiFontSize,
        AppearanceKey::EditorFontSize,
        AppearanceKey::FontFamily,
        AppearanceKey::MonoFontFamily,
        AppearanceKey::TerminalFontSize,
    ];

    /// 这份文档里的 JSON 键名（与 `schema.rs` 的 serde 重命名逐字一致）。
    ///
    /// ⚠️ 这是**第二个**出现键名的地方（第一处是 `Settings` 的 `#[serde(rename)]`）。
    /// 它能存在是因为"从原始 JSON 对象里删掉一个键"这件事只能按字符串做，serde 帮不上忙；
    /// 守卫是测试 `json_keys_come_from_the_overlay_schema`：逐个键断言它出现在
    /// `known_keys::<AppearanceOverlay>()` 里 —— 改名却忘了改这里，那条测试立刻不等。
    pub fn json_key(self) -> &'static str {
        match self {
            Self::Theme => "theme",
            Self::SyncSystemTheme => "syncSystemTheme",
            Self::AutoThemeLight => "autoThemeLight",
            Self::AutoThemeDark => "autoThemeDark",
            Self::UiFontSize => "uiFontSize",
            Self::EditorFontSize => "fontSize",
            Self::FontFamily => "fontFamily",
            Self::MonoFontFamily => "monoFontFamily",
            Self::TerminalFontSize => "terminalFontSize",
        }
    }

    /// 诊断/日志里的短名（与 JSON 键名相同，避免出现第三套名字）。
    pub fn as_str(self) -> &'static str {
        self.json_key()
    }

    /// 由 JSON 键名反查（`--appearance-revert <键名>` 这类诊断入口用）。
    pub fn from_json_key(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|key| key.json_key() == value)
    }
}

/// 一个覆盖值（三选一：文本 / 数字 / 开关）。
#[derive(Debug, Clone, PartialEq)]
pub enum AppearanceValue {
    /// 文本值（主题 id、字体族）。
    Text(String),
    /// 数字值（三个字号）。
    Number(f64),
    /// 开关值（`syncSystemTheme`）。
    Flag(bool),
}

impl AppearanceValue {
    /// 从一份已经规范化过的设置里取这个键的值。
    ///
    /// 用途是"把钳制之后的值写回覆盖层"：钳制只有一份实现（[`Settings::normalize`]），
    /// 本模块不再抄一遍 `10..=24` 那些数字。
    pub fn from_settings(settings: &Settings, key: AppearanceKey) -> Self {
        match key {
            AppearanceKey::Theme => Self::Text(settings.theme.clone()),
            AppearanceKey::SyncSystemTheme => Self::Flag(settings.sync_system_theme),
            AppearanceKey::AutoThemeLight => Self::Text(settings.auto_theme_light.clone()),
            AppearanceKey::AutoThemeDark => Self::Text(settings.auto_theme_dark.clone()),
            AppearanceKey::UiFontSize => Self::Number(settings.ui_font_size),
            AppearanceKey::EditorFontSize => Self::Number(settings.font_size),
            AppearanceKey::FontFamily => Self::Text(settings.font_family.clone()),
            AppearanceKey::MonoFontFamily => Self::Text(settings.mono_font_family.clone()),
            AppearanceKey::TerminalFontSize => Self::Number(settings.terminal_font_size),
        }
    }
}

/// 一层工作区覆盖（`.lithe/settings.json` 或 `.lithe/settings.local.json`）。
///
/// `None` = 这一层**没有**这个键 → 由下一层（或全局层）决定；`Some` = 这一层定了它。
/// 注意这与"值等于默认值"是两件事：显式写 `"fontSize": 14` 也是覆盖。
///
/// ⚠️ 字段刻意**不加** `skip_serializing_if`：`lithe_gpui_shared::parse` 的逐键回落路径
/// 遍历 `known_keys::<T>()`（= `T::default()` 的序列化结果），默认值不序列化的字段会被
/// 当成未知键而读不回来（见 `document.rs` 模块文档的第 2 条硬约束）。代价是"未设的键"
/// 会序列化成 `null`，所以写回时用 [`AppearanceOverlay::unset_json_keys`] 把它们从文档里删掉。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceOverlay {
    /// 见 [`AppearanceKey::Theme`]。
    #[serde(rename = "theme")]
    pub theme: Option<String>,
    /// 见 [`AppearanceKey::SyncSystemTheme`]。
    #[serde(rename = "syncSystemTheme")]
    pub sync_system_theme: Option<bool>,
    /// 见 [`AppearanceKey::AutoThemeLight`]。
    #[serde(rename = "autoThemeLight")]
    pub auto_theme_light: Option<String>,
    /// 见 [`AppearanceKey::AutoThemeDark`]。
    #[serde(rename = "autoThemeDark")]
    pub auto_theme_dark: Option<String>,
    /// 见 [`AppearanceKey::UiFontSize`]。
    #[serde(rename = "uiFontSize")]
    pub ui_font_size: Option<f64>,
    /// 见 [`AppearanceKey::EditorFontSize`]。
    #[serde(rename = "fontSize")]
    pub font_size: Option<f64>,
    /// 见 [`AppearanceKey::FontFamily`]。
    #[serde(rename = "fontFamily")]
    pub font_family: Option<String>,
    /// 见 [`AppearanceKey::MonoFontFamily`]。
    #[serde(rename = "monoFontFamily")]
    pub mono_font_family: Option<String>,
    /// 见 [`AppearanceKey::TerminalFontSize`]。
    #[serde(rename = "terminalFontSize")]
    pub terminal_font_size: Option<f64>,
}

impl AppearanceOverlay {
    /// 这一层有没有设这个键。
    pub fn has(&self, key: AppearanceKey) -> bool {
        self.get(key).is_some()
    }

    /// 取这一层设的值；没设返回 `None`。
    pub fn get(&self, key: AppearanceKey) -> Option<AppearanceValue> {
        match key {
            AppearanceKey::Theme => self.theme.clone().map(AppearanceValue::Text),
            AppearanceKey::SyncSystemTheme => self.sync_system_theme.map(AppearanceValue::Flag),
            AppearanceKey::AutoThemeLight => {
                self.auto_theme_light.clone().map(AppearanceValue::Text)
            }
            AppearanceKey::AutoThemeDark => self.auto_theme_dark.clone().map(AppearanceValue::Text),
            AppearanceKey::UiFontSize => self.ui_font_size.map(AppearanceValue::Number),
            AppearanceKey::EditorFontSize => self.font_size.map(AppearanceValue::Number),
            AppearanceKey::FontFamily => self.font_family.clone().map(AppearanceValue::Text),
            AppearanceKey::MonoFontFamily => {
                self.mono_font_family.clone().map(AppearanceValue::Text)
            }
            AppearanceKey::TerminalFontSize => self.terminal_font_size.map(AppearanceValue::Number),
        }
    }

    /// 设这个键。值类型与键不匹配时**不做任何事**（不 panic、不写错类型）。
    pub fn set(&mut self, key: AppearanceKey, value: AppearanceValue) {
        match (key, value) {
            (AppearanceKey::Theme, AppearanceValue::Text(value)) => self.theme = Some(value),
            (AppearanceKey::SyncSystemTheme, AppearanceValue::Flag(value)) => {
                self.sync_system_theme = Some(value)
            }
            (AppearanceKey::AutoThemeLight, AppearanceValue::Text(value)) => {
                self.auto_theme_light = Some(value)
            }
            (AppearanceKey::AutoThemeDark, AppearanceValue::Text(value)) => {
                self.auto_theme_dark = Some(value)
            }
            (AppearanceKey::UiFontSize, AppearanceValue::Number(value)) => {
                self.ui_font_size = Some(value)
            }
            (AppearanceKey::EditorFontSize, AppearanceValue::Number(value)) => {
                self.font_size = Some(value)
            }
            (AppearanceKey::FontFamily, AppearanceValue::Text(value)) => {
                self.font_family = Some(value)
            }
            (AppearanceKey::MonoFontFamily, AppearanceValue::Text(value)) => {
                self.mono_font_family = Some(value)
            }
            (AppearanceKey::TerminalFontSize, AppearanceValue::Number(value)) => {
                self.terminal_font_size = Some(value)
            }
            _ => {}
        }
    }

    /// 清掉这个键（= 交还给下一层）。
    pub fn clear(&mut self, key: AppearanceKey) {
        match key {
            AppearanceKey::Theme => self.theme = None,
            AppearanceKey::SyncSystemTheme => self.sync_system_theme = None,
            AppearanceKey::AutoThemeLight => self.auto_theme_light = None,
            AppearanceKey::AutoThemeDark => self.auto_theme_dark = None,
            AppearanceKey::UiFontSize => self.ui_font_size = None,
            AppearanceKey::EditorFontSize => self.font_size = None,
            AppearanceKey::FontFamily => self.font_family = None,
            AppearanceKey::MonoFontFamily => self.mono_font_family = None,
            AppearanceKey::TerminalFontSize => self.terminal_font_size = None,
        }
    }

    /// 把这一层设了的键写进 `settings`，返回被它覆盖的键（按 [`AppearanceKey::ALL`] 顺序）。
    ///
    /// 调用方**最后**要再调一次 [`Settings::normalize`]（钳制只有那一个实现）。
    pub fn apply_to(&self, settings: &mut Settings) -> Vec<AppearanceKey> {
        let mut applied = Vec::new();
        for key in AppearanceKey::ALL {
            let Some(value) = self.get(key) else {
                continue;
            };
            match (key, value) {
                (AppearanceKey::Theme, AppearanceValue::Text(value)) => settings.theme = value,
                (AppearanceKey::SyncSystemTheme, AppearanceValue::Flag(value)) => {
                    settings.sync_system_theme = value
                }
                (AppearanceKey::AutoThemeLight, AppearanceValue::Text(value)) => {
                    settings.auto_theme_light = value
                }
                (AppearanceKey::AutoThemeDark, AppearanceValue::Text(value)) => {
                    settings.auto_theme_dark = value
                }
                (AppearanceKey::UiFontSize, AppearanceValue::Number(value)) => {
                    settings.ui_font_size = value
                }
                (AppearanceKey::EditorFontSize, AppearanceValue::Number(value)) => {
                    settings.font_size = value
                }
                (AppearanceKey::FontFamily, AppearanceValue::Text(value)) => {
                    settings.font_family = value
                }
                (AppearanceKey::MonoFontFamily, AppearanceValue::Text(value)) => {
                    settings.mono_font_family = value
                }
                (AppearanceKey::TerminalFontSize, AppearanceValue::Number(value)) => {
                    settings.terminal_font_size = value
                }
                _ => continue,
            }
            applied.push(key);
        }
        applied
    }

    /// 把这一层已经设了的键**按归一之后的值改写**（钳制沿用 [`Settings::normalize`]）。
    ///
    /// 例：文件里写了 `"fontSize": 99`，`settings`（= 归一之后的生效值）里是 `22`，
    /// 于是这一层被改写成 `22`。没设的键不受影响。
    pub fn clamp_with(&mut self, settings: &Settings) {
        for key in AppearanceKey::ALL {
            if self.has(key) {
                self.set(key, AppearanceValue::from_settings(settings, key));
            }
        }
    }

    /// 没设的键名（写回时要从 JSON 文档里删掉它们，而不是留 `null`）。
    fn unset_json_keys(&self) -> Vec<&'static str> {
        AppearanceKey::ALL
            .iter()
            .filter(|key| !self.has(**key))
            .map(|key| key.json_key())
            .collect()
    }
}

/// 外观值的来源。
///
/// 三态 + 回落，逐条对应设计 Note 第七节的表：
///
/// | 层 | 怎么判 | 界面怎么说 |
/// | --- | --- | --- |
/// | [`Self::ProjectLocal`] | `.lithe/settings.local.json` 设了它 | 「你的个人覆盖」 |
/// | [`Self::ProjectShared`] `tracked: true` | `.lithe/settings.json` 设了它，且**该文件已被 Git 跟踪** | 「团队设置」 |
/// | [`Self::ProjectShared`] `tracked: false` | 同上，但没被跟踪 | 「本项目的设置」 |
/// | [`Self::Global`] | 两层都没设 | 不画标注（跟随全局是默认形态） |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceSource {
    /// 跟随全局设置文件（没有工作区覆盖，或是本机层与共享层都没这个键）。
    Global,
    /// 来自 `.lithe/settings.json`；`tracked` 决定界面称它"团队设置"还是"本项目的设置"。
    ProjectShared {
        /// 这个文件是否已被 Git 跟踪（`git ls-files`）。
        tracked: bool,
    },
    /// 来自 `.lithe/settings.local.json`（个人的工作区覆盖）。
    ProjectLocal,
}

impl AppearanceSource {
    /// 界面文案的 locale 键。
    ///
    /// 三态**必须可区分**（设计 Note 的验收标准），所以三条文案各有一个键；
    /// [`Self::Global`] 的键只在"要不要画"的判断里用到，默认不画。
    pub fn locale_key(self) -> &'static str {
        match self {
            Self::Global => "settings.gpui.appearanceSourceGlobal",
            Self::ProjectShared { tracked: true } => "settings.gpui.appearanceSourceTeam",
            Self::ProjectShared { tracked: false } => "settings.gpui.appearanceSourceProject",
            Self::ProjectLocal => "settings.gpui.appearanceSourceLocal",
        }
    }

    /// 是不是"被工作区覆盖"（决定设置页画不画来源标注与「改回我的全局外观」按钮）。
    pub fn is_workspace_override(self) -> bool {
        !matches!(self, Self::Global)
    }
}

/// 一个键的来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppearanceKeySource {
    /// 哪个键。
    pub key: AppearanceKey,
    /// 它的来源。
    pub source: AppearanceSource,
}

/// 三层合并的**唯一**实现：内置默认 < 全局 < 共享 < 本机。
///
/// 只覆盖 [`AppearanceKey::ALL`] 里的九个键；其余键始终来自 `global`
/// （工作区文件里写了它们也不会生效，见模块文档）。
pub fn resolve_effective(
    global: &Settings,
    shared: &AppearanceOverlay,
    local: &AppearanceOverlay,
) -> Settings {
    let mut effective = global.clone();
    shared.apply_to(&mut effective);
    local.apply_to(&mut effective);
    // 钳制与规范化沿用既有实现：工作区里的越界值不会让界面变得不可用。
    effective.normalize();
    effective
}

/// 一个键这一刻的来源（判定顺序：本机 → 共享 → 全局）。
pub fn source_for(
    key: AppearanceKey,
    shared: &AppearanceOverlay,
    local: &AppearanceOverlay,
    shared_tracked: Option<bool>,
) -> AppearanceSource {
    if local.has(key) {
        return AppearanceSource::ProjectLocal;
    }
    if shared.has(key) {
        // 还没问到 Git 时按"尚未提交"显示：那是默认形态（`.lithe/` 默认不共享），
        // 而把没问到的情形说成"团队设置"会指向一个可能不存在的团队约定。
        return AppearanceSource::ProjectShared {
            tracked: shared_tracked.unwrap_or(false),
        };
    }
    AppearanceSource::Global
}

/// 逐键来源（顺序 = [`AppearanceKey::ALL`]）。
pub fn appearance_sources(
    shared: &AppearanceOverlay,
    local: &AppearanceOverlay,
    shared_tracked: Option<bool>,
) -> Vec<AppearanceKeySource> {
    AppearanceKey::ALL
        .iter()
        .map(|key| AppearanceKeySource {
            key: *key,
            source: source_for(*key, shared, local, shared_tracked),
        })
        .collect()
}

/// 决定当前生效主题的是哪一个键。
///
/// 跟随系统时主题由 `autoThemeLight` / `autoThemeDark` 决定（按系统明暗选一支），
/// 否则由 `theme` 决定。设置页的"主题"与"外观模式"两行都用它判来源 ——
/// 只标 `theme` 一个键是不够的：跟随系统时用户改的其实是 `autoTheme*`
/// （真源同一条口径，见 `store.rs` 的 `set_theme`）。
pub fn effective_theme_key(settings: &Settings, system_is_dark: bool) -> AppearanceKey {
    if !settings.sync_system_theme {
        return AppearanceKey::Theme;
    }
    if system_is_dark {
        AppearanceKey::AutoThemeDark
    } else {
        AppearanceKey::AutoThemeLight
    }
}

/// 一次读取一层覆盖的结果。
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedOverlay {
    /// 解析出来的覆盖（任何失败路径都返回可用的值，最差是全 `None`）。
    pub overlay: AppearanceOverlay,
    /// 文件里的原始对象；写回时用它保留未知键。
    pub previous: Option<Value>,
    /// 文件声明的版本高于本程序支持：**不得覆盖这个文件**。
    pub read_only: bool,
    /// 文件是否真的存在（不存在 = 这一层没有覆盖，且**不创建文件**）。
    pub file_existed: bool,
    /// 逐键诊断。调用方负责打印。
    pub diagnostics: Vec<String>,
}

/// 读一层工作区覆盖。**不创建文件、不 panic**。
pub fn load_overlay(path: &Path) -> LoadedOverlay {
    match lithe_gpui_shared::read_document_text(path) {
        Ok(Some(text)) => {
            let parsed = lithe_gpui_shared::parse::<AppearanceOverlay>(&text, WORKSPACE_SETTINGS_VERSION);
            LoadedOverlay {
                overlay: parsed.value,
                previous: parsed.previous,
                read_only: parsed.read_only,
                file_existed: true,
                diagnostics: parsed.diagnostics,
            }
        }
        Ok(None) => LoadedOverlay {
            overlay: AppearanceOverlay::default(),
            previous: None,
            read_only: false,
            file_existed: false,
            diagnostics: Vec::new(),
        },
        Err(error) => LoadedOverlay {
            overlay: AppearanceOverlay::default(),
            previous: None,
            read_only: false,
            file_existed: false,
            diagnostics: vec![format!("workspace_settings_read_failed error={error}")],
        },
    }
}

/// 生成要落盘的工作区设置文档：未设的键从文档里**删掉**、未知键逐字保留、写入 `version`。
///
/// "删掉"这一步与 `merge_document` 有实质差别：那份通用实现让 typed 一方胜出，
/// 而未设的键序列化成 `null`，于是清值之后文件里会留下 `"fontSize": null` ——
/// 那不是这份文档的形状（用户手改时读不出"清掉了"与"没写过"的差别），
/// 所以这里在通用 `preserve_unknown` 之上补一步删除。
pub fn merged_document(
    previous: Option<&Value>,
    overlay: &AppearanceOverlay,
) -> Result<Value, serde_json::Error> {
    let typed = serde_json::to_value(overlay)?;
    let mut merged = lithe_gpui_shared::preserve_unknown(previous, &typed);
    if let Value::Object(object) = &mut merged {
        for key in overlay.unset_json_keys() {
            object.remove(key);
        }
        object.insert(
            lithe_gpui_shared::DOCUMENT_VERSION_KEY.to_string(),
            Value::from(WORKSPACE_SETTINGS_VERSION),
        );
    }
    Ok(merged)
}

/// 原子写一层工作区覆盖，返回落盘的那份文档（调用方要拿它当下一次的 `previous`）。
///
/// 目录不存在时由 `save_json` 建出来（`.lithe/` 在打开项目时已经建过，这里是兜底）。
pub fn write_overlay(
    path: &Path,
    previous: Option<&Value>,
    overlay: &AppearanceOverlay,
) -> io::Result<Value> {
    let document = merged_document(previous, overlay)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    lithe_gpui_shared::save_json(path, &document)?;
    Ok(document)
}

/// 覆盖层的键集（由 schema 派生），给守卫测试与诊断用。
pub fn overlay_known_keys() -> std::collections::BTreeSet<String> {
    lithe_gpui_shared::known_keys::<AppearanceOverlay>()
}

/// 把一份覆盖写成可读的诊断片段（`fontSize,theme`），顺序 = [`AppearanceKey::ALL`]。
pub fn describe_keys(keys: &[AppearanceKey]) -> String {
    keys.iter()
        .map(|key| key.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

/// 从文档对象里取"这一层设了哪些键"（诊断用，不经过 typed 解析）。
pub fn keys_in_document(object: &Map<String, Value>) -> Vec<AppearanceKey> {
    AppearanceKey::ALL
        .iter()
        .copied()
        .filter(|key| object.contains_key(key.json_key()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{EDITOR_FONT_SIZE_MAX, TERMINAL_FONT_SIZE_UNSET, UI_FONT_SIZE_MAX};

    /// 独立的临时目录，用完删掉（目录名带调用方标记，避免并行测试互相踩）。
    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("lithe-ws-settings-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// 一份"全局层"：外观键与非外观键都不是默认值，于是"某个键没被覆盖"必定看得见。
    fn global_settings() -> Settings {
        let mut settings = Settings::default();
        settings.theme = "lithe-light".to_string();
        settings.sync_system_theme = false;
        settings.auto_theme_light = "lithe-light".to_string();
        settings.auto_theme_dark = "lithe-dark".to_string();
        settings.ui_font_size = 13.0;
        settings.font_size = 14.0;
        settings.font_family = "Global UI".to_string();
        settings.mono_font_family = "Global Mono".to_string();
        settings.terminal_font_size = 0.0;
        // 非外观键：工作区文件里出现它们**不许**生效。
        settings.display_language = "en-US".to_string();
        settings.tab_size = 8;
        settings.java_home_path = "D:\\global-jdk".to_string();
        settings.normalize();
        settings
    }

    /// 覆盖顺序：本机 > 共享 > 全局；没设的键回落到全局。
    #[test]
    fn precedence_is_global_then_shared_then_local() {
        let global = global_settings();

        let mut shared = AppearanceOverlay::default();
        shared.set(
            AppearanceKey::EditorFontSize,
            AppearanceValue::Number(18.0),
        );
        shared.set(
            AppearanceKey::FontFamily,
            AppearanceValue::Text("Shared UI".to_string()),
        );
        let mut local = AppearanceOverlay::default();
        local.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(20.0));

        let effective = resolve_effective(&global, &shared, &local);
        assert_eq!(effective.font_size, 20.0, "本机层赢");
        assert_eq!(
            effective.font_family, "Shared UI",
            "本机层没设的键由共享层决定"
        );
        assert_eq!(effective.theme, global.theme, "两层都没设 → 回落到全局");
        assert_eq!(effective.ui_font_size, global.ui_font_size);
    }

    /// **清掉覆盖之后真的回落到全局值**（一键改回守的正是这条）。
    #[test]
    fn clearing_a_layer_key_falls_back_to_the_global_value() {
        let global = global_settings();
        let mut shared = AppearanceOverlay::default();
        shared.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(18.0));
        let mut local = AppearanceOverlay::default();
        local.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(20.0));

        assert_eq!(resolve_effective(&global, &shared, &local).font_size, 20.0);

        local.clear(AppearanceKey::EditorFontSize);
        assert_eq!(
            resolve_effective(&global, &shared, &local).font_size,
            18.0,
            "清掉本机层 → 回落到共享层"
        );

        shared.clear(AppearanceKey::EditorFontSize);
        assert_eq!(
            resolve_effective(&global, &shared, &local).font_size,
            global.font_size,
            "两层都清掉 → 回落到全局值（不是空值、不是默认值）"
        );
    }

    /// **范围限定**：工作区文件里的非外观键不生效（语言 / 缩进 / 工具链都只有全局层）。
    #[test]
    fn non_appearance_keys_are_not_overridable() {
        let global = global_settings();
        let text = r#"{
          "fontSize": 18,
          "displayLanguage": "zh-CN",
          "tabSize": 2,
          "javaHomePath": "D:\\project-jdk",
          "terminalDefaultShellId": "cmd",
          "mavenSettingsPath": "D:\\project-settings.xml"
        }"#;
        let parsed = lithe_gpui_shared::parse::<AppearanceOverlay>(text, WORKSPACE_SETTINGS_VERSION);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let effective = resolve_effective(&global, &parsed.value, &AppearanceOverlay::default());

        assert_eq!(effective.font_size, 18.0, "外观键生效");
        assert_eq!(
            effective.display_language, global.display_language,
            "语言不是外观键，仍然只有全局层"
        );
        assert_eq!(effective.tab_size, global.tab_size, "缩进只有全局层");
        assert_eq!(
            effective.java_home_path, global.java_home_path,
            "工具链只有全局层"
        );
        assert_eq!(
            effective.terminal_default_shell_id, global.terminal_default_shell_id,
            "终端 shell 只有全局层"
        );
        assert_eq!(
            effective.maven_settings_path, global.maven_settings_path,
            "Maven 路径只有全局层"
        );
    }

    /// 钳制沿用既有实现：越界值被夹住，"不覆盖"语义不被夹到下界。
    #[test]
    fn clamping_is_reused_from_the_settings_normalizer() {
        let global = global_settings();
        let mut overlay = AppearanceOverlay::default();
        overlay.set(AppearanceKey::UiFontSize, AppearanceValue::Number(99.0));
        overlay.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(99.0));
        overlay.set(AppearanceKey::TerminalFontSize, AppearanceValue::Number(-3.0));

        let effective = resolve_effective(&global, &overlay, &AppearanceOverlay::default());
        assert_eq!(effective.ui_font_size, UI_FONT_SIZE_MAX);
        assert_eq!(effective.font_size, EDITOR_FONT_SIZE_MAX);
        assert_eq!(
            effective.terminal_font_size, TERMINAL_FONT_SIZE_UNSET,
            "负数必须归一到「不覆盖」，而不是被夹到最小字号"
        );

        // 归一之后的值要能写回这一层（否则界面上是 24、文件里是 99）。
        let mut clamped = overlay.clone();
        clamped.clamp_with(&effective);
        assert_eq!(
            clamped.get(AppearanceKey::UiFontSize),
            Some(AppearanceValue::Number(UI_FONT_SIZE_MAX))
        );
        assert_eq!(
            clamped.get(AppearanceKey::TerminalFontSize),
            Some(AppearanceValue::Number(TERMINAL_FONT_SIZE_UNSET))
        );
    }

    /// 来源三态：本机 > 共享 > 全局；共享层再按"是否被跟踪"分成团队 / 本项目。
    #[test]
    fn sources_report_the_highest_priority_layer() {
        let mut shared = AppearanceOverlay::default();
        shared.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(18.0));
        let mut local = AppearanceOverlay::default();
        local.set(
            AppearanceKey::FontFamily,
            AppearanceValue::Text("Local".to_string()),
        );

        assert_eq!(
            source_for(AppearanceKey::EditorFontSize, &shared, &local, Some(false)),
            AppearanceSource::ProjectShared { tracked: false }
        );
        assert_eq!(
            source_for(AppearanceKey::EditorFontSize, &shared, &local, Some(true)),
            AppearanceSource::ProjectShared { tracked: true }
        );
        assert_eq!(
            source_for(AppearanceKey::FontFamily, &shared, &local, Some(true)),
            AppearanceSource::ProjectLocal,
            "本机层优先于共享层，与它对不对得上 Git 无关"
        );
        assert_eq!(
            source_for(AppearanceKey::UiFontSize, &shared, &local, Some(false)),
            AppearanceSource::Global
        );
        // 还没问到 Git 时不能自称"团队设置"（那是更弱的承诺）。
        assert_eq!(
            source_for(AppearanceKey::EditorFontSize, &shared, &local, None),
            AppearanceSource::ProjectShared { tracked: false }
        );

        let sources = appearance_sources(&shared, &local, Some(true));
        assert_eq!(sources.len(), AppearanceKey::ALL.len());
        assert!(sources[0].source.is_workspace_override() == false);
    }

    /// 三条来源文案必须互不相同（"三种来源在界面上可以区分"是验收标准）。
    #[test]
    fn the_three_sources_have_distinct_locale_keys() {
        let keys = [
            AppearanceSource::Global.locale_key(),
            AppearanceSource::ProjectShared { tracked: true }.locale_key(),
            AppearanceSource::ProjectShared { tracked: false }.locale_key(),
            AppearanceSource::ProjectLocal.locale_key(),
        ];
        let unique: std::collections::BTreeSet<&str> = keys.iter().copied().collect();
        assert_eq!(unique.len(), keys.len(), "四种来源的文案键必须互不相同");
        // 团队设置与"本项目的设置"是同一个文件的两个状态，但界面文案必须是两句。
        assert_ne!(
            AppearanceSource::ProjectShared { tracked: true }.locale_key(),
            AppearanceSource::ProjectShared { tracked: false }.locale_key()
        );
    }

    /// 决定生效主题的键：跟随系统时是 `autoTheme*`，否则是 `theme`。
    #[test]
    fn the_decisive_theme_key_follows_the_sync_flag() {
        let mut settings = Settings::default();
        settings.sync_system_theme = false;
        assert_eq!(
            effective_theme_key(&settings, true),
            AppearanceKey::Theme
        );
        settings.sync_system_theme = true;
        assert_eq!(
            effective_theme_key(&settings, true),
            AppearanceKey::AutoThemeDark
        );
        assert_eq!(
            effective_theme_key(&settings, false),
            AppearanceKey::AutoThemeLight
        );
    }

    /// JSON 键名必须与覆盖层 schema 一致（改名只改一处时这条测试立刻不等）。
    #[test]
    fn json_keys_come_from_the_overlay_schema() {
        let known = overlay_known_keys();
        for key in AppearanceKey::ALL {
            assert!(
                known.contains(key.json_key()),
                "{} 不在 AppearanceOverlay 的键集里（改了 serde rename 却忘了改 json_key？）",
                key.json_key()
            );
            assert_eq!(
                AppearanceKey::from_json_key(key.json_key()),
                Some(key),
                "反查必须回到同一个键"
            );
        }
        assert_eq!(known.len(), AppearanceKey::ALL.len(), "键集规模必须相等");
        assert_eq!(AppearanceKey::from_json_key("displayLanguage"), None);
    }

    /// 文件往返：读 → 改一个键 → 落盘 → 再读，值一致、`version` 是当前版本、无 `.tmp` 残留。
    #[test]
    fn a_layer_round_trips_through_the_file() {
        let dir = temp_dir("round-trip");
        let path = dir.join("settings.json");

        let mut overlay = AppearanceOverlay::default();
        overlay.set(AppearanceKey::EditorFontSize, AppearanceValue::Number(18.0));
        overlay.set(
            AppearanceKey::MonoFontFamily,
            AppearanceValue::Text("JetBrains Mono".to_string()),
        );
        let document = write_overlay(&path, None, &overlay).expect("写入失败");
        assert!(!lithe_gpui_shared::tmp_path(&path).exists(), "临时文件必须被 rename 掉");

        let loaded = load_overlay(&path);
        assert!(loaded.file_existed);
        assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
        assert_eq!(loaded.overlay, overlay);
        assert_eq!(
            document.get(lithe_gpui_shared::DOCUMENT_VERSION_KEY),
            Some(&Value::from(WORKSPACE_SETTINGS_VERSION))
        );

        // 覆盖写：清掉一个字族键 → 文档里那个键**消失**（不是留 null）。
        let mut next = loaded.overlay.clone();
        next.clear(AppearanceKey::MonoFontFamily);
        let document =
            write_overlay(&path, loaded.previous.as_ref(), &next).expect("覆盖写失败");
        assert!(
            document.get("monoFontFamily").is_none(),
            "清掉的键必须从文档里删掉，而不是留一个 null：{document}"
        );
        assert_eq!(load_overlay(&path).overlay, next);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 未知键（含嵌套）在改过值之后仍然逐字保留。
    #[test]
    fn unknown_keys_survive_a_layer_write() {
        let dir = temp_dir("preserve");
        let path = dir.join("settings.json");
        std::fs::create_dir_all(&dir).expect("建目录失败");
        std::fs::write(
            &path,
            r#"{
  "version": 1,
  "fontSize": 18,
  "thirdParty": { "keepMe": [1, 2, 3], "alsoKeep": "yes" },
  "futureKey": true
}"#,
        )
        .expect("写初始文件失败");

        let loaded = load_overlay(&path);
        assert_eq!(
            loaded.overlay.get(AppearanceKey::EditorFontSize),
            Some(AppearanceValue::Number(18.0))
        );

        let mut next = loaded.overlay.clone();
        next.set(AppearanceKey::UiFontSize, AppearanceValue::Number(15.0));
        let document = write_overlay(&path, loaded.previous.as_ref(), &next).expect("写入失败");

        assert_eq!(
            document.get("thirdParty"),
            Some(&serde_json::json!({ "keepMe": [1, 2, 3], "alsoKeep": "yes" })),
            "未知的嵌套对象必须整体保留：{document}"
        );
        assert_eq!(document.get("futureKey"), Some(&Value::from(true)));
        assert_eq!(document.get("fontSize"), Some(&Value::from(18.0)));
        assert_eq!(document.get("uiFontSize"), Some(&Value::from(15.0)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 坏键只回落那一个键，同一份文件里的其它覆盖照用。
    #[test]
    fn a_single_broken_key_falls_back_alone() {
        let parsed = lithe_gpui_shared::parse::<AppearanceOverlay>(
            r#"{"fontSize": 18, "uiFontSize": "big", "theme": "lithe-light"}"#,
            WORKSPACE_SETTINGS_VERSION,
        );
        assert_eq!(
            parsed.value.get(AppearanceKey::EditorFontSize),
            Some(AppearanceValue::Number(18.0))
        );
        assert_eq!(
            parsed.value.get(AppearanceKey::Theme),
            Some(AppearanceValue::Text("lithe-light".to_string()))
        );
        assert_eq!(parsed.value.get(AppearanceKey::UiFontSize), None);
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|line| line.contains("bad_key key=uiFontSize")),
            "{:?}",
            parsed.diagnostics
        );
    }

    /// 读不存在的文件不创建它；文件损坏时整层当没设（不是 panic）。
    #[test]
    fn reading_a_missing_or_broken_file_degrades_safely() {
        let dir = temp_dir("missing");
        let path = dir.join("settings.json");
        let loaded = load_overlay(&path);
        assert!(!loaded.file_existed);
        assert_eq!(loaded.overlay, AppearanceOverlay::default());
        assert!(!path.exists(), "读取不得创建文件");

        std::fs::create_dir_all(&dir).expect("建目录失败");
        std::fs::write(&path, "{ not json ").expect("写文件失败");
        let loaded = load_overlay(&path);
        assert!(loaded.file_existed);
        assert_eq!(loaded.overlay, AppearanceOverlay::default());
        assert!(!loaded.diagnostics.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 版本高于本程序支持 → 照常读取，但标成只读（调用方不得覆盖用户的文件）。
    #[test]
    fn a_newer_document_version_is_read_only() {
        let parsed = lithe_gpui_shared::parse::<AppearanceOverlay>(
            r#"{"version": 99, "fontSize": 18}"#,
            WORKSPACE_SETTINGS_VERSION,
        );
        assert!(parsed.read_only);
        assert_eq!(
            parsed.value.get(AppearanceKey::EditorFontSize),
            Some(AppearanceValue::Number(18.0))
        );
        assert!(!lithe_gpui_shared::parse::<AppearanceOverlay>(
            r#"{"version": 1}"#,
            WORKSPACE_SETTINGS_VERSION
        )
        .read_only);
    }

    /// 诊断用的键名清单从文档对象直取（不经过 typed 解析）。
    #[test]
    fn keys_in_the_document_are_reported_in_ui_order() {
        let Value::Object(object) = serde_json::json!({
            "fontSize": 18,
            "theme": "lithe-light",
            "unknownKey": 1
        }) else {
            panic!("必须是对象");
        };
        assert_eq!(
            describe_keys(&keys_in_document(&object)),
            "theme,fontSize"
        );
    }
}
