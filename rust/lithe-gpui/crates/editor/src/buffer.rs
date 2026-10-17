use std::path::{Path, PathBuf};

use encoding_rs::{GBK, UTF_16BE, UTF_16LE};
use gpui::App;
use gpui::{Entity, SharedString, Subscription, Task};
use lithe_gpui_shared::icons::{FileIcon, file_icon};
use lithe_gpui_shared::tr_args;
use lithe_ui::editor::CodeEditor;

const MAX_EDITOR_BYTES: u64 = 2 * 1024 * 1024;

pub(crate) fn icon_for_file(name: &str, cx: &App) -> FileIcon {
    FileIcon::own(file_icon::fallback_icon(name)).with_theme_icon(name, cx)
}

/// The language a file is highlighted and served in, from the component library's registry.
///
/// The registry is the single source of truth for which extensions and file names belong to a language
/// (`lithe_gpui_languages` registers the grammars at startup), so this side keeps no table of its own. A
/// language the user switched off is not handed out: the file then opens with the built-in lexer colours,
/// which is what "unloading a language pack" means.
pub(crate) fn resolved_language(name: &str) -> Option<&'static lithe_ui::language::Language> {
    let language = lithe_ui::language::by_path(name)?;
    (!lithe_gpui_shared::languages::is_language_disabled(language.id)).then_some(language)
}

/// 语言 id（宿主的日志、Java 服务的接线判据与"这个文件是什么语言"都用它）。
pub(crate) fn language_for_file(name: &str) -> Option<&'static str> {
    resolved_language(name).map(|language| language.id)
}

/// 编辑器认识的文件编码。
///
/// 只列真会读写的几种：UTF-8（有无 BOM 分开记，写回时 BOM 的有无必须保持）、UTF-16 LE/BE、
/// GBK。GBK 与 UTF-16 的码表用 `encoding_rs`（浏览器同一份实现），不自己写表。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Gbk,
}

impl TextEncoding {
    /// 状态栏点一下之后的轮换顺序。
    pub(crate) const ALL: [Self; 5] = [
        Self::Utf8,
        Self::Utf8Bom,
        Self::Utf16Le,
        Self::Utf16Be,
        Self::Gbk,
    ];

    /// 状态栏与诊断里显示的名字。
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf8Bom => "UTF-8 BOM",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Gbk => "GBK",
        }
    }

    /// 轮换到下一个档位。
    pub(crate) fn next(self) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }
}

/// 行尾符。
///
/// 缓冲区**内存里一律 LF**：编辑器按行切分、按字符偏移定位，混着 `\r` 会让列号与折叠都对不上。
/// 文件里原来是哪种行尾符记录在 [`Body::line_ending`]，写回时再换回去。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::Crlf => "CRLF",
        }
    }

    pub(crate) fn toggled(self) -> Self {
        match self {
            Self::Lf => Self::Crlf,
            Self::Crlf => Self::Lf,
        }
    }
}

/// 一次解码的结果。
pub(crate) struct Decoded {
    pub(crate) text: String,
    pub(crate) encoding: TextEncoding,
    pub(crate) line_ending: LineEnding,
    /// 解出来有替换字符：字节不是这个编码的忠实副本，写回会改坏文件。
    pub(crate) lossy: bool,
}

pub(crate) struct Body {
    pub(crate) text: String,
    pub(crate) writable: bool,
    pub(crate) encoding: TextEncoding,
    pub(crate) line_ending: LineEnding,
}

pub(crate) fn read_body(path: &Path, name: &str) -> Body {
    fn notice_body(lines: &[String]) -> Body {
        Body {
            text: notice(lines),
            writable: false,
            encoding: TextEncoding::Utf8,
            line_ending: LineEnding::Lf,
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
            // 带 BOM 的 UTF-16 正文天然全是 0x00 字节。二进制判定必须让开这条路，否则
            // 「UTF-16 文件打不开」。超限判定在上面，`MAX_EDITOR_BYTES` 的口径没动。
            Ok(bytes) if bytes.contains(&0) && !starts_with_utf16_bom(&bytes) => {
                notice_body(&["二进制文件不在编辑器中打开。".to_string()])
            }
            Ok(bytes) => {
                let decoded = decode_body(&bytes);
                Body {
                    text: decoded.text,
                    writable: !decoded.lossy,
                    encoding: decoded.encoding,
                    line_ending: decoded.line_ending,
                }
            }
        },
    }
}

