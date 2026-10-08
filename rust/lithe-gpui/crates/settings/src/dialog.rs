use std::path::PathBuf;

use gpui_kit::assets::IconName;
use gpui_kit::base::{Selectable as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::input::{Input, InputEvent, InputState, NumberInput};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, Sizable as _};
use gpui::{App, AppContext as _, Context, ElementId, Entity, FontWeight, InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div, rems};
use gpui::{Anchor, PathPromptOptions, prelude::FluentBuilder as _};

use lithe_gpui_shared::icons::{idea, idea_icon_image, idea_icon_mono};
use lithe_gpui_shared::workspace_config::{
    OverrideOrigin, OverrideOrigins, ToolchainPaths, load_local_toolchain, save_local_toolchain,
};
use lithe_gpui_shared::{tr, tr_args};
use lithe_ui::overlays::Dialog;
use lithe_ui::shell::{WindowRootExt as _, dismiss_hosted};

use crate::identity::{
    GitIdentityPage, IdentityField, IdentityScope, IdentitySetup, git_identity_page,
    host_workspace_root, identity_value_is_valid, identity_value_rejection,
};
use crate::project::{
    DetectedJdk, DetectedMaven, EffectiveToolchain, MINIMUM_JAVA_MAJOR, Overrides,
    ProjectEnvironment, ToolSource, discover, resolve_overrides,
};
use crate::row::{
    ControlWidth, RowActivation, appearance_source_note, page_stack, page_title, settings_group,
    settings_row, settings_row_with_note,
};
use crate::run::{RunConfigurationView, RunProjectView};
use crate::schema::{
    DISPLAY_LANGUAGES, EDITOR_FONT_SIZE_DEFAULT, EDITOR_FONT_SIZE_MAX, EDITOR_FONT_SIZE_MIN,
    SHELL_SYSTEM_DEFAULT, Settings, TAB_SIZES, TERMINAL_FONT_SIZE_MAX, TERMINAL_FONT_SIZE_MIN,
    TERMINAL_FONT_SIZE_UNSET, TERMINAL_SHELL_IDS, UI_FONT_SIZE_DEFAULT, UI_FONT_SIZE_MAX,
    UI_FONT_SIZE_MIN, UI_FONT_SIZE_STEP,
};
use crate::store::{AppearanceMode, SettingsStore, store};
use crate::theme;
use crate::workspace::{AppearanceKey, AppearanceSource};

pub fn install_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-,", OpenSettings, None)]);
    cx.on_action(|_: &OpenSettings, cx: &mut App| {
        let Some(window_handle) = cx.active_window() else {
            return;
        };
        let _ = window_handle.update(cx, |_, window, cx| open_settings_dialog(window, cx));
    });
}

gpui_kit::actions!(lithe_settings, [OpenSettings]);

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
                .min((viewport.height - margin * 2.).max(gpui_kit::Pixels::ZERO));
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

