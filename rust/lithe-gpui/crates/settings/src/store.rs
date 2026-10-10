use std::path::PathBuf;

use gpui::Global;
use gpui::{
    App, AppContext as _, Context, Entity, SharedString, Subscription, Window, WindowAppearance,
};
use serde_json::Value;

use lithe_gpui_shared::workspace_config::WorkspaceConfigPaths;

use crate::persistence::{self, DebounceState, Loaded};
use crate::schema::{Settings, theme_font_size_for};
use crate::theme;
use crate::workspace::{self, AppearanceKey, AppearanceOverlay, AppearanceSource, AppearanceValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceMode {
    System,
    Light,
    Dark,
}

impl AppearanceMode {
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

#[derive(Debug, Clone, Copy)]
enum Effects {
    Theme,
    FontSize,
    EditorFontSize,
    FontFamily,
    None,
}

pub struct SettingsStore {
    settings: Settings,
    global: Settings,
    workspace_root: Option<PathBuf>,
    workspace_shared: AppearanceOverlay,
    workspace_shared_previous: Option<Value>,
    workspace_shared_read_only: bool,
    workspace_local: AppearanceOverlay,
    workspace_local_previous: Option<Value>,
    workspace_local_read_only: bool,
    workspace_shared_tracked: Option<bool>,
    path: Option<PathBuf>,
    previous: Option<Value>,
    read_only: bool,
    theme_override: Option<SharedString>,
    applied_theme: SharedString,
    debounce: DebounceState,
    appearance_subscription: Option<Subscription>,
    theme_subscription: Option<Subscription>,
    quit_subscription: Option<Subscription>,
}

struct SettingsHandle(Entity<SettingsStore>);

impl Global for SettingsHandle {}

pub fn store(cx: &App) -> Entity<SettingsStore> {
    cx.global::<SettingsHandle>().0.clone()
}

pub fn try_store(cx: &App) -> Option<Entity<SettingsStore>> {
    cx.try_global::<SettingsHandle>()
        .map(|handle| handle.0.clone())
}

pub struct Init {
    pub loaded: Loaded,
    pub theme_override: Option<SharedString>,
}

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

    store.update(cx, |store, cx| store.install_watcher(cx));
    store.update(cx, |store, cx| store.install_quit_flush(cx));

    store.update(cx, |store, cx| store.apply_font_size(cx));
    store.update(cx, |store, cx| store.apply_editor_font_size(cx));

    store.update(cx, |store, cx| {
        store.theme_subscription = Some(
            cx.observe_global::<lithe_ui::theme::Theme>(|store, cx| store.enforce_rem_base(cx)),
        );
    });

    store
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceLayer {
    Shared,
    Local,
}

impl WorkspaceLayer {
    fn as_str(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Local => "local",
        }
    }
}

fn source_tag(source: AppearanceSource) -> &'static str {
    match source {
        AppearanceSource::Global => "global",
        AppearanceSource::ProjectShared { tracked: true } => "team",
        AppearanceSource::ProjectShared { tracked: false } => "project",
        AppearanceSource::ProjectLocal => "local",
    }
}

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

fn overlay_keys(overlay: &AppearanceOverlay) -> Vec<AppearanceKey> {
    AppearanceKey::ALL
        .iter()
        .copied()
        .filter(|key| overlay.has(*key))
        .collect()
}

fn value_text(settings: &Settings, key: AppearanceKey) -> String {
    match AppearanceValue::from_settings(settings, key) {
        AppearanceValue::Text(text) => text,
        AppearanceValue::Number(number) => number.to_string(),
        AppearanceValue::Flag(flag) => flag.to_string(),
    }
}

impl SettingsStore {
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn global_settings(&self) -> &Settings {
        &self.global
    }

    pub fn workspace_root(&self) -> Option<&std::path::Path> {
        self.workspace_root.as_deref()
    }

    pub fn appearance_source(&self, key: AppearanceKey) -> AppearanceSource {
        workspace::source_for(
            key,
            &self.workspace_shared,
            &self.workspace_local,
            self.workspace_shared_tracked,
        )
    }

    pub fn effective_theme_source(&self, system_is_dark: bool) -> AppearanceSource {
        let key = workspace::effective_theme_key(&self.settings, system_is_dark);
        self.appearance_source(key)
    }

