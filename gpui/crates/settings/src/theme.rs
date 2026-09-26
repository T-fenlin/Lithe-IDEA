//! 主题：**内置主题的播种**、装载目录、「按 id 应用」的唯一入口、以及 UI 字号到 rem 基准的映射。
//!
//! ## 内置主题怎么到用户那里（这是本模块最重要的一个决定）
//!
//! gpui-kit 的注册表**只能监视一个目录**：`ThemeRegistry::watch_dir` 里
//! `self.themes_dir = themes_dir` 是覆盖写，而 `reload()` 只读 `self.themes_dir`
//! （`gpui-component-0.6.6/src/theme/registry.rs:102,241-262`）。所以"内置目录 + 用户目录
//! 两个都装载"是**做不到**的 —— 调两次 `watch_dir` 只有最后一次生效，第一个监听器还会继续
//! 触发一次"读另一个目录"的重载。
//!
//! 于是本模块选的是**播种（seed）**：
//!
//! ```text
//! 启动 → 把 7 份内置主题（include_str! 进二进制）写到 <配置目录>/themes/
//!      → 已存在的文件**一个字节都不动**（用户的修改不会被覆盖）
//!      → watch_dir(<配置目录>/themes/) 装载 + 监听
//! ```
//!
//! 为什么不是另外两条路：
//!
//! | 方案 | 为什么不选 |
//! | --- | --- |
//! | 运行期同时装载"内置目录 + 用户目录" | 上游只支持一个目录，要么改上游、要么在每次文件变更后手工重注入；后者要挂在 `observe_global` 上做"缺了就补"，属于可重入观察者技巧 |
//! | 内置主题只走 assets、用户目录只放覆盖 | 同上：注册表没有"内存注入 + 目录装载"并存且能在文件变更后自动复原的入口（`load_themes_from_str` 注入的内容会被下一次 `reload()` 的 `themes.clear()` 清掉，而变更触发的重载**不会**回调我们的 `on_load`） |
//!
//! **代价（如实登记）**：用户目录里的内置主题是**首次启动那一次的副本**，程序升级后
//! 不会自动更新（因为"已存在就不动"是保护用户修改的必然结果）。想拿到新版的某一份，
//! 删除 `<配置目录>/themes/<文件>` 再重启即可重新播种。这条代价换来的是"用户改了主题
//! 永远不被程序覆盖"，对本产品更重要。
//!
//! **打包后的读取路径**：只有 `<配置目录>/themes/`（[`crate::paths::themes_dir`]）。
//! `gpui/themes/`（[`crate::paths::repository_themes_dir`]）是**构建树路径**，打包后不存在，
//! 所以它只在 `include_str!`、播种源和测试里出现 —— 运行期绝不读它。
//!
//! ## 主题标识：持久化用 id，应用用显示名
//!
//! 注册表按 `themes[].name`（显示名）索引，但**设置文件里存的是 `themes[].id`**
//! （`lithe-dark`）：名字是给人看的、随时可能改，id 改名不变。两者的映射由
//! [`crate::schema::ThemeIndex`] 表达，来源是主题文件的原始 JSON ——
//! `ThemeConfig` 里没有 id 字段（它是我们加进文件、被上游静默忽略的）。
//!
//! ## 两个必须记住的坑（沿用原来 `main.rs` 的结论）
//!
//! 1. 主题文件的根是 **`ThemeSet`**（`{ name, author?, themes: [ThemeConfig] }`）而不是
//!    `ThemeConfig`；解析失败的文件会被**整份静默忽略**（`theme/registry.rs:252-258`），
//!    所以"主题没生效"往往一点报错都没有。
//! 2. **必须先切 `ThemeMode` 再 `apply_config`**：`apply_config` 只把配置写进
//!    `light_theme` / `dark_theme` 两个槽里按 `config.mode` 对应的那一个
//!    （`theme/schema.rs:1060-1065`），而实际渲染用的是**当前 `ThemeMode`** 对应的槽。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::component::{Theme, ThemeConfig, ThemeMode, ThemeRegistry};
use gpui_kit::{App, SharedString, WindowAppearance, px};

use crate::paths;
use crate::schema::{ThemeIndex, slugify_theme_id, theme_font_size_for};

