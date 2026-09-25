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
//! | `theme` / `autoTheme*` / `syncSystemTheme` | 立即：`Theme::change` + `apply_config` + `refresh_windows` | [`crate::theme::apply_theme_by_name`] |
//! | `uiFontSize` | 立即：写 `Theme.font_size`（rem 基准），`Root::render` 每帧用它调 `window.set_rem_size` | `gpui-component-0.6.6/src/root.rs:582` |
//! | `fontSize`（编辑器字号） | 立即：写 `Theme.mono_font_size`，编辑器正文当帧就变 | `gpui-component-0.6.6/src/input/editor.rs:137-143` |
//! | `tabSize` | 立即，但**不由本 store 应用**：外壳订阅本实体后转发给 `EditorPane::set_tab_size` | `gpui-base-0.6.6/src/input/editor/indent.rs:504` |
//! | `terminalDefaultShellId` | 只影响**新建**的终端会话：外壳订阅后转发给 `TerminalPane::set_default_shell` | `macos-settings-panels.tsx:379-393` 的原文「用于新的终端会话。」 |
//! | `confirmBeforeDiscard` | 立即：外壳订阅后转发给 `ChangesView::set_confirm_before_discard`，丢弃路径据此决定要不要先弹确认框 | `tabs/git-settings.tsx:93-106`（真源默认 `true`，`default-settings.ts:194`） |
//! | `showStatusBar` | 立即：`ShellWorkspace::render` 条件渲染 + 本 Entity 的 `notify` | `gpui/crates/workbench/src/workspace.rs` |
//! | `displayLanguage` | **重启后**（gpui 侧 `set_locale` 只在启动早期调一次） | `gpui/crates/app/src/main.rs` |
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

use crate::persistence::{self, DebounceState, Loaded};
use crate::schema::{Settings, theme_font_size_for};
use crate::theme;

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
    /// 没有即时副作用（语言要重启、状态栏/缩进/终端 shell 由订阅方自己应用）。
    None,
}

/// 设置状态。
pub struct SettingsStore {
    /// 当前设置（内存真源；落盘只是它的投影）。
    settings: Settings,
    /// 设置文件路径；`None` = 推导不出（例如 Windows 上 `APPDATA` 缺失），只影响落盘。
    path: Option<PathBuf>,
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
    } = loaded;

    // ⚠️ **这里不做"主题名必须在注册表里"的规范化**：主题目录是异步加载的
    // （`ThemeRegistry::watch_dir` 内部 `cx.spawn`），这一刻注册表里只有内置的
    // `Default Light` / `Default Dark`，而 `gpui/themes/` 里的 `Lithe Dark` 还没进来。
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
        settings,
        path,
        theme_override,
        applied_theme,
        debounce: DebounceState::default(),
        appearance_subscription: None,
        theme_subscription: None,
    });

    cx.set_global(SettingsHandle(store.clone()));

    // 主题还没装载，先把 UI 字号落到 rem 基准上：即使主题文件全坏、一个主题都载不进来，
    // 字号设置也照样生效。编辑器字号同理（写的是主题的等宽字号，与主题名无关）。
    store.update(cx, |store, cx| store.apply_font_size(cx));
    store.update(cx, |store, cx| store.apply_editor_font_size(cx));

    // rem 基准不变量：`ThemeRegistry` 的全局观察者（`theme/registry.rs:46-71`）在主题目录
    // 重新加载时会调一次 `Theme::change`，而 `apply_config` 会把主题文件里的 `font.size`
    // （`gpui/themes/lithe-dark.json:9` = 16.0）写回 `Theme.font_size`，把 uiFontSize 的缩放抹掉。
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

