use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

// 组件库组件**自身**的内部装饰图标（关闭 X、勾选 Check、子菜单箭头、严重度图标等）由
// 组件库自带的资源源 `lithe_ui::Assets` 提供 —— 它同样是本仓库自有的 crate（`rust/lithe-ui`）。
//
// 口径（2026-10-08）：**应用**图标一律取自本仓库自己的图标集 `assets/icons/idea/**`
// （`icons/idea/extension.json` 是清单真源），我们没有就留空；不许用组件库图标顶替应用图标。
// 两者命名空间由 `shared::icons::AppIcon` 在类型上隔开：应用图标只装本仓路径。

#[derive(rust_embed::RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../../assets"]
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
    pub fn library_icon_count() -> usize {
        lithe_ui::Assets
            .list("icons/")
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
    "icons/idea/expui/general/search.svg",
    "icons/idea/fileTypes/java.svg",
    "icons/idea/legacy/ui/close.svg",
    "app-icon/icon.ico",
    "icons/idea/extension.json",
];

impl AssetSource for LitheAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(file) = <Self as rust_embed::RustEmbed>::get(path) {
            return Ok(Some(file.data));
        }

        lithe_ui::Assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths: Vec<SharedString> = <Self as rust_embed::RustEmbed>::iter()
            .filter(|name| name.starts_with(path))
            .map(SharedString::from)
            .collect();
        paths.extend(lithe_ui::Assets.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::{LitheAssets, PROBE_PATHS, probe_len};
    use gpui::AssetSource as _;

    fn embedded_paths() -> Vec<String> {
        <LitheAssets as rust_embed::RustEmbed>::iter()
            .map(|name| name.to_string())
            .collect()
    }

    const CR_NORMALIZED_EXTENSIONS: &[&str] = &[
        "json", "svg", "md", "txt", "toml", "yml", "yaml", "xml", "js", "css", "html",
    ];

    fn is_cr_normalized_asset(name: &str) -> bool {
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
            "icons/pierre/",
            "fonts/",
        ] {
            let leaked: Vec<&str> = paths
                .iter()
                .filter(|path| path.starts_with(prefix))
                .map(String::as_str)
                .collect();
            assert!(
                leaked.is_empty(),
                "`{prefix}` 不该进二进制（2026-10-04 起这几个图标主题文件夹已删除），但内嵌表里仍有 {} 个路径，例如 {:?}",
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
            "icons/idea/fileTypes/java.svg",
            "icons/idea/legacy/ui/close.svg",
            "icons/idea/SOURCES.md",
            "app-icon/icon.ico",
            "images/logo.png",
        ] {
            assert!(
                paths.iter().any(|path| path == required),
                "`{required}` 必须留在内嵌表里（当前内嵌 {} 个路径）",
                paths.len()
            );
        }
        for gone in ["icons/idea/icons/", "icons/search.svg", "icons/pierre/"] {
            assert!(
                !paths.iter().any(|path| path.starts_with(gone)),
                "`{gone}` 是搬迁前的旧口径（2026-10-04 起图标真源改为 macOS 图标文件夹的副本），\
                 内嵌表里出现它就说明旧包又被塞回来了"
            );
        }
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
            301,
            "图标包（idea）应是 301 个文件 = macOS 图标文件夹的 178 个副本 + 122 个并集补充的旧图标 + \
             extension.json + SOURCES.md"
        );
        assert_eq!(
            LitheAssets::embedded_count_under("icons/pierre/"),
            0,
            "pierre 图标主题已随 2026-10-04 的图标真源切换删除"
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
            count, 309,
            "排除 `README.md`/`fonts/` 后的内嵌总数（应用图标 7 + 品牌 1 + 图标包 301）"
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
            resource_bytes, 3_246_900,
            "内嵌资源字节数应精确等于 310 个资源文件之和（文本资源行尾归一后的口径）；\
             若变大，先查是不是有图标主题被塞回来了（lithe/symbols/material/pierre 已于 2026-10-04 删除）；\
             再查是不是有资源被整体替换成了更大的版本；\
             是不是又进来了，再查是不是有资源被整体替换成了更大的版本；\
             若变小，通常是清单 `icons/idea/extension.json` 被改写 —— 重测后再改这个数。\
             另注意：清单重写也会让这个数变化（2026-10-04 起路径改指 macOS 图标副本）。\
             2026-10-08 从 3_244_470 涨到 3_246_804：清单补齐了 maven/docker/editorConfig/class/\
             interface/enum/annotation/record 与 python/rust/go/kotlin/ruby/php/scala/binaryData \
             这批我们本来就有图的定义（并删掉两处没有美术的悬空引用），只有一个文件变了、没有增删图标。\
             2026-10-11 涨到 3_246_900：登记了 `unknown`（`fileTypes/unknown.svg` 此前从没进过\
             定义表，是孤儿素材）。同一批里 `fileTypes/powershell_dark.svg` 经核对与 \
             `powershell.svg` 逐字节相同，不是深色变体，不进清单、留作待删副本。"
        );
    }

    /// 组件库自己的图标（宿主会渲染的那些内部装饰图标）必须**全部**能从宿主资源源取到。
    ///
    /// 口径（2026-10-08）：**应用**图标只取本仓 `assets/icons/idea/**`，缺则留空，不用组件库
    /// 图标顶替；而**组件库组件自身**的装饰图标由 `lithe_ui::Assets` 提供——它同样是本仓自有的
    /// crate。
    #[test]
    fn every_library_icon_the_host_draws_is_reachable() {
        use gpui::AssetSource as _;
        use lithe_ui::primitives::IconName;

        for icon in IconName::ALL {
            let path = icon.path();
            let bytes = LitheAssets
                .load(path)
                .unwrap_or_else(|error| panic!("`{path}` load 报错：{error}"))
                .unwrap_or_else(|| panic!("`{path}` 取不到字节：组件库图标没接进宿主资源源"));
            assert!(!bytes.is_empty(), "`{path}` 取到 0 字节，等于取不到");
        }
        assert!(
            LitheAssets::library_icon_count() > 0,
            "组件库图标数为 0，说明 `lithe_ui::Assets` 没接上"
        );
    }

    /// 清单里写着的每一条相对路径（`"./fileTypes/java.svg"` 那种）。
    ///
    /// 只抽 `./` 开头的字符串，不引 `serde_json`：本 crate 的活是"内嵌了什么"，
    /// 把整个清单解析成对象树是 `lithe-gpui-shared` 那边 `file_icon.rs` 的事
    /// （它有 `serde_json`，并且已经按 id 验过一遍引用完整性）。
    /// 而 `iconDefinitions` / `lightIconDefinitions` 里的值**一律**以 `./` 开头，
    /// 所以这一扫正好等于两张定义表引用到的全部路径。
    fn manifest_referenced_paths(manifest: &str) -> Vec<String> {
        let mut paths = Vec::new();
        let mut cursor = 0;
        while let Some(offset) = manifest[cursor..].find("\"./") {
            let start = cursor + offset + 3;
            let Some(end) = manifest[start..].find('"').map(|end| start + end) else {
                break;
            };
            paths.push(manifest[start..end].to_string());
            cursor = end + 1;
        }
        paths
    }

    /// 图标包里**按文件类型取用**的那批图（不是 UI 图）必须全部被清单够得着。
    ///
    /// 口径：`icons/idea/fileTypes/**` 与 `icons/idea/legacy/files/**` 是"后缀 → 图标"的
    /// 素材池；`debugger/stepOver.svg`、`legacy/ui/close.svg` 这类是 UI 图标，永远不会被任何
    /// 后缀引用，不该进 `iconDefinitions`，所以这里只管前两个目录。
    ///
    /// 用测试而不是编译期检查，是因为 `rust_embed` 的文件清单要到运行时才有：
    /// 往 `assets/icons/idea/fileTypes/` 里丢一张新 SVG 却忘了在 `extension.json` 登记，
    /// 这条会直接失败并把漏掉的路径列出来。
    ///
    /// **唯一一类允许的例外**：与被够得着的某张图**逐字节相同**的副本。它不算"漏登记的美术"
    /// —— 那张图已经通过它的孪生文件在服务了，副本只是冗余。这类副本的正解是**删掉**
    /// （连同 `icons::idea` 里对应的常量一起），不是给它补一条定义。所以下面把它钉成一个
    /// 显式清单：多出一张新的副本就会失败，逼着人去清理，而不是悄悄放过。
    #[test]
    fn every_file_type_icon_is_reachable_from_the_manifest() {
        let manifest = LitheAssets
            .load("icons/idea/extension.json")
            .expect("清单必须能 load")
            .expect("清单必须在内嵌表里");
        let manifest = String::from_utf8(manifest.to_vec()).expect("清单必须是 UTF-8");
        let referenced: Vec<String> = manifest_referenced_paths(&manifest)
            .into_iter()
            .map(|relative| format!("icons/idea/{}", relative.trim_start_matches("./")))
            .collect();
        let reachable: Vec<Vec<u8>> = referenced
            .iter()
            .filter_map(|path| {
                LitheAssets
                    .load(path)
                    .ok()
                    .flatten()
                    .map(|bytes| bytes.to_vec())
            })
            .collect();

        let mut missing = Vec::new();
        let mut duplicates = Vec::new();
        let mut checked = 0;
        for path in embedded_paths() {
            // 只看文件类型那两个目录；`nodes/` 是文件夹与 Java 节点图，`legacy/ui/` 等是 UI 图。
            let is_file_type = path.starts_with("icons/idea/fileTypes/")
                || path.starts_with("icons/idea/legacy/files/");
            if !is_file_type || !path.ends_with(".svg") {
                continue;
            }
            checked += 1;
            if referenced.iter().any(|known| *known == path) {
                continue;
            }
            let bytes = LitheAssets
                .load(&path)
                .unwrap_or_else(|error| panic!("`{path}` load 报错：{error}"))
                .unwrap_or_else(|| panic!("`{path}` 在内嵌表里，但 `load` 取不到字节"))
                .to_vec();
            if reachable.iter().any(|served| *served == bytes) {
                duplicates.push(path);
            } else {
                missing.push(path);
            }
        }

        assert!(
            checked >= 60,
            "只查到 {checked} 张文件类型图，这个检查没在干活（目录被搬走了？）"
        );
        assert!(
            missing.is_empty(),
            "这 {} 张文件类型的图没有被 `icons/idea/extension.json` 够得着，而且和已登记的图\
             都不一样 —— 要么在 `iconDefinitions`/`lightIconDefinitions` 里补上定义，要么在 \
             `fileExtensions`/`filenames` 里补上映射：{missing:?}",
            missing.len()
        );
        // 判定必须是**空**：重复的图已经删干净了（曾有一张 `fileTypes/powershell_dark.svg`
        // 与 `powershell.svg` 逐字节相同、`_dark` 后缀是误加的，已删；这种名不副实的副本
        // 会让两个人先后照名字下错结论）。断言空集比钉住具体路径更强：以后再进来任何一张
        // 重复副本都会挂，逼着清理，而不是悄悄放过。
        assert_eq!(
            duplicates,
            Vec::<String>::new(),
            "这些文件类型图与被够得着的某张图逐字节相同，是冗余副本，删掉它们：{duplicates:?}"
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
