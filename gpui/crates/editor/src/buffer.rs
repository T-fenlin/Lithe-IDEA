//! 编辑区的**数据侧**：一个打开的 buffer 与它的正文来源。
//!
//! 从 `shell_probe/editor.rs` 原样拆出（逐字搬迁，只调整可见性与 import）。
//! 读盘与降级文案在 `read_body` / `notice`，标签显示名的同名区分在 `display_names`；
//! 界面与 `EditorPane` 的打开 / 切换 / 关闭在 `editor_view.rs`。

use std::path::{Path, PathBuf};

use gpui_kit::assets::IconName;
use gpui_kit::component::input::EditorState;
use gpui_kit::{Entity, SharedString};

/// 编辑器一次读入的字节上限：2 MiB。
///
/// 真机对大文件是"照常打开但关掉语言智能/Git Blame/CodeLens"并给一条
/// 「仍然启用」横幅（`windows/tauri/src/features/editor/components/code-editor.tsx:664-675`、
/// `windows/tauri/src/i18n/locale.ts:8408-8409`）；探针没有语言服务，
/// 读进来只是占内存，所以这里**整篇不读**，正文给一条说明（见 [`read_body`]）。
const MAX_EDITOR_BYTES: u64 = 2 * 1024 * 1024;

/// 文件类型图标按后缀选，字形一律来自全量 Lucide 目录。
///
/// 应用注册的是 `gpui_kit::assets::AllAssets`，嵌入
/// `gpui-kit-assets-0.6.6/assets/icons/` 下的全部 1830 个 SVG（`src/native_assets.rs:8-11`），
/// 而 `gpui_kit::assets::IconName` 就是按这份目录生成的完整枚举（`build.rs:20-51`），
/// 所以语言级/图片级字形（`FileCode`、`Image` 等）都在可用范围内：
///
/// - 图片后缀：`Image`（`icons/image.svg`）；
/// - JSON / JSONC：目录里**没有 `file-json.svg`**，取 `FileBraces`（`icons/file-braces.svg`）；
/// - 源码后缀：`FileCode`（`icons/file-code.svg`）；
/// - Markdown / 文本：`FileText`（`icons/file-text.svg`）；
/// - 配置（toml/yaml/ini/conf）：`Settings`（`icons/settings.svg`）；
/// - 依赖锁 / 包文件：`Package`（`icons/package.svg`）；
/// - 脚本（sh/ps1/bat）：`Terminal`（`icons/terminal.svg`）；
/// - 其余：`File`（`icons/file.svg`）。
///
/// 真机按 `ThemedFileIcon` + 图标主题选（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:246-251`），
/// 那是 Windows 自己的一套图标资源，gpui-kit 侧无法等价复刻。
pub(crate) fn icon_for_file(name: &str) -> IconName {
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());

    match extension.as_deref() {
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "svg" | "bmp" | "avif") => {
            IconName::Image
        }
        Some("json" | "jsonc") => IconName::FileBraces,
        Some("md" | "markdown" | "txt" | "rst" | "adoc") => IconName::FileText,
        Some("toml" | "yaml" | "yml" | "ini" | "conf" | "cfg" | "properties" | "editorconfig") => {
            IconName::Settings
        }
        Some("lock") => IconName::Package,
        Some("sh" | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "bat" | "cmd") => IconName::Terminal,
        Some(
            "rs" | "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" | "py" | "java"
            | "kt" | "kts" | "swift" | "go" | "c" | "h" | "cc" | "cpp" | "hpp" | "cs" | "rb"
            | "php" | "vue" | "svelte" | "sql" | "css" | "scss" | "less" | "html" | "xml"
            | "gradle" | "lua" | "dart" | "scala" | "ex" | "exs" | "dockerfile",
        ) => IconName::FileCode,
        _ => IconName::File,
    }
}

/// 读盘并按三种"不把内容交给编辑器"的情况给出正文说明。
///
/// 与真机的差别：真机会把文件读进来再降级（大文件关语言服务、
/// 二进制走二进制预览器 `windows/tauri/src/i18n/locale.ts:4680` 的 `panes.binaryFilePreview`）；
/// 探针三种情况都只在正文里留说明文案。文案出处：
///
/// - 读不到：`无法打开 {name}` 取中文 i18n 的 `files.openFailed`
///   （`windows/tauri/src/i18n/locale.ts:7386`），第二行补系统原始错误便于定位；
/// - 超过 [`MAX_EDITOR_BYTES`] 与二进制两句沿用上一轮已验证实现
///   （git HEAD `gpui/shell/src/bin/shell_probe/panels.rs::open_document`，
///   该实现已删除，只在 git 历史里），因为 `locale.ts` 里没有对应文案：
///   `editor.largeFileServicesDisabled`（`:8408`）说的是"关了语言服务"而不是"不打开"。
pub(crate) fn read_body(path: &Path, name: &str) -> String {
    match std::fs::metadata(path) {
        Err(error) => notice(&[format!("无法打开 {name}"), error.to_string()]),
        Ok(metadata) if metadata.len() > MAX_EDITOR_BYTES => notice(&[format!(
            "文件超过 {} MiB，暂不在编辑器中打开。",
            MAX_EDITOR_BYTES / (1024 * 1024)
        )]),
        Ok(_) => match std::fs::read(path) {
            Err(error) => notice(&[format!("无法打开 {name}"), error.to_string()]),
            Ok(bytes) if bytes.contains(&0) => {
                notice(&["二进制文件不在编辑器中打开。".to_string()])
            }
            // 非 UTF-8 的文本用有损转换：编辑器只接受 `SharedString`，
            // 真机 Monaco 也会把非法字节显示成替换字符。
            Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
        },
    }
}

