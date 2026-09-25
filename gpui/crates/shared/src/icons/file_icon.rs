//! 文件类型图标主题的**查找层**：`gpui/assets/icon-themes/**` 的 `extension.json` → SVG 资源路径。
//!
//! 这一层是 Windows 前端 `ThemedFileIcon`（`windows/tauri/src/extensions/icon-themes/components/
//! themed-file-icon.tsx`）在 gpui 侧的等价物。它回答的问题是：
//! **"这个文件名 / 这个目录，在真机当前图标主题下应该画哪张 SVG？"**
//!
//! ## 为什么需要单独一层（而不是像 `icons::idea` 那样做成常量表）
//!
//! `ui-icons/idea/**` 是"一个显示名一张图"，可以直接生成常量表；`icon-themes/**` 是
//! **"按文件名 / 后缀 / 目录名查表"** 的另一套体系（与 VS Code 的 icon theme 同构）：
//! 每套主题包带一个 `extension.json`，里面的 `iconDefinitions` 把**图标 id** 映射到 SVG 相对
//! 路径，再由 `fileNames` / `fileExtensions` / `folders` / `expandedFolders` 把
//! **文件名 / 后缀 / 目录名** 映射到图标 id。所以"查一层"是这个体系的固有形状。
//!
//! ## 选哪套主题、为什么
//!
//! 固定用 **`idea` 包**（`gpui/assets/icon-themes/idea/extension.json`，主题 id `idea-icons`）：
//!
//! - 真机默认就是它 —— `windows/tauri/src/features/settings/config/default-settings.ts:100`
//!   的 `iconTheme: "idea-icons"`。所以"还原真机截图里的文件树"要的就是这一套
//!   （`.gitignore` 是 git 字形、`pom.xml` 是 Maven 的 `m`、`README.md` 是 `MD`、目录是文件夹）。
//! - 它是**唯一**把明暗两套路径都写进同一个 `extension.json` 的包
//!   （`idea/extension.json` 的 `iconDefinitions` = 深色、`lightIconDefinitions` = 浅色），
//!   所以选明暗不用猜路径规则。另外三套各有各的浅色约定：
//!   `lithe` 把浅色放在 `icons/light/**` 目录里（`extension.json` 里的路径不带 `light/`，
//!   要靠目录改写）；`symbols` / `pierre` 的深浅约定与 `../icon-asset-inventory.md` 第 7 节记的
//!   一致。要换主题得先补这套规则，本模块**只实现 `idea` 一套**，把不确定的留给后续。
//!
//! ## 明暗怎么选
//!
//! 与 `icons::idea_icon_svg` 同一口径：读全局 `Theme::global(cx).is_dark()`。
//! 真机的判据是**界面主题**而不是图标主题（`extension-contribution-runtime.ts:81-91` 的
//! `getIconDefinitionsForAppearance` 读 `themeRegistry.getCurrentTheme().isDark`），
//! 这一点两边一致。
//!
//! ## 匹配顺序（与真机逐条一致）
//!
//! 来自 `extension-contribution-runtime.ts:116-138` 的 `getFileIcon`：
//!
//! - 目录：`isExpanded ? expandedFolders[名] : undefined` → `folders[名]` →
//!   `isExpanded ? defaultFolderOpen : defaultFolder`（本包两者都是 `idea-folder`）。
//! - 文件：`filenames[名]` → **逐段后缀**（`archive.tar.gz` 依次试 `.tar.gz`、`.gz`）→ `defaultFile`。
//!   ⚠️ 真机那几个 `"\0lithe:java.class"` 之类的**语义键**要与 Java 语言服务的结果配合
//!   （`file-icon-semantics.ts`），gpui 侧还没有那个信息，所以**本层不查它们**：
//!   带 `\0` 前缀的键在查表前就被跳过（见 [`lookup_definition`]），落到普通后缀规则上。
//!
//! ## 读不到时回落 Lucide（调用方的责任，但契约在这里）
//!
//! 本模块只负责**查**：查不到就返回 `None`，由调用方用 `gpui_kit::assets::IconName`（全量 Lucide
//! 目录）兜底。`explorer` / `editor` 各自的 `icon_for_file` 就是这么用的（见
//! [`crate::icons::FileIcon`]）。
//!
//! ## 缓存
//!
//! `extension.json` 只在**进程内第一次调用时**解析一次（`std::sync::OnceLock`），
//! 之后每次查表都是 `HashMap` 命中 —— 与 `rust-embed` 把资源编译期内嵌、真机
//! `icon-theme-registry` 只解析一次的行为一致。解析失败时**不 panic**：`OnceLock` 存 `Err`，
//! 之后每次调用都立即返回 `None`，于是全局走 Lucide 回落。