/// 内置主题：`(文件名, 文件内容)`。
///
/// 内容用 `include_str!` **编译进二进制**，所以打包后照样能播种 —— 这正是旧实现
/// （运行期读 `CARGO_MANIFEST_DIR/../../themes`）在安装包里失效的原因。
///
/// ⚠️ 加一份主题文件时**必须在这里登记**，否则它只存在于仓库里、运行期看不到。
/// `every_theme_file_is_embedded_and_loadable` 会拿 [`paths::repository_themes_dir`] 的目录
/// 内容与这张表对比，漏登记就是测试失败。
pub const BUNDLED_THEMES: &[(&str, &str)] = &[
    ("gruvbox.json", include_str!("../../../themes/gruvbox.json")),
    (
        "jetbrains.json",
        include_str!("../../../themes/jetbrains.json"),
    ),
    (
        "lithe-dark.json",
        include_str!("../../../themes/lithe-dark.json"),
    ),
    (
        "lithe-light.json",
        include_str!("../../../themes/lithe-light.json"),
    ),
    ("nord.json", include_str!("../../../themes/nord.json")),
    ("one.json", include_str!("../../../themes/one.json")),
    ("vscode.json", include_str!("../../../themes/vscode.json")),
];

/// 一次播种的结果（诊断与测试用）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeedReport {
    /// 这次新写入的文件名（用户目录里本来没有）。
    pub created: Vec<String>,
    /// 已经存在、**原样保留**的文件名（用户可能改过，绝不覆盖）。
    pub kept: Vec<String>,
    /// 写失败的文件名。
    pub failed: Vec<String>,
}

impl SeedReport {
    /// 一次性写进日志行的一行摘要。
    pub fn summary(&self) -> String {
        format!(
            "created={} kept={} failed={}",
            self.created.len(),
            self.kept.len(),
            self.failed.len()
        )
    }
}

/// 把内置主题播种到 `dir`（不存在就建）。已存在的文件**不动**。
///
/// 纯文件操作、不碰 GPUI，所以可以直接单测（见本文件的测试）。
///
/// ## 为什么每一份都必须**原子写**（tmp + rename）
///
/// 播种规则是"已存在就保留、**绝不覆盖**"。那条规则保护的是用户的修改，但它同时意味着
/// **一份写坏的产物永远修不回来**：如果这里直接用 `fs::write`，而进程在写到一半时崩溃或断电，
/// 目录里就会留下一个**截断的 JSON**；下次启动看到"文件已存在"就跳过，于是这份内置主题
/// **永久损坏**（注册表整份忽略它 → `S1_THEME missing`），而用户没有明显线索 ——
/// 文件好端端在那儿，他还得自己想到去删。
///
/// 所以这里走与本仓其它 JSON 落盘同一条路（[`crate::persistence::tmp_path`] + `rename`）：
/// 断电最坏只留下一个 `.tmp`（注册表只认 `.json` 扩展名，不会读它），
/// 而目标文件要么不存在、要么是**完整**的。
///
/// ⚠️ **刻意不做的事**：不会因为"已存在的文件解析失败"就重新播种。那会把用户**手写坏**的
/// 主题悄悄换成内置版 —— 比留着一个坏文件更糟（用户的意图被无声覆盖）。坏文件的出路是用户
/// 删掉它，或者以后另做一个显式的"恢复内置主题"动作。
pub fn seed_bundled_themes_into(dir: &Path) -> SeedReport {
    let mut report = SeedReport::default();

    if let Err(error) = std::fs::create_dir_all(dir) {
        eprintln!("S1_THEME seed_failed dir={} error={error}", dir.display());
        report
            .failed
            .extend(BUNDLED_THEMES.iter().map(|(name, _)| (*name).to_string()));
        return report;
    }

    for (name, content) in BUNDLED_THEMES {
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

/// 原子写一份**字节**：先写 `<文件>.tmp`，再 `rename` 覆盖目标。
///
/// 为什么不复用 [`crate::persistence::save_json`]：那个函数收的是 `Serialize` 值，
/// 会把内容**重新序列化**（key 顺序与缩进都可能变），而内置主题必须与 `include_str!`
/// 的内容**逐字相等** —— `every_theme_file_is_embedded_and_loadable` 正是在断言这一点。
/// 所以这里只借它的临时文件名规则（[`crate::persistence::tmp_path`]），不借它的序列化。
///
/// `rename` 失败时残留的 `.tmp` 不清理（与 `save_json` 同口径）：下次播种会把它覆盖掉
/// （目标仍不存在），而且注册表只扫 `.json`，不会把 `.tmp` 当成主题。
fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = crate::persistence::tmp_path(path);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// 把内置主题播种到运行时主题目录（[`paths::themes_dir`]）。
///
/// 返回 `None` = 推导不出主题目录（配置目录都拿不到），调用方应当跳过装载并留诊断。
pub fn seed_bundled_themes() -> Option<SeedReport> {
    let dir = paths::themes_dir()?;
    let report = seed_bundled_themes_into(&dir);
    println!("S1_THEME seeded dir={} {}", dir.display(), report.summary());
    Some(report)
}

/// 只由**内置主题**构成的索引（编译期内容，零文件 IO），缓存一份。
fn bundled_index() -> &'static ThemeIndex {
    static INDEX: OnceLock<ThemeIndex> = OnceLock::new();
    INDEX.get_or_init(|| {
        ThemeIndex::from_documents(
            BUNDLED_THEMES
                .iter()
                .map(|(_, content)| (*content).to_string())
                .collect::<Vec<_>>(),
        )
    })
}

/// 读一个目录里**顶层**的 `*.json` 文本，按文件名排序（顺序确定）。
///
/// 只在顶层收集，与注册表的 `reload()` 一致（它 `read_dir` + `path.is_file()` + 扩展名
/// `json`，不递归；子目录里的文件不会被装载，见 `gpui/themes/README.md` §1.1）。
/// 读取失败的文件跳过：注册表那边也会忽略它。
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

/// 用户主题目录的修改时间（用于缓存失效判定）。
fn user_themes_stamp(dir: &Path) -> Option<SystemTime> {
    std::fs::metadata(dir)
        .ok()
        .and_then(|meta| meta.modified().ok())
}

/// 主题索引缓存的类型别名（键是用户主题目录的 mtime）。
type ThemeIndexCache = Mutex<Option<(Option<SystemTime>, ThemeIndex)>>;

/// 取缓存锁。中毒时取回内部值继续用：这张表是可重建的派生数据，没有"半写坏"的风险
/// （最坏是重算一次），所以不该因为它 panic 而让设置界面挂掉。
fn lock_index_cache(
    cache: &ThemeIndexCache,
) -> std::sync::MutexGuard<'_, Option<(Option<SystemTime>, ThemeIndex)>> {
    cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 当前主题索引：**用户目录在前、内置在后**（去重时先出现的赢，于是用户目录覆盖内置）。
///
/// 顺序是有意的：用户如果把播种出来的某个文件改名/改了 id，用户目录里的那一条必须在前面，
/// 否则我们把 id 解析回**播种时**的旧名字，而注册表里只有新名字 → 应用失败。
///
/// ## 为什么要缓存
///
/// 设置对话框的主题下拉在**渲染路径**上问它（`dialog::theme_choices`），那里不能每次
/// `read_dir` + 读 7 份文件。这里以"用户主题目录的 mtime"为缓存键：目录内容变了就重读，
/// 没变就复用（一次 `metadata` 系统调用）。内置部分本来就在二进制里（[`bundled_index`]）。
///
/// ⚠️ 已知边界：mtime 不变而**同一个文件内部**改了主题显示名时，这张表可能晚一拍
/// （目录 mtime 不变）。它不影响"当前已选主题能不能应用"（那条路在
/// [`resolve_name_for_id`] 里还有一次按注册表 + slug 的兜底），最坏是下拉里晚一帧才更新。
pub fn theme_index() -> ThemeIndex {
    static CACHE: OnceLock<ThemeIndexCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));

    let Some(dir) = paths::themes_dir() else {
        return bundled_index().clone();
    };
    let stamp = user_themes_stamp(&dir);

    // 先看缓存。`return` 在守卫作用域里，Guard 会正常释放（下面那次加锁不会死锁）。
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
        BUNDLED_THEMES
            .iter()
            .map(|(_, content)| (*content).to_string()),
    );
    let index = ThemeIndex::from_documents(documents);
    *lock_index_cache(cache) = Some((stamp, index.clone()));
    index
}

