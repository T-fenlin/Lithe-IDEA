use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::component::{ActiveTheme as _, Theme, ThemeConfig, ThemeMode, ThemeRegistry};
use gpui_kit::{App, SharedString, WindowAppearance, px};
use lithe_gpui_shared::design;
use lithe_gpui_shared::rem_px;

use crate::paths;
use crate::schema::{ThemeIndex, slugify_theme_id, theme_font_size_for};

pub const BUILTIN_THEMES: &[(&str, &str)] = &[(
    "jetbrains.json",
    include_str!("../../../themes/jetbrains.json"),
)];

pub fn bundled_theme_files() -> impl Iterator<Item = (&'static str, &'static str, &'static [u8])> {
    crate::extensions::BUNDLED_EXTENSIONS
        .iter()
        .filter(|entry| entry.kind == crate::extensions::BundledKind::Theme)
        .filter_map(|entry| {
            let (relative, content) = entry.files.first()?;
            Some((entry.id, *relative, *content))
        })
}

pub const NOTIFICATION_MAX_ITEMS: usize = 3;

pub const NOTIFICATION_MARGIN_RIGHT_SPEC: f32 = design::ACTIVITY_RAIL_WIDTH_SPEC + 12.;

pub const NOTIFICATION_MARGIN_BOTTOM_SPEC: f32 = 38.;

const LEGACY_THEME_FILE_NAMES: &[&str] = &["gruvbox.json", "nord.json", "one.json", "vscode.json"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedReport {
    pub created: Vec<String>,
    pub kept: Vec<String>,
    pub failed: Vec<String>,
    pub removed: Vec<String>,
}

impl SeedReport {
    pub fn summary(&self) -> String {
        format!(
            "created={} kept={} failed={}",
            self.created.len(),
            self.kept.len(),
            self.failed.len()
        )
    }
}

pub fn seed_bundled_themes_into(dir: &Path) -> SeedReport {
    let mut report = SeedReport::default();

    if let Err(error) = std::fs::create_dir_all(dir) {
        eprintln!("S1_THEME seed_failed dir={} error={error}", dir.display());
        report
            .failed
            .extend(BUILTIN_THEMES.iter().map(|(name, _)| (*name).to_string()));
        return report;
    }

    for (name, content) in BUILTIN_THEMES {
        let path = dir.join(name);
        if path.exists() {
            report.kept.push((*name).to_string());
            continue;
        }
        match write_atomically(&path, content.as_bytes()) {
            Ok(()) => report.created.push((*name).to_string()),
            Err(error) => {
                eprintln!(
                    "S1_THEME seed_write_failed path={} error={error}",
                    path.display()
                );
                report.failed.push((*name).to_string());
            }
        }
    }

    report
}

fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = crate::persistence::tmp_path(path);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

pub fn seed_bundled_themes() -> Option<SeedReport> {
    let dir = paths::themes_dir()?;
    let report = seed_bundled_themes_into(&dir);
    println!("S1_THEME seeded dir={} {}", dir.display(), report.summary());
    Some(report)
}

pub fn migrate_legacy_theme_files(load_dir: &Path, store_dir: &Path) -> SeedReport {
    let mut report = SeedReport::default();
    if !load_dir.is_dir() {
        return report;
    }
    let _ = std::fs::create_dir_all(store_dir);
    let current: std::collections::BTreeSet<&str> =
        bundled_theme_files().map(|(_, file, _)| file).collect();

    for name in LEGACY_THEME_FILE_NAMES {
        let legacy_path = load_dir.join(name);
        if !legacy_path.exists() {
            continue;
        }
        let store_path = store_dir.join(name);
        if store_path.exists() {
            if current.contains(name) {
                continue;
            }
            match std::fs::remove_file(&legacy_path) {
                Ok(()) => report.removed.push((*name).to_string()),
                Err(error) => eprintln!(
                    "S1_THEME migrate_remove_failed path={} error={error}",
                    legacy_path.display()
                ),
            }
            continue;
        }
        match std::fs::rename(&legacy_path, &store_path) {
            Ok(()) => report.removed.push(format!("{name} (migrated)")),
            Err(error) => eprintln!(
                "S1_THEME migrate_failed path={} error={error}",
                legacy_path.display()
            ),
        }
    }
    for name in ["lithe-dark.json", "lithe-light.json"] {
        let legacy_path = load_dir.join(name);
        if legacy_path.exists() {
            match std::fs::remove_file(&legacy_path) {
                Ok(()) => report.removed.push((*name).to_string()),
                Err(error) => eprintln!(
                    "S1_THEME migrate_remove_failed path={} error={error}",
                    legacy_path.display()
                ),
            }
        }
    }
    report
}

fn builtin_index() -> &'static ThemeIndex {
    static INDEX: OnceLock<ThemeIndex> = OnceLock::new();
    INDEX.get_or_init(|| {
        ThemeIndex::from_documents(
            BUILTIN_THEMES
                .iter()
                .map(|(_, content)| (*content).to_string())
                .collect::<Vec<_>>(),
        )
    })
}

