//! 编辑区：**标签栏 + 正文 + 空状态**。
//!
//! 形状照 Windows 前端的 `windows/tauri/src/features/panes/components/pane-container.tsx:1094-1100`：
//! 上面一条标签栏、下面是 `relative min-h-0 flex-1 overflow-hidden` 的正文区；
//! **没有活动 buffer 时标签栏仍在**，只有正文位置换成空状态
//! （`windows/tauri/src/features/panes/components/pane-container.tsx:1100` →
//! `windows/tauri/src/features/panes/components/empty-editor-state.tsx`）。
//!
//! 界面规格来源是 Windows 前端（`gpui/UI-MAP.md` 顶部横幅），
//! 观感用 gpui-kit 0.6.6 真实存在的组件与主题 token 实现，不逐层翻译 Tailwind class：
//!
//! > 引用约定：Windows 侧路径一律写全（相对仓库根）；gpui-kit 侧的行号相对
//! > `gpui-component-0.6.6/src/`（`base::` 的少量引用相对 `gpui-base-0.6.6/src/`），
//! > 两类根目录见 `gpui/UI-MAP.md` §1.1 第 7 条。
//!
//! - 标签条 = `component::tab::TabBar` 的 `TabVariant::Underline`
//!   （Windows 活动标签是"透明底 + 底边主色条"= IntelliJ 风格，
//!   `windows/tauri/src/ui/tab-bar.tsx:204-209`）；
//! - 标签 = `Tab::child(...)`（**不是** `Tab::icon()`，后者会丢掉 label）；
//! - 空状态 = `component::empty::{Empty, EmptyHeader, EmptyMedia, EmptyTitle, EmptyDescription}`；
//! - 正文 = `component::input::Editor`（`EditorState` 每个标签一份，对应真机"一个 buffer 一个
//!   Monaco model"）。
//!
//! 本轮**明确没做**的两件事（都只做外观/位置，行为留待后续接线）：
//!
//! 1. **脏标记（未保存圆点）恒为干净**：`Buffer::is_dirty` 永远写不进去
//!    （没有编辑事件订阅），所以圆点不会出现；圆点的位置与颜色按规格写在
//!    [`EditorPane::render_tab`] 里，接上 `InputEvent` 后把它改成真实值即可；
//! 2. **`← →` 只有外观 + 禁用态**：探针里没有 jump list（历史栈），
//!    两个按钮恒为 `disabled(true)`，见 [`EditorPane::nav_button`]。
//!
//! 还有几处**组件写死、公开 API 改不动**的尺寸偏差（细节见交付报告）：
//! `TabVariant::Underline` 的标签高度是 36px（真机标签 28px 居中在 36px 条里，
//! `tab/tab.rs:38-41`）、标签字号默认 14px（真机 `--ui-text-chrome` 是 13px，
//! `tab/tab.rs:803-807` 按 `Size` 档位给字号）、标签之间的间距写死 16px
//! （真机 `--lithe-chrome-gap` 是 4px；`tab/tab_bar.rs:393-403` 只把这个 gap 给
//! 内部的 `tabs-inner`，`TabBar` 没有公开的 gap setter，`.gap()` 设的是外层容器）。
//! 这些值都在组件自己的 `render` 里最后写入，外层 `.h()` / `.text_size()` 覆盖不掉
//! （`Tab::style()` 与组件自己写的字段是同一个 `StyleRefinement`，后写的赢）。
//!
//! 本轮**范围外**（真机有、这里没有）还有：标签悬停才显示关闭按钮的那一档
//! （`TabBar` 不暴露每个标签的悬停状态）、标签拖拽重排 / 拖出成新窗格、
//! 标签右键菜单、面包屑栏、查找替换、分屏与轮播、外部冲突横幅与大文件「仍然启用」降级、
//! 状态栏的光标位置/编码/缩进等项（那些属于外壳的状态栏区域，不在编辑区里）。

use std::path::{Path, PathBuf};

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::input::{Editor, EditorState};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::tab::{Tab, TabBar, TabVariant};
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, Sizable as _};
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollWheelEvent, SharedString, Styled as _, Window, div, point, px,
    relative,
};

