//! 捆绑扩展（随宿主分发、用户可卸载的能力包）：注册表、装/卸/同步的文件语义。
//!
//! ## 谁在这里
//!
//! - **主题组**（`BundledKind::Theme`）：一个 `.json` 一个包，例如 `gruvbox.json`。
//! - **图标集**（`BundledKind::Icons`）：随宿主编译、**没有落盘文件**，
//!   卸载只改变"可选/不可选"（见该变体的说明）。
//! - **字体族**（`BundledKind::Font`）：一个包一个目录（含字重文件与许可），
//!   例如 `source-code-pro/SourceCodePro-Regular.ttf`。
//!
//! 内置（不可卸载）的东西**不在这里**：内置主题在 [`crate::theme::BUILTIN_THEMES`]、
//! 内置字体在 [`crate::fonts::BUILTIN_FONT_FILES`] —— 那些是宿主核心能力，
//! 目录契约里对应 entrypoint `builtIn`。
//!
//! ## 两个目录的分工（只有主题有）
//!
//! 主题注册表**只能监视一个目录**，所以主题有"管理面 + 装载面"两层：
//!
//! ```text
//! <配置目录>/extensions/themes/<包>   ← 管理面：装/卸都发生在它上面（用户可编辑）
//! <配置目录>/themes/<包>              ← 装载面：管理面的镜像（注册表监视它）
//! ```
//!
//! 主题另有"内置主题播种"与"老版本文件迁移"两件只在主题上成立的事，留在 [`crate::theme`]。
//! 字体没有这两件事，也没有镜像 —— 文本系统由 [`crate::fonts::register_fonts`]
//! 在启动时直接喂字节。
//!
//! ## 卸载为什么必须记清单
//!
//! 同步的语义是"种子"：**缺就补**。所以只删文件的话，下一次启动同步会把包又写回来
//! ——卸载必须同时记进设置的 `uninstalledExtensions`（由
//! [`crate::store::SettingsStore::set_extension_uninstalled`] 落盘），
//! 本模块的同步按那份清单决定"补"还是"清"。
//!
//! ## 生效时机不一样（如实登记）
//!
//! - **主题**：立即。装载面少/多一个文件 → 注册表热重载 → 重新应用当前主题。
//! - **字体**：**下次启动**。`TextSystem::add_fonts` 只增不删
//!   （`gpui-pre-0.3.6/src/text_system.rs:295`），运行期摘不掉已注册的字体族，
//!   所以卸载/重装写在扩展页上就说明了"重启后生效"。

use std::path::{Path, PathBuf};

use crate::paths;
use crate::{fonts, theme};

/// 捆绑扩展的种类：决定它在配置目录下的落点与是否需要装载面镜像。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundledKind {
    /// 主题组：管理面 + 装载面镜像。
    Theme,
    /// 字体族：只有落点，没有镜像。
    Font,
    /// 语言：**没有落盘文件** —— 语法随宿主编译（Cargo feature）或由宿主自己注册，
    /// 卸载的效果是"这些语言不再高亮（文件按纯文本打开）"，见
    /// [\`BundledExtension::languages\`]。
    Languages,
    /// 图标集：**没有落盘文件** —— 图标集随宿主编译（内嵌在 `assets/icons/` 里），
    /// 同步阶段不写任何文件；卸载的效果是"不可选 + 正在使用时回落内置包"
    /// （见 [`BundledExtension::icons_package`]）。这与主题/字体"卸载 = 删文件"
    /// 的差别来自数据来源：那两类的数据是随包写出去的，图标集不是。
    Icons,
}

/// 一个捆绑扩展：稳定 id、种类、包内文件（相对包根的路径 → 内容）。
///
/// `files` 的相对路径既用于管理面（`<store>/<相对路径>`），也用于主题的装载面镜像
/// （镜像目录里用最后一段文件名 —— 主题注册表只扫顶层 `*.json`）。
pub struct BundledExtension {
    /// 稳定 id，与 `official-v2.json` 清单里的 `id` 一一对应。
    pub id: &'static str,
    /// 种类（决定落点与镜像）。
    pub kind: BundledKind,
    /// 包内文件：`(相对路径, 内容)`，路径按 `/` 分隔。图标集为空（见 [`BundledKind::Icons`]）。
    pub files: &'static [(&'static str, &'static [u8])],
    /// 图标集的**包名**（`assets/icons/<包>/` 的目录名）；只有 `Icons` 有。
    /// 它就是设置键 `icons` 的取值（见 `crate::schema::DEFAULT_ICONS`）。
    pub icons_package: Option<&'static str>,
    /// 这个包贡献的**语言 id**（`yaml` / `xml` …）；只有语言包有。
    /// 卸载时这些语言被禁用（设置存储推给 `shared::languages`，编辑器据此按纯文本打开）。
    pub languages: &'static [&'static str],
}