use std::collections::HashMap;
use std::sync::OnceLock;

use gpui_kit::assets::IconName;
use gpui_kit::component::Theme;
use gpui_kit::{AnyElement, App, IntoElement as _, SharedString, Styled as _};

/// gpui 侧使用的图标主题 id（= `windows/tauri/src/features/settings/config/default-settings.ts:100`
/// 的 `iconTheme` 默认值，也是 `file-icon-semantics.ts:14` 的 `IDEA_ICON_THEME_ID`）。
///
/// 它同时是 `gpui/assets/icon-themes/` 下的**目录名**：`gpui/assets/icon-themes/idea/extension.json`。
/// 目前没有"用户在设置里换图标主题"这条路径（gpui 侧还没有这个设置项），所以它是常量而不是配置。
pub const ACTIVE_FILE_ICON_THEME: &str = "idea";

/// 主题包目录（相对 `gpui/assets/`）的前缀；资源路径就是 `{THEMES_ROOT}/{主题 id}/{定义里的相对路径}`。
const THEMES_ROOT: &str = "icon-themes";

/// 真机 `FileIcon` 的 `defaultFile` / `defaultFolder` / `defaultFolderOpen`
/// （`idea/extension.json` 的 `icons[0].defaultFile|defaultFolder|defaultFolderOpen`）。
const DEFAULT_FILE: &str = "idea-text";
const DEFAULT_FOLDER: &str = "idea-folder";

/// `idea` 主题包的 `extension.json`，**编译期内嵌**。
///
/// 用 `include_str!` 而不是运行期读盘 / 走 `AssetSource`：这个文件是图标查找的**真源**，
/// 编译期嵌进来之后"资源在不在"是编译错误而不是运行期静默回落；而且它与
/// `crates/app/src/assets.rs` 的 `rust-embed` 嵌的是**同一个文件**，不会漂移。
/// 路径相对**本文件**：`crates/shared/src/icons/` → 上溯四级到 `gpui/`，再进 `assets/`。
const IDEA_EXTENSION_JSON: &str =
    include_str!("../../../../assets/icon-themes/idea/extension.json");

/// 已经解析好的 `idea` 主题包：图标 id → SVG 相对路径，以及各级查找表。
///
/// 只在进程内构造一次（见 [`active_theme`]），之后只读。
#[derive(Debug)]
pub struct FileIconTheme {
    /// 资源路径前缀，例如 `icon-themes/idea/`。
    root: String,
    /// 深色 `iconDefinitions`：图标 id → 相对主题根的资源路径。
    dark: HashMap<String, String>,
    /// 浅色 `lightIconDefinitions`：图标 id → 相对主题根的资源路径。
    light: HashMap<String, String>,
    /// `fileNames`（小写文件名 → 图标 id）。
    file_names: HashMap<String, String>,
    /// `fileExtensions`（小写、带点后缀 → 图标 id）。
    file_extensions: HashMap<String, String>,
    /// `folders`（小写目录名 → 图标 id）。
    folders: HashMap<String, String>,
    /// `expandedFolders`（小写目录名 → 图标 id）。
    expanded_folders: HashMap<String, String>,
}

impl FileIconTheme {
    /// `extension.json` 里 `icons[0]` 的解析形状（只取本层用得到的字段）。
    ///
    /// 用 `serde_json::Value` 现取现转而不是给结构体加 `Deserialize`：这份 JSON 来自
    /// 上游图标包，字段后续可能增删（`material` 包里同一份 schema 的形状就不一样），
    /// 逐字段取值能让"某个字段缺失"退化成"该级查表不命中"，而不是整个解析失败。
    fn from_json(theme: &str, json: &str) -> Result<Self, serde_json::Error> {
        let root: serde_json::Value = serde_json::from_str(json)?;
        let icon = root
            .get("icons")
            .and_then(|icons| icons.get(0))
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        Ok(Self {
            root: format!("{THEMES_ROOT}/{theme}/"),
            dark: definitions(icon.get("iconDefinitions")),
            light: definitions(icon.get("lightIconDefinitions")),
            file_names: string_map(icon.get("filenames")),
            file_extensions: string_map(icon.get("fileExtensions")),
            folders: string_map(icon.get("folders")),
            expanded_folders: string_map(icon.get("expandedFolders")),
        })
    }

