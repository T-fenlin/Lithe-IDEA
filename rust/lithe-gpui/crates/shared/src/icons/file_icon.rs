use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use gpui_kit::assets::IconName;
use gpui_kit::component::Theme;
use gpui_kit::{AnyElement, App, AssetSource, IntoElement as _, SharedString, Styled as _};

pub const BUILTIN_FILE_ICONS: &str = "idea";

pub const ACTIVE_FILE_ICONS: &str = BUILTIN_FILE_ICONS;

const THEMES_ROOT: &str = "icons";

const BUILTIN_EXTENSION_JSON: &str = include_str!("../../../../assets/icons/idea/extension.json");

static ACTIVE_THEME: Mutex<Option<(&'static str, &'static FileIcons)>> = Mutex::new(None);

pub fn set_active_icons(package: &str, source: &dyn AssetSource) -> bool {
    if active_theme_package() == Some(package) {
        return true;
    }
    let manifest_path = format!("{THEMES_ROOT}/{package}/extension.json");
    let Some(bytes) = source.load(&manifest_path).ok().flatten() else {
        eprintln!("S1_ICONS missing package={package} path={manifest_path}");
        return false;
    };
    let Ok(json) = std::str::from_utf8(&bytes) else {
        eprintln!("S1_ICONS invalid_utf8 package={package}");
        return false;
    };
    match FileIcons::from_json(package, json) {
        Ok(theme) => {
            let package: &'static str = Box::leak(package.to_string().into_boxed_str());
            let theme: &'static FileIcons = Box::leak(Box::new(theme));
            {
                let mut guard = ACTIVE_THEME
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                *guard = Some((package, theme));
            }
            println!(
                "S1_ICONS applied package={package} definitions={} root={}",
                theme.definition_count(),
                theme.root()
            );
            true
        }
        Err(error) => {
            eprintln!("S1_ICONS parse_failed package={package} error={error}");
            false
        }
    }
}

#[must_use]
pub fn active_theme_package() -> Option<&'static str> {
    lock_active_theme().map(|(package, _)| package)
}

fn lock_active_theme() -> Option<(&'static str, &'static FileIcons)> {
    *ACTIVE_THEME
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Debug)]
pub struct FileIcons {
    root: String,
    dark: HashMap<String, String>,
    light: HashMap<String, String>,
    file_names: HashMap<String, String>,
    file_extensions: HashMap<String, String>,
    folders: HashMap<String, String>,
    expanded_folders: HashMap<String, String>,
    default_file: Option<String>,
    default_folder: Option<String>,
    default_folder_open: Option<String>,
}

impl FileIcons {
    fn from_json(theme: &str, json: &str) -> Result<Self, serde_json::Error> {
        let root: serde_json::Value = serde_json::from_str(json)?;
        let root_path = format!("{THEMES_ROOT}/{theme}/");
        let icon = root
            .get("icons")
            .and_then(|icons| icons.as_array())
            .and_then(|entries| entries.iter().max_by_key(|entry| definition_weight(entry)))
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        Ok(Self {
            root: root_path.clone(),
            dark: definitions(icon.get("iconDefinitions"), &root_path),
            light: definitions(icon.get("lightIconDefinitions"), &root_path),
            file_names: string_map(icon.get("filenames")),
            file_extensions: string_map(icon.get("fileExtensions")),
            folders: string_map(icon.get("folders")),
            expanded_folders: string_map(icon.get("expandedFolders")),
            default_file: icon
                .get("defaultFile")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            default_folder: icon
                .get("defaultFolder")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            default_folder_open: icon
                .get("defaultFolderOpen")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        })
    }

    #[must_use]
    pub fn root(&self) -> &str {
        &self.root
    }

    #[must_use]
    pub fn definition_count(&self) -> usize {
        self.dark.len()
    }

    #[must_use]
    pub fn file_id(&self, file_name: &str) -> Option<&str> {
        let lower = file_name.to_ascii_lowercase();
        lookup_mapped(&lower, &self.file_names).or_else(|| {
            extension_candidates(&lower)
                .into_iter()
                .find_map(|candidate| lookup_mapped(candidate, &self.file_extensions))
        })
    }

