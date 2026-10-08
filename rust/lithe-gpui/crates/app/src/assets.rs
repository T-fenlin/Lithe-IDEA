use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

gpui_kit::assets::icon_assets!(
    GpuiKitFallbackIcons,
    [
        AppWindow,
        ArrowDown,
        ArrowDownLeft,
        ArrowDownToLine,
        ArrowUpRight,
        Asterisk,
        Ban,
        BookOpen,
        Braces,
        Calendar,
        CaseSensitive,
        Check,
        ChevronDown,
        ChevronLeft,
        ChevronRight,
        ChevronUp,
        ChevronsUpDown,
        CircleCheck,
        CircleDot,
        CircleSlash,
        CircleX,
        Clipboard,
        Close,
        Code,
        Copy,
        CopyPlus,
        Ellipsis,
        ExternalLink,
        Eye,
        EyeOff,
        File,
        FileBraces,
        FileCode,
        FilePlusCorner,
        FileText,
        FileType,
        FoldVertical,
        Folder,
        FolderClosed,
        FolderCog,
        FolderOpen,
        FolderTree,
        FolderX,
        GitBranch,
        GitGraph,
        Github,
        Image,
        Inbox,
        Info,
        Inspector,
        Leaf,
        List,
        ListOrdered,
        Loader,
        Map,
        Maximize,
        Minimize,
        Minus,
        Network,
        Package,
        PanelBottom,
        PanelBottomOpen,
        PanelLeft,
        PanelLeftClose,
        PanelLeftOpen,
        PanelRight,
        PanelRightClose,
        PanelRightOpen,
        Parentheses,
        Pilcrow,
        Plus,
        Power,
        Puzzle,
        RefreshCw,
        Replace,
        SaveAll,
        SaveCheck,
        SavePen,
        ScrollText,
        Search,
        Settings,
        Signature,
        SortAscending,
        SortDescending,
        Space,
        Sparkles,
        Square,
        SquareActivity,
        SquareDashed,
        SquarePilcrow,
        SquareTerminal,
        SquareX,
        Star,
        StarFill,
        Tag,
        Terminal,
        TextWrap,
        TriangleAlert,
        Undo2,
        UnfoldVertical,
        User,
        WindowClose,
        WindowMaximize,
        WindowMinimize,
        WindowRestore,
        X,
    ]
);

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
    pub fn fallback_count() -> usize {
        GpuiKitFallbackIcons
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

        GpuiKitFallbackIcons.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths: Vec<SharedString> = <Self as rust_embed::RustEmbed>::iter()
            .filter(|name| name.starts_with(path))
            .map(SharedString::from)
            .collect();
        paths.extend(GpuiKitFallbackIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::{GpuiKitFallbackIcons, LitheAssets, PROBE_PATHS, probe_len};
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
            302,
            "图标包（idea）应是 302 个文件 = macOS 图标文件夹的 178 个副本 + 122 个并集补充的旧图标 + \
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
            count, 310,
            "排除 `README.md`/`fonts/` 后的内嵌总数（应用图标 7 + 品牌 1 + 图标包 302）"
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
            resource_bytes, 3_244_470,
            "内嵌资源字节数应精确等于 310 个资源文件之和（文本资源行尾归一后的口径）；\
             若变大，先查是不是有图标主题被塞回来了（lithe/symbols/material/pierre 已于 2026-10-04 删除）；\
             再查是不是有资源被整体替换成了更大的版本；\
             是不是又进来了，再查是不是有资源被整体替换成了更大的版本；\
             若变小，通常是清单 `icons/idea/extension.json` 被改写 —— 重测后再改这个数。\
             另注意：清单重写也会让这个数变化（2026-10-04 起路径改指 macOS 图标副本）"
        );
    }

    #[test]
    fn gpui_kit_fallback_icons_are_the_used_subset_and_loadable() {
        let paths = GpuiKitFallbackIcons
            .list("")
            .expect("列出 gpui-kit 回落图标");
        assert_eq!(
            paths.len(),
            106,
            "回落子集只登记「宿主代码 + gpui-kit 组件内部」实际用到的图标名（2026-10-04 实测 106 个），\
             不再内嵌 gpui-kit 的整份目录（AllAssets，1830 个）；\
             新增 `IconName::X` 之后跑 `node rust/lithe-gpui/tools/sync-gpui-kit-icons.mjs --apply`"
        );
        for path in [
            "icons/x.svg",
            "icons/chevron-down.svg",
            "icons/search.svg",
            "icons/square-terminal.svg",
        ] {
            assert!(
                paths.iter().any(|listed| listed == path),
                "`{path}` 必须在回落子集里，否则界面上会静默画空白"
            );
        }
        for path in &paths {
            let bytes = LitheAssets
                .load(path)
                .unwrap_or_else(|error| panic!("`{path}` load 报错：{error}"))
                .unwrap_or_else(|| panic!("`{path}` 在回落子集里，但取不到字节"));
            assert!(!bytes.is_empty(), "`{path}` 取到了 0 字节，等于取不到");
        }
        assert_eq!(LitheAssets::fallback_count(), 106);
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