fn starts_with_utf16_bom(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF])
}

/// 辨认编码并解码，正文统一成 LF。
///
/// 顺序是 BOM → 合法 UTF-8 → GBK。GBK 是 Windows 中文环境最常见的历史编码，放在最后兜底：
/// 一份合法 UTF-8 正文在 GBK 里也可能解得出字，先判 UTF-8 才不会把 UTF-8 文件读成乱码。
pub(crate) fn decode_body(bytes: &[u8]) -> Decoded {
    let (text, encoding, lossy) = if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        match std::str::from_utf8(rest) {
            Ok(text) => (text.to_string(), TextEncoding::Utf8Bom, false),
            Err(_) => (
                String::from_utf8_lossy(rest).into_owned(),
                TextEncoding::Utf8Bom,
                true,
            ),
        }
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let (text, had_errors) = UTF_16LE.decode_without_bom_handling(rest);
        (text.into_owned(), TextEncoding::Utf16Le, had_errors)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let (text, had_errors) = UTF_16BE.decode_without_bom_handling(rest);
        (text.into_owned(), TextEncoding::Utf16Be, had_errors)
    } else if let Ok(text) = std::str::from_utf8(bytes) {
        (text.to_string(), TextEncoding::Utf8, false)
    } else {
        let (text, had_errors) = GBK.decode_without_bom_handling(bytes);
        if had_errors {
            (
                String::from_utf8_lossy(bytes).into_owned(),
                TextEncoding::Utf8,
                true,
            )
        } else {
            (text.into_owned(), TextEncoding::Gbk, false)
        }
    };

    let line_ending = detect_line_ending(&text);
    Decoded {
        text: normalize_to_lf(&text),
        encoding,
        line_ending,
        lossy,
    }
}

/// 文件里用的行尾符。混用时记 CRLF：写回 CRLF 至少不会让 Windows 工具看到半截 LF。
fn detect_line_ending(text: &str) -> LineEnding {
    if text.contains("\r\n") {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    }
}

/// 缓冲区一律 LF：先把 CRLF 换成 LF，再收掉孤立的 CR（老 Mac 行尾）。
fn normalize_to_lf(text: &str) -> String {
    if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text.to_string()
    }
}

/// 按选定编码与行尾符把缓冲区正文变成文件字节。
pub(crate) fn encode_body(
    text: &str,
    encoding: TextEncoding,
    line_ending: LineEnding,
) -> Vec<u8> {
    let text = match line_ending {
        LineEnding::Lf => std::borrow::Cow::Borrowed(text),
        LineEnding::Crlf => std::borrow::Cow::Owned(text.replace('\n', "\r\n")),
    };

    let mut out = Vec::with_capacity(text.len());
    match encoding {
        TextEncoding::Utf8 => out.extend_from_slice(text.as_bytes()),
        TextEncoding::Utf8Bom => {
            out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
            out.extend_from_slice(text.as_bytes());
        }
        TextEncoding::Utf16Le => {
            out.extend_from_slice(&[0xFF, 0xFE]);
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_le_bytes());
            }
        }
        TextEncoding::Utf16Be => {
            out.extend_from_slice(&[0xFE, 0xFF]);
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_be_bytes());
            }
        }
        TextEncoding::Gbk => {
            let (encoded, _, _) = GBK.encode(&text);
            out.extend_from_slice(&encoded);
        }
    }
    out
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