    #[must_use]
    pub fn folder_id(&self, folder_name: &str, expanded: bool) -> Option<&str> {
        let lower = folder_name.to_ascii_lowercase();
        let expanded_hit = if expanded {
            lookup_mapped(&lower, &self.expanded_folders)
        } else {
            None
        };
        expanded_hit.or_else(|| lookup_mapped(&lower, &self.folders))
    }

    #[must_use]
    pub fn default_file_id(&self) -> Option<&str> {
        self.default_file.as_deref()
    }

    #[must_use]
    pub fn default_folder_id(&self, expanded: bool) -> Option<&str> {
        if expanded {
            self.default_folder_open
                .as_deref()
                .or(self.default_folder.as_deref())
        } else {
            self.default_folder.as_deref()
        }
    }

    #[must_use]
    pub fn resolve(&self, icon: &str, dark: bool) -> Option<&str> {
        let (primary, secondary) = if dark {
            (&self.dark, &self.light)
        } else {
            (&self.light, &self.dark)
        };
        primary
            .get(icon)
            .or_else(|| secondary.get(icon))
            .map(String::as_str)
    }
}

#[must_use]
pub fn lookup_path<'a>(
    active: &'a FileIcons,
    builtin: &'a FileIcons,
    name: &str,
    is_dir: bool,
    expanded: bool,
    dark: bool,
) -> Option<&'a str> {
    let packs = [active, builtin];
    for pack in packs {
        let icon = if is_dir {
            pack.folder_id(name, expanded)
        } else {
            pack.file_id(name)
        };
        if let Some(path) = icon.and_then(|icon| pack.resolve(icon, dark)) {
            return Some(path);
        }
    }
    for pack in packs {
        let icon = if is_dir {
            pack.default_folder_id(expanded)
        } else {
            pack.default_file_id()
        };
        if let Some(path) = icon.and_then(|icon| pack.resolve(icon, dark)) {
            return Some(path);
        }
    }
    None
}

const SEMANTIC_PREFIX: char = '\0';

#[must_use]
pub fn builtin_theme() -> Option<&'static FileIcons> {
    static BUILTIN: OnceLock<Result<FileIcons, serde_json::Error>> = OnceLock::new();
    BUILTIN
        .get_or_init(|| FileIcons::from_json(BUILTIN_FILE_ICONS, BUILTIN_EXTENSION_JSON))
        .as_ref()
        .ok()
}

#[must_use]
pub fn active_theme() -> Option<&'static FileIcons> {
    if let Some((_, theme)) = lock_active_theme() {
        return Some(theme);
    }
    builtin_theme()
}

#[must_use]
pub fn theme_icon_path(name: &str, is_dir: bool, expanded: bool, cx: &App) -> Option<&'static str> {
    let active = active_theme()?;
    let builtin = builtin_theme()?;
    let dark = Theme::global(cx).is_dark();
    lookup_path(active, builtin, name, is_dir, expanded, dark)
}

#[must_use]
pub fn theme_icon_bytes(asset_path: &str, cx: &App) -> Option<Vec<u8>> {
    cx.asset_source()
        .load(asset_path)
        .ok()
        .flatten()
        .map(|bytes| bytes.into_owned())
}

pub fn theme_file_icon(
    asset_path: &str,
    side: impl Into<gpui_kit::Length>,
    cx: &App,
) -> Option<AnyElement> {
    theme_icon_bytes(asset_path, cx)?;
    let side: gpui_kit::Length = side.into();
    Some(
        gpui_kit::img(gpui_kit::ImageSource::Resource(
            gpui_kit::Resource::Embedded(SharedString::from(asset_path.to_string())),
        ))
        .flex_shrink_0()
        .w(side)
        .h(side)
        .into_any_element(),
    )
}

