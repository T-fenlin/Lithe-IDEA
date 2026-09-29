//! Lithe 的资源源：内嵌 `lithe-gpui/assets/**`，未命中回落 gpui-kit 的 `AllAssets`。
//!
//! ## 为什么需要这一层
//!
//! `Application::with_assets`（`gpui-pre-0.3.6/src/app.rs:198-206`）**只收一个** `AssetSource`：
//! 它把值 `Arc::new` 成 `Arc<dyn AssetSource>` 存进 `App`，重复调用只是覆盖。而
//! `gpui_kit::assets::AllAssets` 只嵌了 `icons/**/*.svg`（1830 个 Lucide 字形，
//! `gpui-kit-assets-0.6.6/src/native_assets.rs:8-11`），**看不到**我们从 Windows 前端搬进来的
//! `lithe-gpui/assets/ui-icons/**` 与 `lithe-gpui/assets/icon-themes/**`。所以必须自己实现一个包装层，
//! 在里面做"先查自己的、再回落"的组合。官方 0.6.6 文档就是这么建议的
//! （`lithe-gpui/docs/gpui-kit/0.6.6/zh-CN/docs/assets.md` 的「自定义资源」一节），
//! `gpui-kit-assets-0.6.6/examples/extra_assets.rs:23-37` 是同型例子。
//!
//! ## 只有注册在这里（四步里的第 1 步）
//!
//! 按主题明暗挑图标、把路径交给 `gpui::svg().path(..)` 的 helper 是
//! `lithe_gpui_shared::icons::idea_icon_svg`（四步里的第 3 步）—— 它**不能**放在本 crate：
//! 依赖方向是 `app → settings → shared`，feature 拿不到 app 的公开项。
//!
//! ## 两个方法各自的关键点
//!
//! - [`AssetSource::load`]：`AllAssets::load` 在**未命中时返回 `Err`**（不是 `Ok(None)`，
//!   `native_assets.rs:27-29` 的 `ok_or_else(|| anyhow!(..))`）。所以回落**不能**用 `?` 透传
//!   —— 那样"查一个不存在的字形"会变成硬错误，而约定是"加载失败"应该是 `Ok(None)`。
//!   这里的做法是把 `Err` 折叠成 `Ok(None)`。这也是 `lithe-gpui/research/app-icon-and-assets.md`
//!   §4.4 记下的坑。
//! - [`AssetSource::list`]：`list` 未命中是 `Ok(vec![])`，可以放心透传；但两边都可能命中同一
//!   路径，所以必须**合并 + 排序 + 去重**（官方文档 `assets.md:38-42` 的同口径）。
//!   未排序的列表会让依赖 `list` 顺序的诊断输出不稳定。
//!
//! ## 内嵌范围与代价
//!
//! 嵌的是 `lithe-gpui/assets/**` **减去三套未接线的图标包，再减去根目录的 `README.md`**。
//! `LitheAssets::iter().count()` 实测 **269 个文件** = **269 个资源文件**（`icons/**` 7 个位图
//! + `images/logo.png` 1 个 + `ui-icons/**` 157 个 SVG + `icon-themes/idea/**` 104 个文件），
//! 合计 **3 200 771 字节（约 3.05 MiB）**。
//!
//! `README.md`（29 798 字节）由 `#[exclude]` 挡在二进制外：它是**文档不是资源**，
//! 而且它一变，所有"总文件数 / 总字节数"就跟着漂 —— 它挂在 `#[folder]` 范围内的那段时间里，
//! 本模块文档的"收窄前总量"就被追加内容带偏过（`8 561 207` 是旧读数，实测已是 `8 567 891`）。
//! 收窄前是 `lithe-gpui/assets/**` 全量（含 `README.md` 与三套死载荷）**1 203 个文件 /
//! 8 567 891 字节（约 8.17 MiB）**。
//! 这样"资源在 `lithe-gpui/assets` 下"就是一条统一规则，不需要每加一批资源就改一次 `#[include]`。
//! 代价是二进制体积（SVG 压缩率高，实测影响见 `lithe-gpui/research/icon-asset-inventory.md` 的接线一节）。
//! 想再收窄范围，`#[exclude]` / `#[include]` 是唯一的开关位置（见 `LitheAssets` 的属性）。
//!
//! ### 归档目录（随旧前端删除而迁入，全部 `#[exclude]`）
//!
//! 旧前端（`windows/`、`macos/`）删除前，只存在于那两侧、gpui 侧没有副本的资源按
//! `lithe-gpui/research/legacy-frontend-removal-audit.md` 的结论迁到了 `lithe-gpui/assets/` 下，但**都还没有接线**：
//! `fonts/`（JetBrains Mono 4 字重 + OFL）、`legacy-ide-icons/`（macOS 侧 IDEAIcons 144 文件，
//! 与 `ui-icons/idea/**` 的 expui 命名体系**不是同一套**）、`gutter-icons/`（行号旁运行/测试标记）、
//! `database-icons/`（数据库品牌图标）、`feature-icons/`（MavenIcon / RunIcon / JavaCupIcon ——
//! 旧前端只有内联 SVG 的 React 组件，没有文件对象，这里落成了 SVG）、
//! `icon-themes/material/**`。全部由 `#[exclude]` 挡在二进制外：**磁盘文件保留，接线时再去掉
//! 对应的 `#[exclude]`**。理由与既有三套图标包完全一样（见下一节），本文件 `mod tests` 里的
//! 回归测试会在有人误删这些 `#[exclude]` 时失败。
//!
//! `lithe-gpui/themes/legacy-builtin/` 同理：旧 schema 的 12 份主题真源（35 条主题）只作追溯用，
//! 生效主题仍是 `lithe-gpui/themes/*.json` 那 7 份 gpui-kit schema 文件。
//!
//! ### 为什么 `icon-themes/{lithe,pierre,symbols}` 已从内嵌范围排除
//!
//! 这三套（459 + 149 + 325 = 933 个文件 / 5 337 322 字节 / 约 5.09 MiB）**没有任何代码路径能取到**：
//! 文件图标主题 id 是编译期常量
//! `lithe_gpui_shared::icons::file_icon::ACTIVE_FILE_ICON_THEME = "idea"`（`file_icon.rs:72`），
//! 主题映射表由 `include_str!` 在编译期读入（`file_icon.rs:89,216`），仓库里也没有任何运行期
//! 枚举 `icon-themes/` 的代码（唯一的 `.list("")` 是本文件 `fallback_count` 那类启动诊断，不按主题取文件）。
//! 所以那三套只是死载荷，用 `#[exclude]` 挡在二进制外。
//! **磁盘上文件一个都没删**：将来做图标主题切换还要用它们。
//!
//! ⚠️ 要真正启用图标主题切换，得先做完前两步，再去掉对应的 `#[exclude]`：
//! ① 把 `file_icon.rs` 从"编译期常量 + `include_str!`"改成运行期主题注册表（能按主题 id 取到各包
//! `extension.json`）；② 让取图标的那一侧走 `LitheAssets::load(..)` / `list(..)`，而不是编译期内嵌的 JSON。
//! 两步都做完之前去掉 `#[exclude]` 只会白占体积，没有任何行为变化。
//! 本文件 `mod tests` 里的回归测试会在有人误删 `#[exclude]`、或把整包拷回来时失败。
//!
//! ## 与窗口 / 任务栏图标无关
//!
//! `AssetSource` 只服务 `img()`、`svg().path(..)`、以及 gpui-component 的原生菜单图标
//! （`gpui-component-0.6.6/src/native_menu/mod.rs:32,229`）。exe / 任务栏图标走 PE 资源
//! （`crates/app/lithe.rc` + `build.rs`），两者互不影响。