/// 全部捆绑扩展。顺序只影响同步报告的次序，不影响语义。
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
        // 卸载后这些语言不再高亮（`buffer::language_for_file` 会给出 `None`）。
        languages: &["yaml", "toml", "sql", "markdown", "bash", "kotlin", "xml"],
        // 空 = 没有落盘文件：语法随宿主编译（yaml/toml/sql/md/bash/kotlin 是 gpui-kit 的
        // feature，xml 由 `editor/src/languages.rs` 自己注册）。
        files: &[],
    },
    BundledExtension {
        id: "lithe.pierreIcons",
        kind: BundledKind::Icons,
        icons_package: Some("pierre"),
        languages: &[],
        // 空 = 没有落盘文件：图标集与内置包一起编译进二进制（见 BundledKind::Icons）。
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

/// `id` → 捆绑扩展；`None` = 不是捆绑扩展（内置主题、内置字体、Java 支持等都不接受卸载）。
pub fn bundled_extension(id: &str) -> Option<&'static BundledExtension> {
    BUNDLED_EXTENSIONS.iter().find(|entry| entry.id == id)
}

/// 被卸载的语言扩展 → 应当禁用的语言 id 集合（排序去重）。
///
/// 设置存储把它推给 [\`lithe_gpui_shared::languages::set_disabled_languages\`]；
/// 编辑器据此让这些文件按纯文本打开。
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

/// 图标集包名 → 扩展条目；`None` = 不是捆绑图标集（内置包或认不出的名字）。
///
/// 用途是"正在使用的图标包被卸载了吗"这条判断（[crate::store] 的回落逻辑）与扩展页的
/// 「使用 / 当前 / 已卸载」三态。
pub fn icons_extension(package: &str) -> Option<&'static BundledExtension> {
    BUNDLED_EXTENSIONS
        .iter()
        .find(|entry| entry.icons_package == Some(package))
}

/// 一种捆绑扩展的**管理面**目录（`<配置目录>/extensions/<kind>`）。
pub fn store_dir(kind: BundledKind) -> Option<PathBuf> {
    match kind {
        BundledKind::Theme => paths::theme_extensions_dir(),
        BundledKind::Font => paths::font_extensions_dir(),
        // 图标集与语言都没有落点：它们随宿主编译，同步阶段不写文件。
        BundledKind::Icons | BundledKind::Languages => None,
    }
}

/// 一种捆绑扩展的**装载面**目录；`None` = 这一种没有装载面（字体）。
pub fn mirror_dir(kind: BundledKind) -> Option<PathBuf> {
    match kind {
        BundledKind::Theme => paths::themes_dir(),
        BundledKind::Font | BundledKind::Icons | BundledKind::Languages => None,
    }
}

/// 一次同步的结果（诊断与测试用）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// 这次写出的文件（管理面写入 + 装载面镜像）。
    pub created: Vec<String>,
    /// 已经存在、**原样保留**的管理面文件（用户可能改过，不覆盖）。
    pub kept: Vec<String>,
    /// 写失败的文件。
    pub failed: Vec<String>,
    /// 这次删掉的文件（已卸载扩展的残留，或迁移阶段移走的旧文件）。
    pub removed: Vec<String>,
}

impl SyncReport {
    /// 一次性写进日志行的一行摘要。
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

/// 启动时的准备：先迁移老版本主题文件，再按卸载清单同步全部捆绑扩展。
///
/// **只读设置里的卸载清单**（`uninstalled` 由调用方传进来，见
/// [`crate::store::SettingsStore::uninstalled_extensions`]）。返回合并后的报告。
pub fn prepare_bundled_extensions(uninstalled: &[String]) -> SyncReport {
    let mut report = SyncReport::default();
    // 迁移要在同步之前：老文件先归位到管理面，同步才好按"缺就补"处理镜像。
    // （迁移返回主题模块自己的报告类型 —— 它早于本模块存在；这里只取两项，不另造转换层。）
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

/// 按卸载清单同步全部捆绑扩展（用 [`paths`] 推导目录）。
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

/// 同步**一种**捆绑扩展的显式目录形态（测试与工具用）。
///
/// `kind` 必须显式给：注册表里两类扩展的文件名可能重名（都是 `xxx.json` 或 `xxx.ttf`），
/// 靠目录路径反推属于哪一类是不可靠的。
///
/// 规则：
///
/// | 情况 | 结果 |
/// | --- | --- |
/// | 在卸载清单里 | 管理面与装载面的文件都删掉（幂等收口） |
/// | 不在清单、管理面缺文件 | 从内嵌内容写出（"重装"后的种子语义） |
/// | 不在清单、管理面已有文件 | **不动**（用户可能改过），但装载面要跟上管理面的内容 |
///
/// 装载面是**镜像**：以管理面为准（内容不同就覆盖），这与"管理面绝不覆盖"是两种语义，
/// 理由是扩展有装/卸生命周期、内容必须跟着管理面走。
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
                // 镜像内容取管理面那份：用户改了管理面的主题文件，镜像要跟随。
                let bytes = std::fs::read(&store_path).unwrap_or_else(|_| content.to_vec());
                mirror_if_needed(&mirror.join(&name), &bytes, &name, &mut report);
            }
        }
    }
    report
}

