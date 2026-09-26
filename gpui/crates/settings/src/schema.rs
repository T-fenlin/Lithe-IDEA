//! 设置的数据模型与规范化（**不碰文件、不碰 GPUI**，纯数据）。
//!
//! ## 键名与真源
//!
//! 字段的 JSON 键名是 **Windows 前端 settings 的键名**（逐字，驼峰），真源
//! `windows/tauri/src/features/settings/config/default-settings.ts`；每个字段的文档注释里写清
//! 「Windows 键 / 默认值 / 真源 `文件:行号`」。这样这份文件将来做导入/导出时不需要第二张映射表。
//!
//! ## 逐键默认回退：serde 字段级 `default`
//!
//! 结构体上是 `#[serde(default)]`：**缺哪个键就回退到哪个字段的默认值**，不是整份丢弃
//! （`windows/tauri/src/features/settings/lib/settings-persistence.ts:73-79` 的
//! 「从默认快照出发逐键覆盖」语义）。`serde_json::from_str::<Settings>("{}")` 因此等于
//! `Settings::default()` —— 见本文件末尾的测试。
//!
//! ⚠️ **类型损坏的键**（例如 `"uiFontSize": "big"`）会让整份反序列化失败，所以启动路径**不直接**
//! 用 `from_str`，而是走 [`crate::persistence::parse`] 的逐键容错解析
//! （坏键 → 默认值 + 一条诊断），`#[serde(default)]` 仍然负责"缺键"这一半语义。
//!
//! ## 只落「gpui 侧真的能生效」的键
//!
//! 取舍与理由见 `gpui/PLAN.md` 的「阶段 8」（v1 的 7 个键）与「阶段 14」（设置剩余页新增的
//! `fontSize` / `tabSize` / `terminalDefaultShellId`）；`gpui/research/windows/07-settings-ui.md`
//! §7.3 逐项列了哪些键在 gpui 侧能立刻生效、哪些没有对应子系统。**没有消费方的键不进来**
//! —— 那只会变成"存了没用"的设置项。

use serde::{Deserialize, Serialize};

/// UI 字体大小下限。真源 `windows/tauri/src/features/settings/lib/ui-font-size.ts:3`。
pub const UI_FONT_SIZE_MIN: f64 = 10.0;
/// UI 字体大小上限。真源 `ui-font-size.ts:4`。
pub const UI_FONT_SIZE_MAX: f64 = 24.0;
/// UI 字体大小的步长。真源 `ui-font-size.ts:5`。
pub const UI_FONT_SIZE_STEP: f64 = 0.5;
/// UI 字体大小默认值。真源 `ui-font-size.ts:6` → `config/typography-defaults.ts:14`
/// `DEFAULT_UI_FONT_SIZE = 13`。
pub const UI_FONT_SIZE_DEFAULT: f64 = 13.0;

/// 「显示语言」的默认值。真源 `default-settings.ts:92`（`displayLanguage: "zh-CN"`）。
pub const DEFAULT_DISPLAY_LANGUAGE: &str = "zh-CN";

/// 编辑器字号的下限 / 上限。真源 `macos-settings-panels.tsx:283-292` 的数字输入
/// `min={10} max={22}`（真实对话框的「编辑器」页）。
pub const EDITOR_FONT_SIZE_MIN: f64 = 10.0;
/// 见 [`EDITOR_FONT_SIZE_MIN`]。
pub const EDITOR_FONT_SIZE_MAX: f64 = 22.0;
/// 编辑器字号的默认值。真源 `default-settings.ts:53` 的 `fontSize: DEFAULT_CODE_FONT_SIZE`，
/// 而 `config/typography-defaults.ts:13` 里 `DEFAULT_CODE_FONT_SIZE = 14`。
pub const EDITOR_FONT_SIZE_DEFAULT: f64 = 14.0;

/// 制表符宽度的合法取值（**枚举序即下拉顺序**）。真源 `macos-settings-panels.tsx:313-325`
/// 的 `2 / 4 / 8`，显示时拼成「N 个空格」（`settings.mac.spaces`）。
pub const TAB_SIZES: [u32; 3] = [2, 4, 8];
/// 制表符宽度默认值。真源 `default-settings.ts:55`（`tabSize: 2`）——正好是
/// `EditorState` 自己的默认档（`gpui-base-0.6.6/src/input/editor/indent.rs:20-27`）。
pub const TAB_SIZE_DEFAULT: u32 = 2;

/// 「系统默认 shell」的内部取值：**空串**。真源 `default-settings.ts:87`
/// （`terminalDefaultShellId: ""`），对应下拉里的「系统默认」（`settings.mac.systemDefault`）。
pub const SHELL_SYSTEM_DEFAULT: &str = "";
/// 终端默认 shell 的合法取值（**枚举序即下拉顺序**）。真源 `macos-settings-panels.tsx:379-393`
/// 的 `"" / powershell / cmd / wsl`；这里的取值是**内部 id**（不是界面文案），
/// 解析成真实程序在 `lithe-gpui-terminal`（`profile.rs`）。
pub const TERMINAL_SHELL_IDS: [&str; 4] = [SHELL_SYSTEM_DEFAULT, "powershell", "cmd", "wsl"];

/// 把 `fontSize`（编辑器字号）归一到 `10..=22` 的整数。
///
/// Windows 那侧是 `<input type="number" min={10} max={22}>`（`macos-settings-panels.tsx:283-292`），
/// 步长没写、渲染出的是整数档；`normalizeUiFontSize` 那套「0.5 步长 + 两位小数」**只针对
/// `uiFontSize`**（`lib/ui-font-size.ts:10-19`），编辑器字号不该跟着它走。非有限值回落默认。
pub fn normalize_editor_font_size(value: f64) -> f64 {
    if !value.is_finite() {
        return EDITOR_FONT_SIZE_DEFAULT;
    }
    value
        .round()
        .clamp(EDITOR_FONT_SIZE_MIN, EDITOR_FONT_SIZE_MAX)
}

/// 「显示语言」的合法取值白名单（**枚举序即下拉顺序**）。
///
/// 真源 `windows/tauri/src/i18n/locale.ts:2` 的 `DISPLAY_LANGUAGES = ["en-US", "zh-CN"]`：
/// 顺序照抄真源（英文在前），**不按"默认值在前"重排** —— 下拉里谁在上面是可见规格的一部分。
/// 默认值是另一件事，由 [`DEFAULT_DISPLAY_LANGUAGE`]（`zh-CN`）决定。
pub const DISPLAY_LANGUAGES: [&str; 2] = ["en-US", DEFAULT_DISPLAY_LANGUAGE];

