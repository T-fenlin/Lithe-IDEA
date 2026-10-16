use std::path::PathBuf;

use gpui::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Rems, Render,
    SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, px,
    rems,
};
use gpui::{PathPromptOptions, prelude::FluentBuilder as _};

use lithe_gpui_shared::commands::CommandId;
use lithe_gpui_shared::icons::{idea, idea_icon_image, idea_icon_svg_px};
use lithe_gpui_shared::workspace_config::{
    OverrideOrigin, OverrideOrigins, ToolchainPaths, load_local_toolchain, save_local_toolchain,
};
use lithe_gpui_shared::{tr, tr_args};
use lithe_ui::buttons::{Button, ButtonVariant};
use lithe_ui::feedback::{Alert, EmptyState};
use lithe_ui::forms::{Input, InputEvent, InputState, NumberInput, Switch};
use lithe_ui::layout::{h_stack, v_stack};
use lithe_ui::menus::{DropdownMenu, Menu, MenuItem};
use lithe_ui::overlays::Dialog;
use lithe_ui::primitives::Severity;
use lithe_ui::shell::{WindowRootExt as _, dismiss_hosted};
use lithe_ui::theme::{ActiveTheme as _, Radius, Size};

use crate::identity::{
    GitIdentityPage, IdentityField, IdentityScope, IdentitySetup, git_identity_page,
    host_workspace_root, identity_value_is_valid, identity_value_rejection,
};
use crate::keymap::{KeySource, KeymapPreset, preset_bindings};
use crate::keymap_io::{EXPORT_FILE_NAME, ImportResult, export_payload, parse_import};
use crate::project::{
    DetectedJdk, DetectedMaven, EffectiveToolchain, MINIMUM_JAVA_MAJOR, Overrides,
    ProjectEnvironment, ToolSource, discover, resolve_overrides,
};
use crate::row::{
    RowActivation, appearance_source_note, page_stack, page_title, settings_group, settings_row,
    settings_row_with_note,
};
use crate::run::{RunConfigurationView, RunProjectView};
use crate::schema::{
    DISPLAY_LANGUAGES, EDITOR_FONT_SIZE_MAX, EDITOR_FONT_SIZE_MIN, SHELL_SYSTEM_DEFAULT, Settings,
    TAB_SIZES, TERMINAL_FONT_SIZE_MAX, TERMINAL_FONT_SIZE_MIN, TERMINAL_FONT_SIZE_UNSET,
    TERMINAL_SHELL_IDS, UI_FONT_SIZE_MAX, UI_FONT_SIZE_MIN, UI_FONT_SIZE_STEP,
};
use crate::store::{AppearanceMode, SettingsStore, store};
use crate::theme;
use crate::workspace::{AppearanceKey, AppearanceSource};

pub fn install_actions(cx: &mut App) {
    // `secondary-,` 与 `keymap.rs::DEFAULT_BINDINGS` 报的形态一致（macOS ⌘, / 其他 Ctrl+,）；
    // 旧写的 `ctrl-,` 在 macOS 上等于「设置页显示 ⌘, 但 Ctrl+, 才生效」（F2）。
    cx.bind_keys([KeyBinding::new("secondary-,", OpenSettings, None)]);
    cx.on_action(|_: &OpenSettings, cx: &mut App| {
        let Some(window_handle) = cx.active_window() else {
            return;
        };
        let _ = window_handle.update(cx, |_, window, cx| open_settings_dialog(window, cx));
    });
}

gpui::actions!(lithe_settings, [OpenSettings]);

pub fn open_settings_dialog(window: &mut Window, cx: &mut App) {
    open_settings_dialog_at(window, cx, Category::DEFAULT);
}

fn page_loads_on_open(category: Category) -> bool {
    matches!(category, Category::Project | Category::Run | Category::Lsp)
}

pub fn open_settings_dialog_at(window: &mut Window, cx: &mut App, category: Category) {
    if window.has_active_dialog(cx) {
        return;
    }
    let store = store(cx);
    let view = cx.new(|cx| SettingsDialog::new(store, category, window, cx));
    println!("S1_SETTINGS dialog_opened category={}", category.id());

    if page_loads_on_open(category) {
        view.update(cx, |view, cx| view.load_page_on_open(category, cx));
    }

    window.open_dialog(
        "lithe-settings",
        move |id, window, _cx| {
            let rem = window.rem_size();
            let viewport = window.viewport_size();
            let margin = rem;
            let width = rem_px(rem, DIALOG_WIDTH);
            let height = rem_px(rem, DIALOG_HEIGHT)
                .min((viewport.height - margin * 2.).max(gpui::Pixels::ZERO));
            let view = view.clone();
            Dialog::new(id)
                // 居中卡片（上游 `margin_top((viewport-height)/2)` 等价于默认居中），高度给死；
                // 正文外面套一层定高容器，设置页里的 `h_full()` 才有确定父级。
                // 上游这段本来就没有 `on_close`，所以这里只做一件事：把这一层摘掉。
                .on_close(dismiss_hosted)
                .w(width)
                .h(height)
                .p_0()
                .gap_0()
                .child(div().h(height).child(view))
                .into_any_element()
        },
        cx,
    );
}

pub(crate) use lithe_gpui_shared::rem_px;

fn rem_length(spec_px: f32) -> gpui::Length {
    rems(spec_px / 16.).into()
}

/// 建一个带初值的一行文本输入状态。
///
/// 库的 `InputState` 没有上游的 `default_value(..)` 构造器，初值只能构造后 `set_text(..)`
/// （`set_text` 会发一次 `Changed`；此刻还没有订阅者，不会回流成一次保存）。
fn text_input(
    window: &mut Window,
    cx: &mut Context<InputState>,
    text: impl Into<String>,
) -> InputState {
    let mut state = InputState::new(window, cx);
    state.set_text(text, cx);
    state
}

const DIALOG_WIDTH: f32 = 820.;
const DIALOG_HEIGHT: f32 = 620.;
const NAV_WIDTH: f32 = 190.;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    General,
    Appearance,
    Editor,
    Terminal,
    Project,
    Run,
    Keyboard,
    Lsp,
    Git,
    Logs,
    Updates,
}

impl Category {
    pub const ALL: [Category; 11] = [
        Category::General,
        Category::Appearance,
        Category::Project,
        Category::Run,
        Category::Editor,
        Category::Keyboard,
        Category::Terminal,
        Category::Lsp,
        Category::Git,
        Category::Logs,
        Category::Updates,
    ];

    /// 有真实页面、不需要前置条件的分类。
    ///
    /// `Keyboard` 2026-10-15 从「未实现」转正：`keyboard_page` 已经是完整页——预设下拉、
    /// 可搜索键位表、录制改绑、导入导出都在，重绑链路也是通的（`workbench::keymap_engine`
    /// 注入 `crate::keymap::set_bindable_resolver` 之后由它写回 gpui 的 `Keymap`）。此前的
    /// `prerequisite_key()` 文案还停在「只有固定绑定的少数快捷键」，把能用的功能对用户说成
    /// 没做，所以一并撤掉。
    pub const IMPLEMENTED: [Category; 8] = [
        Category::General,
        Category::Appearance,
        Category::Project,
        Category::Run,
        Category::Editor,
        Category::Keyboard,
        Category::Terminal,
        Category::Lsp,
    ];

    pub const DEFAULT: Category = Category::General;

    pub const fn id(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Appearance => "appearance",
            Self::Editor => "editor",
            Self::Terminal => "terminal",
            Self::Project => "project",
            Self::Run => "run",
            Self::Keyboard => "keyboard",
            Self::Lsp => "lsp",
            Self::Git => "git",
            Self::Logs => "logs",
            Self::Updates => "updates",
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Self::General => "lithe.settings.tabs.general",
            Self::Appearance => "lithe.settings.tabs.appearance",
            Self::Editor => "lithe.settings.tabs.editor",
            Self::Terminal => "lithe.settings.tabs.terminal",
            Self::Project => "lithe.settings.project.title",
            Self::Run => "lithe.settings.run.title",
            Self::Keyboard => "lithe.settings.tabs.keyboard",
            Self::Lsp => "lithe.settings.tabs.lsp",
            Self::Git => "lithe.settings.tabs.git",
            Self::Logs => "lithe.settings.tabs.logs",
            Self::Updates => "lithe.settings.tabs.updates",
        }
    }

    fn prerequisite_key(self) -> Option<&'static str> {
        match self {
            Self::General
            | Self::Appearance
            | Self::Project
            | Self::Run
            | Self::Editor
            | Self::Keyboard
            | Self::Terminal
            | Self::Lsp => None,
            Self::Git => Some("lithe.settings.gpui.prerequisiteGit"),
            Self::Logs => Some("lithe.settings.gpui.prerequisiteLogs"),
            Self::Updates => Some("lithe.settings.gpui.prerequisiteUpdates"),
        }
    }

    /// 本仓自有图标集（`lithe_gpui_shared::icons::idea`）里的分类图形；**没有对应图形的分类给
    /// `None`**，由调用点占一个同尺寸空槽，不拿组件库的 `IconName` 顶替（同
    /// `crates/terminal/src/terminal_view.rs` 的图标口径）。
    fn icon(self) -> Option<&'static idea::IdeaIcon> {
        match self {
            Self::General => Some(&idea::GEAR_ICON),
            Self::Appearance => Some(&idea::PALETTE_ICON),
            Self::Run => Some(&idea::PLAY_ICON),
            Self::Keyboard => Some(&idea::KEYBOARD_ICON),
            Self::Git => Some(&idea::GIT_BRANCH_ICON),
            Self::Updates => Some(&idea::DOWNLOAD_ICON),
            // 本仓图标集里没有「编辑器 / 终端 / 项目 / 语言服务 / 日志」这五张图。
            Self::Editor | Self::Terminal | Self::Project | Self::Lsp | Self::Logs => None,
        }
    }
}

/// 一个分类的图标位：有自有图形就画（按调用点给的边长与颜色），没有就占同尺寸空位
/// ——空位不画任何图形，只保证版面不跳。
fn category_icon(category: Category, side: Rems, color: Hsla, cx: &App) -> AnyElement {
    match category.icon() {
        Some(icon) => idea_icon_svg_px(icon, cx, side)
            .text_color(color)
            .into_any_element(),
        None => div().flex_none().w(side).h(side).into_any_element(),
    }
}

struct GitPageState {
    host: Option<GitIdentityPage>,
    scope: IdentityScope,
    setup: Option<IdentitySetup>,
    busy: bool,
    error: Option<SharedString>,
    saved: Option<IdentityField>,
    generation: u64,
}

impl GitPageState {
    fn new() -> Self {
        Self {
            host: git_identity_page(),
            scope: IdentityScope::default(),
            setup: None,
            busy: false,
            error: None,
            saved: None,
            generation: 0,
        }
    }
}

const GIT_IDENTITY_TAG: &str = "S1_GIT_IDENTITY";

struct ProjectPageState {
    environment: Option<ProjectEnvironment>,
    busy: bool,
    saved: bool,
    generation: u64,
    workspace_root: Option<PathBuf>,
    overrides: Overrides,
    origins: OverrideOrigins,
}

impl ProjectPageState {
    fn new(
        workspace_root: Option<PathBuf>,
        overrides: Overrides,
        origins: OverrideOrigins,
    ) -> Self {
        Self {
            environment: None,
            busy: false,
            saved: false,
            generation: 0,
            workspace_root,
            overrides,
            origins,
        }
    }
}

struct RunPageState {
    root: Option<PathBuf>,
    requested: bool,
    view: Option<RunProjectView>,
    busy: bool,
    error: Option<SharedString>,
    generation: u64,
}

impl RunPageState {
    fn new() -> Self {
        Self {
            root: None,
            requested: false,
            view: None,
            busy: false,
            error: None,
            generation: 0,
        }
    }