/// A buffer's stable identity: a file on disk, or a scratch buffer that has no path yet.
///
/// Files are keyed by their path, so opening the same file twice lands on the same tab. Scratch
/// buffers have no path, so they carry a number that a pane never reuses; that keeps two of them
/// distinct and lets Save As move one onto a path key without an ambiguous duplicate.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum BufferKey {
    Disk(PathBuf),
    Untitled(u32),
}

impl BufferKey {
    /// The path of a file on disk; `None` for a scratch buffer.
    pub(crate) fn disk_path(&self) -> Option<&Path> {
        match self {
            BufferKey::Disk(path) => Some(path),
            BufferKey::Untitled(_) => None,
        }
    }

    /// The path shown and used for a tab id: real files use their path, scratch buffers the name
    /// they display, which is unique because the number never repeats within a pane.
    pub(crate) fn display(&self, name: &str) -> PathBuf {
        match self {
            BufferKey::Disk(path) => path.clone(),
            BufferKey::Untitled(_) => PathBuf::from(name),
        }
    }
}

pub(crate) struct Buffer {
    /// The identity every lookup compares on (never `name`, which can collide across folders).
    pub(crate) key: BufferKey,
    /// The file path for a disk buffer, or the display name for a scratch buffer; the latter is
    /// only ever shown, never opened or written — use [`Self::disk_path`] for those.
    pub(crate) path: PathBuf,
    pub(crate) name: SharedString,
    pub(crate) icon: FileIcon,
    pub(crate) editor: Entity<CodeEditor>,
    pub(crate) is_dirty: bool,
    pub(crate) writable: bool,
    /// 文件读进来时的编码。写回按它编码；状态栏显示它。
    pub(crate) encoding: TextEncoding,
    /// 文件读进来时的行尾符。缓冲区正文始终是 LF，写回时换回这一档。
    pub(crate) line_ending: LineEnding,
    pub(crate) save_generation: u64,
    pub(crate) revision: u64,
    pub(crate) auto_save_task: Option<Task<()>>,
    pub(crate) diagnostics_generation: u64,
    pub(crate) diagnostics_task: Option<Task<()>>,
    /// 内嵌提示与 Code Lens：与诊断同一条代次/正文复核规矩，但各走各的计数——
    /// 两者取数节奏不同（诊断要退避重试，注记不需要），共用一个计数会互相误判过期。
    pub(crate) decorations_generation: u64,
    pub(crate) decorations_task: Option<Task<()>>,
    pub(crate) _subscriptions: Vec<Subscription>,
}

impl Buffer {
    pub(crate) fn new(
        key: BufferKey,
        name: SharedString,
        icon: FileIcon,
        editor: Entity<CodeEditor>,
        writable: bool,
        subscriptions: Vec<Subscription>,
    ) -> Self {
        let path = key.display(&name);
        Self {
            key,
            path,
            name,
            icon,
            editor,
            is_dirty: false,
            writable,
            // 新建缓冲（无标题 / 只读虚拟正文）没有源文件，按 UTF-8 + LF 写。
            encoding: TextEncoding::Utf8,
            line_ending: LineEnding::Lf,
            save_generation: 0,
            revision: 0,
            auto_save_task: None,
            diagnostics_generation: 0,
            diagnostics_task: None,
            decorations_generation: 0,
            decorations_task: None,
            _subscriptions: subscriptions,
        }
    }

    /// The file path this buffer is backed by, or `None` for a scratch buffer.
    pub(crate) fn disk_path(&self) -> Option<&Path> {
        self.key.disk_path()
    }

