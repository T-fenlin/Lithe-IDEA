//! 字体：内置 IDEA 字体（**默认**）、捆绑字体扩展的注册，以及"哪些字体现在可用"。
//!
//! ## 内置 vs 捆绑（与主题同一套口径）
//!
//! | 类别 | 落点 | 可卸载 | 目录契约 |
//! | --- | --- | --- | --- |
//! | 内置字体（JetBrains Mono，IDEA 默认） | `lithe-gpui/assets/fonts/`，`include_bytes!` 进二进制 | **否** | `builtIn` |
//! | 捆绑字体扩展（Source Code Pro / IBM Plex Mono） | `<配置目录>/extensions/fonts/<包>/`，内容来自 [`crate::extensions`] | **是** | `bundled` |
//!
//! ## 为什么字体不需要装载面镜像（主题却需要）
//!
//! 主题走的是 gpui-kit 的 `ThemeRegistry::watch_dir`，它**只能监视一个目录**，
//! 所以主题必须把内容镜像到那唯一一个目录里（见 [`crate::extensions`] 的模块文档）。
//! 字体不一样：文本系统的入口是 `TextSystem::add_fonts(Vec<Cow<[u8]>>)`
//! （`gpui-pre-0.3.6/src/text_system.rs:295`），由**我们自己**在启动时喂字节 ——
//! 于是"文件在哪"只由本模块决定：内置那份直接喂内嵌字节，捆绑那份读管理面目录。
//!
//! ## 卸载为什么要等重启（如实登记）
//!
//! `add_fonts` **只增不删**（同一处源码里没有任何 unregister 入口），运行期摘不掉
//! 已注册的字体族。所以卸载/重装的文件语义立刻生效（文件真的删/写了），
//! 但**字体本身**要等下次启动才不再/重新注册 —— 扩展页在字体卡片的详情里写明了这一点
//! （`lithe.extensions.fontRestartHint`）。
//!
//! ## 默认字体
//!
//! [`crate::schema::DEFAULT_MONO_FONT_FAMILY`] = `JetBrains Mono`：内置那份从
//! 本模块注册，所以设置里的默认值一定有个真实可用的字族兜底（不会落到"字族不存在"的
//! panic 路径，那条约束见 [`crate::theme::apply_font_families`]）。

use std::path::PathBuf;
use std::sync::OnceLock;

use gpui_kit::App;

use crate::extensions;
use crate::extensions::{BundledExtension, BundledKind};

/// **内置**字体文件：`(文件名, 内容)`。内置 = 宿主核心能力、不可卸载。
///
/// 当前只有 IDEA 的默认代码字体 JetBrains Mono（四个字重）。内容 `include_bytes!`
/// 进二进制，所以打包后照样能注册 —— 与内置主题同一条理由。
///
/// ⚠️ 加一份内置字体时**必须在这里登记**（并同步目录 fixture 的 `fonts` 贡献项）；
/// `every_bundled_font_file_is_embedded` 会拿 [`repository_builtin_font_dir`]
/// 的目录内容与两张表对比，漏登记就是测试失败。
pub const BUILTIN_FONT_FILES: &[(&str, &[u8])] = &[
    (
        "JetBrainsMono-Bold.ttf",
        include_bytes!("../../../assets/fonts/JetBrainsMono-Bold.ttf"),
    ),
    (
        "JetBrainsMono-BoldItalic.ttf",
        include_bytes!("../../../assets/fonts/JetBrainsMono-BoldItalic.ttf"),
    ),
    (
        "JetBrainsMono-Italic.ttf",
        include_bytes!("../../../assets/fonts/JetBrainsMono-Italic.ttf"),
    ),
    (
        "JetBrainsMono-Regular.ttf",
        include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf"),
    ),
];

/// 内置字体的**字族名**（与 TTF 内部的 family name 一致）。
pub const BUILTIN_FONT_FAMILY: &str = "JetBrains Mono";