/// 编辑器一次读入的字节上限：2 MiB。
///
/// 真机对大文件是"照常打开但关掉语言智能/Git Blame/CodeLens"并给一条
/// 「仍然启用」横幅（`windows/tauri/src/features/editor/components/code-editor.tsx:664-675`、
/// `windows/tauri/src/i18n/locale.ts:8408-8409`）；探针没有语言服务，
/// 读进来只是占内存，所以这里**整篇不读**，正文给一条说明（见 [`read_body`]）。
const MAX_EDITOR_BYTES: u64 = 2 * 1024 * 1024;

/// 标签栏高度 36px：`--lithe-tab-bar-height`（=`--lithe-pane-header-height`=2.25rem，
/// `windows/tauri/src/styles/theme.css:120-121`，用在 `windows/tauri/src/ui/tab-bar.tsx:248`
/// 的 `h-(--lithe-tab-bar-height)`）。
const TAB_BAR_HEIGHT: f32 = 36.;

/// 标签栏左右内边距 8px：`--lithe-chrome-padding-inline`（`windows/tauri/src/styles/theme.css:132`，
/// 用在 `windows/tauri/src/ui/tab-bar.tsx:248` 的 `px-(--lithe-chrome-padding-inline)`）。
const TAB_BAR_PADDING_INLINE: f32 = 8.;

/// 标签栏段间距 4px：`--lithe-chrome-gap`（`windows/tauri/src/styles/theme.css:130`，用在 `windows/tauri/src/ui/tab-bar.tsx:248`）。
///
/// `TabBar` 的 `Underline` 变体不给外层容器设 gap（`tab/tab_bar.rs:393-403` 返回的 gap
/// 只用在 `tabs-inner` 上），所以这里把 4px 挂在左侧导航组的右外边距上。
const TAB_BAR_GAP: f32 = 4.;

/// 单个标签宽度上限 200px：`--lithe-tab-max-width`（12.5rem，`windows/tauri/src/styles/theme.css:123`，
/// 用在 `windows/tauri/src/ui/tab-bar.tsx:260`）。**这是标签文字能截断的前提**。
const TAB_MAX_WIDTH: f32 = 200.;

/// 标签左内边距 8px（`pl-2`）与右内边距 24px（`pr-6`，给关闭按钮留位）：
/// `windows/tauri/src/ui/tab-bar.tsx:260`。
///
/// 右边的 24px 是**常驻**的（真机也一样：关闭按钮绝对定位在这块留白上），
/// 所以切换标签时标签宽度不会跳。
const TAB_PADDING_LEFT: f32 = 8.;
const TAB_PADDING_RIGHT: f32 = 24.;

/// 标签内图标↔文字间距 6px：`--lithe-chrome-gap-loose`（`windows/tauri/src/styles/theme.css:131`，
/// 用在 `windows/tauri/src/ui/tab-bar.tsx:170` 的 `gap-(--lithe-chrome-gap-loose)`）。
const TAB_ICON_GAP: f32 = 6.;

/// 标签图标槽 12×12px（`grid size-3`）：`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:183`。
const TAB_ICON_SIZE: f32 = 12.;

/// 标签栏左侧导航组内间距 2px（`gap-0.5`）：`windows/tauri/src/features/tabs/components/tab-bar.tsx:633`。
const NAV_GAP: f32 = 2.;

/// 关闭按钮距标签右边缘 4px（`absolute right-1`）：
/// `windows/tauri/src/features/tabs/components/tab-bar-item.tsx:164-166`。
/// 按钮自身是 24×24（真机的 Button `icon-xs` = `size-6`，
/// `windows/tauri/src/ui/button.tsx:27`），gpui-kit 侧用 `Sizable::small()`
/// 才是这个尺寸（`xsmall()` 给图标按钮只有 20×20，`button/button.rs:618-623`）。
const TAB_CLOSE_INSET: f32 = 4.;

/// 脏标记圆点 8×8px（`size-2 rounded-full bg-primary`）：
/// `windows/tauri/src/features/tabs/components/tab-bar-item.tsx:284-291`。
const DIRTY_DOT_SIZE: f32 = 8.;

