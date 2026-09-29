//! 编辑区的**数据侧**：一个打开的 buffer 与它的正文来源。
//!
//! 从 `shell_probe/editor.rs` 原样拆出（逐字搬迁，只调整可见性与 import）。
//! 读盘与降级文案在 `read_body` / `notice`，标签显示名的同名区分在 `display_names`；
//! 界面与 `EditorPane` 的打开 / 切换 / 关闭 / 保存 / 自动保存在 `editor_view.rs`。

use std::path::{Path, PathBuf};

use gpui_kit::App;
use gpui_kit::component::input::EditorState;
use gpui_kit::{Entity, SharedString, Subscription, Task};
use lithe_gpui_shared::icons::{FileIcon, file_icon};
use lithe_gpui_shared::tr_args;

/// 编辑器一次读入的字节上限：2 MiB。
///
/// 真机对大文件是"照常打开但关掉语言智能/Git Blame/CodeLens"并给一条
/// 「仍然启用」横幅（`windows/tauri/src/features/editor/components/code-editor.tsx:664-675`、
/// `windows/tauri/src/i18n/locale.ts:8408-8409`）；探针没有语言服务，
/// 读进来只是占内存，所以这里**整篇不读**，正文给一条说明（见 [`read_body`]）。
const MAX_EDITOR_BYTES: u64 = 2 * 1024 * 1024;

/// 按文件名挑标签图标：**先查真机默认图标主题（`idea-icons`）的文件类型图标，查不到再回落 Lucide**。
///
/// 与 `crates/explorer/src/model.rs` 的同名函数**共用同一份实现**
/// （`lithe_gpui_shared::icons::file_icon`），所以两处的判断不会再漂移：
///
/// 1. **真源**：`lithe-gpui/assets/icon-themes/idea/**` —— 真机默认图标主题
///    （`windows/tauri/src/features/settings/config/default-settings.ts:100` 的
///    `iconTheme: "idea-icons"`）。按 `filenames` → `fileExtensions` → `defaultFile` 顺序查，
///    明暗由 `Theme::global(cx).is_dark()` 决定。图标是彩色的，渲染走 `img()`。
/// 2. **回落**：`gpui_kit::assets::IconName`（全量 Lucide 目录）—— 主题包没有对应条目时用它。
///    Lucide 目录**没有 `file-json.svg`**，所以 JSON / JSONC 取语义最近的 `FileBraces`
///    （`icons/file-braces.svg`）；这条例外与两张旧表逐字同源，现在只有一份。
///
/// 真机对照：`ThemedFileIcon` + `features/tabs/components/tab-bar-item.tsx:246-251`。
pub(crate) fn icon_for_file(name: &str, cx: &App) -> FileIcon {
    FileIcon::lucide(file_icon::lucide_fallback(name)).with_theme_icon(name, cx)
}

/// 按文件名挑**语法高亮语言名**：交给 `EditorState::language(..)` 的那个名字。
///
/// 返回值是**语言名**，不是扩展名、也不是图标名：它必须是上游 `Language::from_name`
/// 认得的字面量（`gpui-component-0.6.6/src/highlighter/languages.rs:178-256`）。
/// 传不认识的名字不会报错 —— 工厂拿不到 highlighter 就**静默不上色**
/// （`gpui-base-0.6.6/src/input/base/mode.rs:290-298`），所以"名字写错"和
/// "没写 `.language`" 在界面上完全一样，只能靠这张表自己保证正确。
///
/// ## 为什么表里只有两种语言（而不是一长串扩展名）
///
/// 光有语言名不亮：名字还得在 `LanguageRegistry` 里注册过 grammar，而 grammar 由
/// **Cargo feature** 决定（`gpui-component-0.6.6/Cargo.toml:112-115`：`tree-sitter-java`
/// 才 `dep:tree-sitter-java`）。本 crate 只开了 `tree-sitter-java`（见 `Cargo.toml`），
/// 它隐含基础特性 `tree-sitter` = `dep:tree-sitter` + `dep:tree-sitter-json`
/// （同文件 `:52-55`），所以**当前真正能亮的只有 Java 与 JSON**。
///
/// 其余扩展名一律回落 `None`：把没开 feature 的扩展名写进表里就是"假装支持"，
/// 用户看到的仍是纯文本，与不写这张表完全一样，却会让人误以为接好了。
/// 要加语言：`Cargo.toml` 打开对应的 `tree-sitter-<语言>` feature，**再**在下面加一行。
///
/// ⚠️ 语言名与扩展名**不是**一一对应，别自造：`.h` 只能映射到 `"c"`
/// （上游没有 `"h"` 这个名字，`languages.rs:186-187` 只认 `"c"`），
/// `.ts` 要写 `"typescript"` 而不是 `"ts"` 之外的任何东西（`languages.rs:248-249`）。
pub(crate) fn language_for_file(name: &str) -> Option<&'static str> {
    // 扩展名大小写不敏感：Windows 上 `Foo.JAVA` 与 `foo.java` 是同一个文件。
    let extension = Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        // `"java"` → `Language::Java`（`languages.rs:211`），grammar 来自 `tree-sitter-java`。
        Some("java") => Some("java"),
        // `"json"` → `Language::Json`（`languages.rs:181`），**不需要**额外 feature：
        // 上面的 `tree-sitter-java` 已经把基础特性 `tree-sitter`（含 tree-sitter-json）带进来了。
        Some("json" | "jsonc") => Some("json"),
        _ => None,
    }
}

