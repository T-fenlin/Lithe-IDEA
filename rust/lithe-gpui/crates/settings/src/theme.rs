use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use gpui::{App, Global, Hsla, SharedString, WindowAppearance};
use lithe_ui::devtools::contrast;
use lithe_ui::theme::{Mode as ComponentMode, Palette, Theme as ComponentTheme};
use serde::Deserialize;

use crate::paths;
use crate::schema::{ThemeEntry, ThemeIndex, slugify_theme_id, theme_font_size_for};

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

/// 主题文件里的明/暗模式。
///
/// 组件库的 [`Mode`](lithe_ui::theme::Mode) 没有 `Deserialize`，主题文件里那一栏又是字符串
/// （`"light"` / `"dark"`），所以宿主自持一个可反序列化的模式类型，落到组件库时再换算。
/// 主题文件的读法归宿主，不借任何组件栈自带的那套。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub fn is_dark(self) -> bool {
        matches!(self, Self::Dark)
    }

    fn to_component(self) -> ComponentMode {
        match self {
            Self::Light => ComponentMode::Light,
            Self::Dark => ComponentMode::Dark,
        }
    }
}

/// 主题文件的顶层结构。只声明 `themes`，其余字段（`$schema` / `author` / `url`）与宿主无关。
#[derive(Debug, Clone, Deserialize)]
struct ThemeSet {
    #[serde(default)]
    themes: Vec<ThemeRecord>,
}

/// 主题文件里的一条主题。
///
/// 只收宿主真正要用的字段：名字、模式、颜色表。**颜色表按主题文件里的原始键收**
/// （`background` / `muted.foreground` / `list.hover.background` …）而不是落成结构体——
/// 主题文件多一个键、少一个键都不会让整份主题读不出来，也免得宿主的类型跟着上游的
/// `ThemeConfigColors` 走。
#[derive(Debug, Clone, Deserialize)]
struct ThemeRecord {
    #[serde(default)]
    name: String,
    #[serde(default)]
    mode: ThemeMode,
    #[serde(default)]
    colors: BTreeMap<String, serde_json::Value>,
}

/// 当前生效的主题名（宿主的真源）。
///
/// 换主题、探主题（诊断 / `--theme-probe`）都以它为真源，组件库那边的全局由本 crate
/// 统一写，宿主其它地方不再各记一份。
#[derive(Default, Debug)]
pub struct ActiveTheme(pub Option<SharedString>);

impl Global for ActiveTheme {}

pub fn active_theme_name(cx: &App) -> Option<SharedString> {
    if !cx.has_global::<ActiveTheme>() {
        return None;
    }
    cx.global::<ActiveTheme>().0.clone()
}

fn set_active_theme(cx: &mut App, name: &str) {
    cx.set_global(ActiveTheme(Some(SharedString::from(name.to_string()))));
}

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

/// 宿主能看到的全部主题文档：**用户主题目录在前**（同名时压过内置），内置主题在后。
///
/// 顺序就是优先级：`ThemeIndex::from_documents` 撞到重复的 id/name 会跳过后来者，
/// 「用户目录里那条赢」这条规矩靠的就是这个顺序（见
/// `user_directory_documents_win_over_builtin_ones`）。
fn theme_documents() -> Vec<String> {
    let mut documents = paths::themes_dir()
        .map(|dir| theme_documents_in(&dir))
        .unwrap_or_default();
    documents.extend(
        BUILTIN_THEMES
            .iter()
            .map(|(_, content)| (*content).to_string()),
    );
    documents
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

    let index = ThemeIndex::from_documents(theme_documents());
    *lock_index_cache(cache) = Some((stamp, index.clone()));
    index
}

/// 按名字在全部主题文档里找那一条（用户目录在前，同名时先见者赢）。
///
/// 每次现读现解析：换主题是低频动作（换一次、改一次文件），几份小 JSON 不值得为它再
/// 养一层缓存；而缓存一旦与磁盘错开，换主题就会换错颜色。
fn locate_theme(name: &str) -> Option<ThemeRecord> {
    for document in theme_documents() {
        let Ok(set) = serde_json::from_str::<ThemeSet>(&document) else {
            continue;
        };
        if let Some(record) = set.themes.into_iter().find(|record| record.name == name) {
            return Some(record);
        }
    }
    None
}

