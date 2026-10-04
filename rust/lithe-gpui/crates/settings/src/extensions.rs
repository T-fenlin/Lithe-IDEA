use std::path::{Path, PathBuf};

use crate::paths;
use crate::{fonts, theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundledKind {
    Theme,
    Font,
    Languages,
    Icons,
}

pub struct BundledExtension {
    pub id: &'static str,
    pub kind: BundledKind,
    pub files: &'static [(&'static str, &'static [u8])],
    pub icons_package: Option<&'static str>,
    pub languages: &'static [&'static str],
}

pub const BUNDLED_EXTENSIONS: &[BundledExtension] = &[
    BundledExtension {
        id: "lithe.gruvboxTheme",
        kind: BundledKind::Theme,
        icons_package: None,
        languages: &[],
        files: &[(
            "gruvbox.json",
            include_bytes!("../../../extensions/themes/gruvbox.json"),
        )],
    },
    BundledExtension {
        id: "lithe.litheTheme",
        kind: BundledKind::Theme,
        icons_package: None,
        languages: &[],
        files: &[(
            "lithe.json",
            include_bytes!("../../../extensions/themes/lithe.json"),
        )],
    },
    BundledExtension {
        id: "lithe.nordTheme",
        kind: BundledKind::Theme,
        icons_package: None,
        languages: &[],
        files: &[(
            "nord.json",
            include_bytes!("../../../extensions/themes/nord.json"),
        )],
    },
    BundledExtension {
        id: "lithe.oneTheme",
        kind: BundledKind::Theme,
        icons_package: None,
        languages: &[],
        files: &[(
            "one.json",
            include_bytes!("../../../extensions/themes/one.json"),
        )],
    },
    BundledExtension {
        id: "lithe.projectLanguages",
        kind: BundledKind::Languages,
        icons_package: None,
        languages: &["yaml", "toml", "sql", "markdown", "bash", "kotlin", "xml"],
        files: &[],
    },
    BundledExtension {
        id: "lithe.pierreIcons",
        kind: BundledKind::Icons,
        icons_package: Some("pierre"),
        languages: &[],
        files: &[],
    },
    BundledExtension {
        id: "lithe.vscodeTheme",
        kind: BundledKind::Theme,
        icons_package: None,
        languages: &[],
        files: &[(
            "vscode.json",
            include_bytes!("../../../extensions/themes/vscode.json"),
        )],
    },
    BundledExtension {
        id: "lithe.ibmPlexMonoFont",
        kind: BundledKind::Font,
        icons_package: None,
        languages: &[],
        files: &[
            (
                "ibm-plex-mono/IBMPlexMono-Bold.ttf",
                include_bytes!("../../../extensions/fonts/ibm-plex-mono/IBMPlexMono-Bold.ttf"),
            ),
            (
                "ibm-plex-mono/IBMPlexMono-Regular.ttf",
                include_bytes!("../../../extensions/fonts/ibm-plex-mono/IBMPlexMono-Regular.ttf"),
            ),
            (
                "ibm-plex-mono/LICENSE.txt",
                include_bytes!("../../../extensions/fonts/ibm-plex-mono/LICENSE.txt"),
            ),
        ],
    },
    BundledExtension {
        id: "lithe.sourceCodeProFont",
        kind: BundledKind::Font,
        icons_package: None,
        languages: &[],
        files: &[
            (
                "source-code-pro/SourceCodePro-Bold.ttf",
                include_bytes!("../../../extensions/fonts/source-code-pro/SourceCodePro-Bold.ttf"),
            ),
            (
                "source-code-pro/SourceCodePro-Regular.ttf",
                include_bytes!(
                    "../../../extensions/fonts/source-code-pro/SourceCodePro-Regular.ttf"
                ),
            ),
            (
                "source-code-pro/LICENSE.md",
                include_bytes!("../../../extensions/fonts/source-code-pro/LICENSE.md"),
            ),
        ],
    },
];

pub fn bundled_extension(id: &str) -> Option<&'static BundledExtension> {
    BUNDLED_EXTENSIONS.iter().find(|entry| entry.id == id)
}

#[must_use]
pub fn disabled_languages(uninstalled: &[String]) -> Vec<String> {
    let mut disabled: Vec<String> = BUNDLED_EXTENSIONS
        .iter()
        .filter(|entry| entry.kind == BundledKind::Languages)
        .filter(|entry| uninstalled.iter().any(|id| id == entry.id))
        .flat_map(|entry| entry.languages.iter().map(|id| (*id).to_string()))
        .collect();
    disabled.sort();
    disabled.dedup();
    disabled
}