fn definitions(value: Option<&serde_json::Value>, root: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(object) = value.and_then(serde_json::Value::as_object) else {
        return map;
    };
    for (key, value) in object {
        if let Some(text) = value.as_str() {
            insert_definition(&mut map, key, text, root);
            continue;
        }
        let Some(group) = value.as_object() else {
            continue;
        };
        for (id, definition) in group {
            if let Some(text) = definition.as_str() {
                insert_definition(&mut map, id, text, root);
            }
        }
    }
    map
}

fn insert_definition(map: &mut HashMap<String, String>, id: &str, text: &str, root: &str) {
    map.insert(
        id.to_string(),
        if text.starts_with('<') {
            text.to_string()
        } else {
            format!("{root}{}", text.trim_start_matches("./"))
        },
    );
}

fn definition_weight(entry: &serde_json::Value) -> usize {
    entry
        .get("iconDefinitions")
        .and_then(serde_json::Value::as_object)
        .map(|defs| {
            defs.values()
                .map(|value| match value.as_object() {
                    Some(group) => group.values().filter(|leaf| leaf.is_string()).count(),
                    None => usize::from(value.is_string()),
                })
                .sum()
        })
        .unwrap_or(0)
}

fn string_map(value: Option<&serde_json::Value>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Some(object) = value.and_then(serde_json::Value::as_object) else {
        return map;
    };
    for (key, value) in object {
        if key.starts_with(SEMANTIC_PREFIX) {
            continue;
        }
        if let Some(text) = value.as_str() {
            map.insert(
                key.to_ascii_lowercase(),
                text.trim_start_matches("./").to_string(),
            );
        }
    }
    map
}

fn extension_candidates(lower_name: &str) -> Vec<&str> {
    if lower_name.starts_with('.') || !lower_name.contains('.') {
        return Vec::new();
    }
    lower_name
        .match_indices('.')
        .map(|(index, _)| &lower_name[index..])
        .collect()
}

fn lookup_mapped<'a>(key: &str, table: &'a HashMap<String, String>) -> Option<&'a str> {
    table.get(key).map(String::as_str)
}