/// 配色主题默认值：**主题 id**（`themes[].id`），不是显示名、也不是注册表索引键。
///
/// ## 为什么持久化的是 id 而不是名字
///
/// 注册表按 `themes[].name` 索引（`gpui-component-0.6.6/src/theme/registry.rs:121,154`），
/// 但**名字是给人看的、随时可能改**（改一次名字，所有用户的设置文件里那一项就指向不存在
/// 的东西）。id 是机器标识：改名时把新名字写进 `name`、id 保持不变，用户的引用不受影响。
///
/// Windows 侧存的也是 id（`lithe-dark`，`default-settings.ts:99`），所以两者现在同构；
/// 主题 id 与显示名的对应关系由 [`ThemeIndex`] 表达，见 [`crate::theme`]。
pub const DEFAULT_THEME: &str = "lithe-dark";
/// 「首选浅色主题」的默认值（主题 id）。对应 Windows `autoThemeLight: "lithe-light"`
/// （`default-settings.ts:102`）。
pub const DEFAULT_AUTO_THEME_LIGHT: &str = "lithe-light";
/// 「首选深色主题」的默认值（主题 id）。对应 Windows `autoThemeDark: "lithe-dark"`
/// （`default-settings.ts:103`）。
pub const DEFAULT_AUTO_THEME_DARK: &str = "lithe-dark";

/// 主题显示名 → id 的**兜底**规则（`"VS Code Light+"` → `"vs-code-light"`）。
///
/// 只在主题条目**没有**写 `id` 时使用：gpui-kit 的内置主题（`Default Light` / `Default Dark`）
/// 与我们自己的文件都写了 id，但用户手写的主题文件不会写。规则是
/// "只保留字母数字（含非 ASCII），其余连续片段折叠成一个 `-`，转小写，不留首尾 `-`"。
///
/// 兜底而不是唯一来源：id 一旦由名字派生，"改名字就换 id"，用户设置里那条引用就断了 ——
/// 而那正是我们改用 id 要避免的事。所以**内置主题一律显式写 id**，这个函数只服务
/// 没写 id 的文件。结果为空时返回 `"theme"`（id 不允许是空串）。
pub fn slugify_theme_id(name: &str) -> String {
    let mut out = String::new();
    let mut pending_separator = false;
    for ch in name.chars() {
        if ch.is_alphanumeric() {
            if pending_separator && !out.is_empty() {
                out.push('-');
            }
            pending_separator = false;
            out.extend(ch.to_lowercase());
        } else if !out.is_empty() {
            pending_separator = true;
        }
    }
    if out.is_empty() {
        "theme".to_string()
    } else {
        out
    }
}

/// 一条主题：机器标识、显示名、明暗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeEntry {
    /// 机器标识（设置文件里存的就是它）。
    pub id: String,
    /// 显示名，同时也是注册表的索引键（`themes[].name`）。
    pub name: String,
    /// 是否深色（`mode == "dark"`）。
    pub dark: bool,
}

/// 主题 id ↔ 显示名 的映射表，由**主题文件的原始 JSON** 构建。
///
/// ## 为什么必须自己解析文件，而不是问注册表
///
/// 注册表的 `ThemeConfig` 只有 `name`（`gpui-component-0.6.6/src/theme/schema.rs:36-82`），
/// **没有** id 字段：`themes[].id` 是我们加进主题文件的、被上游静默忽略的字段。所以
/// "id 是什么"只能自己读原始 JSON 才知道。
///
/// ## 输入与去重
///
/// 每个元素是一份 `ThemeSet` 的 JSON 文本（不是路径）。解析失败或 `name` 为空的条目跳过：
/// 注册表那边也会忽略它们，索引跟着忽略才一致。**同一个 id 或同一个显示名重复时，
/// 先出现的赢** —— 所以结果只取决于输入顺序，调用方按文件名排序传入即可得到确定结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThemeIndex {
    entries: Vec<ThemeEntry>,
}

impl ThemeIndex {
    /// 从一组主题文件文本构建索引。
    pub fn from_documents(documents: impl IntoIterator<Item = String>) -> Self {
        let mut entries: Vec<ThemeEntry> = Vec::new();

        for document in documents {
            let Ok(set) = serde_json::from_str::<RawThemeSet>(&document) else {
                continue;
            };
            for theme in set.themes {
                let name = theme.name.trim();
                if name.is_empty() {
                    continue;
                }
                let id = match theme.id.as_deref().map(str::trim) {
                    Some(id) if !id.is_empty() => id.to_string(),
                    _ => slugify_theme_id(name),
                };
                let duplicate = entries
                    .iter()
                    .any(|entry| entry.id == id || entry.name == name);
                if duplicate {
                    continue;
                }
                entries.push(ThemeEntry {
                    id,
                    name: name.to_string(),
                    dark: theme.mode.as_deref() == Some("dark"),
                });
            }
        }

        Self { entries }
    }

    /// 一条都没有（主题目录还没播种、或一份文件都读不到）。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 表里全部条目（顺序 = 输入顺序）。
    pub fn entries(&self) -> &[ThemeEntry] {
        &self.entries
    }

    /// 按显示名找 id。
    pub fn id_for_name(&self, name: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.id.as_str())
    }

    /// 按 id 找显示名（界面要显示的是它）。
    pub fn name_for_id(&self, id: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.name.as_str())
    }

    /// 把设置文件里的值归一成 id：**同时接受 id 与显示名**。
    ///
    /// 这就是老设置文件的兼容路径：升级前文件里存的是显示名（`"Lithe Dark"`），
    /// 这条规则把它换成 `"lithe-dark"`，于是老文件不用迁移也能用、也不会回落默认主题。
    /// 精确匹配优先，最后才做一次不区分大小写的匹配（大小写写错不该导致"主题丢失"）。
    pub fn canonicalize(&self, value: &str) -> Option<String> {
        let value = value.trim();
        if value.is_empty() {
            return None;
        }
        if let Some(entry) = self.entries.iter().find(|entry| entry.id == value) {
            return Some(entry.id.clone());
        }
        if let Some(entry) = self.entries.iter().find(|entry| entry.name == value) {
            return Some(entry.id.clone());
        }
        let lowered = value.to_lowercase();
        self.entries
            .iter()
            .find(|entry| {
                entry.id.to_lowercase() == lowered || entry.name.to_lowercase() == lowered
            })
            .map(|entry| entry.id.clone())
    }

    /// 表里全部 id（顺序 = 输入顺序）。
    pub fn ids(&self) -> Vec<String> {
        self.entries.iter().map(|entry| entry.id.clone()).collect()
    }
}

/// 主题文件的**原始**形状：只要 id / name / mode 三个字段。
///
/// 刻意不复用 gpui-kit 的 `ThemeSet`：那个类型里没有 id，而且我们只想要索引需要的最小信息
/// （颜色、highlight 那些字段解析失败也不该让索引整体失效）。
#[derive(Deserialize)]
struct RawThemeSet {
    #[serde(default)]
    themes: Vec<RawTheme>,
}

#[derive(Deserialize)]
struct RawTheme {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    mode: Option<String>,
}