fn theme_documents_in(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json")
        })
        .collect();
    paths.sort();
    paths
        .into_iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .collect()
}

fn user_themes_stamp(dir: &Path) -> Option<SystemTime> {
    std::fs::metadata(dir)
        .ok()
        .and_then(|meta| meta.modified().ok())
}

type ThemeIndexCache = Mutex<Option<(Option<SystemTime>, ThemeIndex)>>;

fn lock_index_cache(
    cache: &ThemeIndexCache,
) -> std::sync::MutexGuard<'_, Option<(Option<SystemTime>, ThemeIndex)>> {
    cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn theme_index() -> ThemeIndex {
    static CACHE: OnceLock<ThemeIndexCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));

    let Some(dir) = paths::themes_dir() else {
        return builtin_index().clone();
    };
    let stamp = user_themes_stamp(&dir);

    {
        let guard = lock_index_cache(cache);
        if let Some((cached_stamp, index)) = guard.as_ref() {
            if *cached_stamp == stamp {
                return index.clone();
            }
        }
    }

    let mut documents = theme_documents_in(&dir);
    documents.extend(
        BUILTIN_THEMES
            .iter()
            .map(|(_, content)| (*content).to_string()),
    );
    let index = ThemeIndex::from_documents(documents);
    *lock_index_cache(cache) = Some((stamp, index.clone()));
    index
}