/// 读盘结果：正文 + **这份正文是不是文件的忠实副本**。
///
/// 第二个字段是"能不能写回去"的判据（[`Buffer::writable`]）：读不到 / 超大 / 二进制 /
/// 非 UTF-8 这四种情况给出的都是**宿主补的说明或替换字符**，把它们写回磁盘会毁掉原文件，
/// 所以这些 buffer 一律标成不可写、编辑器也只读。
pub(crate) struct Body {
    /// 交给编辑器的正文。
    pub(crate) text: String,
    /// `true` = 正文就是文件内容（可以保存）；`false` = 正文是说明/有损转换的替代文本。
    pub(crate) writable: bool,
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
///   （git HEAD `lithe-gpui/shell/src/bin/shell_probe/panels.rs::open_document`，
///   该实现已删除，只在 git 历史里），因为 `locale.ts` 里没有对应文案：
///   `editor.largeFileServicesDisabled`（`:8408`）说的是"关了语言服务"而不是"不打开"。
pub(crate) fn read_body(path: &Path, name: &str) -> Body {
    /// 说明性正文：不可写（写回去会覆盖真实文件）。
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
                // 非 UTF-8 的文本用有损转换：编辑器只接受 `SharedString`，
                // 真机 Monaco 也会把非法字节显示成替换字符。
                // ⚠️ 但这份正文**不是**文件字节的忠实副本（非法字节已经变成 U+FFFD），
                // 写回去会改坏文件 —— 所以标成不可写。真机的编码探测与"按编码保存"
                // 还没有（阶段 9 范围外，与"外部修改冲突"同批后置）。
                Err(_) => Body {
                    text: String::from_utf8_lossy(&bytes).to_string(),
                    writable: false,
                },
            },
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
    /// 打开时用的路径，去重键（同一路径重复打开只切标签、不读盘），也是保存的写入目标。
    pub(crate) path: PathBuf,
    /// 文件名，也是 [`display_names`] 分组的键。
    pub(crate) name: SharedString,
    /// 标签左侧的文件类型图标。
    ///
    /// 是 [`FileIcon`] 而不是裸 `IconName`：它同时带上"真机默认图标主题里的真源资源路径"
    /// 与"Lucide 回落字形"（见 [`icon_for_file`] 的文档）。
    ///
    /// **存值而不是每帧现查**：buffer 是长期存活的（切标签、保存、自动保存都碰它），
    /// 而查表要 `&App`。打开那一刻的明暗就够用 —— 与真机差别是"切主题后已打开的标签
    /// 仍用打开时那张变体"（真机 `tab-bar-item.tsx:246-251` 是每帧现查）。
    /// 这个偏差只在"打开标签后切换明暗主题"时可见，且只影响那一张 16×16 图标；
    /// explorer 侧没有这个偏差（它在 `render` 里现查）。
    pub(crate) icon: FileIcon,
    /// 正文状态：文件内容，读不到时是说明文案。
    pub(crate) editor: Entity<EditorState>,
    /// 未保存圆点的真实值：`InputEvent::Change` 置 `true`，**写盘成功**置 `false`。
    ///
    /// 判据照真机的文档生命周期而不是"正文 == 磁盘内容"：
    /// `buffer.isDirty = documentLifecycle.status !== "clean"`
    /// （`windows/tauri/src/features/editor/stores/buffer.store.ts:307,1569`），
    /// 所以"改回原样"仍然算脏 —— 不做每次按键都物化整篇正文的内容比较。
    pub(crate) is_dirty: bool,
    /// 这份正文能不能写回磁盘。
    ///
    /// `false` = 正文是宿主补的说明（读不到 / 超过 [`MAX_EDITOR_BYTES`] / 二进制）
    /// 或非 UTF-8 的有损替代文本（见 [`Body`]）。这种 buffer 的编辑器被设成只读，
    /// 不参与脏标记、`Ctrl+S` 与自动保存 —— 否则会把说明文案写进用户的真实文件。
    pub(crate) writable: bool,
    /// 防抖自动保存的版本号：**每改动一次 +1**。
    ///
    /// 它是 Windows stale-content 守卫的等价物：旧定时器醒来时发现版本已经变了就不写
    /// （`windows/tauri/src/features/editor/stores/editor-app.store.ts:518-529` 比的是
    /// 正文内容，内容变一次版本必然也变一次，判据等价，且不必为每次按键物化整篇正文）。
    pub(crate) save_generation: u64,
    /// 正文修订号：**每改动一次 +1**，与保存无关。
    ///
    /// 跳转请求要发到后台线程（Core 调用是同步的，见 `shared::core_client`），结果回来时
    /// 正文可能已经变了 —— 那时按旧正文算出来的行列已经没有意义。它的判据与
    /// [`Buffer::save_generation`] 不同：那个会被"写盘成功"重置语义，而修订号只增不减，
    /// 所以两者分开记（一个字段两用会让"保存过就不算脏"这类改动悄悄破坏跳转守卫）。
    pub(crate) revision: u64,
    /// 待执行的防抖自动保存任务。
    ///
    /// gpui 的 `Task` **一 drop 就取消**，所以"换掉上一个"就是 Windows 的 `clearTimeout`
    /// （`editor-app.store.ts:505-509`）：连续输入只会留下最后一次。
    pub(crate) auto_save_task: Option<Task<()>>,
    /// 这个 buffer 最新一次诊断刷新的**代次**（`EditorPane::diagnostics_generation` 的值）。
    ///
    /// 结果从后台回来时按它判断"这次结果还算不算数"：同一份文件被排了两次时只认新的那次。
    /// 与 [`Buffer::revision`] 是两个不同的问题 —— 代次管"有没有更新的请求"，
    /// 修订号管"正文有没有变过"（见 `crate::diagnostics` 的模块文档）。
    pub(crate) diagnostics_generation: u64,
    /// 在飞的诊断刷新任务（同上：drop 即取消，换掉它就是取消上一次）。
    pub(crate) diagnostics_task: Option<Task<()>>,
    /// 对这个 `EditorState` 的订阅（`InputEvent` + 通知）。
    ///
    /// 必须**被持有**：`Subscription` 一 drop 就取消（gpui 的 RAII 语义），
    /// 丢在局部变量里等于没订阅。带 `_` 前缀是因为除了"持有"之外没人读它
    /// （与 `ShellWorkspace::_settings_subscription` 同一约定）。
    pub(crate) _subscriptions: Vec<Subscription>,
}