#[must_use]
pub fn lucide_fallback(name: &str) -> IconName {
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());

    match extension.as_deref() {
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "svg" | "bmp" | "avif") => {
            IconName::Image
        }
        Some("json" | "jsonc") => IconName::FileBraces,
        Some("md" | "markdown" | "txt" | "rst" | "adoc") => IconName::FileText,
        Some("toml" | "yaml" | "yml" | "ini" | "conf" | "cfg" | "properties" | "editorconfig") => {
            IconName::Settings
        }
        Some("lock") => IconName::Package,
        Some("sh" | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "bat" | "cmd") => IconName::Terminal,
        Some(
            "rs" | "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" | "py" | "java"
            | "kt" | "kts" | "swift" | "go" | "c" | "h" | "cc" | "cpp" | "hpp" | "cs" | "rb"
            | "php" | "vue" | "svelte" | "sql" | "css" | "scss" | "less" | "html" | "xml"
            | "gradle" | "lua" | "dart" | "scala" | "ex" | "exs" | "dockerfile",
        ) => IconName::FileCode,
        _ => IconName::File,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn relative(path: Option<&str>) -> Option<&str> {
        path.map(|path| path.trim_start_matches("icons/idea/"))
    }

    fn idea_path(name: &str, is_dir: bool, expanded: bool, dark: bool) -> Option<&'static str> {
        let builtin = builtin_theme().expect("idea theme must parse");
        lookup_path(builtin, builtin, name, is_dir, expanded, dark)
    }

    fn partial_pack() -> FileIcons {
        FileIcons::from_json(
            "demo",
            r#"{
              "icons": [{
                "defaultFile": "demo-file",
                "defaultFolder": "demo-folder",
                "iconDefinitions": {
                  "demo-file": "./icons/file.svg",
                  "demo-folder": "./icons/folder.svg",
                  "demo-src": "./icons/src.svg"
                },
                "folders": { "src": "demo-src" }
              }]
            }"#,
        )
        .expect("synthetic pack must parse")
    }

    #[test]
    fn active_theme_parses() {
        let theme = active_theme().expect("idea theme must parse");
        assert_eq!(theme.root(), "icons/idea/");
        assert_eq!(theme.definition_count(), 128);
    }

    #[test]
    fn idea_theme_matches_the_documented_files() {
        for (name, dark, light) in [
            (
                ".gitignore",
                "icons/files/gitignore.svg",
                "icons/files/gitignore.svg",
            ),
            (
                "pom.xml",
                "icons/files/maven_dark.svg",
                "icons/files/maven.svg",
            ),
            (
                "README.md",
                "icons/files/markdown_dark.svg",
                "icons/files/markdown.svg",
            ),
            (
                "A.java",
                "icons/files/java_dark.svg",
                "icons/files/java.svg",
            ),
            ("x.yml", "icons/files/yaml_dark.svg", "icons/files/yaml.svg"),
        ] {
            assert_eq!(
                relative(idea_path(name, false, false, true)),
                Some(dark),
                "dark: {name}"
            );
            assert_eq!(
                relative(idea_path(name, false, false, false)),
                Some(light),
                "light: {name}"
            );
        }
    }

    #[test]
    fn unknown_file_falls_back_to_default_file() {
        assert_eq!(
            relative(idea_path("notes.unknownext", false, false, true)),
            Some("icons/files/text_dark.svg"),
        );
        assert_eq!(
            relative(idea_path("no-extension-at-all", false, false, true)),
            Some("icons/files/text_dark.svg"),
        );
    }

    #[test]
    fn multi_dot_names_use_the_longest_suffix_first() {
        assert_eq!(
            relative(idea_path("archive.tar.gz", false, false, true)),
            Some("icons/files/archive_dark.svg"),
        );
    }

    #[test]
    fn folders_use_default_folder_icon() {
        assert_eq!(
            relative(idea_path("src", true, false, true)),
            Some("icons/folders/folder_dark.svg"),
        );
        assert_eq!(
            relative(idea_path("src", true, true, false)),
            Some("icons/folders/folder.svg"),
        );
    }

    #[test]
    fn missing_entries_fall_back_to_the_builtin_pack() {
        let builtin = builtin_theme().expect("idea theme must parse");
        let pack = partial_pack();

        assert_eq!(
            lookup_path(&pack, builtin, "src", true, false, true),
            Some("icons/demo/icons/src.svg"),
        );
        assert_eq!(
            lookup_path(&pack, builtin, "A.java", false, false, true),
            Some("icons/idea/icons/files/java_dark.svg"),
        );
        assert_eq!(
            lookup_path(&pack, builtin, "notes.unknownext", false, false, true),
            Some("icons/demo/icons/file.svg"),
        );
        assert_eq!(
            lookup_path(&pack, builtin, "whatever", true, false, true),
            Some("icons/demo/icons/folder.svg"),
        );
    }

    #[test]
    fn every_definition_exists_in_the_assets_tree() {
        let theme = active_theme().expect("idea theme must parse");
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let missing = |table: &HashMap<String, String>| -> Vec<String> {
            table
                .values()
                .filter(|path| !assets.join(path).exists())
                .cloned()
                .collect()
        };
        assert!(
            missing(&theme.dark).is_empty(),
            "dark definitions without a file: {:?}",
            missing(&theme.dark),
        );
        assert!(
            missing(&theme.light).is_empty(),
            "light definitions without a file: {:?}",
            missing(&theme.light),
        );
    }

    #[test]
    fn lucide_fallback_covers_the_documented_cases() {
        assert_eq!(lucide_fallback("data.json"), IconName::FileBraces);
        assert_eq!(lucide_fallback("x.yml"), IconName::Settings);
        assert_eq!(lucide_fallback("README.md"), IconName::FileText);
        assert_eq!(lucide_fallback("A.java"), IconName::FileCode);
        assert_eq!(lucide_fallback("run.sh"), IconName::Terminal);
        assert_eq!(lucide_fallback("Cargo.lock"), IconName::Package);
        assert_eq!(lucide_fallback("logo.png"), IconName::Image);
        assert_eq!(lucide_fallback("notes.unknownext"), IconName::File);
        assert_eq!(lucide_fallback("no-extension"), IconName::File);
    }
}