pub fn theme_ids(cx: &App) -> Vec<String> {
    let index = theme_index();
    let mut seen = BTreeSet::new();
    ThemeRegistry::global(cx)
        .sorted_themes()
        .into_iter()
        .map(|theme| {
            index
                .id_for_name(&theme.name)
                .map(str::to_string)
                .unwrap_or_else(|| slugify_theme_id(&theme.name))
        })
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

pub fn theme_options(cx: &App) -> Vec<(String, String)> {
    let index = theme_index();
    let mut seen = BTreeSet::new();
    ThemeRegistry::global(cx)
        .sorted_themes()
        .into_iter()
        .filter_map(|theme| {
            let name = theme.name.to_string();
            let id = index
                .id_for_name(&name)
                .map(str::to_string)
                .unwrap_or_else(|| slugify_theme_id(&name));
            seen.insert(id.clone()).then_some((id, name))
        })
        .collect()
}

pub fn display_name_for(value: &str) -> String {
    let index = theme_index();
    index
        .canonicalize(value)
        .and_then(|id| index.name_for_id(&id).map(str::to_string))
        .unwrap_or_else(|| value.to_string())
}

pub fn canonical_theme_id(cx: &App, value: &str) -> String {
    let name = resolve_name_for_id(cx, value);
    theme_index()
        .id_for_name(&name)
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

fn resolve_name_for_id(cx: &App, id: &str) -> String {
    let registry = ThemeRegistry::global(cx);
    let index = theme_index();

    if let Some(canonical) = index.canonicalize(id) {
        if let Some(name) = index.name_for_id(&canonical) {
            if registry
                .themes()
                .contains_key(&SharedString::from(name.to_string()))
            {
                return name.to_string();
            }
        }
    }

    registry
        .sorted_themes()
        .into_iter()
        .find(|theme| slugify_theme_id(&theme.name) == id)
        .map(|theme| theme.name.to_string())
        .unwrap_or_else(|| id.to_string())
}

pub fn watch_lithe_themes(cx: &mut App) {
    if seed_bundled_themes().is_none() {
        eprintln!("S1_THEME seed_skipped reason=no_themes_dir");
    }

    let Some(themes_dir) = paths::themes_dir() else {
        eprintln!("S1_THEME watch_skipped reason=no_themes_dir");
        return;
    };

    let result =
        ThemeRegistry::watch_dir(
            themes_dir.clone(),
            cx,
            move |cx| match crate::store::try_store(cx) {
                Some(store) => {
                    store.update(cx, |store, cx| store.reapply_theme(cx));
                }
                None => {
                    let id = crate::schema::DEFAULT_THEME;
                    if !apply_theme_by_id(cx, id) {
                        eprintln!("S1_THEME missing id={id}");
                    }
                }
            },
        );

    match result {
        Ok(()) => println!("S1_THEME watching dir={}", themes_dir.display()),
        Err(error) => eprintln!(
            "S1_THEME watch_failed dir={} error={error}",
            themes_dir.display()
        ),
    }
}

pub fn apply_theme_by_id(cx: &mut App, id: &str) -> bool {
    let name = resolve_name_for_id(cx, id);
    if name != id {
        println!("S1_THEME resolved id={id} name={name}");
    }
    apply_theme_by_name(cx, &name)
}

pub fn apply_theme_by_name(cx: &mut App, name: &str) -> bool {
    let theme = ThemeRegistry::global(cx)
        .themes()
        .get(&SharedString::from(name.to_string()))
        .cloned();

    let Some(theme) = theme else {
        eprintln!("S1_THEME missing name={name}");
        return false;
    };

    let mode = theme.mode;
    Theme::change(mode, None, cx);
    Theme::global_mut(cx).notification.max_items = NOTIFICATION_MAX_ITEMS;
    stamp_builtin_highlight_if_absent(cx, &theme, mode);
    Theme::global_mut(cx).apply_config(&theme);
    let rem = cx.theme().font_size;
    let notification = &mut Theme::global_mut(cx).notification;
    notification.margins.right = rem_px(rem, NOTIFICATION_MARGIN_RIGHT_SPEC);
    notification.margins.bottom = rem_px(rem, NOTIFICATION_MARGIN_BOTTOM_SPEC);
    Theme::sync_base(cx);
    cx.refresh_windows();
    println!("S1_THEME applied={name} dark={}", mode.is_dark());
    true
}

fn stamp_builtin_highlight_if_absent(cx: &mut App, theme: &ThemeConfig, mode: ThemeMode) {
    if theme.highlight.is_some() {
        return;
    }

    let builtin = ThemeRegistry::global(cx)
        .default_themes()
        .get(&mode)
        .cloned();
    let Some(builtin) = builtin else {
        return;
    };
    let Some(style) = builtin.highlight.clone() else {
        return;
    };

    Theme::global_mut(cx).highlight_theme = Arc::new(HighlightTheme {
        name: builtin.name.to_string(),
        appearance: mode,
        style,
    });
    println!(
        "S1_THEME highlight=builtin theme={} dark={} source={}",
        theme.name,
        mode.is_dark(),
        builtin.name
    );
}

pub fn apply_theme_font_size(cx: &mut App, ui_font_size: f64) {
    let font_size = px(theme_font_size_for(ui_font_size));
    Theme::global_mut(cx).font_size = font_size;
    Theme::sync_base(cx);
    cx.refresh_windows();
    println!(
        "S1_THEME font_size={:?} ui_font_size={ui_font_size}",
        font_size
    );
}

pub fn apply_editor_font_size(cx: &mut App, editor_font_size: f64) {
    let font_size = px(editor_font_size as f32);
    Theme::global_mut(cx).mono_font_size = font_size;
    Theme::sync_base(cx);
    cx.refresh_windows();
    println!("S1_THEME mono_font_size={font_size:?} editor_font_size={editor_font_size}");
}

pub const SYSTEM_UI_FONT_FAMILY: &str = ".SystemUIFont";

pub fn apply_font_families(cx: &mut App, ui_family: &str, mono_family: &str) {
    let installed = installed_font_names(cx);

    if let Some(family) = usable_family("fontFamily", ui_family, installed) {
        Theme::global_mut(cx).font_family = SharedString::from(family.to_string());
        println!("S1_THEME font_family key=fontFamily value={family}");
    }
    if let Some(family) = usable_family("monoFontFamily", mono_family, installed) {
        Theme::global_mut(cx).mono_font_family = SharedString::from(family.to_string());
        println!("S1_THEME font_family key=monoFontFamily value={family}");
    }

    Theme::sync_base(cx);
    cx.refresh_windows();
}

pub fn installed_font_names(cx: &App) -> &'static [String] {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES.get_or_init(|| cx.text_system().all_font_names())
}

pub fn usable_family<'a>(key: &str, value: &'a str, installed: &[String]) -> Option<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if value == SYSTEM_UI_FONT_FAMILY {
        return Some(value);
    }
    if installed.iter().any(|name| name == value) {
        return Some(value);
    }
    eprintln!("S1_THEME font_family_rejected key={key} family={value} reason=not_installed");
    None
}