/// 空状态容器内边距 24px / 32px（`px-6 py-8`）与内容块间距 12px（`gap-3`）：
/// `windows/tauri/src/features/panes/components/empty-editor-state.tsx:15-16`。
const EMPTY_PADDING_X: f32 = 24.;
const EMPTY_PADDING_Y: f32 = 32.;
const EMPTY_GAP: f32 = 12.;

/// 空状态内容块宽度上限 448px（`max-w-md`）：`windows/tauri/src/features/panes/components/empty-editor-state.tsx:16`。
const EMPTY_CONTENT_MAX_WIDTH: f32 = 448.;

/// 空状态图标块 48×48（`size-12`）、主图标 40×40（`size-10`）、
/// 右下角放大镜 20×20（`size-5`）：`windows/tauri/src/features/panes/components/empty-editor-state.tsx:17-22`。
const EMPTY_MEDIA_BOX: f32 = 48.;
const EMPTY_MEDIA_ICON: f32 = 40.;
const EMPTY_MEDIA_BADGE: f32 = 20.;

/// 空状态标题/说明字号 13px：`--ui-text-base` 与 `--ui-text-sm`
/// （`windows/tauri/src/styles/theme.css:116-117`，`windows/tauri/src/features/panes/components/empty-editor-state.tsx:24,27` 的 `ui-text-base` / `ui-text-sm`）。
const EMPTY_TEXT_SIZE: f32 = 13.;