    fn mode(&self) -> RunPageMode {
        if !self.requested {
            return RunPageMode::Loading;
        }
        if self.root.is_none() {
            return RunPageMode::NoProject;
        }
        if self.error.is_some() {
            return RunPageMode::Failed;
        }
        match self.view {
            Some(_) => RunPageMode::Ready,
            None => RunPageMode::Loading,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunPageMode {
    NoProject,
    Failed,
    Loading,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolKind {
    Jdk,
    Maven,
}

impl ToolKind {
    fn not_found_key(self) -> &'static str {
        match self {
            Self::Jdk => "lithe.toolchain.javaNotFound",
            Self::Maven => "lithe.toolchain.mavenNotFound",
        }
    }

    fn name_key(self) -> &'static str {
        match self {
            Self::Jdk => "lithe.toolchain.jdk",
            Self::Maven => "lithe.toolchain.maven",
        }
    }

    fn bare_name(self) -> &'static str {
        match self {
            Self::Jdk => "JDK",
            Self::Maven => "Maven",
        }
    }
}

struct EffectiveLine {
    text: SharedString,
    is_error: bool,
    detail: Option<SharedString>,
}

fn describe_effective(kind: ToolKind, effective: &EffectiveToolchain) -> EffectiveLine {
    match effective {
        EffectiveToolchain::Resolved {
            mode,
            path,
            version,
            source,
        } => {
            let name = match version {
                Some(version) => tr_args(kind.name_key(), &[("version", version.as_str())]),
                None => SharedString::from(kind.bare_name()),
            };
            let mut parts = vec![name.to_string(), path.clone()];
            if let Some(source) = source {
                parts.push(source_label(*source).to_string());
            }
            EffectiveLine {
                text: SharedString::from(format!(
                    "{} → {}",
                    tr(mode.label_key()),
                    parts.join(" · ")
                )),
                is_error: false,
                detail: None,
            }
        }
        EffectiveToolchain::Unusable {
            path,
            reason,
            language_service_rejection,
        } => {
            let minimum = MINIMUM_JAVA_MAJOR.to_string();
            let below_gate = SharedString::from(format!(
                "{path} · {}",
                tr_args(
                    "lithe.settings.gpui.jdkBelowLanguageServiceMinimum",
                    &[("minimum", minimum.as_str())],
                )
            ));
            EffectiveLine {
                text: tr_args("lithe.toolchain.invalid", &[("message", reason.as_str())]),
                is_error: true,
                detail: Some(match language_service_rejection {
                    true => below_gate,
                    false => SharedString::from(path.clone()),
                }),
            }
        }
        EffectiveToolchain::NotFound { reason } => EffectiveLine {
            text: tr(kind.not_found_key()),
            is_error: true,
            detail: Some(SharedString::from(reason.clone())),
        },
    }
}

fn source_label(source: ToolSource) -> SharedString {
    match source.label_arg() {
        Some(name) => tr_args(source.label_key(), &[("name", name)]),
        None => tr(source.label_key()),
    }
}

fn draft_value(input: &Entity<InputState>, cx: &App) -> String {
    input.read(cx).text().trim().to_string()
}

fn maven_config_effective(
    field: MavenConfigField,
    environment: Option<&ProjectEnvironment>,
) -> EffectiveLine {
    let Some(environment) = environment else {
        return EffectiveLine {
            text: tr("lithe.toolchain.detecting"),
            is_error: false,
            detail: None,
        };
    };
    let config = &environment.maven_config;

    match field {
        MavenConfigField::SettingsXml => match config.effective_settings() {
            Some(path) if config.override_settings_missing => EffectiveLine {
                text: SharedString::from(path.to_string()),
                is_error: true,
                detail: Some(tr("lithe.settings.gpui.overridePathMissing")),
            },
            Some(path) if config.settings_is_overridden() => EffectiveLine {
                text: SharedString::from(path.to_string()),
                is_error: false,
                detail: Some(tr("lithe.settings.gpui.mavenSettingsEffective")),
            },
            Some(path) => EffectiveLine {
                text: SharedString::from(path.to_string()),
                is_error: false,
                detail: config
                    .installation_settings
                    .as_deref()
                    .filter(|installation| *installation != path)
                    .map(SharedString::from),
            },
            None => EffectiveLine {
                text: tr("lithe.settings.gpui.mavenSettingsMissing"),
                is_error: true,
                detail: None,
            },
        },
        MavenConfigField::LocalRepository => {
            let overridden = environment.overrides.maven_local_repository.trim();
            if !overridden.is_empty() {
                return EffectiveLine {
                    text: SharedString::from(overridden.to_string()),
                    is_error: false,
                    detail: Some(tr("lithe.settings.gpui.mavenOverrideNotEffective")),
                };
            }
            match config.local_repository.as_ref() {
                Some(repository) => EffectiveLine {
                    text: SharedString::from(repository.path.clone()),
                    is_error: false,
                    detail: Some(tr(repository.source.label_key())),
                },
                None => EffectiveLine {
                    text: tr("lithe.settings.gpui.mavenLocalRepositoryUnknown"),
                    is_error: true,
                    detail: None,
                },
            }
        }
    }
}

fn run_page_hint(text: SharedString, cx: &Context<SettingsDialog>) -> gpui::AnyElement {
    div()
        .w_full()
        .text_sm()
        .text_color(cx.theme().colors.fg)
        .child(text)
        .into_any_element()
}

fn detected_language_servers(environment: &ProjectEnvironment) -> Option<SharedString> {
    let jdk = environment
        .jdk
        .detected()
        .or_else(|| environment.jdk.closest_below_requirement())?;
    let home = if jdk.home.is_empty() {
        jdk.executable.clone()
    } else {
        jdk.home.clone()
    };
    let version = if jdk.version.is_empty() {
        "-".to_string()
    } else {
        jdk.version.clone()
    };
    Some(SharedString::from(format!(
        "{home} ({version}) · {}",
        jdk.executable
    )))
}

fn detected_installations(entries: Vec<String>) -> Option<SharedString> {
    if entries.is_empty() {
        return None;
    }
    Some(SharedString::from(format!(
        "{}: {}",
        tr("lithe.settings.project.detected"),
        entries.join("; ")
    )))
}

fn jdk_entry(jdk: &DetectedJdk) -> String {
    let path = if jdk.home.is_empty() {
        jdk.executable.clone()
    } else {
        jdk.home.clone()
    };
    if jdk.version.is_empty() {
        path
    } else {
        format!("{path} ({})", jdk.version)
    }
}

/// 「Java 版本」下拉的选项。
///
/// 第一项固定是「自动检测」（值＝空串，写回去就是清掉全局覆盖），后面是探测到的每个 JDK。
/// 值取 `home` 而不是 `executable`：`javaHomePath` 的语义就是 JDK 主目录，语言服务拿它拼
/// `bin/java`（`java/src/jdtls.rs`），填 java.exe 的路径只会拼出不存在的文件——只认识
/// `executable`、`home` 为空的候选因此不列进来，列了也是写不进去的值。当前值不在候选里时
/// 把它插在「自动检测」后面，免得用户别处填的路径在下拉里看不见（与主题下拉同一条规矩）。
fn java_version_options(
    candidates: &[DetectedJdk],
    configured: &str,
    auto_label: SharedString,
) -> Vec<(SharedString, SharedString, bool)> {
    let configured = configured.trim();
    let mut options = vec![(SharedString::from(""), auto_label, configured.is_empty())];
    let mut seen = vec![String::new()];
    for jdk in candidates {
        let home = jdk.home.trim();
        if home.is_empty() || seen.iter().any(|value| value == home) {
            continue;
        }
        seen.push(home.to_string());
        let label = if jdk.version.trim().is_empty() {
            SharedString::from(home.to_string())
        } else {
            SharedString::from(format!("{home} ({})", jdk.version))
        };
        options.push((SharedString::from(home.to_string()), label, home == configured));
    }
    if !configured.is_empty() && !options.iter().any(|(_, _, checked)| *checked) {
        options.insert(
            1,
            (
                SharedString::from(configured.to_string()),
                SharedString::from(configured.to_string()),
                true,
            ),
        );
    }
    options
}

fn maven_entry(maven: &DetectedMaven) -> String {
    if maven.version.is_empty() {
        maven.executable.clone()
    } else {
        format!("{} ({})", maven.executable, maven.version)
    }
}

fn override_origin_key(origin: OverrideOrigin) -> Option<&'static str> {
    match origin {
        OverrideOrigin::Project => Some("lithe.settings.gpui.overrideFromProject"),
        OverrideOrigin::Global => Some("lithe.settings.gpui.overrideFromGlobal"),
        OverrideOrigin::Unset => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectField {
    Jdk,
    Maven,
    MavenJdk,
}

impl ProjectField {
    const ALL: [ProjectField; 3] = [Self::Jdk, Self::Maven, Self::MavenJdk];

    fn id(self) -> &'static str {
        match self {
            Self::Jdk => "jdk",
            Self::Maven => "maven",
            Self::MavenJdk => "maven_jdk",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Jdk => 0,
            Self::Maven => 1,
            Self::MavenJdk => 2,
        }
    }

    fn origin(self, origins: &OverrideOrigins) -> OverrideOrigin {
        match self {
            Self::Jdk => origins.java_home_path,
            Self::Maven => origins.maven_executable_path,
            Self::MavenJdk => origins.maven_java_home_path,
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            Self::Jdk => "lithe.run.jdkHome",
            Self::Maven => "lithe.run.mavenExecutable",
            Self::MavenJdk => "lithe.run.mavenJdkHome",
        }
    }

    fn hint_key(self) -> &'static str {
        match self {
            Self::Jdk => "lithe.run.jdkHomeHint",
            Self::Maven => "lithe.settings.gpui.mavenExecutableHint",
            Self::MavenJdk => "lithe.run.mavenJdkHomeHint",
        }
    }

    fn pending_key(self) -> Option<&'static str> {
        match self {
            Self::Maven | Self::MavenJdk => Some("lithe.settings.gpui.mavenOverrideNotEffective"),
            Self::Jdk => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MavenConfigField {
    SettingsXml,
    LocalRepository,
}

impl MavenConfigField {
    const ALL: [MavenConfigField; 2] = [Self::SettingsXml, Self::LocalRepository];

    fn id(self) -> &'static str {
        match self {
            Self::SettingsXml => "settings_xml",
            Self::LocalRepository => "local_repository",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::SettingsXml => 0,
            Self::LocalRepository => 1,
        }
    }

    fn origin(self, origins: &OverrideOrigins) -> OverrideOrigin {
        match self {
            Self::SettingsXml => origins.maven_settings_path,
            Self::LocalRepository => origins.maven_local_repository_path,
        }
    }

    fn label(self) -> SharedString {
        match self {
            Self::SettingsXml => SharedString::from("settings.xml"),
            Self::LocalRepository => tr("lithe.maven.localRepository"),
        }
    }

    fn hint_key(self) -> &'static str {
        match self {
            Self::SettingsXml => "lithe.settings.gpui.mavenSettingsHint",
            Self::LocalRepository => "lithe.settings.gpui.mavenLocalRepositoryHint",
        }
    }

    fn picks_file(self) -> bool {
        matches!(self, Self::SettingsXml)
    }
}

fn git_save_enabled(
    setup: Option<&IdentitySetup>,
    scope: IdentityScope,
    field: IdentityField,
    draft: &str,
    busy: bool,
) -> bool {
    let Some(setup) = setup else {
        return false;
    };
    !busy && setup.can_save(scope, field, draft)
}

fn git_clear_enabled(setup: Option<&IdentitySetup>, field: IdentityField, busy: bool) -> bool {
    let Some(setup) = setup else {
        return false;
    };
    !busy && setup.can_clear(field)
}

pub struct SettingsDialog {
    store: Entity<SettingsStore>,
    category: Category,
    git_name_input: Entity<InputState>,
    git_email_input: Entity<InputState>,
    git: GitPageState,
    java_home_input: Entity<InputState>,
    maven_executable_input: Entity<InputState>,
    maven_java_home_input: Entity<InputState>,
    maven_settings_input: Entity<InputState>,
    maven_local_repository_input: Entity<InputState>,
    project: ProjectPageState,
    run: RunPageState,
    /// 快捷键页的搜索框：组件只收 `query`，输入框由页面自己出。
    keymap_search: Entity<InputState>,
    /// 正在录键位的那条命令（`KeybindingsEditor` 用它把该行换成 `HotkeyInput`）。
    keymap_recording: Option<SharedString>,
    /// 导入导出的结果提示。画在键盘页里，走内联 `Alert`：
    /// `ShellWorkspace::notify` 是 `pub(crate)`，为了一条提示把它放开不值得。
    keymap_message: Option<KeymapMessage>,
    _subscriptions: Vec<Subscription>,
}

/// 一条导入导出的结果。
struct KeymapMessage {
    severity: Severity,
    text: SharedString,
    /// 跳过的条数。**非 0 时另起一行说清楚**——跳过数永远不许静默。
    skipped: usize,
}

/// 导出文件上的时间戳。**只给人看**，导入侧从不读它，所以格式随意。
fn export_stamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

/// 键位预设的显示名。**专有名词，不进 i18n**：Lithe / IDEA Classic / Eclipse 中英文同形，
/// 进了翻译表只会变成一条没人维护的空转条目。
fn preset_label(preset: KeymapPreset) -> &'static str {
    match preset {
        KeymapPreset::Lithe => "Lithe",
        KeymapPreset::IdeaClassic => "IDEA Classic",
        KeymapPreset::Eclipse => "Eclipse",
    }
}

/// 当前预设表**没有**覆盖到的命令数。
///
/// 判据只看预设表本身，不看合成结果：一条命令被用户覆盖过（`KeySource::User`）不等于预设
/// 覆盖了它，把用户那几条算进「已覆盖」会把缺口数报小。
///
/// `Lithe` 档**按设计是空表**——它的键位就是默认值本身（`keymap.rs` 模块文档那四层合成的
/// 第 2 条），所以对它一律报 0；否则页面一进来就挂着「此预设不完整」，那是在撒谎。
fn preset_gap(preset: KeymapPreset) -> usize {
    if preset == KeymapPreset::Lithe {
        return 0;
    }
    let rows = preset_bindings(preset);
    CommandId::ALL
        .iter()
        // 分母只数**能绑键**的那 14 条，不是全部 28 条。
        //
        // 那 14 条没有 gpui `Action`，根本绑不了键，预设永远也不可能覆盖它们——给一个没有
        // action 的命令写预设绑定，写出来也是死的。把它们算进来就是报一堆永远修不了的
        // 数字（IDEA Classic 24、Eclipse 25），虚高告警；而页面已经在同批行上标了
        // 「未接入」，告警再数一遍等于同一件事说两遍。
        .filter(|command| crate::keymap::is_bindable(**command))
        .filter(|command| !rows.iter().any(|row| row.command == **command))
        .count()
}