pub fn icons_extension(package: &str) -> Option<&'static BundledExtension> {
    BUNDLED_EXTENSIONS
        .iter()
        .find(|entry| entry.icons_package == Some(package))
}

pub fn store_dir(kind: BundledKind) -> Option<PathBuf> {
    match kind {
        BundledKind::Theme => paths::theme_extensions_dir(),
        BundledKind::Font => paths::font_extensions_dir(),
        BundledKind::Icons | BundledKind::Languages => None,
    }
}

pub fn mirror_dir(kind: BundledKind) -> Option<PathBuf> {
    match kind {
        BundledKind::Theme => paths::themes_dir(),
        BundledKind::Font | BundledKind::Icons | BundledKind::Languages => None,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub created: Vec<String>,
    pub kept: Vec<String>,
    pub failed: Vec<String>,
    pub removed: Vec<String>,
}

impl SyncReport {
    pub fn summary(&self) -> String {
        format!(
            "created={} kept={} failed={} removed={}",
            self.created.len(),
            self.kept.len(),
            self.failed.len(),
            self.removed.len()
        )
    }

    fn merge(&mut self, other: SyncReport) {
        self.created.extend(other.created);
        self.kept.extend(other.kept);
        self.failed.extend(other.failed);
        self.removed.extend(other.removed);
    }
}

pub fn prepare_bundled_extensions(uninstalled: &[String]) -> SyncReport {
    let mut report = SyncReport::default();
    if let (Some(store_dir), Some(mirror_dir)) = (
        store_dir(BundledKind::Theme),
        mirror_dir(BundledKind::Theme),
    ) {
        let migrated = theme::migrate_legacy_theme_files(&mirror_dir, &store_dir);
        report.removed.extend(migrated.removed);
        report.failed.extend(migrated.failed);
    }
    report.merge(sync_bundled_extensions(uninstalled));
    report
}

pub fn sync_bundled_extensions(uninstalled: &[String]) -> SyncReport {
    let mut report = SyncReport::default();
    for kind in [
        BundledKind::Theme,
        BundledKind::Font,
        BundledKind::Languages,
    ] {
        let (Some(store), mirror) = (store_dir(kind), mirror_dir(kind)) else {
            eprintln!("S1_EXTENSION sync_skipped reason=no_dirs kind={kind:?}");
            continue;
        };
        report.merge(sync_kind_into(kind, &store, mirror.as_deref(), uninstalled));
    }
    report
}

pub fn sync_kind_into(
    kind: BundledKind,
    store_dir: &Path,
    mirror_dir: Option<&Path>,
    uninstalled: &[String],
) -> SyncReport {
    let mut report = SyncReport::default();
    if let Err(error) = std::fs::create_dir_all(store_dir) {
        eprintln!(
            "S1_EXTENSION sync_failed dir={} error={error}",
            store_dir.display()
        );
        report.failed.extend(
            BUNDLED_EXTENSIONS
                .iter()
                .filter(|entry| entry.kind == kind)
                .map(|entry| entry.id.to_string()),
        );
        return report;
    }
    if let Some(mirror) = mirror_dir {
        if let Err(error) = std::fs::create_dir_all(mirror) {
            eprintln!(
                "S1_EXTENSION sync_failed dir={} error={error}",
                mirror.display()
            );
            report.failed.extend(
                BUNDLED_EXTENSIONS
                    .iter()
                    .filter(|entry| entry.kind == kind)
                    .map(|entry| entry.id.to_string()),
            );
            return report;
        }
    }

    for extension in BUNDLED_EXTENSIONS.iter().filter(|entry| entry.kind == kind) {
        if uninstalled.iter().any(|value| value == extension.id) {
            remove_files(extension, store_dir, mirror_dir, &mut report);
            continue;
        }
        for (relative, content) in extension.files {
            let store_path = store_dir.join(relative);
            if let Some(parent) = store_path.parent() {
                if let Err(error) = std::fs::create_dir_all(parent) {
                    eprintln!(
                        "S1_EXTENSION sync_failed dir={} error={error}",
                        parent.display()
                    );
                    report.failed.push(relative.to_string());
                    continue;
                }
            }
            if !store_path.exists() {
                match write_atomically(&store_path, content) {
                    Ok(()) => report.created.push((*relative).to_string()),
                    Err(error) => {
                        eprintln!(
                            "S1_EXTENSION sync_write_failed path={} error={error}",
                            store_path.display()
                        );
                        report.failed.push((*relative).to_string());
                        continue;
                    }
                }
            } else {
                report.kept.push((*relative).to_string());
            }
            if let Some(mirror) = mirror_dir {
                let name = Path::new(relative)
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| (*relative).to_string());
                let bytes = std::fs::read(&store_path).unwrap_or_else(|_| content.to_vec());
                mirror_if_needed(&mirror.join(&name), &bytes, &name, &mut report);
            }
        }
    }
    report
}