    /// 主题包的资源路径前缀（`icon-themes/idea/`）。
    #[must_use]
    pub fn root(&self) -> &str {
        &self.root
    }

    /// 图标定义的数量（诊断用：启动日志里一行就能看出"表到底读进来了没有"）。
    #[must_use]
    pub fn definition_count(&self) -> usize {
        self.dark.len()
    }

    /// 查一个**文件**的资源路径。`dark` 为 `true` 时取 `iconDefinitions`，
    /// 否则取 `lightIconDefinitions`（没有浅色定义时回落深色，与真机
    /// `getIconDefinitionsForAppearance` 的 `?? contribution.iconDefinitions` 同序）。
    ///
    /// 返回的是**完整资源路径**（`icon-themes/idea/icons/expui/fileTypes/java.svg`），
    /// 可直接交给 [`theme_file_icon`] 渲染，或交给 `AssetSource::load` 探针。
    #[must_use]
    pub fn file_path(&self, file_name: &str, dark: bool) -> Option<&str> {
        let lower = file_name.to_ascii_lowercase();
        let icon = lookup_definition(
            &lower,
            &self.file_names,
            &self.file_extensions,
            DEFAULT_FILE,
        )?;
        self.resolve(icon, dark)
    }

    /// 查一个**目录**的资源路径；`expanded` 决定先查 `expandedFolders`。
    #[must_use]
    pub fn folder_path(&self, folder_name: &str, expanded: bool, dark: bool) -> Option<&str> {
        let lower = folder_name.to_ascii_lowercase();
        let icon = if expanded {
            lookup_mapped(&lower, &self.expanded_folders)
        } else {
            None
        }
        .or_else(|| lookup_mapped(&lower, &self.folders))
        .unwrap_or(DEFAULT_FOLDER);
        self.resolve(icon, dark)
    }

    /// 图标 id → 完整资源路径。
    ///
    /// **逐条目回落**：浅色定义缺失时补深色那份（真机
    /// `getIconDefinitionsForAppearance` 的 `?? contribution.iconDefinitions` 是整个表级的回落，
    /// 逐条目回落是它的更细版本 —— 本包的 53 个 id 明暗都齐，两者等价；
    /// 换成浅色定义不全的主题包时逐条目回落更稳）。
    fn resolve(&self, icon: &str, dark: bool) -> Option<&str> {
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

/// 语义键前缀：真机用 `"\0lithe:java.class"` 这类**不可由文件名推导**的键表示
/// "Java class / 源码根目录"等语义（`file-icon-semantics.ts:16-28`）。gpui 侧还没有
/// 对应的语义信息，所以查表时把它们全部跳过，让这些条目走普通后缀规则。
const SEMANTIC_PREFIX: char = '\0';
/// 进程内唯一一份已解析的主题表。
///
/// 解析失败会存 `Err`（而不是重试）：`extension.json` 是编译期内嵌的常量，失败必然是
/// 打包/生成出了错；每次查表都重试只会把同一个错误重复上千次。
static ACTIVE_THEME: OnceLock<Result<FileIconTheme, serde_json::Error>> = OnceLock::new();

/// 取当前生效的文件图标主题（进程内缓存）。
///
/// 出错时返回 `None`，调用方据此走 Lucide 回落 —— **这是"读不到时回落"的落地点**。
#[must_use]
pub fn active_theme() -> Option<&'static FileIconTheme> {
    ACTIVE_THEME
        .get_or_init(|| FileIconTheme::from_json(ACTIVE_FILE_ICON_THEME, IDEA_EXTENSION_JSON))
        .as_ref()
        .ok()
}

/// 查一个文件或目录在**当前主题 + 当前明暗**下的真源图标资源路径。
///
/// 这是给渲染用的薄包装：调用方不用自己处理"主题没读进来"与"这个文件没有专属图标"
/// 两种 `None`。`is_dir` / `expanded` 的语义与真机 `getFileIcon(fileName, isDir, isExpanded)` 一致。
#[must_use]
pub fn theme_icon_path(name: &str, is_dir: bool, expanded: bool, cx: &App) -> Option<&'static str> {
    let theme = active_theme()?;
    let dark = Theme::global(cx).is_dark();
    if is_dir {
        theme.folder_path(name, expanded, dark)
    } else {
        theme.file_path(name, dark)
    }
}