use std::borrow::Cow;

use gpui_kit::assets::AllAssets;
use gpui_kit::{AssetSource, Result, SharedString};

/// 内嵌 `lithe-gpui/assets/` 的 rust-embed 资产表。
///
/// 路径键**相对 `lithe-gpui/assets/`**（例如 `ui-icons/idea/expui/general/search.svg`、
/// `icons/icon.ico`、`icon-themes/idea/extension.json`、`images/logo.png`），可以直接传给
/// `svg().path(..)` 或 `img(..)`。
///
/// `#[folder]` 必须是绝对路径字符串：crate 在 `lithe-gpui/crates/app`、资源在 `lithe-gpui/assets`，
/// 相对写法 `../../assets` 在 rust-embed 里**不可靠**（它的 `get()` 直接把 `folder` 与
/// `path` 拼接后做字符串 `starts_with` 校验，路径里带 `..` 时会比出错误的边界；
/// 见 `rust-embed-impl-8.12.0/src/lib.rs:198-223`）。`$CARGO_MANIFEST_DIR` 插值由
/// `interpolate-folder-path` 特性提供 —— 特性和 `gpui-kit-assets` 打开的是同一份
/// （见 `Cargo.toml` 的注释）。
///
/// `#[exclude]` 是 rust-embed 的 1:1 + glob 开关（`include-exclude` 特性，`gpui-kit-assets`
/// 已打开同一份；见 `rust-embed-impl-8.12.0/src/lib.rs:441-451` 与
/// `rust-embed-utils-8.12.0/src/lib.rs:189-224`：`is_path_included = !exclude.matches && include.is_empty_or_matches`）。
/// 可重复出现，只影响**编译期内嵌**，不动磁盘文件。理由见模块文档。
///
/// `README.md` 是**文档不是资源**：它挂在 `#[folder]` 范围内就会进二进制，而且每编辑一次
/// 「内嵌总文件数 / 总字节数」就跟着变（模块文档里的总量曾经因此漂过一次）。这里 1:1 排除它 ——
/// globset 对 `README.md` 这种不带通配符的模式就是**精确匹配**该相对路径。
#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../assets"]
#[exclude = "icon-themes/lithe/**"]
#[exclude = "icon-themes/pierre/**"]
#[exclude = "icon-themes/symbols/**"]
#[exclude = "icon-themes/material/**"]
#[exclude = "fonts/**"]
#[exclude = "legacy-ide-icons/**"]
#[exclude = "gutter-icons/**"]
#[exclude = "database-icons/**"]
#[exclude = "feature-icons/**"]
#[exclude = "README.md"]
pub struct LitheAssets;