/// 滚轮增量换算用的行高兜底值：真机默认行高就是 **20**
/// （`windows/tauri/src/features/editor/config/constants.ts:5` 的 `DEFAULT_LINE_HEIGHT: 20`；
/// 算法是 `ceil(fontSize × 1.4)`，`windows/tauri/src/features/editor/utils/lines.ts:8-15`）。
/// 编辑器还没完成首次布局时 `line_height()` 是 `None`，用这个值兜底，
/// 否则 `ScrollDelta::Lines` 换算出 0，整段滚不动。
const FALLBACK_LINE_HEIGHT: f32 = 20.;

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
fn icon_for_file(name: &str) -> IconName {
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
fn read_body(path: &Path, name: &str) -> String {
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
fn display_names(buffers: &[Buffer]) -> Vec<SharedString> {
    let segments: Vec<Vec<String>> = buffers
        .iter()
        .map(|buffer| path_segments(&buffer.path))
        .collect();
    let mut groups: Vec<(SharedString, Vec<usize>)> = Vec::new();
    let mut names: Vec<Option<SharedString>> = vec![None; buffers.len()];

    for (index, buffer) in buffers.iter().enumerate() {
        // 先算出分组下标再动 `groups`：把查找写进 `match` 的 scrutinee 会让
        // `iter()` 的借用活到整个 `match`，`None` 分支的 `push` 就借不动了。
        let existing = groups
            .iter()
            .position(|(name, _)| *name == buffer.name);
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
struct Buffer {
    /// 打开时用的路径，去重键（同一路径重复打开只切标签、不读盘）。
    path: PathBuf,
    /// 文件名，也是 [`display_names`] 分组的键。
    name: SharedString,
    /// 标签左侧的文件类型图标。
    icon: IconName,
    /// 正文状态：文件内容，读不到时是说明文案。
    editor: Entity<EditorState>,
    /// 未保存圆点。
    ///
    /// **本轮恒为 `false`**：没有订阅编辑事件，不去编造脏状态
    /// （`gpui/UI-MAP.md` §1「状态齐全」要求的是真实状态，不是假圆点）。
    /// 圆点的位置/尺寸/颜色已经按规格写在 [`EditorPane::render_tab`]，
    /// 接上 `InputEvent` 后把这里改成真实值即可。
    is_dirty: bool,
}

/// 编辑区视图：标签栏 + 正文（正文没有活动 buffer 时是空状态）。
pub struct EditorPane {
    /// 打开的 buffer，顺序就是标签栏里的顺序。
    buffers: Vec<Buffer>,
    /// 活动 buffer 在 [`Self::buffers`] 里的下标；`None` = 没有活动 buffer → 空状态。
    active: Option<usize>,
}

impl EditorPane {
    /// 建一个没有打开任何文件的编辑区。
    ///
    /// 真机启动时编辑区就是空状态（标签栏在、正文是空状态），
    /// 打开动作由用户触发（`panes/components/pane-container.tsx:1100`）。
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            buffers: Vec::new(),
            active: None,
        }
    }

    /// 打开一个文件：读盘、判定类型、更新标签栏与正文。
    ///
    /// **同一路径重复打开只切换活动标签、不再读盘**（沿用上一轮已验证实现的约定，
    /// git HEAD `panels.rs::open_document`）：正文已经在这个 [`Buffer`] 的
    /// `EditorState` 里，重读会白白丢掉撤销栈与光标。
    /// 注意"同路径"是按传入的 `Path` 字面比较；调用方要保证同一个文件每次传同一种写法
    /// （绝对路径就一直绝对路径）。
    ///
    /// 关闭标签会把该 buffer 的 `EditorState`（以及缓存正文）一起丢掉 —— 真机同样会释放
    /// 对应 model，所以"重复打开不读盘"只对**仍然打开着**的标签成立。
    pub fn open(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        let path = path.to_path_buf();

        if let Some(index) = self.buffers.iter().position(|buffer| buffer.path == path) {
            self.active = Some(index);
            cx.notify();
            return;
        }

        let name: SharedString = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string())
            .into();
        let body = read_body(&path, &name);

        // 一个标签一个 `EditorState`：先建状态再灌正文，然后才入列。
        let editor = cx.new(|cx| EditorState::new(window, cx));
        editor.update(cx, |state, cx| state.set_value(body, window, cx));

        self.buffers.push(Buffer {
            icon: icon_for_file(&name),
            name,
            path,
            editor,
            is_dirty: false,
        });
        self.active = Some(self.buffers.len() - 1);
        cx.notify();
    }

    /// 关闭一个标签。
    ///
    /// 关掉最后一个标签后编辑区回到空状态（`windows/tauri/src/features/panes/components/pane-container.tsx:1100`）；
    /// 关掉活动标签时顺位接上它的邻居（真机就是"关掉后激活相邻标签"）。
    fn close(&mut self, index: usize) {
        if index >= self.buffers.len() {
            return;
        }

        // 被关掉的标签之后的标签整体左移一位，所以旧下标大于 index 的要减一。
        let shift = |active: usize| if active > index { active - 1 } else { active };
        self.buffers.remove(index);

        self.active = if self.buffers.is_empty() {
            None
        } else {
            Some(
                self.active
                    .map(shift)
                    .unwrap_or(index)
                    .min(self.buffers.len() - 1),
            )
        };
    }

    /// 标签栏：左侧 `← →` 导航组 + 各文件的标签。
    ///
    /// 组件的选型与为什么这么用：
    ///
    /// - `TabVariant::Underline` 是 gpui-kit 里唯一"活动标签 = 透明底 + 底边主色条"的变体
    ///   （`tab/tab.rs:253-262`：`bg` 透明、`border_b` 2px `primary`），与 Windows 的
    ///   IntelliJ 风格一致（`windows/tauri/src/ui/tab-bar.tsx:204-209`：`bg-transparent` +
    ///   `before:h-[3px] before:bg-primary`）；
    /// - Underline 变体的标签条自带 1px 下边框（`tab/tab_bar.rs:501-514`，
    ///   `border_b_1()` + `theme.border`），正好是 Windows 的 `border-b border-border`；
    /// - Underline 变体的条底色是**透明**、外层 padding 是 0
    ///   （`tab/tab_bar.rs:393-403`），所以这里显式补 `.bg(theme.tab_bar)` 与
    ///   `.px(8.)`，把 Windows 的 `bg-tab-bar` + `px-(--lithe-chrome-padding-inline)` 找回来；
    /// - `.h(36.)` 是**必须**的：标签条自身没有高度，没有标签时会被压成 0；
    ///   真机里没有活动 buffer 时标签栏也照常占着 36px（`windows/tauri/src/features/panes/components/pane-container.tsx:1094-1100`）。
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let names = display_names(&self.buffers);

        let mut bar = TabBar::new("editor-tab-bar")
            .with_variant(TabVariant::Underline)
            .bg(cx.theme().tab_bar)
            .h(px(TAB_BAR_HEIGHT))
            .px(px(TAB_BAR_PADDING_INLINE))
            // `max_width` 让**标签文字**在空间不够时让位（图标与关闭按钮保持原尺寸），
            // 也就是真机那种 `OrderChargeService.j…` 的截断。
            .max_width(px(TAB_MAX_WIDTH))
            .prefix(Self::render_nav_group())
            .on_click(cx.listener(|pane, index: &usize, _window, cx| {
                // `TabBar::on_click` 给的是被点标签的下标
                // （`tab/tab_bar.rs:168-177`），切换活动 buffer 就是切标签。
                if *index < pane.buffers.len() {
                    pane.active = Some(*index);
                    cx.notify();
                }
            }));

        for (index, (buffer, name)) in self.buffers.iter().zip(names).enumerate() {
            bar = bar.child(self.render_tab(index, buffer, name, cx));
        }

        if let Some(active) = self.active {
            bar = bar.selected_index(active);
        }

        bar.into_any_element()
    }

    /// 标签栏左侧的后退/前进按钮组。
    ///
    /// 真机是一个 `h-8` 的行（`windows/tauri/src/features/tabs/components/tab-bar.tsx:633-660`），
    /// 与后面的标签区之间有标签栏自己的 4px gap；`TabBar` 的 `Underline` 变体不给
    /// 外层容器设 gap，所以这里用右外边距补上同样的 4px。
    fn render_nav_group() -> impl IntoElement {
        h_flex()
            .items_center()
            .gap(px(NAV_GAP))
            .mr(px(TAB_BAR_GAP))
            // 文案取中文 i18n 原文：tooltip = `tabs.goBackShort` / `tabs.goForwardShort`
            // （"后退" / "前进"，`windows/tauri/src/i18n/locale.ts:7854-7855`），
            // 无障碍名 = `tabs.goBack` / `tabs.goForward`
            // （"后退到上一个位置" / "前进到下一个位置"，`windows/tauri/src/i18n/locale.ts:7850-7851`）。
            .child(Self::nav_button(
                "editor-nav-back",
                IconName::ArrowLeft,
                "后退",
                "后退到上一个位置",
            ))
            .child(Self::nav_button(
                "editor-nav-forward",
                IconName::ArrowRight,
                "前进",
                "前进到下一个位置",
            ))
    }

    /// 单个导航按钮。
    ///
    /// 真机：`variant="ghost" size="icon-xs"`、`disabled={!canGoBack}`、
    /// tooltip `tooltipSide="bottom"`（`windows/tauri/src/features/tabs/components/tab-bar.tsx:634-659`）。
    ///
    /// ⚠️ **尺寸陷阱**：真机的 `icon-xs` 是 **24×24**（`windows/tauri/src/ui/button.tsx:27`
    /// 的 `icon-xs size-6`），而 gpui-kit 的 `Sizable::xsmall()` 给图标按钮是 **20×20**、
    /// `small()` 才是 24×24（`button/button.rs:618-623`）。这里对齐的是**尺寸值**，
    /// 所以用 `.small()`，不要被变体名带偏。
    ///
    /// ⚠️ **本轮只有外观 + 禁用态，没有历史栈**：探针没有 jump list，
    /// 所以两个按钮恒为 `disabled(true)`（禁用态 ghost = 灰图标，`button/button.rs:1273-1306`），
    /// 而不是"点了没反应"的假按钮。接上历史栈后改成 `disabled(!can_go_back)`
    /// 并在 `on_click` 里跳转即可。
    fn nav_button(
        id: &'static str,
        icon: IconName,
        tooltip: &'static str,
        label: &'static str,
    ) -> Button {
        Button::new(id)
            .icon(icon)
            .ghost()
            .small()
            .disabled(true)
            .tab_stop(false)
            .tooltip(tooltip)
            .accessibility_label(label)
    }

    /// 一个标签：文件类型图标 + 显示名（+ 未保存圆点）+ 关闭按钮。
    ///
    /// ⚠️ **图标 + 文件名必须走 `Tab::child(...)`**：`Tab::icon()` 只渲染图标，
    /// label 与 children 在图标分支里被整个丢掉（`tab/tab.rs:719-744`，
    /// 图标分支没有 `.children(self.children)`）。
    ///
    /// 颜色照 Windows（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:276`：活动 `--foreground`、非活动
    /// `--subtle-foreground`；`:250`：图标恒为 `--subtle-foreground`）。
    /// 这里必须**自己给文字设色**：`TabBar` 会把标签的 `text_color` 换成自己的
    /// `tab_foreground`/`tab_active_foreground`（`tab/tab.rs:253-264`），
    /// 而 Windows 的非活动标签要的是更暗的 `subtle-foreground`。
    ///
    /// 关闭按钮用**绝对定位**放在标签里，不走 `Tab::suffix`：
    /// suffix 是 flex 项，会算进标签宽度，切换标签时标签宽度会跳；
    /// 绝对定位后宽度只由 `pr(24px)` 决定，与真机一致（`right-1` + `pr-6`）。
    fn render_tab(
        &self,
        index: usize,
        buffer: &Buffer,
        name: SharedString,
        cx: &mut Context<Self>,
    ) -> Tab {
        let is_active = self.active == Some(index);
        let label_color = if is_active {
            cx.theme().foreground
        } else {
            cx.theme().muted_foreground
        };

        // 无障碍名照真机拼「名称 +（未保存）」（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:137-141`、
        // `windows/tauri/src/i18n/locale.ts:7860` 的 `tabs.ariaUnsavedSuffix`）；
        // 固定/预览后缀本轮没有这两种状态。必须在 `name` 被移进正文之前算好。
        let aria_label = if buffer.is_dirty {
            SharedString::from(format!("{name}（未保存）"))
        } else {
            name.clone()
        };

        // 未保存圆点：8×8、`rounded-full`、`primary`（`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:284-291`）。
        // `is_dirty` 本轮恒为 false，所以画不出来；位置留在这里，
        // 接上编辑事件后把开关改成真实值即可。
        let dirty_dot = buffer.is_dirty.then(|| {
            div()
                .flex_shrink_0()
                .w(px(DIRTY_DOT_SIZE))
                .h(px(DIRTY_DOT_SIZE))
                .rounded_full()
                .bg(cx.theme().primary)
        });

        // 关闭按钮：只有活动标签显示。真机默认设置是
        // `tabCloseButtonVisibility: "active"`（`windows/tauri/src/features/settings/config/default-settings.ts:96`），
        // 规则见 `windows/tauri/src/features/settings/lib/ui-preferences.ts:13-19`
        // （固定标签恒显 / `always` 全显 / `active` 只有活动标签 / 否则悬停才显）。
        // 悬停那一档没做：`TabBar` 不暴露每个标签的悬停状态，见报告的"未能实现"一节。
        //
        // 绝对定位而不是 `Tab::suffix`：suffix 是 flex 项，会算进标签宽度，
        // 切换标签时标签宽度会跳；绝对定位后宽度只由正文常驻的 `pr(24px)` 决定，
        // 与真机一致（关掉按钮绝对定位在 `right-1` 上，见 `windows/tauri/src/features/tabs/components/tab-bar-item.tsx:164-166`）。
        let close = is_active.then(|| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .right(px(TAB_CLOSE_INSET))
                .flex()
                .items_center()
                .child(Self::close_button(index, cx))
        });

        let content = h_flex()
            .items_center()
            .pl(px(TAB_PADDING_LEFT))
            .pr(px(TAB_PADDING_RIGHT))
            .gap(px(TAB_ICON_GAP))
            // 没有 `min_w_0` 的话 flex 项的最小尺寸会按内容算，文字截断不了。
            .min_w_0()
            .child(
                // 全量目录的 `IconName` 是 `Copy`（`gpui-kit-assets-0.6.6/build.rs:52-53`
                // 的 `#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, IntoElement)]`），
                // 从借用里直接取出即可。
                Icon::new(buffer.icon)
                    .w(px(TAB_ICON_SIZE))
                    .h(px(TAB_ICON_SIZE))
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(label_color)
                    .child(name),
            )
            .children(dirty_dot)
            .children(close);

        Tab::new().aria_label(aria_label).child(content)
    }

    /// 标签上的关闭按钮。
    ///
    /// 真机：ghost 图标按钮 `icon-xs`（24×24），tooltip `tabs.close`（"关闭"，
    /// `windows/tauri/src/i18n/locale.ts:7845`），点击关闭该标签
    /// （`windows/tauri/src/features/tabs/components/tab-bar-item.tsx:150-180`）。
    /// gpui-kit 的 `Button` 点击时会 `stop_propagation`（`button/button.rs:797-808`），
    /// 所以点关闭不会顺带把标签激活。
    /// 尺寸用 `.small()` 的理由（gpui-kit 的 `small()` = 24×24、`xsmall()` = 20×20）
    /// 见 [`TAB_CLOSE_INSET`] 的注释。
    fn close_button(index: usize, cx: &mut Context<Self>) -> Button {
        Button::new(format!("editor-tab-close-{index}"))
            // Windows 的关闭字形是 lucide `x`，`icons/x.svg` 在全量目录里确有该字形。
            .icon(IconName::X)
            .ghost()
            .small()
            .tab_stop(false)
            .tooltip("关闭")
            .accessibility_label("关闭")
            .on_click(cx.listener(move |pane, _event, _window, cx| {
                pane.close(index);
                cx.notify();
            }))
    }

    /// 正文区：有活动 buffer 时是代码编辑器，没有时是空状态。
    fn render_body(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(buffer) = self.active.and_then(|index| self.buffers.get(index)) else {
            return Self::render_empty_state(cx);
        };
        // 渲染闭包要 `'static`，先把活动编辑器的句柄取出来（`&Buffer` 借用到此结束）。
        // 两份：滚轮闭包会把它 move 走，正文还要再渲染一次同一个编辑器。
        let editor = buffer.editor.clone();
        let scroll_editor = editor.clone();

        div()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            // ⚠️ **滚轮必须自己接**：编辑器元素的样式只有 `position: absolute` + `100%`，
            // **没有** `overflow: scroll`（`gpui-base-0.6.6/src/input/base/element.rs:202-213`），
            // 而 gpui 只把滚轮交给"命中元素样式里带 `Overflow::Scroll`"的那个
            // （`gpui-pre-0.3.6/src/elements/div.rs:3332-3370`）。
            // 做法是在包裹层接，把增量交给编辑器自己的滚动偏移
            // （`InputBaseState::{scroll_offset, set_scroll_offset}`，
            // `gpui-base-0.6.6/src/input/base/state.rs:2806-2817`；后者会自己 clamp、
            // 下一帧生效）。
            .on_scroll_wheel(cx.listener(
                move |_pane, event: &ScrollWheelEvent, _window, cx| {
                    let line_height = scroll_editor
                        .read(cx)
                        .line_height()
                        .unwrap_or(px(FALLBACK_LINE_HEIGHT));
                    let delta = event.delta.pixel_delta(line_height).y;
                    if delta == px(0.) {
                        return;
                    }
                    let current = scroll_editor.read(cx).scroll_offset();
                    scroll_editor.update(cx, |state, cx| {
                        state.set_scroll_offset(point(current.x, current.y + delta), cx);
                    });
                },
            ))
            // ⚠️ **多行编辑器必须显式给高度**：多行走 `.h_auto()`，高度 = **内容高度**，
            // 而内容高度来自 `LayoutMode::CodeEditor { rows }`，`rows` 默认只有 2
            // （`gpui-component-0.6.6/src/input/input.rs:706-709`、
            // `gpui-base-0.6.6/src/input/base/mode.rs:83-85`）—— 不给高度时编辑区只有两行高、
            // 下面整片空白。`relative(1.)` 让它填满外层这一格
            // （等价于组件自带的 `Input::full_height()`，`input.rs:250-252`）。
            //
            // `.bordered(false)`：真机的编辑面是**无边框**的纯表面
            // （`windows/tauri/src/features/editor/components/code-editor.tsx:650,731`：
            // `absolute inset-0 bg-background`，没有 border），而 `Editor` 默认
            // `bordered: true`（`gpui-component-0.6.6/src/input/editor.rs:46`）会画出
            // 输入框那种 1px `theme.input` 描边（`input/input.rs:713-715`）。
            // 焦点环不用管：`Editor` 已经把 `focus_bordered(false)` 写死了
            // （`input/editor.rs:146`）。
            .child(Editor::new(&editor).h(relative(1.)).bordered(false))
            .into_any_element()
    }

    /// 没有活动 buffer 时的空状态。
    ///
    /// 结构照 `windows/tauri/src/features/panes/components/empty-editor-state.tsx`：
    /// 居中一列 = 图标块（48×48 的 `FileText` 40×40 + 右下角 20×20 放大镜）
    /// + 标题 + 说明（`:15-30`）。文案取**中文 i18n 原文，逐字**：
    ///
    /// - 标题 `workbench.emptyEditorTitle` = "选择文件以查看"（`windows/tauri/src/i18n/locale.ts:6150`）；
    /// - 说明 `workbench.emptyEditorDescription` = "外部工具产生的更改会自动显示。"
    ///   （`windows/tauri/src/i18n/locale.ts:6151`）；
    /// - 右键菜单只有一项禁用项 `ui.noActionsHere` = "此处无任何内容"
    ///   （`windows/tauri/src/i18n/locale.ts:4630`；空态菜单在
    ///   `windows/tauri/src/features/panes/components/empty-editor-state.tsx:32-34`）。
    fn render_empty_state(cx: &App) -> AnyElement {
        // 外层用 `v_flex` 而不是裸 `div`：`Empty` 自己带 `flex_1`，
        // 但"flex 项"只在 flex 容器里才成立 —— 裸 div 里它会塌成内容高度，
        // 垂直居中就没了。
        v_flex()
            .flex_1()
            .min_h_0()
            .context_menu(|menu, _window, _cx| {
                menu.item(PopupMenuItem::new("此处无任何内容").disabled(true))
            })
            .child(
                Empty::new()
                    // ⚠️ 真机空状态**没有边框**；`Empty` 的 render 里硬编码了
                    // `.border_dashed().border_color(cx.theme().border)`
                    // （`gpui-component-0.6.6/src/empty.rs:74-75`），
                    // 所以把边框色改成透明来关掉那圈虚线。
                    .border_color(cx.theme().transparent)
                    .gap(px(EMPTY_GAP))
                    .px(px(EMPTY_PADDING_X))
                    .py(px(EMPTY_PADDING_Y))
                    .header(
                        EmptyHeader::new()
                            .max_w(px(EMPTY_CONTENT_MAX_WIDTH))
                            .gap(px(EMPTY_GAP))
                            .media(
                                EmptyMedia::new().child(
                                    div()
                                        .relative()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .w(px(EMPTY_MEDIA_BOX))
                                        .h(px(EMPTY_MEDIA_BOX))
                                        .child(
                                            Icon::new(IconName::FileText)
                                                .w(px(EMPTY_MEDIA_ICON))
                                                .h(px(EMPTY_MEDIA_ICON))
                                                .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(
                                            Icon::new(IconName::Search)
                                                .absolute()
                                                .right_0()
                                                .bottom_0()
                                                .w(px(EMPTY_MEDIA_BADGE))
                                                .h(px(EMPTY_MEDIA_BADGE))
                                                .text_color(cx.theme().muted_foreground),
                                        ),
                                ),
                            )
                            // 字号 13px：`ui-text-base` / `ui-text-sm`
                            // （`windows/tauri/src/styles/theme.css:116-117`）；组件默认的 `text_sm` 是 14px，
                            // 所以显式覆盖成规格值。颜色用组件默认：
                            // 标题 `foreground`、说明 `muted_foreground`（`empty.rs:264-324`）。
                            .title(
                                EmptyTitle::new()
                                    .text_size(px(EMPTY_TEXT_SIZE))
                                    .child("选择文件以查看"),
                            )
                            .description(
                                EmptyDescription::new()
                                    .text_size(px(EMPTY_TEXT_SIZE))
                                    .child("外部工具产生的更改会自动显示。"),
                            ),
                    ),
            )
            .into_any_element()
    }
}

impl Render for EditorPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 两层：标签栏在上，正文占剩下的高度。容器要 `min_h_0` + `overflow_hidden`，
        // 否则正文里的编辑器会把整个视图撑出父容器。
        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .child(self.render_tab_bar(cx))
            .child(self.render_body(cx))
    }
}
