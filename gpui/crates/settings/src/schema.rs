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
//! 用 `from_str`，而是走 [`crate::persistence::load_from_str`] 的逐键容错解析
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

/// 配色主题默认值：**gpui 侧的 `ThemeSet` 主题名**，不是 Windows 的 `themes[].id`。
///
/// Windows 的默认是 `lithe-dark`（`default-settings.ts:99`），而 gpui 的主题注册表按
/// `themes[].name` 索引（`gpui-component-0.6.6/src/theme/registry.rs:121,154`），
/// `gpui/themes/lithe-dark.json:7` 里这个名字是 `Lithe Dark`。两套标识的对应关系见
/// `gpui/research/windows/07-settings-ui.md` §5.4。本文件属于 gpui 侧新开的设置文件
/// （Windows 那份是 Tauri Store 写的另一份 `settings.json`），所以直接存 gpui 的名字。
pub const DEFAULT_THEME: &str = "Lithe Dark";
/// 「首选浅色主题」的默认值（gpui 名）。对应 Windows `autoThemeLight: "lithe-light"`
/// （`default-settings.ts:102`）。
pub const DEFAULT_AUTO_THEME_LIGHT: &str = "Lithe Light";
/// 「首选深色主题」的默认值（gpui 名）。对应 Windows `autoThemeDark: "lithe-dark"`
/// （`default-settings.ts:103`）。
pub const DEFAULT_AUTO_THEME_DARK: &str = "Lithe Dark";

/// 全部设置。
///
/// **依赖方向**：本类型不引用任何 GPUI 类型，`App` / `Window` 只在
/// [`crate::store`] 里出现；落盘格式与 UI 因此可以各自单测。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// 配色主题名（`ThemeSet` 的 `themes[].name`）。
    ///
    /// - Windows 键：`theme`，默认 `lithe-dark`（`default-settings.ts:99`）。
    /// - gpui 取值：`Lithe Dark`（见 [`DEFAULT_THEME`] 的说明）。
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
    }

    /// 再补一层"主题名必须在注册表里"的规范化（`known` 是 `ThemeRegistry::themes()` 的键）。
    ///
    /// 只把**不存在的名字**换成默认，不动存在的名字：用户可能装了自定义主题，
    /// 而那些主题在启动早期还没加载完。
    pub fn normalize_with_themes(&mut self, known: &[String]) {
        // 一个主题都没加载出来（注册表还没就绪 / 主题目录为空）时不做判断，
        // 否则会把用户选的主题误判成"不存在"。
        if known.is_empty() {
            return;
        }
        if !known.iter().any(|name| name == &self.theme) {
            self.theme = DEFAULT_THEME.to_string();
        }
        if !known.iter().any(|name| name == &self.auto_theme_light) {
            self.auto_theme_light = DEFAULT_AUTO_THEME_LIGHT.to_string();
        }
        if !known.iter().any(|name| name == &self.auto_theme_dark) {
            self.auto_theme_dark = DEFAULT_AUTO_THEME_DARK.to_string();
        }
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
        assert_eq!(settings.theme, "Lithe Dark");
        assert!(!settings.sync_system_theme);
        assert_eq!(settings.auto_theme_light, "Lithe Light");
        assert_eq!(settings.auto_theme_dark, "Lithe Dark");
        assert_eq!(settings.ui_font_size, 13.0);
        assert!(settings.show_status_bar);
        assert_eq!(settings.display_language, "zh-CN");
        assert_eq!(settings.font_size, 14.0);
        assert_eq!(settings.tab_size, 2);
        assert_eq!(settings.terminal_default_shell_id, "");
        assert!(settings.confirm_before_discard);
        // 「项目 · JDK 与 Maven」页的三个覆盖值默认都是"自动"（空串）。
        assert_eq!(settings.java_home_path, "");
        assert_eq!(settings.maven_executable_path, "");
        assert_eq!(settings.maven_java_home_path, "");
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
        ] {
            assert!(json.contains(key), "缺少键 {key}：{json}");
        }
    }

    /// 三个工具链覆盖值：粘贴路径带的首尾空白被去掉，空串仍是空串（= 自动）。
    #[test]
    fn toolchain_overrides_are_trimmed() {
        let mut settings = Settings {
            java_home_path: "  D:\\ProgramData\\java\\openjdk-21 \n".to_string(),
            maven_executable_path: "\tD:\\tools\\apache-maven-3.9.9\\bin\\mvn.cmd ".to_string(),
            maven_java_home_path: "   ".to_string(),
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

    /// 空串主题回落默认；不存在的主题名在拿到注册表后再回落。
    #[test]
    fn theme_names_fall_back_to_defaults() {
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
        // 注册表为空时不做判断（启动早期注册表还没加载完，不能误判）。
        settings.normalize_with_themes(&[]);
        assert_eq!(settings.theme, "No Such Theme");
        settings.normalize_with_themes(&["Lithe Dark".to_string()]);
        assert_eq!(settings.theme, DEFAULT_THEME);
    }

    /// 跟随系统时按系统外观在两支里选；不跟随时用显式主题。
    #[test]
    fn effective_theme_respects_sync_flag() {
        let mut settings = Settings {
            theme: "Lithe Light".to_string(),
            auto_theme_light: "Lithe Light".to_string(),
            auto_theme_dark: "Lithe Dark".to_string(),
            sync_system_theme: false,
            ..Settings::default()
        };
        assert_eq!(settings.effective_theme(true), "Lithe Light");
        assert_eq!(settings.effective_theme(false), "Lithe Light");

        settings.sync_system_theme = true;
        assert_eq!(settings.effective_theme(true), "Lithe Dark");
        assert_eq!(settings.effective_theme(false), "Lithe Light");
    }
}
