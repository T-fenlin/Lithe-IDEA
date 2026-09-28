//! 设置的状态所有者：一个 gpui `Entity`（外加一个 `Global` 句柄），负责
//! **改设置 → 立刻生效 → 300ms 防抖落盘** 这条闭环。
//!
//! ## 为什么状态放在 `Entity` 而不是普通 `Global`
//!
//! 别的区域（`workbench` 的状态栏要读 `showStatusBar`）需要"设置变了就重绘"。
//! `Entity` 的 `notify()` 是现成的广播机制：`ShellWorkspace` 只要 `cx.observe(&store, ..)`
//! 就能自动跟随，不需要再造一套观察者。句柄本身放在 [`Global`] 里，任何地方用
//! [`store`] 一行就能拿到（`Global` 只是"哪里能找到这个 Entity"，不是状态本身）。
//!
//! ## 三类副作用（逐项对应 `07-settings-ui.md` §7.3）
//!
//! | 设置 | 生效方式 | 依据 |
//! | --- | --- | --- |
//! | `theme` / `autoTheme*` / `syncSystemTheme` | 立即：`Theme::change` + `apply_config` + `refresh_windows` | [`crate::theme::apply_theme_by_id`] |
//! | `uiFontSize` | 立即：写 `Theme.font_size`（rem 基准），`Root::render` 每帧用它调 `window.set_rem_size` | `gpui-component-0.6.6/src/root.rs:582` |
//! | `fontSize`（编辑器字号） | 立即：写 `Theme.mono_font_size`，编辑器正文当帧就变 | `gpui-component-0.6.6/src/input/editor.rs:137-143` |
//! | `tabSize` | 立即，但**不由本 store 应用**：外壳订阅本实体后转发给 `EditorPane::set_tab_size` | `gpui-base-0.6.6/src/input/editor/indent.rs:504` |
//! | `terminalDefaultShellId` | 只影响**新建**的终端会话：外壳订阅后转发给 `TerminalPane::set_default_shell` | `macos-settings-panels.tsx:379-393` 的原文「用于新的终端会话。」 |
//! | `confirmBeforeDiscard` | 立即：外壳订阅后转发给 `ChangesView::set_confirm_before_discard`，丢弃路径据此决定要不要先弹确认框 | `tabs/git-settings.tsx:93-106`（真源默认 `true`，`default-settings.ts:194`） |
//! | `showStatusBar` | 立即：`ShellWorkspace::render` 条件渲染 + 本 Entity 的 `notify` | `lithe-db-gpui/crates/workbench/src/workspace.rs` |
//! | `displayLanguage` | **重启后**（gpui 侧 `set_locale` 只在启动早期调一次） | `lithe-db-gpui/crates/app/src/main.rs` |
//!
//! ## 落盘时机
//!
//! 普通改动走 300ms 防抖（真源 `lib/settings-persistence.ts:96-112`）；「恢复默认设置」走立即写
//! （真源 `stores/settings.store.ts:88-97` 的 `resetToDefaults` 也是立即 `await`）；
//! 关闭对话框时补一次 `flush_pending`，避免最后一次改动落在防抖窗口里还没写就退出。

use std::path::PathBuf;

use gpui_kit::{
    App, AppContext as _, Context, Entity, Global, SharedString, Subscription, Window,
    WindowAppearance, px,
};
use serde_json::Value;

use lithe_db_gpui_shared::workspace_config::WorkspaceConfigPaths;

use crate::persistence::{self, DebounceState, Loaded};
use crate::schema::{Settings, theme_font_size_for};
use crate::theme;
use crate::workspace::{self, AppearanceKey, AppearanceOverlay, AppearanceSource, AppearanceValue};

/// 「外观模式」下拉的三个取值。真源 `macos-settings-panels.tsx:87-91,158-171`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceMode {
    /// 跟随系统（`syncSystemTheme = true`）。
    System,
    /// 显式浅色（`syncSystemTheme = false` + 浅色主题）。
    Light,
    /// 显式深色（`syncSystemTheme = false` + 深色主题）。
    Dark,
}

impl AppearanceMode {
    /// 从设置里推出当前该显示哪一个。
    ///
    /// 逐字照 Windows：`syncSystemTheme ? "system" : theme.includes("light") ? "light" : "dark"`
    /// （`macos-settings-panels.tsx:87-91`）—— 它看的是主题名里的 `light` 字样，
    /// 因为 Windows 侧的主题 id 形如 `lithe-light`；gpui 侧的主题名是 `Lithe Light`，
    /// 同样含 `light`（不区分大小写），所以这条判断可以直接沿用。
    pub fn from_settings(settings: &Settings) -> Self {
        if settings.sync_system_theme {
            return Self::System;
        }
        if settings.theme.to_lowercase().contains("light") {
            Self::Light
        } else {
            Self::Dark
        }
    }
}

/// 一次改动要顺带做的副作用（[`SettingsStore::commit`] 用）。
#[derive(Debug, Clone, Copy)]
enum Effects {
    /// 主题变了：重新解析并应用主题 + 字号。
    Theme,
    /// 只有 UI 字号变了：只更新 rem 基准。
    FontSize,
    /// 只有编辑器字号变了：只更新主题的等宽字号（编辑器正文）。
    EditorFontSize,
    /// 只有字体族变了：只更新主题的两个字体族 token（界面 / 等宽）。
    FontFamily,
    /// 没有即时副作用（语言要重启、状态栏/缩进/终端 shell 与终端字号由订阅方自己应用）。
    None,
}

/// 设置状态。
pub struct SettingsStore {
    /// **生效值**：全局层 + 工作区两层外观覆盖合并之后的结果（`workspace::resolve_effective`）。
    /// 界面、主题应用、字号转发读的都是它。
    settings: Settings,
    /// **全局层**（设置文件里那一份，19 个键的真源）。落盘写的是它，
    /// 非外观键（语言 / 缩进 / 终端 shell / 工具链…）的唯一来源也是它。
    ///
    /// ⚠️ 这两个字段必须分开：只留 `settings`（生效值）会让"改一个字幕大小"把工作区覆盖的
    /// 值一起写进全局文件 —— 那是静默的数据污染。
    global: Settings,
    /// 工作区根；`None` = 没有打开工作区，只有全局层。
    workspace_root: Option<PathBuf>,
    /// `.lithe/settings.json` 的外观覆盖（共享层：团队 / 本项目的设置）。
    workspace_shared: AppearanceOverlay,
    /// 共享层上一次落盘的原始文档（写回时保留未知键）。
    workspace_shared_previous: Option<Value>,
    /// 共享层声明了更高的文档版本：**不覆盖**这个文件。
    workspace_shared_read_only: bool,
    /// `.lithe/settings.local.json` 的外观覆盖（本机层：你的个人覆盖）。
    workspace_local: AppearanceOverlay,
    /// 本机层上一次落盘的原始文档（写回时保留未知键）。
    workspace_local_previous: Option<Value>,
    /// 本机层声明了更高的文档版本：**不覆盖**这个文件。
    workspace_local_read_only: bool,
    /// `.lithe/settings.json` 是否已被 Git 跟踪（决定界面叫它"团队设置"还是"本项目的设置"）。
    /// `None` = 还没问到（按"尚未提交"显示，见 `workspace::source_for`）。
    workspace_shared_tracked: Option<bool>,
    /// 设置文件路径；`None` = 推导不出（例如 Windows 上 `APPDATA` 缺失），只影响落盘。
    path: Option<PathBuf>,
    /// 设置文件里上一次的原始文档。写回时用它保留未知键（见
    /// [`crate::persistence::merge_document`]），所以每次写成功都要把它换成新文档。
    previous: Option<Value>,
    /// 文件声明的版本高于本程序支持：**只读**，任何改动都不落盘（但内存里照样生效）。
    read_only: bool,
    /// `--theme` 的显式覆盖（**验证/诊断用**，不写进设置文件）。用户一改主题就作废。
    theme_override: Option<SharedString>,
    /// 最近一次解析出来的、应该生效的主题名（`watch_dir` 热重载后按它复原，界面也显示它）。
    applied_theme: SharedString,
    /// 300ms 防抖状态机。
    debounce: DebounceState,
    /// 系统外观监听（跟随系统时才真的做动作）。
    appearance_subscription: Option<Subscription>,
    /// rem 基准不变量（见 [`SettingsStore::enforce_rem_base`] 的说明）。
    theme_subscription: Option<Subscription>,
    /// 进程退出前的补写钩子（见 [`SettingsStore::install_quit_flush`]）。
    quit_subscription: Option<Subscription>,
}

