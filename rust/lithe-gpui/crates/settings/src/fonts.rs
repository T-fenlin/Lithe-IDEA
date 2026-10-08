use std::path::PathBuf;
use std::sync::OnceLock;

use gpui::App;

use crate::extensions;
use crate::extensions::{BundledExtension, BundledKind};

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

pub const BUILTIN_FONT_FAMILY: &str = "JetBrains Mono";

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
        Err(error) => eprintln!("S1_FONTS register_failed error={error}"),
    }
}

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

pub fn family_of(extension: &BundledExtension) -> Option<&'static str> {
    font_family_name(extension_first_ttf(extension)?)
}

fn extension_first_ttf(extension: &BundledExtension) -> Option<&'static [u8]> {
    extension
        .files
        .iter()
        .find(|(relative, _)| relative.ends_with(".ttf"))
        .map(|(_, bytes)| *bytes)
}

pub fn font_family_name(bytes: &[u8]) -> Option<&'static str> {
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

pub fn repository_builtin_font_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts"))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn family_name_is_read_from_the_font_bytes() {
        let (_, content) = BUILTIN_FONT_FILES
            .iter()
            .find(|(name, _)| *name == "JetBrainsMono-Regular.ttf")
            .expect("内置里有 JetBrains Mono Regular");
        assert_eq!(font_family_name(content), Some("JetBrains Mono"));
    }

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

    #[test]
    fn family_name_rejects_non_font_bytes() {
        assert_eq!(font_family_name(b""), None);
        assert_eq!(font_family_name(b"not a font at all, just some text"), None);
        assert_eq!(font_family_name(&[0x00, 0x01, 0x00, 0x00]), None);
    }
}
