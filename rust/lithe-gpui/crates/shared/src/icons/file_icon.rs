use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use gpui::{AnyElement, App, AssetSource, IntoElement as _, SharedString, Styled as _};
use lithe_ui::theme::ActiveTheme as _;

use super::AppIcon;

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
    let dark = cx.theme().is_dark();
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
    side: impl Into<gpui::Length>,
    cx: &App,
) -> Option<AnyElement> {
    theme_icon_bytes(asset_path, cx)?;
    let side: gpui::Length = side.into();
    Some(
        gpui::img(gpui::ImageSource::Resource(gpui::Resource::Embedded(
            SharedString::from(asset_path.to_string()),
        )))
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

/// 扩展名 → 我们自己的 SVG 路径（不含点，全小写）。表里的每一条都必须指向本仓库
/// `assets/icons/idea/**` 下**真实存在**的文件，由 `fallback_icons_only_name_our_own_assets` 守着。
const FALLBACK_ICONS: &[(&str, &str)] = &[
    ("png", "icons/idea/fileTypes/image.svg"),
    ("jpg", "icons/idea/fileTypes/image.svg"),
    ("jpeg", "icons/idea/fileTypes/image.svg"),
    ("gif", "icons/idea/fileTypes/image.svg"),
    ("webp", "icons/idea/fileTypes/image.svg"),
    ("ico", "icons/idea/fileTypes/image.svg"),
    ("svg", "icons/idea/fileTypes/image.svg"),
    ("bmp", "icons/idea/fileTypes/image.svg"),
    ("avif", "icons/idea/fileTypes/image.svg"),
    ("json", "icons/idea/fileTypes/json.svg"),
    ("jsonc", "icons/idea/fileTypes/json.svg"),
    ("json5", "icons/idea/fileTypes/json.svg"),
    ("md", "icons/idea/fileTypes/markdown.svg"),
    ("markdown", "icons/idea/fileTypes/markdown.svg"),
    ("mdx", "icons/idea/fileTypes/markdown.svg"),
    ("txt", "icons/idea/fileTypes/text.svg"),
    ("log", "icons/idea/fileTypes/text.svg"),
    ("lock", "icons/idea/fileTypes/text.svg"),
    // 清单里 `.adoc` 也是文本图（那边没有 Markdown 专用名），这里必须跟它一致。
    ("adoc", "icons/idea/fileTypes/text.svg"),
    ("yml", "icons/idea/fileTypes/yaml.svg"),
    ("yaml", "icons/idea/fileTypes/yaml.svg"),
    ("toml", "icons/idea/fileTypes/toml.svg"),
    ("ini", "icons/idea/fileTypes/properties.svg"),
    ("conf", "icons/idea/fileTypes/properties.svg"),
    ("cfg", "icons/idea/fileTypes/properties.svg"),
    ("properties", "icons/idea/fileTypes/properties.svg"),
    ("editorconfig", "icons/idea/fileTypes/editorConfig.svg"),
    ("sh", "icons/idea/fileTypes/shell.svg"),
    ("bash", "icons/idea/fileTypes/shell.svg"),
    ("zsh", "icons/idea/fileTypes/shell.svg"),
    ("fish", "icons/idea/fileTypes/shell.svg"),
    ("ps1", "icons/idea/fileTypes/powershell.svg"),
    ("psm1", "icons/idea/fileTypes/powershell.svg"),
    ("psd1", "icons/idea/fileTypes/powershell.svg"),
    ("bat", "icons/idea/fileTypes/powershell.svg"),
    ("cmd", "icons/idea/fileTypes/powershell.svg"),
    ("py", "icons/idea/fileTypes/python.svg"),
    ("rs", "icons/idea/fileTypes/rust.svg"),
    ("go", "icons/idea/fileTypes/go.svg"),
    // `.kt` 有我们自己的图；`.kts` 不出第二种说法，一律留给清单（那边是 Gradle 脚本）。
    ("kt", "icons/idea/fileTypes/kotlin.svg"),
    ("rb", "icons/idea/fileTypes/ruby.svg"),
    ("php", "icons/idea/fileTypes/php.svg"),
    ("scala", "icons/idea/fileTypes/scala.svg"),
    ("java", "icons/idea/fileTypes/java.svg"),
    ("swift", "icons/idea/fileTypes/swiftLang.svg"),
    ("c", "icons/idea/fileTypes/c.svg"),
    ("h", "icons/idea/fileTypes/h.svg"),
    ("hpp", "icons/idea/fileTypes/h.svg"),
    ("cc", "icons/idea/fileTypes/cpp.svg"),
    ("cpp", "icons/idea/fileTypes/cpp.svg"),
    ("cxx", "icons/idea/fileTypes/cpp.svg"),
    ("cs", "icons/idea/fileTypes/csharp.svg"),
    ("css", "icons/idea/fileTypes/css.svg"),
    ("scss", "icons/idea/fileTypes/css.svg"),
    ("less", "icons/idea/fileTypes/css.svg"),
    ("sass", "icons/idea/fileTypes/css.svg"),
    ("html", "icons/idea/fileTypes/html.svg"),
    ("htm", "icons/idea/fileTypes/html.svg"),
    ("xml", "icons/idea/fileTypes/xml.svg"),
    ("sql", "icons/idea/fileTypes/sql.svg"),
    ("csv", "icons/idea/fileTypes/csv.svg"),
    ("gradle", "icons/idea/fileTypes/gradle.svg"),
    ("groovy", "icons/idea/fileTypes/groovy.svg"),
    ("zip", "icons/idea/fileTypes/archive.svg"),
    ("tar", "icons/idea/fileTypes/archive.svg"),
    ("gz", "icons/idea/fileTypes/archive.svg"),
    ("tgz", "icons/idea/fileTypes/archive.svg"),
    ("7z", "icons/idea/fileTypes/archive.svg"),
    ("rar", "icons/idea/fileTypes/archive.svg"),
    ("bin", "icons/idea/fileTypes/binaryData.svg"),
    ("exe", "icons/idea/fileTypes/binaryData.svg"),
    ("dll", "icons/idea/fileTypes/binaryData.svg"),
    ("so", "icons/idea/fileTypes/binaryData.svg"),
    ("dylib", "icons/idea/fileTypes/binaryData.svg"),
    ("class", "icons/idea/fileTypes/binaryData.svg"),
];

/// 主题清单没命中时，**我们自己**图标集里还给不给得出这张图。
///
/// **口径**（维护者，2026-10-08）：应用图标只用本仓库自己的图标集
/// `rust/lithe-gpui/assets/icons/idea/**`；我们没有那张图就**留空、不画**，不许拿组件库
/// （`lithe-ui`）的图标顶替。
///
/// **查找规则**：只看**扩展名**（完整文件名的真源是主题清单，已由 [`theme_icon_path`] 命中），
/// 命中 [`FALLBACK_ICONS`] 就返回 `icons/idea/**` 下的**我们自己的**路径，否则 `None` ——
/// 调用点据此**留空**（[`FileIcon::render`](crate::icons::FileIcon::render) 会占住同一个
/// 图标位而不画图形）。
///
/// **这张表和主题清单的关系**：清单（`extension.json`）是文件类型图标的真源，本表只登记
/// 「清单整份读不出来」时我们仍然有的图，给清单当第二层兜底。两边都覆盖到的扩展名必须给出
/// 同一条路径，由 `fallback_icons_agree_with_the_theme_manifest` 一条一条守着，免得长出
/// 第二套说法。
///
/// **改口径前的样子**：这个函数原来叫 `lucide_fallback`，按扩展名返回 `lithe_ui::IconName`
/// （png→`Image`、json→`Braces`、md→`FileText`、yml→`Settings`、java→`FileCode`、
/// sh→`Terminal`、lock→`Package`、其余→`File`）—— 那就是「用库图标顶我们的」；
/// 那些分支已全部删除，`.json` 也改回我们自己的 `fileTypes/json.svg`。
#[must_use]
pub fn fallback_icon(name: &str) -> Option<AppIcon> {
    let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
    FALLBACK_ICONS
        .iter()
        .find(|(candidate, _)| *candidate == extension.as_str())
        .map(|(_, path)| AppIcon::same(path))
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

    /// 图标包真源：`assets/icons/idea/extension.json` 的 `iconDefinitions` 条数。
    /// 2026-10-08 从 43 涨到 59：`filenames` 点名的 maven/docker/editorConfig/class/interface/
    /// enum/annotation/record 与磁盘上早就有图、清单却没登记的 python/rust/go/kotlin/ruby/php/
    /// scala/binaryData 一并补了进来。改清单就要同步这个数。
    /// 2026-10-11 涨到 60：登记 `unknown`（`fileTypes/unknown.svg` 是「认不出类型的文件」那张
    /// 图，此前从没进过定义表）。注意它**还没有任何 `fileExtensions`/`filenames` 指向**，
    /// 也就是说现在仍不会显示 —— 真要用起来得动 `defaultFile`，那会改掉无扩展名文件的外观，
    /// 属于设计决定，暂不在此改。
    const MANIFEST_DEFINITIONS: usize = 60;

    #[test]
    fn active_theme_parses() {
        let theme = active_theme().expect("idea theme must parse");
        assert_eq!(theme.root(), "icons/idea/");
        assert_eq!(theme.definition_count(), MANIFEST_DEFINITIONS);
    }

    #[test]
    fn every_definition_reference_resolves() {
        let theme = active_theme().expect("idea theme must parse");
        let root: serde_json::Value =
            serde_json::from_str(BUILTIN_EXTENSION_JSON).expect("manifest must be JSON");
        let block = &root["icons"][0];
        let mut references = 0;
        // 清单里每一条按名字/扩展名点到的 id 都必须真的解析到一条定义。0.3.0 的清单做不到这件事：
        // 43 条定义 + 10 处悬空引用（`pom.xml` 没有 maven 图、`\0lithe:java.class` 没有 class 图），
        // 根因是同步工具的裁剪规则只把 `fileExtensions`/`folders`/默认值算作「被引用」。
        //
        // 注意 `\0lithe:*` 这批语义名：gpui 宿主按 `SEMANTIC_PREFIX` 把它们**跳过**（它们不吃
        // 文件名/扩展名查表），所以这里直接读清单原文核对，不走 [`FileIcons::file_id`]。
        // 它们仍是 Windows 前端 `file-icon-semantics.ts` 的契约，必须继续指向真实定义。
        for table in ["fileExtensions", "filenames", "folders", "expandedFolders"] {
            let entries = block[table]
                .as_object()
                .unwrap_or_else(|| panic!("`{table}` 应当是对象"));
            for (key, id) in entries {
                let id = id.as_str().expect("id 应当是字符串");
                assert!(
                    theme.resolve(id, true).is_some() && theme.resolve(id, false).is_some(),
                    "`{table}` 的 `{key:?}` 指向 `{id}`，但清单里没有这条定义（会悄悄变成默认图）"
                );
                references += 1;
            }
        }
        for key in ["defaultFile", "defaultFolder", "defaultFolderOpen"] {
            let id = block[key].as_str().expect("默认值应当是字符串");
            assert!(
                theme.resolve(id, true).is_some() && theme.resolve(id, false).is_some(),
                "`{key}` = `{id}` 在清单里没有定义"
            );
            references += 1;
        }
        assert!(
            references >= 100,
            "只查到 {references} 条引用，这个检查没在干活"
        );
    }

    #[test]
    fn idea_theme_matches_the_documented_files() {
        for (name, dark, light) in [
            // 我们自己的图；`pom.xml` 走的是清单 `filenames` 里的 maven。
            (
                "pom.xml",
                "legacy/files/maven_dark.svg",
                "legacy/files/maven.svg",
            ),
            ("data.json", "fileTypes/json.svg", "fileTypes/json.svg"),
            (
                "README.md",
                "fileTypes/markdown.svg",
                "fileTypes/markdown.svg",
            ),
            ("A.java", "fileTypes/java.svg", "fileTypes/java.svg"),
            ("x.yml", "fileTypes/yaml.svg", "fileTypes/yaml.svg"),
            ("Main.kt", "fileTypes/kotlin.svg", "fileTypes/kotlin.svg"),
            // 我们自己没有 `.gitignore` 的专用图：落到 `defaultFile`，仍是我们自己的文本图。
            (".gitignore", "fileTypes/text.svg", "fileTypes/text.svg"),
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
            Some("fileTypes/text.svg"),
        );
        assert_eq!(
            relative(idea_path("no-extension-at-all", false, false, true)),
            Some("fileTypes/text.svg"),
        );
    }

    #[test]
    fn multi_dot_names_use_the_longest_suffix_first() {
        assert_eq!(
            relative(idea_path("archive.tar.gz", false, false, true)),
            Some("fileTypes/archive.svg"),
        );
    }

    #[test]
    fn folders_use_default_folder_icon() {
        assert_eq!(
            relative(idea_path("src", true, false, true)),
            Some("nodes/folder.svg"),
        );
        assert_eq!(
            relative(idea_path("src", true, true, false)),
            Some("nodes/folder_light.svg"),
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
            Some("icons/idea/fileTypes/java.svg"),
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

    fn assets_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
    }

    /// 改口径的验收点：兜底图只能是**我们自己的**资源，而且那份资源真的在磁盘上。
    #[test]
    fn fallback_icons_only_name_our_own_assets() {
        let assets = assets_root();
        for (extension, path) in FALLBACK_ICONS {
            assert!(
                path.starts_with("icons/idea/"),
                "`{extension}` 的兜底图 `{path}` 不在我们自己的图标集里"
            );
            assert!(
                assets.join(path).exists(),
                "`{extension}` 的兜底图 `{path}` 在 assets 里不存在（界面上会画空白）"
            );
        }
    }

    /// 回归点：`.json` 曾经被映射成组件库的 `IconName::Braces`（注释理由是「`icons/file-json.svg`
    /// 宿主不提供」）。我们自己的 `fileTypes/json.svg` 一直都在。
    #[test]
    fn fallback_icon_keeps_json_ours() {
        assert_eq!(
            fallback_icon("data.json"),
            Some(AppIcon::same("icons/idea/fileTypes/json.svg")),
        );
        assert_eq!(
            fallback_icon("settings.jsonc"),
            Some(AppIcon::same("icons/idea/fileTypes/json.svg")),
        );
    }

    /// 没有对应图的类型一律 `None`（⇒ 调用点留空），不再回落到组件库图标。
    #[test]
    fn fallback_icon_leaves_unknown_types_empty() {
        for name in [
            "notes.unknownext",
            "no-extension",
            "Makefile",
            "logo.tiff",
            "script.lua",
        ] {
            assert_eq!(fallback_icon(name), None, "`{name}` 应当留空");
        }
    }

    /// 扩展名大小写不敏感（清单那边也是这个口径）。
    #[test]
    fn fallback_icon_ignores_extension_case() {
        assert_eq!(fallback_icon("LOGO.PNG"), fallback_icon("logo.png"));
        assert_eq!(fallback_icon("Main.JAVA"), fallback_icon("Main.java"));
    }

    /// 兜底表和主题清单都覆盖到的扩展名必须给出同一条路径 —— 不允许长出第二套说法。
    #[test]
    fn fallback_icons_agree_with_the_theme_manifest() {
        let theme = active_theme().expect("idea theme must parse");
        let mut overlapping = 0;
        for (extension, path) in FALLBACK_ICONS {
            let probe = format!("probe.{extension}");
            // 清单没按扩展名登记的类型（本表专管的那些缺口）不参与比对。
            let Some(id) = theme.file_id(&probe) else {
                continue;
            };
            overlapping += 1;
            for dark in [true, false] {
                assert_eq!(
                    theme.resolve(id, dark),
                    Some(*path),
                    "`.{extension}` 在清单里是 `{id}`，兜底表给的是 `{path}` —— 两处必须一致"
                );
            }
        }
        assert!(
            overlapping > 20,
            "只比到 {overlapping} 个重叠扩展名，这个检查没在干活"
        );
    }
}