/// `Global` 只承载"哪里能找到那个 Entity"。
struct SettingsHandle(Entity<SettingsStore>);

impl Global for SettingsHandle {}

/// 取设置状态。**必须在 [`init_store`] 之后调用**。
pub fn store(cx: &App) -> Entity<SettingsStore> {
    cx.global::<SettingsHandle>().0.clone()
}

/// 取设置状态；还没初始化时返回 `None`（主题目录的热重载回调可能比 `init` 早/晚，见
/// [`crate::theme::watch_lithe_themes`]）。
pub fn try_store(cx: &App) -> Option<Entity<SettingsStore>> {
    cx.try_global::<SettingsHandle>().map(|handle| handle.0.clone())
}

/// 启动参数。
pub struct Init {
    /// [`persistence::load`] 的结果（已含"文件不存在 → 全默认值"的语义）。
    pub loaded: Loaded,
    /// `--theme <名>`：显式覆盖，验证/诊断用；不写回设置文件。
    pub theme_override: Option<SharedString>,
}

/// 建立设置状态：规范化 → 登记 `Global` → 应用字号 → 装 rem 基准不变量。
///
/// ⚠️ **这里不应用主题**：主题目录是异步加载的（`ThemeRegistry::watch_dir` 里
/// `cx.spawn`，`theme/registry.rs:104-115`），启动这一刻注册表里还没有 `Lithe Dark`。
/// 应用主题由 [`crate::theme::watch_lithe_themes`] 的回调统一做，这样启动时
/// `S1_THEME applied=` 只会出现一次，也不会出现误导性的 `S1_THEME missing`。
pub fn init_store(cx: &mut App, init: Init) -> Entity<SettingsStore> {
    let Init {
        loaded,
        theme_override,
    } = init;
    let Loaded {
        mut settings,
        path,
        file_existed,
        diagnostics,
        previous,
        read_only,
    } = loaded;

    // ⚠️ **这里不做"主题名必须在注册表里"的规范化**：主题目录是异步加载的
    // （`ThemeRegistry::watch_dir` 内部 `cx.spawn`），这一刻注册表里只有内置的
    // `Default Light` / `Default Dark`，而 `lithe-db-gpui/themes/` 里的 `Lithe Dark` 还没进来。
    // 在这个时点校验会把设置文件里的 `Lithe Light` 误判成"不存在"并换成默认主题（实测踩过一次）。
    // 那条校验放在 [`SettingsStore::apply_theme_for`]：主题装载回调里注册表已经就绪。
    settings.normalize();

    report_load(&settings, path.as_deref(), file_existed, &diagnostics);

    let applied_theme = theme_override.clone().unwrap_or_else(|| {
        SharedString::from(
            settings
                .effective_theme(theme::system_is_dark(cx.window_appearance()))
                .to_string(),
        )
    });

    let store = cx.new(|_| SettingsStore {
        // 打开工作区之前，生效值就是全局值。
        settings: settings.clone(),
        global: settings,
        workspace_root: None,
        workspace_shared: AppearanceOverlay::default(),
        workspace_shared_previous: None,
        workspace_shared_read_only: false,
        workspace_local: AppearanceOverlay::default(),
        workspace_local_previous: None,
        workspace_local_read_only: false,
        workspace_shared_tracked: None,
        path,
        previous,
        read_only,
        theme_override,
        applied_theme,
        debounce: DebounceState::default(),
        appearance_subscription: None,
        theme_subscription: None,
        quit_subscription: None,
    });

    cx.set_global(SettingsHandle(store.clone()));

    // 外部改动监听（手改文件不必重启）与退出前补写。两件事都在这里装，调用方不需要记得：
    // 少装监听 = 手改文件不生效，少装退出补写 = 退出前 300ms 内的改动丢掉。
    store.update(cx, |store, cx| store.install_watcher(cx));
    store.update(cx, |store, cx| store.install_quit_flush(cx));

    // 主题还没装载，先把 UI 字号落到 rem 基准上：即使主题文件全坏、一个主题都载不进来，
    // 字号设置也照样生效。编辑器字号同理（写的是主题的等宽字号，与主题名无关）。
    store.update(cx, |store, cx| store.apply_font_size(cx));
    store.update(cx, |store, cx| store.apply_editor_font_size(cx));

    // rem 基准不变量：`ThemeRegistry` 的全局观察者（`theme/registry.rs:46-71`）在主题目录
    // 重新加载时会调一次 `Theme::change`，而 `apply_config` 会把主题文件里的 `font.size`
    // （`lithe-db-gpui/themes/lithe-dark.json:9` = 16.0）写回 `Theme.font_size`，把 uiFontSize 的缩放抹掉。
    // 这里把它纠回来：只在真的不一致时才写，所以最多再触发一轮观察者就收敛。
    store.update(cx, |store, cx| {
        store.theme_subscription = Some(cx.observe_global::<gpui_kit::component::Theme>(
            |store, cx| store.enforce_rem_base(cx),
        ));
    });

    store
}

/// 打印读取诊断（`S1_SETTINGS ...`，与 `main.rs` 的 `S1_THEME` 同一风格，可 grep）。
fn report_load(
    settings: &Settings,
    path: Option<&std::path::Path>,
    file_existed: bool,
    diagnostics: &[String],
) {
    let path_text = path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(none)".to_string());
    if file_existed {
        println!(
            "S1_SETTINGS loaded path={path_text} theme={} locale={} ui_font_size={} show_status_bar={} sync_system_theme={}",
            settings.theme,
            settings.display_language,
            settings.ui_font_size,
            settings.show_status_bar,
            settings.sync_system_theme
        );
    } else {
        // 首次启动（文件不存在）与"文件读不了"都落在这里，用 reason 区分。
        let reason = diagnostics
            .first()
            .cloned()
            .unwrap_or_else(|| "not_found".to_string());
        println!("S1_SETTINGS defaults path={path_text} reason={reason}");
    }
    for diagnostic in diagnostics {
        eprintln!("S1_SETTINGS {diagnostic}");
    }
}

/// 工作区覆盖的两层（写回与日志都要按层区分）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceLayer {
    /// `.lithe/settings.json`（共享层）。
    Shared,
    /// `.lithe/settings.local.json`（本机层）。
    Local,
}