/// 注册内置字体与**已安装**的捆绑字体扩展。
///
/// 顺序与归属：
/// 1. 内置那份永远注册（它是默认字体，不能被卸载拿走）；
/// 2. 捆绑那份从管理面目录读文件（[`extensions::store_dir`]）——**没装就不读**，
///    所以卸载后（文件已删）下次启动不会注册它。读不到的文件跳过：用户可能自己
///    删了某个字重，其余字重照常可用。
///
/// 调用时机：**启动早期、第一次布局之前**（`app/src/main.rs`）。另外
/// [`crate::theme::installed_font_names`] 的已装字族表是进程内缓存一次的，
/// 所以本函数必须在它被首次调用（主题应用路径）之前跑完，否则校验会拒掉
/// 内置字族名（`S1_THEME font_family_rejected`）。
pub fn register_fonts(cx: &mut App) {
    let mut fonts: Vec<std::borrow::Cow<'static, [u8]>> = Vec::new();
    for (_, content) in BUILTIN_FONT_FILES {
        fonts.push(std::borrow::Cow::Borrowed(*content));
    }
    let (bundled_paths, bundled_bytes) = bundled_font_bytes();
    fonts.extend(
        bundled_bytes
            .iter()
            .map(|bytes| std::borrow::Cow::Owned(bytes.clone())),
    );

    if fonts.is_empty() {
        eprintln!("S1_FONTS registered=0 reason=no_files");
        return;
    }
    match cx.text_system().add_fonts(fonts) {
        Ok(()) => println!(
            "S1_FONTS registered builtin={} bundled={} paths={:?}",
            BUILTIN_FONT_FILES.len(),
            bundled_paths.len(),
            bundled_paths
        ),
        // 注册失败不是致命错误：拿不到字族时 `apply_font_families` 会留
        // `S1_THEME font_family_rejected` 诊断并按"不覆盖"处理，界面仍可用。
        Err(error) => eprintln!("S1_FONTS register_failed error={error}"),
    }
}

/// 从管理面目录读**已安装**的捆绑字体文件字节。
///
/// 返回 `(读到的相对路径, 字节)`：路径只用于诊断（哪几个文件真的被注册了）。
fn bundled_font_bytes() -> (Vec<String>, Vec<Vec<u8>>) {
    let mut paths_read = Vec::new();
    let mut bytes_read = Vec::new();
    let Some(dir) = extensions::store_dir(BundledKind::Font) else {
        return (paths_read, bytes_read);
    };
    for extension in extensions::BUNDLED_EXTENSIONS
        .iter()
        .filter(|entry| entry.kind == BundledKind::Font)
    {
        for (relative, _) in extension.files {
            // 许可文本不是字体：只读 `.ttf`。
            if !relative.ends_with(".ttf") {
                continue;
            }
            let path = dir.join(relative);
            match std::fs::read(&path) {
                Ok(bytes) => {
                    paths_read.push(relative.to_string());
                    bytes_read.push(bytes);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    eprintln!("S1_FONTS read_failed path={} error={error}", path.display())
                }
            }
        }
    }
    (paths_read, bytes_read)
}

/// 一个捆绑扩展包声明的**字族名**。
///
/// 字族名不写死在代码里，而是从包内第一个 `.ttf` 的 name 表里读出来 ——
/// "清单里写 Fira Code、文件里其实是别的字族"这种漂移不该靠人去核对。
/// 读不出来（文件缺失或不是字体）返回 `None`。
pub fn family_of(extension: &BundledExtension) -> Option<&'static str> {
    font_family_name(extension_first_ttf(extension)?)
}

/// 包内第一个字体文件的内容。
fn extension_first_ttf(extension: &BundledExtension) -> Option<&'static [u8]> {
    extension
        .files
        .iter()
        .find(|(relative, _)| relative.ends_with(".ttf"))
        .map(|(_, bytes)| *bytes)
}

/// 读 TrueType `name` 表里的 family name（name ID 1，平台 3 编码 1 = Windows BMP Unicode）。
///
/// 只实现这一条最小解析路径：本模块的用途是"把包和它声明的字族对上号"，
/// 不需要完整的字体解析器（也因此没有引入字体解析依赖）。
pub fn font_family_name(bytes: &[u8]) -> Option<&'static str> {
    // 头部：sfntVersion(4) + numTables(2) + …；表目录每项 16 字节。
    let num_tables = u16::from_be_bytes([*bytes.get(4)?, *bytes.get(5)?]) as usize;
    let mut name_offset = None;
    for index in 0..num_tables {
        let record = 12 + index * 16;
        let tag = bytes.get(record..record + 4)?;
        if tag == b"name" {
            let offset = u32::from_be_bytes([
                *bytes.get(record + 8)?,
                *bytes.get(record + 9)?,
                *bytes.get(record + 10)?,
                *bytes.get(record + 11)?,
            ]) as usize;
            name_offset = Some(offset);
            break;
        }
    }
    let name = bytes.get(name_offset?..)?;
    let count = u16::from_be_bytes([*name.get(2)?, *name.get(3)?]) as usize;
    let storage = u16::from_be_bytes([*name.get(4)?, *name.get(5)?]) as usize;
    for index in 0..count {
        let record = 6 + index * 12;
        let platform = u16::from_be_bytes([*name.get(record)?, *name.get(record + 1)?]);
        let name_id = u16::from_be_bytes([*name.get(record + 6)?, *name.get(record + 7)?]);
        let length = u16::from_be_bytes([*name.get(record + 8)?, *name.get(record + 9)?]) as usize;
        let offset =
            u16::from_be_bytes([*name.get(record + 10)?, *name.get(record + 11)?]) as usize;
        if platform != 3 || name_id != 1 {
            continue;
        }
        let start = storage.checked_add(offset)?;
        let raw = name.get(start..start + length)?;
        let text: String = raw
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .map(|unit| char::from_u32(u32::from(unit)).unwrap_or('\u{fffd}'))
            .collect();
        return leaked_family(text);
    }
    None
}