impl SettingsStore {
    /// 当前设置（只读）。
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// 设置文件路径（诊断/界面提示用）。
    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }

    /// 当前应该生效的主题名（界面用它作为下拉的当前值）。
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
    pub fn set_theme(&mut self, name: SharedString, cx: &mut Context<Self>) {
        self.theme_override = None;
        let mut next = self.settings.clone();
        let is_dark = theme::theme_is_dark(cx, &name);
        if next.sync_system_theme {
            match is_dark {
                Some(true) => next.auto_theme_dark = name.to_string(),
                Some(false) => next.auto_theme_light = name.to_string(),
                // 主题不在注册表里：能确定的只有"用户挑了它"，写进 `theme` 更不容易丢。
                None => next.theme = name.to_string(),
            }
        } else {
            next.theme = name.to_string();
        }
        self.commit(cx, next, Effects::Theme);
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
    pub fn set_theme_explicit(&mut self, name: SharedString, cx: &mut Context<Self>) {
        self.theme_override = None;
        let mut next = self.settings.clone();
        next.sync_system_theme = false;
        next.theme = name.to_string();
        self.commit(cx, next, Effects::Theme);
    }

    /// 改「外观模式」。立即生效 + 防抖落盘。
    ///
    /// 照 Windows：选浅/深色会**同时**把 `syncSystemTheme` 关掉并写 `theme`
    /// （`macos-settings-panels.tsx:158-166`）；选「跟随系统」只打开同步位，
    /// 由 [`Self::apply_theme`] 按当前系统外观解析。
    pub fn set_appearance_mode(&mut self, mode: AppearanceMode, cx: &mut Context<Self>) {
        self.theme_override = None;
        let mut next = self.settings.clone();
        match mode {
            AppearanceMode::System => next.sync_system_theme = true,
            AppearanceMode::Light => {
                next.sync_system_theme = false;
                next.theme = next.auto_theme_light.clone();
            }
            AppearanceMode::Dark => {
                next.sync_system_theme = false;
                next.theme = next.auto_theme_dark.clone();
            }
        }
        self.commit(cx, next, Effects::Theme);
    }

    /// 改「界面字体大小」。立即生效（同时被 rem 基准不变量兜底）+ 防抖落盘。
    pub fn set_ui_font_size(&mut self, value: f64, cx: &mut Context<Self>) {
        let mut next = self.settings.clone();
        next.ui_font_size = value;
        self.commit(cx, next, Effects::FontSize);
    }

    /// 改「显示状态栏」。立即生效（订阅方重绘）+ 防抖落盘。
    pub fn set_show_status_bar(&mut self, value: bool, cx: &mut Context<Self>) {
        let mut next = self.settings.clone();
        next.show_status_bar = value;
        self.commit(cx, next, Effects::None);
    }

    /// 改「编辑器字号」。立即生效（写主题的 `mono_font_size`，编辑器正文当帧就变）+ 防抖落盘。
    pub fn set_editor_font_size(&mut self, value: f64, cx: &mut Context<Self>) {
        let mut next = self.settings.clone();
        next.font_size = value;
        self.commit(cx, next, Effects::EditorFontSize);
    }

    /// 改「制表符宽度」。**本 store 不自己应用**（缩进只对编辑器状态有意义），
    /// 由外壳订阅本实体后转发给 `EditorPane::set_tab_size`；这里只落状态 + 防抖落盘 + `notify`。
    pub fn set_tab_size(&mut self, value: u32, cx: &mut Context<Self>) {
        let mut next = self.settings.clone();
        next.tab_size = value;
        self.commit(cx, next, Effects::None);
    }

    /// 改「终端默认 Shell」。同样由外壳订阅后转发给 `TerminalPane`（只影响新建会话）。
    pub fn set_terminal_default_shell_id(&mut self, id: String, cx: &mut Context<Self>) {
        let mut next = self.settings.clone();
        next.terminal_default_shell_id = id;
        self.commit(cx, next, Effects::None);
    }

    /// 改「丢弃前确认」（阶段 15，「Git」页）。同样由外壳订阅后转发给
    /// `ChangesView::set_confirm_before_discard`（只影响下一次丢弃）。
    pub fn set_confirm_before_discard(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let mut next = self.settings.clone();
        next.confirm_before_discard = enabled;
        self.commit(cx, next, Effects::None);
    }

    /// 改「项目 · JDK 与 Maven」页的 JDK 覆盖值（空串 = 用自动检测到的那个）。
    ///
    /// **本 store 不自己应用**：这个值是"这台机器上用什么 JDK"的声明，页面上的「生效值」
    /// 那一行由设置对话框自己按草稿 + 探测结论重算（`dialog.rs` 的 `project_page`），
    /// 落盘走防抖。运行配置那一侧将来接上时，走的是"外壳订阅本实体后转发"这条既有路子。
    pub fn set_java_home_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.settings.clone();
        next.java_home_path = path;
        self.commit(cx, next, Effects::None);
    }

    /// 改 Maven 主目录 / 可执行文件的覆盖值。见 [`Self::set_java_home_path`]。
    pub fn set_maven_executable_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.settings.clone();
        next.maven_executable_path = path;
        self.commit(cx, next, Effects::None);
    }

    /// 改 Maven 使用的 JDK 覆盖值。见 [`Self::set_java_home_path`]。
    pub fn set_maven_java_home_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.settings.clone();
        next.maven_java_home_path = path;
        self.commit(cx, next, Effects::None);
    }

    /// 改「显示语言」。返回是否真的变了。**立即落盘**（不等 300ms 防抖）：调用方紧接着就会
    /// 重启应用（[`crate::restart::restart_application`]），新进程必须马上读到新语言。
    ///
    /// 为什么语言不做运行中热切：gpui 侧 `set_locale` 只在启动早期调一次，而界面里有构造期就
    /// `tr()` 过的文案（活动栏项、状态栏文案），热切只会"一半变、一半不变"。生效路径是重启。
    pub fn set_display_language(&mut self, tag: String, cx: &mut Context<Self>) -> bool {
        let mut next = self.settings.clone();
        next.display_language = tag;
        let changed = self.commit(cx, next, Effects::None);
        if changed {
            self.debounce.mark_flushed();
            self.write();
        }
        changed
    }

    /// 所有改动入口的唯一落点：规范化 → **值没变就直接返回**（不应用、不落盘）→ 应用副作用 →
    /// 通知 + 防抖写。
    ///
    /// 去重不是优化而是语义：Windows 也是"只有真正变化的键才 `store.set`"
    /// （`lib/settings-persistence.ts:34` 的 `if (!isEqual(currentValue, nextValue))`）。
    /// 它同时挡住一类噪音：数字输入框在**创建**时会先发一次值不变的 `InputEvent::Change`，
    /// 不去重就会出现"只是打开设置对话框，什么都没改，却写了一次盘"。
    fn commit(&mut self, cx: &mut Context<Self>, mut next: Settings, effects: Effects) -> bool {
        next.normalize();
        if next == self.settings {
            return false;
        }
        self.settings = next;
        match effects {
            Effects::Theme => self.apply_theme(cx),
            Effects::FontSize => self.apply_font_size(cx),
            Effects::EditorFontSize => self.apply_editor_font_size(cx),
            Effects::None => {}
        }
        self.after_change(cx);
        true
    }

    /// 恢复默认设置：**立即落盘**（真源 `stores/settings.store.ts:88-97`）。
    pub fn restore_defaults(&mut self, cx: &mut Context<Self>) {
        self.settings = Settings::default();
        self.settings.normalize_with_themes(&theme::theme_names(cx));
        self.theme_override = None;
        self.apply_theme(cx);
        self.apply_font_size(cx);
        self.debounce.mark_flushed();
        self.write();
        println!("S1_SETTINGS reset_to_defaults");
        cx.notify();
    }

    /// 把待写的改动立刻落盘（关闭对话框时调用，避免最后一次改动落在防抖窗口里）。
    pub fn flush_pending(&mut self) {
        if !self.debounce.has_pending() {
            return;
        }
        self.debounce.mark_flushed();
        self.write();
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
        let mut name = self.theme_override.clone().unwrap_or_else(|| {
            SharedString::from(
                self.settings
                    .effective_theme(theme::system_is_dark(appearance))
                    .to_string(),
            )
        });

        // 「主题名必须在注册表里」的规范化落在**这里**而不是加载设置时：只有到这一步
        // （主题目录装载回调、或用户改设置）注册表才是就绪的。做法是"应用默认主题 + 留一条诊断"，
        // **不写回设置文件**：主题文件可能在热重载后重新出现，写回就把用户的选择抹掉了。
        let known = theme::theme_names(cx);
        if !known.is_empty() && !known.iter().any(|theme_name| theme_name == name.as_ref()) {
            eprintln!(
                "S1_SETTINGS theme_missing name={name} fallback={}",
                crate::schema::DEFAULT_THEME
            );
            name = SharedString::from(crate::schema::DEFAULT_THEME);
        }

        self.applied_theme = name.clone();
        // 查不到名字时 `apply_theme_by_name` 自己会打 `S1_THEME missing`，不做静默回落：
        // 主题文件被删掉这种事必须留下证据。
        theme::apply_theme_by_name(cx, &name);
        self.apply_font_size(cx);
        // ⚠️ 顺序不能反：`apply_config` 会把主题文件里的字体档写回主题 token，
        // 所以两个"用户设置的字号"必须在它之后各补一次（rem 基准 + 等宽字号）。
        self.apply_editor_font_size(cx);
    }

    fn apply_font_size(&self, cx: &mut Context<Self>) {
        theme::apply_theme_font_size(cx, self.settings.ui_font_size);
    }

    fn apply_editor_font_size(&self, cx: &mut Context<Self>) {
        theme::apply_editor_font_size(cx, self.settings.font_size);
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
        match persistence::save(&path, &self.settings) {
            Ok(bytes) => println!(
                "S1_SETTINGS saved path={} bytes={bytes}",
                path.display()
            ),
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
        let (settings, diagnostics) =
            persistence::load_from_str(r#"{"displayLanguage": "en-US", "uiFontSize": 15.0}"#);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(settings.display_language, "en-US");
        assert_eq!(settings.gpui_locale(), "en");
        assert_eq!(settings.ui_font_size, 15.0);
        // 缺的键回落默认，不是整份丢弃。
        assert_eq!(settings.display_language.is_empty(), false);
        let (only_theme, _) = persistence::load_from_str(r#"{"theme": "Lithe Light"}"#);
        assert_eq!(only_theme.display_language, DEFAULT_DISPLAY_LANGUAGE);
    }
}