impl LitheAssets {
    /// 内嵌资源总数。启动诊断（`S1_ASSETS`）用它证明"包装层确实被注册且能看到内嵌文件"。
    ///
    /// `rust_embed` 的 `iter()` 是编译期生成的静态表，这个调用很便宜（不需要遍历文件系统）。
    #[must_use]
    pub fn embedded_count() -> usize {
        <Self as rust_embed::RustEmbed>::iter().count()
    }

    /// 内嵌资源里以 `prefix` 开头的路径数，用于诊断分组（例如 `ui-icons/`）。
    #[must_use]
    pub fn embedded_count_under(prefix: &str) -> usize {
        <Self as rust_embed::RustEmbed>::iter()
            .filter(|name| name.starts_with(prefix))
            .count()
    }

    /// 回落侧（gpui-kit `AllAssets` 的 Lucide 字形）能列出的资源总数。
    ///
    /// 诊断用：`S1_ASSETS` 把它和 `embedded_count()` 一起打印，一眼能看出"回落那一侧还在"。
    /// `AllAssets::list("")` 的未命中约定是 `Ok(vec![])`（不是 `Err`），所以这里不会失败；
    /// 真出错时返回 0，而不是让启动诊断本身 panic。
    #[must_use]
    pub fn fallback_count() -> usize {
        AllAssets
            .list("")
            .map(|paths| paths.len())
            .unwrap_or_default()
    }
}

/// 诊断用：走一遍真正的 [`AssetSource::load`]，返回拿到的字节数（`None` = 没找到）。
///
/// 只用来看路径键对不对 —— "能 list 出来"不等于"能用 `load` 取到"，两者在 rust-embed 里
/// 走的是不同的代码路径（`iter()` vs `get()`，后者还带一层 `starts_with` 边界校验）。
#[must_use]
pub fn probe_len(path: &str) -> Option<usize> {
    match LitheAssets.load(path) {
        Ok(Some(bytes)) => Some(bytes.len()),
        _ => None,
    }
}