/// 注册表里当前可选的**主题 id**（顺序 = 注册表的稳定序：默认主题在前、浅色在前、名字不区分大小写）。
///
/// 判据与 [`resolve_name_for_id`] 互补：这里是"名字 → id"（索引优先、否则 slug 兜底），
/// 那里是"id → 名字"。两边用同一套派生规则，所以"下拉里选得出的 id"必定能被应用。
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

/// 主题下拉的候选项：`(id, 显示名)`，顺序与 [`theme_ids`] 一致。
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

/// 一个值（id **或**显示名）对应的显示名；两者都不认识时原样返回。
///
/// 设置对话框用它给"当前值不在候选里"的那一项做标签（照 Windows 的
/// `normalizedThemeOptions`：设置文件里写了一个已卸载的主题时，下拉不能显示空值）。
pub fn display_name_for(value: &str) -> String {
    let index = theme_index();
    index
        .canonicalize(value)
        .and_then(|id| index.name_for_id(&id).map(str::to_string))
        .unwrap_or_else(|| value.to_string())
}

/// 把一个值（**id 或显示名**）归一成 id；两者都不认识时**原样返回**。
///
/// 调用方是设置界面与 `--theme`：用户可能给 id（新设置文件 / 下拉）、也可能给显示名
/// （升级前的设置文件、命令行手输）。不认识时原样返回是有意的 —— 那是用户自己写的值，
/// 不该在我们手里被悄悄改成别的东西；"它到底存不存在"由调用方按注册表判定并回落默认。
pub fn canonical_theme_id(cx: &App, value: &str) -> String {
    let name = resolve_name_for_id(cx, value);
    theme_index()
        .id_for_name(&name)
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

/// 把 id 解析成注册表里的**显示名**（注册表的索引键）。
///
/// 两层：先问索引（能处理"改名但保留 id"的主题），再退回"在注册表里找 slug 相同的那个"
/// （索引晚一拍时仍然能应用，见 [`theme_index`] 的缓存说明）。都不命中时原样返回，
/// 由 [`apply_theme_by_name`] 打 `S1_THEME missing`。
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

/// 装载运行时主题目录并监听它；**目录重新加载后按"当前生效主题"复原**。
///
/// 顺序（缺一不可）：
/// 1. [`seed_bundled_themes`] —— 内置主题必须先落到目录里，否则注册表看不到它们（见模块文档）；
/// 2. `ThemeRegistry::watch_dir` —— 它自己会先 `reload_themes` 再调回调
///    （`theme/registry.rs:104-115`），所以回调里 `themes()` 已经是新的一份。
///
/// 回调不记住某一次的主题名，而是问 [`crate::store::SettingsStore`] "现在应该是什么主题"
/// —— 这样设置界面改过主题之后再热改主题文件也不会退回旧主题。
pub fn watch_lithe_themes(cx: &mut App) {
    // 播种失败也继续：用户目录里可能有他自己的主题，能装载多少算多少。
    if seed_bundled_themes().is_none() {
        eprintln!("S1_THEME seed_skipped reason=no_themes_dir");
    }

    let Some(themes_dir) = paths::themes_dir() else {
        eprintln!("S1_THEME watch_skipped reason=no_themes_dir");
        return;
    };

    let result = ThemeRegistry::watch_dir(themes_dir.clone(), cx, move |cx| {
        // 注册表在这一刻可能还没有 `SettingsStore`（启动顺序见 `main.rs`），
        // 拿不到就退回默认主题，至少让主题目录里的主题能被应用一次。
        match crate::store::try_store(cx) {
            Some(store) => {
                store.update(cx, |store, cx| store.reapply_theme(cx));
            }
            None => {
                let id = crate::schema::DEFAULT_THEME;
                if !apply_theme_by_id(cx, id) {
                    eprintln!("S1_THEME missing id={id}");
                }
            }
        }
    });

    match result {
        Ok(()) => println!("S1_THEME watching dir={}", themes_dir.display()),
        Err(error) => eprintln!(
            "S1_THEME watch_failed dir={} error={error}",
            themes_dir.display()
        ),
    }
}

/// 按 **id** 应用一个主题；`false` = 注册表里找不到对应的主题（**不改动当前主题**）。
///
/// 这是设置界面与 `--theme` 的入口：两者拿到的都可能是 id，也可能是显示名
/// （老设置文件、命令行手输），[`resolve_name_for_id`] 两种都收。
pub fn apply_theme_by_id(cx: &mut App, id: &str) -> bool {
    let name = resolve_name_for_id(cx, id);
    if name != id {
        println!("S1_THEME resolved id={id} name={name}");
    }
    apply_theme_by_name(cx, &name)
}

/// 按**显示名**应用一个主题；`false` = 注册表里没有这个名字（**不改动当前主题**）。
///
/// 顺序（缺一不可，逐条理由见模块文档）：
/// 1. `Theme::change(mode)` —— 先让当前 `ThemeMode` 指向要应用的那个槽；
/// 2. `apply_config` —— 把 Lithe 的配置写进该槽并应用到当前主题；
/// 3. 让调用方随后覆盖 `font_size`（`apply_config` 会把主题文件里的 `font.size` 写回来）；
/// 4. `sync_base` —— 把颜色/圆角/滚动条样式推给 Base 层（直接改 `Theme::global_mut` 的字段后
///    不调它，滚动条会继续用旧样式）；
/// 5. `refresh_windows` —— 让所有窗口重绘（`Root::render` 每帧会按 `font_size` 重设 rem）。
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
    stamp_builtin_highlight_if_absent(cx, &theme, mode);
    Theme::global_mut(cx).apply_config(&theme);
    Theme::sync_base(cx);
    cx.refresh_windows();
    println!("S1_THEME applied={name} dark={}", mode.is_dark());
    true
}