fn remove_files(
    extension: &BundledExtension,
    store_dir: &Path,
    mirror_dir: Option<&Path>,
    report: &mut SyncReport,
) {
    let mut directories: Vec<PathBuf> = Vec::new();
    for (relative, _) in extension.files {
        let store_path = store_dir.join(relative);
        directories.push(store_path.clone());
        remove_file_if_present(&store_path, relative, report);
        if let Some(mirror) = mirror_dir {
            let name = Path::new(relative)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| (*relative).to_string());
            remove_file_if_present(&mirror.join(&name), &name, report);
        }
    }
    for path in directories {
        if let Some(parent) = path.parent() {
            if parent != store_dir {
                let _ = std::fs::remove_dir(parent);
            }
        }
    }
}

fn remove_file_if_present(path: &Path, label: &str, report: &mut SyncReport) {
    match std::fs::remove_file(path) {
        Ok(()) => report.removed.push(label.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            eprintln!(
                "S1_EXTENSION sync_remove_failed path={} error={error}",
                path.display()
            );
            report.failed.push(label.to_string());
        }
    }
}

fn mirror_if_needed(load_path: &Path, bytes: &[u8], label: &str, report: &mut SyncReport) {
    let differs = std::fs::read(load_path)
        .map(|current| current != bytes)
        .unwrap_or(true);
    if !differs {
        return;
    }
    match write_atomically(load_path, bytes) {
        Ok(()) => report.created.push(format!("{label} (mirror)")),
        Err(error) => {
            eprintln!(
                "S1_EXTENSION sync_mirror_failed path={} error={error}",
                load_path.display()
            );
            report.failed.push(label.to_string());
        }
    }
}

pub fn uninstall(id: &str) -> Result<(), String> {
    let extension = bundled_extension(id).ok_or_else(|| format!("未知捆绑扩展：{id}"))?;
    let store = store_dir(extension.kind).ok_or_else(|| "无法推导扩展目录".to_string())?;
    let mirror = mirror_dir(extension.kind);
    uninstall_into(&store, mirror.as_deref(), id)
}

pub fn uninstall_into(store_dir: &Path, mirror_dir: Option<&Path>, id: &str) -> Result<(), String> {
    let extension = bundled_extension(id).ok_or_else(|| format!("未知捆绑扩展：{id}"))?;
    let mut report = SyncReport::default();
    remove_files(extension, store_dir, mirror_dir, &mut report);
    if !report.failed.is_empty() {
        return Err(format!("删除失败：{:?}", report.failed));
    }
    println!(
        "S1_EXTENSION uninstalled id={id} removed={}",
        report.removed.len()
    );
    Ok(())
}

pub fn reinstall(id: &str) -> Result<(), String> {
    let extension = bundled_extension(id).ok_or_else(|| format!("未知捆绑扩展：{id}"))?;
    let store = store_dir(extension.kind).ok_or_else(|| "无法推导扩展目录".to_string())?;
    let mirror = mirror_dir(extension.kind);
    reinstall_into(&store, mirror.as_deref(), id)
}

pub fn reinstall_into(store_dir: &Path, mirror_dir: Option<&Path>, id: &str) -> Result<(), String> {
    let extension = bundled_extension(id).ok_or_else(|| format!("未知捆绑扩展：{id}"))?;
    for (relative, content) in extension.files {
        let store_path = store_dir.join(relative);
        if let Some(parent) = store_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("建目录 {} 失败：{error}", parent.display()))?;
        }
        write_atomically(&store_path, content)
            .map_err(|error| format!("写 {} 失败：{error}", store_path.display()))?;
        if let Some(mirror) = mirror_dir {
            let name = Path::new(relative)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| (*relative).to_string());
            write_atomically(&mirror.join(&name), content)
                .map_err(|error| format!("写 {} 失败：{error}", mirror.join(&name).display()))?;
        }
    }
    println!("S1_EXTENSION reinstalled id={id}");
    Ok(())
}

fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = crate::persistence::tmp_path(path);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