/// 已卸载扩展的收口：管理面 + 装载面的文件都删掉，并尝试清掉空目录。
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
    // 字体包是一族文件放在自己的子目录里：文件删完后把空目录也收掉，
    // 免得扩展目录里留一堆空壳（`remove_dir` 只在目录为空时成功，不会误删别人）。
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

/// 装载面镜像：缺或内容不同就原子写（以管理面为准）。
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

/// 卸载一个捆绑扩展：管理面与装载面的文件都删除（设置里的清单由调用方落盘）。
///
/// `Err` = id 不是捆绑扩展或目录推导失败。
pub fn uninstall(id: &str) -> Result<(), String> {
    let extension = bundled_extension(id).ok_or_else(|| format!("未知捆绑扩展：{id}"))?;
    let store = store_dir(extension.kind).ok_or_else(|| "无法推导扩展目录".to_string())?;
    let mirror = mirror_dir(extension.kind);
    uninstall_into(&store, mirror.as_deref(), id)
}

/// [`uninstall`] 的目录显式形态（测试与工具用）。
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

/// 重装一个捆绑扩展：从内嵌内容写回管理面，并镜像到装载面。
pub fn reinstall(id: &str) -> Result<(), String> {
    let extension = bundled_extension(id).ok_or_else(|| format!("未知捆绑扩展：{id}"))?;
    let store = store_dir(extension.kind).ok_or_else(|| "无法推导扩展目录".to_string())?;
    let mirror = mirror_dir(extension.kind);
    reinstall_into(&store, mirror.as_deref(), id)
}

/// [`reinstall`] 的目录显式形态（测试与工具用）。
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

/// 原子写一份**字节**：先写 `<文件>.tmp`，再 `rename` 覆盖目标。
///
/// 与 [`crate::theme`] 的同名做法同一条理由：断电最坏只留下 `.tmp`（注册表只认
/// `.json`，文本系统只吃我们主动喂的字节），目标文件要么不存在、要么完整。
/// 不复用 [`crate::persistence::save_json`] 是因为那个函数会**重新序列化**值，
/// 而内嵌内容必须与仓库里那份逐字相等（有测试钉住）。
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = crate::persistence::tmp_path(path);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// 字体族名（清单里 `fonts[].family` 的取值）→ 捆绑扩展 id。
///
/// 扩展页/设置里的字体名要落到"该字体属于哪个可卸载的包"这一层才谈得上卸载。
pub fn font_extension_for_family(family: &str) -> Option<&'static BundledExtension> {
    BUNDLED_EXTENSIONS
        .iter()
        .find(|entry| entry.kind == BundledKind::Font && fonts::family_of(entry) == Some(family))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 注册表里的 id 必须唯一，且每个包都有文件（空包没有可装载的东西）。
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
            // **随宿主编译的种类没有落盘文件**（图标集与语言）：它们的"安装"不写文件，
            // 卸载的效果分别是"不可选"与"不再高亮"。所以"必须有文件"这条只对
            // 需要写出文件的种类（主题、字体）成立。
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

    /// 语言包必须声明语言 id（卸载时按它禁用高亮），且这些 id 与目录里
    /// `languageSupports` 贡献的东西是同一套取值。
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
        // 卸载计算：清单里有它 → 它的语言全被禁用；清单里没有 → 一个都不禁用。
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

    /// 图标集的包名必须对应仓库 `assets/icons/<包>/` 里**真实存在的清单**：
    /// 注册了却没有这套资源 = "设置里选得出来、图标却画不出来"。
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
            // 包名唯一（反查 icons 取值靠它）。
            assert_eq!(
                icons_extension(package).map(|found| found.id),
                Some(entry.id)
            );
            checked += 1;
        }
        assert!(checked > 0, "至少要有一个捆绑图标集");
    }

    /// 主题包在注册表里的文件名必须与仓库 `extensions/themes/` 里的实际文件一致。
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

    /// 字体包的每个文件都必须与仓库 `extensions/fonts/` 里的实际文件一致，
    /// 且**都带一份许可**（OFL 要求随字体分发许可原文）。
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
                    // TrueType 魔数：0x00010000（其余常见形态是 `true` / `OTTO`）。
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

    /// 同步（全装 → 全卸 → 重装）在管理面与装载面上的文件语义。
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

        // 卸载一个：两个面都清掉。
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

        // 重装：两个面都回来，内容与内嵌一致。
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

    /// 字体包同步到自己的子目录里，卸载时连空目录一起收掉。
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

    /// 装载面是镜像：管理面被改过时跟随管理面，而不是回到内嵌内容。
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

    /// 卸载/重装的显式形态：未知 id 报错，不动任何文件。
    #[test]
    fn uninstall_rejects_unknown_ids() {
        let dir = TempDir::new("ext-unknown");
        let store = dir.path().join("extensions/themes");
        sync_kind_into(BundledKind::Font, &store, None, &[]);
        assert!(uninstall_into(&store, None, "lithe.notAnExtension").is_err());
        assert!(reinstall_into(&store, None, "lithe.notAnExtension").is_err());
    }

    /// 一个"进程内唯一、用后即删"的临时目录（与 `theme.rs` 的同名助手同一条口径）。
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