/// 目标主题**没有** `highlight` 段时，把 `highlight_theme` 复位成该明暗的内置那一份。
///
/// 为什么必须有这一步：`apply_config` 只在 `config.highlight` 是 `Some` 时整段替换
/// `highlight_theme`（`gpui-component-0.6.6/src/theme/schema.rs:1066-1073`），**没有**按字段合并；
/// 而 `Theme::change(mode)` 重放的是"该明暗槽里存着的那个配置"，对没有 `highlight` 的主题
/// （`gpui/themes/lithe-*.json`）同样不会清掉上一份。于是：
///
/// `Lithe Dark`（无 `highlight`）→ `Gruvbox Light`（有）→ 切回 `Lithe Dark`
/// 会把 **Gruvbox Light 的浅色语法色留在深色底上**（深底深字）。这不是"主题没生效"，
/// 而是编辑器那一层沿用了上一个主题的高亮表。
///
/// 复位源是注册表的 `default_themes()`：它就是内置的 `Default Light` / `Default Dark`
/// （`theme/registry.rs:139-149`），两者都带 `highlight`。复位发生在 `apply_config`
/// **之前**，所以带 `highlight` 的主题不会被这里影响。
///
/// ⚠️ 覆盖不到的地方：`Theme::change` 内部会把当时的 `highlight_theme` 拷进
/// `install_text_view_defaults`（`src/text/mod.rs:64-75`，供 TextView 的 markdown 代码块用），
/// 这一步在复位之前，所以那些代码块仍可能拿着上一份。本仓库目前没有任何 TextView / markdown
/// 代码块消费方，编辑器正文每帧读 `cx.theme().highlight_theme`，所以这条不影响当前界面。
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