/// 资源取字节：**只走注册的 `AssetSource`**；`None` = 这个路径在内嵌资源里不存在。
///
/// ⚠️ 必须走 `crates/app/src/assets.rs` 注册的 `LitheAssets`，不能用 `std::fs::read`：
/// 资源是 `rust-embed` 编译期内嵌的，发布版没有对应的磁盘目录。
/// `cx.asset_source()` 拿到的正是那个包装层（`AssetSource::load`）。
///
/// 它是"查找层算出来的路径真的有资源"的唯一校验入口；启动诊断
/// `S1_ASSETS probe path=icon-themes/… bytes=…`（`crates/app/src/main.rs`）走同一条
/// `load` 路径，两者一起证明"路径对、字节取得到"。
#[must_use]
pub fn theme_icon_bytes(asset_path: &str, cx: &App) -> Option<Vec<u8>> {
    cx.asset_source()
        .load(asset_path)
        .ok()
        .flatten()
        .map(|bytes| bytes.into_owned())
}

/// 把 [`theme_icon_path`] 查到的 SVG 路径画成一个**单色、锐利**的元素。
///
/// `side` 是边长，接受任何 `Into<Length>`：`rems(1.)`（16，文件树）、`rems(0.75)`（12，编辑器标签）
/// 或 `px(..)` 都可以 —— 布局规范要求优先用 rem 档位，所以调用点通常传 `rems(..)`。
///
/// `None` 表示资源取不到（查找层算出来的路径在内嵌资源里不存在）：调用方回落到
/// Lucide 字形（见 `crate::icons::FileIcon::render`），**不画空白**。
///
/// ## 为什么是 `svg()`（单色）而不是 `img()`（保留原色）—— 实测结论
///
/// 真机的文件类型图标是**彩色**的（`.java` 橙、`.gitignore` 红、`pom.xml` 蓝），
/// 理论上该走 `img()`（`Img` 把 SVG 渲成彩色位图）。**但在这台机器上实测不可行**：
///
/// | 现象 | 证据 |
/// | --- | --- |
/// | `Img` 在文件树里**时灵时不灵** | 同一个构建：`rems(1.)`（16px）时 9 行树里只有目录行有字形；改成 `rems(4.)`（64px）后 9 行全部有字形 |
/// | `Img` 放进状态栏 chip **完全不画** | 同一帧里 `branch.svg` 那块区域一个非背景像素都没有（`.artifacts/p5/statuszoom-{before,after}.png`） |
/// | 颜色也不是原色 | 64px 那次的像素直方图里没有 `#F34E29` / `#3574F0` 这类原色，只有被压暗、去饱和的近似值 |
///
/// 成因指向 gpui 的图片链路：`Img::request_layout` 只在有 `GlobalElementId` 时才 `use_data`
/// （`gpui-pre-0.3.6/src/elements/img.rs:289-315`），而这条链路上的元素（`Tree` 的行内容、
/// 状态栏 chip）没有稳定的全局 id；`svg()` 没有这个约束，在**所有**这些位置都稳定出图。
///
/// 取舍：**优先"画得出来 + 边缘锐利"，把"彩色"留作后续**（要补彩色得换掉 `Img` 这条链路，
/// 例如预渲染成 `RenderImage` 后走 `ImageSource::Render`，或用 `gpui::canvas` 自绘 ——
/// 都不是改一个参数能解决的）。单色也是旧实现的既有口径（`IconName` 一律单色），
/// 所以换源**没有引入新的视觉回归**。
///
/// ## 颜色由谁给
///
/// 取 [`Theme::muted_foreground`]：文件树与编辑器标签原本就用它画文件图标
/// （`Icon::new(..).text_color(cx.theme().muted_foreground)`），所以换源前后明暗一致。
/// 调用点要别的颜色可以自己再 `.text_color(..)` 覆盖（返回的是具体类型 `Svg`，`Styled`
/// 方法都可用）。这也与 `idea_icon_svg` 的"helper 自己设色、调用点不要猜"同一口径。
pub fn theme_file_icon(
    asset_path: &str,
    side: impl Into<gpui_kit::Length>,
    cx: &App,
) -> Option<AnyElement> {
    theme_icon_bytes(asset_path, cx)?;
    let theme = Theme::global(cx);
    let side: gpui_kit::Length = side.into();
    Some(
        gpui_kit::svg()
            .path(SharedString::from(asset_path.to_string()))
            .flex_shrink_0()
            .text_color(theme.muted_foreground)
            .w(side)
            .h(side)
            .into_any_element(),
    )
}