    pub(crate) fn is_disk(&self) -> bool {
        matches!(self.key, BufferKey::Disk(_))
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
    fn crlf_file_is_read_as_lf_and_remembers_crlf() {
        let dir = TempDir::new("crlf");
        let path = dir.path("Win.java");
        fs::write(&path, "class A {\r\n}\r\n").expect("写样本");

        let body = read_body(&path, "Win.java");

        assert_eq!(body.text, "class A {\n}\n", "缓冲区里应收成 LF");
        assert_eq!(body.line_ending, LineEnding::Crlf, "文件原来是 CRLF");
        assert_eq!(body.encoding, TextEncoding::Utf8);
        assert_eq!(
            encode_body(&body.text, body.encoding, body.line_ending),
            b"class A {\r\n}\r\n".to_vec(),
            "写回必须原样还原 CRLF"
        );
    }

    #[test]
    fn utf8_bom_is_stripped_and_remembered() {
        let dir = TempDir::new("bom");
        let path = dir.path("bom.txt");
        fs::write(&path, [0xEFu8, 0xBB, 0xBF, b'h', b'i']).expect("写样本");

        let body = read_body(&path, "bom.txt");

        assert_eq!(body.text, "hi", "BOM 不该留在正文里");
        assert_eq!(body.encoding, TextEncoding::Utf8Bom);
        assert!(body.writable);
        assert_eq!(
            encode_body(&body.text, body.encoding, body.line_ending),
            vec![0xEF, 0xBB, 0xBF, b'h', b'i'],
            "写回必须把 BOM 加回去"
        );
    }

    #[test]
    fn utf16le_file_opens_instead_of_being_called_binary() {
        let dir = TempDir::new("utf16");
        let path = dir.path("notes.txt");
        // "a\n" 的 UTF-16LE 正文，加上 BOM —— 正文里全是 0x00 字节。
        fs::write(&path, [0xFFu8, 0xFE, b'a', 0x00, 0x0A, 0x00]).expect("写样本");

        let body = read_body(&path, "notes.txt");

        assert_eq!(body.text, "a\n", "UTF-16LE 应正常解码");
        assert_eq!(body.encoding, TextEncoding::Utf16Le);
        assert!(body.writable, "带 BOM 的 UTF-16 不是二进制");
        assert_eq!(
            encode_body(&body.text, body.encoding, body.line_ending),
            vec![0xFF, 0xFE, b'a', 0x00, 0x0A, 0x00]
        );
    }

    #[test]
    fn gbk_bytes_fall_back_to_gbk_and_stay_writable() {
        let dir = TempDir::new("gbk-ok");
        let path = dir.path("cn.txt");
        // "中文" 的 GBK 编码；这串字节不是合法 UTF-8。
        fs::write(&path, [0xD6u8, 0xD0, 0xCE, 0xC4]).expect("写样本");

        let body = read_body(&path, "cn.txt");

        assert_eq!(body.encoding, TextEncoding::Gbk, "应认出 GBK");
        assert_eq!(body.text, "中文", "GBK 应解出正确汉字");
        assert!(body.writable, "无损的 GBK 解码可以写回");
        assert_eq!(
            encode_body(&body.text, body.encoding, body.line_ending),
            vec![0xD6, 0xD0, 0xCE, 0xC4],
            "按 GBK 写回必须逐字节还原"
        );
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
        languages();
        assert_eq!(language_for_file("HighlightProbe.java"), Some("java"));
        assert_eq!(
            language_for_file("Foo.JAVA"),
            Some("java"),
            "Windows 上扩展名大小写不敏感"
        );
    }

    #[test]
    fn json_maps_to_json_language() {
        languages();
        assert_eq!(language_for_file("tsconfig.json"), Some("json"));
        assert_eq!(language_for_file("settings.jsonc"), Some("json"));
    }

    #[test]
    fn unmapped_extension_falls_back_to_none() {
        languages();
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
        languages();
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

    /// 注册表是进程级的全局状态：每个用到 `language_for_file` 的测试都自己注册一次，
    /// 免得测试并行跑时依赖别人先跑。重复注册是幂等的（注册表自己记一条警告）。
    ///
    /// 这里**不**断言「关掉某种语言」那条分支：`set_disabled_languages` 改的是进程级全局状态，
    /// 与并行跑的同批测试互相踩。那条分支的守卫在 `shared::languages` 与设置侧，不在这里重复。
    fn languages() {
        lithe_gpui_languages::register();
    }
}