/// 把 UI 字号写进主题的 rem 基准并刷新窗口。
///
/// `font_size = 16 × (uiFontSize / 13)`，见 [`theme_font_size_for`]。**必须在
/// [`apply_theme_by_name`] 之后调用**，因为 `apply_config` 会用主题文件里的 `font.size` 覆盖它。
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

/// 把「编辑器字号」写进主题的等宽字号并刷新窗口。
///
/// 落点是 `Theme::mono_font_size`：gpui-kit 的编辑器正文就用它
/// （`gpui-component-0.6.6/src/input/editor.rs:137-143`），所以改这一个 token
/// 就是"编辑器字号立即生效"。
///
/// 与 [`apply_theme_font_size`] 同一口径：**必须在 [`apply_theme_by_name`] 之后调用**
/// （主题文件里若写了 `font.mono_size`，`apply_config` 会先覆盖一次）。
pub fn apply_editor_font_size(cx: &mut App, editor_font_size: f64) {
    let font_size = px(editor_font_size as f32);
    Theme::global_mut(cx).mono_font_size = font_size;
    Theme::sync_base(cx);
    cx.refresh_windows();
    println!("S1_THEME mono_font_size={font_size:?} editor_font_size={editor_font_size}");
}

/// GPUI 自己解析的虚拟家族：它不是一个真实安装的字族，但**永远可用**，所以校验时放行。
pub const SYSTEM_UI_FONT_FAMILY: &str = ".SystemUIFont";

/// 把两个字族覆盖值写进主题 token；空串 = **不覆盖**（保留主题文件里的值）。
///
/// 落点：`Theme::font_family`（界面正文）与 `Theme::mono_font_family`（编辑器
/// `gpui-component-0.6.6/src/input/editor.rs:141`、终端
/// `gpui/crates/terminal/src/terminal_view.rs`）。
///
/// 与两个字号函数同一条口径：**必须在 [`apply_theme_by_name`] 之后调用**
/// （主题文件里若写了 `font.family` / `font.mono_family`，`apply_config` 会先覆盖一次）。
///
/// ## 为什么必须先校验"这个字族装没装"
///
/// GPUI 在**首次布局一行、而该字族找不到**时会 panic
/// （`gpui-component-0.6.6/src/theme/mono_font.rs:1-13` 就是为这件事存在的：连
/// `Font::fallbacks` 都救不了，它只在字族本身加载成功后才作为缺字回退链被查）。而这两个键是
/// **用户在设置里填的字符串**，一次手写错就能让应用再也起不来。所以：
///
/// - 认不出的字族**不写入**，保留当前值并留一条
///   `S1_THEME font_family_rejected` 诊断；
/// - 空串不写入（= 不覆盖）；
/// - 校验用系统已装字族列表，它按进程缓存一次（枚举字体在 macOS 上要上百毫秒，
///   同 `mono_font.rs` 的做法）。
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

/// 进程内缓存一次系统已装字族（枚举字体在 macOS 上要上百毫秒）。
///
/// 与 `gpui-component-0.6.6/src/theme/mono_font.rs:69-72` 同一个理由与同一种做法；
/// 那边的函数是 `pub(super)`，拿不到，所以这里自己缓存一份。
pub fn installed_font_names(cx: &App) -> &'static [String] {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES.get_or_init(|| cx.text_system().all_font_names())
}

/// 一个字族值能不能安全写进主题：`Some(值)` = 可以，`None` = 不覆盖或不可用。
///
/// 纯函数，`installed` 由调用方给（测试因此不需要真实文本系统）。
/// `.SystemUIFont` 是虚拟家族，永远放行。
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