/// `iconDefinitions` / `lightIconDefinitions`：图标 id → 相对路径（去掉前导 `./`）。
fn definitions(value: Option<&serde_json::Value>) -> HashMap<String, String> {
    string_map(value)
}

/// 把一份 `{键: 图标 id}` 的 JSON 对象转成表。
///
/// - 键统一转小写：真机在查表前就把文件名 lower 掉（`extension-contribution-runtime.ts:118`），
///   这里把**表侧**也归一化，两边对称。
/// - 跳过语义键（以 [`SEMANTIC_PREFIX`] 开头）：见该常量的说明。
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
            map.insert(key.to_ascii_lowercase(), text.trim_start_matches("./").to_string());
        }
    }
    map
}

/// 真机的文件匹配顺序：`filenames[名]` → 逐段后缀 → `defaultFile`。
///
/// 后缀按**从长到短**试（`archive.tar.gz` 先试 `.tar.gz` 再试 `.gz`），与
/// `extension-contribution-runtime.ts:93-100` 的 `getFileExtensionCandidates` 同序。
fn lookup_definition<'a>(
    lower_name: &str,
    file_names: &'a HashMap<String, String>,
    file_extensions: &'a HashMap<String, String>,
    default_file: &'a str,
) -> Option<&'a str> {
    if let Some(icon) = lookup_mapped(lower_name, file_names) {
        return Some(icon);
    }
    extension_candidates(lower_name)
        .into_iter()
        .find_map(|candidate| lookup_mapped(candidate, file_extensions))
        .or(Some(default_file))
}

/// 真机的后缀候选：`archive.tar.gz` → `[".tar.gz", ".gz"]`（长 → 短）。
///
/// 与 `extension-contribution-runtime.ts:93-100` 的 `getFileExtensionCandidates` 同序：
/// 从**第一个点**开始逐段缩短，所以 `.tar.gz` 先于 `.gz` 被试到。
fn extension_candidates(lower_name: &str) -> Vec<&str> {
    if lower_name.starts_with('.') || !lower_name.contains('.') {
        // 真机对 `.gitignore` 这种"点开头"的名字返回空候选（`parts[0] === ""`），
        // 这类名字靠 `filenames` 表命中；纯名字（无点）没有后缀可试。
        return Vec::new();
    }
    lower_name.match_indices('.').map(|(index, _)| &lower_name[index..]).collect()
}

/// 单表命中；`None` 表示"这一级没有专属图标"。
fn lookup_mapped<'a>(key: &str, table: &'a HashMap<String, String>) -> Option<&'a str> {
    table.get(key).map(String::as_str)
}