/// 全部设置。
///
/// **依赖方向**：本类型不引用任何 GPUI 类型，`App` / `Window` 只在
/// [`crate::store`] 里出现；落盘格式与 UI 因此可以各自单测。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// 配色主题 **id**（`themes[].id`）。
    ///
    /// - Windows 键：`theme`，默认 `lithe-dark`（`default-settings.ts:99`）—— 两侧现在同构。
    /// - gpui 取值：`lithe-dark`（见 [`DEFAULT_THEME`] 的说明）；显示名由
    ///   [`ThemeIndex::name_for_id`] 得到，注册表的索引键（`themes[].name`）也由它解析。
    /// - 老设置文件里存的是显示名（`"Lithe Dark"`）：读入时由
    ///   [`Self::normalize_with_themes`] 归一成 id，**不需要用户做迁移**。
    /// - 生效方式：**立即**（`Theme::change` + `apply_config` + `refresh_windows`）。
    #[serde(rename = "theme")]
    pub theme: String,

    /// 是否跟随系统外观（明/暗）。Windows 键 `syncSystemTheme`，默认 `false`
    /// （`default-settings.ts:101`）。
    ///
    /// 为 `true` 时生效主题取 [`Self::auto_theme_light`] / [`Self::auto_theme_dark`]
    /// 中与系统外观匹配的那个；系统外观变化由
    /// [`crate::store::SettingsStore::attach_window`] 注册的监听跟进。
    #[serde(rename = "syncSystemTheme")]
    pub sync_system_theme: bool,

    /// 跟随系统时使用的浅色主题。Windows 键 `autoThemeLight`，默认 `lithe-light`
    /// （`default-settings.ts:102`）。
    #[serde(rename = "autoThemeLight")]
    pub auto_theme_light: String,

    /// 跟随系统时使用的深色主题。Windows 键 `autoThemeDark`，默认 `lithe-dark`
    /// （`default-settings.ts:103`）。
    #[serde(rename = "autoThemeDark")]
    pub auto_theme_dark: String,

    /// UI 字体大小（px）。Windows 键 `uiFontSize`，默认 `13`（`default-settings.ts:91` →
    /// `typography-defaults.ts:14`），范围 `10..=24`、步长 `0.5`（`ui-font-size.ts:3-5`）。
    ///
    /// 生效方式：**立即**。换算到 gpui 的 rem 基准是 `font_size = 16px × (uiFontSize / 13)`
    /// （`gpui/research/windows/04-theme-and-components.md` §0.1；`Root::render` 每帧用它调
    /// `window.set_rem_size`，`gpui-component-0.6.6/src/root.rs:582`）。
    #[serde(rename = "uiFontSize")]
    pub ui_font_size: f64,

    /// 是否显示状态栏。Windows 键 `showStatusBar`，默认 `true`
    /// （`default-settings.ts:94`）。生效方式：**立即**（`ShellWorkspace::render` 条件渲染）。
    #[serde(rename = "showStatusBar")]
    pub show_status_bar: bool,

    /// 显示语言。Windows 键 `displayLanguage`，默认 `zh-CN`（`default-settings.ts:92`）。
    ///
    /// 取值是 Windows 的 BCP-47 标签（`zh-CN` / `en-US`），落到 gpui 时经
    /// [`Self::gpui_locale`] 映射成 yml 的 locale 名。
    /// ⚠️ **生效方式：重启后**（gpui 侧 `set_locale` 只在启动早期调一次，见
    /// `gpui/crates/app/src/main.rs` 的启动顺序说明）。
    #[serde(rename = "displayLanguage")]
    pub display_language: String,

    /// 编辑器字号（px）。Windows 键 `fontSize`，默认 `14`（`default-settings.ts:53` →
    /// `typography-defaults.ts:13` 的 `DEFAULT_CODE_FONT_SIZE`），范围 `10..=22`
    /// （真实对话框的数字输入，`macos-settings-panels.tsx:283-292`）。
    ///
    /// 生效方式：**立即**。落到 gpui 的主题 token `Theme::mono_font_size`
    /// （编辑器正文用它，`gpui-component-0.6.6/src/input/editor.rs:137-143`），
    /// 由 [`crate::theme::apply_editor_font_size`] 在每次应用主题之后补写
    /// （`apply_config` 会把主题文件里的字体档写回来，顺序不能反）。
    ///
    /// ⚠️ **这是"编辑器字号"在 gpui 侧唯一的全局落点**：真源把 `fontSize` 与
    /// `terminalFontSize` 分成两个键（`default-settings.ts:53,76`），而 gpui 的主题只有
    /// 一个 `mono_font_size`，终端正文也用它。真实对话框的「终端」页只有「默认 Shell」
    /// 一项（`macos-settings-panels.tsx:372-396`），所以本侧不新增 `terminalFontSize` 控件
    /// —— 那会是"存了没用"的键。副作用（终端字号跟着编辑器走）登记在
    /// `gpui/PLAN.md` 与设置页的描述里。
    #[serde(rename = "fontSize")]
    pub font_size: f64,

    /// 制表符宽度。Windows 键 `tabSize`，默认 `2`（`default-settings.ts:55`），
    /// 取值域 [`TAB_SIZES`]。
    ///
    /// 生效方式：**立即**（对**所有已打开**的 buffer 与之后新开的都生效）——
    /// 由外壳把值转发给 `EditorPane::set_tab_size`，后者写进每个 `EditorState`
    /// （`gpui-base-0.6.6/src/input/editor/indent.rs:504` 的 `set_tab_size`）。
    #[serde(rename = "tabSize")]
    pub tab_size: u32,

    /// 终端默认 Shell 的内部 id。Windows 键 `terminalDefaultShellId`，默认 `""`
    /// （`default-settings.ts:87`），取值域 [`TERMINAL_SHELL_IDS`]。
    ///
    /// 生效方式：**只影响之后新建的终端会话**（真源描述原文
    /// `settings.mac.defaultShellDescription` =「用于新的终端会话。」，
    /// `macos-settings-panels.tsx:379-393`）。已开的会话不会换 shell ——
    /// 这一点与 Windows 完全一致，所以界面上照抄了那句描述。
    #[serde(rename = "terminalDefaultShellId")]
    pub terminal_default_shell_id: String,

    /// 丢弃更改前是否先弹确认框。Windows 键 `confirmBeforeDiscard`，默认 `true`
    /// （`default-settings.ts:194`；真源渲染点 `tabs/git-settings.tsx:93-106`）。
    ///
    /// 生效方式：**立即**，落到左栏「源代码管理」的丢弃路径 ——
    /// `ChangesView::set_confirm_before_discard` 由外壳订阅本实体后转发
    /// （与 `tabSize` / `terminalDefaultShellId` 同一条"值型设置经外壳转发"的路子）。
    /// 关掉之后走的是**同一条** `git.write discard` 写操作，被省掉的只有弹窗那一段。
    #[serde(rename = "confirmBeforeDiscard")]
    pub confirm_before_discard: bool,

    /// JDK 主目录的**覆盖值**（空串 = 用自动检测到的那个）。键名 `javaHomePath`。
    ///
    /// ## 为什么键名与真源的**运行配置**字段逐字相同
    ///
    /// 真源这一页（`components/project-environment-settings.tsx`）读写的是**项目级**文件
    /// （`.lithe/run/local.json`，经 `services/project-environment.ts` 的
    /// `runConfig.updateOptions`），它携带的 toolchain 对象就是
    /// `{ javaHomePath, mavenExecutablePath, mavenJavaHomePath }`
    /// （`shared/contracts/rust-core-api.md:1625-1627`）—— 这三个键名是 Core 契约里的名字。
    ///
    /// ## gpui 侧为什么只能落在**全局**设置文件里（如实登记）
    ///
    /// 1. 本 crate 的依赖方向是"只向下依赖 `gpui-kit` + `lithe-gpui-shared`"
    ///    （见 `lib.rs` 的模块文档），拿不到工作区根；设置对话框里唯一能拿到根的那条路是
    ///    宿主钩子（如 `identity::set_git_identity_host`），而登记钩子的 `workbench` 不在本次写域内；
    /// 2. gpui 侧**没有项目级存储**（`.lithe/run/local.json` 的读写归 Core 的 `runConfig.*`，
    ///    本侧还没有那条通路）。
    ///
    /// 所以这三个键是**机器级（全局）**的覆盖值，页面文案也必须如实这么写
    /// （`settings.gpui.projectScopeGlobal`）——**不假装**是项目级。
    /// 键名故意选 Core 契约里那三个，是为了将来接上项目级存储时，
    /// 落盘形状与 `runConfig.updateOptions` 的 toolchain 载荷**同名同义**，不需要第二张映射表。
    ///
    /// 生效方式：**立即**（本页"生效值"那一行当帧就按新草稿重算；真正的消费方是运行配置，
    /// 而 gpui 侧还没有采购它的一页）。
    #[serde(rename = "javaHomePath")]
    pub java_home_path: String,

    /// Maven 主目录 / 可执行文件的覆盖值。键名与语义见 [`Self::java_home_path`]。
    #[serde(rename = "mavenExecutablePath")]
    pub maven_executable_path: String,

    /// Maven 使用的 JDK 主目录的覆盖值（空串 = 跟随 [`Self::java_home_path`] 的生效值）。
    /// 键名与语义见 [`Self::java_home_path`]。
    #[serde(rename = "mavenJavaHomePath")]
    pub maven_java_home_path: String,

    /// Maven 用户 `settings.xml` 的覆盖值（空串 = 用检测到的那一份）。键名 `mavenSettingsPath`。
    ///
    /// ## 这一个键在真源里没有对应的**全局**键（如实登记）
    ///
    /// 真源把 `settings.xml` 与本地仓库写进 **Maven 工具窗的项目本地配置**
    /// （`components/project-environment-settings.tsx:287-314` 读写 `maven.settingsPath` /
    /// `maven.localRepositoryPath`，落点是项目级 `.lithe` 文档），**不经过**全局设置文件；
    /// 而 gpui 侧没有项目级存储（理由逐条见 [`Self::java_home_path`]）。
    ///
    /// 所以这两个键是**机器级（全局）**覆盖值，字段名沿用真源那一对同名同义的名字，
    /// 将来接上项目级存储时仍然不需要第二张映射表。
    ///
    /// ## 生效方式（两个键不一样，分别是"真生效"与"尚无消费方"）
    ///
    /// - `mavenSettingsPath` **有真消费方**：`workbench` 把它登记进 `lithe-gpui-java` 的覆盖槽
    ///   （`java/src/toolchain.rs`），JDT LS 启动时随 `mavenContext.settingsPath` 交给 Core，
    ///   Core 发布成 `java.configuration.maven.userSettings`
    ///   （`shared/contracts/rust-core-api.md:1170-1178`）。时机与 [`Self::java_home_path`] 相同：
    ///   下一次语言服务启动（今天是重启应用）。
    /// - `mavenLocalRepositoryPath` 今天**没有消费方**：本地仓库只在 `maven.launchPlan`
    ///   （`rust-core-api.md:1477-1487` 的 `-Dmaven.repo.local=<path>`）里用，而 gpui 侧还没有
    ///   执行 Maven 的通路。页面照样画出它，但**如实标注**"尚未生效"（不自称已生效）。
    #[serde(rename = "mavenSettingsPath")]
    pub maven_settings_path: String,

    /// Maven 本地仓库的覆盖值。键名与生效方式见 [`Self::maven_settings_path`]。
    #[serde(rename = "mavenLocalRepositoryPath")]
    pub maven_local_repository_path: String,

    /// 自动补全（输入时是否自动弹出补全菜单）。Windows 键 `autoCompletion`，默认 `true`
    /// （`default-settings.ts:153`；真源渲染点 `macos-settings-panels.tsx:406-415`）。
    ///
    /// ## 为什么只有这一个 LSP 键
    ///
    /// 真源 LSP 页有三个开关（`autoCompletion` / `parameterHints` / `semanticTokens`），
    /// 只有本键在 gpui 侧**有真消费方**：
    ///
    /// - `parameterHints`：上游 `gpui-base-0.6.6` 的 `Lsp` 结构体**没有**签名帮助接口
    ///   （`input/editor/lsp/mod.rs:39-69` 的 provider 清单里没有，全 crate `SignatureHelp`
    ///   零命中）⇒ 画一个永远不生效的开关违反"不画假控件"的口径，所以**连置灰都不画**；
    /// - `semanticTokens`：上游 trait 在（`lsp/semantic_tokens.rs:36-55`）但 Java 侧零实现、
    ///   零装载点 ⇒ 那是"新做一个特性"，不是"接一个开关"，本批不做、页面也不画。
    ///
    /// ## 生效方式与边界
    ///
    /// **立即**：落到 `lithe-gpui-editor` 的补全 provider 触发判据
    /// （`editor/src/completion.rs` 的 `is_completion_trigger`），由外壳在每次设置变化后经
    /// `EditorPane::set_auto_completion` 转发（与 `tabSize` / `terminalDefaultShellId`
    /// 同一条"值型设置经外壳转发"的路子）。
    ///
    /// ⚠️ 关掉的是**自动弹出**：JDTLS 补全与 Core 的轻量兜底（同一个 provider 的两个数据源）
    /// 都随之不再自动弹菜单。provider 本身**保留**（不卸载），因为那是将来做手动触发
    /// （`Ctrl+Space`）的基础；真源的描述句只说"活动语言服务器提供的补全建议"，
    /// 落到本侧会在实现里点明"关掉 = 不再自动弹出"。
    #[serde(rename = "autoCompletion")]
    pub auto_completion: bool,

    /// 选择打开其他项目时是否**每次询问**（弹「你想在哪里打开项目"X"？」）。
    /// Windows 键 `askWhereToOpenProjects`，默认 `true`（`default-settings.ts:111`）。
    ///
    /// 生效方式：**立即**，判据在 `workbench` 的换项目决策里（照真源
    /// `chooseProjectOpenDestination`，`project-open-destination.ts:78-110` 的第 ③ 段）：
    /// 为假时按 [`Self::open_folders_in_new_window`] 直接定目的地，不再弹对话框。
    /// 对话框里勾上「不再询问」之后由换项目那条链路**写这个键为假**
    /// （真源 `:112-131` 的 `rememberAfterOpen`，且是"先打开成功、后写偏好"）。
    #[serde(rename = "askWhereToOpenProjects")]
    pub ask_where_to_open_projects: bool,

    /// 打开项目时默认在**新窗口**里打开。Windows 键 `openFoldersInNewWindow`，
    /// 默认 `true`（`default-settings.ts:112`）。
    ///
    /// 生效方式：**立即**，只在 [`Self::ask_where_to_open_projects`] 为假时被读到
    /// （真源同一条决策的第 ③ 段）。⚠️ gpui 侧目前**没有多窗口**（维护者决定，规格 §5），
    /// 所以这个键取真时那条决策会落到"给提示"而不是真的开第二个窗口——
    /// 键本身照真源存取，缺的是窗口级句柄路由（B5 暂缓）。
    #[serde(rename = "openFoldersInNewWindow")]
    pub open_folders_in_new_window: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_string(),
            sync_system_theme: false,
            auto_theme_light: DEFAULT_AUTO_THEME_LIGHT.to_string(),
            auto_theme_dark: DEFAULT_AUTO_THEME_DARK.to_string(),
            ui_font_size: UI_FONT_SIZE_DEFAULT,
            show_status_bar: true,
            display_language: DEFAULT_DISPLAY_LANGUAGE.to_string(),
            font_size: EDITOR_FONT_SIZE_DEFAULT,
            tab_size: TAB_SIZE_DEFAULT,
            terminal_default_shell_id: SHELL_SYSTEM_DEFAULT.to_string(),
            confirm_before_discard: true,
            // 空串 = 自动检测（真源也是 `""` 表示"用自动值"）。
            java_home_path: String::new(),
            maven_executable_path: String::new(),
            maven_java_home_path: String::new(),
            // Maven 自己那份配置的两个覆盖值同样默认"自动"（用检测到 / 推导出来的值）。
            maven_settings_path: String::new(),
            maven_local_repository_path: String::new(),
            // 真源默认 `true`（`default-settings.ts:153`）—— 默认就该"打字有提示"。
            auto_completion: true,
            // 「打开其他项目」的两个键都照真源默认 `true`（`default-settings.ts:111-112`）：
            // 默认每次询问，且（真源口径下）默认在新窗口打开。
            ask_where_to_open_projects: true,
            open_folders_in_new_window: true,
        }
    }
}