/// 注册表里当前可选的**主题显示名**（注册表的稳定序）。
///
/// 保留显示名这条 API 是因为**菜单栏**（`gpui/crates/workbench/src/menu_bar.rs`）用它渲染
/// 主题子菜单的条目文字，并把点中的名字交给 `SettingsStore::set_theme_explicit` ——
/// 那条路内部会把名字归一成 id，所以菜单栏不需要知道 id 的存在。
pub fn theme_names(cx: &App) -> Vec<String> {
    ThemeRegistry::global(cx)
        .sorted_themes()
        .into_iter()
        .map(|theme| theme.name.to_string())
        .collect()
}

/// 某个主题（**id 或显示名**）在注册表里是不是深色主题；两者都不认识返回 `None`。
///
/// 设置界面在两处要用它：
/// 1. 「配色主题」下拉在**跟随系统**时要把选择写进 `autoThemeDark` 还是 `autoThemeLight`；
/// 2. [`crate::store::SettingsStore::appearance_mode`] 需要判断当前主题的明暗。
pub fn theme_is_dark(cx: &App, value: &str) -> Option<bool> {
    let name = resolve_name_for_id(cx, value);
    ThemeRegistry::global(cx)
        .themes()
        .get(&SharedString::from(name))
        .map(|theme| theme.mode.is_dark())
}

