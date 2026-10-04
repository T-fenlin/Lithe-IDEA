use std::borrow::Cow;

use gpui_kit::assets::AllAssets;
use gpui_kit::{AssetSource, Result, SharedString};

#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../assets"]
#[exclude = "icons/lithe/**"]
#[exclude = "icons/symbols/**"]
#[exclude = "icons/material/**"]
#[exclude = "fonts/**"]
#[exclude = "README.md"]
pub struct LitheAssets;

impl LitheAssets {
    #[must_use]
    pub fn embedded_count() -> usize {
        <Self as rust_embed::RustEmbed>::iter().count()
    }

    #[must_use]
    pub fn embedded_count_under(prefix: &str) -> usize {
        <Self as rust_embed::RustEmbed>::iter()
            .filter(|name| name.starts_with(prefix))
            .count()
    }

    #[must_use]
    pub fn fallback_count() -> usize {
        AllAssets
            .list("")
            .map(|paths| paths.len())
            .unwrap_or_default()
    }
}

#[must_use]
pub fn probe_len(path: &str) -> Option<usize> {
    match LitheAssets.load(path) {
        Ok(Some(bytes)) => Some(bytes.len()),
        _ => None,
    }
}

pub const PROBE_PATHS: &[&str] = &[
    "icons/idea/icons/ui/settings.svg",
    "icons/idea/icons/ui/settings_dark.svg",
    "app-icon/icon.ico",
    "icons/idea/extension.json",
    "icons/idea/icons/files/gitignore.svg",
];

impl AssetSource for LitheAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(file) = <Self as rust_embed::RustEmbed>::get(path) {
            return Ok(Some(file.data));
        }

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
        paths.extend(AllAssets.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::{LitheAssets, PROBE_PATHS, probe_len};
    use gpui_kit::AssetSource as _;

    fn embedded_paths() -> Vec<String> {
        <LitheAssets as rust_embed::RustEmbed>::iter()
            .map(|name| name.to_string())
            .collect()
    }

    const CR_NORMALIZED_EXTENSIONS: &[&str] = &[
        "json", "svg", "md", "txt", "toml", "yml", "yaml", "xml", "js", "css", "html",
    ];

    const CR_NORMALIZED_PATHS: &[&str] = &["icons/pierre/LICENSE"];

    fn is_cr_normalized_asset(name: &str) -> bool {
        if CR_NORMALIZED_PATHS.contains(&name) {
            return true;
        }
        match name.rsplit_once('.') {
            Some((_, extension)) => CR_NORMALIZED_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known)),
            None => false,
        }
    }

    fn crlf_normalized_len(data: &[u8]) -> usize {
        data.len()
            - data
                .windows(2)
                .filter(|pair| pair[0] == b'\r' && pair[1] == b'\n')
                .count()
    }

    #[test]
    fn unwired_icons_are_not_embedded() {
        let paths = embedded_paths();
        for prefix in [
            "icons/lithe/",
            "icons/symbols/",
            "icons/material/",
            "fonts/",
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

    #[test]
    fn wired_assets_are_still_embedded() {
        let paths = embedded_paths();
        for required in [
            "icons/idea/extension.json",
            "icons/idea/icons/ui/search.svg",
            "app-icon/icon.ico",
            "images/logo.png",
        ] {
            assert!(
                paths.iter().any(|path| path == required),
                "`{required}` 必须留在内嵌表里（当前内嵌 {} 个路径）",
                paths.len()
            );
        }
        assert!(
            !paths.iter().any(|path| path == "icons/search.svg"),
            "`icons/search.svg` 从不存在，内嵌表里出现它说明路径口径又漂移了"
        );
    }

    #[test]
    fn embedded_range_stays_within_the_documented_envelope() {
        let count = LitheAssets::embedded_count();
        println!("LitheAssets::iter().count() = {count}");
        println!(
            "icons/idea/ = {}, icons/pierre/ = {}, icons/ = {}, images/ = {}",
            LitheAssets::embedded_count_under("icons/idea/"),
            LitheAssets::embedded_count_under("icons/pierre/"),
            LitheAssets::embedded_count_under("icons/"),
            LitheAssets::embedded_count_under("images/"),
        );

        let known_groups = ["app-icon/", "icons/", "images/"];
        let paths = embedded_paths();
        let ungrouped: Vec<&str> = paths
            .iter()
            .filter(|path| !known_groups.iter().any(|group| path.starts_with(group)))
            .map(String::as_str)
            .collect();
        assert!(
            ungrouped.is_empty(),
            "三组之外不应存在任何路径（`README.md` 与 `fonts/` 已被 `#[exclude]` 排除），实得 {ungrouped:?}"
        );

        assert_eq!(
            LitheAssets::embedded_count_under("icons/idea/"),
            254,
            "内置图标集（idea）应是 254 个文件 —— 2026-09-30 起 expui 界面图标并入本包"
        );
        assert_eq!(
            LitheAssets::embedded_count_under("icons/pierre/"),
            149,
            "捆绑图标集（pierre）应是 149 个文件 —— 它随宿主编译、可卸载可切换"
        );
        assert_eq!(
            LitheAssets::embedded_count_under("app-icon/"),
            7,
            "`app-icon/**` 是应用图标的 7 个位图"
        );
        assert_eq!(
            LitheAssets::embedded_count_under("images/"),
            1,
            "`images/**` 只有品牌 logo 一个文件"
        );
        assert_eq!(
            count, 411,
            "排除 `README.md`/`fonts/` 后的内嵌总数（应用图标 7 + 品牌 1 + idea 254 + pierre 149）"
        );

        let resource_bytes: usize = <LitheAssets as rust_embed::RustEmbed>::iter()
            .filter_map(|name| {
                <LitheAssets as rust_embed::RustEmbed>::get(&name).map(|file| {
                    if is_cr_normalized_asset(&name) {
                        crlf_normalized_len(&file.data)
                    } else {
                        file.data.len()
                    }
                })
            })
            .sum();
        println!(
            "embedded resource bytes = {resource_bytes} ({:.2} MiB)",
            resource_bytes as f64 / 1_048_576.0
        );
        assert_eq!(
            resource_bytes, 3_325_504,
            "内嵌资源字节数应精确等于 411 个资源文件之和（文本资源行尾归一后的口径）；\
             若变大，先查未接线的图标包\
             （lithe/symbols/material 合计约 4.97 MiB，注意 pierre 已接线、**在**二进制里）\
             是不是又进来了，再查是不是有资源被整体替换成了更大的版本；\
             若变小，通常是清单 `icons/idea/extension.json` 被改写 —— 重测后再改这个数。\
             另注意：清单改写也可能让这个数**变大**（2026-10-02 b66dbd12 的三分组改写 +8 015）"
        );
    }

    #[test]
    fn the_readme_is_not_embedded() {
        let paths = embedded_paths();
        assert!(
            !paths.iter().any(|path| path == "README.md"),
            "`README.md` 是文档不是资源，必须由 `#[exclude]` 挡在二进制外（当前内嵌 {} 个路径）",
            paths.len()
        );
    }

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

    #[test]
    fn every_idea_constant_path_is_embedded() {
        let mut missing = Vec::new();
        for icon in lithe_gpui_shared::icons::idea::ALL {
            for path in [icon.light, icon.dark] {
                if probe_len(path).is_none() {
                    missing.push(path);
                }
            }
        }
        assert!(
            missing.is_empty(),
            "这些 `idea.rs` 常量路径取不到内嵌字节（界面上会画空白）：{missing:?}"
        );
    }
}