impl Buffer {
    /// 新建一个 buffer：刚读进来，所以干净（`is_dirty = false`）。
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

    /// 一个"进程内唯一、用后即删"的临时目录。
    ///
    /// 名字用「用例名 + 进程 id」而不是时钟/随机数：测试要确定性，而同名冲突只可能来自
    /// 同一个进程里的不同用例 —— 它们各自用不同的 `case`。
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

    // `read_body` 的分类结果是**保存路径的安全闸门**：只有"正文就是文件字节的忠实副本"
    // （`writable == true`）才允许写回磁盘。下面几条守的都是"不可写"的情形 —— 它们一旦回归，
    // `Ctrl+S` 与防抖自动保存就会把宿主补的说明文案，或带 U+FFFD 的有损文本，写进用户的真实文件。

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
        // 0xFF 0xFE 不是合法 UTF-8 序列 —— 真机上 GBK/CP936 文件走到的就是这个分支。
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

        assert!(!body.writable, "二进制文件给出的是说明文案，写回会毁掉原文件");
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
        // 用 `set_len` 造稀疏文件，避免真的写 2 MiB 进磁盘。
        let file = fs::File::create(&path).expect("建文件");
        file.set_len(MAX_EDITOR_BYTES + 1).expect("set_len");
        drop(file);

        let body = read_body(&path, "huge.txt");

        assert!(!body.writable, "超限文件不打开，给出的是说明");
        assert!(body.text.contains("MiB"), "应是超限说明：{:?}", body.text);
    }

    // `language_for_file` 的返回值直接进 `EditorState::language(..)`。**写错名字不会报错**
    // （上游只是静默不上色，见该函数文档），所以"表里到底有哪些扩展名"只能靠这几条守住：
    // 它们一旦回归（例如有人把 `.xyz` 也映射进来，或把 `java` 拼错），界面会退回纯文本
    // 而没有任何日志，肉眼很难发现。

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
        // 没开 grammar feature 的扩展名**不进表**：写进来只是"假装支持"。
        for name in [
            "notes.txt",
            "sample.xyz",
            "Makefile",
            "half.h",
            "Main.kt",
            "app.rs",
            "Foo.java.bak",
        ] {
            assert_eq!(language_for_file(name), None, "{name} 不该有语言名");
        }
    }
}