fn rem_length(spec_px: f32) -> gpui_kit::Length {
    rems(spec_px / 16.).into()
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

    pub const IMPLEMENTED: [Category; 7] = [
        Category::General,
        Category::Appearance,
        Category::Project,
        Category::Run,
        Category::Editor,
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
            | Self::Terminal
            | Self::Lsp => None,
            Self::Keyboard => Some("lithe.settings.gpui.prerequisiteKeyboard"),
            Self::Git => Some("lithe.settings.gpui.prerequisiteGit"),
            Self::Logs => Some("lithe.settings.gpui.prerequisiteLogs"),
            Self::Updates => Some("lithe.settings.gpui.prerequisiteUpdates"),
        }
    }

    fn icon(self, is_dark: bool) -> gpui_kit::component::Icon {
        match self {
            Self::General => idea_icon_mono(&idea::GEAR_ICON, is_dark),
            Self::Appearance => idea_icon_mono(&idea::PALETTE_ICON, is_dark),
            Self::Editor => Icon::new(IconName::Code),
            Self::Terminal => Icon::new(IconName::SquareTerminal),
            Self::Project => Icon::new(IconName::FolderCog),
            Self::Run => idea_icon_mono(&idea::PLAY_ICON, is_dark),
            Self::Keyboard => idea_icon_mono(&idea::KEYBOARD_ICON, is_dark),
            Self::Lsp => Icon::new(IconName::Braces),
            Self::Git => idea_icon_mono(&idea::GIT_BRANCH_ICON, is_dark),
            Self::Logs => Icon::new(IconName::ScrollText),
            Self::Updates => idea_icon_mono(&idea::DOWNLOAD_ICON, is_dark),
        }
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
    input.read(cx).value().trim().to_string()
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

fn run_page_hint(text: SharedString, cx: &Context<SettingsDialog>) -> gpui_kit::AnyElement {
    div()
        .w_full()
        .text_sm()
        .text_color(cx.theme().foreground)
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
    font_size_input: Entity<InputState>,
    editor_font_size_input: Entity<InputState>,
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
    _subscriptions: Vec<Subscription>,
}

impl SettingsDialog {
    fn new(
        store: Entity<SettingsStore>,
        category: Category,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let value = store.read(cx).settings().ui_font_size;
        let editor_value = store.read(cx).settings().font_size;
        let saved = store.read(cx).settings().clone();
        let font_size_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(format_ui_font_size(value))
                .step(UI_FONT_SIZE_STEP)
                .min(UI_FONT_SIZE_MIN)
                .max(UI_FONT_SIZE_MAX)
        });
        let editor_font_size_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(format_editor_font_size(editor_value))
                .step(1.)
                .min(EDITOR_FONT_SIZE_MIN)
                .max(EDITOR_FONT_SIZE_MAX)
        });

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
        let java_home_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(project_overrides.java_home.clone())
        });
        let maven_executable_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(project_overrides.maven_executable.clone())
        });
        let maven_java_home_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(project_overrides.maven_java_home.clone())
        });
        let maven_settings_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(project_overrides.maven_settings.clone())
        });
        let maven_local_repository_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(project_overrides.maven_local_repository.clone())
        });

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&store, |_, _, cx| cx.notify()));
        subscriptions.push(cx.subscribe_in(
            &font_size_input,
            window,
            |this: &mut Self, input, event: &InputEvent, _window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let Ok(parsed) = input.read(cx).value().parse::<f64>() else {
                    return;
                };
                if !(UI_FONT_SIZE_MIN..=UI_FONT_SIZE_MAX).contains(&parsed) {
                    return;
                }
                let store = this.store.clone();
                store.update(cx, |store, cx| store.set_ui_font_size(parsed, cx));
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &editor_font_size_input,
            window,
            |this: &mut Self, input, event: &InputEvent, _window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let Ok(parsed) = input.read(cx).value().parse::<f64>() else {
                    return;
                };
                if !(EDITOR_FONT_SIZE_MIN..=EDITOR_FONT_SIZE_MAX).contains(&parsed) {
                    return;
                }
                let store = this.store.clone();
                store.update(cx, |store, cx| store.set_editor_font_size(parsed, cx));
            },
        ));
        for input in [&git_name_input, &git_email_input] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |_this: &mut Self, _input, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
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
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                },
            ));
        }

        Self {
            store,
            category,
            font_size_input,
            editor_font_size_input,
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
            _subscriptions: subscriptions,
        }
    }

    fn header(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .h_11()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .text_color(cx.theme().foreground)
            .child(idea_icon_image(&idea::GEAR_ICON, cx, rems(1.)))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(tr("lithe.workbench.settings")),
            )
            .child(div().flex_1())
            .child(
                Button::new("settings-close")
                    .small()
                    .ghost()
                    .child(idea_icon_image(&idea::X_ICON, cx, rems(1.)))
                    .px_0()
                    .accessibility_label(tr("lithe.tabs.close"))
                    .tooltip(tr("lithe.tabs.close"))
                    .on_click(|_, window, cx| {
                        window.close_dialog(cx);
                    }),
            )
    }

    fn nav(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .id("settings-nav")
            .h_full()
            .w(rem_length(NAV_WIDTH))
            .flex_shrink_0()
            .gap_0p5()
            .p_2()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .aria_label(tr("lithe.settings.mac.categories"))
            .children(Category::ALL.iter().enumerate().map(|(index, category)| {
                let selected = *category == self.category;
                Button::new(("settings-category", index))
                    .ghost()
                    .selected(selected)
                    .w_full()
                    .child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .items_center()
                            .gap_2p5()
                            .child(category.icon(cx.theme().is_dark()).size_4())
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .when(selected, |this| this.font_weight(FontWeight::MEDIUM))
                                    .child(tr(category.label_key())),
                            ),
                    )
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
            .bg(cx.theme().background)
            .overflow_y_scrollbar()
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
                        Category::Keyboard | Category::Logs | Category::Updates => {
                            self.empty_page(cx)
                        }
                    }),
            )
    }

    fn general_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
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

    fn appearance_note(
        &self,
        key: AppearanceKey,
        cx: &Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        let source = self.store.read(cx).appearance_source(key);
        self.appearance_note_for(source, &[key], cx)
    }

    fn appearance_note_for(
        &self,
        source: AppearanceSource,
        keys: &[AppearanceKey],
        cx: &Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        if !source.is_workspace_override() {
            return None;
        }
        let store = self.store.clone();
        let keys = keys.to_vec();
        let id = SharedString::from(format!("settings-appearance-revert-{}", keys[0].as_str()));
        let revert = Button::new(id)
            .small()
            .ghost()
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

    fn appearance_page(
        &self,
        settings: &Settings,
        cx: &Context<Self>,
    ) -> Vec<gpui_kit::AnyElement> {
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
                        NumberInput::new(&self.font_size_input)
                            .w(ControlWidth::Number.length())
                            .into_any_element(),
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
                            .small()
                            .checked(settings.show_status_bar)
                            .on_change(move |checked, _, cx| {
                                let checked = *checked;
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

    fn editor_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
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
                        NumberInput::new(&self.editor_font_size_input)
                            .w(ControlWidth::Number.length())
                            .into_any_element(),
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
                                .small()
                                .checked(settings.word_wrap)
                                .on_change(move |checked, _, cx| {
                                    let checked = *checked;
                                    store.update(cx, |store, cx| store.set_word_wrap(checked, cx));
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
                            .small()
                            .checked(settings.auto_save)
                            .on_change(move |checked, _, cx| {
                                let checked = *checked;
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

    fn terminal_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
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
        self.git_input(field).read(cx).value()
    }

    fn git_fill_input(
        &self,
        field: IdentityField,
        setup: &IdentitySetup,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = setup.configured(field).unwrap_or("").to_string();
        self.git_input(field).update(cx, |input, cx| {
            input.set_value(SharedString::from(text), window, cx);
        });
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
            move |setup, window, cx| {
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
                            this.git_fill_input(IdentityField::Name, &setup, window, cx);
                            this.git_fill_input(IdentityField::Email, &setup, window, cx);
                        }
                        None => {
                            this.git_diagnose(&format!(
                                "run=load result=failed scope={}",
                                scope.id()
                            ));
                            this.git.error = Some(tr("lithe.git.setup.readFailed"));
                            this.git_fill_input(
                                IdentityField::Name,
                                &IdentitySetup::default(),
                                window,
                                cx,
                            );
                            this.git_fill_input(
                                IdentityField::Email,
                                &IdentitySetup::default(),
                                window,
                                cx,
                            );
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
            move |setup, window, cx| {
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
                            this.git_fill_input(field, &setup, window, cx);
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

    fn git_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
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

        let mut rows: Vec<gpui_kit::AnyElement> = Vec::new();

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
                .text_color(cx.theme().muted_foreground)
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
                    .text_color(cx.theme().foreground)
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
                .text_color(cx.theme().muted_foreground)
                .child(tr("lithe.git.setup.separateSave"))
                .into_any_element(),
        );
        rows.push(
            h_flex()
                .w_full()
                .items_center()
                .gap_3()
                .child(
                    Button::new("settings-git-reload")
                        .small()
                        .ghost()
                        .label(tr("lithe.git.setup.reload"))
                        .disabled(self.git.busy)
                        .on_click(cx.listener(|this, _, window, cx| this.git_load(window, cx))),
                )
                .when(self.git.busy, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(tr("lithe.git.setup.loading")),
                    )
                })
                .when_some(self.git.error.clone(), |this, error| {
                    this.child(div().text_xs().text_color(cx.theme().danger).child(error))
                })
                .when(self.git.saved.is_some(), |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
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
                            .text_color(cx.theme().muted_foreground)
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
                            .small()
                            .checked(settings.confirm_before_discard)
                            .on_change(move |checked, _, cx| {
                                let checked = *checked;
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

    fn git_identity_row(&self, field: IdentityField, cx: &Context<Self>) -> gpui_kit::AnyElement {
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

        v_flex()
            .id(("settings-git-field", field.index()))
            .w_full()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(tr(field.label_key())),
            )
            .child(
                h_flex()
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
                            .small()
                            .label(tr("lithe.git.setup.save"))
                            .disabled(!save_enabled)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.git_save(field, false, window, cx)
                            })),
                    )
                    .child(
                        Button::new(("settings-git-clear", field.index()))
                            .small()
                            .ghost()
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
                    .text_color(cx.theme().muted_foreground)
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
                        .text_color(cx.theme().muted_foreground)
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
            let _ = async_cx.update(move |window, cx| {
                let _ = this.update(cx, |this, cx| {
                    input.update(cx, |input, cx| {
                        input.set_value(SharedString::from(value), window, cx);
                    });
                    this.project.saved = false;
                    cx.notify();
                });
            });
        })
        .detach();
    }

    fn project_clear(&mut self, field: ProjectField, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.project_field_input(field).clone();
        input.update(cx, |input, cx| {
            input.set_value(SharedString::from(""), window, cx);
        });
        self.project.saved = false;
        self.project_diagnose(&format!("run=clear field={}", field.id()));
        cx.notify();
    }

    fn project_clear_maven_config(
        &mut self,
        field: MavenConfigField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = self.maven_config_input(field).clone();
        input.update(cx, |input, cx| {
            input.set_value(SharedString::from(""), window, cx);
        });
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

    fn project_page(&self, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let environment = self.project.environment.as_ref();
        let detecting = || EffectiveLine {
            text: tr("lithe.toolchain.detecting"),
            is_error: false,
            detail: None,
        };

        let mut toolchain_rows: Vec<gpui_kit::AnyElement> = Vec::new();

        let scope_key = if self.project.workspace_root.is_some() {
            "lithe.settings.gpui.projectScopeProject"
        } else {
            "lithe.settings.gpui.projectScopeGlobal"
        };
        toolchain_rows.push(
            div()
                .w_full()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(tr(scope_key))
                .into_any_element(),
        );

        if environment.is_some_and(ProjectEnvironment::has_no_toolchain) {
            toolchain_rows.push(
                div()
                    .w_full()
                    .text_sm()
                    .text_color(cx.theme().danger)
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

        let maven_rows: Vec<gpui_kit::AnyElement> = MavenConfigField::ALL
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
    ) -> gpui_kit::AnyElement {
        v_flex()
            .id(("settings-project-row", field.index()))
            .w_full()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(tr(field.label_key())),
            )
            .child(
                h_flex()
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
                            .small()
                            .ghost()
                            .label(tr("lithe.settings.gpui.pickPath"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.project_pick_toolchain(field, window, cx)
                            })),
                    )
                    .child(
                        Button::new(("settings-project-clear", field.index()))
                            .small()
                            .ghost()
                            .label(tr("lithe.ui.clear"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.project_clear(field, window, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr(field.hint_key())),
            )
            .children(
                override_origin_key(field.origin(&self.project.origins)).map(|key| {
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr(key))
                }),
            )
            .children(field.pending_key().map(|key| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().warning)
                    .child(tr(key))
            }))
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(if effective.is_error {
                        cx.theme().danger
                    } else {
                        cx.theme().foreground
                    })
                    .child(effective.text),
            )
            .children(effective.detail.map(|detail| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(detail)
            }))
            .children(detected.map(|detected| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
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
    ) -> gpui_kit::AnyElement {
        v_flex()
            .id(id)
            .w_full()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(label),
            )
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(if is_error {
                        cx.theme().danger
                    } else {
                        cx.theme().foreground
                    })
                    .child(value),
            )
            .children(note.map(|note| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(note)
            }))
            .into_any_element()
    }

    fn project_maven_config_row(
        &self,
        field: MavenConfigField,
        cx: &Context<Self>,
    ) -> gpui_kit::AnyElement {
        let effective = maven_config_effective(field, self.project.environment.as_ref());
        v_flex()
            .id(("settings-maven-config-row", field.index()))
            .w_full()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(field.label()),
            )
            .child(
                h_flex()
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
                            .small()
                            .ghost()
                            .label(tr("lithe.settings.gpui.pickPath"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.project_pick_maven_config(field, window, cx)
                            })),
                    )
                    .child(
                        Button::new(("settings-maven-clear", field.index()))
                            .small()
                            .ghost()
                            .label(tr("lithe.ui.clear"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.project_clear_maven_config(field, window, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr(field.hint_key())),
            )
            .children(
                override_origin_key(field.origin(&self.project.origins)).map(|key| {
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr(key))
                }),
            )
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(if effective.is_error {
                        cx.theme().danger
                    } else {
                        cx.theme().foreground
                    })
                    .child(effective.text),
            )
            .children(effective.detail.map(|detail| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(detail)
            }))
            .into_any_element()
    }

    fn project_actions(&self, cx: &Context<Self>) -> gpui_kit::AnyElement {
        h_flex()
            .w_full()
            .items_center()
            .gap_3()
            .pt_2()
            .child(
                Button::new("settings-project-refresh")
                    .small()
                    .ghost()
                    .label(tr("lithe.settings.project.refresh"))
                    .disabled(self.project.busy)
                    .on_click(cx.listener(|this, _, _window, cx| this.project_load(cx))),
            )
            .child(
                Button::new("settings-project-save")
                    .small()
                    .label(tr("lithe.ui.save"))
                    .disabled(self.project.busy)
                    .on_click(cx.listener(|this, _, _window, cx| this.project_save(cx))),
            )
            .when(self.project.busy, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.toolchain.detecting")),
                )
            })
            .when(self.project.saved && !self.project.busy, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
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

    fn lsp_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let environment = self.project.environment.as_ref();
        let detecting = || SharedString::from(tr("lithe.toolchain.detecting"));

        let mut service_rows: Vec<gpui_kit::AnyElement> = vec![settings_row(
            "settings-row-lsp-auto-completion",
            tr("lithe.settings.mac.autoCompletion"),
            Some(tr("lithe.settings.gpui.autoCompletionDescription")),
            {
                let store = self.store.clone();
                Switch::new("settings-lsp-auto-completion")
                    .small()
                    .checked(settings.auto_completion)
                    .on_change(move |checked, _, cx| {
                        let checked = *checked;
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
                .text_color(cx.theme().muted_foreground)
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

        vec![
            settings_group(tr("lithe.settings.mac.languageServices"), service_rows, cx)
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

    fn run_page(&self, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let mut rows: Vec<gpui_kit::AnyElement> = Vec::new();

        rows.push(
            div()
                .w_full()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(tr("lithe.settings.gpui.runReadOnlyDescription"))
                .into_any_element(),
        );

        if let Some(error) = self.run.error.as_ref() {
            rows.push(
                div()
                    .w_full()
                    .text_sm()
                    .text_color(cx.theme().danger)
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
                        .text_color(cx.theme().muted_foreground)
                        .child(tr_args(
                            "lithe.run.generatedEntries",
                            &[("count", &view.entry_count.to_string())],
                        ))
                        .into_any_element(),
                );
            }
        }

        rows.push(
            h_flex()
                .w_full()
                .items_center()
                .gap_3()
                .pt_2()
                .child(
                    Button::new("settings-run-refresh")
                        .small()
                        .ghost()
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
                .text_color(cx.theme().muted_foreground)
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
    ) -> gpui_kit::AnyElement {
        let fact = |label_key: &'static str, value: SharedString, muted: bool| {
            h_flex()
                .w_full()
                .items_start()
                .gap_2()
                .child(
                    div()
                        .w(rems(7.5))
                        .flex_shrink_0()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr(label_key)),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_sm()
                        .text_color(if muted {
                            cx.theme().muted_foreground
                        } else {
                            cx.theme().foreground
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

        v_flex()
            .id(("settings-run-row", index))
            .w_full()
            .gap_1()
            .p_3()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover.opacity(0.35))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().foreground)
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

    fn empty_page(&self, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let prerequisite = self
            .category
            .prerequisite_key()
            .expect("空态分类必须登记一条前置条件文案（`Category::prerequisite_key`）");
        vec![
            v_flex()
                .w_full()
                .py_8()
                .child(
                    Empty::new()
                        .border_color(cx.theme().transparent)
                        .gap_3()
                        .px_6()
                        .header(
                            EmptyHeader::new()
                                .max_w_112()
                                .gap_3()
                                .media(
                                    EmptyMedia::new().child(
                                        self.category
                                            .icon(cx.theme().is_dark())
                                            .size_10()
                                            .text_color(cx.theme().muted_foreground),
                                    ),
                                )
                                .title(
                                    EmptyTitle::new()
                                        .text_sm()
                                        .child(tr("lithe.settings.gpui.pageNotAvailableTitle")),
                                )
                                .description(
                                    EmptyDescription::new().text_sm().child(tr(prerequisite)),
                                ),
                        ),
                )
                .into_any_element(),
        ]
    }

    fn footer(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("settings-restore-defaults")
                    .ghost()
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
        let input = self.font_size_input.clone();
        let editor_input = self.editor_font_size_input.clone();

        window.open_dialog(
            "lithe-settings-restore-defaults",
            move |id, _window, cx| {
                let store = store.clone();
                let input = input.clone();
                let editor_input = editor_input.clone();
                let (cancel_id, confirm_id) = ((id.clone(), "cancel"), (id.clone(), "confirm"));
                Dialog::new(id)
                    .title(tr("lithe.settings.gpui.restoreDefaultsTitle"))
                    .on_close(dismiss_hosted)
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(tr("lithe.settings.gpui.restoreDefaultsBody")),
                    )
                    .action(move |close| {
                        Button::new(cancel_id)
                            .label(tr("lithe.ui.cancel"))
                            .on_click(move |_, window, cx| close(window, cx))
                    })
                    .action(move |close| {
                        Button::new(confirm_id)
                            .danger()
                            .label(tr("lithe.settings.mac.restoreDefaults"))
                            .on_click(move |_, window, cx| {
                                store.update(cx, |store, cx| store.restore_defaults(cx));
                                input.update(cx, |input, cx| {
                                    input.set_value(
                                        SharedString::from(format_ui_font_size(
                                            UI_FONT_SIZE_DEFAULT,
                                        )),
                                        window,
                                        cx,
                                    );
                                });
                                editor_input.update(cx, |input, cx| {
                                    input.set_value(
                                        SharedString::from(format_editor_font_size(
                                            EDITOR_FONT_SIZE_DEFAULT,
                                        )),
                                        window,
                                        cx,
                                    );
                                });
                                close(window, cx);
                            })
                    })
                    .into_any_element()
            },
            cx,
        );
    }

    fn dropdown(
        &self,
        id: impl Into<ElementId>,
        current_label: SharedString,
        options: Vec<(SharedString, SharedString, bool)>,
        on_select: Box<dyn Fn(SharedString, &mut Window, &mut App) + 'static>,
    ) -> gpui_kit::AnyElement {
        let on_select = std::rc::Rc::new(on_select);
        Button::new(id)
            .outline()
            .w(ControlWidth::Default.length())
            .justify_between()
            .child(div().min_w_0().truncate().child(current_label))
            .dropdown_caret(true)
            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                options
                    .iter()
                    .fold(menu, |menu, (value, label, checked)| {
                        let on_select = on_select.clone();
                        let value = value.clone();
                        menu.item(
                            PopupMenuItem::new(label.clone())
                                .checked(*checked)
                                .on_click(move |_, window, cx| {
                                    on_select(value.clone(), window, cx);
                                }),
                        )
                    })
                    .scrollable(true)
            })
            .into_any_element()
    }
}

impl Render for SettingsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.header(cx))
            .child(
                h_flex()
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

fn format_ui_font_size(value: f64) -> String {
    let normalized = crate::schema::normalize_ui_font_size(value);
    if normalized.fract() == 0.0 {
        format!("{}", normalized as i64)
    } else {
        format!("{normalized}")
    }
}

fn format_editor_font_size(value: f64) -> String {
    format!(
        "{}",
        crate::schema::normalize_editor_font_size(value) as i64
    )
}

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

    #[test]
    fn font_size_labels_are_compact() {
        assert_eq!(format_ui_font_size(13.0), "13");
        assert_eq!(format_ui_font_size(16.5), "16.5");
        assert_eq!(format_ui_font_size(13.24), "13");
        assert_eq!(format_editor_font_size(14.0), "14");
        assert_eq!(format_editor_font_size(14.6), "15");
    }

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