/// 把 `uiFontSize` 归一到 `10..=24`、`0.5` 步长、两位小数。
///
/// 逐行照 `windows/tauri/src/features/settings/lib/ui-font-size.ts:10-19` 的
/// `normalizeUiFontSize`（先四舍五入到步长、再夹范围、最后定点到两位）；非有限值回落默认。
pub fn normalize_ui_font_size(value: f64) -> f64 {
    if !value.is_finite() {
        return UI_FONT_SIZE_DEFAULT;
    }
    let snapped = (value / UI_FONT_SIZE_STEP).round() * UI_FONT_SIZE_STEP;
    let clamped = snapped.clamp(UI_FONT_SIZE_MIN, UI_FONT_SIZE_MAX);
    // `(x * 100).round() / 100` 就是 JS 的 `Number(clamped.toFixed(2))`；
    // 用整数运算避免 `0.1 + 0.2` 那类二进制尾差。
    (clamped * 100.0).round() / 100.0
}

/// UI 字体大小 → gpui 主题基准字号（px）。
///
/// 换算口径：`font_size = 16 × (uiFontSize / 13)`（`ui-font-size.ts:30-33` 的
/// `getUiFontScale` 是 `uiFontSize / 13`，乘 16 是因为 gpui 的 rem 基准在主题里写作
/// `font.size: 16.0`，见 `gpui/themes/lithe-dark.json:9`）。默认 13 → 16px，其余档位等比例缩放。
pub fn theme_font_size_for(ui_font_size: f64) -> f32 {
    16.0 * (normalize_ui_font_size(ui_font_size) / UI_FONT_SIZE_DEFAULT) as f32
}