/// 含 `name` 这条主题的那份主题文档原文。
///
/// 交出去的是原文而不是还原后的调色板，于是**逐键无损**：需要查某一栏原始取值
/// （`button.hover.background` / `tab.active.background` 这类）时读的就是主题文件本身。
pub fn theme_document_for(name: &str) -> Option<String> {
    theme_documents().into_iter().find(|document| {
        serde_json::from_str::<ThemeSet>(document)
            .map(|set| set.themes.iter().any(|record| record.name == name))
            .unwrap_or(false)
    })
}

/// 主题文件里的颜色字符串 → `Hsla`。
///
/// 宿主主题文件里全是纯色（`#rrggbb` / `#rrggbbaa`；实测 37 个不同取值、无渐变），
/// 所以只认 gpui 自己的 `Rgba` 解析，不认识的一律当「这一栏没给」——宁可用组件库的
/// 默认值，也不要为了猜一个颜色把整份主题读失败。
fn parse_color(value: &str) -> Option<Hsla> {
    gpui::Rgba::try_from(value.trim()).ok().map(Hsla::from)
}

/// 一条主题 → 组件库调色板。
///
/// 从主题文件的原始键直取到组件库调色板：`bg`/`surface`←`background`、
/// `overlay`←`popover.background`、`backdrop`←`overlay`、`hover`←`list.hover.background`、
/// `active`←`list.active.background`、`focus`/`link`←`ring`、`selection`←`sidebar.accent.background`
/// …… 没给的栏保留组件库自己的默认值。
///
/// 例外是 `chrome_hover` / `chrome_active`：它们不是某个键的直取，而是**相对 chrome 底色**
/// 推出来的一档（见函数里那段说明）——悬停与选中是「从它所在底色上迈的一档」，不是一个
/// 到处通用的绝对色。
fn palette_for(record: &ThemeRecord) -> Palette {
    let mut palette = match record.mode {
        ThemeMode::Light => Palette::light(false),
        ThemeMode::Dark => Palette::dark(false),
    };
    let color = |key: &str| {
        record
            .colors
            .get(key)
            .and_then(serde_json::Value::as_str)
            .and_then(parse_color)
    };

    if let Some(color) = color("background") {
        // 面板与中央列同一个底色（外壳原先就这么画）；分隔线另取 `border`。
        palette.bg = color;
        palette.surface = color;
    }
    if let Some(color) = color("tab_bar.background") {
        palette.tab_bar = color;
    }
    if let Some(color) = color("title_bar.background") {
        palette.title_bar = color;
    }
    if let Some(color) = color("status_bar.background") {
        palette.status_bar = color;
    }
    if let Some(color) = color("muted.background") {
        palette.sunken = color;
    }
    if let Some(color) = color("list.hover.background") {
        palette.hover = color;
    }
    if let Some(color) = color("list.active.background") {
        palette.active = color;
    }
    // 主题文件里那一栏 `overlay` 是 scrim（模态浮层压在最上面的那层暗底，实测两组内置主题
    // 都给 `#00000033`≈20% 黑），**不是**浮层自己的底色；它归 `backdrop`——`Backdrop` 那层
    // 全窗 scrim 读的就是它（见 `lithe-ui::primitives::backdrop`）。
    if let Some(color) = color("overlay") {
        palette.backdrop = color;
    }
    // —— 悬停与选中是「从它所在底色上迈的一档」，不是一个到处通用的绝对色 ——
    //
    // `hover` 那一档是贴着 `list.background`（浅色纯白）调出来的：同一个绝对色铺到同色的
    // `title_bar` / `popover` 上就消失了——浅色里 `list.hover.background`、`title_bar.background`、
    // `popover.background`、`secondary.hover.background` 四栏全是 `#f2f2f2`，于是菜单条条目
    // 与弹出菜单条目都没有悬停反馈。所以 chrome 面（标题栏 / 状态栏 / 浮层面板）另取一档：
    // 候选全在主题文件里、不新造色值，取**第一个与三个 chrome 面都不同色**的。深色那边
    // `list.hover.background` 的 `#3c3f41` 本就与 `#313335` 差一档，直接用它；浅色撞色后退到
    // `button.hover.background` 的 `#d9e5f2`——按钮底色 `button.background` 正是 chrome 的
    // `#f2f2f2`，这一栏就是主题自己给出的「在 #f2f2f2 上悬停用什么」。
    let popover = color("popover.background");
    let chrome_bases = [
        palette.title_bar,
        palette.status_bar,
        popover.unwrap_or(palette.overlay),
    ];
    let chrome_hover = [
        color("list.hover.background"),
        color("button.hover.background"),
        color("secondary.hover.background"),
    ]
    .into_iter()
    .flatten()
    .find(|each| chrome_bases.iter().all(|base| base != each))
    .unwrap_or(palette.hover);
    // 选中态必须比悬停态更强、且不与之同色：浅色里 `list.active.background` 与上面退到的
    // `button.hover.background` 同为 `#d9e5f2`，于是再退到 `selection.background` 的
    // `#b3d7ff`——主题里唯一比它更深、且成体系的那一档（对比度 1.33:1 对 1.14:1）。深色那边
    // `list.active.background` 的 `#214283` 与悬停的 `#3c3f41` 本就不同且更强，取它。
    let chrome_active = [
        color("list.active.background"),
        color("selection.background"),
        color("secondary.active.background"),
    ]
    .into_iter()
    .flatten()
    .find(|each| {
        *each != chrome_hover
            && chrome_bases
                .iter()
                .all(|base| contrast(*each, *base) >= contrast(chrome_hover, *base))
    })
    .unwrap_or(palette.active);
    palette.chrome_hover = chrome_hover;
    palette.chrome_active = chrome_active;
    // 浮层底色另有其键：`popover.background`。
    //
    // 守卫：只有当它会与**真正画在浮层里的那一档高亮**（`chrome_hover`，不是 `hover`）同色时
    // 才退到 `surface`（= `background`）：宁可浮层与页面同色、靠描边与投影分辨层次，也不要丢掉
    // 行高亮。两组内置主题都不触发——浅色 `#f2f2f2` 对 `#d9e5f2`、深色 `#313335` 对 `#3c3f41`，
    // 于是浮层拿回主题给它的那一档「抬起来」的底色。
    if let Some(popover) = popover {
        let merges_into_the_row_highlight = popover == chrome_hover && palette.surface != popover;
        palette.overlay = if merges_into_the_row_highlight {
            palette.surface
        } else {
            popover
        };
    }
    if let Some(color) = color("border") {
        palette.border = color;
        palette.border_strong = color;
    }
    if let Some(color) = color("foreground") {
        palette.fg = color;
    }
    if let Some(color) = color("muted.foreground") {
        palette.fg_muted = color;
        palette.fg_subtle = color;
        palette.fg_disabled = color.opacity(0.6);
    }
    if let Some(color) = color("accent.background") {
        palette.accent = color;
        palette.accent_hover = color;
    }
    if let Some(color) = color("accent.foreground") {
        palette.on_accent = color;
    }
    // 宿主的品牌色（活动栏选中态读的就是它）；库侧没有更近的槽，2026-10-10 起 Palette 有专槽。
    if let Some(color) = color("primary.background") {
        palette.primary = color;
    }
    if let Some(color) = color("primary.foreground") {
        palette.on_primary = color;
    }
    if let Some(color) = color("ring") {
        palette.focus = color;
        palette.link = color;
    }
    if let Some(color) = color("sidebar.accent.background") {
        palette.selection = color;
    }
    if let Some(color) = color("success.background") {
        palette.success = color;
    }
    if let Some(color) = color("warning.background") {
        palette.warning = color;
    }
    if let Some(color) = color("danger.background") {
        palette.danger = color;
    }
    if let Some(color) = color("info.background") {
        palette.info = color;
    }
    palette
}

