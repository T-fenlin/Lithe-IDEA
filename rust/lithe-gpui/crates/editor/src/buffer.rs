use std::path::{Path, PathBuf};

use gpui::App;
use gpui::{Entity, SharedString, Subscription, Task};
use gpui_kit::component::input::EditorState;
use lithe_gpui_shared::icons::{FileIcon, file_icon};
use lithe_gpui_shared::tr_args;

const MAX_EDITOR_BYTES: u64 = 2 * 1024 * 1024;

pub(crate) fn icon_for_file(name: &str, cx: &App) -> FileIcon {
    FileIcon::own(file_icon::fallback_icon(name)).with_theme_icon(name, cx)
}

pub(crate) fn language_for_file(name: &str) -> Option<&'static str> {
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);

    let resolve = |language: Option<&'static str>| -> Option<&'static str> {
        match language {
            Some(name) if lithe_gpui_shared::languages::is_language_disabled(name) => None,
            other => other,
        }
    };

    let language = match extension.as_deref() {
        Some("java") => Some("java"),
        Some("json" | "jsonc") => Some("json"),
        Some("yml" | "yaml") => Some("yaml"),
        Some("toml") => Some("toml"),
        Some("sql") => Some("sql"),
        Some("md" | "markdown") => Some("markdown"),
        Some("sh" | "bash") => Some("bash"),
        Some("kt" | "kts" | "ktm") => Some("kotlin"),
        Some("xml" | "xsd" | "xsl" | "xslt" | "wsdl") => Some("xml"),
        _ => match name.to_ascii_lowercase().as_str() {
            "mvnw" | "gradlew" | "mvnw.cmd" | "gradlew.bat" => Some("bash"),
            _ => None,
        },
    };
    resolve(language)
}

pub(crate) struct Body {
    pub(crate) text: String,
    pub(crate) writable: bool,
}

pub(crate) fn read_body(path: &Path, name: &str) -> Body {
    fn notice_body(lines: &[String]) -> Body {
        Body {
            text: notice(lines),
            writable: false,
        }
    }

    match std::fs::metadata(path) {
        Err(error) => notice_body(&[
            tr_args("lithe.files.openFailed", &[("name", name)]).to_string(),
            error.to_string(),
        ]),
        Ok(metadata) if metadata.len() > MAX_EDITOR_BYTES => notice_body(&[format!(
            "文件超过 {} MiB，暂不在编辑器中打开。",
            MAX_EDITOR_BYTES / (1024 * 1024)
        )]),
        Ok(_) => match std::fs::read(path) {
            Err(error) => notice_body(&[
                tr_args("lithe.files.openFailed", &[("name", name)]).to_string(),
                error.to_string(),
            ]),
            Ok(bytes) if bytes.contains(&0) => {
                notice_body(&["二进制文件不在编辑器中打开。".to_string()])
            }
            Ok(bytes) => match std::str::from_utf8(&bytes) {
                Ok(text) => Body {
                    text: text.to_string(),
                    writable: true,
                },
                Err(_) => Body {
                    text: String::from_utf8_lossy(&bytes).to_string(),
                    writable: false,
                },
            },
        },
    }
}