    pub fn appearance_mode_source(&self, system_is_dark: bool) -> AppearanceSource {
        if self.settings.sync_system_theme {
            let key = AppearanceKey::SyncSystemTheme;
            if self.appearance_source(key).is_workspace_override() {
                return self.appearance_source(key);
            }
        }
        self.effective_theme_source(system_is_dark)
    }

    pub fn appearance_sources(&self) -> Vec<workspace::AppearanceKeySource> {
        workspace::appearance_sources(
            &self.workspace_shared,
            &self.workspace_local,
            self.workspace_shared_tracked,
        )
    }

    pub fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }

    pub fn applied_theme(&self) -> SharedString {
        self.applied_theme.clone()
    }

    pub fn appearance_mode(&self) -> AppearanceMode {
        AppearanceMode::from_settings(&self.settings)
    }

    pub fn set_theme(&mut self, value: SharedString, cx: &mut Context<Self>) {
        self.theme_override = None;
        let id = theme::canonical_theme_id(cx, &value);
        let is_dark = theme::theme_is_dark(cx, &id);
        let key = if self.settings.sync_system_theme {
            match is_dark {
                Some(true) => AppearanceKey::AutoThemeDark,
                Some(false) => AppearanceKey::AutoThemeLight,
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

    pub fn set_theme_explicit(&mut self, value: SharedString, cx: &mut Context<Self>) {
        self.theme_override = None;
        let id = theme::canonical_theme_id(cx, &value);
        self.commit_appearance(
            cx,
            &[
                (AppearanceKey::SyncSystemTheme, AppearanceValue::Flag(false)),
                (AppearanceKey::Theme, AppearanceValue::Text(id.to_string())),
            ],
            Effects::Theme,
        );
    }

    pub fn set_appearance_mode(&mut self, mode: AppearanceMode, cx: &mut Context<Self>) {
        self.theme_override = None;
        let mut values = vec![(
            AppearanceKey::SyncSystemTheme,
            AppearanceValue::Flag(mode == AppearanceMode::System),
        )];
        match mode {
            AppearanceMode::System => {}
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

    pub fn set_ui_font_size(&mut self, value: f64, cx: &mut Context<Self>) {
        self.commit_appearance(
            cx,
            &[(AppearanceKey::UiFontSize, AppearanceValue::Number(value))],
            Effects::FontSize,
        );
    }

    pub fn set_show_status_bar(&mut self, value: bool, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.show_status_bar = value;
        self.commit_global(cx, next, Effects::None);
    }

    pub fn icons(&self) -> String {
        self.settings.icons.clone()
    }

    pub fn uninstalled_extensions(&self) -> &[String] {
        &self.global.uninstalled_extensions
    }

    pub fn set_icons(&mut self, package: &str, cx: &mut Context<Self>) {
        if !self.apply_icons_package(package, cx) {
            return;
        }
        let mut next = self.global.clone();
        next.icons = package.to_string();
        self.commit_global(cx, next, Effects::None);
    }

    fn apply_icons(&mut self, cx: &mut Context<Self>) {
        let mut package = self.settings.icons.clone();
        if let Some(extension) = crate::extensions::icons_extension(&package) {
            if self
                .global
                .uninstalled_extensions
                .iter()
                .any(|id| id == extension.id)
            {
                eprintln!(
                    "S1_SETTINGS icons_uninstalled package={package} fallback={}",
                    crate::schema::DEFAULT_ICONS
                );
                package = crate::schema::DEFAULT_ICONS.to_string();
            }
        }
        if !self.apply_icons_package(&package, cx) {
            let fallback = crate::schema::DEFAULT_ICONS;
            eprintln!("S1_SETTINGS icons_missing package={package} fallback={fallback}");
            self.apply_icons_package(fallback, cx);
        }
    }

    fn apply_languages(&self) {
        let disabled = crate::extensions::disabled_languages(&self.global.uninstalled_extensions);
        lithe_gpui_shared::languages::set_disabled_languages(&disabled);
    }

    fn apply_icons_package(&self, package: &str, cx: &App) -> bool {
        let source = cx.asset_source();
        lithe_gpui_shared::icons::file_icon::set_active_icons(package, source.as_ref())
    }

    pub fn set_extension_uninstalled(
        &mut self,
        id: &str,
        uninstalled: bool,
        cx: &mut Context<Self>,
    ) {
        if crate::extensions::bundled_extension(id).is_none() {
            eprintln!("S1_SETTINGS extension_unknown id={id}");
            return;
        }
        let result = if uninstalled {
            crate::extensions::uninstall(id)
        } else {
            crate::extensions::reinstall(id)
        };
        if let Err(error) = result {
            eprintln!(
                "S1_SETTINGS extension_failed id={id} uninstalled={uninstalled} error={error}"
            );
            return;
        }
        let mut next = self.global.clone();
        if uninstalled {
            if !next.uninstalled_extensions.iter().any(|value| value == id) {
                next.uninstalled_extensions.push(id.to_string());
            }
        } else {
            next.uninstalled_extensions.retain(|value| value != id);
        }
        self.commit_global(cx, next, Effects::None);
    }

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

    pub fn set_font_family(&mut self, family: String, cx: &mut Context<Self>) {
        self.commit_appearance(
            cx,
            &[(AppearanceKey::FontFamily, AppearanceValue::Text(family))],
            Effects::FontFamily,
        );
    }

    pub fn set_mono_font_family(&mut self, family: String, cx: &mut Context<Self>) {
        self.commit_appearance(
            cx,
            &[(AppearanceKey::MonoFontFamily, AppearanceValue::Text(family))],
            Effects::FontFamily,
        );
    }

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

    pub fn set_tab_size(&mut self, value: u32, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.tab_size = value;
        self.commit_global(cx, next, Effects::None);
    }

    pub fn set_terminal_default_shell_id(&mut self, id: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.terminal_default_shell_id = id;
        self.commit_global(cx, next, Effects::None);
    }

    pub fn set_confirm_before_discard(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.confirm_before_discard = enabled;
        self.commit_global(cx, next, Effects::None);
    }

    pub fn set_java_home_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.java_home_path = path;
        self.commit_global(cx, next, Effects::None);
    }

    pub fn set_maven_executable_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.maven_executable_path = path;
        self.commit_global(cx, next, Effects::None);
    }

    pub fn set_maven_java_home_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.maven_java_home_path = path;
        self.commit_global(cx, next, Effects::None);
    }

    pub fn set_maven_settings_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.maven_settings_path = path;
        self.commit_global(cx, next, Effects::None);
    }

    pub fn set_maven_local_repository_path(&mut self, path: String, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.maven_local_repository_path = path;
        self.commit_global(cx, next, Effects::None);
    }

    pub fn set_auto_completion(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.auto_completion = enabled;
        self.commit_global(cx, next, Effects::None);
    }

    pub fn set_word_wrap(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.word_wrap = enabled;
        self.commit_global(cx, next, Effects::None);
    }

    pub fn set_auto_save(&mut self, enabled: bool, cx: &mut Context<Self>) {
        let mut next = self.global.clone();
        next.auto_save = enabled;
        self.commit_global(cx, next, Effects::None);
    }

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

    fn commit_global(
        &mut self,
        cx: &mut Context<Self>,
        mut next: Settings,
        effects: Effects,
    ) -> bool {
        next.normalize();
        if next == self.global {
            return false;
        }
        self.global = next;
        self.recompute_effective(cx, effects);
        self.after_change(cx);
        true
    }

    fn commit_appearance(
        &mut self,
        cx: &mut Context<Self>,
        values: &[(AppearanceKey, AppearanceValue)],
        effects: Effects,
    ) -> bool {
        if self.workspace_root.is_none() {
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

    pub fn restore_defaults(&mut self, cx: &mut Context<Self>) {
        self.global = Settings::default();
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

    pub fn flush_pending(&mut self) {
        if !self.debounce.has_pending() {
            return;
        }
        self.debounce.mark_flushed();
        self.write();
    }

    pub fn set_workspace_root(&mut self, root: Option<PathBuf>, cx: &mut Context<Self>) {
        self.workspace_root = root;
        self.workspace_shared = AppearanceOverlay::default();
        self.workspace_shared_previous = None;
        self.workspace_shared_read_only = false;
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

    pub fn set_workspace_settings_tracked(
        &mut self,
        tracked: Option<bool>,
        cx: &mut Context<Self>,
    ) {
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
        self.report_appearance_sources();
        cx.notify();
    }

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

    pub fn revert_appearance_to_global(&mut self, key: AppearanceKey, cx: &mut Context<Self>) {
        self.revert_appearance_group_to_global(&[key], cx);
    }

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

        let effects = if keys.len() == 1 {
            effects_for(keys[0])
        } else {
            Effects::Theme
        };
        self.recompute_effective(cx, effects);

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
            Err(error) => eprintln!(
                "S1_SETTINGS workspace_save_failed layer={} path={} error={error}",
                layer.as_str(),
                path.display()
            ),
        }
    }

    fn install_watcher(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.path.clone() else {
            eprintln!("S1_SETTINGS watch_skipped reason=no_path");
            return;
        };
        crate::watch::watch_settings_file(cx, path);
    }

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
            async {}
        }));
    }

    pub fn reload_from_disk(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.path.clone() else {
            return;
        };
        let loaded = persistence::load_from(Some(path.clone()));

        if !loaded.file_existed {
            eprintln!(
                "S1_SETTINGS reload_skipped path={} reason=file_missing",
                path.display()
            );
            return;
        }

        let changed = loaded.settings != self.global;
        let read_only_changed = loaded.read_only != self.read_only;
        self.previous = loaded.previous;
        if !changed && !read_only_changed {
            return;
        }

        self.global = loaded.settings;
        self.read_only = loaded.read_only;
        self.settings = workspace::resolve_effective(
            &self.global,
            &self.workspace_shared,
            &self.workspace_local,
        );
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

        self.apply_theme(cx);
        self.apply_font_size(cx);
        self.apply_editor_font_size(cx);
        cx.notify();
    }

    pub fn reapply_theme(&mut self, cx: &mut Context<Self>) {
        self.apply_theme(cx);
    }

    pub fn attach_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.appearance_subscription.is_some() {
            return;
        }
        self.appearance_subscription =
            Some(cx.observe_window_appearance(window, |store, window, cx| {
                if !store.settings.sync_system_theme {
                    return;
                }
                store.apply_theme_for(window.appearance(), cx);
            }));
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

        let id = theme::canonical_theme_id(cx, &raw);

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
        theme::apply_theme_by_id(cx, &applied);
        self.apply_font_size(cx);
        self.apply_editor_font_size(cx);
        self.apply_font_families(cx);
        self.apply_icons(cx);
        self.apply_languages();
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

    /// 缩放锚点归**组件库**主题：库的 `font_size` 是唯一杠杆，`Theme::update` 会把它标到
    /// 所有窗口的 rem 大小上（等价于原先写 gpui-kit 主题再 `sync_base`）。
    fn enforce_rem_base(&mut self, cx: &mut App) {
        if !cx.has_global::<lithe_ui::theme::Theme>() {
            return;
        }
        let desired = theme_font_size_for(self.settings.ui_font_size);
        if cx.global::<lithe_ui::theme::Theme>().font_size == desired {
            return;
        }
        lithe_ui::theme::Theme::update(cx, |theme| theme.font_size = desired);
    }

    fn after_change(&mut self, cx: &mut Context<Self>) {
        self.schedule_save(cx);
        cx.notify();
    }

    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        if self.path.is_none() {
            return;
        }
        let armed = self.debounce.arm();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(
                    persistence::SAVE_DEBOUNCE_MS,
                ))
                .await;
            let _ = this.update(cx, |store, _| store.flush_if_current(armed));
        })
        .detach();
    }

    fn flush_if_current(&mut self, armed: u64) {
        if self.debounce.should_flush(armed) {
            self.write();
        }
    }

    fn write(&mut self) {
        let Some(path) = self.path.clone() else {
            return;
        };
        if self.read_only {
            eprintln!(
                "S1_SETTINGS save_skipped path={} reason=document_version_newer",
                path.display()
            );
            return;
        }

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

    #[test]
    fn appearance_mode_follows_windows_heuristic() {
        let mut settings = Settings::default();
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

    #[test]
    fn loaded_file_drives_the_schema() {
        let parsed = persistence::parse(r#"{"displayLanguage": "en-US", "uiFontSize": 15.0}"#);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(parsed.settings.display_language, "en-US");
        assert_eq!(parsed.settings.gpui_locale(), "en");
        assert_eq!(parsed.settings.ui_font_size, 15.0);
        assert_eq!(parsed.settings.display_language.is_empty(), false);
        let only_theme = persistence::parse(r#"{"theme": "Lithe Light"}"#);
        assert_eq!(
            only_theme.settings.display_language,
            DEFAULT_DISPLAY_LANGUAGE
        );
    }
}