pub fn theme_names(cx: &App) -> Vec<String> {
    ThemeRegistry::global(cx)
        .sorted_themes()
        .into_iter()
        .map(|theme| theme.name.to_string())
        .collect()
}

pub fn theme_is_dark(cx: &App, value: &str) -> Option<bool> {
    let name = resolve_name_for_id(cx, value);
    ThemeRegistry::global(cx)
        .themes()
        .get(&SharedString::from(name))
        .map(|theme| theme.mode.is_dark())
}

pub fn system_is_dark(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_right_inset_derives_from_the_activity_rail_owner() {
        assert_eq!(NOTIFICATION_MARGIN_RIGHT_SPEC, 52.);
        assert_eq!(
            NOTIFICATION_MARGIN_RIGHT_SPEC,
            design::ACTIVITY_RAIL_WIDTH_SPEC + 12.,
            "通知右内衬必须派生自活动栏 rail 宽 owner（真源 `WorkbenchView.swift:553` 的 `+ 12`）"
        );
    }

    #[test]
    fn usable_family_only_accepts_empty_virtual_or_installed() {
        let installed = vec!["Inter".to_string(), "JetBrains Mono".to_string()];

        assert_eq!(usable_family("fontFamily", "", &installed), None);
        assert_eq!(usable_family("fontFamily", "   ", &installed), None);

        assert_eq!(
            usable_family("fontFamily", "Inter", &installed),
            Some("Inter")
        );
        assert_eq!(
            usable_family("monoFontFamily", " JetBrains Mono ", &installed),
            Some("JetBrains Mono")
        );

        assert_eq!(
            usable_family("fontFamily", SYSTEM_UI_FONT_FAMILY, &installed),
            Some(SYSTEM_UI_FONT_FAMILY)
        );

        assert_eq!(
            usable_family("fontFamily", "No Such Font", &installed),
            None
        );
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(case: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("lithe-theme-seed-{case}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("建临时目录");
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn every_theme_file_is_embedded_and_loadable() {
        use gpui_kit::component::ThemeSet;
        use std::collections::BTreeMap;

        let bundled: BTreeMap<&str, &str> = bundled_theme_files()
            .map(|(_, file, content)| {
                (
                    file,
                    std::str::from_utf8(content).expect("主题文件是 UTF-8 文本"),
                )
            })
            .collect();
        assert_eq!(
            bundled.len(),
            bundled_theme_files().count(),
            "捆绑主题扩展里有重复的文件名"
        );

        for (dir, embedded, label) in [
            (
                paths::repository_themes_dir(),
                BUILTIN_THEMES.iter().copied().collect::<BTreeMap<_, _>>(),
                "内置主题",
            ),
            (
                paths::repository_theme_extensions_dir(),
                bundled,
                "捆绑主题扩展",
            ),
        ] {
            let mut files: Vec<String> = std::fs::read_dir(&dir)
                .unwrap_or_else(|error| {
                    panic!("read_dir failed dir={} error={error}", dir.display())
                })
                .map(|entry| {
                    entry
                        .expect("read_dir entry")
                        .file_name()
                        .to_string_lossy()
                        .to_string()
                })
                .filter(|name| name.ends_with(".json"))
                .collect();
            files.sort();
            assert!(!files.is_empty(), "no theme json in {}", dir.display());
            assert_eq!(
                files.len(),
                embedded.len(),
                "{label} 登记的文件数与 {} 里的不一致：{files:?}",
                dir.display()
            );

            for file in &files {
                let content = embedded
                    .get(file.as_str())
                    .unwrap_or_else(|| panic!("{file} 没有登记进 {label} 表（打包后读不到它）"));

                let on_disk = std::fs::read_to_string(dir.join(file)).expect("读主题文件失败");
                assert_eq!(
                    content, &on_disk,
                    "{file} 的 include_str! 内容与磁盘上的文件不一致"
                );

                let set: ThemeSet = serde_json::from_str(content)
                    .unwrap_or_else(|error| panic!("{file}: not a loadable ThemeSet: {error}"));
                assert!(!set.themes.is_empty(), "{file}: themes[] is empty");

                let index = ThemeIndex::from_documents(vec![(*content).to_string()]);
                assert_eq!(
                    index.entries().len(),
                    set.themes.len(),
                    "{file}: 每条主题都必须有可解析的 id/name"
                );
                if label == "捆绑主题扩展" {
                    assert_eq!(
                        set.themes.len(),
                        2,
                        "{file}: 捆绑主题组必须有浅色与深色两条"
                    );
                    assert!(
                        set.themes
                            .iter()
                            .any(|theme| theme.mode == gpui_kit::component::ThemeMode::Light)
                            && set
                                .themes
                                .iter()
                                .any(|theme| theme.mode == gpui_kit::component::ThemeMode::Dark),
                        "{file}: 捆绑主题组缺浅色或深色变体"
                    );
                }
                for entry in index.entries() {
                    assert!(
                        !entry.id.trim().is_empty(),
                        "{file}: 主题 {:?} 的 id 是空的",
                        entry.name
                    );
                }
            }
        }
    }

    const REQUIRED_COLOR_KEYS: [&str; 13] = [
        "background",
        "border",
        "foreground",
        "muted.background",
        "primary.background",
        "secondary.background",
        "overlay",
        "base.red",
        "base.green",
        "base.blue",
        "base.yellow",
        "base.magenta",
        "base.cyan",
    ];

    #[test]
    fn every_theme_file_is_a_loadable_theme_set() {
        use gpui_kit::component::{ThemeSet, try_parse_background, try_parse_color};
        use std::collections::BTreeMap;

        for (dir, label) in [
            (paths::repository_themes_dir(), "内置主题"),
            (paths::repository_theme_extensions_dir(), "捆绑主题扩展"),
        ] {
            let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
                .unwrap_or_else(|error| {
                    panic!("read_dir failed dir={} error={error}", dir.display())
                })
                .map(|entry| entry.expect("read_dir entry").path())
                .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
                .collect();
            files.sort();
            assert!(!files.is_empty(), "no theme json in {}", dir.display());

            let mut owners: BTreeMap<String, String> = BTreeMap::new();
            let mut theme_count = 0usize;

            for path in &files {
                let file = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let text = std::fs::read_to_string(path)
                    .unwrap_or_else(|error| panic!("{file}: not readable/UTF-8: {error}"));
                let set: ThemeSet = serde_json::from_str(&text)
                    .unwrap_or_else(|error| panic!("{file}: not a loadable ThemeSet: {error}"));
                assert!(!set.themes.is_empty(), "{file}: themes[] is empty");

                for theme in &set.themes {
                    let name = theme.name.to_string();
                    assert!(!name.trim().is_empty(), "{file}: a theme has an empty name");
                    if let Some(owner) = owners.insert(name.clone(), file.clone()) {
                        panic!("{file}: theme name {name:?} is already defined in {owner}");
                    }

                    let colors = serde_json::to_value(&theme.colors)
                        .unwrap_or_else(|error| panic!("{file}/{name}: colors re-encode: {error}"));
                    let colors = colors
                        .as_object()
                        .unwrap_or_else(|| panic!("{file}/{name}: colors is not an object"));

                    for key in REQUIRED_COLOR_KEYS {
                        match colors.get(key) {
                            Some(serde_json::Value::String(_)) => {}
                            other => panic!(
                                "{file}/{name}: required color {key:?} is missing/not a string ({other:?})"
                            ),
                        }
                    }

                    for (key, value) in colors {
                        let Some(value) = value.as_str() else {
                            assert!(
                                value.is_null(),
                                "{file}/{name}: color {key:?} is not a string ({value:?})"
                            );
                            continue;
                        };
                        assert!(
                            try_parse_color(value).is_ok() || try_parse_background(value).is_ok(),
                            "{file}/{name}: color {key:?} = {value:?} is not parseable by gpui"
                        );
                    }

                    theme_count += 1;
                }
            }

            assert!(theme_count > 0, "no theme defined in {label}");
        }
    }

    #[test]
    fn builtin_index_maps_ids_to_the_expected_names() {
        let index = builtin_index();
        assert_eq!(index.name_for_id("intellij-light"), Some("IntelliJ Light"));
        assert_eq!(index.name_for_id("darcula"), Some("Darcula"));
        assert_eq!(index.entries().len(), 2, "内置只有 IDEA 一组（两条）");
    }

    #[test]
    fn seeding_creates_missing_files_and_never_overwrites() {
        let dir = TempDir::new("seed");

        let first = seed_bundled_themes_into(dir.path());
        assert_eq!(first.created.len(), BUILTIN_THEMES.len());
        assert!(first.kept.is_empty());
        assert!(first.failed.is_empty(), "{first:?}");
        for (name, content) in BUILTIN_THEMES {
            let written = std::fs::read_to_string(dir.path().join(name)).expect("播种产物读不到");
            assert_eq!(&written, content, "{name} 的播种内容不一致");
        }

        let edited = dir.path().join("jetbrains.json");
        std::fs::write(&edited, "{\"name\":\"mine\",\"themes\":[]}").expect("改写失败");

        let second = seed_bundled_themes_into(dir.path());
        assert!(
            second.created.is_empty(),
            "已存在的文件不该被重新播种：{second:?}"
        );
        assert_eq!(second.kept.len(), BUILTIN_THEMES.len());
        assert_eq!(
            std::fs::read_to_string(&edited).expect("读失败"),
            "{\"name\":\"mine\",\"themes\":[]}",
            "已存在的文件绝不能被覆盖"
        );
    }

    #[test]
    fn seeding_creates_the_directory_when_missing() {
        let dir = TempDir::new("seed-missing");
        let nested = dir.path().join("themes");
        assert!(!nested.exists());

        let report = seed_bundled_themes_into(&nested);
        assert_eq!(report.created.len(), BUILTIN_THEMES.len());
        assert!(nested.is_dir());
    }

    #[test]
    fn seeding_writes_atomically_and_leaves_no_temporary_files() {
        let dir = TempDir::new("seed-atomic");
        let report = seed_bundled_themes_into(dir.path());
        assert_eq!(report.created.len(), BUILTIN_THEMES.len());
        assert!(report.failed.is_empty(), "{report:?}");

        let leftovers: Vec<String> = std::fs::read_dir(dir.path())
            .expect("read_dir failed")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "播种残留了临时文件（rename 没成功？）：{leftovers:?}"
        );

        for (name, content) in BUILTIN_THEMES {
            let path = dir.path().join(name);
            assert!(
                !crate::persistence::tmp_path(&path).exists(),
                "{name}: 临时文件必须被 rename 掉"
            );
            assert_eq!(
                &std::fs::read_to_string(&path).expect("播种产物读不到"),
                content,
                "{name}: 播种内容与内置内容不一致"
            );
        }
    }

    #[test]
    fn user_directory_documents_win_over_builtin_ones() {
        let dir = TempDir::new("index-order");
        std::fs::write(
            dir.path().join("jetbrains.json"),
            r#"{"name":"mine","themes":[{"id":"darcula","name":"My Dark","mode":"dark"}]}"#,
        )
        .expect("写入失败");

        let mut documents = theme_documents_in(dir.path());
        documents.extend(
            BUILTIN_THEMES
                .iter()
                .map(|(_, content)| (*content).to_string()),
        );
        let index = ThemeIndex::from_documents(documents);

        assert_eq!(
            index.name_for_id("darcula"),
            Some("My Dark"),
            "用户目录里的那条必须在前面"
        );
        assert_eq!(index.name_for_id("intellij-light"), Some("IntelliJ Light"));
    }

    #[test]
    fn theme_documents_are_top_level_json_in_name_order() {
        let dir = TempDir::new("documents");
        std::fs::write(dir.path().join("b.json"), "{}").expect("写入失败");
        std::fs::write(dir.path().join("a.json"), "{}").expect("写入失败");
        std::fs::write(dir.path().join("notes.md"), "x").expect("写入失败");
        std::fs::create_dir_all(dir.path().join("nested")).expect("建目录失败");
        std::fs::write(dir.path().join("nested").join("c.json"), "{}").expect("写入失败");

        let documents = theme_documents_in(dir.path());
        assert_eq!(documents, vec!["{}".to_string(), "{}".to_string()]);
    }

    #[test]
    fn appearance_maps_to_dark_and_light() {
        assert!(system_is_dark(WindowAppearance::Dark));
        assert!(system_is_dark(WindowAppearance::VibrantDark));
        assert!(!system_is_dark(WindowAppearance::Light));
        assert!(!system_is_dark(WindowAppearance::VibrantLight));
    }

    #[test]
    fn migration_leaves_the_load_surface_mirror_alone() {
        let dir = TempDir::new("migrate-mirror");
        let store_dir = dir.path().join("extensions/themes");
        let load_dir = dir.path().join("themes");
        std::fs::create_dir_all(&store_dir).expect("建管理面");
        std::fs::create_dir_all(&load_dir).expect("建装载面");

        for (_, file, content) in bundled_theme_files() {
            let text = std::str::from_utf8(content).expect("主题文件是 UTF-8");
            std::fs::write(store_dir.join(file), text).expect("写管理面");
            std::fs::write(load_dir.join(file), text).expect("写镜像");
        }

        let report = migrate_legacy_theme_files(&load_dir, &store_dir);
        assert!(report.removed.is_empty(), "镜像不该被动：{report:?}");
        for (_, file, _) in bundled_theme_files() {
            assert!(load_dir.join(file).exists(), "{file} 的镜像必须还在");
        }
    }

    #[test]
    fn migration_moves_legacy_files_and_drops_lithe_splits() {
        let dir = TempDir::new("migrate");
        let store_dir = dir.path().join("extensions/themes");
        let load_dir = dir.path().join("themes");
        std::fs::create_dir_all(&load_dir).expect("建装载面");

        for name in LEGACY_THEME_FILE_NAMES {
            std::fs::write(load_dir.join(name), "{}").expect("造旧文件失败");
        }
        std::fs::write(load_dir.join("lithe-dark.json"), "{}").expect("造旧文件失败");
        std::fs::write(load_dir.join("lithe-light.json"), "{}").expect("造旧文件失败");

        let report = migrate_legacy_theme_files(&load_dir, &store_dir);
        assert_eq!(
            report.removed.len(),
            LEGACY_THEME_FILE_NAMES.len() + 2,
            "{report:?}"
        );
        for name in LEGACY_THEME_FILE_NAMES {
            assert!(store_dir.join(name).exists(), "{name} 应该被搬进扩展目录");
            assert!(!load_dir.join(name).exists(), "{name} 不该还留在装载面");
        }
        assert!(!load_dir.join("lithe-dark.json").exists());
        assert!(!load_dir.join("lithe-light.json").exists());

        let second = migrate_legacy_theme_files(&load_dir, &store_dir);
        assert!(second.removed.is_empty(), "{second:?}");
    }
}