impl SettingsDialog {
    fn new(
        store: Entity<SettingsStore>,
        category: Category,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let saved = store.read(cx).settings().clone();

        let git_name_input = cx.new(|cx| InputState::new(window, cx));
        let git_email_input = cx.new(|cx| InputState::new(window, cx));

        let workspace_root = host_workspace_root();
        let local_toolchain = workspace_root
            .as_deref()
            .map(load_local_toolchain)
            .unwrap_or_default();
        for diagnostic in &local_toolchain.diagnostics {
            eprintln!(
                "{} local_toolchain {diagnostic}",
                crate::project::PROJECT_DIAGNOSTIC_TAG
            );
        }
        let (project_overrides, override_origins) =
            resolve_overrides(&local_toolchain.paths, &Overrides::from_settings(&saved));
        let java_home_input =
            cx.new(|cx| text_input(window, cx, project_overrides.java_home.clone()));
        let maven_executable_input =
            cx.new(|cx| text_input(window, cx, project_overrides.maven_executable.clone()));
        let maven_java_home_input =
            cx.new(|cx| text_input(window, cx, project_overrides.maven_java_home.clone()));
        let maven_settings_input =
            cx.new(|cx| text_input(window, cx, project_overrides.maven_settings.clone()));
        let maven_local_repository_input =
            cx.new(|cx| text_input(window, cx, project_overrides.maven_local_repository.clone()));

        let keymap_search = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr("lithe.settings.keyboard.search"))
        });

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&store, |_, _, cx| cx.notify()));
        subscriptions.push(cx.subscribe_in(
            &keymap_search,
            window,
            |_this: &mut Self, _input, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Changed) {
                    cx.notify();
                }
            },
        ));
        for input in [&git_name_input, &git_email_input] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |_this: &mut Self, _input, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Changed) {
                        cx.notify();
                    }
                },
            ));
        }

        for input in [
            &java_home_input,
            &maven_executable_input,
            &maven_java_home_input,
            &maven_settings_input,
            &maven_local_repository_input,
        ] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |_this: &mut Self, _input, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Changed) {
                        cx.notify();
                    }
                },
            ));
        }

        Self {
            store,
            category,
            git_name_input,
            git_email_input,
            git: GitPageState::new(),
            java_home_input,
            maven_executable_input,
            maven_java_home_input,
            maven_settings_input,
            maven_local_repository_input,
            project: ProjectPageState::new(workspace_root, project_overrides, override_origins),
            run: RunPageState::new(),
            keymap_search,
            keymap_recording: None,
            keymap_message: None,
            _subscriptions: subscriptions,
        }
    }

    fn header(&self, cx: &Context<Self>) -> impl IntoElement {
        h_stack()
            .w_full()
            .h_11()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(cx.theme().colors.border)
            .bg(cx.theme().colors.overlay)
            .text_color(cx.theme().colors.fg)
            .child(idea_icon_image(&idea::GEAR_ICON, cx, rems(1.)))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(tr("lithe.workbench.settings")),
            )
            .child(div().flex_1())
            .child(
                // 库 `Button` 没有 gpui 的 `Styled`：上游的 `.px_0()` 没有落点（图标按钮的方形
                // 几何归库自己）；图标改走 `.icon_element(..)`（库 `.icon(..)` 只吃库内图标名）。
                // 上游那条 `.accessibility_label(..)` 也没有对等物：库只有 `label` 会转成
                // `aria_label`，图标按钮只剩 `tooltip`——这一处登记在汇报的缺口清单里。
                Button::new("settings-close")
                    .size(Size::Small)
                    .variant(ButtonVariant::Ghost)
                    .icon_element(idea_icon_image(&idea::X_ICON, cx, rems(1.)))
                    .tooltip(tr("lithe.tabs.close"))
                    .on_click(|_, window, cx| {
                        window.close_dialog(cx);
                    }),
            )
    }

    fn nav(&self, cx: &Context<Self>) -> impl IntoElement {
        v_stack()
            .id("settings-nav")
            .h_full()
            .w(rem_length(NAV_WIDTH))
            .flex_shrink_0()
            .gap_0p5()
            .p_2()
            .border_r_1()
            .border_color(cx.theme().colors.border)
            .bg(cx.theme().colors.overlay)
            .aria_label(tr("lithe.settings.mac.categories"))
            .children(Category::ALL.iter().enumerate().map(|(index, category)| {
                let selected = *category == self.category;
                // 库 `Button` 没有 gpui 的 `Styled` / `ParentElement`：上游的 `.w_full()` 与
                // `.child(..)`（图标 + 文案的自定义行）在库侧没有落点，几何与内部排版归库自己；
                // `.selected(..)` 也没有对等物——"当前分类"用 `Subtle` 档表达（底色 = `colors.hover`，
                // 与库 `NavItem` 的 active 底色同源）。这两处差异登记在汇报的缺口清单里。
                Button::new(("settings-category", index))
                    .variant(if selected {
                        ButtonVariant::Subtle
                    } else {
                        ButtonVariant::Ghost
                    })
                    .icon_element(category_icon(*category, rems(1.), cx.theme().colors.fg, cx))
                    .label(tr(category.label_key()))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.category = *category;
                        if *category == Category::Git && this.git.setup.is_none() && !this.git.busy
                        {
                            this.git_load(window, cx);
                        }
                        if *category == Category::Project
                            && this.project.environment.is_none()
                            && !this.project.busy
                        {
                            this.project_load(cx);
                        }
                        if *category == Category::Run
                            && this.run.view.is_none()
                            && this.run.error.is_none()
                            && !this.run.busy
                        {
                            this.run_load(cx);
                        }
                        if *category == Category::Lsp
                            && this.project.environment.is_none()
                            && !this.project.busy
                        {
                            this.project_load(cx);
                        }
                        cx.notify();
                    }))
            }))
    }

    fn content(&self, cx: &Context<Self>) -> impl IntoElement {
        let settings = self.store.read(cx).settings().clone();
        div()
            .id("settings-content-scroll")
            .flex_1()
            .min_w_0()
            .h_full()
            .p_6()
            .bg(cx.theme().colors.bg)
            // 上游的 `.overflow_y_scrollbar()` 来自带自绘滚动条的 `ScrollableElement`。
            // 库侧的同位件是 `lithe_ui::layout::ScrollArea`（外层定高 + 内层滚动的盒模型），
            // 直接把本层换成它会改掉 padding 与滚动条的落点；本片按迁移既定做法改用 gpui 自己的
            // `.overflow_y_scroll()`（同 `crates/run/src/pane.rs`、`crates/git/src/log_view.rs`），
            // 「滚动条不再自绘」这一处差异登记在汇报的缺口清单里。
            .overflow_y_scroll()
            .child(
                page_stack()
                    .child(page_title(tr(self.category.label_key())))
                    .children(match self.category {
                        Category::General => self.general_page(&settings, cx),
                        Category::Appearance => self.appearance_page(&settings, cx),
                        Category::Editor => self.editor_page(&settings, cx),
                        Category::Terminal => self.terminal_page(&settings, cx),
                        Category::Project => self.project_page(cx),
                        Category::Run => self.run_page(cx),
                        Category::Git => self.git_page(&settings, cx),
                        Category::Lsp => self.lsp_page(&settings, cx),
                        Category::Keyboard => self.keyboard_page(cx),
                        Category::Logs | Category::Updates => self.empty_page(cx),
                    }),
            )
    }

    fn general_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        vec![
            settings_group(
                tr("lithe.settings.mac.language"),
                vec![settings_row(
                    "settings-row-language",
                    tr("lithe.settings.mac.language"),
                    Some(tr("lithe.settings.gpui.languageRestartDescription")),
                    self.dropdown(
                        "settings-language",
                        language_label(&settings.display_language),
                        DISPLAY_LANGUAGES
                            .iter()
                            .map(|tag| {
                                let value = SharedString::from(*tag);
                                (
                                    value.clone(),
                                    language_label(tag),
                                    settings.display_language == *tag,
                                )
                            })
                            .collect(),
                        {
                            let store = self.store.clone();
                            Box::new(move |value, _, cx| {
                                let changed = store.update(cx, |store, cx| {
                                    store.set_display_language(value.to_string(), cx)
                                });
                                if changed {
                                    crate::restart::restart_application(cx);
                                }
                            })
                        },
                    ),
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    fn appearance_note(&self, key: AppearanceKey, cx: &Context<Self>) -> Option<gpui::AnyElement> {
        let source = self.store.read(cx).appearance_source(key);
        self.appearance_note_for(source, &[key], cx)
    }

    fn appearance_note_for(
        &self,
        source: AppearanceSource,
        keys: &[AppearanceKey],
        cx: &Context<Self>,
    ) -> Option<gpui::AnyElement> {
        if !source.is_workspace_override() {
            return None;
        }
        let store = self.store.clone();
        let keys = keys.to_vec();
        let id = SharedString::from(format!("settings-appearance-revert-{}", keys[0].as_str()));
        let revert = Button::new(id)
            .size(Size::Small)
            .variant(ButtonVariant::Ghost)
            .label(tr("settings.gpui.appearanceRevertToGlobal"))
            .on_click(move |_, _, cx| {
                let keys = keys.clone();
                store.update(cx, |store, cx| {
                    store.revert_appearance_group_to_global(&keys, cx)
                });
            })
            .into_any_element();
        Some(appearance_source_note(tr(source.locale_key()), revert, cx))
    }

    fn appearance_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        let mode = AppearanceMode::from_settings(settings);
        let applied_theme = self.store.read(cx).applied_theme();
        let themes = theme_choices(cx, applied_theme.clone());

        let system_is_dark = theme::system_is_dark(cx.window_appearance());
        let theme_source = self.store.read(cx).effective_theme_source(system_is_dark);
        let mode_source = self.store.read(cx).appearance_mode_source(system_is_dark);
        let theme_keys = [
            AppearanceKey::Theme,
            AppearanceKey::SyncSystemTheme,
            AppearanceKey::AutoThemeLight,
            AppearanceKey::AutoThemeDark,
        ];
        let theme_note = self.appearance_note_for(theme_source, &theme_keys, cx);
        let mode_note = self.appearance_note_for(mode_source, &theme_keys, cx);

        vec![
            settings_group(
                tr("lithe.settings.appearance.theme"),
                vec![
                    settings_row_with_note(
                        "settings-row-theme",
                        tr("lithe.settings.appearance.colorTheme"),
                        None,
                        theme_note,
                        self.dropdown("settings-theme", applied_theme, themes, {
                            let store = self.store.clone();
                            Box::new(move |value, _, cx| {
                                store.update(cx, |store, cx| store.set_theme(value, cx));
                            })
                        }),
                        None,
                        cx,
                    ),
                    settings_row_with_note(
                        "settings-row-appearance-mode",
                        tr("lithe.settings.mac.appearanceMode"),
                        Some(tr("lithe.settings.mac.appearanceDescription")),
                        mode_note,
                        self.dropdown(
                            "settings-appearance-mode",
                            mode_label(mode),
                            vec![
                                (
                                    SharedString::from(MODE_SYSTEM),
                                    tr("lithe.settings.mac.followSystem"),
                                    mode == AppearanceMode::System,
                                ),
                                (
                                    SharedString::from(MODE_LIGHT),
                                    tr("lithe.settings.mac.light"),
                                    mode == AppearanceMode::Light,
                                ),
                                (
                                    SharedString::from(MODE_DARK),
                                    tr("lithe.settings.mac.dark"),
                                    mode == AppearanceMode::Dark,
                                ),
                            ],
                            {
                                let store = self.store.clone();
                                Box::new(move |value, _, cx| {
                                    let mode = match value.as_ref() {
                                        MODE_LIGHT => AppearanceMode::Light,
                                        MODE_DARK => AppearanceMode::Dark,
                                        _ => AppearanceMode::System,
                                    };
                                    store.update(cx, |store, cx| {
                                        store.set_appearance_mode(mode, cx)
                                    });
                                })
                            },
                        ),
                        None,
                        cx,
                    ),
                ],
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.appearance.typography"),
                vec![
                    settings_row_with_note(
                        "settings-row-ui-font-family",
                        tr("lithe.settings.appearance.uiFontFamily"),
                        Some(tr("lithe.settings.appearance.uiFontFamilyDescription")),
                        self.appearance_note(AppearanceKey::FontFamily, cx),
                        self.dropdown(
                            "settings-ui-font-family",
                            font_family_label(&settings.font_family),
                            font_family_options(cx, &settings.font_family),
                            {
                                let store = self.store.clone();
                                Box::new(move |value, _, cx| {
                                    store.update(cx, |store, cx| {
                                        store.set_font_family(value.to_string(), cx)
                                    });
                                })
                            },
                        ),
                        None,
                        cx,
                    ),
                    settings_row_with_note(
                        "settings-row-ui-font-size",
                        tr("lithe.settings.appearance.uiFontSize"),
                        Some(tr("lithe.settings.appearance.uiFontSizeDescription")),
                        self.appearance_note(AppearanceKey::UiFontSize, cx),
                        {
                            let store = self.store.clone();
                            // 库 `NumberInput` 自持文本框（不再由宿主的 `InputState` 驱动），
                            // 值经 `.on_change(..)` 回流；上下限/步长归库，宿主的
                            // `normalize_ui_font_size` 仍是写入时的真源。
                            NumberInput::new("settings-ui-font-size", settings.ui_font_size)
                                .range(UI_FONT_SIZE_MIN, UI_FONT_SIZE_MAX)
                                .step(UI_FONT_SIZE_STEP)
                                // 上游按 0.5 步进，显示走 `format_ui_font_size`（整数不带小数点）；
                                // 库按固定小数位显示（见汇报缺口清单）。
                                .precision(1)
                                .on_change(move |value, _, cx| {
                                    store.update(cx, |store, cx| store.set_ui_font_size(value, cx));
                                })
                                .into_any_element()
                        },
                        None,
                        cx,
                    ),
                ],
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.appearance.interface"),
                vec![settings_row(
                    "settings-row-status-bar",
                    tr("lithe.settings.appearance.showStatusBar"),
                    None,
                    {
                        let store = self.store.clone();
                        Switch::new("settings-show-status-bar")
                            .checked(settings.show_status_bar)
                            .on_change(move |checked, _, cx| {
                                store
                                    .update(cx, |store, cx| store.set_show_status_bar(checked, cx));
                            })
                            .into_any_element()
                    },
                    Some({
                        let activation: RowActivation = {
                            let store = self.store.clone();
                            let next = !settings.show_status_bar;
                            Box::new(move |_, _, cx| {
                                store.update(cx, |store, cx| store.set_show_status_bar(next, cx));
                            })
                        };
                        activation
                    }),
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    fn editor_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        vec![
            settings_group(
                tr("lithe.settings.mac.display"),
                vec![
                    settings_row_with_note(
                        "settings-row-editor-font-family",
                        tr("lithe.settings.editor.fontFamily"),
                        Some(tr("lithe.settings.editor.fontFamilyDescription")),
                        self.appearance_note(AppearanceKey::MonoFontFamily, cx),
                        self.dropdown(
                            "settings-editor-font-family",
                            font_family_label(&settings.mono_font_family),
                            font_family_options(cx, &settings.mono_font_family),
                            {
                                let store = self.store.clone();
                                Box::new(move |value, _, cx| {
                                    store.update(cx, |store, cx| {
                                        store.set_mono_font_family(value.to_string(), cx)
                                    });
                                })
                            },
                        ),
                        None,
                        cx,
                    ),
                    settings_row_with_note(
                        "settings-row-editor-font-size",
                        tr("lithe.settings.mac.fontSize"),
                        Some(tr("lithe.settings.gpui.editorFontSizeDescription")),
                        self.appearance_note(AppearanceKey::EditorFontSize, cx),
                        {
                            let store = self.store.clone();
                            // 同上一处：库 `NumberInput` 自持文本框，整数档（`precision` 缺省 0）
                            // 与上游的 `format_editor_font_size` 显示一致。
                            NumberInput::new("settings-editor-font-size", settings.font_size)
                                .range(EDITOR_FONT_SIZE_MIN, EDITOR_FONT_SIZE_MAX)
                                .step(1.)
                                .on_change(move |value, _, cx| {
                                    store.update(cx, |store, cx| {
                                        store.set_editor_font_size(value, cx)
                                    });
                                })
                                .into_any_element()
                        },
                        None,
                        cx,
                    ),
                    settings_row(
                        "settings-row-editor-word-wrap",
                        tr("lithe.settings.editor.wordWrap"),
                        Some(tr("lithe.settings.editor.wordWrapDescription")),
                        {
                            let store = self.store.clone();
                            Switch::new("settings-editor-word-wrap")
                                .checked(settings.word_wrap)
                                .on_change(move |checked, _, cx| {
                                    store.update(cx, |store, cx| store.set_word_wrap(checked, cx));
                                })
                                .into_any_element()
                        },
                        None,
                        cx,
                    ),
                    // 下面三行与视图菜单里的同名开关是**同一个设置**（`schema::Settings` 的
                    // `minimap` / `line_numbers` / `render_whitespace`）：外壳订阅设置存储后统一
                    // 推给 `EditorPane::set_*`（`workbench/src/workspace.rs`），所以从设置页翻和
                    // 从菜单翻走的是同一条链，不会各记一份状态。
                    settings_row(
                        "settings-row-editor-minimap",
                        tr("lithe.settings.editor.showMinimap"),
                        Some(tr("lithe.settings.editor.showMinimapDescription")),
                        {
                            let store = self.store.clone();
                            Switch::new("settings-editor-minimap")
                                .checked(settings.minimap)
                                .on_change(move |checked, _, cx| {
                                    store.update(cx, |store, cx| store.set_minimap(checked, cx));
                                })
                                .into_any_element()
                        },
                        None,
                        cx,
                    ),
                    settings_row(
                        "settings-row-editor-line-numbers",
                        tr("lithe.settings.editor.lineNumbers"),
                        Some(tr("lithe.settings.editor.lineNumbersDescription")),
                        {
                            let store = self.store.clone();
                            Switch::new("settings-editor-line-numbers")
                                .checked(settings.line_numbers)
                                .on_change(move |checked, _, cx| {
                                    store.update(cx, |store, cx| store.set_line_numbers(checked, cx));
                                })
                                .into_any_element()
                        },
                        None,
                        cx,
                    ),
                    settings_row(
                        "settings-row-editor-render-whitespace",
                        tr("lithe.settings.editor.renderWhitespace"),
                        Some(tr("lithe.settings.editor.renderWhitespaceDescription")),
                        {
                            let store = self.store.clone();
                            Switch::new("settings-editor-render-whitespace")
                                .checked(settings.render_whitespace)
                                .on_change(move |checked, _, cx| {
                                    store.update(cx, |store, cx| {
                                        store.set_render_whitespace(checked, cx)
                                    });
                                })
                                .into_any_element()
                        },
                        None,
                        cx,
                    ),
                ],
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.mac.indentation"),
                vec![settings_row(
                    "settings-row-tab-size",
                    tr("lithe.settings.mac.tabWidth"),
                    None,
                    self.dropdown(
                        "settings-tab-size",
                        tab_size_label(settings.tab_size),
                        TAB_SIZES
                            .iter()
                            .map(|size| {
                                let value = SharedString::from(size.to_string());
                                (value, tab_size_label(*size), settings.tab_size == *size)
                            })
                            .collect(),
                        {
                            let store = self.store.clone();
                            Box::new(move |value, _, cx| {
                                let Ok(size) = value.parse::<u32>() else {
                                    return;
                                };
                                store.update(cx, |store, cx| store.set_tab_size(size, cx));
                            })
                        },
                    ),
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.mac.files"),
                vec![settings_row(
                    "settings-row-editor-auto-save",
                    tr("lithe.settings.mac.autoSave"),
                    Some(tr("lithe.settings.editor.autoSaveDescription")),
                    {
                        let store = self.store.clone();
                        Switch::new("settings-editor-auto-save")
                            .checked(settings.auto_save)
                            .on_change(move |checked, _, cx| {
                                store.update(cx, |store, cx| store.set_auto_save(checked, cx));
                            })
                            .into_any_element()
                    },
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    fn terminal_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        vec![
            settings_group(
                tr("lithe.settings.mac.shell"),
                vec![settings_row(
                    "settings-row-terminal-shell",
                    tr("lithe.settings.mac.defaultShell"),
                    Some(tr("lithe.settings.mac.defaultShellDescription")),
                    self.dropdown(
                        "settings-terminal-shell",
                        shell_label(&settings.terminal_default_shell_id),
                        TERMINAL_SHELL_IDS
                            .iter()
                            .map(|id| {
                                let value = SharedString::from(*id);
                                (
                                    value,
                                    shell_label(id),
                                    settings.terminal_default_shell_id == *id,
                                )
                            })
                            .collect(),
                        {
                            let store = self.store.clone();
                            Box::new(move |value, _, cx| {
                                store.update(cx, |store, cx| {
                                    store.set_terminal_default_shell_id(value.to_string(), cx)
                                });
                            })
                        },
                    ),
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.terminal.typography"),
                vec![settings_row_with_note(
                    "settings-row-terminal-font-size",
                    tr("lithe.settings.terminal.fontSize"),
                    Some(tr("lithe.settings.terminal.fontSizeDescription")),
                    self.appearance_note(AppearanceKey::TerminalFontSize, cx),
                    self.dropdown(
                        "settings-terminal-font-size",
                        terminal_font_size_label(settings.terminal_font_size),
                        terminal_font_size_options(settings.terminal_font_size),
                        {
                            let store = self.store.clone();
                            Box::new(move |value, _, cx| {
                                let Ok(size) = value.parse::<f64>() else {
                                    return;
                                };
                                store
                                    .update(cx, |store, cx| store.set_terminal_font_size(size, cx));
                            })
                        },
                    ),
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    fn git_input(&self, field: IdentityField) -> &Entity<InputState> {
        match field {
            IdentityField::Name => &self.git_name_input,
            IdentityField::Email => &self.git_email_input,
        }
    }

    fn git_draft(&self, field: IdentityField, cx: &Context<Self>) -> SharedString {
        // 库的 `InputState::text()` 给 `&str`（上游 `value()` 直接给 `SharedString`）。
        SharedString::from(self.git_input(field).read(cx).text())
    }

    fn git_fill_input(&self, field: IdentityField, setup: &IdentitySetup, cx: &mut Context<Self>) {
        let text = setup.configured(field).unwrap_or("").to_string();
        // 库的 `set_text(.., cx)` 是 2 参（上游 `set_value(.., window, cx)` 要 `window`）。
        self.git_input(field)
            .update(cx, |input, cx| input.set_text(text, cx));
    }

    fn git_load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(host) = self.git.host.clone() else {
            return;
        };
        self.git.generation = self.git.generation.wrapping_add(1);
        let generation = self.git.generation;
        let scope = self.git.scope;
        self.git.busy = true;
        self.git.error = None;
        self.git.saved = None;
        self.git.setup = None;
        cx.notify();

        self.git_diagnose(&format!(
            "run=load scope={} root={}",
            scope.id(),
            host.workspace_root.display()
        ));

        let entity = cx.entity().downgrade();
        host.load(
            scope,
            move |setup, _window, cx| {
                let _ = entity.update(cx, |this, cx| {
                    if this.git.generation != generation {
                        return;
                    }
                    this.git.busy = false;
                    match setup {
                        Some(setup) => {
                            this.git.setup = Some(setup.clone());
                            this.git_diagnose(&format!(
                                "run=load result=ok scope={} repository={} commits={} configured_name={} configured_email={} effective_name={} effective_email={}",
                                scope.id(),
                                setup.is_repository,
                                setup.has_commits,
                                setup.configured_name.is_some(),
                                setup.configured_email.is_some(),
                                setup.effective_name.is_some(),
                                setup.effective_email.is_some(),
                            ));
                            this.git_fill_input(IdentityField::Name, &setup, cx);
                            this.git_fill_input(IdentityField::Email, &setup, cx);
                        }
                        None => {
                            this.git_diagnose(&format!(
                                "run=load result=failed scope={}",
                                scope.id()
                            ));
                            this.git.error = Some(tr("lithe.git.setup.readFailed"));
                            this.git_fill_input(IdentityField::Name, &IdentitySetup::default(), cx);
                            this.git_fill_input(IdentityField::Email, &IdentitySetup::default(), cx);
                        }
                    }
                    cx.notify();
                });
            },
            window,
            cx,
        );
    }

    fn git_save(
        &mut self,
        field: IdentityField,
        clear: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(host) = self.git.host.clone() else {
            return;
        };
        let scope = self.git.scope;
        let draft = self.git_draft(field, cx);
        let value = if clear {
            None
        } else {
            let value = draft.trim().to_string();
            if !identity_value_is_valid(&value) {
                self.git_diagnose(&format!(
                    "run=save field={} result=blocked reason={}",
                    field.id(),
                    identity_value_rejection(&value).unwrap_or("invalid")
                ));
                return;
            }
            Some(value)
        };

        self.git.generation = self.git.generation.wrapping_add(1);
        let generation = self.git.generation;
        self.git.busy = true;
        self.git.error = None;
        self.git.saved = None;
        cx.notify();

        self.git_diagnose(&format!(
            "run=save scope={} field={} action={} value_len={}",
            scope.id(),
            field.id(),
            if clear { "clear" } else { "set" },
            value.as_ref().map(|value| value.len()).unwrap_or(0)
        ));

        let entity = cx.entity().downgrade();
        host.save(
            scope,
            field,
            value,
            move |setup, _window, cx| {
                let _ = entity.update(cx, |this, cx| {
                    if this.git.generation != generation {
                        return;
                    }
                    this.git.busy = false;
                    match setup {
                        Some(setup) => {
                            this.git.setup = Some(setup.clone());
                            this.git.saved = Some(field);
                            this.git_diagnose(&format!(
                                "run=save field={} result=ok configured={} effective={}",
                                field.id(),
                                setup.configured(field).is_some(),
                                setup.effective(field).is_some(),
                            ));
                            this.git_fill_input(field, &setup, cx);
                        }
                        None => {
                            this.git_diagnose(&format!(
                                "run=save field={} result=failed",
                                field.id()
                            ));
                            this.git.error = Some(tr("lithe.git.setup.save"));
                        }
                    }
                    cx.notify();
                });
            },
            window,
            cx,
        );
    }

    fn git_diagnose(&self, detail: &str) {
        eprintln!("{GIT_IDENTITY_TAG} {detail}");
    }

    fn git_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        if self.git.host.is_none() {
            return self.empty_page(cx);
        }
        let host_root = self
            .git
            .host
            .as_ref()
            .map(|host| host.workspace_root.display().to_string())
            .unwrap_or_default();
        self.git_diagnose(&format!("run=render host=present root={host_root}"));

        let mut rows: Vec<gpui::AnyElement> = Vec::new();

        rows.push(settings_row(
            "settings-row-git-scope",
            tr("lithe.git.setup.scope"),
            Some(tr(if self.git.scope == IdentityScope::Local {
                "lithe.git.setup.localDescription"
            } else {
                "lithe.git.setup.globalDescription"
            })),
            self.dropdown(
                "settings-git-scope",
                git_scope_label(self.git.scope),
                IdentityScope::ALL
                    .iter()
                    .map(|scope| {
                        let value = SharedString::from(scope.id());
                        (value, git_scope_label(*scope), self.git.scope == *scope)
                    })
                    .collect(),
                {
                    let entity = cx.entity().downgrade();
                    Box::new(move |value, window, cx| {
                        let scope = match value.as_ref() {
                            "global" => IdentityScope::Global,
                            _ => IdentityScope::Local,
                        };
                        let _ = entity.update(cx, |this, cx| {
                            if this.git.scope == scope {
                                return;
                            }
                            this.git.scope = scope;
                            this.git_load(window, cx);
                        });
                    })
                },
            ),
            None,
            cx,
        ));

        rows.push(
            div()
                .w_full()
                .text_xs()
                .text_color(cx.theme().colors.fg_muted)
                .child(SharedString::from(host_root))
                .into_any_element(),
        );

        let blocked = self
            .git
            .setup
            .as_ref()
            .is_some_and(|setup| !setup.editable_in(self.git.scope));
        if blocked {
            rows.push(
                div()
                    .w_full()
                    .text_sm()
                    .text_color(cx.theme().colors.fg)
                    .child(tr("lithe.git.setup.initializeFirst"))
                    .into_any_element(),
            );
        }

        for field in [IdentityField::Name, IdentityField::Email] {
            rows.push(self.git_identity_row(field, cx));
        }

        rows.push(
            div()
                .w_full()
                .text_xs()
                .text_color(cx.theme().colors.fg_muted)
                .child(tr("lithe.git.setup.separateSave"))
                .into_any_element(),
        );
        rows.push(
            h_stack()
                .w_full()
                .items_center()
                .gap_3()
                .child(
                    Button::new("settings-git-reload")
                        .size(Size::Small)
                        .variant(ButtonVariant::Ghost)
                        .label(tr("lithe.git.setup.reload"))
                        .disabled(self.git.busy)
                        .on_click(cx.listener(|this, _, window, cx| this.git_load(window, cx))),
                )
                .when(self.git.busy, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().colors.fg_muted)
                            .child(tr("lithe.git.setup.loading")),
                    )
                })
                .when_some(self.git.error.clone(), |this, error| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().colors.danger)
                            .child(error),
                    )
                })
                .when(self.git.saved.is_some(), |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().colors.fg_muted)
                            .child(tr("lithe.git.setup.saved")),
                    )
                })
                .into_any_element(),
        );

        vec![
            settings_group(
                tr("lithe.git.setup.identity"),
                {
                    let mut group_rows = vec![
                        div()
                            .w_full()
                            .text_xs()
                            .text_color(cx.theme().colors.fg_muted)
                            .child(tr("lithe.git.setup.identityDescription"))
                            .into_any_element(),
                    ];
                    group_rows.extend(rows);
                    group_rows
                },
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.git.integration"),
                vec![settings_row(
                    "settings-row-git-confirm-discard",
                    tr("lithe.settings.git.confirmDiscard"),
                    Some(tr("lithe.settings.git.confirmDiscardDescription")),
                    {
                        let store = self.store.clone();
                        Switch::new("settings-git-confirm-discard")
                            .checked(settings.confirm_before_discard)
                            .on_change(move |checked, _, cx| {
                                store.update(cx, |store, cx| {
                                    store.set_confirm_before_discard(checked, cx)
                                });
                            })
                            .into_any_element()
                    },
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    fn git_identity_row(&self, field: IdentityField, cx: &Context<Self>) -> gpui::AnyElement {
        let draft = self.git_draft(field, cx);
        let save_enabled = git_save_enabled(
            self.git.setup.as_ref(),
            self.git.scope,
            field,
            draft.as_ref(),
            self.git.busy,
        );
        let clear_enabled = git_clear_enabled(self.git.setup.as_ref(), field, self.git.busy);
        let input_enabled = !self.git.busy
            && self
                .git
                .setup
                .as_ref()
                .is_some_and(|setup| setup.editable_in(self.git.scope));
        let effective = self
            .git
            .setup
            .as_ref()
            .and_then(|setup| setup.effective(field))
            .map(str::to_string);

        v_stack()
            .id(("settings-git-field", field.index()))
            .w_full()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().colors.fg)
                    .child(tr(field.label_key())),
            )
            .child(
                h_stack()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(self.git_input(field))),
                    )
                    .child(
                        Button::new(("settings-git-save", field.index()))
                            .size(Size::Small)
                            .label(tr("lithe.git.setup.save"))
                            .disabled(!save_enabled)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.git_save(field, false, window, cx)
                            })),
                    )
                    .child(
                        Button::new(("settings-git-clear", field.index()))
                            .size(Size::Small)
                            .variant(ButtonVariant::Ghost)
                            .label(tr("lithe.git.setup.clear"))
                            .disabled(!clear_enabled)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.git_save(field, true, window, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(match effective {
                        Some(value) => SharedString::from(format!(
                            "{}: {value}",
                            tr("lithe.git.setup.effective")
                        )),
                        None => tr("lithe.git.setup.unconfigured"),
                    }),
            )
            .when(!input_enabled, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(tr("lithe.git.setup.initializeFirst")),
                )
            })
            .into_any_element()
    }

    fn project_field_input(&self, field: ProjectField) -> &Entity<InputState> {
        match field {
            ProjectField::Jdk => &self.java_home_input,
            ProjectField::Maven => &self.maven_executable_input,
            ProjectField::MavenJdk => &self.maven_java_home_input,
        }
    }

    fn maven_config_input(&self, field: MavenConfigField) -> &Entity<InputState> {
        match field {
            MavenConfigField::SettingsXml => &self.maven_settings_input,
            MavenConfigField::LocalRepository => &self.maven_local_repository_input,
        }
    }

    fn project_pick_toolchain(
        &self,
        field: ProjectField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_pick(
            self.project_field_input(field).clone(),
            false,
            tr(field.label_key()),
            field.id(),
            window,
            cx,
        );
    }

    fn project_pick_maven_config(
        &self,
        field: MavenConfigField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_pick(
            self.maven_config_input(field).clone(),
            field.picks_file(),
            field.label(),
            field.id(),
            window,
            cx,
        );
    }

    /// 「Java 运行环境」的「选择路径」：原生目录选择器 → 直写全局 `javaHomePath` → 立刻重探一次。
    ///
    /// 直写而不像「项目」页那样等「保存」：这一行是下拉（选中即生效），同一页里两个控件必须同一
    /// 时机，否则用户分不清哪个才算数。重探（`project_load`）是为了让下面的「已检测语言服务器」
    /// 一行马上反映新值——那一行是这一页唯一能回答「选对没有」的地方。
    fn java_version_pick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(tr("lithe.settings.gpui.pickPath")),
        });
        let store = self.store.clone();
        cx.spawn_in(window, async move |this, async_cx| {
            let result = match picked.await {
                Ok(Ok(result)) => result,
                Ok(Err(error)) => {
                    eprintln!("S1_SETTINGS_LSP run=pick state=failed error={error}");
                    return;
                }
                Err(_) => {
                    eprintln!("S1_SETTINGS_LSP run=pick state=cancelled reason=channel-closed");
                    return;
                }
            };
            let Some(path) = result.and_then(|paths| paths.into_iter().next()) else {
                eprintln!("S1_SETTINGS_LSP run=pick state=cancelled");
                return;
            };
            let value = path.to_string_lossy().to_string();
            println!("S1_SETTINGS_LSP run=pick state=picked path={value}");
            let _ = async_cx.update(move |_window, cx| {
                store.update(cx, |store, cx| store.set_java_home_path(value, cx));
                let _ = this.update(cx, |this, cx| this.project_load(cx));
            });
        })
        .detach();
    }

    fn project_pick(
        &self,
        input: Entity<InputState>,
        files: bool,
        label: SharedString,
        target: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files,
            directories: !files,
            multiple: false,
            prompt: Some(label),
        });
        cx.spawn_in(window, async move |this, async_cx| {
            let result = match picked.await {
                Ok(Ok(result)) => result,
                Ok(Err(error)) => {
                    eprintln!(
                        "S1_SETTINGS_PROJECT run=pick target={target} state=failed error={error}"
                    );
                    return;
                }
                Err(_) => {
                    eprintln!(
                        "S1_SETTINGS_PROJECT run=pick target={target} state=cancelled reason=channel-closed"
                    );
                    return;
                }
            };
            let Some(path) = result.and_then(|paths| paths.into_iter().next()) else {
                eprintln!("S1_SETTINGS_PROJECT run=pick target={target} state=cancelled");
                return;
            };
            let value = path.to_string_lossy().to_string();
            println!("S1_SETTINGS_PROJECT run=pick target={target} state=picked path={value}");
            let _ = async_cx.update(move |_window, cx| {
                let _ = this.update(cx, |this, cx| {
                    input.update(cx, |input, cx| input.set_text(value, cx));
                    this.project.saved = false;
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn project_clear(&mut self, field: ProjectField, cx: &mut Context<Self>) {
        let input = self.project_field_input(field).clone();
        input.update(cx, |input, cx| input.set_text("", cx));
        self.project.saved = false;
        self.project_diagnose(&format!("run=clear field={}", field.id()));
        cx.notify();
    }

    fn project_clear_maven_config(&mut self, field: MavenConfigField, cx: &mut Context<Self>) {
        let input = self.maven_config_input(field).clone();
        input.update(cx, |input, cx| input.set_text("", cx));
        self.project.saved = false;
        self.project_diagnose(&format!("run=clear field={}", field.id()));
        cx.notify();
    }

    fn project_diagnose(&self, detail: &str) {
        println!("{} {detail}", crate::project::PROJECT_DIAGNOSTIC_TAG);
    }

    fn project_load(&mut self, cx: &mut Context<Self>) {
        self.project.generation = self.project.generation.wrapping_add(1);
        let generation = self.project.generation;
        self.project.busy = true;
        self.project.saved = false;
        cx.notify();

        let settings = self.store.read(cx).settings().clone();
        let root = self.project.workspace_root.clone();
        self.project_diagnose(&format!(
            "run=discover scope={}",
            if root.is_some() { "project" } else { "global" }
        ));

        cx.spawn(async move |this, cx| {
            let (overrides, origins, environment) = cx
                .background_spawn(async move {
                    let local = root
                        .as_deref()
                        .map(load_local_toolchain)
                        .unwrap_or_default();
                    let (overrides, origins) =
                        resolve_overrides(&local.paths, &Overrides::from_settings(&settings));
                    let environment = discover(&overrides);
                    (overrides, origins, environment)
                })
                .await;
            let empty = environment.has_no_toolchain();
            let _ = this.update(cx, |this, cx| {
                if this.project.generation != generation {
                    return;
                }
                this.project.busy = false;
                this.project.overrides = overrides;
                this.project.origins = origins;
                this.project.environment = Some(environment);
                this.project_diagnose(&format!(
                    "run=discover result=ok generation={generation} no_toolchain={empty} {}",
                    ToolchainPaths::diagnostic_suffix(&this.project.origins)
                ));
                cx.notify();
            });
        })
        .detach();
    }

    fn project_save(&mut self, cx: &mut Context<Self>) {
        let java_home = draft_value(&self.java_home_input, cx);
        let maven_executable = draft_value(&self.maven_executable_input, cx);
        let maven_java_home = draft_value(&self.maven_java_home_input, cx);
        let maven_settings = draft_value(&self.maven_settings_input, cx);
        let maven_local_repository = draft_value(&self.maven_local_repository_input, cx);
        let drafts = Overrides {
            java_home,
            maven_executable,
            maven_java_home,
            maven_settings,
            maven_local_repository,
        };
        let state = |value: &str| if value.is_empty() { "-" } else { "set" };
        let scope = if self.project.workspace_root.is_some() {
            "project"
        } else {
            "global"
        };
        self.project_diagnose(&format!(
            "run=save scope={scope} javaHome={} mavenExecutable={} mavenJavaHome={} mavenSettings={} mavenLocalRepo={}",
            state(&drafts.java_home),
            state(&drafts.maven_executable),
            state(&drafts.maven_java_home),
            state(&drafts.maven_settings),
            state(&drafts.maven_local_repository),
        ));

        let Some(root) = self.project.workspace_root.clone() else {
            let store = self.store.clone();
            store.update(cx, |store, cx| {
                store.set_java_home_path(drafts.java_home, cx);
                store.set_maven_executable_path(drafts.maven_executable, cx);
                store.set_maven_java_home_path(drafts.maven_java_home, cx);
                store.set_maven_settings_path(drafts.maven_settings, cx);
                store.set_maven_local_repository_path(drafts.maven_local_repository, cx);
            });
            self.project_load(cx);
            self.project.saved = true;
            return;
        };

        let paths = drafts.to_local();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { save_local_toolchain(&root, &paths) })
                .await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(saved) => {
                    this.project_diagnose(&format!(
                        "run=save result=ok runLocalBytes={} mavenLocalBytes={} exclusion={:?} ignore={:?}",
                        saved.run_local_bytes,
                        saved.maven_local_bytes,
                        saved.exclusion,
                        saved.ignore
                    ));
                    this.project_load(cx);
                    this.project.saved = true;
                }
                Err(error) => {
                    this.project_diagnose(&format!("run=save result=failed error={error}"));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn project_page(&self, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        let environment = self.project.environment.as_ref();
        let detecting = || EffectiveLine {
            text: tr("lithe.toolchain.detecting"),
            is_error: false,
            detail: None,
        };

        let mut toolchain_rows: Vec<gpui::AnyElement> = Vec::new();

        let scope_key = if self.project.workspace_root.is_some() {
            "lithe.settings.gpui.projectScopeProject"
        } else {
            "lithe.settings.gpui.projectScopeGlobal"
        };
        toolchain_rows.push(
            div()
                .w_full()
                .text_xs()
                .text_color(cx.theme().colors.fg_muted)
                .child(tr(scope_key))
                .into_any_element(),
        );

        if environment.is_some_and(ProjectEnvironment::has_no_toolchain) {
            toolchain_rows.push(
                div()
                    .w_full()
                    .text_sm()
                    .text_color(cx.theme().colors.danger)
                    .child(tr("lithe.settings.gpui.projectNothingDetected"))
                    .into_any_element(),
            );
        }

        let jdk_candidates = || {
            environment.and_then(|environment| {
                detected_installations(environment.jdk.candidates.iter().map(jdk_entry).collect())
            })
        };
        for field in ProjectField::ALL {
            let effective = match (field, environment) {
                (ProjectField::Jdk, Some(environment)) => {
                    describe_effective(ToolKind::Jdk, &environment.jdk.effective())
                }
                (ProjectField::Maven, Some(environment)) => {
                    describe_effective(ToolKind::Maven, &environment.maven.effective())
                }
                (ProjectField::MavenJdk, Some(environment)) => {
                    describe_effective(ToolKind::Jdk, &environment.maven_jdk_effective())
                }
                (_, None) => detecting(),
            };
            let detected = match field {
                ProjectField::Maven => environment.and_then(|environment| {
                    detected_installations(
                        environment
                            .maven
                            .candidates
                            .iter()
                            .map(maven_entry)
                            .collect(),
                    )
                }),
                ProjectField::Jdk | ProjectField::MavenJdk => jdk_candidates(),
            };
            toolchain_rows.push(self.project_toolchain_row(field, effective, detected, cx));
        }

        toolchain_rows.push(self.project_actions(cx));

        let maven_rows: Vec<gpui::AnyElement> = MavenConfigField::ALL
            .into_iter()
            .map(|field| self.project_maven_config_row(field, cx))
            .collect();

        vec![
            settings_group(tr("lithe.settings.project.detected"), toolchain_rows, cx)
                .into_any_element(),
            settings_group(tr("lithe.maven.settings"), maven_rows, cx).into_any_element(),
        ]
    }

    fn project_toolchain_row(
        &self,
        field: ProjectField,
        effective: EffectiveLine,
        detected: Option<SharedString>,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        v_stack()
            .id(("settings-project-row", field.index()))
            .w_full()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().colors.fg)
                    .child(tr(field.label_key())),
            )
            .child(
                h_stack()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(self.project_field_input(field))),
                    )
                    .child(
                        Button::new(("settings-project-pick", field.index()))
                            .size(Size::Small)
                            .variant(ButtonVariant::Ghost)
                            .label(tr("lithe.settings.gpui.pickPath"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.project_pick_toolchain(field, window, cx)
                            })),
                    )
                    .child(
                        Button::new(("settings-project-clear", field.index()))
                            .size(Size::Small)
                            .variant(ButtonVariant::Ghost)
                            .label(tr("lithe.ui.clear"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                this.project_clear(field, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(tr(field.hint_key())),
            )
            .children(
                override_origin_key(field.origin(&self.project.origins)).map(|key| {
                    div()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(tr(key))
                }),
            )
            .children(field.pending_key().map(|key| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().colors.warning)
                    .child(tr(key))
            }))
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(if effective.is_error {
                        cx.theme().colors.danger
                    } else {
                        cx.theme().colors.fg
                    })
                    .child(effective.text),
            )
            .children(effective.detail.map(|detail| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(detail)
            }))
            .children(detected.map(|detected| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(detected)
            }))
            .into_any_element()
    }

    fn project_fact_row(
        &self,
        id: &'static str,
        label: SharedString,
        value: SharedString,
        note: Option<SharedString>,
        is_error: bool,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        v_stack()
            .id(id)
            .w_full()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().colors.fg)
                    .child(label),
            )
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(if is_error {
                        cx.theme().colors.danger
                    } else {
                        cx.theme().colors.fg
                    })
                    .child(value),
            )
            .children(note.map(|note| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(note)
            }))
            .into_any_element()
    }

    fn project_maven_config_row(
        &self,
        field: MavenConfigField,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let effective = maven_config_effective(field, self.project.environment.as_ref());
        v_stack()
            .id(("settings-maven-config-row", field.index()))
            .w_full()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().colors.fg)
                    .child(field.label()),
            )
            .child(
                h_stack()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(self.maven_config_input(field))),
                    )
                    .child(
                        Button::new(("settings-maven-pick", field.index()))
                            .size(Size::Small)
                            .variant(ButtonVariant::Ghost)
                            .label(tr("lithe.settings.gpui.pickPath"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.project_pick_maven_config(field, window, cx)
                            })),
                    )
                    .child(
                        Button::new(("settings-maven-clear", field.index()))
                            .size(Size::Small)
                            .variant(ButtonVariant::Ghost)
                            .label(tr("lithe.ui.clear"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                this.project_clear_maven_config(field, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(tr(field.hint_key())),
            )
            .children(
                override_origin_key(field.origin(&self.project.origins)).map(|key| {
                    div()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(tr(key))
                }),
            )
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(if effective.is_error {
                        cx.theme().colors.danger
                    } else {
                        cx.theme().colors.fg
                    })
                    .child(effective.text),
            )
            .children(effective.detail.map(|detail| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().colors.fg_muted)
                    .child(detail)
            }))
            .into_any_element()
    }

    fn project_actions(&self, cx: &Context<Self>) -> gpui::AnyElement {
        h_stack()
            .w_full()
            .items_center()
            .gap_3()
            .pt_2()
            .child(
                Button::new("settings-project-refresh")
                    .size(Size::Small)
                    .variant(ButtonVariant::Ghost)
                    .label(tr("lithe.settings.project.refresh"))
                    .disabled(self.project.busy)
                    .on_click(cx.listener(|this, _, _window, cx| this.project_load(cx))),
            )
            .child(
                Button::new("settings-project-save")
                    .size(Size::Small)
                    .label(tr("lithe.ui.save"))
                    .disabled(self.project.busy)
                    .on_click(cx.listener(|this, _, _window, cx| this.project_save(cx))),
            )
            .when(self.project.busy, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(tr("lithe.toolchain.detecting")),
                )
            })
            .when(self.project.saved && !self.project.busy, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(tr("lithe.settings.project.saved")),
                )
            })
            .into_any_element()
    }

    fn load_page_on_open(&mut self, category: Category, cx: &mut Context<Self>) {
        match category {
            Category::Project => self.project_load(cx),
            Category::Run => self.run_load(cx),
            Category::Lsp => {
                if self.project.environment.is_none() && !self.project.busy {
                    self.project_load(cx);
                }
            }
            _ => {}
        }
    }

    fn lsp_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        let environment = self.project.environment.as_ref();
        let detecting = || SharedString::from(tr("lithe.toolchain.detecting"));

        let mut service_rows: Vec<gpui::AnyElement> = vec![settings_row(
            "settings-row-lsp-auto-completion",
            tr("lithe.settings.mac.autoCompletion"),
            Some(tr("lithe.settings.gpui.autoCompletionDescription")),
            {
                let store = self.store.clone();
                Switch::new("settings-lsp-auto-completion")
                    .checked(settings.auto_completion)
                    .on_change(move |checked, _, cx| {
                        store.update(cx, |store, cx| store.set_auto_completion(checked, cx));
                    })
                    .into_any_element()
            },
            None,
            cx,
        )];
        service_rows.push(
            div()
                .w_full()
                .text_xs()
                .text_color(cx.theme().colors.fg_muted)
                .child(tr("lithe.settings.gpui.lspOnlyAutoCompletion"))
                .into_any_element(),
        );

        let minimum = MINIMUM_JAVA_MAJOR.to_string();
        let (server_value, server_is_error) = match environment {
            None => (detecting(), false),
            Some(environment) => match detected_language_servers(environment) {
                Some(servers) => {
                    let usable = environment.jdk.detected().is_some();
                    let text = if usable {
                        servers
                    } else {
                        SharedString::from(format!(
                            "{} · {}",
                            servers,
                            tr_args(
                                "lithe.settings.gpui.jdkBelowLanguageServiceMinimum",
                                &[("minimum", minimum.as_str())],
                            )
                        ))
                    };
                    (text, !usable)
                }
                None => (
                    SharedString::from(tr("lithe.settings.gpui.noLanguageServer")),
                    true,
                ),
            },
        };

        let configured = settings.java_home_path.trim().to_string();
        let candidates = environment
            .map(|environment| environment.jdk.candidates.clone())
            .unwrap_or_default();
        let auto_label = tr("lithe.settings.gpui.javaVersionAuto");
        let options = java_version_options(&candidates, &configured, auto_label.clone());
        let current_label = options
            .iter()
            .find(|(_, _, checked)| *checked)
            .map(|(_, label, _)| label.clone())
            .unwrap_or(auto_label);
        // 下拉只列得出「探测到的」JDK：装在别处（另一个盘、非标准目录）的那个只能用「选择路径」
        // 指给它。用户的实际场景就是这一条——项目要 1.8，语言服务必须 21，而 21 不在探测范围里。
        let control = h_stack()
            .items_center()
            .gap_2()
            .child(self.dropdown("settings-lsp-java-version", current_label, options, {
                let store = self.store.clone();
                Box::new(move |value, _, cx| {
                    store.update(cx, |store, cx| store.set_java_home_path(value.to_string(), cx));
                })
            }))
            .child(
                Button::new("settings-lsp-java-version-pick")
                    .size(Size::Small)
                    .variant(ButtonVariant::Ghost)
                    .label(tr("lithe.settings.gpui.pickPath"))
                    .on_click(cx.listener(|this, _, window, cx| this.java_version_pick(window, cx))),
            )
            .into_any_element();
        let java_rows: Vec<gpui::AnyElement> = vec![settings_row(
            "settings-row-lsp-java-version",
            tr("lithe.settings.gpui.javaVersion"),
            Some(tr("lithe.settings.gpui.javaVersionDescription")),
            control,
            None,
            cx,
        )];

        vec![
            settings_group(tr("lithe.settings.mac.languageServices"), service_rows, cx)
                .into_any_element(),
            settings_group(tr("lithe.settings.gpui.javaRuntime"), java_rows, cx)
                .into_any_element(),
            settings_group(
                tr("lithe.settings.mac.detectedServers"),
                vec![self.project_fact_row(
                    "settings-lsp-detected-servers",
                    SharedString::from(tr("lithe.settings.mac.detectedServers")),
                    server_value,
                    Some(tr("lithe.settings.mac.detectedServersDescription")),
                    server_is_error,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    fn run_diagnose(&self, detail: &str) {
        println!("{} {detail}", crate::run::RUN_DIAGNOSTIC_TAG);
    }

    fn run_load(&mut self, cx: &mut Context<Self>) {
        self.run.generation = self.run.generation.wrapping_add(1);
        let generation = self.run.generation;
        let root = crate::identity::host_workspace_root();
        self.run.root = root.clone();
        self.run.requested = true;
        self.run.busy = true;
        self.run.error = None;
        cx.notify();

        let Some(root) = root else {
            self.run.busy = false;
            self.run_diagnose(&format!(
                "run=load generation={generation} result=skipped reason=no_workspace_root"
            ));
            cx.notify();
            return;
        };

        self.run_diagnose(&format!(
            "run=load generation={generation} root={}",
            root.display()
        ));

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { crate::run::load(&root) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.run.generation != generation {
                    return;
                }
                this.run.busy = false;
                match result {
                    Ok(view) => {
                        this.run_diagnose(&format!(
                            "run=loaded generation={generation} rows={} entryCount={}",
                            view.configurations.len(),
                            view.entry_count
                        ));
                        this.run.view = Some(view);
                    }
                    Err(error) => {
                        this.run_diagnose(&format!("run=failed generation={generation}"));
                        this.run.error = Some(tr_args(
                            "lithe.settings.gpui.runLoadFailed",
                            &[("reason", &error)],
                        ));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn run_page(&self, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        let mut rows: Vec<gpui::AnyElement> = Vec::new();

        rows.push(
            div()
                .w_full()
                .text_sm()
                .text_color(cx.theme().colors.fg_muted)
                .child(tr("lithe.settings.gpui.runReadOnlyDescription"))
                .into_any_element(),
        );

        if let Some(error) = self.run.error.as_ref() {
            rows.push(
                div()
                    .w_full()
                    .text_sm()
                    .text_color(cx.theme().colors.danger)
                    .child(error.clone())
                    .into_any_element(),
            );
        }

        if self.run.mode() == RunPageMode::Ready {
            if let Some(view) = self.run.view.as_ref() {
                rows.push(
                    div()
                        .w_full()
                        .text_xs()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(tr_args(
                            "lithe.run.generatedEntries",
                            &[("count", &view.entry_count.to_string())],
                        ))
                        .into_any_element(),
                );
            }
        }

        rows.push(
            h_stack()
                .w_full()
                .items_center()
                .gap_3()
                .pt_2()
                .child(
                    Button::new("settings-run-refresh")
                        .size(Size::Small)
                        .variant(ButtonVariant::Ghost)
                        .label(if self.run.busy {
                            tr("lithe.settings.project.loading")
                        } else {
                            tr("lithe.run.identifyAgain")
                        })
                        .disabled(self.run.busy)
                        .on_click(cx.listener(|this, _, _window, cx| this.run_load(cx))),
                )
                .into_any_element(),
        );

        match self.run.mode() {
            RunPageMode::NoProject => {
                rows.push(run_page_hint(tr("lithe.settings.gpui.runNoProject"), cx));
            }
            RunPageMode::Failed | RunPageMode::Loading => {}
            RunPageMode::Ready => {
                if let Some(view) = self.run.view.as_ref() {
                    if view.configurations.is_empty() {
                        rows.push(run_page_hint(
                            tr("lithe.settings.gpui.runNothingDetected"),
                            cx,
                        ));
                    } else {
                        for (index, configuration) in view.configurations.iter().enumerate() {
                            rows.push(self.run_configuration_row(index, configuration, cx));
                        }
                    }
                }
            }
        }

        rows.push(
            div()
                .w_full()
                .text_xs()
                .text_color(cx.theme().colors.fg_muted)
                .child(tr("lithe.settings.gpui.runNotWired"))
                .into_any_element(),
        );

        rows
    }

    fn run_configuration_row(
        &self,
        index: usize,
        configuration: &RunConfigurationView,
        cx: &Context<Self>,
    ) -> gpui::AnyElement {
        let fact = |label_key: &'static str, value: SharedString, muted: bool| {
            h_stack()
                .w_full()
                .items_start()
                .gap_2()
                .child(
                    div()
                        .w(rems(7.5))
                        .flex_shrink_0()
                        .text_sm()
                        .text_color(cx.theme().colors.fg_muted)
                        .child(tr(label_key)),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_sm()
                        .text_color(if muted {
                            cx.theme().colors.fg_muted
                        } else {
                            cx.theme().colors.fg
                        })
                        .child(value),
                )
        };

        let kind = format!(
            "{} · {}（{}）",
            configuration.provider_title,
            tr(configuration.execution.label_key()),
            configuration.provider
        );

        let (command, command_muted) = match configuration.command_line() {
            Some(line) => (SharedString::from(line), false),
            None => (tr("lithe.settings.gpui.runNoCommand"), true),
        };

        let mut source = tr(configuration.layer.label_key()).to_string();
        if let Some(manifest) = configuration.manifest.as_ref() {
            source.push_str(" · ");
            source.push_str(manifest);
        }

        v_stack()
            .id(("settings-run-row", index))
            .w_full()
            .gap_1()
            .p_3()
            .rounded(cx.theme().radius(Radius::Md))
            .border_1()
            .border_color(cx.theme().colors.border)
            .bg(cx.theme().colors.overlay.opacity(0.35))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().colors.fg)
                    .child(configuration.name.clone()),
            )
            .child(fact("lithe.run.type", SharedString::from(kind), false))
            .child(fact("lithe.run.command", command, command_muted))
            .child(fact(
                "lithe.run.workingDirectory",
                SharedString::from(configuration.cwd.clone()),
                false,
            ))
            .child(fact(
                "lithe.run.effectiveSource",
                SharedString::from(source),
                false,
            ))
            .children(configuration.main_class.as_ref().map(|main_class| {
                fact(
                    "lithe.run.mainClass",
                    SharedString::from(main_class.clone()),
                    false,
                )
            }))
            .into_any_element()
    }

    fn empty_page(&self, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        let prerequisite = self
            .category
            .prerequisite_key()
            .expect("空态分类必须登记一条前置条件文案（`Category::prerequisite_key`）");
        let category = self.category;
        vec![
            v_stack()
                .w_full()
                .py_8()
                .child(
                    // 上游 `Empty` / `EmptyHeader` / `EmptyMedia` / `EmptyTitle` / `EmptyDescription`
                    // 一族，在库侧是**槽位式**的 `EmptyState`：`EmptyMedia` → `media_element`、
                    // `EmptyTitle` → `title`、`EmptyDescription` → `body`；版面（间距、字号、
                    // 最大宽度、那圈透明边框）归库自己定，宿主不再逐项给。
                    // 头槽位的边长沿用上游的 `size_10`（2.5rem = 40px）。
                    EmptyState::slots("settings-empty-page")
                        .media_element(category_icon(
                            category,
                            rems(2.5),
                            cx.theme().colors.fg_muted,
                            cx,
                        ))
                        .title(tr("lithe.settings.gpui.pageNotAvailableTitle"))
                        .body(tr(prerequisite)),
                )
                .into_any_element(),
        ]
    }

    fn footer(&self, cx: &Context<Self>) -> impl IntoElement {
        h_stack()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .border_t_1()
            .border_color(cx.theme().colors.border)
            .child(
                Button::new("settings-restore-defaults")
                    .variant(ButtonVariant::Ghost)
                    .label(tr("lithe.settings.gpui.restoreDefaultsOpen"))
                    .on_click(
                        cx.listener(|this, _, window, cx| {
                            this.confirm_restore_defaults(window, cx)
                        }),
                    ),
            )
            .child(
                Button::new("settings-done")
                    .primary()
                    .label(tr("lithe.settings.mac.done"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        let store = this.store.clone();
                        store.update(cx, |store, _| store.flush_pending());
                        window.close_dialog(cx);
                    })),
            )
    }

    fn confirm_restore_defaults(&self, window: &mut Window, cx: &mut Context<Self>) {
        let store = self.store.clone();

        window.open_dialog(
            "lithe-settings-restore-defaults",
            move |id, _window, cx| {
                let store = store.clone();
                let (cancel_id, confirm_id) = ((id.clone(), "cancel"), (id.clone(), "confirm"));
                Dialog::new(id)
                    .title(tr("lithe.settings.gpui.restoreDefaultsTitle"))
                    .on_close(dismiss_hosted)
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().colors.fg_muted)
                            .child(tr("lithe.settings.gpui.restoreDefaultsBody")),
                    )
                    .action(move |close| {
                        Button::new(cancel_id)
                            .label(tr("lithe.ui.cancel"))
                            .on_click(move |_, window, cx| close(window, cx))
                    })
                    .action(move |close| {
                        Button::new(confirm_id)
                            .variant(ButtonVariant::Danger)
                            .label(tr("lithe.settings.mac.restoreDefaults"))
                            .on_click(move |_, window, cx| {
                                // 字号输入现在是库的 `NumberInput`：它自己把显示同步到传入的
                                // `value`（`settings.ui_font_size` / `settings.font_size`）。
                                // `restore_defaults` 会 `notify()` 存储，本对话框的
                                // `cx.observe(&store, ..)` 随即重绘，上游那两处手写 `set_value`
                                // 因此不再需要。
                                store.update(cx, |store, cx| store.restore_defaults(cx));
                                close(window, cx);
                            })
                    })
                    .into_any_element()
            },
            cx,
        );
    }

    /// 快捷键页：预设选择 + 一张可搜索的键位表。
    ///
    /// 命令文案取自 [`crate::command_label_key`]，键位与来源取自 `KeymapSettings::resolve`，
    /// **能不能改**由 workbench 注入的解析器定（[`crate::keymap::is_bindable`]）：判不了可绑的
    /// 一律灰掉，不放「改了也不生效」的编辑框。
    fn keyboard_page(&self, cx: &Context<Self>) -> Vec<gpui::AnyElement> {
        use lithe_ui::editor::{KeySource as RowSource, Keybinding, KeybindingsEditor};

        let keymap = self.store.read(cx).keymap().clone();
        let preset = keymap.preset;

        // 预设名是专有名词，中英文同形，按裁定不进 i18n。
        let options: Vec<(SharedString, SharedString, bool)> = KeymapPreset::ALL
            .iter()
            .map(|option| {
                (
                    SharedString::from(option.id()),
                    SharedString::from(preset_label(*option)),
                    *option == preset,
                )
            })
            .collect();
        let preset_control = {
            let store = self.store.clone();
            self.dropdown(
                "settings-keymap-preset",
                SharedString::from(preset_label(preset)),
                options,
                Box::new(move |value, _, cx| {
                    let Some(next) = KeymapPreset::from_id(&value) else {
                        return;
                    };
                    store.update(cx, |store, cx| store.set_keymap_preset(next, cx));
                }),
            )
        };

        let rows: Vec<Keybinding> = CommandId::ALL
            .iter()
            .map(|command| {
                let resolved = keymap.resolve(*command);
                let bindable = crate::keymap::is_bindable(*command);
                Keybinding {
                    command: SharedString::from(command.id()),
                    title: tr(&format!("lithe.{}", crate::command_label_key(*command))),
                    keys: resolved
                        .is_bound()
                        .then(|| SharedString::from(resolved.keystrokes.join(" "))),
                    when: resolved.context.map(SharedString::from),
                    source: match resolved.source {
                        KeySource::Default => RowSource::Default,
                        KeySource::Preset => RowSource::Preset,
                        KeySource::User => RowSource::User,
                    },
                    bindable,
                }
            })
            .collect();

        let query = self.keymap_search.read(cx).text().to_string();
        let recording = self.keymap_recording.clone();
        let store = self.store.clone();
        let search = self.keymap_search.clone();
        // 录制状态挂在本对话框上，而 `on_record` 的闭包只有 `&mut App`，所以先抓句柄。
        let this = cx.entity();

        // 预设覆盖缺口：老前端（`keyboard-settings.tsx`）也是这一条告警，文案与插值占位
        // 沿用既有的 `settings.keyboard.presetIncomplete` / `presetSingularSuffix` /
        // `presetPluralSuffix`，不新增 key。
        let gap = preset_gap(preset);
        let gap_alert = (gap > 0).then(|| {
            let count = gap.to_string();
            let suffix = if gap == 1 {
                tr("lithe.settings.keyboard.presetSingularSuffix")
            } else {
                tr("lithe.settings.keyboard.presetPluralSuffix")
            };
            Alert::new(
                "settings-keymap-preset-gap",
                lithe_ui::primitives::Severity::Warning,
                tr_args(
                    "lithe.settings.keyboard.presetIncomplete",
                    &[("count", count.as_str()), ("suffix", suffix.as_str())],
                ),
            )
            .into_any_element()
        });

        // 导入导出。文件是 gpui 端自闭环的，格式串刻意与 Windows 遗留前端不同
        // （`keymap_io` 模块文档），老文件会在第一道格式校验就被挡下。
        let io_row = h_stack()
            .gap_2()
            .child(
                Button::new("settings-keymap-import")
                    .size(Size::Small)
                    .label(tr("lithe.settings.keyboard.import"))
                    .on_click(cx.listener(|this, _, window, cx| this.keymap_import(window, cx))),
            )
            .child(
                Button::new("settings-keymap-export")
                    .size(Size::Small)
                    .label(tr("lithe.settings.keyboard.export"))
                    .on_click(cx.listener(|this, _, window, cx| this.keymap_export(window, cx))),
            )
            .into_any_element();

        // 结果提示画在按钮下面：用户刚点的地方，比全局通知好找。
        let message_alert = self.keymap_message.as_ref().map(|message| {
            let alert = Alert::new(
                "settings-keymap-io-message",
                message.severity,
                message.text.clone(),
            );
            // 跳过数**永远**要说出来：只报「已导入 N 个」会让用户以为那几条也进去了。
            if message.skipped > 0 {
                let count = message.skipped.to_string();
                alert
                    .body(tr_args(
                        "lithe.settings.keyboard.importSkipped",
                        &[("count", count.as_str())],
                    ))
                    .into_any_element()
            } else {
                alert.into_any_element()
            }
        });

        vec![
            settings_group(
                tr("lithe.settings.keyboard.presetLabel"),
                vec![settings_row(
                    "settings-row-keymap-preset",
                    tr("lithe.settings.keyboard.presetLabel"),
                    Some(tr("lithe.settings.keyboard.presetDescription")),
                    preset_control,
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
        .into_iter()
        .chain(gap_alert)
        .chain(std::iter::once(io_row))
        .chain(message_alert)
        .chain(std::iter::once(
            v_stack()
                .w_full()
                .gap_2()
                .child(
                    div()
                        .w(px(320.))
                        .child(lithe_ui::forms::Input::new(&search)),
                )
                .child(
                    KeybindingsEditor::new("settings-keymap-editor", rows)
                        .query(query)
                        .recording(recording.clone())
                        .on_record(move |command, _, cx| {
                            // 再点一次同一行 = 取消录制。
                            this.update(cx, |this, cx| {
                                this.keymap_recording =
                                    if this.keymap_recording.as_ref() == Some(command) {
                                        None
                                    } else {
                                        Some(command.clone())
                                    };
                                cx.notify();
                            });
                        })
                        .on_bind({
                            let store = store.clone();
                            move |command, stroke, _, cx| {
                                let Some(id) = CommandId::from_id(command) else {
                                    return;
                                };
                                let keys = match stroke {
                                    Some(stroke) => vec![stroke.to_string()],
                                    // 清掉键位 = 显式不绑定，落到 `set_key_override` 的空列表语义。
                                    None => Vec::new(),
                                };
                                store.update(cx, |store, cx| {
                                    let preset = store.keymap().preset;
                                    store.set_key_override(preset, id, keys, cx);
                                });
                            }
                        }),
                )
                .into_any_element(),
        ))
        .collect()
    }

    /// 导出：把**当前预设那一桶**覆盖写到用户选的文件。
    fn keymap_export(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let keymap = self.store.read(cx).keymap().clone();
        let text = match serde_json::to_string_pretty(&export_payload(&keymap, &export_stamp())) {
            Ok(text) => text,
            Err(error) => {
                self.keymap_message = Some(KeymapMessage {
                    severity: Severity::Danger,
                    text: tr_args(
                        "lithe.settings.keyboard.exportFailed",
                        &[("error", &error.to_string())],
                    ),
                    skipped: 0,
                });
                cx.notify();
                return;
            }
        };
        // 初始目录给设置文件所在目录：用户下一次要找的就是这里。
        let directory = crate::paths::settings_file_path()
            .as_deref()
            .and_then(std::path::Path::parent)
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default();
        let picked = cx.prompt_for_new_path(&directory, Some(EXPORT_FILE_NAME));
        cx.spawn_in(window, async move |this, async_cx| {
            let path = match picked.await {
                Ok(Ok(Some(path))) => path,
                Ok(Ok(None)) => {
                    eprintln!("S1_KEYMAP_IO run=export state=cancelled");
                    return;
                }
                Ok(Err(error)) => {
                    eprintln!("S1_KEYMAP_IO run=export state=failed error={error}");
                    return;
                }
                Err(_) => {
                    eprintln!("S1_KEYMAP_IO run=export state=cancelled reason=channel-closed");
                    return;
                }
            };
            // 键位文件是几 KB，直接在前景任务里写完；与仓库里别的设置文件写入同型。
            let outcome = std::fs::write(&path, text);
            eprintln!(
                "S1_KEYMAP_IO run=export state={} path={}",
                if outcome.is_ok() { "written" } else { "failed" },
                path.display()
            );
            let _ = async_cx.update(move |_, cx| {
                let _ = this.update(cx, |this, cx| {
                    this.keymap_message = Some(match outcome {
                        Ok(()) => KeymapMessage {
                            severity: Severity::Success,
                            text: tr("lithe.settings.keyboard.keybindingsExported"),
                            skipped: 0,
                        },
                        Err(error) => KeymapMessage {
                            severity: Severity::Danger,
                            text: tr_args(
                                "lithe.settings.keyboard.exportFailed",
                                &[("error", &error.to_string())],
                            ),
                            skipped: 0,
                        },
                    });
                    cx.notify();
                });
            });
        })
        .detach();
    }

    /// 导入：读文件 → 解析 → 逐条写进覆盖。判定三档见 `keymap_io` 模块文档。
    fn keymap_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(tr("lithe.settings.keyboard.import")),
        });
        let store = self.store.clone();
        cx.spawn_in(window, async move |this, async_cx| {
            let path = match picked.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => path,
                    None => {
                        eprintln!("S1_KEYMAP_IO run=import state=cancelled");
                        return;
                    }
                },
                Ok(Ok(None)) => {
                    eprintln!("S1_KEYMAP_IO run=import state=cancelled");
                    return;
                }
                Ok(Err(error)) => {
                    eprintln!("S1_KEYMAP_IO run=import state=failed error={error}");
                    return;
                }
                Err(_) => {
                    eprintln!("S1_KEYMAP_IO run=import state=cancelled reason=channel-closed");
                    return;
                }
            };
            let outcome = std::fs::read_to_string(&path);
            let _ = async_cx.update(move |_, cx| {
                let _ = this.update(cx, |this, cx| {
                    let Ok(text) = outcome else {
                        let error = outcome.unwrap_err().to_string();
                        eprintln!("S1_KEYMAP_IO run=import state=read_failed error={error}");
                        this.keymap_message = Some(KeymapMessage {
                            severity: Severity::Danger,
                            text: tr_args(
                                "lithe.settings.keyboard.importFailed",
                                &[("error", &error)],
                            ),
                            skipped: 0,
                        });
                        cx.notify();
                        return;
                    };
                    match parse_import(&text) {
                        // 「格式无效」那一档也包含「文件里有条目却一条都导不进」——那对用户
                        // 就是「这个文件本应用不了」，比报「已导入 0 个」诚实。
                        ImportResult::Rejected => {
                            eprintln!("S1_KEYMAP_IO run=import state=rejected");
                            this.keymap_message = Some(KeymapMessage {
                                severity: Severity::Warning,
                                text: tr("lithe.settings.keyboard.invalidFile"),
                                skipped: 0,
                            });
                        }
                        ImportResult::Applied(plan) => {
                            let skipped = plan.skipped();
                            let mut imported = 0usize;
                            // 文件带预设就顺带切过去：覆盖是按预设分桶的，写进文件那一桶而
                            // 当前停在另一档，用户会看到「导入成功但页面没变」。
                            let preset =
                                plan.preset.unwrap_or_else(|| store.read(cx).keymap().preset);
                            if let Some(next) = plan.preset {
                                store.update(cx, |store, cx| store.set_keymap_preset(next, cx));
                            }
                            for entry in &plan.entries {
                                let Some(command) = entry.command else {
                                    continue;
                                };
                                imported += 1;
                                let keystrokes = entry.keystrokes.clone();
                                store.update(cx, |store, cx| {
                                    store.set_key_override(preset, command, keystrokes, cx);
                                });
                            }
                            eprintln!(
                                "S1_KEYMAP_IO run=import state=applied imported={imported} skipped={skipped} preset={}",
                                preset.id()
                            );
                            let count = imported.to_string();
                            let suffix = if plan.preset.is_some() {
                                tr("lithe.settings.keyboard.andPreset")
                            } else {
                                SharedString::from("")
                            };
                            this.keymap_message = Some(KeymapMessage {
                                // 有跳过就是「部分成功」：给警告档，别让人以为全都进去了。
                                severity: if skipped > 0 {
                                    Severity::Warning
                                } else {
                                    Severity::Success
                                },
                                text: tr_args(
                                    "lithe.settings.keyboard.keybindingsImported",
                                    &[("count", count.as_str()), ("preset", suffix.as_str())],
                                ),
                                skipped,
                            });
                        }
                    }
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn dropdown(
        &self,
        id: impl Into<ElementId>,
        current_label: SharedString,
        options: Vec<(SharedString, SharedString, bool)>,
        on_select: Box<dyn Fn(SharedString, &mut Window, &mut App) + 'static>,
    ) -> gpui::AnyElement {
        let on_select = std::rc::Rc::new(on_select);
        // "一组里选一个"：上游的 `PopupMenuItem::checked(..)` 在库侧对到
        // `MenuItem::radio(label, selected)`（单选行，带选中标记；`MenuItem::check` 是独立勾选项）。
        let mut menu = Menu::new();
        for (value, label, checked) in options {
            let on_select = on_select.clone();
            menu = menu.item(MenuItem::radio(label, checked).on_click(move |window, cx| {
                on_select(value.clone(), window, cx);
            }));
        }
        // 库 `DropdownMenu` 自己画触发按钮（label + 尾部折叠箭头）并贴在触发器下方开菜单。
        // 上游的定宽 `.w(ControlWidth::Default.length())`、`.justify_between()`、
        // `Anchor::TopRight` 定位与 `.scrollable(true)` 在库侧没有落点——几何与弹层定位归库自己，
        // 这几处差异登记在汇报的缺口清单里。
        DropdownMenu::new(id, current_label, menu)
            .variant(ButtonVariant::Outline)
            .into_any_element()
    }
}

impl Render for SettingsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_stack()
            .size_full()
            .bg(cx.theme().colors.bg)
            .text_color(cx.theme().colors.fg)
            .child(self.header(cx))
            .child(
                h_stack()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .child(self.nav(cx))
                    .child(self.content(cx)),
            )
            .child(self.footer(cx))
    }
}

const MODE_SYSTEM: &str = "system";
const MODE_LIGHT: &str = "light";
const MODE_DARK: &str = "dark";

fn theme_choices(cx: &App, applied: SharedString) -> Vec<(SharedString, SharedString, bool)> {
    let mut options = theme::theme_options(cx);
    if !options.iter().any(|(id, _)| id == applied.as_ref()) {
        options.insert(0, (applied.to_string(), theme::display_name_for(&applied)));
    }
    options
        .into_iter()
        .map(|(id, label)| {
            let checked = id == applied.as_ref();
            (SharedString::from(id), SharedString::from(label), checked)
        })
        .collect()
}

fn font_family_options(cx: &App, current: &str) -> Vec<(SharedString, SharedString, bool)> {
    let current = current.trim();
    let mut options = vec![(
        SharedString::from(""),
        tr("lithe.settings.gpui.valueNotOverridden"),
        current.is_empty(),
    )];

    let mut names: Vec<String> = theme::installed_font_names(cx).to_vec();
    names.sort();
    names.dedup();

    if !current.is_empty() && !names.iter().any(|name| name == current) {
        options.push((
            SharedString::from(current),
            SharedString::from(current),
            true,
        ));
    }
    for name in names {
        let checked = name == current;
        options.push((
            SharedString::from(name.clone()),
            SharedString::from(name),
            checked,
        ));
    }
    options
}

fn font_family_label(current: &str) -> SharedString {
    let current = current.trim();
    if current.is_empty() {
        tr("lithe.settings.gpui.valueNotOverridden")
    } else {
        SharedString::from(current.to_string())
    }
}

fn terminal_font_size_label(current: f64) -> SharedString {
    if current == TERMINAL_FONT_SIZE_UNSET {
        tr("lithe.settings.gpui.valueNotOverridden")
    } else {
        SharedString::from(format!("{} px", current as u32))
    }
}

fn terminal_font_size_options(current: f64) -> Vec<(SharedString, SharedString, bool)> {
    let mut options = vec![(
        SharedString::from((TERMINAL_FONT_SIZE_UNSET as u32).to_string()),
        tr("lithe.settings.gpui.valueNotOverridden"),
        current == TERMINAL_FONT_SIZE_UNSET,
    )];
    for size in (TERMINAL_FONT_SIZE_MIN as u32)..=(TERMINAL_FONT_SIZE_MAX as u32) {
        options.push((
            SharedString::from(size.to_string()),
            SharedString::from(format!("{size} px")),
            current == f64::from(size),
        ));
    }
    options
}

fn language_label(tag: &str) -> SharedString {
    match tag {
        "en-US" => tr("lithe.settings.gpui.languageEnglish"),
        _ => tr("lithe.settings.gpui.languageChinese"),
    }
}

fn mode_label(mode: AppearanceMode) -> SharedString {
    match mode {
        AppearanceMode::System => tr("lithe.settings.mac.followSystem"),
        AppearanceMode::Light => tr("lithe.settings.mac.light"),
        AppearanceMode::Dark => tr("lithe.settings.mac.dark"),
    }
}

// 上游这里还有 `format_ui_font_size` / `format_editor_font_size` 两个显示格式化函数。
// 字号输入现在交给库的 `NumberInput`，**显示文本由库自己按 `precision` 生成**，宿主不再有
// 格式化落点，所以两个函数随调用点一起删掉（连同只测它们的 `font_size_labels_are_compact`）。
// 参考实现的显示口径是"最多几位小数、去掉尾随零"（`windows/tauri/src/ui/number-input.tsx:71-89`
// 的 `format.maximumFractionDigits`），库是"固定几位小数"，这一处差异登记在汇报的缺口清单里。
// 库侧补齐后，这条显示契约应作为库测试回到 `lithe-ui`。

fn tab_size_label(size: u32) -> SharedString {
    SharedString::from(format!("{size} {}", tr("lithe.settings.mac.spaces")))
}

fn shell_label(id: &str) -> SharedString {
    match id {
        "powershell" => tr("lithe.settings.mac.shellPowerShell"),
        "cmd" => tr("lithe.settings.mac.shellCommandPrompt"),
        "wsl" => tr("lithe.settings.mac.shellWsl"),
        _ => {
            debug_assert_eq!(id, SHELL_SYSTEM_DEFAULT);
            tr("lithe.settings.mac.systemDefault")
        }
    }
}

fn git_scope_label(scope: IdentityScope) -> SharedString {
    match scope {
        IdentityScope::Local => tr("lithe.git.setup.local"),
        IdentityScope::Global => tr("lithe.git.setup.global"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    /// 默认档（Lithe）永远不报覆盖缺口。
    ///
    /// 它的预设表**按设计是空表**——它的键位就是默认值本身。若照「表里没有就算缺口」直算，
    /// 一进设置页就挂着「此预设不完整」，那是在撒谎。
    #[test]
    fn the_default_preset_never_reports_a_coverage_gap() {
        assert_eq!(
            preset_gap(KeymapPreset::Lithe),
            0,
            "Lithe 档的键位就是默认值本身，不该被判成不完整"
        );
    }

    /// 没有真源的预设档，缺口就是全部命令。
    ///
    /// 顺带把「Eclipse 档没有 Windows 侧真源」这个事实钉住：将来真源补齐、表被填上，这条
    /// 会失败，那就是提醒该改这条断言的时候了。
    #[test]
    fn a_preset_without_a_real_source_reports_every_bindable_command_missing() {
        assert!(
            preset_bindings(KeymapPreset::Eclipse).is_empty(),
            "ECLIPSE_BINDINGS 今天是空表：macOS 那两份 Eclipse 表是 ⌘ 方言，Windows 上没有对应真源"
        );

        // 注入一个只认两条的解析器，把「分母只数可绑的」这条语义钉死。
        // 刻意不写死 9 或 23：那依赖「到底哪几条有 gpui Action」这个 workbench 侧的知识，
        // 写在这里会变成第二份真相。
        crate::keymap::set_bindable_resolver(Some(std::rc::Rc::new(|command| {
            matches!(command, CommandId::SaveBuffer | CommandId::OpenQuickOpen)
        })));
        assert_eq!(
            preset_gap(KeymapPreset::Eclipse),
            2,
            "空表 ⇒ 每一条可绑的都算缺失"
        );

        // 没有解析器时一律判不可绑 ⇒ 缺口为 0（宁可不报，也不虚报）。
        crate::keymap::set_bindable_resolver(None);
        assert_eq!(preset_gap(KeymapPreset::Eclipse), 0);
    }

    // `font_size_labels_are_compact` 随 `format_ui_font_size` / `format_editor_font_size`
    // 一起删除：显示文本的格式化已归库的 `NumberInput`（见上方注释与汇报的缺口清单）。

    #[test]
    fn categories_match_the_windows_list_without_ai() {
        let ids: Vec<&str> = Category::ALL.iter().map(|category| category.id()).collect();
        assert_eq!(
            ids,
            [
                "general",
                "appearance",
                "project",
                "run",
                "editor",
                "keyboard",
                "terminal",
                "lsp",
                "git",
                "logs",
                "updates",
            ],
            "顺序与取值都要跟 settings-dialog.tsx:35-48 一致（AI 两个分类 + 本侧新增的 appearance 除外）"
        );
        assert_eq!(Category::DEFAULT, Category::General);
    }

    #[test]
    fn every_category_is_implemented_or_declares_a_prerequisite() {
        for category in Category::ALL {
            let implemented = Category::IMPLEMENTED.contains(&category);
            let prerequisite = category.prerequisite_key();
            assert_eq!(
                implemented,
                prerequisite.is_none(),
                "{:?} 的实现状态与前置条件登记不一致",
                category
            );
        }
        for category in Category::IMPLEMENTED {
            assert!(
                Category::ALL.contains(&category),
                "{category:?} 不在 ALL 里"
            );
        }
    }

    #[test]
    fn category_ids_are_probe_tokens() {
        assert_eq!(Category::General.id(), "general");
        assert_eq!(Category::Appearance.id(), "appearance");
        assert_eq!(Category::Editor.id(), "editor");
        assert_eq!(Category::Terminal.id(), "terminal");
        assert_eq!(Category::Lsp.id(), "lsp");
        assert_eq!(Category::Updates.id(), "updates");
    }

    #[test]
    fn rem_conversion_matches_the_spec_pixels() {
        assert_eq!(rem_px(px(16.), DIALOG_WIDTH), px(820.));
        assert_eq!(rem_px(px(16.), DIALOG_HEIGHT), px(620.));
        assert_eq!(rem_px(px(16.), NAV_WIDTH), px(190.));
        assert_eq!(rem_px(px(32.), NAV_WIDTH), px(380.));
    }

    #[test]
    fn git_page_degrades_to_the_empty_state_without_a_host() {
        crate::identity::set_git_identity_host(None);
        assert!(
            git_identity_page().is_none(),
            "没有宿主登记钩子时，Git 页必须走空态分支"
        );
        assert!(
            !Category::IMPLEMENTED.contains(&Category::Git),
            "Git 页是**条件实现**：宿主没登记钩子时它就是空态，不能记进 IMPLEMENTED"
        );
        assert_eq!(
            Category::Git.prerequisite_key(),
            Some("lithe.settings.gpui.prerequisiteGit")
        );
        assert_eq!(Category::Git.id(), "git");
        assert_eq!(Category::Git.label_key(), "lithe.settings.tabs.git");
    }

    #[test]
    fn git_buttons_follow_the_truth_source_gates() {
        let setup = IdentitySetup {
            is_repository: true,
            configured_name: Some("Lithe Dev".to_string()),
            effective_name: Some("Lithe Dev".to_string()),
            ..IdentitySetup::default()
        };
        assert!(git_save_enabled(
            Some(&setup),
            IdentityScope::Local,
            IdentityField::Name,
            "Other",
            false
        ));
        assert!(!git_save_enabled(
            Some(&setup),
            IdentityScope::Local,
            IdentityField::Name,
            "Lithe Dev",
            false
        ));
        assert!(!git_save_enabled(
            Some(&setup),
            IdentityScope::Local,
            IdentityField::Name,
            "  ",
            false
        ));
        assert!(!git_save_enabled(
            Some(&setup),
            IdentityScope::Local,
            IdentityField::Name,
            "Other",
            true
        ));
        assert!(!git_clear_enabled(Some(&setup), IdentityField::Name, true));
        assert!(!git_save_enabled(
            None,
            IdentityScope::Local,
            IdentityField::Name,
            "Other",
            false
        ));
        assert!(!git_clear_enabled(None, IdentityField::Name, false));
        assert!(git_clear_enabled(Some(&setup), IdentityField::Name, false));
        assert!(!git_clear_enabled(
            Some(&setup),
            IdentityField::Email,
            false
        ));
    }

    #[test]
    fn project_field_keys_follow_the_truth_source() {
        assert_eq!(ProjectField::ALL.len(), 3);
        assert_eq!(ProjectField::Jdk.label_key(), "lithe.run.jdkHome");
        assert_eq!(ProjectField::Maven.label_key(), "lithe.run.mavenExecutable");
        assert_eq!(ProjectField::MavenJdk.label_key(), "lithe.run.mavenJdkHome");
        assert_eq!(ProjectField::Jdk.hint_key(), "lithe.run.jdkHomeHint");
        assert_eq!(
            ProjectField::Maven.hint_key(),
            "lithe.settings.gpui.mavenExecutableHint"
        );
        assert_eq!(
            ProjectField::MavenJdk.hint_key(),
            "lithe.run.mavenJdkHomeHint"
        );
        let ids: Vec<&str> = ProjectField::ALL.iter().map(|field| field.id()).collect();
        assert_eq!(ids, ["jdk", "maven", "maven_jdk"]);
        let indices: Vec<usize> = ProjectField::ALL
            .iter()
            .map(|field| field.index())
            .collect();
        assert_eq!(indices, [0, 1, 2]);
    }

    #[test]
    fn only_the_maven_overrides_are_marked_as_not_effective() {
        assert_eq!(ProjectField::Jdk.pending_key(), None);
        assert_eq!(
            ProjectField::Maven.pending_key(),
            Some("lithe.settings.gpui.mavenOverrideNotEffective")
        );
        assert_eq!(
            ProjectField::MavenJdk.pending_key(),
            Some("lithe.settings.gpui.mavenOverrideNotEffective")
        );
    }

    #[test]
    fn project_page_is_implemented_not_an_empty_state() {
        assert!(Category::IMPLEMENTED.contains(&Category::Project));
        assert_eq!(Category::Project.prerequisite_key(), None);
        assert_eq!(
            Category::Project.label_key(),
            "lithe.settings.project.title"
        );
        assert_eq!(Category::Project.id(), "project");
    }

    #[test]
    fn maven_config_fields_follow_the_truth_source() {
        assert_eq!(MavenConfigField::ALL.len(), 2);
        assert_eq!(
            MavenConfigField::SettingsXml.label().as_ref(),
            "settings.xml"
        );
        assert_eq!(
            MavenConfigField::LocalRepository.label(),
            tr("lithe.maven.localRepository")
        );
        assert_eq!(
            MavenConfigField::SettingsXml.hint_key(),
            "lithe.settings.gpui.mavenSettingsHint"
        );
        assert_eq!(
            MavenConfigField::LocalRepository.hint_key(),
            "lithe.settings.gpui.mavenLocalRepositoryHint"
        );
        let ids: Vec<&str> = MavenConfigField::ALL
            .iter()
            .map(|field| field.id())
            .collect();
        assert_eq!(ids, ["settings_xml", "local_repository"]);
        let indices: Vec<usize> = MavenConfigField::ALL
            .iter()
            .map(|field| field.index())
            .collect();
        assert_eq!(indices, [0, 1]);
        assert!(MavenConfigField::SettingsXml.picks_file());
        assert!(!MavenConfigField::LocalRepository.picks_file());
    }

    #[test]
    fn maven_config_lines_report_who_is_effective() {
        use crate::project::{LocalRepository, LocalRepositorySource, MavenConfiguration};

        let environment = |config: MavenConfiguration, overrides: Overrides| ProjectEnvironment {
            overrides,
            jdk: crate::project::JdkDiscovery::default(),
            maven: crate::project::MavenDiscovery::default(),
            maven_config: config,
            maven_jdk: crate::project::JdkDiscovery::default(),
        };

        for field in MavenConfigField::ALL {
            let line = maven_config_effective(field, None);
            assert_eq!(line.text, tr("lithe.toolchain.detecting"));
            assert!(!line.is_error);
        }

        let auto = environment(
            MavenConfiguration {
                user_settings: Some(r"C:\Users\x\.m2\settings.xml".to_string()),
                installation_settings: Some(r"C:\maven\conf\settings.xml".to_string()),
                local_repository: Some(LocalRepository {
                    path: r"D:\repo".to_string(),
                    source: LocalRepositorySource::SettingsXml,
                }),
                ..MavenConfiguration::default()
            },
            Overrides::default(),
        );
        let line = maven_config_effective(MavenConfigField::SettingsXml, Some(&auto));
        assert_eq!(line.text.as_ref(), r"C:\Users\x\.m2\settings.xml");
        assert!(!line.is_error);
        assert_eq!(
            line.detail.as_deref(),
            Some(r"C:\maven\conf\settings.xml"),
            "安装级那份只在与生效值不同时出现"
        );
        let line = maven_config_effective(MavenConfigField::LocalRepository, Some(&auto));
        assert_eq!(line.text.as_ref(), r"D:\repo");
        assert_eq!(
            line.detail,
            Some(tr(LocalRepositorySource::SettingsXml.label_key()))
        );

        let picked = environment(
            MavenConfiguration {
                override_settings: Some(r"D:\ci\settings.xml".to_string()),
                override_settings_missing: false,
                local_repository: Some(LocalRepository {
                    path: r"D:\ci-repo".to_string(),
                    source: LocalRepositorySource::SettingsXml,
                }),
                ..MavenConfiguration::default()
            },
            Overrides::default(),
        );
        let line = maven_config_effective(MavenConfigField::SettingsXml, Some(&picked));
        assert_eq!(line.text.as_ref(), r"D:\ci\settings.xml");
        assert!(!line.is_error);
        assert_eq!(
            line.detail,
            Some(tr("lithe.settings.gpui.mavenSettingsEffective"))
        );

        let missing = environment(
            MavenConfiguration {
                override_settings: Some(r"D:\nope\settings.xml".to_string()),
                override_settings_missing: true,
                ..MavenConfiguration::default()
            },
            Overrides::default(),
        );
        let line = maven_config_effective(MavenConfigField::SettingsXml, Some(&missing));
        assert_eq!(line.text.as_ref(), r"D:\nope\settings.xml");
        assert!(line.is_error);
        assert_eq!(
            line.detail,
            Some(tr("lithe.settings.gpui.overridePathMissing"))
        );

        let none = environment(MavenConfiguration::default(), Overrides::default());
        let line = maven_config_effective(MavenConfigField::SettingsXml, Some(&none));
        assert_eq!(line.text, tr("lithe.settings.gpui.mavenSettingsMissing"));
        assert!(line.is_error);

        let overridden = environment(
            MavenConfiguration {
                local_repository: Some(LocalRepository {
                    path: r"D:\repo".to_string(),
                    source: LocalRepositorySource::MavenDefault,
                }),
                ..MavenConfiguration::default()
            },
            Overrides {
                maven_local_repository: " D:\\my-repo ".to_string(),
                ..Overrides::default()
            },
        );
        let line = maven_config_effective(MavenConfigField::LocalRepository, Some(&overridden));
        assert_eq!(line.text.as_ref(), "D:\\my-repo");
        assert!(!line.is_error);
        assert_eq!(
            line.detail,
            Some(tr("lithe.settings.gpui.mavenOverrideNotEffective"))
        );

        let unknown = environment(MavenConfiguration::default(), Overrides::default());
        let line = maven_config_effective(MavenConfigField::LocalRepository, Some(&unknown));
        assert_eq!(
            line.text,
            tr("lithe.settings.gpui.mavenLocalRepositoryUnknown")
        );
        assert!(line.is_error);
    }

    #[test]
    fn run_page_is_implemented_not_an_empty_state() {
        assert!(Category::IMPLEMENTED.contains(&Category::Run));
        assert_eq!(Category::Run.prerequisite_key(), None);
        assert_eq!(Category::Run.label_key(), "lithe.settings.run.title");
        assert_eq!(Category::Run.id(), "run");
    }

    /// 编辑器页的三个视图开关（缩略图 / 行号 / 空白字符）用真源里已有的文案键，不是新造的。
    ///
    /// 这一条防的是「键写错了没人发现」：`tr()` 缺键时会把键名原样回显，页面上就会出现
    /// `lithe.settings.editor.showMinimap` 这样的英文串，而单测不读文案是看不见的。
    #[test]
    fn the_editor_view_toggles_use_keys_that_exist_in_both_locales() {
        fn locale(name: &str) -> String {
            let path = format!("{}/../shared/locales/{}", env!("CARGO_MANIFEST_DIR"), name);
            std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("读不了 i18n 文件 {path}：{error}"))
        }
        fn has_key(source: &str, key: &str) -> bool {
            source
                .lines()
                .any(|line| line.trim_end() == format!("  {key}:").as_str())
        }
        let zh = locale("lithe.zh-CN.yml");
        let en = locale("lithe.en.yml");
        for key in [
            "settings.editor.showMinimap",
            "settings.editor.showMinimapDescription",
            "settings.editor.lineNumbers",
            "settings.editor.lineNumbersDescription",
            "settings.editor.renderWhitespace",
            "settings.editor.renderWhitespaceDescription",
        ] {
            assert!(has_key(&zh, key), "中文档案里没有 {key}");
            assert!(has_key(&en, key), "英文档案里没有 {key}");
        }
    }
    /// 快捷键页 2026-10-15 转正：它**不再**是「未接入」的空态，也不该再挂着前置条件文案。
    ///
    /// 这一条钉两件事：`IMPLEMENTED` 名单里有它（否则设置页会把整页换成 `EmptyState`），
    /// 以及 `prerequisite_key()` 已经撤掉——留着它就会写出「前置条件：…只有固定绑定的少数
    /// 快捷键」，而 `keyboard_page` 早就是完整页了。
    #[test]
    fn the_keyboard_page_is_implemented_not_an_empty_state() {
        assert!(Category::IMPLEMENTED.contains(&Category::Keyboard));
        assert_eq!(Category::Keyboard.prerequisite_key(), None);
        assert_eq!(Category::Keyboard.label_key(), "lithe.settings.tabs.keyboard");
        assert_eq!(Category::Keyboard.id(), "keyboard");
    }

    #[test]
    fn only_the_lazy_pages_load_on_open() {
        assert!(page_loads_on_open(Category::Project));
        assert!(page_loads_on_open(Category::Run));
        assert!(page_loads_on_open(Category::Lsp));
        for category in Category::ALL {
            if matches!(category, Category::Project | Category::Run | Category::Lsp) {
                continue;
            }
            assert!(
                !page_loads_on_open(category),
                "{category:?} 不该在打开时读盘 / 起探测子进程"
            );
        }
    }

    #[test]
    fn lsp_page_is_implemented_not_an_empty_state() {
        assert!(Category::IMPLEMENTED.contains(&Category::Lsp));
        assert_eq!(Category::Lsp.prerequisite_key(), None);
        assert_eq!(Category::Lsp.label_key(), "lithe.settings.tabs.lsp");
        assert_eq!(Category::Lsp.id(), "lsp");
    }

    #[test]
    fn run_page_never_confuses_a_failure_with_an_empty_result() {
        let mut state = RunPageState::new();
        assert_eq!(state.mode(), RunPageMode::Loading);

        state.requested = true;
        assert_eq!(state.mode(), RunPageMode::NoProject);

        state.root = Some(PathBuf::from(r"D:\proj"));
        state.error = Some(SharedString::from("识别运行配置失败"));
        assert_eq!(state.mode(), RunPageMode::Failed);

        assert_ne!(state.mode(), RunPageMode::Ready);

        state.error = None;
        state.view = Some(RunProjectView {
            configurations: Vec::new(),
            entry_count: 0,
            java_entrypoints_origin: None,
        });
        assert_eq!(state.mode(), RunPageMode::Ready);

        state.busy = true;
        assert_eq!(state.mode(), RunPageMode::Ready);
    }

    #[test]
    fn effective_lines_follow_the_truth_source_wording() {
        fn resolved(
            mode: crate::project::ToolMode,
            version: Option<&str>,
            source: Option<ToolSource>,
        ) -> EffectiveToolchain {
            EffectiveToolchain::Resolved {
                mode,
                path: r"D:\ProgramData\java\openjdk-21".to_string(),
                version: version.map(str::to_string),
                source,
            }
        }

        let automatic = resolved(
            crate::project::ToolMode::Automatic,
            Some("21.0.2"),
            Some(ToolSource::JavaHome),
        );
        let line = describe_effective(ToolKind::Jdk, &automatic);
        assert!(!line.is_error, "自动选中的值是正常态");
        assert!(line.text.contains(" → "), "{}", line.text);
        assert!(line.text.contains(r"D:\ProgramData\java\openjdk-21"));
        assert!(line.text.contains("21.0.2"), "{}", line.text);
        assert!(
            line.text
                .contains(source_label(ToolSource::JavaHome).as_ref()),
            "自动选中必须报来源：{}",
            line.text
        );
        assert!(line.detail.is_none());

        let configured = resolved(crate::project::ToolMode::Configured, Some("21.0.2"), None);
        let line = describe_effective(ToolKind::Jdk, &configured);
        assert!(!line.is_error);
        assert!(
            !line
                .text
                .contains(source_label(ToolSource::JavaHome).as_ref()),
            "覆盖值不该再拼来源：{}",
            line.text
        );
        assert!(!line.text.contains(source_label(ToolSource::Path).as_ref()));

        let line = describe_effective(
            ToolKind::Jdk,
            &resolved(
                crate::project::ToolMode::Automatic,
                None,
                Some(ToolSource::Path),
            ),
        );
        assert!(
            line.text.contains(ToolKind::Jdk.bare_name()),
            "{}",
            line.text
        );
        let line = describe_effective(
            ToolKind::Maven,
            &EffectiveToolchain::Resolved {
                mode: crate::project::ToolMode::Automatic,
                path: r"D:\tools\apache-maven-3.9.9\bin\mvn.cmd".to_string(),
                version: None,
                source: Some(ToolSource::MavenHome),
            },
        );
        assert!(
            line.text.contains(ToolKind::Maven.bare_name()),
            "{}",
            line.text
        );
        assert!(
            line.text
                .contains(source_label(ToolSource::MavenHome).as_ref()),
            "Maven 的来源要报出 MAVEN_HOME：{}",
            line.text
        );

        let unusable = EffectiveToolchain::Unusable {
            path: r"D:\nope".to_string(),
            reason: "找不到 java.exe".to_string(),
            language_service_rejection: false,
        };
        let line = describe_effective(ToolKind::Jdk, &unusable);
        assert!(line.is_error);
        assert!(line.text.contains("找不到 java.exe"), "{}", line.text);
        assert_eq!(line.detail.as_deref(), Some(r"D:\nope"));

        let below_gate = EffectiveToolchain::Unusable {
            path: r"D:\ProgramData\java\openjdk-17".to_string(),
            reason: crate::project::below_requirement_reason("17.0.16"),
            language_service_rejection: true,
        };
        let line = describe_effective(ToolKind::Jdk, &below_gate);
        assert!(line.is_error);
        assert!(line.text.contains("17.0.16"), "{}", line.text);
        let detail = line.detail.as_deref().unwrap_or_default();
        assert!(detail.contains("openjdk-17"), "{detail}");
        assert!(
            detail.contains(&crate::project::MINIMUM_JAVA_MAJOR.to_string()),
            "详情行必须报出闸门数值：{detail}"
        );
        assert!(
            detail.contains("JDT LS"),
            "详情行必须点明是哪个语言服务：{detail}"
        );
        assert!(
            detail.contains("language service") || detail.contains("语言服务"),
            "详情行必须说清后果落在语言服务上：{detail}"
        );

        let not_found = EffectiveToolchain::NotFound {
            reason: "JAVA_HOME=未设置；已排除：C:\\jdk8（版本 1.8.0_402）".to_string(),
        };
        let line = describe_effective(ToolKind::Jdk, &not_found);
        assert!(line.is_error);
        assert_eq!(line.text, tr(ToolKind::Jdk.not_found_key()));
        let detail = line.detail.expect("没找到时必须给出已排除清单");
        assert!(detail.contains("已排除"), "{detail}");
    }

    /// 「Java 版本」下拉：第一项永远是「自动检测」，值取 JDK 主目录，认不出主目录的候选不列。
    #[test]
    fn java_version_options_offer_automatic_first_and_a_home_to_write_back() {
        let jdk = |home: &str, version: &str| DetectedJdk {
            home: home.to_string(),
            executable: format!("{home}/bin/java.exe"),
            version: version.to_string(),
            meets_language_service: true,
            source: ToolSource::JavaHome,
        };
        let auto = SharedString::from("自动检测");

        // 没配过：只有「自动检测」，且它被选中。
        let options = java_version_options(&[], "", auto.clone());
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].0, SharedString::from(""));
        assert!(options[0].2, "空值时选中的必须是自动检测");
        assert_eq!(options[0].1, auto);

        let candidates = vec![
            jdk(r"D:\java\jdk-21", "21.0.2"),
            jdk(r"D:\java\jdk-17", "17.0.9"),
        ];

        // 配了一个探测到的 JDK：选中它，写回去的值是主目录而不是 java.exe。
        let options = java_version_options(&candidates, r"D:\java\jdk-17", auto.clone());
        assert_eq!(options.len(), 3);
        assert_eq!(options[0].1, auto);
        assert!(!options[0].2);
        assert_eq!(options[1].0, SharedString::from(r"D:\java\jdk-21"));
        assert_eq!(options[1].1, SharedString::from(r"D:\java\jdk-21 (21.0.2)"));
        assert!(options[2].2, "配过的那个候选要选中");
        assert_eq!(options[2].0, SharedString::from(r"D:\java\jdk-17"));

        // 配了一个候选里没有的路径：插在「自动检测」后面，照原样显示，别让它消失。
        let options = java_version_options(&candidates, r"E:\my-jdk", auto.clone());
        assert_eq!(options.len(), 4);
        assert!(options[1].2);
        assert_eq!(options[1].1, SharedString::from(r"E:\my-jdk"));

        // 只探测到 java.exe、认不出主目录的候选不列：写进 javaHomePath 只会拼出不存在的 bin/java。
        let options = java_version_options(
            &[DetectedJdk {
                home: String::new(),
                ..jdk(r"D:\x", "21")
            }],
            "",
            auto,
        );
        assert_eq!(options.len(), 1, "没有主目录的候选不列进来");
    }

    #[test]
    fn detected_installations_omit_a_missing_version() {
        let jdk = DetectedJdk {
            home: r"D:\ProgramData\java\openjdk-21".to_string(),
            executable: r"D:\ProgramData\java\openjdk-21\bin\java.exe".to_string(),
            version: "21.0.2".to_string(),
            meets_language_service: true,
            source: ToolSource::JavaHome,
        };
        assert_eq!(jdk_entry(&jdk), r"D:\ProgramData\java\openjdk-21 (21.0.2)");
        assert_eq!(
            jdk_entry(&DetectedJdk {
                home: String::new(),
                ..jdk.clone()
            }),
            r"D:\ProgramData\java\openjdk-21\bin\java.exe (21.0.2)"
        );
        assert_eq!(
            jdk_entry(&DetectedJdk {
                version: String::new(),
                ..jdk.clone()
            }),
            r"D:\ProgramData\java\openjdk-21"
        );

        let maven = DetectedMaven {
            executable: r"D:\tools\apache-maven-3.9.9\bin\mvn.cmd".to_string(),
            version: "3.9.9".to_string(),
            home: Some(r"D:\tools\apache-maven-3.9.9".to_string()),
            java_runtime: Some("21.0.2".to_string()),
            source: ToolSource::MavenHome,
        };
        assert_eq!(
            maven_entry(&maven),
            r"D:\tools\apache-maven-3.9.9\bin\mvn.cmd (3.9.9)"
        );
        assert_eq!(
            maven_entry(&DetectedMaven {
                version: String::new(),
                ..maven
            }),
            r"D:\tools\apache-maven-3.9.9\bin\mvn.cmd"
        );

        assert!(detected_installations(Vec::new()).is_none());
    }
}