/// 启动诊断 `S1_ASSETS` 探针用的路径清单 —— **只有这一处真源**。
///
/// `crates/app/src/main.rs` 的 `S1_ASSETS` 循环直接遍历它，本文件 `mod tests` 的
/// `probe_paths_are_loadable_from_the_embedded_table` 也拿它做回归输入。两处共用同一个数组是
/// 刻意的：探针路径与 `#[exclude]` / `#[include]` 是同一件事的两侧，写两份迟早漂移。
///
/// ⚠️ 这些键**必须**来自**我们内嵌的那张表**。历史回归：这里曾有一条
/// `icons/settings.svg`，而 `lithe-gpui/assets/icons/**` 下只有 7 个位图（`icon.ico` / `icon.icns` /
/// `icon.png` / `32x32.png` / `64x64.png` / `128x128.png` / `128x128@2x.png`），**一个 `.svg`
/// 都没有**。⚠️ 它当年**没有**打 `MISSING` —— [`AssetSource::load`] 会回落到 gpui-kit 的
/// Lucide，而那里**恰好也有** `icons/settings.svg`，于是探针拿到了 **586 字节**
/// （真机日志见 `lithe-gpui/research/icon-asset-inventory.md:531`）：日志看着一切正常，
/// 却完全没有证明"我们自己内嵌的键能用"。所以回归测试先断言路径在**内嵌表**里，
/// 再断言 `load` 拿得到非空字节 —— 这两条缺一不可。
///
/// 五条路径覆盖四类**真的会被界面取**的资源：
///
/// 1. `ui-icons/**`：IDE 图标，明暗两套都要能取到（`shared/src/icons/idea.rs` 按主题二选一）；
/// 2. `icons/icon.ico`：应用图标（PE 资源那份走 `crates/app/build.rs`，这一份是 `AssetSource` 侧）；
/// 3. `icon-themes/idea/extension.json`：文件类型图标主题的**清单**，由 `file_icon.rs` 的
///    `include_str!` 在编译期读入；
/// 4. `icon-themes/idea/icons/expui/fileTypes/gitignore.svg`：主题里的一个**具体图标** —— 这条是
///    "查找层算出的路径确实能 `load` 到字节"的证据。没有它，主题图标取不到字节时界面会
///    **静默回落到 Lucide**（`FileIcon::render` 的 `else` 分支），截图上看不出区别。
pub const PROBE_PATHS: &[&str] = &[
    "ui-icons/idea/expui/general/settings.svg",
    "ui-icons/idea/expui/general/settings_dark.svg",
    "icons/icon.ico",
    "icon-themes/idea/extension.json",
    "icon-themes/idea/icons/expui/fileTypes/gitignore.svg",
];