/// 主题包里**没有**这个文件 / 目录的专属条目时，用哪个 Lucide 字形兜底。
///
/// 这张表是 `explorer` / `editor` 原来各自维护的"后缀 → `IconName`"判断的**唯一副本**：
/// 两份逐条同值的表以前分别写在 `explorer/src/model.rs` 与 `editor/src/buffer.rs`，
/// 搬到这里之后两边共用，不会再漂移。字形一律取**全量 Lucide 目录里确实存在**的那个
/// （`gpui-kit-assets-0.6.6/assets/icons/`，1830 个；`IconName` 就是按这份目录生成的枚举）。
///
/// 唯一"名不副实"的一条：Lucide 目录**没有 `file-json.svg`**，所以 JSON / JSONC 用语义最近的
/// `FileBraces`（`icons/file-braces.svg`）—— Lucide 里花括号就是 JSON 的通用符号。
/// 这条例外在两张旧表里都有记录，这里保留同样的说明。
///
/// ⚠️ 这一层只在**主题包查不到**时才生效（见 [`FileIcon::theme_icon`]）。主题包本身能认
/// `.java`、`.md`、`pom.xml`、`.gitignore` 等 79 个后缀 + 13 个文件名，所以实际界面上
/// 绝大多数文件走的是真源图标；走到这里的是一些冷门后缀（如 `.unknownext`）
/// 与"主题包没登记但 Lucide 有近似字形"的情况。
#[must_use]
pub fn lucide_fallback(name: &str) -> IconName {
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());

    match extension.as_deref() {
        // 图片/图标：`Image`（`icons/image.svg`）。
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "svg" | "bmp" | "avif") => {
            IconName::Image
        }
        // JSON / JSONC：目录里**没有 `file-json.svg`**，取 `FileBraces`
        // （`icons/file-braces.svg`）。
        Some("json" | "jsonc") => IconName::FileBraces,
        // Markdown / 纯文本：`FileText`（`icons/file-text.svg`）。
        Some("md" | "markdown" | "txt" | "rst" | "adoc") => IconName::FileText,
        // 配置：`Settings`（`icons/settings.svg`）。
        Some("toml" | "yaml" | "yml" | "ini" | "conf" | "cfg" | "properties" | "editorconfig") => {
            IconName::Settings
        }
        // 依赖锁 / 包文件：`Package`（`icons/package.svg`）。
        Some("lock") => IconName::Package,
        // 脚本 / 终端入口：`Terminal`（`icons/terminal.svg`）。
        Some("sh" | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "bat" | "cmd") => IconName::Terminal,
        // 源码：`FileCode`（`icons/file-code.svg`）。
        Some(
            "rs" | "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" | "py" | "java"
            | "kt" | "kts" | "swift" | "go" | "c" | "h" | "cc" | "cpp" | "hpp" | "cs" | "rb"
            | "php" | "vue" | "svelte" | "sql" | "css" | "scss" | "less" | "html" | "xml"
            | "gradle" | "lua" | "dart" | "scala" | "ex" | "exs" | "dockerfile",
        ) => IconName::FileCode,
        // 其余（含没有扩展名的）：`File`（`icons/file.svg`）。
        _ => IconName::File,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 把完整资源路径削成"相对主题包根"的短形式，让断言只钉住**主题包内部**的相对路径
    /// （`icon-themes/idea/` 这段前缀本身由 `active_theme_parses` 钉住）。
    fn relative(path: Option<&str>) -> Option<&str> {
        path.map(|path| path.trim_start_matches("icon-themes/idea/"))
    }

    /// 主题包必须能读进来 —— 这是"接线真的生效"的前置条件。
    ///
    /// 它同时钉住内嵌资源的**数量级**：`idea/extension.json` 里有 53 条 `iconDefinitions`，
    /// 生成脚本或搬运步骤把文件弄丢/弄空时这里会立刻红。
    #[test]
    fn active_theme_parses() {
        let theme = active_theme().expect("idea theme must parse");
        assert_eq!(theme.root(), "icon-themes/idea/");
        assert_eq!(theme.definition_count(), 53);
    }

    /// 真机默认图标主题下，这几个文件必须命中**专属**图标（不是 defaultFile）。
    ///
    /// 这一组就是维护者截图里的那几类：`.gitignore` 与 `pom.xml` 走 `filenames`，
    /// `README.md`、`A.java`、`x.yml` 走后缀表。逐条断言**具体 SVG 路径**，
    /// 所以"查表把 `.java` 当成 `.jar`"这类顺序错误也会被抓到。
    #[test]
    fn idea_theme_matches_the_documented_files() {
        let theme = active_theme().expect("idea theme must parse");
        for (name, dark, light) in [
            (
                ".gitignore",
                "icons/expui/fileTypes/gitignore.svg",
                "icons/expui/fileTypes/gitignore.svg",
            ),
            (
                "pom.xml",
                "icons/expui/fileTypes/maven_dark.svg",
                "icons/expui/fileTypes/maven.svg",
            ),
            (
                "README.md",
                "icons/expui/fileTypes/markdown_dark.svg",
                "icons/expui/fileTypes/markdown.svg",
            ),
            (
                "A.java",
                "icons/expui/fileTypes/java_dark.svg",
                "icons/expui/fileTypes/java.svg",
            ),
            (
                "x.yml",
                "icons/expui/fileTypes/yaml_dark.svg",
                "icons/expui/fileTypes/yaml.svg",
            ),
        ] {
            assert_eq!(relative(theme.file_path(name, true)), Some(dark), "dark: {name}");
            assert_eq!(
                relative(theme.file_path(name, false)),
                Some(light),
                "light: {name}"
            );
        }
    }

    /// 没有专属条目的文件名要落到 `defaultFile`（`idea-text`），
    /// **不是**随便挑一个后缀命中 —— 这是"全局回落 Lucide"之外的第二种回落地。
    #[test]
    fn unknown_file_falls_back_to_default_file() {
        let theme = active_theme().expect("idea theme must parse");
        assert_eq!(
            relative(theme.file_path("notes.unknownext", true)),
            Some("icons/expui/fileTypes/text_dark.svg"),
        );
        assert_eq!(
            relative(theme.file_path("no-extension-at-all", true)),
            Some("icons/expui/fileTypes/text_dark.svg"),
        );
    }

    /// 多段后缀必须**从长到短**试：`.tar.gz` 表里没有、`.gz`（→ archive）有，
    /// 所以 `archive.tar.gz` 应该命中 archive，而不是被"第一个点之后全当后缀"错配。
    #[test]
    fn multi_dot_names_use_the_longest_suffix_first() {
        let theme = active_theme().expect("idea theme must parse");
        assert_eq!(
            relative(theme.file_path("archive.tar.gz", true)),
            Some("icons/expui/fileTypes/archive_dark.svg"),
        );
    }

    /// 目录名与展开态：`src` 没有专属条目 → 默认文件夹图标；
    /// 展开用 `expandedFolders`（本包里只有语义键，所以与折叠时同值）。
    #[test]
    fn folders_use_default_folder_icon() {
        let theme = active_theme().expect("idea theme must parse");
        assert_eq!(
            relative(theme.folder_path("src", false, true)),
            Some("icons/expui/nodes/folder_dark.svg"),
        );
        assert_eq!(
            relative(theme.folder_path("src", true, false)),
            Some("icons/expui/nodes/folder.svg"),
        );
    }

    /// 查到的路径必须**在 gpui/assets 下真的存在**：查找层与 `rust-embed` 的表是两份数据，
    /// 只有这条能把"表对、资源不在"这种漂移钉住（`theme_file_icon` 会因此回落 Lucide，
    /// 界面上看起来只是"图标变了"，不会报错）。
    ///
    /// 走**磁盘**而不是 `AssetSource`：本测试只覆盖"`extension.json` 里的相对路径是否指向
    /// `gpui/assets/` 下真实存在的文件"，不需要起一个 gpui App。
    #[test]
    fn every_definition_exists_in_the_assets_tree() {
        let theme = active_theme().expect("idea theme must parse");
        // `CARGO_MANIFEST_DIR` = `gpui/crates/shared`，资源在 `gpui/assets`。
        // 表里存的是**主题包根之下的相对路径**（`icons/expui/…`），所以这里要补回
        // `icon-themes/idea/` 那段前缀 —— 正好顺带钉住 `root()` 与定义表能拼成完整资源键。
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let missing = |table: &HashMap<String, String>| -> Vec<String> {
            table
                .values()
                .filter(|path| !assets.join(theme.root()).join(path).exists())
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

    /// Lucide 回落表的三条关键口径（这张表是 `explorer` / `editor` 两份旧表的唯一副本，
    /// 所以它自己必须有测试）：
    ///
    /// - JSON **没有** `file-json.svg`，取 `FileBraces`；
    /// - 配置类后缀取 `Settings`（`x.yml` 在界面上会看到齿轮 —— 这正是"回落没坏"的
    ///   可见证据：`idea` 主题包的 79 张 SVG 里没有齿轮）；
    /// - 什么都不认识才是 `File`。
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