impl Settings {
    /// gpui 侧的 locale 名（yml 的 file stem）。
    ///
    /// Windows 存的是 `en-US`，而本仓库的 locale 文件是 `lithe.en.yml`
    /// （`rust_i18n::set_locale("en")`，见 `gpui/crates/shared/src/i18n.rs:271`），
    /// 所以要映射一次；没有 `en-US` 这个 locale 表。
    pub fn gpui_locale(&self) -> &'static str {
        match self.display_language.as_str() {
            "en-US" => "en",
            _ => "zh-CN",
        }
    }

    /// 规范化：只做**不依赖运行时状态**的部分（枚举白名单 / 范围钳制 / 空串回退）。
    ///
    /// 对应 `windows/tauri/src/features/settings/lib/settings-normalization.ts` 里
    /// **对 v1 生效键适用**的条目：
    ///
    /// | 条目 | 真源 | 这里怎么落 |
    /// | --- | --- | --- |
    /// | `uiFontSize` 归一 | `settings-normalization.ts:480` | [`normalize_ui_font_size`] |
    /// | 枚举白名单 | `settings-normalization.ts:499-519` 的白名单集合写法 | `displayLanguage` / `tabSize` / `terminalDefaultShellId` 白名单 |
    /// | `fontSize` 范围钳制 | `macos-settings-panels.tsx:283-292` 的 `min=10 max=22`（真源**没有**单独一条归一，范围由控件属性兜） | [`normalize_editor_font_size`] |
    ///
    /// 主题名是不是"注册表里真有这个主题"要拿到 `App` 才能判断，那条在
    /// [`Self::normalize_with_themes`]。
    ///
    /// §4.3 里的数组类条目（`hiddenFilePatterns` 等 trim/去空行、四个"项目顺序"数组）**不适用**：
    /// v1 没有任何数组键，做了也没有消费方。
    pub fn normalize(&mut self) {
        self.ui_font_size = normalize_ui_font_size(self.ui_font_size);
        self.font_size = normalize_editor_font_size(self.font_size);

        if !TAB_SIZES.contains(&self.tab_size) {
            self.tab_size = TAB_SIZE_DEFAULT;
        }

        if !TERMINAL_SHELL_IDS.contains(&self.terminal_default_shell_id.as_str()) {
            self.terminal_default_shell_id = SHELL_SYSTEM_DEFAULT.to_string();
        }

        if !DISPLAY_LANGUAGES.contains(&self.display_language.as_str()) {
            self.display_language = DEFAULT_DISPLAY_LANGUAGE.to_string();
        }

        // 空串是"这个键被手改坏了"，回落默认；非空的名字留给注册表校验。
        if self.theme.trim().is_empty() {
            self.theme = DEFAULT_THEME.to_string();
        }
        if self.auto_theme_light.trim().is_empty() {
            self.auto_theme_light = DEFAULT_AUTO_THEME_LIGHT.to_string();
        }
        if self.auto_theme_dark.trim().is_empty() {
            self.auto_theme_dark = DEFAULT_AUTO_THEME_DARK.to_string();
        }

        // 三个工具链覆盖值：**去掉首尾空白**（粘贴路径最容易带上空格 / 换行），
        // 空串保持空串（= 自动）。路径本身**不在这里校验** —— "这个目录里到底有没有
        // `bin/java`"要碰文件系统与子进程，归 `project.rs` 的探测层，且结论只影响
        // "生效值"那一行的显示，不改变落盘的值。
        self.java_home_path = self.java_home_path.trim().to_string();
        self.maven_executable_path = self.maven_executable_path.trim().to_string();
        self.maven_java_home_path = self.maven_java_home_path.trim().to_string();
        self.maven_settings_path = self.maven_settings_path.trim().to_string();
        self.maven_local_repository_path = self.maven_local_repository_path.trim().to_string();
    }

    /// 再补一层"主题必须在索引里"的规范化，并把**老值（显示名）归一成 id**。
    ///
    /// 只把**不存在的主题**换成默认，不动存在的那一个：用户可能装了自定义主题，
    /// 而那些主题在启动早期还没读到。索引为空的语义与"注册表还没就绪"一致 —— 不做判断。
    ///
    /// ⚠️ 这里会把内存里的显示名换成 id，但**不写回文件**：下一次因为别的原因落盘时才会写成
    /// id。这样"老文件继续可用"和"不做一次无关的强制迁移"两件事同时成立。
    pub fn normalize_with_themes(&mut self, index: &ThemeIndex) {
        if index.is_empty() {
            return;
        }
        self.theme = index
            .canonicalize(&self.theme)
            .unwrap_or_else(|| DEFAULT_THEME.to_string());
        self.auto_theme_light = index
            .canonicalize(&self.auto_theme_light)
            .unwrap_or_else(|| DEFAULT_AUTO_THEME_LIGHT.to_string());
        self.auto_theme_dark = index
            .canonicalize(&self.auto_theme_dark)
            .unwrap_or_else(|| DEFAULT_AUTO_THEME_DARK.to_string());
    }

    /// 按系统外观解析出**当前应该生效**的主题名。
    ///
    /// `system_is_dark` 由调用方从 `Window::appearance()` 或 `App::window_appearance()`
    /// 得到（生成 `ThemeMode` 的映射见 `gpui-component-0.6.6/src/theme/mod.rs:722-727`）。
    pub fn effective_theme(&self, system_is_dark: bool) -> &str {
        if !self.sync_system_theme {
            return &self.theme;
        }
        if system_is_dark {
            &self.auto_theme_dark
        } else {
            &self.auto_theme_light
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认值必须与 Windows 的默认值一一对应（改错这里等于改错产品默认外观）。
    #[test]
    fn defaults_match_the_windows_truth() {
        let settings = Settings::default();
        // 主题持久化的是 **id**（与 Windows 的 `lithe-dark` / `lithe-light` 同构）。
        assert_eq!(settings.theme, "lithe-dark");
        assert!(!settings.sync_system_theme);
        assert_eq!(settings.auto_theme_light, "lithe-light");
        assert_eq!(settings.auto_theme_dark, "lithe-dark");
        assert_eq!(settings.ui_font_size, 13.0);
        assert!(settings.show_status_bar);
        assert_eq!(settings.display_language, "zh-CN");
        assert_eq!(settings.font_size, 14.0);
        assert_eq!(settings.tab_size, 2);
        assert_eq!(settings.terminal_default_shell_id, "");
        assert!(settings.confirm_before_discard);
        // 「项目 · JDK 与 Maven」页的五个覆盖值默认都是"自动"（空串）。
        assert_eq!(settings.java_home_path, "");
        assert_eq!(settings.maven_executable_path, "");
        assert_eq!(settings.maven_java_home_path, "");
        // Maven 自己那份配置（`settings.xml` / 本地仓库）的覆盖值同样默认为空。
        assert_eq!(settings.maven_settings_path, "");
        assert_eq!(settings.maven_local_repository_path, "");
        // LSP 页的 `autoCompletion` 默认 `true`（真源 `default-settings.ts:153`）。
        assert!(settings.auto_completion);
        // 「打开其他项目」的两个键默认都是 `true`（真源 `default-settings.ts:111-112`）：
        // 默认每次询问在哪打开，且真源默认偏好"新窗口"。
        assert!(settings.ask_where_to_open_projects);
        assert!(settings.open_folders_in_new_window);
    }

    /// 字段级 `default`：**缺键**回退到该字段默认值（不是整份丢弃）。
    #[test]
    fn missing_keys_fall_back_per_field() {
        let settings: Settings = serde_json::from_str("{}").expect("空对象应当能反序列化");
        assert_eq!(settings, Settings::default());

        // 只给一个键：其余键保持默认，给的键生效。
        let settings: Settings =
            serde_json::from_str(r#"{"uiFontSize": 18.0}"#).expect("部分键应当能反序列化");
        assert_eq!(settings.ui_font_size, 18.0);
        assert_eq!(settings.theme, DEFAULT_THEME);
        assert_eq!(settings.display_language, DEFAULT_DISPLAY_LANGUAGE);
    }

    /// 未知键必须被**忽略**（用户可能手改过文件，也可能装过别的版本的 Lithe）。
    #[test]
    fn unknown_keys_are_ignored() {
        let settings: Settings = serde_json::from_str(
            r#"{"theme": "Lithe Light", "iconTheme": "idea-icons", "nope": {"a": 1}}"#,
        )
        .expect("未知键不该让解析失败");
        assert_eq!(settings.theme, "Lithe Light");
    }

    /// 序列化用 Windows 的驼峰键名（落盘格式与真源逐字一致）。
    #[test]
    fn serializes_with_windows_key_names() {
        let json = serde_json::to_string(&Settings::default()).expect("序列化失败");
        for key in [
            "\"theme\"",
            "\"syncSystemTheme\"",
            "\"autoThemeLight\"",
            "\"autoThemeDark\"",
            "\"uiFontSize\"",
            "\"showStatusBar\"",
            "\"displayLanguage\"",
            // 阶段 14 新增的三个键同样必须用 Windows 的驼峰键名落盘。
            "\"fontSize\"",
            "\"tabSize\"",
            "\"terminalDefaultShellId\"",
            // 阶段 15（「Git」页）新增的键。
            "\"confirmBeforeDiscard\"",
            // 「项目 · JDK 与 Maven」页的三个覆盖值：键名逐字取自 Core 契约
            // `runConfig.updateOptions` 的 toolchain 载荷（`rust-core-api.md:1625-1627`）。
            "\"javaHomePath\"",
            "\"mavenExecutablePath\"",
            "\"mavenJavaHomePath\"",
            // 同页 Maven 自己那份配置的两个覆盖值（真源的项目级字段名，见字段文档）。
            "\"mavenSettingsPath\"",
            "\"mavenLocalRepositoryPath\"",
            // 阶段 18（「LSP」页）：真源三键里唯一有消费方的那一个。
            "\"autoCompletion\"",
            // 「打开其他项目」的两个键（B4）：真源 `default-settings.ts:111-112` 的键名。
            "\"askWhereToOpenProjects\"",
            "\"openFoldersInNewWindow\"",
        ] {
            assert!(json.contains(key), "缺少键 {key}：{json}");
        }
    }

    /// 五个覆盖值：粘贴路径带的首尾空白被去掉，空串仍是空串（= 自动）。
    #[test]
    fn toolchain_overrides_are_trimmed() {
        let mut settings = Settings {
            java_home_path: "  D:\\ProgramData\\java\\openjdk-21 \n".to_string(),
            maven_executable_path: "\tD:\\tools\\apache-maven-3.9.9\\bin\\mvn.cmd ".to_string(),
            maven_java_home_path: "   ".to_string(),
            maven_settings_path: " D:\\m2\\settings.xml ".to_string(),
            maven_local_repository_path: "\t \n".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.java_home_path, "D:\\ProgramData\\java\\openjdk-21");
        assert_eq!(
            settings.maven_executable_path,
            "D:\\tools\\apache-maven-3.9.9\\bin\\mvn.cmd"
        );
        // 只有空白的值 = 没设（否则探测层会拿一个空路径去查文件系统）。
        assert_eq!(settings.maven_java_home_path, "");
        assert_eq!(settings.maven_settings_path, "D:\\m2\\settings.xml");
        assert_eq!(settings.maven_local_repository_path, "");
        // 路径里**内部**的空格不能被动（`C:\Program Files\...` 是合法安装目录）。
        let mut settings = Settings {
            java_home_path: "C:\\Program Files\\Java\\jdk-21".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.java_home_path, "C:\\Program Files\\Java\\jdk-21");
    }

    /// `uiFontSize` 的钳制 + 步长（照 `ui-font-size.ts:10-19` 逐条）。
    #[test]
    fn ui_font_size_is_snapped_and_clamped() {
        assert_eq!(normalize_ui_font_size(13.0), 13.0);
        assert_eq!(normalize_ui_font_size(13.24), 13.0);
        assert_eq!(normalize_ui_font_size(13.26), 13.5);
        assert_eq!(normalize_ui_font_size(0.0), UI_FONT_SIZE_MIN);
        assert_eq!(normalize_ui_font_size(999.0), UI_FONT_SIZE_MAX);
        assert_eq!(normalize_ui_font_size(-5.0), UI_FONT_SIZE_MIN);
        assert_eq!(normalize_ui_font_size(f64::NAN), UI_FONT_SIZE_DEFAULT);
        assert_eq!(normalize_ui_font_size(f64::INFINITY), UI_FONT_SIZE_DEFAULT);
        // 两位定点：0.5 步长的值不出现二进制尾差。
        assert_eq!(normalize_ui_font_size(16.5), 16.5);
    }

    /// `13` 必须正好落在 gpui 的 rem 基准 16px 上（否则默认外观会整体缩放一次）。
    #[test]
    fn default_ui_font_size_maps_to_the_theme_rem_base() {
        assert_eq!(theme_font_size_for(13.0), 16.0);
        // 19.5 = 1.5 × 13 → rem 基准 24px。
        assert_eq!(theme_font_size_for(19.5), 24.0);
        // 越界值先被规范化夹到 24，再换算（照 `ui-font-size.ts:30-33` 的
        // `normalizeUiFontSize(value) / UI_FONT_SIZE_DEFAULT`）。
        assert!((theme_font_size_for(99.0) - 16.0 * 24.0 / 13.0).abs() < 1e-4);
        assert!((theme_font_size_for(0.0) - 16.0 * 10.0 / 13.0).abs() < 1e-4);
    }

    /// 编辑器字号：四舍五入到整数档 + 夹在 10..=22（照控件属性，不跟 `uiFontSize` 的 0.5 步长）。
    #[test]
    fn editor_font_size_is_rounded_and_clamped() {
        assert_eq!(normalize_editor_font_size(14.0), 14.0);
        assert_eq!(normalize_editor_font_size(14.4), 14.0);
        assert_eq!(normalize_editor_font_size(14.6), 15.0);
        assert_eq!(normalize_editor_font_size(10.0), EDITOR_FONT_SIZE_MIN);
        assert_eq!(normalize_editor_font_size(22.0), EDITOR_FONT_SIZE_MAX);
        assert_eq!(normalize_editor_font_size(0.0), EDITOR_FONT_SIZE_MIN);
        assert_eq!(normalize_editor_font_size(999.0), EDITOR_FONT_SIZE_MAX);
        assert_eq!(normalize_editor_font_size(f64::NAN), EDITOR_FONT_SIZE_DEFAULT);
    }

    /// 制表符宽度与终端 shell 都是白名单：非法值回落默认，合法值原样留下。
    #[test]
    fn tab_size_and_shell_id_are_whitelisted() {
        for value in TAB_SIZES {
            let mut settings = Settings {
                tab_size: value,
                ..Settings::default()
            };
            settings.normalize();
            assert_eq!(settings.tab_size, value);
        }
        let mut settings = Settings {
            tab_size: 3,
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.tab_size, TAB_SIZE_DEFAULT);

        for id in TERMINAL_SHELL_IDS {
            let mut settings = Settings {
                terminal_default_shell_id: id.to_string(),
                ..Settings::default()
            };
            settings.normalize();
            assert_eq!(settings.terminal_default_shell_id, id);
        }
        let mut settings = Settings {
            terminal_default_shell_id: "fish".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.terminal_default_shell_id, SHELL_SYSTEM_DEFAULT);
    }

    /// 语言白名单：非法值回落默认。
    #[test]
    fn display_language_is_whitelisted() {
        let mut settings = Settings {
            display_language: "fr-FR".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.display_language, DEFAULT_DISPLAY_LANGUAGE);

        let mut settings = Settings {
            display_language: "en-US".to_string(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.display_language, "en-US");
        // Windows 的 `en-US` → gpui 的 `en`（yml 的 locale 名）。
        assert_eq!(settings.gpui_locale(), "en");
    }

    /// 空串主题回落默认；索引为空时不做判断；索引就绪后把**老值（显示名）归一成 id**。
    #[test]
    fn theme_ids_fall_back_to_defaults_and_accept_legacy_names() {
        let mut settings = Settings {
            theme: "  ".to_string(),
            auto_theme_light: String::new(),
            auto_theme_dark: String::new(),
            ..Settings::default()
        };
        settings.normalize();
        assert_eq!(settings.theme, DEFAULT_THEME);
        assert_eq!(settings.auto_theme_light, DEFAULT_AUTO_THEME_LIGHT);
        assert_eq!(settings.auto_theme_dark, DEFAULT_AUTO_THEME_DARK);

        let mut settings = Settings {
            theme: "No Such Theme".to_string(),
            ..Settings::default()
        };
        // 索引为空时不做判断（主题目录还没播种完，不能误判）。
        settings.normalize_with_themes(&ThemeIndex::default());
        assert_eq!(settings.theme, "No Such Theme");
        settings.normalize_with_themes(&index_of(&["lithe-dark"]));
        assert_eq!(settings.theme, DEFAULT_THEME);
    }

    /// 跟随系统时按系统外观在两支里选；不跟随时用显式主题。
    #[test]
    fn effective_theme_respects_sync_flag() {
        let mut settings = Settings {
            theme: "lithe-light".to_string(),
            auto_theme_light: "lithe-light".to_string(),
            auto_theme_dark: "lithe-dark".to_string(),
            sync_system_theme: false,
            ..Settings::default()
        };
        assert_eq!(settings.effective_theme(true), "lithe-light");
        assert_eq!(settings.effective_theme(false), "lithe-light");

        settings.sync_system_theme = true;
        assert_eq!(settings.effective_theme(true), "lithe-dark");
        assert_eq!(settings.effective_theme(false), "lithe-light");
    }

    /// 一份最小的主题文件文本（一条主题）。
    fn document(id: Option<&str>, name: &str, mode: &str) -> String {
        let id = match id {
            Some(id) => format!(r#""id": "{id}","#),
            None => String::new(),
        };
        format!(r#"{{"name":"test","themes":[{{{id}"name":"{name}","mode":"{mode}"}}]}}"#)
    }

    /// 用一批主题**显示名**造索引（id 由 slugify 兜底）——测试里最常用的简写。
    fn index_of(names: &[&str]) -> ThemeIndex {
        ThemeIndex::from_documents(
            names
                .iter()
                .map(|name| document(None, name, "dark"))
                .collect::<Vec<_>>(),
        )
    }

    /// 显式 `id` 优先于名字派生；没有 id 时按名字 slugify（用户手写的主题文件走这条）。
    #[test]
    fn theme_index_prefers_the_explicit_id_and_falls_back_to_the_name() {
        let index = ThemeIndex::from_documents(vec![
            document(Some("lithe-dark"), "Lithe Dark", "dark"),
            document(None, "VS Code Light+", "light"),
        ]);

        assert_eq!(index.entries().len(), 2);
        assert_eq!(index.id_for_name("Lithe Dark"), Some("lithe-dark"));
        assert_eq!(index.name_for_id("lithe-dark"), Some("Lithe Dark"));
        assert_eq!(index.id_for_name("VS Code Light+"), Some("vs-code-light"));
        assert!(index.entries()[0].dark);
        assert!(!index.entries()[1].dark, "light 主题不是深色");
        assert_eq!(index.ids(), vec!["lithe-dark", "vs-code-light"]);
    }

    /// **老设置文件的兼容路径**：显示名与 id 都能归一到同一个 id。
    ///
    /// 升级前文件里存的是 `"Lithe Dark"`（显示名），升级后存的是 `"lithe-dark"`；
    /// 两条都必须解析成同一个值，否则老用户会看到"主题丢失、回落默认"。
    #[test]
    fn canonicalize_accepts_both_ids_and_display_names() {
        let index = ThemeIndex::from_documents(vec![
            document(Some("lithe-dark"), "Lithe Dark", "dark"),
            document(Some("lithe-light"), "Lithe Light", "light"),
            document(Some("darcula"), "Darcula", "dark"),
        ]);

        // 新值：id。
        assert_eq!(
            index.canonicalize("lithe-dark").as_deref(),
            Some("lithe-dark")
        );
        // 老值：显示名（大小写与前后空白都容忍）。
        assert_eq!(
            index.canonicalize("  Lithe Dark  ").as_deref(),
            Some("lithe-dark")
        );
        assert_eq!(
            index.canonicalize("lithe LIGHT").as_deref(),
            Some("lithe-light")
        );
        assert_eq!(index.canonicalize("DARCULA").as_deref(), Some("darcula"));
        // 显示名写错大小写也要认（id 与显示名都做一次不区分大小写的兜底）。
        assert_eq!(
            index.canonicalize("lItHe LiGhT").as_deref(),
            Some("lithe-light")
        );
        // 不存在的主题（含空串）→ None，由调用方决定回落。
        assert_eq!(index.canonicalize("No Such Theme"), None);
        assert_eq!(index.canonicalize("   "), None);
    }

    /// 归一：老文件里的显示名在拿到索引后变成 id；不存在的回落默认。
    #[test]
    fn normalize_with_themes_rewrites_display_names_to_ids() {
        let index = ThemeIndex::from_documents(vec![
            document(Some("lithe-dark"), "Lithe Dark", "dark"),
            document(Some("lithe-light"), "Lithe Light", "light"),
            document(Some("gruvbox-dark"), "Gruvbox Dark", "dark"),
        ]);

        let mut settings = Settings {
            // 三份都是老值：显示名。
            theme: "Gruvbox Dark".to_string(),
            auto_theme_light: "Lithe Light".to_string(),
            auto_theme_dark: "Lithe Dark".to_string(),
            ..Settings::default()
        };
        settings.normalize_with_themes(&index);
        assert_eq!(settings.theme, "gruvbox-dark");
        assert_eq!(settings.auto_theme_light, "lithe-light");
        assert_eq!(settings.auto_theme_dark, "lithe-dark");

        // 一份已卸载的主题 → 回落默认（那条主题确实不在索引里）。
        let mut settings = Settings {
            theme: "Uninstalled Theme".to_string(),
            ..Settings::default()
        };
        settings.normalize_with_themes(&index);
        assert_eq!(settings.theme, DEFAULT_THEME);
    }

    /// 重名/重 id 的条目只保留先出现的那个，结果只依赖输入顺序。
    #[test]
    fn theme_index_keeps_the_first_of_each_id_and_name() {
        let index = ThemeIndex::from_documents(vec![
            document(Some("dup"), "First", "dark"),
            document(Some("dup"), "Second", "dark"),
            document(Some("other"), "First", "light"),
            document(None, "   ", "light"),
        ]);
        assert_eq!(index.entries().len(), 1);
        assert_eq!(index.ids(), vec!["dup"]);
        assert_eq!(index.name_for_id("dup"), Some("First"));
    }

    /// slugify 的边界：非字母数字折叠成一个 `-`、不留首尾、空串兜底。
    #[test]
    fn slugify_folds_punctuation_and_never_returns_empty() {
        assert_eq!(slugify_theme_id("Lithe Dark"), "lithe-dark");
        assert_eq!(slugify_theme_id("VS Code Light+"), "vs-code-light");
        assert_eq!(slugify_theme_id("  One   Dark  "), "one-dark");
        assert_eq!(slugify_theme_id("gruvbox-dark"), "gruvbox-dark");
        // 非 ASCII 字母数字保留（中文主题名不该全部塌成同一个 id）。
        assert_eq!(slugify_theme_id("深色主题"), "深色主题");
        assert_eq!(slugify_theme_id("+++"), "theme");
        assert_eq!(slugify_theme_id(""), "theme");
    }
}