impl AssetSource for LitheAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // ① 我们自己的资源：编译期内嵌，路径名与 `lithe-gpui/assets/` 下的相对路径一一对应。
        if let Some(file) = <Self as rust_embed::RustEmbed>::get(path) {
            return Ok(Some(file.data));
        }

        // ② 回落 gpui-kit 的全量 Lucide 字形（`icons/**/*.svg`）。
        // ⚠️ `AllAssets::load` 未命中是 **Err**（`native_assets.rs:27-29`），
        // 所以这里把 Err 折叠成 `Ok(None)`；用 `?` 会把"资源不存在"变成硬错误。
        match AllAssets.load(path) {
            Ok(found) => Ok(found),
            Err(_) => Ok(None),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths: Vec<SharedString> = <Self as rust_embed::RustEmbed>::iter()
            .filter(|name| name.starts_with(path))
            .map(SharedString::from)
            .collect();
        // `AllAssets::list` 未命中返回 `Ok(vec![])`，可以直接透传。
        paths.extend(AllAssets.list(path)?);
        // 两边都可能命中同一前缀（例如空串），所以合并后必须排序 + 去重。
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::{LitheAssets, PROBE_PATHS};
    use gpui_kit::AssetSource as _;

    /// 内嵌表的全部路径键（相对 `lithe-gpui/assets/`）。`iter()` 是编译期静态表，测试里读它很便宜。
    fn embedded_paths() -> Vec<String> {
        <LitheAssets as rust_embed::RustEmbed>::iter()
            .map(|name| name.to_string())
            .collect()
    }

    /// 回归：未接线的图标包与归档目录不得再进二进制。
    ///
    /// 保护的是"整包被拷回来"或 `#[exclude]` 被误删/写错（例如把 `lithe` 拼成 `light`）这类回归：
    /// 这两种情况都不会让任何测试或编译失败，只会让二进制静默多背死载荷
    /// （三套图标包约 5.09 MiB，加归档目录约 1.79 MiB）。
    #[test]
    fn unwired_icon_themes_are_not_embedded() {
        let paths = embedded_paths();
        for prefix in [
            "icon-themes/lithe/",
            "icon-themes/pierre/",
            "icon-themes/symbols/",
            "icon-themes/material/",
            "fonts/",
            "legacy-ide-icons/",
            "gutter-icons/",
            "database-icons/",
            "feature-icons/",
        ] {
            let leaked: Vec<&str> = paths
                .iter()
                .filter(|path| path.starts_with(prefix))
                .map(String::as_str)
                .collect();
            assert!(
                leaked.is_empty(),
                "`{prefix}` 应该被 #[exclude] 挡在二进制外，但内嵌表里仍有 {} 个路径，例如 {:?}",
                leaked.len(),
                &leaked[..leaked.len().min(3)]
            );
        }
    }

    /// 回归：被代码真正引用的资源必须还在内嵌表里。
    ///
    /// 收窄 `#[exclude]` / `#[include]` 时最容易连坐删掉在用的那一批；那种回归的表现是
    /// 文件树图标、exe 图标、品牌 logo **静默**变空白（`load` 回落失败只是 `Ok(None)`）。
    /// 这四个路径分别代表四类在用资源，逐一钉住。
    #[test]
    fn wired_assets_are_still_embedded() {
        let paths = embedded_paths();
        for required in [
            // 文件图标主题：`shared/src/icons/file_icon.rs` 的 `include_str!` 读它。
            "icon-themes/idea/extension.json",
            // UI 图标：`shared/src/icons/idea.rs` 里的常量路径之一。
            "ui-icons/idea/expui/general/search.svg",
            // exe / 窗口 / 任务栏图标（PE 资源走 `build.rs`，这一份是 AssetSource 侧）。
            "icons/icon.ico",
            // 品牌 logo。
            "images/logo.png",
        ] {
            assert!(
                paths.iter().any(|path| path == required),
                "`{required}` 必须留在内嵌表里（当前内嵌 {} 个路径）",
                paths.len()
            );
        }
        // 反向保护：`icons/` 下只有位图，没有 `icons/search.svg` 这种 SVG。
        assert!(
            !paths.iter().any(|path| path == "icons/search.svg"),
            "`icons/search.svg` 从不存在，内嵌表里出现它说明路径口径又漂移了"
        );
    }

    /// 实测口径：把内嵌文件数、分组数与总字节数打出来，供人工核对模块文档里那组数字。
    ///
    /// 这里的数字是**刻意的变更检测器**：资源增减本来就要同步更新模块文档的口径，
    /// 于是也让这条测试失败一次，强迫那个改动是有意的（而不是 `#[exclude]` 写错导致的静默漏嵌）。
    /// 文件数、分组数、**总字节数**都逐字钉住（字节数是本轮从"只钉上界"收紧成精确值的：
    /// 精确值同时蕴含原来那条"不得高于 5 000 000 字节"的死载荷上界断言）。
    #[test]
    fn embedded_range_stays_within_the_documented_envelope() {
        let count = LitheAssets::embedded_count();
        println!("LitheAssets::iter().count() = {count}");
        println!(
            "ui-icons/ = {}, icon-themes/idea/ = {}, icons/ = {}, images/ = {}",
            LitheAssets::embedded_count_under("ui-icons/"),
            LitheAssets::embedded_count_under("icon-themes/idea/"),
            LitheAssets::embedded_count_under("icons/"),
            LitheAssets::embedded_count_under("images/"),
        );

        // 内嵌范围只应是这四组：`README.md` 由 `#[exclude]` 挡在表外，所以这里**没有**例外项。
        // 出现别的路径说明 `lithe-gpui/assets/` 下多了新目录，那要么该加 `#[exclude]`、要么该更新文档。
        let known_groups = ["ui-icons/", "icon-themes/", "icons/", "images/"];
        let paths = embedded_paths();
        let ungrouped: Vec<&str> = paths
            .iter()
            .filter(|path| !known_groups.iter().any(|group| path.starts_with(group)))
            .map(String::as_str)
            .collect();
        assert!(
            ungrouped.is_empty(),
            "四组之外不应存在任何路径（`README.md` 已被 `#[exclude]` 排除），实得 {ungrouped:?}"
        );

        assert_eq!(
            LitheAssets::embedded_count_under("icon-themes/"),
            104,
            "只应剩 `icon-themes/idea/**` 这 104 个文件"
        );
        assert_eq!(
            LitheAssets::embedded_count_under("ui-icons/"),
            157,
            "`ui-icons/**` 应当全量内嵌"
        );
        assert_eq!(
            LitheAssets::embedded_count_under("icons/"),
            7,
            "`icons/**` 只有 7 个位图"
        );
        assert_eq!(
            LitheAssets::embedded_count_under("images/"),
            1,
            "`images/**` 只有品牌 logo 一个文件"
        );
        assert_eq!(
            count, 269,
            "排除 `README.md` 后的内嵌总数（7 + 1 + 157 + 104 = 269 个资源文件）"
        );

        // 本 crate 没有打开 rust-embed 的 `compression` 特性，所以内嵌字节数 == 源文件长度之和，
        // 可以直接用 `get()` 实测。`README.md` 已经不在表里，所以这里的和就是"资源总量"本身，
        // 不需要再按名字过滤。打印出来是为了让模块文档里那组数字可核对。
        let resource_bytes: usize = <LitheAssets as rust_embed::RustEmbed>::iter()
            .filter_map(|name| {
                <LitheAssets as rust_embed::RustEmbed>::get(&name).map(|file| file.data.len())
            })
            .sum();
        println!(
            "embedded resource bytes = {resource_bytes} ({:.2} MiB)",
            resource_bytes as f64 / 1_048_576.0
        );
        // 三套图标包 + material 合计 5 864 529 字节，归档目录约 1.79 MiB：
        // 精确钉住 3 200 771 已经蕴含"它们一个都不在里面"。
        assert_eq!(
            resource_bytes, 3_200_771,
            "内嵌资源字节数应精确等于 269 个资源文件之和；若变大，先查未接线的图标包\
             （lithe/pierre/symbols/material 合计 5 864 529 字节）与归档目录是不是又进来了，\
             再查是不是有资源被整体替换成了更大的版本"
        );
    }

    /// 回归：根目录的 `README.md` **不得**再进内嵌表。
    ///
    /// 保护两件不会让编译或别的测试失败的事：
    ///
    /// 1. `#[exclude = "README.md"]` 被误删/写错（例如写成 `readme.md`、或加上 `**/` 前缀后
    ///    反而匹配不到根路径）—— 那样它又会被嵌进二进制，而且每编辑一次文档就改一次
    ///    「内嵌总文件数与总字节数」这个口径；
    /// 2. 上面那条总量断言被"顺手放宽"成只钉近似值 —— 这条单独把"表里没有 README.md"
    ///    变成可执行的判据。
    #[test]
    fn the_readme_is_not_embedded() {
        let paths = embedded_paths();
        assert!(
            !paths.iter().any(|path| path == "README.md"),
            "`README.md` 是文档不是资源，必须由 `#[exclude]` 挡在二进制外（当前内嵌 {} 个路径）",
            paths.len()
        );
    }

    /// 回归：`S1_ASSETS` 探针指的**每一条**路径都必须来自**我们内嵌的那张表**，且能取到字节。
    ///
    /// 保护的是"诊断自己失信"这一类回归（两类触发方式，都不会让编译或别的测试失败）：
    ///
    /// 1. 探针路径写错 —— 历史上就有 `icons/settings.svg`（`lithe-gpui/assets/icons/**` 下只有
    ///    7 个位图，一个 `.svg` 都没有）；
    /// 2. `#[exclude]` / `#[include]` 改动把探针要用的那一批挡在了内嵌表外
    ///    （例如把 `icon-themes/idea/**` 也排除掉）。
    ///
    /// ⚠️ **只断言 `load` 非空是不够的**（本轮反向验证实测踩到）：[`LitheAssets::load`] 会回落
    /// `gpui_kit::assets::AllAssets`，而 Lucide 那 1 830 个字形里**就有** `icons/settings.svg`
    /// —— 于是那条写错的探针**经回落拿到了 586 字节**（真机日志见
    /// `lithe-gpui/research/icon-asset-inventory.md:531`）：既不打 `MISSING`、也不让任何测试失败，
    /// 却完全没证明"我们内嵌的键能用"。所以这里先断言路径在**内嵌表**里（`iter()` 的静态表），
    /// 再走真正的 [`AssetSource::load`]（与 `probe_len` 同一条路）确认拿到非空字节。
    /// 无 sleep、无文件系统访问。
    #[test]
    fn probe_paths_are_loadable_from_the_embedded_table() {
        let paths = embedded_paths();
        for &path in PROBE_PATHS {
            assert!(
                paths.iter().any(|embedded| embedded == path),
                "`{path}` 不在 LitheAssets 的内嵌表里（当前内嵌 {} 个路径）—— 这种探针要么经 \
                 Lucide 回落拿到字节、要么永远打 MISSING，两者都证明不了我们的键能用",
                paths.len()
            );
            let bytes = LitheAssets
                .load(path)
                .unwrap_or_else(|error| panic!("`{path}` load 报错：{error}"))
                .unwrap_or_else(|| panic!("`{path}` 在内嵌表里，但 `load` 取不到字节"));
            assert!(!bytes.is_empty(), "`{path}` 取到了 0 字节，等于取不到");
        }
    }
}