pub fn font_extension_for_family(family: &str) -> Option<&'static BundledExtension> {
    BUNDLED_EXTENSIONS
        .iter()
        .find(|entry| entry.kind == BundledKind::Font && fonts::family_of(entry) == Some(family))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_well_formed() {
        let mut ids: Vec<&str> = BUNDLED_EXTENSIONS.iter().map(|entry| entry.id).collect();
        let unique = {
            let mut sorted = ids.clone();
            sorted.sort_unstable();
            sorted.dedup();
            sorted
        };
        ids.sort_unstable();
        assert_eq!(unique, ids, "捆绑扩展 id 不能重复");
        for entry in BUNDLED_EXTENSIONS {
            if matches!(entry.kind, BundledKind::Icons | BundledKind::Languages) {
                assert!(
                    entry.files.is_empty(),
                    "{} 不需要落盘文件（随宿主编译）",
                    entry.id
                );
                continue;
            }
            assert!(!entry.files.is_empty(), "{} 没有文件", entry.id);
            for (relative, bytes) in entry.files {
                assert!(
                    !relative.starts_with('/') && !relative.contains(".."),
                    "{} 的相对路径越界：{relative}",
                    entry.id
                );
                assert!(!bytes.is_empty(), "{} 的 {relative} 是空内容", entry.id);
            }
        }
    }

    #[test]
    fn language_packages_declare_their_languages() {
        let mut checked = 0;
        for entry in BUNDLED_EXTENSIONS
            .iter()
            .filter(|entry| entry.kind == BundledKind::Languages)
        {
            assert!(
                !entry.languages.is_empty(),
                "{} 是语言包，必须声明它贡献的语言 id",
                entry.id
            );
            let mut sorted: Vec<&str> = entry.languages.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(
                sorted.len(),
                entry.languages.len(),
                "{} 的语言 id 必须唯一",
                entry.id
            );
            checked += 1;
        }
        assert!(checked > 0, "至少要有一个语言包");
        let entry = BUNDLED_EXTENSIONS
            .iter()
            .find(|entry| entry.kind == BundledKind::Languages)
            .expect("有语言包");
        assert!(
            disabled_languages(&[]).is_empty(),
            "没卸载就不该禁用任何语言"
        );
        assert_eq!(
            disabled_languages(&[entry.id.to_string()]).len(),
            entry.languages.len(),
            "卸载语言包 → 它声明的语言全部禁用"
        );
    }

    #[test]
    fn icons_packages_have_a_real_manifest() {
        let dir = crate::paths::repository_icons_dir();
        let mut checked = 0;
        for entry in BUNDLED_EXTENSIONS
            .iter()
            .filter(|entry| entry.kind == BundledKind::Icons)
        {
            let package = entry
                .icons_package
                .unwrap_or_else(|| panic!("{} 没有包名", entry.id));
            let manifest = dir.join(package).join("extension.json");
            let text = std::fs::read_to_string(&manifest).unwrap_or_else(|error| {
                panic!("{}: {} 读不到：{error}", entry.id, manifest.display())
            });
            assert!(
                text.contains("iconDefinitions"),
                "{}: {} 不像一份图标主题清单",
                entry.id,
                manifest.display()
            );
            assert_eq!(
                icons_extension(package).map(|found| found.id),
                Some(entry.id)
            );
            checked += 1;
        }
        assert!(checked > 0, "至少要有一个捆绑图标集");
    }

    #[test]
    fn theme_entries_match_the_repository_files() {
        let dir = paths::repository_theme_extensions_dir();
        for entry in BUNDLED_EXTENSIONS
            .iter()
            .filter(|entry| entry.kind == BundledKind::Theme)
        {
            for (relative, bytes) in entry.files {
                let on_disk = std::fs::read(dir.join(relative))
                    .unwrap_or_else(|error| panic!("{}: {relative} 读不到：{error}", entry.id));
                assert_eq!(
                    *bytes, on_disk,
                    "{} 的 {relative} 与仓库文件不一致",
                    entry.id
                );
            }
        }
    }

    #[test]
    fn font_entries_match_the_repository_files_and_ship_a_license() {
        let dir = paths::repository_font_extensions_dir();
        let fonts_in_registry: Vec<&BundledExtension> = BUNDLED_EXTENSIONS
            .iter()
            .filter(|entry| entry.kind == BundledKind::Font)
            .collect();
        assert!(
            (1..=2).contains(&fonts_in_registry.len()),
            "字体扩展保持一两个即可，当前 {}",
            fonts_in_registry.len()
        );
        for entry in fonts_in_registry {
            let mut has_ttf = false;
            let mut has_license = false;
            for (relative, bytes) in entry.files {
                let on_disk = std::fs::read(dir.join(relative))
                    .unwrap_or_else(|error| panic!("{}: {relative} 读不到：{error}", entry.id));
                assert_eq!(
                    *bytes, on_disk,
                    "{} 的 {relative} 与仓库文件不一致",
                    entry.id
                );
                if relative.ends_with(".ttf") {
                    has_ttf = true;
                    assert!(
                        bytes.starts_with(&[0x00, 0x01, 0x00, 0x00])
                            || bytes.starts_with(b"true")
                            || bytes.starts_with(b"OTTO"),
                        "{} 的 {relative} 不是字体文件",
                        entry.id
                    );
                }
                if relative.ends_with("LICENSE.txt") || relative.ends_with("LICENSE.md") {
                    has_license = true;
                }
            }
            assert!(has_ttf, "{} 没有字体文件", entry.id);
            assert!(has_license, "{} 没有随包许可文件", entry.id);
        }
    }

    #[test]
    fn sync_installs_uninstalls_and_restores_both_surfaces() {
        let dir = TempDir::new("ext-sync");
        let store = dir.path().join("extensions/themes");
        let mirror = dir.path().join("themes");

        let report = sync_kind_into(BundledKind::Theme, &store, Some(&mirror), &[]);
        assert!(report.failed.is_empty(), "{report:?}");
        let theme_entries: Vec<&BundledExtension> = BUNDLED_EXTENSIONS
            .iter()
            .filter(|entry| entry.kind == BundledKind::Theme)
            .collect();
        assert_eq!(report.created.len(), theme_entries.len() * 2, "{report:?}");

        let target = theme_entries[0];
        let relative = target.files[0].0;
        let report = sync_kind_into(
            BundledKind::Theme,
            &store,
            Some(&mirror),
            &[target.id.to_string()],
        );
        assert!(!store.join(relative).exists(), "管理面不该有已卸载的包");
        assert!(!mirror.join(relative).exists(), "装载面不该有已卸载的镜像");
        assert_eq!(report.removed.len(), 2, "{report:?}");

        sync_kind_into(BundledKind::Theme, &store, Some(&mirror), &[]);
        assert_eq!(
            std::fs::read(store.join(relative)).expect("管理面读不到"),
            target.files[0].1
        );
        assert_eq!(
            std::fs::read(mirror.join(relative)).expect("装载面读不到"),
            target.files[0].1
        );
    }

    #[test]
    fn font_package_syncs_into_its_own_directory_and_cleans_up() {
        let dir = TempDir::new("font-sync");
        let store = dir.path().join("extensions/fonts");
        let report = sync_kind_into(BundledKind::Font, &store, None, &[]);
        assert!(report.failed.is_empty(), "{report:?}");
        let fonts_in_registry: Vec<&BundledExtension> = BUNDLED_EXTENSIONS
            .iter()
            .filter(|entry| entry.kind == BundledKind::Font)
            .collect();
        let target = fonts_in_registry[0];
        for (relative, content) in target.files {
            assert_eq!(
                std::fs::read(store.join(relative)).expect("字体包读不到"),
                *content
            );
        }

        sync_kind_into(BundledKind::Font, &store, None, &[target.id.to_string()]);
        for (relative, _) in target.files {
            assert!(!store.join(relative).exists(), "{relative} 应被删除");
        }
        let package_dir = store.join(Path::new(target.files[0].0).parent().expect("有父目录"));
        assert!(!package_dir.exists(), "空目录要一并收掉：{package_dir:?}");
    }

    #[test]
    fn mirror_follows_the_store_copy() {
        let dir = TempDir::new("ext-mirror");
        let store = dir.path().join("extensions/themes");
        let mirror = dir.path().join("themes");
        sync_kind_into(BundledKind::Theme, &store, Some(&mirror), &[]);

        let target = BUNDLED_EXTENSIONS
            .iter()
            .find(|entry| entry.kind == BundledKind::Theme)
            .expect("至少一个主题包");
        let relative = target.files[0].0;
        let edited = b"{\"name\":\"Mine\",\"themes\":[]}".to_vec();
        std::fs::write(store.join(relative), &edited).expect("改写管理面失败");

        let report = sync_kind_into(BundledKind::Theme, &store, Some(&mirror), &[]);
        assert!(report.failed.is_empty(), "{report:?}");
        assert_eq!(
            std::fs::read(mirror.join(relative)).expect("装载面读不到"),
            edited,
            "装载面必须跟随管理面（用户可编辑的那一面）"
        );
    }

    #[test]
    fn uninstall_rejects_unknown_ids() {
        let dir = TempDir::new("ext-unknown");
        let store = dir.path().join("extensions/themes");
        sync_kind_into(BundledKind::Font, &store, None, &[]);
        assert!(uninstall_into(&store, None, "lithe.notAnExtension").is_err());
        assert!(reinstall_into(&store, None, "lithe.notAnExtension").is_err());
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(case: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("lithe-extensions-{case}-{}", std::process::id()));
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
}