/// 主题列表的展示顺序：浅色在前、深色在后，同类按名字不分大小写。
///
/// 次序只由宿主自己的主题索引决定（主题文件没有 `is_default` 这一栏），于是主题菜单
/// 与 `--theme-probe <下标>` 看到的是同一份顺序。
fn sorted_entries() -> Vec<ThemeEntry> {
    let mut entries = theme_index().entries().to_vec();
    entries.sort_by(|a, b| {
        a.dark
            .cmp(&b.dark)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

pub fn theme_ids(_cx: &App) -> Vec<String> {
    sorted_entries().into_iter().map(|entry| entry.id).collect()
}

pub fn theme_options(_cx: &App) -> Vec<(String, String)> {
    sorted_entries()
        .into_iter()
        .map(|entry| (entry.id, entry.name))
        .collect()
}

pub fn display_name_for(value: &str) -> String {
    let index = theme_index();
    index
        .canonicalize(value)
        .and_then(|id| index.name_for_id(&id).map(str::to_string))
        .unwrap_or_else(|| value.to_string())
}

pub fn canonical_theme_id(_cx: &App, value: &str) -> String {
    theme_index()
        .canonicalize(value)
        .unwrap_or_else(|| value.to_string())
}

/// 主题 id（或名字）→ 主题文件里的那一条名字。
fn resolve_name_for_id(id: &str) -> String {
    let index = theme_index();
    if let Some(canonical) = index.canonicalize(id) {
        if let Some(name) = index.name_for_id(&canonical) {
            return name.to_string();
        }
    }
    index
        .entries()
        .iter()
        .find(|entry| slugify_theme_id(&entry.name) == id)
        .map(|entry| entry.name.clone())
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

    watch_themes_dir(cx, themes_dir);
}

/// 盯住主题目录：文件一改就重新应用当前主题。
///
/// 组件栈没有「主题目录」这一说，所以宿主自持（`notify` 与 `async-channel` 本来就在本 crate 的依赖里，见
/// `Cargo.toml`——它们不引入任何新的第三方代码，只是从传递依赖升成直接依赖）。
fn watch_themes_dir(cx: &mut App, dir: PathBuf) {
    let (sender, receiver) = async_channel::bounded::<()>(16);
    let mut watcher =
        match notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            let Ok(event) = result else {
                return;
            };
            if !matches!(
                event.kind,
                notify::EventKind::Create(_)
                    | notify::EventKind::Modify(_)
                    | notify::EventKind::Remove(_)
            ) {
                return;
            }
            let _ = sender.try_send(());
        }) {
            Ok(watcher) => watcher,
            Err(error) => {
                eprintln!("S1_THEME watch_failed dir={} error={error}", dir.display());
                return;
            }
        };

    let watched = dir.clone();
    let watched_for_log = dir.clone();
    cx.spawn(async move |cx| {
        use notify::Watcher as _;

        if let Err(error) = watcher.watch(&watched, notify::RecursiveMode::NonRecursive) {
            eprintln!(
                "S1_THEME watch_failed dir={} error={error}",
                watched.display()
            );
            return;
        }
        println!("S1_THEME watching dir={}", watched_for_log.display());

        while receiver.recv().await.is_ok() {
            let _ = cx.update(|cx| match crate::store::try_store(cx) {
                Some(store) => {
                    store.update(cx, |store, cx| store.reapply_theme(cx));
                }
                None => {
                    let id = crate::schema::DEFAULT_THEME;
                    if !apply_theme_by_id(cx, id) {
                        eprintln!("S1_THEME missing id={id}");
                    }
                }
            });
        }
    })
    .detach();
}

pub fn apply_theme_by_id(cx: &mut App, id: &str) -> bool {
    let name = resolve_name_for_id(id);
    if name != id {
        println!("S1_THEME resolved id={id} name={name}");
    }
    apply_theme_by_name(cx, &name)
}

/// 按名字应用主题：把主题文件的颜色落成组件库调色板，写进**组件库的主题全局**。
///
/// 宿主主题的真源只有一个：`lithe_ui::theme::Theme`。本函数记下活动主题名，颜色与字体
/// 都往那一个全局写。
pub fn apply_theme_by_name(cx: &mut App, name: &str) -> bool {
    let Some(record) = locate_theme(name) else {
        eprintln!("S1_THEME missing name={name}");
        return false;
    };

    if !cx.has_global::<ComponentTheme>() {
        eprintln!("S1_THEME component_theme_missing name={name}");
        return false;
    }

    // 先记活动主题名，再改主题全局：改全局会通知观察者（过渡适配就在那条链上），
    // 它读到的必须是这一次的名字。
    set_active_theme(cx, &record.name);
    ComponentTheme::apply(record.mode.to_component(), Some(palette_for(&record)), cx);
    println!(
        "S1_THEME applied={} dark={}",
        record.name,
        record.mode.is_dark()
    );
    true
}

/// 字体设置写进**组件库主题**（字体在这里是唯一真源）。
///
/// 组件库组件读的是 `lithe_ui::theme::Theme` 的 `font_family` / `mono_family` / `font_size` /
/// `mono_font_size`；尺寸缩放的唯一锚点也是它的 `font_size`——`Theme::update` 会把新锚点
/// 标到所有窗口的 rem 大小上。
pub fn apply_theme_font_size(cx: &mut App, ui_font_size: f64) {
    if !cx.has_global::<ComponentTheme>() {
        return;
    }
    let font_size = theme_font_size_for(ui_font_size);
    ComponentTheme::update(cx, |theme| theme.font_size = font_size);
    println!("S1_THEME font_size={font_size} ui_font_size={ui_font_size}");
}

pub fn apply_editor_font_size(cx: &mut App, editor_font_size: f64) {
    if !cx.has_global::<ComponentTheme>() {
        return;
    }
    let font_size = editor_font_size as f32;
    ComponentTheme::update(cx, |theme| theme.mono_font_size = font_size);
    println!("S1_THEME mono_font_size={font_size} editor_font_size={editor_font_size}");
}

pub const SYSTEM_UI_FONT_FAMILY: &str = ".SystemUIFont";

pub fn apply_font_families(cx: &mut App, ui_family: &str, mono_family: &str) {
    if !cx.has_global::<ComponentTheme>() {
        return;
    }
    let installed = installed_font_names(cx);
    let ui = usable_family("fontFamily", ui_family, installed).map(str::to_string);
    let mono = usable_family("monoFontFamily", mono_family, installed).map(str::to_string);
    if ui.is_none() && mono.is_none() {
        return;
    }

    ComponentTheme::update(cx, |theme| {
        if let Some(family) = &ui {
            theme.font_family = SharedString::from(family.clone());
        }
        if let Some(family) = &mono {
            theme.mono_family = SharedString::from(family.clone());
        }
    });

    if let Some(family) = ui {
        println!("S1_THEME font_family key=fontFamily value={family}");
    }
    if let Some(family) = mono {
        println!("S1_THEME font_family key=monoFontFamily value={family}");
    }
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

pub fn theme_names(_cx: &App) -> Vec<String> {
    sorted_entries()
        .into_iter()
        .map(|entry| entry.name)
        .collect()
}

pub fn theme_is_dark(_cx: &App, value: &str) -> Option<bool> {
    let index = theme_index();
    let id = index.canonicalize(value)?;
    index
        .entries()
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.dark)
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
                            .any(|theme| theme.mode == ThemeMode::Light)
                            && set.themes.iter().any(|theme| theme.mode == ThemeMode::Dark),
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
                            parse_color(value).is_some(),
                            "{file}/{name}: color {key:?} = {value:?} 宿主解析不了\
                             （宿主主题文件只收纯色 `#rrggbb[aa]`，渐变要另开一条映射）"
                        );
                    }

                    theme_count += 1;
                }
            }

            assert!(theme_count > 0, "no theme defined in {label}");
        }
    }

    /// 内置主题里那条主题记录。
    ///
    /// 测试直接翻 [`BUILTIN_THEMES`] 而不走 `locate_theme`：那条路会连带读用户主题目录，
    /// 同一个名字在用户目录里被改写时这个断言就不再是「内置主题长什么样」了。
    fn builtin_record(name: &str) -> ThemeRecord {
        for (file, content) in BUILTIN_THEMES {
            let Ok(set) = serde_json::from_str::<ThemeSet>(content) else {
                panic!("{file}: not a loadable ThemeSet");
            };
            if let Some(record) = set.themes.into_iter().find(|record| record.name == name) {
                return record;
            }
        }
        panic!("内置主题里没有 {name:?}");
    }

    #[test]
    fn overlay_panel_background_never_merges_into_the_row_highlight() {
        // 回归：浮层底色曾经取主题文件里那条 scrim 语义的 `overlay`（`#00000033`），菜单、
        // 下拉、命令面板、悬浮卡片统统是半透明的；改取 `popover.background` 后又撞上浅色
        // 里 `popover.background` 与 `list.hover.background` 同为 `#f2f2f2`——那时被指着的
        // 那一行与面板底色一致，等于没有高亮，所以有了 `palette_for` 里的退避。
        //
        // 2026-10-11：比对对象从 `hover` 换成 `chrome_hover`。浮层里的行用的是「画在 chrome
        // 面上」的那一档（菜单条目读 `chrome_hover`），`hover` 只画在列表白底上；拿它跟浮层
        // 底色比会误报，也挡不住真正的那条撞色。
        for name in ["IntelliJ Light", "Darcula"] {
            let palette = palette_for(&builtin_record(name));

            assert!(
                palette.overlay.a >= 1.0,
                "{name}: 浮层底色必须是实色（不能是 scrim），alpha={}",
                palette.overlay.a
            );
            assert_ne!(
                palette.overlay, palette.chrome_hover,
                "{name}: 浮层底色与画在它上面的行高亮同色，菜单里被指着的那一行会看不见"
            );
            // scrim 那条不能丢：Backdrop 铺的全窗暗底读的就是 `backdrop`。
            assert!(
                palette.backdrop.a < 1.0,
                "{name}: `overlay` 那栏是 scrim，应当落到 `backdrop` 上，alpha={}",
                palette.backdrop.a
            );
        }
    }

    /// chrome 面上的悬停/选中两档必须与**它所在的那块底色**分得开，且选中强于悬停。
    ///
    /// 量化下限取 1.10:1：深色里那条「本来就看得见」的悬停（`#3c3f41` 落在 `#313335` 上）
    /// 实测 1.195:1，浅色退到按钮那一档后是 1.141:1，取整到 1.10 做门槛——比只写 `assert_ne!`
    /// 强在：同色会挂，差 1/255 那种「理论不同、肉眼无别」也会挂。
    ///
    /// 顺带把「这一档到底从哪个键推出来」钉死，免得以后有人改了候选顺序却看不出后果。
    #[test]
    fn chrome_hover_and_active_stand_out_on_every_chrome_surface() {
        for (name, hover_key, active_key) in [
            (
                "IntelliJ Light",
                "button.hover.background",
                "selection.background",
            ),
            ("Darcula", "list.hover.background", "list.active.background"),
        ] {
            let record = builtin_record(name);
            let palette = palette_for(&record);
            let from_theme = |key: &str| {
                record
                    .colors
                    .get(key)
                    .and_then(serde_json::Value::as_str)
                    .and_then(parse_color)
                    .unwrap_or_else(|| panic!("{name}: 主题文件里没有 {key}"))
            };

            assert_eq!(
                palette.chrome_hover,
                from_theme(hover_key),
                "{name}: chrome 悬停那一档应当从 `{hover_key}` 推出"
            );
            assert_eq!(
                palette.chrome_active,
                from_theme(active_key),
                "{name}: chrome 选中那一档应当从 `{active_key}` 推出"
            );

            for (slot, base) in [
                ("title_bar", palette.title_bar),
                ("status_bar", palette.status_bar),
                ("overlay", palette.overlay),
            ] {
                let hover = contrast(palette.chrome_hover, base);
                assert!(
                    hover >= 1.10,
                    "{name}: chrome 悬停与 {slot} 只差 {hover:.3}:1（下限 1.10）"
                );
                assert_ne!(
                    palette.chrome_active, palette.chrome_hover,
                    "{name}: 选中态不能与悬停态同色"
                );
                let active = contrast(palette.chrome_active, base);
                assert!(
                    active >= hover,
                    "{name}: 选中态在 {slot} 上是 {active:.3}:1，弱于悬停的 {hover:.3}:1；\
                     选中必须比悬停更强"
                );
            }
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