fn notice(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| format!("// {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn path_segments(path: &Path) -> Vec<String> {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let mut parts: Vec<String> = normalized.split('/').map(str::to_string).collect();
    parts.pop();
    parts
}

fn segment_from_end(segments: &[String], depth: usize) -> &str {
    segments
        .len()
        .checked_sub(depth)
        .and_then(|index| segments.get(index))
        .map(String::as_str)
        .unwrap_or("")
}

pub(crate) fn display_names(buffers: &[Buffer]) -> Vec<SharedString> {
    let segments: Vec<Vec<String>> = buffers
        .iter()
        .map(|buffer| path_segments(&buffer.path))
        .collect();
    let mut groups: Vec<(SharedString, Vec<usize>)> = Vec::new();
    let mut names: Vec<Option<SharedString>> = vec![None; buffers.len()];

    for (index, buffer) in buffers.iter().enumerate() {
        let existing = groups.iter().position(|(name, _)| *name == buffer.name);
        match existing {
            Some(group) => groups[group].1.push(index),
            None => groups.push((buffer.name.clone(), vec![index])),
        }
    }

    for (name, items) in &groups {
        if items.len() == 1 {
            names[items[0]] = Some(name.clone());
            continue;
        }

        let max_segments = items
            .iter()
            .map(|index| segments[*index].len())
            .max()
            .unwrap_or(0);
        let mut resolved = false;

        for depth in 1..=max_segments {
            let mut seen: Vec<&str> = Vec::new();
            let mut all_distinct = true;
            for index in items {
                let segment = segment_from_end(&segments[*index], depth);
                if seen.contains(&segment) {
                    all_distinct = false;
                    break;
                }
                seen.push(segment);
            }

            if all_distinct {
                for index in items {
                    let segment = segment_from_end(&segments[*index], depth);
                    names[*index] = Some(labelled(name, segment));
                }
                resolved = true;
                break;
            }
        }

        if !resolved {
            for index in items {
                let segments = &segments[*index];
                let suffix = segments[segments.len().saturating_sub(2)..].join("/");
                names[*index] = Some(labelled(name, &suffix));
            }
        }
    }

    names
        .into_iter()
        .zip(buffers)
        .map(|(name, buffer)| name.unwrap_or_else(|| buffer.name.clone()))
        .collect()
}

fn labelled(name: &SharedString, segment: &str) -> SharedString {
    if segment.is_empty() {
        name.clone()
    } else {
        SharedString::from(format!("{name} · {segment}"))
    }
}

pub(crate) struct Buffer {
    pub(crate) path: PathBuf,
    pub(crate) name: SharedString,
    pub(crate) icon: FileIcon,
    pub(crate) editor: Entity<EditorState>,
    pub(crate) is_dirty: bool,
    pub(crate) writable: bool,
    pub(crate) save_generation: u64,
    pub(crate) revision: u64,
    pub(crate) auto_save_task: Option<Task<()>>,
    pub(crate) diagnostics_generation: u64,
    pub(crate) diagnostics_task: Option<Task<()>>,
    pub(crate) _subscriptions: Vec<Subscription>,
}

impl Buffer {
    pub(crate) fn new(
        path: PathBuf,
        name: SharedString,
        icon: FileIcon,
        editor: Entity<EditorState>,
        writable: bool,
        subscriptions: Vec<Subscription>,
    ) -> Self {
        Self {
            path,
            name,
            icon,
            editor,
            is_dirty: false,
            writable,
            save_generation: 0,
            revision: 0,
            auto_save_task: None,
            diagnostics_generation: 0,
            diagnostics_task: None,
            _subscriptions: subscriptions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(case: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "lithe-editor-read-body-{case}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("建临时目录");
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn utf8_text_is_writable_and_exact() {
        let dir = TempDir::new("utf8");
        let path = dir.path("Hello.java");
        fs::write(&path, "class Hello {}\n").expect("写样本");

        let body = read_body(&path, "Hello.java");

        assert!(body.writable, "合法 UTF-8 正文必须可写");
        assert_eq!(body.text, "class Hello {}\n", "正文应与文件逐字相同");
    }

    #[test]
    fn lossy_conversion_is_not_writable() {
        let dir = TempDir::new("lossy");
        let path = dir.path("gbk.txt");
        fs::write(&path, [0x66u8, 0xFF, 0xFE, 0x0A]).expect("写样本");

        let body = read_body(&path, "gbk.txt");

        assert!(
            !body.writable,
            "有损转换后的正文不是文件字节的忠实副本，写回会改坏文件"
        );
        assert!(
            body.text.contains('\u{FFFD}'),
            "非法字节应显示为替换字符：{:?}",
            body.text
        );
    }

    #[test]
    fn binary_file_is_not_writable() {
        let dir = TempDir::new("binary");
        let path = dir.path("app.bin");
        fs::write(&path, [0x7Fu8, b'E', b'L', b'F', 0x00, 0x01]).expect("写样本");

        let body = read_body(&path, "app.bin");

        assert!(
            !body.writable,
            "二进制文件给出的是说明文案，写回会毁掉原文件"
        );
        assert!(
            body.text.contains("二进制"),
            "应是二进制说明：{:?}",
            body.text
        );
    }

    #[test]
    fn missing_file_is_not_writable() {
        let dir = TempDir::new("missing");
        let path = dir.path("nope.txt");

        let body = read_body(&path, "nope.txt");

        assert!(!body.writable, "读不到时给出的是失败说明，不可写");
        assert!(!body.text.is_empty(), "说明文案不应为空");
    }

    #[test]
    fn oversized_file_is_not_writable() {
        let dir = TempDir::new("oversized");
        let path = dir.path("huge.txt");
        let file = fs::File::create(&path).expect("建文件");
        file.set_len(MAX_EDITOR_BYTES + 1).expect("set_len");
        drop(file);

        let body = read_body(&path, "huge.txt");

        assert!(!body.writable, "超限文件不打开，给出的是说明");
        assert!(body.text.contains("MiB"), "应是超限说明：{:?}", body.text);
    }

    #[test]
    fn java_maps_to_java_language() {
        assert_eq!(language_for_file("HighlightProbe.java"), Some("java"));
        assert_eq!(
            language_for_file("Foo.JAVA"),
            Some("java"),
            "Windows 上扩展名大小写不敏感"
        );
    }

    #[test]
    fn json_maps_to_json_language() {
        assert_eq!(language_for_file("tsconfig.json"), Some("json"));
        assert_eq!(language_for_file("settings.jsonc"), Some("json"));
    }

    #[test]
    fn unmapped_extension_falls_back_to_none() {
        for name in [
            "notes.txt",
            "sample.xyz",
            "Makefile",
            "half.h",
            "app.rs",
            "Foo.java.bak",
            "application.properties",
            "build.gradle",
        ] {
            assert_eq!(language_for_file(name), None, "{name} 不该有语言名");
        }
    }

    #[test]
    fn project_files_map_to_their_languages() {
        for (name, expected) in [
            ("application.yml", "yaml"),
            ("application-dev.yaml", "yaml"),
            ("gradle.properties.toml", "toml"),
            ("schema.sql", "sql"),
            ("README.md", "markdown"),
            ("mvnw", "bash"),
            ("gradlew", "bash"),
            ("deploy.sh", "bash"),
            ("build.gradle.kts", "kotlin"),
            ("Main.kt", "kotlin"),
            ("pom.xml", "xml"),
            ("mapper/UserMapper.xml", "xml"),
            ("spring.xsd", "xml"),
        ] {
            assert_eq!(language_for_file(name), Some(expected), "{name} 的映射");
        }
    }
}
