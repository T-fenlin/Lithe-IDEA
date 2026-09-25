//! Lithe 的资源源：内嵌 `gpui/assets/**`，未命中回落 gpui-kit 的 `AllAssets`。
//!
//! ## 为什么需要这一层
//!
//! `Application::with_assets`（`gpui-pre-0.3.6/src/app.rs:198-206`）**只收一个** `AssetSource`：
//! 它把值 `Arc::new` 成 `Arc<dyn AssetSource>` 存进 `App`，重复调用只是覆盖。而
//! `gpui_kit::assets::AllAssets` 只嵌了 `icons/**/*.svg`（1830 个 Lucide 字形，
//! `gpui-kit-assets-0.6.6/src/native_assets.rs:8-11`），**看不到**我们从 Windows 前端搬进来的
//! `gpui/assets/ui-icons/**` 与 `gpui/assets/icon-themes/**`。所以必须自己实现一个包装层，
//! 在里面做"先查自己的、再回落"的组合。官方 0.6.6 文档就是这么建议的
//! （`gpui/docs/gpui-kit/0.6.6/zh-CN/docs/assets.md` 的「自定义资源」一节），
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
//!   这里的做法是把 `Err` 折叠成 `Ok(None)`。这也是 `gpui/research/app-icon-and-assets.md`
//!   §4.4 记下的坑。
//! - [`AssetSource::list`]：`list` 未命中是 `Ok(vec![])`，可以放心透传；但两边都可能命中同一
//!   路径，所以必须**合并 + 排序 + 去重**（官方文档 `assets.md:38-42` 的同口径）。
//!   未排序的列表会让依赖 `list` 顺序的诊断输出不稳定。
//!
//! ## 内嵌范围与代价
//!
//! 嵌的是**整个 `gpui/assets/**`**（1 204 个文件、约 8.2 MiB）：`icons/**` 8 个位图 + `images/logo.png`、
//! `ui-icons/**` 158 个文件 + `icon-themes/**` 1 037 个文件。这样"资源在 `gpui/assets` 下"
//! 就是一条统一规则，不需要每加一批资源就改一次 `#[include]`。
//! 代价是二进制体积（SVG 压缩率高，实测影响见 `gpui/research/icon-asset-inventory.md` 的接线一节）。
//! 如果将来只想带一部分，`#[include]` 是唯一的开关位置。
//!
//! ## 与窗口 / 任务栏图标无关
//!
//! `AssetSource` 只服务 `img()`、`svg().path(..)`、以及 gpui-component 的原生菜单图标
//! （`gpui-component-0.6.6/src/native_menu/mod.rs:32,229`）。exe / 任务栏图标走 PE 资源
//! （`crates/app/lithe.rc` + `build.rs`），两者互不影响。

use std::borrow::Cow;

use gpui_kit::assets::AllAssets;
use gpui_kit::{AssetSource, Result, SharedString};

/// 内嵌 `gpui/assets/` 的 rust-embed 资产表。
///
/// 路径键**相对 `gpui/assets/`**（例如 `ui-icons/idea/expui/general/search.svg`、
/// `icons/search.svg`、`images/logo.png`），可以直接传给 `svg().path(..)` 或 `img(..)`。
///
/// `#[folder]` 必须是绝对路径字符串：crate 在 `gpui/crates/app`、资源在 `gpui/assets`，
/// 相对写法 `../../assets` 在 rust-embed 里**不可靠**（它的 `get()` 直接把 `folder` 与
/// `path` 拼接后做字符串 `starts_with` 校验，路径里带 `..` 时会比出错误的边界；
/// 见 `rust-embed-impl-8.12.0/src/lib.rs:198-223`）。`$CARGO_MANIFEST_DIR` 插值由
/// `interpolate-folder-path` 特性提供 —— 特性和 `gpui-kit-assets` 打开的是同一份
/// （见 `Cargo.toml` 的注释）。
#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../assets"]
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

impl AssetSource for LitheAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // ① 我们自己的资源：编译期内嵌，路径名与 `gpui/assets/` 下的相对路径一一对应。
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