/// 把说明文案拼成正文。
///
/// 每行加 `// ` 前缀（沿用上一轮实现的约定），标明这一行是宿主补的说明
/// **不是文件内容**，用户一眼能看出来。
fn notice(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| format!("// {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 取路径的**目录**段（丢掉文件名），与真机的 `getPathSegments`
/// 一致：先把 `\` 归一成 `/` 再按 `/` 切，然后 `slice(0, -1)`
/// （`windows/tauri/src/features/tabs/utils/path-shortener.ts:7-13`）。
fn path_segments(path: &Path) -> Vec<String> {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let mut parts: Vec<String> = normalized.split('/').map(str::to_string).collect();
    parts.pop();
    parts
}

/// 从路径末尾往前数第 `depth` 个目录段；越界时给空串
/// （对应 TS 的 `segments[segments.length - depth] ?? ""`，
/// `windows/tauri/src/features/tabs/utils/path-shortener.ts:79`）。
fn segment_from_end(segments: &[String], depth: usize) -> &str {
    segments
        .len()
        .checked_sub(depth)
        .and_then(|index| segments.get(index))
        .map(String::as_str)
        .unwrap_or("")
}

/// 标签显示名：真机 `calculateDisplayNames` 的等价实现
/// （`windows/tauri/src/features/tabs/utils/path-shortener.ts:31-113`）。
///
/// 规则（逐条对照该文件）：
///
/// 1. 按**文件名**分组（`:38-59`）；组里只有一个文件 → 只显示文件名（`:64-68`）；
/// 2. 组里有同名文件 → 从路径末尾往前找**第一个"所有同名文件的这一段都不同"的深度**
///    `depth`，显示 `文件名 · 该目录段`（`:75-93`）。**这就是多个 `mod.rs` 能区分开的原因**；
/// 3. 找不到这样的单层（例如 `a/p`、`b/p`、`a/q`）→ 退化成最后两段目录 `文件名 · a/p`
///    （`:97-102`）；
/// 4. 兜底用文件名（`:105-110` 的 `buffer.name`）。
///
/// 真机的分隔符是 `" · "`（U+00B7，前后各一个空格，`:89`），这里逐字照搬。
pub(crate) fn display_names(buffers: &[Buffer]) -> Vec<SharedString> {
    let segments: Vec<Vec<String>> = buffers
        .iter()
        .map(|buffer| path_segments(&buffer.path))
        .collect();
    let mut groups: Vec<(SharedString, Vec<usize>)> = Vec::new();
    let mut names: Vec<Option<SharedString>> = vec![None; buffers.len()];

    for (index, buffer) in buffers.iter().enumerate() {
        // 先算出分组下标再动 `groups`：把查找写进 `match` 的 scrutinee 会让
        // `iter()` 的借用活到整个 `match`，`None` 分支的 `push` 就借不动了。
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

/// `文件名 · 区分段`；区分段为空时退回文件名（TS 里的 `segment ? ... : fileName`）。
fn labelled(name: &SharedString, segment: &str) -> SharedString {
    if segment.is_empty() {
        name.clone()
    } else {
        SharedString::from(format!("{name} · {segment}"))
    }
}

/// 一个打开的 buffer（标签 + 正文状态）。
///
/// 真机里"标签"不是文件而是 buffer，一个 buffer 一个编辑模型；
/// 这里对应"一个标签一个 `EditorState`"，所以切标签只是换渲染哪个编辑器，
/// 不需要把正文倒来倒去，撤销栈与滚动位置各自独立。
pub(crate) struct Buffer {
    /// 打开时用的路径，去重键（同一路径重复打开只切标签、不读盘）。
    pub(crate) path: PathBuf,
    /// 文件名，也是 [`display_names`] 分组的键。
    pub(crate) name: SharedString,
    /// 标签左侧的文件类型图标。
    pub(crate) icon: IconName,
    /// 正文状态：文件内容，读不到时是说明文案。
    pub(crate) editor: Entity<EditorState>,
    /// 未保存圆点。
    ///
    /// **本轮恒为 `false`**：没有订阅编辑事件，不去编造脏状态
    /// （`gpui/UI-MAP.md` §1「状态齐全」要求的是真实状态，不是假圆点）。
    /// 圆点的位置/尺寸/颜色已经按规格写在 [`EditorPane::render_tab`]，
    /// 接上 `InputEvent` 后把这里改成真实值即可。
    pub(crate) is_dirty: bool,
}

impl Buffer {
    /// 新建一个 buffer：`is_dirty` 恒为 `false`（理由见字段自己的注释）。
    pub(crate) fn new(
        path: PathBuf,
        name: SharedString,
        icon: IconName,
        editor: Entity<EditorState>,
    ) -> Self {
        Self {
            path,
            name,
            icon,
            editor,
            is_dirty: false,
        }
    }
}