impl WorkspaceLayer {
    /// 日志里的层名。
    fn as_str(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Local => "local",
        }
    }
}

/// 日志/界面用的来源短名（`global` / `team` / `project` / `local`）。
fn source_tag(source: AppearanceSource) -> &'static str {
    match source {
        AppearanceSource::Global => "global",
        AppearanceSource::ProjectShared { tracked: true } => "team",
        AppearanceSource::ProjectShared { tracked: false } => "project",
        AppearanceSource::ProjectLocal => "local",
    }
}

/// 一个外观键变化之后要应用的副作用（与用户改设置走同一条路）。
fn effects_for(key: AppearanceKey) -> Effects {
    match key {
        AppearanceKey::Theme
        | AppearanceKey::SyncSystemTheme
        | AppearanceKey::AutoThemeLight
        | AppearanceKey::AutoThemeDark => Effects::Theme,
        AppearanceKey::UiFontSize => Effects::FontSize,
        AppearanceKey::EditorFontSize => Effects::EditorFontSize,
        AppearanceKey::FontFamily | AppearanceKey::MonoFontFamily => Effects::FontFamily,
        AppearanceKey::TerminalFontSize => Effects::None,
    }
}

/// 一个覆盖层里设了的键（顺序 = [`AppearanceKey::ALL`]）。
fn overlay_keys(overlay: &AppearanceOverlay) -> Vec<AppearanceKey> {
    AppearanceKey::ALL
        .iter()
        .copied()
        .filter(|key| overlay.has(*key))
        .collect()
}

/// 一个键在给定设置里的值的文本形式（只用于诊断行）。
fn value_text(settings: &Settings, key: AppearanceKey) -> String {
    match AppearanceValue::from_settings(settings, key) {
        AppearanceValue::Text(text) => text,
        AppearanceValue::Number(number) => number.to_string(),
        AppearanceValue::Flag(flag) => flag.to_string(),
    }
}

impl SettingsStore {
    /// **生效值**（全局层 + 工作区外观覆盖）。界面与各消费方读的都是它。
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// 全局层的值（不含工作区外观覆盖）。只有需要区分"这一层"的地方才读它
    /// （例如诊断、以及测试里对照"改回全局之后到底回落到了哪个值"）。
    pub fn global_settings(&self) -> &Settings {
        &self.global
    }

    /// 当前打开的工作区根（没有打开工作区时是 `None`）。
    pub fn workspace_root(&self) -> Option<&std::path::Path> {
        self.workspace_root.as_deref()
    }

    /// 一个外观键这一刻的来源（三态 + 跟随全局）。
    ///
    /// "跟随系统"时主题由 `autoTheme*` 决定，所以调用方要么用
    /// [`Self::effective_theme_source`]，要么自己传 [`AppearanceKey`]。
    pub fn appearance_source(&self, key: AppearanceKey) -> AppearanceSource {
        workspace::source_for(
            key,
            &self.workspace_shared,
            &self.workspace_local,
            self.workspace_shared_tracked,
        )
    }

    /// 当前**决定生效主题**的那个键的来源（设置页「配色主题」那一行用它）。
    pub fn effective_theme_source(&self, system_is_dark: bool) -> AppearanceSource {
        let key = workspace::effective_theme_key(&self.settings, system_is_dark);
        self.appearance_source(key)
    }

    /// 「外观模式」（跟随系统 / 浅色 / 深色）这一刻的来源。
    ///
    /// 跟随系统时模式由 `syncSystemTheme` 决定；不跟随时模式由生效主题属于浅色还是深色决定，
    /// 所以取"决定生效主题的那个键"的来源。这样"工作区设了跟随系统"与"工作区设了主题"
    /// 两种情况都会如实显示出来。
    pub fn appearance_mode_source(&self, system_is_dark: bool) -> AppearanceSource {
        if self.settings.sync_system_theme {
            let key = AppearanceKey::SyncSystemTheme;
            if self.appearance_source(key).is_workspace_override() {
                return self.appearance_source(key);
            }
        }
        self.effective_theme_source(system_is_dark)
    }

    /// 逐键来源（诊断用；顺序 = [`AppearanceKey::ALL`]）。
    pub fn appearance_sources(&self) -> Vec<workspace::AppearanceKeySource> {
        workspace::appearance_sources(
            &self.workspace_shared,
            &self.workspace_local,
            self.workspace_shared_tracked,
        )
    }