/// 系统当前是否处于深色外观。
///
/// 用 `Window::appearance()` 的同源 API `App::window_appearance()`，映射规则与 gpui-kit
/// 自己的 `impl From<WindowAppearance> for ThemeMode`
/// （`gpui-component-0.6.6/src/theme/mod.rs:722-727`）逐字一致。
pub fn system_is_dark(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 字体族校验：空串 = 不覆盖；`.SystemUIFont` 是虚拟家族，永远放行；
    /// 未安装的名字被拒（**这一条是防崩溃的**：GPUI 在字族缺失时首次布局就 panic）。
    #[test]
    fn usable_family_only_accepts_empty_virtual_or_installed() {
        let installed = vec!["Inter".to_string(), "JetBrains Mono".to_string()];

        // 空串 / 只有空白 = 不覆盖（`None`，调用方保留主题里的值）。
        assert_eq!(usable_family("fontFamily", "", &installed), None);
        assert_eq!(usable_family("fontFamily", "   ", &installed), None);

        // 装了的字族：原样放行（首尾空白被去掉）。
        assert_eq!(
            usable_family("fontFamily", "Inter", &installed),
            Some("Inter")
        );
        assert_eq!(
            usable_family("monoFontFamily", " JetBrains Mono ", &installed),
            Some("JetBrains Mono")
        );

        // 虚拟家族不受已装列表限制。
        assert_eq!(
            usable_family("fontFamily", SYSTEM_UI_FONT_FAMILY, &installed),
            Some(SYSTEM_UI_FONT_FAMILY)
        );

        // 没装的字族被拒（返回 `None`，调用方不写主题 token）。
        assert_eq!(
            usable_family("fontFamily", "No Such Font", &installed),
            None
        );
    }

    /// 一个"进程内唯一、用后即删"的临时目录。
    ///
    /// 名字用「用例名 + 进程 id」而不是时钟/随机数：测试要确定性，同名冲突只可能来自同一个
    /// 进程里的不同用例，而它们各自用不同的 `case`。`Drop` 保证断言失败时也会清理。
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

    /// 内置主题清单必须覆盖仓库 `gpui/themes/` 里的**每一份** `*.json`，而且每份都能被
    /// 真实的 `ThemeSet` 反序列化、每条主题都有非空且唯一的 id。
    ///
    /// 这条测试守两件**静默失效**：
    /// 1. 往目录里丢了新主题文件却忘了在 [`BUNDLED_THEMES`] 登记 → 运行期看不到它（打包后更看不到）；
    /// 2. 忘了写 `id` → 运行期只能按名字派生 id，改名就会断开用户的引用。
    #[test]
    fn every_theme_file_is_embedded_and_loadable() {
        use gpui_kit::component::ThemeSet;
        use std::collections::BTreeMap;

        let dir = paths::repository_themes_dir();
        let mut files: Vec<String> = std::fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("read_dir failed dir={} error={error}", dir.display()))
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

        let embedded: BTreeMap<&str, &str> = BUNDLED_THEMES.iter().copied().collect();
        assert_eq!(
            embedded.len(),
            BUNDLED_THEMES.len(),
            "BUNDLED_THEMES 里有重复的文件名"
        );

        let mut seen_ids: BTreeMap<String, String> = BTreeMap::new();
        let mut theme_count = 0usize;

        for file in &files {
            let content = embedded
                .get(file.as_str())
                .unwrap_or_else(|| panic!("{file} 没有登记进 BUNDLED_THEMES（打包后读不到它）"));

            // 登记的内容必须与磁盘上的文件逐字一致：否则"播种出去的内置主题"与仓库里那份
            // 不是同一个东西，排查时会对着两份不同的文件。
            let on_disk = std::fs::read_to_string(dir.join(file)).expect("读主题文件失败");
            assert_eq!(
                content, &on_disk,
                "{file} 的 include_str! 内容与磁盘上的文件不一致"
            );

            let set: ThemeSet = serde_json::from_str(content)
                .unwrap_or_else(|error| panic!("{file}: not a loadable ThemeSet: {error}"));
            assert!(!set.themes.is_empty(), "{file}: themes[] is empty");

            // `ThemeSet` 会忽略未知字段，所以 id 要用原始 JSON 取。
            let index = ThemeIndex::from_documents(vec![(*content).to_string()]);
            assert_eq!(
                index.entries().len(),
                set.themes.len(),
                "{file}: 每条主题都必须有可解析的 id/name"
            );
            for entry in index.entries() {
                assert!(
                    !entry.id.trim().is_empty(),
                    "{file}: 主题 {:?} 的 id 是空的",
                    entry.name
                );
                if let Some(owner) = seen_ids.insert(entry.id.clone(), file.clone()) {
                    panic!("{file}: 主题 id {:?} 已在 {owner} 里用过", entry.id);
                }
                theme_count += 1;
            }
        }

        assert_eq!(
            files.len(),
            BUNDLED_THEMES.len(),
            "BUNDLED_THEMES 登记的文件数与 gpui/themes 里的不一致：{files:?}"
        );
        assert!(theme_count >= 12, "主题条数太少：{theme_count}");
    }

    /// `apply_config` **没有 fallback** 的 13 个颜色 key（`theme/schema.rs:685`、`:717-719` 的宏）。
    ///
    /// 少写这些 key 不会报错，但缺失时会去读编译进二进制的 shadcn 默认值，把中性灰/蓝
    /// 漏进主题（`schema.rs:740`、`:774-777`、`:802`、`:819`、`:1016`、`:743-772`）。
    /// 与 `gpui/themes/README.md` §1.5 是同一份清单。
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

    /// `gpui/themes/` 里每一份 JSON 都必须能被**真实的** `ThemeSet` 反序列化，并且
    /// 主题名非空且全局唯一、13 个无 fallback 的 key 齐全、每个颜色值 gpui 都解析得了。
    ///
    /// 这四件事全是**静默失效**，只会在界面上表现为"主题没出现"或"某个角还是 shadcn 的灰"：
    /// 1. 解析失败的文件被整份忽略，只打一行日志（`theme/registry.rs:252-258`）；
    /// 2. 同名主题条目被跳过（`registry.rs:270-273`），所以重名就是"少一个主题"；
    /// 3. 无 fallback 的 key 缺失会漏进内置默认色（见 [`REQUIRED_COLOR_KEYS`]）；
    /// 4. 颜色解析器只认 `#RRGGBB` / `#RRGGBBAA` / Tailwind 色名，**不认** `rgba(...)`
    ///    （`theme/color.rs:693-697`、`:763`）。
    ///
    /// 走的是真类型（`ThemeSet` / 真解析器），不是另写一份 schema，所以上游改了字段形状
    /// 这里会一起失败。纯同步读文件 + 反序列化，没有等待、没有全局状态。
    ///
    /// ⚠️ 读的是**仓库**目录（`gpui/themes`，只服务测试与 `include_str!`），不是运行时目录 ——
    /// 运行时的目录由用户决定内容，不能拿来做仓库契约的断言。
    #[test]
    fn every_theme_file_is_a_loadable_theme_set() {
        use gpui_kit::component::{ThemeSet, try_parse_background, try_parse_color};
        use std::collections::BTreeMap;

        let dir = paths::repository_themes_dir();
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("read_dir failed dir={} error={error}", dir.display()))
            .map(|entry| entry.expect("read_dir entry").path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect();
        files.sort();
        assert!(!files.is_empty(), "no theme json in {}", dir.display());

        // 主题名 -> 定义它的文件：注册表按名字去重，先把重名找出来。
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

                // 序列化回 JSON 是为了按 **JSON key** 取值：`None` 会出现成 `null`，
                // 于是"这个 key 生效了没有"和文件的字面量对得上（`schema.rs` 用 `rename`）。
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
                    // 没写的 key 反序列化成 `None`、再序列化回来就是 `null`：那是合法的
                    // （走 `apply_config` 的 fallback），只有"写了但 gpui 解析不了"才是错。
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

        assert!(theme_count > 0, "no theme defined in {}", dir.display());
    }

    /// 内置主题的 id 必须等于**仓库里那份文件**写的 id（改错一个字母就是"用户的主题不见了"）。
    #[test]
    fn bundled_index_maps_ids_to_the_expected_names() {
        let index = bundled_index();
        assert_eq!(index.name_for_id("lithe-dark"), Some("Lithe Dark"));
        assert_eq!(index.name_for_id("lithe-light"), Some("Lithe Light"));
        assert_eq!(index.name_for_id("darcula"), Some("Darcula"));
        assert_eq!(index.name_for_id("vs-code-light"), Some("VS Code Light+"));
        assert_eq!(index.id_for_name("Nord Dark"), Some("nord-dark"));
        assert_eq!(index.entries().len(), 12, "12 条内置主题");
    }

    /// 播种：第一次全部新建；第二次一个都不动（用户的修改不能被覆盖）。
    #[test]
    fn seeding_creates_missing_files_and_never_overwrites() {
        let dir = TempDir::new("seed");

        let first = seed_bundled_themes_into(dir.path());
        assert_eq!(first.created.len(), BUNDLED_THEMES.len());
        assert!(first.kept.is_empty());
        assert!(first.failed.is_empty(), "{first:?}");
        for (name, content) in BUNDLED_THEMES {
            let written = std::fs::read_to_string(dir.path().join(name)).expect("播种产物读不到");
            assert_eq!(&written, content, "{name} 的播种内容不一致");
        }

        // 用户改了其中一份，并删掉另一份。
        let edited = dir.path().join("lithe-dark.json");
        std::fs::write(&edited, "{\"name\":\"mine\",\"themes\":[]}").expect("改写失败");
        let removed = dir.path().join("nord.json");
        std::fs::remove_file(&removed).expect("删除失败");

        let second = seed_bundled_themes_into(dir.path());
        assert_eq!(
            second.created,
            vec!["nord.json".to_string()],
            "只补回缺失的那一份"
        );
        assert_eq!(second.kept.len(), BUNDLED_THEMES.len() - 1);
        assert_eq!(
            std::fs::read_to_string(&edited).expect("读失败"),
            "{\"name\":\"mine\",\"themes\":[]}",
            "已存在的文件绝不能被覆盖"
        );
    }

    /// 播种会在目录不存在时建出来（首次启动的路径）。
    #[test]
    fn seeding_creates_the_directory_when_missing() {
        let dir = TempDir::new("seed-missing");
        let nested = dir.path().join("themes");
        assert!(!nested.exists());

        let report = seed_bundled_themes_into(&nested);
        assert_eq!(report.created.len(), BUNDLED_THEMES.len());
        assert!(nested.is_dir());
    }

    /// 播种必须走**原子写**：目录里不残留 `*.tmp`，产物正是内置内容本身。
    ///
    /// 这条守的是崩溃语义：播种规则是"已存在就保留、绝不覆盖"，所以一份**截断的**产物
    /// 永远修不回来（用户还没有线索）。原子写让断电最坏只留下一个 `.tmp`，
    /// 目标文件要么不存在、要么是完整的 —— 后者正是"下次启动看到文件存在就跳过"能够成立的前提。
    ///
    /// 断言用的是 `persistence::tmp_path` 的命名规则本身，所以"另起一套临时文件名"也会失败。
    #[test]
    fn seeding_writes_atomically_and_leaves_no_temporary_files() {
        let dir = TempDir::new("seed-atomic");
        let report = seed_bundled_themes_into(dir.path());
        assert_eq!(report.created.len(), BUNDLED_THEMES.len());
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

        for (name, content) in BUNDLED_THEMES {
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

    /// 索引顺序：**用户目录在前**，于是"改了播种文件的显示名"能覆盖内置那一条。
    #[test]
    fn user_directory_documents_win_over_bundled_ones() {
        let dir = TempDir::new("index-order");
        std::fs::write(
            dir.path().join("lithe-dark.json"),
            r#"{"name":"mine","themes":[{"id":"lithe-dark","name":"My Dark","mode":"dark"}]}"#,
        )
        .expect("写入失败");

        let mut documents = theme_documents_in(dir.path());
        documents.extend(
            BUNDLED_THEMES
                .iter()
                .map(|(_, content)| (*content).to_string()),
        );
        let index = ThemeIndex::from_documents(documents);

        assert_eq!(
            index.name_for_id("lithe-dark"),
            Some("My Dark"),
            "用户目录里的那条必须在前面"
        );
        // 用户目录里没有的主题仍由内置那份提供。
        assert_eq!(index.name_for_id("nord-dark"), Some("Nord Dark"));
    }

    /// 只收集顶层的 `*.json`，顺序按文件名（与注册表的 `reload` 一致）。
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

    /// 系统外观 → 明暗：四个变体逐个钉住（Vibrant* 也属于 mac 的深浅两态）。
    #[test]
    fn appearance_maps_to_dark_and_light() {
        assert!(system_is_dark(WindowAppearance::Dark));
        assert!(system_is_dark(WindowAppearance::VibrantDark));
        assert!(!system_is_dark(WindowAppearance::Light));
        assert!(!system_is_dark(WindowAppearance::VibrantLight));
    }
}