/// 把读出来的字族名转成 `&'static str`。
///
/// 进程内只会有寥寥几个字族，且注册表是 `'static` 结构（[`crate::extensions`]），
/// 所以这里做一次有界的 `Box::leak`：每个包第一个字重只泄漏一次（`OnceLock` 之外
/// 再按内容去重）。代价是几十字节/包，换来"注册表不需要生命周期参数"。
fn leaked_family(text: String) -> Option<&'static str> {
    static CACHE: OnceLock<std::sync::Mutex<Vec<(String, &'static str)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(Vec::new()));
    let mut guard = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((_, leaked)) = guard.iter().find(|(known, _)| *known == text) {
        return Some(*leaked);
    }
    let leaked: &'static str = Box::leak(text.clone().into_boxed_str());
    guard.push((text, leaked));
    Some(leaked)
}

/// 仓库里的**内置字体源目录**（`lithe-gpui/assets/fonts`，构建树路径，只服务测试）。
pub fn repository_builtin_font_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 内置字体目录里的**每一份** `.ttf` 都必须在 [`BUILTIN_FONT_FILES`] 里登记，
    /// 且登记内容与磁盘逐字一致；每份都是可识别的 TrueType（魔数 + 有 `name` 表）。
    #[test]
    fn every_builtin_font_file_is_embedded() {
        let dir = repository_builtin_font_dir();
        let mut files: Vec<String> = std::fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("read_dir failed dir={} error={error}", dir.display()))
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.ends_with(".ttf"))
            .collect();
        files.sort();
        assert!(!files.is_empty(), "no ttf in {}", dir.display());

        let embedded: std::collections::BTreeMap<&str, &[u8]> =
            BUILTIN_FONT_FILES.iter().copied().collect();
        assert_eq!(
            embedded.len(),
            BUILTIN_FONT_FILES.len(),
            "BUILTIN_FONT_FILES 里有重复的文件名"
        );
        assert_eq!(
            files.len(),
            BUILTIN_FONT_FILES.len(),
            "登记的文件数与 {} 里的不一致：{files:?}",
            dir.display()
        );

        for file in &files {
            let content = embedded.get(file.as_str()).unwrap_or_else(|| {
                panic!("{file} 没有登记进 BUILTIN_FONT_FILES（打包后读不到它）")
            });
            let on_disk = std::fs::read(dir.join(file)).expect("读字体失败");
            assert_eq!(*content, on_disk, "{file} 的内嵌内容与磁盘不一致");
            assert!(
                content.starts_with(&[0x00, 0x01, 0x00, 0x00]) || content.starts_with(b"true"),
                "{file} 不是 TrueType 字体"
            );
        }
        assert_eq!(
            BUILTIN_FONT_FAMILY, "JetBrains Mono",
            "内置字族名是产品默认值的一部分（schema::DEFAULT_MONO_FONT_FAMILY）"
        );
    }

    /// 字族名解析真的能从 TTF 里读出 family（JetBrains Mono 那份已知结果）。
    #[test]
    fn family_name_is_read_from_the_font_bytes() {
        let (_, content) = BUILTIN_FONT_FILES
            .iter()
            .find(|(name, _)| *name == "JetBrainsMono-Regular.ttf")
            .expect("内置里有 JetBrains Mono Regular");
        assert_eq!(font_family_name(content), Some("JetBrains Mono"));
    }

    /// 每个字体扩展包的字族名都能读出来（注册表与目录契约要对得上号）。
    #[test]
    fn every_font_extension_declares_a_readable_family() {
        for entry in extensions::BUNDLED_EXTENSIONS
            .iter()
            .filter(|entry| entry.kind == BundledKind::Font)
        {
            let family = family_of(entry).unwrap_or_else(|| panic!("{} 读不出字族名", entry.id));
            assert!(!family.trim().is_empty(), "{} 的字族名是空的", entry.id);
            assert_eq!(
                extensions::font_extension_for_family(family).map(|found| found.id),
                Some(entry.id),
                "按字族名反查必须回到同一个包"
            );
        }
    }

    /// 不是字体的字节返回 `None` 而不是 panic（许可文件被误喂进解析器时）。
    #[test]
    fn family_name_rejects_non_font_bytes() {
        assert_eq!(font_family_name(b""), None);
        assert_eq!(font_family_name(b"not a font at all, just some text"), None);
        assert_eq!(font_family_name(&[0x00, 0x01, 0x00, 0x00]), None);
    }
}