    /// 设置文件路径（诊断/界面提示用）。
    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }

    /// 当前应该生效的主题 **id**（界面用它作为下拉的当前值，设置文件里存的也是它）。
    pub fn applied_theme(&self) -> SharedString {
        self.applied_theme.clone()
    }

    /// 外观模式下拉的当前值。
    pub fn appearance_mode(&self) -> AppearanceMode {
        AppearanceMode::from_settings(&self.settings)
    }

    // ---------------------------------------------------------------- 改动入口

    /// 改「配色主题」。立即生效 + 防抖落盘。
    ///
    /// 与 Windows 一致：**跟随系统时改的是"首选深/浅主题"**，不是 `theme`
    /// （`macos-settings-panels.tsx:115-123`）。
    ///
    /// 参数接受 **id 或显示名**（[`theme::canonical_theme_id`] 归一）：设置界面给的是 id，
    /// 菜单栏给的是显示名，两条路都通。
    ///
    /// 写哪一层由 [`Self::commit_appearance`] 决定（有工作区 → 本机层）。
    pub fn set_theme(&mut self, value: SharedString, cx: &mut Context<Self>) {
        self.theme_override = None;
        let id = theme::canonical_theme_id(cx, &value);
        let is_dark = theme::theme_is_dark(cx, &id);
        // 跟随系统时改的是"首选深/浅主题"，不是 `theme` —— 判据在**生效值**上，
        // 因为工作区层也可能把 `syncSystemTheme` 打开（那时用户的改动要落到 autoTheme* 上，
        // 否则他改了主题却看不出任何变化）。
        let key = if self.settings.sync_system_theme {
            match is_dark {
                Some(true) => AppearanceKey::AutoThemeDark,
                Some(false) => AppearanceKey::AutoThemeLight,
                // 主题不在注册表里：能确定的只有"用户挑了它"，写进 `theme` 更不容易丢。
                None => AppearanceKey::Theme,
            }
        } else {
            AppearanceKey::Theme
        };
        self.commit_appearance(
            cx,
            &[(key, AppearanceValue::Text(id.to_string()))],
            Effects::Theme,
        );
    }

    /// 显式选一个配色主题：**同时关掉「跟随系统」**，立即生效 + 防抖落盘。
    ///
    /// 与 [`Self::set_theme`] 的差别只有一条：这里把 `syncSystemTheme` 一起关掉。
    /// 为什么需要它：真机命令面板切主题走的是两种写法 —— 设置面板那种是"跟随系统时改的是
    /// 首选深/浅主题"（`macos-settings-panels.tsx:115-123`），而命令面板**切主题**是
    /// 先 `updateSetting("syncSystemTheme", false)` 再写 `theme`
    /// （`features/command-palette/components/command-palette.tsx:107-116`）——
    /// 用户从命令面板点名要一个主题，意思就是"别跟着系统了"。
    /// 这里把真源那两步合成一次 `commit`，避免中间态落一次盘。
    ///
    /// 参数同样接受 **id 或显示名**：菜单栏的主题子菜单列的是显示名。
    pub fn set_theme_explicit(&mut self, value: SharedString, cx: &mut Context<Self>) {
        self.theme_override = None;
        let id = theme::canonical_theme_id(cx, &value);
        // 两个键**一起写**（真源写两次是两步，本侧合成一次，避免中间态落一次盘。
        // 与 `remember_project_open_destination` 同一条口径）。
        self.commit_appearance(
            cx,
            &[
                (
                    AppearanceKey::SyncSystemTheme,
                    AppearanceValue::Flag(false),
                ),
                (AppearanceKey::Theme, AppearanceValue::Text(id.to_string())),
            ],
            Effects::Theme,
        );
    }

    /// 改「外观模式」。立即生效 + 防抖落盘。
    ///
    /// 照 Windows：选浅/深色会**同时**把 `syncSystemTheme` 关掉并写 `theme`
    /// （`macos-settings-panels.tsx:158-166`）；选「跟随系统」只打开同步位，
    /// 由 [`Self::apply_theme`] 按当前系统外观解析。
    pub fn set_appearance_mode(&mut self, mode: AppearanceMode, cx: &mut Context<Self>) {
        self.theme_override = None;
        let mut values = vec![(
            AppearanceKey::SyncSystemTheme,
            AppearanceValue::Flag(mode == AppearanceMode::System),
        )];
        match mode {
            AppearanceMode::System => {}
            // 浅/深色模式要写 `theme`：真源写的也是 `settings.theme`（`macos-settings-panels.tsx:158-166`）。
            AppearanceMode::Light => values.push((
                AppearanceKey::Theme,
                AppearanceValue::Text(self.settings.auto_theme_light.clone()),
            )),
            AppearanceMode::Dark => values.push((
                AppearanceKey::Theme,
                AppearanceValue::Text(self.settings.auto_theme_dark.clone()),
            )),
        }
        self.commit_appearance(cx, &values, Effects::Theme);
    }

    /// 改「界面字体大小」。立即生效（同时被 rem 基准不变量兜底）+ 防抖落盘。
    pub fn set_ui_font_size(&mut self, value: f64, cx: &mut Context<Self>) {
        self.commit_appearance(
            cx,
            &[(AppearanceKey::UiFontSize, AppearanceValue::Number(value))],
            Effects::FontSize,
        );
    }

    /// 改「显示状态栏」。立即生效（订阅方重绘）+ 防抖落盘。
    ///
    /// **只有全局层**：状态栏不是外观覆盖范围内的键（见 `workspace.rs` 的键白名单）。
    pub fn set_show_status_bar(&mut self, value: bool, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.show_status_bar = value;
        self.commit_global(cx, next, Effects::None);
    }

    /// 改「编辑器字号」。立即生效（写主题的 `mono_font_size`，编辑器正文当帧就变）+ 防抖落盘。
    pub fn set_editor_font_size(&mut self, value: f64, cx: &mut Context<Self>) {
        self.commit_appearance(
            cx,
            &[(
                AppearanceKey::EditorFontSize,
                AppearanceValue::Number(value),
            )],
            Effects::EditorFontSize,
        );
    }

    /// 改「界面字体族」。立即生效（写主题的 `font_family`）+ 防抖落盘。
    ///
    /// 空串 = 不覆盖（保留主题文件里的值）。**不可用的字族不会被写进主题**（见
    /// [`crate::theme::apply_font_families`]：GPUI 在字族缺失时会 panic），
    /// 但用户填的值照旧落盘 —— 可能只是这台机器上还没装那个字体。
    /// 工作区层里的字族值走**同一条**校验：生效值最终都要经 `apply_font_families` 才进主题。
    pub fn set_font_family(&mut self, family: String, cx: &mut Context<Self>) {
        self.commit_appearance(
            cx,
            &[(
                AppearanceKey::FontFamily,
                AppearanceValue::Text(family),
            )],
            Effects::FontFamily,
        );
    }

    /// 改「代码字体族」（编辑器与终端正文）。见 [`Self::set_font_family`]。
    pub fn set_mono_font_family(&mut self, family: String, cx: &mut Context<Self>) {
        self.commit_appearance(
            cx,
            &[(
                AppearanceKey::MonoFontFamily,
                AppearanceValue::Text(family),
            )],
            Effects::FontFamily,
        );
    }

    /// 改「终端字号」。**本 store 不自己应用**：终端正文是 `lithe-db-gpui-terminal` 自己的视图，
    /// 由外壳订阅本实体后转发给 `TerminalPane::set_font_size`（与 `tabSize` 同一条路子）。
    /// `0` = 不覆盖，终端用它自己的默认档。
    pub fn set_terminal_font_size(&mut self, value: f64, cx: &mut Context<Self>) {
        self.commit_appearance(
            cx,
            &[(
                AppearanceKey::TerminalFontSize,
                AppearanceValue::Number(value),
            )],
            Effects::None,
        );
    }

    /// 改「制表符宽度」。**本 store 不自己应用**（缩进只对编辑器状态有意义），
    /// 由外壳订阅本实体后转发给 `EditorPane::set_tab_size`；这里只落状态 + 防抖落盘 + `notify`。
    pub fn set_tab_size(&mut self, value: u32, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.tab_size = value;
        self.commit_global(cx, next, Effects::None);
    }

    /// 改「终端默认 Shell」。同样由外壳订阅后转发给 `TerminalPane`（只影响新建会话）。
    pub fn set_terminal_default_shell_id(&mut self, id: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.terminal_default_shell_id = id;
        self.commit_global(cx, next, Effects::None);
    }

    /// 改「丢弃前确认」（阶段 15，「Git」页）。同样由外壳订阅后转发给
    /// `ChangesView::set_confirm_before_discard`（只影响下一次丢弃）。
    pub fn set_confirm_before_discard(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.confirm_before_discard = enabled;
        self.commit_global(cx, next, Effects::None);
    }

    /// 改「项目 · JDK 与 Maven」页的 JDK 覆盖值（空串 = 用自动检测到的那个）。
    ///
    /// **本 store 不自己应用**：这个值是"这台机器上用什么 JDK"的声明，页面上的「生效值」
    /// 那一行由设置对话框自己按草稿 + 探测结论重算（`dialog.rs` 的 `project_page`），
    /// 落盘走防抖。运行配置那一侧将来接上时，走的是"外壳订阅本实体后转发"这条既有路子。
    pub fn set_java_home_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.java_home_path = path;
        self.commit_global(cx, next, Effects::None);
    }

    /// 改 Maven 主目录 / 可执行文件的覆盖值。见 [`Self::set_java_home_path`]。
    pub fn set_maven_executable_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.maven_executable_path = path;
        self.commit_global(cx, next, Effects::None);
    }

    /// 改 Maven 使用的 JDK 覆盖值。见 [`Self::set_java_home_path`]。
    pub fn set_maven_java_home_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.maven_java_home_path = path;
        self.commit_global(cx, next, Effects::None);
    }

    /// 改 Maven 用户 `settings.xml` 的覆盖值。见 [`Self::set_java_home_path`]。
    ///
    /// 这一个键**有真消费方**：外壳把它登记进 `lithe-db-gpui-java` 的覆盖槽，JDT LS 启动时随
    /// `mavenContext.settingsPath` 交给 Core（判据见 `schema.rs` 字段文档），所以它和
    /// `javaHomePath` 一样是"下一次语言服务启动时生效"，不是当帧生效。
    pub fn set_maven_settings_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.maven_settings_path = path;
        self.commit_global(cx, next, Effects::None);
    }

    /// 改 Maven 本地仓库的覆盖值。见 [`Self::set_java_home_path`]（今天没有消费方）。
    pub fn set_maven_local_repository_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.maven_local_repository_path = path;
        self.commit_global(cx, next, Effects::None);
    }

    /// 改「自动补全」（阶段 18，「LSP」页）。**本 store 不自己应用**：补全菜单的触发判据
    /// 在 `lithe-db-gpui-editor` 的 provider 上，由外壳订阅本实体后转发给
    /// `EditorPane::set_auto_completion`（与 `tabSize` 同一条路子，依赖方向：
    /// `workbench` → `editor`）。
    pub fn set_auto_completion(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.auto_completion = enabled;
        self.commit_global(cx, next, Effects::None);
    }

    /// 记住「这次项目是怎么打开的」（真源 `executeProjectOpenDecision`，
    /// `windows/tauri/src/features/file-system/controllers/project-open-destination.ts:112-131`）。
    ///
    /// ⚠️ **调用时机是"打开成功之后"**，不是点了按钮就写：真源那里是
    /// `const opened = await open(decision.destination); if (!opened) return false;`
    /// 之后才写这两个键（`:117-128`）。调用方是 `workbench` 的换项目链路。
    ///
    /// 两个键**一起写**（真源写两次 `updateSetting`，本侧合成一次 `commit`）：
    /// 分开写会在中间态落一次盘（`askWhereToOpenProjects = false` 而
    /// `openFoldersInNewWindow` 还是旧值），下一次启动就会按半套偏好直接定目的地。
    ///
    /// `Effects::None`：这两个键没有即时副作用 —— 它们只被"下一次打开项目"的决策读到
    /// （不像主题/字号那样要当帧生效）。
    pub fn remember_project_open_destination(
        &mut self,
        open_in_new_window: bool,
        cx: &mut Context<Self>,
    ) {
        let mut next = self.global.clone();
        next.open_folders_in_new_window = open_in_new_window;
        next.ask_where_to_open_projects = false;
        self.commit_global(cx, next, Effects::None);
    }

    /// 改「显示语言」。返回是否真的变了。**立即落盘**（不等 300ms 防抖）：调用方紧接着就会
    /// 重启应用（[`crate::restart::restart_application`]），新进程必须马上读到新语言。
    ///
    /// 为什么语言不做运行中热切：gpui 侧 `set_locale` 只在启动早期调一次，而界面里有构造期就
    /// `tr()` 过的文案（活动栏项、状态栏文案），热切只会"一半变、一半不变"。生效路径是重启。
    pub fn set_display_language(&mut self, tag: String, cx: &mut Context<Self>) -> bool {
        let mut next = self.global.clone();
        next.display_language = tag;
        let changed = self.commit_global(cx, next, Effects::None);
        if changed {
            self.debounce.mark_flushed();
            self.write();
        }
        changed
    }

    /// 改动**全局层**的唯一落点：规范化 → **值没变就直接返回**（不应用、不落盘）→ 应用副作用 →
    /// 通知 + 防抖写。
    ///
    /// 去重不是优化而是语义：Windows 也是"只有真正变化的键才 `store.set`"
    /// （`lib/settings-persistence.ts:34` 的 `if (!isEqual(currentValue, nextValue))`）。
    /// 它同时挡住一类噪音：数字输入框在**创建**时会先发一次值不变的 `InputEvent::Change`，
    /// 不去重就会出现"只是打开设置对话框，什么都没改，却写了一次盘"。
    fn commit_global(&mut self, cx: &mut Context<Self>, mut next: Settings, effects: Effects) -> bool {
        next.normalize();
        if next == self.global {
            return false;
        }
        self.global = next;
        // 全局层变了要重算生效值（工作区覆盖可能正盖在上面）。
        self.recompute_effective(cx, effects);
        self.after_change(cx);
        true
    }

    /// 改动**外观键**的唯一落点：按"有没有工作区"分流到正确的那一层。
    ///
    /// | 情况 | 写哪里 | 为什么 |
    /// | --- | --- | --- |
    /// | 有工作区 | `.lithe/settings.local.json`（本机层） | 与工具链五值同一条口径（`project.rs` 的"有工作区写项目本机层"）：当前项目的外观改动属于"这个人 + 这个项目"。写在个人层而不是共享层，因为**用户自己的改动不该悄悄进团队文件**；共享层要由"共享此项目的配置"那个显式动作写（那一批还没落地） |
    /// | 没有工作区 | 全局 `settings.json` | 与改动之前的行为完全一致 |
    ///
    /// ⚠️ 无论写哪一层，**生效值都当场重算**（[`Self::recompute_effective`]），
    /// 所以"改了没反应"不可能发生：本机层优先级最高，用户改的值一定赢。
    fn commit_appearance(
        &mut self,
        cx: &mut Context<Self>,
        values: &[(AppearanceKey, AppearanceValue)],
        effects: Effects,
    ) -> bool {
        if self.workspace_root.is_none() {
            // 没有工作区：外观改动照旧写全局设置文件。借用覆盖层的那一个 set，避免在这里
            // 再写一遍"键 → 字段"的映射（第二份映射迟早与 schema 漂移）。
            let mut patch = AppearanceOverlay::default();
            for (key, value) in values {
                patch.set(*key, value.clone());
            }
            let mut next = self.global.clone();
            patch.apply_to(&mut next);
            return self.commit_global(cx, next, effects);
        }

        let before = self.workspace_local.clone();
        for (key, value) in values {
            self.workspace_local.set(*key, value.clone());
        }
        if self.workspace_local == before {
            return false;
        }

        // 归一（钳制范围沿用 `Settings::normalize` 那一份实现）之后再落盘：
        // 否则会出现"界面上是 24、文件里是 99"。
        let effective = workspace::resolve_effective(
            &self.global,
            &self.workspace_shared,
            &self.workspace_local,
        );
        self.workspace_local.clamp_with(&effective);
        self.settings = effective;
        self.write_workspace_layer(WorkspaceLayer::Local);

        match effects {
            Effects::Theme => self.apply_theme(cx),
            Effects::FontSize => self.apply_font_size(cx),
            Effects::EditorFontSize => self.apply_editor_font_size(cx),
            Effects::FontFamily => self.apply_font_families(cx),
            Effects::None => {}
        }
        cx.notify();
        true
    }

    /// 重算生效值（全局层 + 两层工作区覆盖），再按需应用副作用。
    ///
    /// **优先级只有一份实现**（[`workspace::resolve_effective`]）：任何"全局层 / 工作区层
    /// 变了"的路径都必须走这里，不要各自手写合并。
    fn recompute_effective(&mut self, cx: &mut Context<Self>, effects: Effects) {
        self.settings = workspace::resolve_effective(
            &self.global,
            &self.workspace_shared,
            &self.workspace_local,
        );
        match effects {
            Effects::Theme => self.apply_theme(cx),
            Effects::FontSize => self.apply_font_size(cx),
            Effects::EditorFontSize => self.apply_editor_font_size(cx),
            Effects::FontFamily => self.apply_font_families(cx),
            Effects::None => {}
        }
        cx.notify();
    }

    /// 恢复默认设置：**立即落盘**（真源 `stores/settings.store.ts:88-97`）。
    ///
    /// ⚠️ 恢复的是**全局层**。工作区两层的外观覆盖**不会**被它删掉 —— 那是团队/项目的文件，
    /// 一个叫"恢复默认设置"的按钮去改别人的提交文件是错的。被覆盖的键照旧显示工作区值，
    /// 设置页那一行的来源标注会如实说明原因；这里再留一行诊断，让"按了没反应"可排查。
    pub fn restore_defaults(&mut self, cx: &mut Context<Self>) {
        self.global = Settings::default();
        // 默认值本身已经是 id，这里再归一一次是为了兜住"默认主题那份文件不在索引里"的极端情况
        // （索引来自主题文件，见 `theme::theme_index`）。
        self.global.normalize_with_themes(&theme::theme_index());
        self.theme_override = None;
        let overridden: Vec<AppearanceKey> = self
            .appearance_sources()
            .into_iter()
            .filter(|entry| entry.source.is_workspace_override())
            .map(|entry| entry.key)
            .collect();
        self.settings = workspace::resolve_effective(
            &self.global,
            &self.workspace_shared,
            &self.workspace_local,
        );
        self.apply_theme(cx);
        self.apply_font_size(cx);
        self.debounce.mark_flushed();
        self.write();
        println!(
            "S1_SETTINGS reset_to_defaults workspace_overridden={}",
            workspace::describe_keys(&overridden)
        );
        cx.notify();
    }

    /// 把待写的改动立刻落盘（关闭对话框、进程退出时调用，避免最后一次改动落在防抖窗口里）。
    pub fn flush_pending(&mut self) {
        if !self.debounce.has_pending() {
            return;
        }
        self.debounce.mark_flushed();
        self.write();
    }

    // ------------------------------------------------------- 工作区外观覆盖层

    /// 登记当前工作区根并读入它的两层外观覆盖（打开 / 切换项目时调）。
    ///
    /// - 这是"外部改动至少要在打开 / 切换项目时重新读"这条要求的落点；设置文件本身的
    ///   watcher（[`crate::watch`]）监听的是**全局**设置文件，工作区文件不共享它。
    /// - 读两份很小的 JSON，**不碰 Core、不起子进程**（同 `dialog.rs` 构造期读工具链本机层
    ///   那条口径）。传 `None` = 没有工作区：两层清空，生效值就是全局值。
    /// - 失败不 panic、也不让打开项目失败：坏文件只留诊断（那一层当"没设"）。
    pub fn set_workspace_root(&mut self, root: Option<PathBuf>, cx: &mut Context<Self>) {
        self.workspace_root = root;
        self.workspace_shared = AppearanceOverlay::default();
        self.workspace_shared_previous = None;
        self.workspace_shared_read_only = false;
        // 换根之后"上一个项目的共享文件有没有被跟踪"不再成立，重新等一次 Core 的答复。
        self.workspace_shared_tracked = None;
        self.workspace_local = AppearanceOverlay::default();
        self.workspace_local_previous = None;
        self.workspace_local_read_only = false;

        let Some(root) = self.workspace_root.clone() else {
            println!("S1_SETTINGS workspace_appearance root=(none) overridden=");
            self.recompute_effective(cx, Effects::Theme);
            return;
        };

        let paths = WorkspaceConfigPaths::new(root.clone());
        let shared = workspace::load_overlay(&paths.settings());
        let local = workspace::load_overlay(&paths.settings_local());
        for diagnostic in shared.diagnostics.iter().chain(local.diagnostics.iter()) {
            eprintln!("S1_SETTINGS workspace_appearance {diagnostic}");
        }

        self.workspace_shared = shared.overlay;
        self.workspace_shared_previous = shared.previous;
        self.workspace_shared_read_only = shared.read_only;
        self.workspace_local = local.overlay;
        self.workspace_local_previous = local.previous;
        self.workspace_local_read_only = local.read_only;

        self.recompute_effective(cx, Effects::Theme);

        // 一行启动证据：生效值到底来自哪一层。它同时是"工作区覆盖真的被读进来了"的判据。
        let overridden: Vec<AppearanceKey> = self
            .appearance_sources()
            .into_iter()
            .filter(|entry| entry.source.is_workspace_override())
            .map(|entry| entry.key)
            .collect();
        println!(
            "S1_SETTINGS workspace_appearance root={} shared_file={} local_file={} shared_read_only={} local_read_only={} overridden={}",
            root.display(),
            shared.file_existed,
            local.file_existed,
            shared.read_only,
            local.read_only,
            workspace::describe_keys(&overridden)
        );
        self.report_appearance_sources();
    }

    /// 登记 `.lithe/settings.json` 是否已被 Git 跟踪。
    ///
    /// 只有这一件事需要问 Git（判据见 [`AppearanceSource::ProjectShared`]），所以由外壳在
    /// 打开项目的后台任务里查一次再回填，本 crate 不认识 Core。`None` = 还没问到。
    pub fn set_workspace_settings_tracked(&mut self, tracked: Option<bool>, cx: &mut Context<Self>) {
        if self.workspace_shared_tracked == tracked {
            return;
        }
        self.workspace_shared_tracked = tracked;
        println!(
            "S1_SETTINGS workspace_settings_tracked tracked={}",
            tracked
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        );
        // 来源标注跟着这次答复变（团队设置 ↔ 本项目的设置），所以再打一遍证据行：
        // 设置页那一行也会在同一帧之后重画（`notify`）。
        self.report_appearance_sources();
        cx.notify();
    }

    /// 打出被工作区覆盖的每一个外观键的**来源 + 生效值**（诊断证据行）。
    fn report_appearance_sources(&self) {
        for entry in self.appearance_sources() {
            if entry.source.is_workspace_override() {
                println!(
                    "S1_SETTINGS appearance_source key={} source={} value={}",
                    entry.key.as_str(),
                    source_tag(entry.source),
                    value_text(&self.settings, entry.key)
                );
            }
        }
    }

    /// 「改回我的全局外观」：把这个外观键从**两层**工作区文件里删掉，于是生效值回落到全局值。
    ///
    /// 为什么是两层一起：工作区的"那一层"由共享层与本机层共同构成（本机层盖在共享层上）。
    /// 只清本机层的话，一个同时被两层覆盖的键会回落到**共享层的值**而不是用户的全局值 ——
    /// 按钮的名字（"改回我的全局外观"）承诺的是后者。清掉之后文件里**不留空值**：
    /// 写回时未设的键会从文档里删掉（见 `workspace::merged_document`）。
    pub fn revert_appearance_to_global(&mut self, key: AppearanceKey, cx: &mut Context<Self>) {
        self.revert_appearance_group_to_global(&[key], cx);
    }

    /// 一次把一组外观键改回全局。
    ///
    /// "主题"那一组（`theme` / `syncSystemTheme` / `autoThemeLight` / `autoThemeDark`）是
    /// **一件事**：只清其中一个，生效主题可能仍然被另一个工作区键决定，按钮就变成"按了没用"。
    /// 所以主题行一次清四个；其余键各自一组。
    pub fn revert_appearance_group_to_global(
        &mut self,
        keys: &[AppearanceKey],
        cx: &mut Context<Self>,
    ) {
        if self.workspace_root.is_none() {
            eprintln!(
                "S1_SETTINGS appearance_revert_skipped keys={} reason=no_workspace",
                workspace::describe_keys(keys)
            );
            return;
        }

        let mut cleared_shared = Vec::new();
        let mut cleared_local = Vec::new();
        for key in keys {
            if self.workspace_shared.has(*key) {
                self.workspace_shared.clear(*key);
                cleared_shared.push(*key);
            }
            if self.workspace_local.has(*key) {
                self.workspace_local.clear(*key);
                cleared_local.push(*key);
            }
        }
        if !cleared_shared.is_empty() {
            self.write_workspace_layer(WorkspaceLayer::Shared);
        }
        if !cleared_local.is_empty() {
            self.write_workspace_layer(WorkspaceLayer::Local);
        }

        // 一组里只要有一个键带即时副作用就按最强的那个重应用（主题那一组四键都归 Theme）。
        let effects = if keys.len() == 1 {
            effects_for(keys[0])
        } else {
            Effects::Theme
        };
        self.recompute_effective(cx, effects);

        // 证据行：`value=` 是**回落之后**的生效值，`global=` 是全局层里的值。
        // 两者相等即"真的回落到全局值"（而不是空值、也不是默认值）。
        let layers = match (cleared_shared.is_empty(), cleared_local.is_empty()) {
            (true, true) => "(none)".to_string(),
            (false, true) => "shared".to_string(),
            (true, false) => "local".to_string(),
            (false, false) => "shared+local".to_string(),
        };
        for key in keys {
            println!(
                "S1_SETTINGS appearance_reverted key={} layers={} value={} global={}",
                key.as_str(),
                layers,
                value_text(&self.settings, *key),
                value_text(&self.global, *key)
            );
        }
    }

    /// 把一层工作区覆盖原子写回磁盘，并刷新它的"上一次文档"快照（写回时保留未知键要用）。
    fn write_workspace_layer(&mut self, layer: WorkspaceLayer) {
        let Some(root) = self.workspace_root.clone() else {
            return;
        };
        let paths = WorkspaceConfigPaths::new(root);
        let (path, overlay, previous, read_only) = match layer {
            WorkspaceLayer::Shared => (
                paths.settings(),
                self.workspace_shared.clone(),
                self.workspace_shared_previous.clone(),
                self.workspace_shared_read_only,
            ),
            WorkspaceLayer::Local => (
                paths.settings_local(),
                self.workspace_local.clone(),
                self.workspace_local_previous.clone(),
                self.workspace_local_read_only,
            ),
        };

        // 文件声明的版本比本程序新：与全局设置文件同一条口径 —— 内存里照常生效，
        // 但**绝不覆盖**用户的文件。
        if read_only {
            eprintln!(
                "S1_SETTINGS workspace_save_skipped layer={} path={} reason=document_version_newer",
                layer.as_str(),
                path.display()
            );
            return;
        }

        match workspace::write_overlay(&path, previous.as_ref(), &overlay) {
            Ok(document) => {
                match layer {
                    WorkspaceLayer::Shared => self.workspace_shared_previous = Some(document),
                    WorkspaceLayer::Local => self.workspace_local_previous = Some(document),
                }
                println!(
                    "S1_SETTINGS workspace_saved layer={} path={} keys={}",
                    layer.as_str(),
                    path.display(),
                    workspace::describe_keys(&overlay_keys(&overlay))
                );
            }
            // 写失败不 panic：内存里的覆盖仍然生效（这一次会话内），下一次改动会再试。
            Err(error) => eprintln!(
                "S1_SETTINGS workspace_save_failed layer={} path={} error={error}",
                layer.as_str(),
                path.display()
            ),
        }
    }

    // ------------------------------------------------------- 外部改动 / 退出补写

    /// 装外部改动监听（手改设置文件不必重启）。没有文件路径时什么都不做。
    fn install_watcher(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.path.clone() else {
            eprintln!("S1_SETTINGS watch_skipped reason=no_path");
            return;
        };
        crate::watch::watch_settings_file(cx, path);
    }

    /// 装上"进程退出前补写一次"的钩子。
    ///
    /// 300ms 防抖的写入是一个 detached 任务：进程在窗口内退出时它会随实体一起消失，
    /// 那次改动就丢了。这里用 gpui 的 `on_app_quit` 在退出流程里补一次同步写。
    fn install_quit_flush(&mut self, cx: &mut Context<Self>) {
        if self.quit_subscription.is_some() {
            return;
        }
        self.quit_subscription = Some(cx.on_app_quit(|store, _cx| {
            let pending = store.debounce.has_pending();
            store.flush_pending();
            if pending {
                println!("S1_SETTINGS flush_on_quit pending=yes");
            }
            // 回调要求返回一个 future；补写本身是同步的，所以这里立即完成。
            async {}
        }));
    }

    /// 外部改动之后重新读文件：内容与内存一致时什么都不做（挡住"自己的写入触发自己"）。
    ///
    /// 详见 [`crate::watch`] 的模块文档（为什么以文件为准、文件被删为什么不动内存）。
    pub fn reload_from_disk(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.path.clone() else {
            return;
        };
        let loaded = persistence::load_from(Some(path.clone()));

        // 文件被删 / 暂时读不到：保留内存里的设置，不把用户打回默认值。
        if !loaded.file_existed {
            eprintln!(
                "S1_SETTINGS reload_skipped path={} reason=file_missing",
                path.display()
            );
            return;
        }

        let changed = loaded.settings != self.global;
        let read_only_changed = loaded.read_only != self.read_only;
        // 未知键的快照无论内容有没有变都要刷新：下一次写回要靠它保留这些键。
        self.previous = loaded.previous;
        if !changed && !read_only_changed {
            return;
        }

        self.global = loaded.settings;
        self.read_only = loaded.read_only;
        // 全局层变了要重算生效值：工作区覆盖可能正盖在上面（界面与消费方读的是生效值）。
        self.settings = workspace::resolve_effective(
            &self.global,
            &self.workspace_shared,
            &self.workspace_local,
        );
        // 内存已经等于文件内容，那次待写没有必要了（留着只会把同一份内容再写一遍）。
        self.debounce.mark_flushed();

        for diagnostic in &loaded.diagnostics {
            eprintln!("S1_SETTINGS reload_diagnostic {diagnostic}");
        }
        println!(
            "S1_SETTINGS reloaded path={} theme={} ui_font_size={} read_only={}",
            path.display(),
            self.settings.theme,
            self.settings.ui_font_size,
            self.read_only
        );

        // 即时副作用与用户改设置走同一条路：主题、rem 基准、编辑器字号。
        // 语言要重启才生效、终端 shell 只影响新会话，两者由订阅方按 `notify` 自行跟随。
        self.apply_theme(cx);
        self.apply_font_size(cx);
        self.apply_editor_font_size(cx);
        cx.notify();
    }

    // ---------------------------------------------------------------- 主题应用

    /// 按"当前应该生效"的主题重新应用（`watch_dir` 热重载回调走这里）。
    pub fn reapply_theme(&mut self, cx: &mut Context<Self>) {
        self.apply_theme(cx);
    }

    /// 注册系统外观监听。`syncSystemTheme` 为假时回调什么都不做。
    ///
    /// 需要 `&mut Window`（`gpui-pre-0.3.6/src/app/context.rs:457`），所以由
    /// `main.rs` 在窗口建好之后调一次；`Root::new` 之前不存在窗口外观可听。
    pub fn attach_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.appearance_subscription.is_some() {
            return;
        }
        self.appearance_subscription = Some(cx.observe_window_appearance(
            window,
            |store, window, cx| {
                if !store.settings.sync_system_theme {
                    return;
                }
                store.apply_theme_for(window.appearance(), cx);
            },
        ));
    }

    fn apply_theme(&mut self, cx: &mut Context<Self>) {
        let appearance = cx.window_appearance();
        self.apply_theme_for(appearance, cx);
    }

    fn apply_theme_for(&mut self, appearance: WindowAppearance, cx: &mut Context<Self>) {
        let raw = self.theme_override.clone().unwrap_or_else(|| {
            SharedString::from(
                self.settings
                    .effective_theme(theme::system_is_dark(appearance))
                    .to_string(),
            )
        });

        // 先归一成 id：`--theme` 可能给显示名，设置文件里也可能是升级前写的显示名
        // （`init_store` 不做主题校验，理由见那里的注释）。两条路都不该因为我们"拿到的是名字"
        // 而回落默认主题。
        let id = theme::canonical_theme_id(cx, &raw);

        // 「这个主题在注册表里吗」的判定落在**这里**而不是加载设置时：只有到这一步
        // （主题目录装载回调、或用户改设置）注册表才是就绪的。做法是"应用默认主题 + 留一条诊断"，
        // **不写回设置文件**：主题文件可能在热重载后重新出现，写回就把用户的选择抹掉了。
        let known = theme::theme_ids(cx);
        let mut applied = id.clone();
        if !known.is_empty() && !known.iter().any(|known_id| known_id == &id) {
            eprintln!(
                "S1_SETTINGS theme_missing id={id} fallback={}",
                crate::schema::DEFAULT_THEME
            );
            applied = crate::schema::DEFAULT_THEME.to_string();
        }

        self.applied_theme = SharedString::from(applied.clone());
        // 查不到时 `apply_theme_by_id` 自己会打 `S1_THEME missing`，不做静默回落：
        // 主题文件被删掉这种事必须留下证据。
        theme::apply_theme_by_id(cx, &applied);
        self.apply_font_size(cx);
        // ⚠️ 顺序不能反：`apply_config` 会把主题文件里的字体档写回主题 token，
        // 所以"用户设置的字体族与字号"必须在它之后各补一次
        // （rem 基准 + 等宽字号 + 两个字族）。
        self.apply_editor_font_size(cx);
        self.apply_font_families(cx);
    }

    fn apply_font_size(&self, cx: &mut Context<Self>) {
        theme::apply_theme_font_size(cx, self.settings.ui_font_size);
    }

    fn apply_editor_font_size(&self, cx: &mut Context<Self>) {
        theme::apply_editor_font_size(cx, self.settings.font_size);
    }

    fn apply_font_families(&self, cx: &mut Context<Self>) {
        theme::apply_font_families(
            cx,
            &self.settings.font_family,
            &self.settings.mono_font_family,
        );
    }

    /// rem 基准不变量（见 [`init_store`] 的说明）：主题的 `font_size` 必须等于 uiFontSize 的换算值。
    fn enforce_rem_base(&mut self, cx: &mut App) {
        let desired = px(theme_font_size_for(self.settings.ui_font_size));
        if gpui_kit::component::Theme::global(cx).font_size == desired {
            return;
        }
        gpui_kit::component::Theme::global_mut(cx).font_size = desired;
        gpui_kit::component::Theme::sync_base(cx);
        cx.refresh_windows();
    }

    // ---------------------------------------------------------------- 落盘

    fn after_change(&mut self, cx: &mut Context<Self>) {
        self.schedule_save(cx);
        cx.notify();
    }

    /// 排一次 300ms 防抖写。同一窗口里的多次改动会被 [`DebounceState`] 合并成一次写。
    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        if self.path.is_none() {
            return;
        }
        let armed = self.debounce.arm();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(persistence::SAVE_DEBOUNCE_MS))
                .await;
            // 实体可能已经销毁：`update` 返回 `Err` 时静默忽略，不 panic。
            let _ = this.update(cx, |store, _| store.flush_if_current(armed));
        })
        .detach();
    }

    /// 防抖唤醒：只有"最后一次改动"对应的那次唤醒才真的写。
    fn flush_if_current(&mut self, armed: u64) {
        if self.debounce.should_flush(armed) {
            self.write();
        }
    }

    fn write(&mut self) {
        let Some(path) = self.path.clone() else {
            return;
        };
        // 文件版本比本程序新：改动能生效（内存里已经改了），但**不落盘** ——
        // 覆盖它等于把用户在新版本里设置的东西连同我们不认识的键一起无声降级。
        if self.read_only {
            eprintln!(
                "S1_SETTINGS save_skipped path={} reason=document_version_newer",
                path.display()
            );
            return;
        }

        // 先合并出要写的文档：它同时是"带未知键的落盘内容"和"下一次写回时的 previous"，
        // 所以只合并一次，写成功后就地更新，避免两次写之间丢掉这一轮保留的键。
        //
        // ⚠️ 写的是**全局层**（`self.global`），不是生效值（`self.settings`）：
        // 把工作区覆盖的值写进全局文件会静默污染用户跨项目的默认外观。
        let document = match persistence::merge_document(self.previous.as_ref(), &self.global) {
            Ok(document) => document,
            Err(error) => {
                eprintln!(
                    "S1_SETTINGS save_failed path={} error={error}",
                    path.display()
                );
                return;
            }
        };

        match persistence::save_json(&path, &document) {
            Ok(bytes) => {
                self.previous = Some(document);
                println!("S1_SETTINGS saved path={} bytes={bytes}", path.display());
            }
            // 写失败不 panic：内存里的设置仍然是权威的，下一次改动会再试一次。
            Err(error) => eprintln!(
                "S1_SETTINGS save_failed path={} error={error}",
                path.display()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::DEFAULT_DISPLAY_LANGUAGE;

    /// 外观模式下拉的当前值判定（照 Windows 的 `theme.includes("light")` 口径）。
    #[test]
    fn appearance_mode_follows_windows_heuristic() {
        let mut settings = Settings::default();
        // 默认主题 `Lithe Dark` 不含 light → 深色。
        assert_eq!(
            AppearanceMode::from_settings(&settings),
            AppearanceMode::Dark
        );

        settings.theme = "Lithe Light".to_string();
        assert_eq!(
            AppearanceMode::from_settings(&settings),
            AppearanceMode::Light
        );

        settings.sync_system_theme = true;
        assert_eq!(
            AppearanceMode::from_settings(&settings),
            AppearanceMode::System
        );
    }

    /// `LITHE_GPUI_SETTINGS_FILE` 指向的文件被读进来之后，语言/字号必须逐字生效。
    #[test]
    fn loaded_file_drives_the_schema() {
        let parsed = persistence::parse(r#"{"displayLanguage": "en-US", "uiFontSize": 15.0}"#);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(parsed.settings.display_language, "en-US");
        assert_eq!(parsed.settings.gpui_locale(), "en");
        assert_eq!(parsed.settings.ui_font_size, 15.0);
        // 缺的键回落默认，不是整份丢弃。
        assert_eq!(parsed.settings.display_language.is_empty(), false);
        let only_theme = persistence::parse(r#"{"theme": "Lithe Light"}"#);
        assert_eq!(
            only_theme.settings.display_language,
            DEFAULT_DISPLAY_LANGUAGE
        );
    }
}
