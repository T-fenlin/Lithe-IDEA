//! 悬停文档（U20）：把 `textDocument/hover` 接到上游编辑器**自带的**悬停浮层上。
//!
//! ## 上游有没有入口：**有**，而且浮层、触发时机、防抖全是现成的
//!
//! | 部件 | 位置（`gpui-base-0.7.0/` 与 `gpui-component-0.7.0/`） | 说明 |
//! | --- | --- | --- |
//! | trait | `src/input/editor/lsp/hover.rs:11-22` | `HoverProvider::hover(&self, text, offset, window, cx) -> Task<Result<Option<lsp_types::Hover>>>` |
//! | 挂载点 | `src/input/editor/lsp/mod.rs:45` | `Lsp.hover_provider: Option<Rc<dyn HoverProvider>>`，经 `EditorState::lsp_mut()` 赋值（同文件 `:112` 一带） |
//! | 触发 | `src/input/editor/lsp/hover.rs:88-110` | 上游自己在 `handle_mouse_move` 里调：**非** Alt、**非** Ctrl/Cmd 的普通悬停，且**先等 150ms**（`:53-57`），鼠标扫过去不会发请求 |
//! | 浮层 | `src/input/popovers/hover_popover.rs:39-65` | `HoverPopover` 渲染 `hover.contents`（Markdown 走 `render_markdown`）；装载点在 `gpui-component/src/input/overlay.rs:176,253` —— 宿主**不需要**自绘任何东西 |
//! | 清空 | `src/input/editor/lsp/hover.rs:112-120` | 指针离开 / 失焦 / 文本整体替换都会清 `hover_popover` |
//!
//! ⚠️ 所以本模块**只提供数据**：一行渲染代码都没有（与 [`crate::code_actions`] 的处境相反
//! —— 那个要自己落地 `edit`，上游完全不处理）。
//!
//! ## 两条数据源，归一化形状**逐字段相同**
//!
//! | 数据源 | 条件 | Core 命令 | 内容 |
//! | --- | --- | --- | --- |
//! | JDTLS（语义） | `.java` 文件且语言服务已起 | `lsp.request{operation:"hover"}` | javadoc（Markdown）+ 符号范围 |
//! | Core 轻量悬停 | **其它一切情况**（任何文件、服务没起、请求失败） | `lsp.builtinHover` | 光标处标识符本身（`` `launch` ``） |
//!
//! 两条路的信封里都是同一个 `hover` 结构（`{ contents, isMarkdown, range }`，
//! 见 `rust/lithe-core/src/lsp/interface/client.rs:1067-1078` 与
//! `rust/lithe-core/src/lsp/lightweight/symbols.rs:52-63`），所以
//! [`lithe_gpui_java::parse_hover`]（`pub(crate)`，测试同源）**一份解析两条路都用**。
//!
//! 轻量那一档不是凑数：它让"悬停"这个功能在**任何**文件上都成立（不装 JDK、不启进程），
//! 与补全的 `lsp.builtinCompletions` / 跳转的 `lsp.builtinNavigation` 是同一条产品口径
//! （`java/src/service.rs` 的模块文档："起不来当作能力的降级，而不是错误状态"）。
//!
//! ## 失败为什么**外抛 0 次**
//!
//! 上游对 `Err` 的处置是**静默**（`hover.rs:66-79` 只看 `Ok`/`None` 两种，`?` 直接把错误
//! 变成"这次不弹"）。所以本模块不把"服务不可用 / 超时 / 请求失败"做成错误：一律
//! **回落到轻量悬停**，失败原因走 stderr 的 `S1_EDITOR_HOVER`（`source=` 直接指明这一次
//! 的数据来自哪一路）—— 否则"悬停没反应"与"悬停给的是标识符"这两件事在日志里分不开。
//!
//! ## 列口径：Core 的 UTF-16 列**必须**换算后才交给上游
//!
//! 上游拿到 `Hover.range` 之后走 `editor.text.position_to_offset(&range.start)`
//! （`hover.rs:68-70`），而那个函数把 `Position.character` 当**字符列**（`rope_ext.rs` 的
//! `char_to_byte_idx`）。Core 给的是 UTF-16 码元列 —— 两套列只在"行内没有非 BMP 字符"时相等。
//! 所以这里用 `crate::navigation::editor_position`（那三套列的唯一换算点）转一次，
//! 而不是把 Core 的列原样塞进 `Position`（那样 emoji 行上的高亮会偏）。

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use gpui_kit::component::input::{EditorState, HoverProvider, Rope};
use gpui_kit::{App, AppContext as _, Task, Window};
use lithe_gpui_java::{JavaHover, JavaLanguageService};
use lithe_gpui_shared::core_json;
use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use serde_json::json;

/// 把悬停 provider 装到一个 `EditorState` 上（`None` 时什么都不做）。
///
/// 由 [`crate::editor_view`] 在两个位置调用：`open()` / `open_virtual()`（新开的 buffer）
/// 与 `prepare_java()`（服务就绪后给已经打开的 Java buffer **补装**一次）。
///
/// 与 [`crate::code_actions::install`] 的差别：那边"没拿到服务句柄就干脆不装"（它没有兜底
/// 数据源，装一个永远为空的 provider 会把右键菜单那一项变成"可用但没反应"）；悬停**永远**
/// 有数据源（Core 的轻量悬停不需要任何进程），所以调用方总是给 `Some` —— 也就没有
/// "没装 provider 时空悬停一下什么都不发生"这种不好解释的状态。
pub(crate) fn install(state: &mut EditorState, provider: Option<Rc<dyn HoverProvider>>) {
    let Some(provider) = provider else {
        return;
    };
    state.lsp_mut().hover_provider = Some(provider);
}

/// 悬停的数据源：优先 JDTLS 语义结果，失败 / 不适用时回落到 Core 的轻量悬停。
pub(crate) struct LitheHoverProvider {
    /// Java 语言服务（`Arc`：请求要带进后台任务）。
    ///
    /// `None` 有两种情况，**都走轻量悬停**：①这不是 Java 文件（服务只认 `.java`，
    /// 见 `JavaLanguageService::hover` 里写死的 `"java"` 语言名）；②外壳还没起服务。
    java: Option<Arc<JavaLanguageService>>,
    /// 这个 buffer 对应的磁盘路径（`lsp.builtinHover` 的 `filePath` 字段）。
    file_path: PathBuf,
}

impl LitheHoverProvider {
    /// 建一个 provider。`java` 由调用方判定（只有 `.java` 文件才该给服务句柄）。
    pub(crate) fn new(
        java: Option<Arc<JavaLanguageService>>,
        file_path: PathBuf,
    ) -> Rc<dyn HoverProvider> {
        Rc::new(Self { java, file_path })
    }
}

impl HoverProvider for LitheHoverProvider {
    /// `textDocument/hover`（或 Core 的轻量兜底）。
    ///
    /// `offset` 是**字节**偏移（上游 `handle_mouse_move` 从命中点算出来的），换算成
    /// Core 的 `(0 基行, UTF-16 列)` 只有 [`crate::navigation::core_position`] 一条路
    /// （三套列的坑见 `crate::navigation` 的模块文档）。
    ///
    /// ⚠️ **先换算、再进后台任务**：`&Rope` 不是 `'static`，move 不进任务；而
    /// `core_position` 只需要读 rope 一次（O(log n)），所以这一步留在调用线程上是对的，
    /// 进任务的是一份拥有所有权的正文快照。
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui_kit::Result<Option<Hover>>> {
        let (line, utf16_column) = crate::navigation::core_position(text, offset);
        let body = text.to_string();
        let file_path = self.file_path.clone();
        let java = self.java.clone();

        cx.background_spawn(async move {
            let file = file_path.display().to_string();
            let started = Instant::now();

            // ① 语言服务那条路（只有 .java 且服务句柄在时才走）。
            if let Some(service) = java {
                match service.hover(&file_path, &body, line as u32, utf16_column as u32) {
                    Ok(Some(hover)) => {
                        println!(
                            "S1_EDITOR_HOVER source=jdtls file={file} line={} col={} markdown={} chars={} ms={}",
                            line + 1,
                            utf16_column + 1,
                            hover.is_markdown,
                            hover.contents.len(),
                            started.elapsed().as_millis()
                        );
                        // 服务端给了符号范围就换算一次（理由见模块文档的"列口径"一节）；
                        // 没给就让上游按"光标下那个词"锚定（`hover.rs:47` 的 `word_range`）。
                        let range = hover.range.and_then(|(start, end)| {
                            let rope = Rope::from(body.as_str());
                            Some(Range {
                                start: crate::navigation::editor_position(
                                    &rope,
                                    start.line as usize,
                                    start.utf16_column as usize,
                                ),
                                end: crate::navigation::editor_position(
                                    &rope,
                                    end.line as usize,
                                    end.utf16_column as usize,
                                ),
                            })
                        });
                        return Ok(Some(to_lsp_hover(&hover, range)));
                    }
                    // 服务端答了"这里没有可悬停的东西"（空白处 / 关键字）：**不是失败**，
                    // 但也不该再问一次轻量悬停 —— 那会给出"标识符"这种服务端明确否认过的
                    // 结果。这一档就是"不弹浮层"。
                    Ok(None) => {
                        println!(
                            "S1_EDITOR_HOVER source=jdtls-empty file={file} line={} col={} ms={}",
                            line + 1,
                            utf16_column + 1,
                            started.elapsed().as_millis()
                        );
                        return Ok(None);
                    }
                    // 服务不可用 / 超时：回落到轻量悬停（理由见模块文档）。
                    Err(error) => eprintln!(
                        "S1_EDITOR_HOVER source=jdtls-failed file={file} line={} col={} error={error}",
                        line + 1,
                        utf16_column + 1
                    ),
                }
            }

            // ② Core 的轻量悬停：不需要任何进程，任何文件都有。
            match core_json(
                "lsp.builtinHover",
                json!({
                    "filePath": file_path.display().to_string(),
                    "text": body,
                    "position": { "line": line, "utf16Column": utf16_column },
                }),
            ) {
                Ok(Some(response)) => match response.get("hover").and_then(lithe_gpui_java::parse_hover)
                {
                    Some(hover) => {
                        println!(
                            "S1_EDITOR_HOVER source=builtin file={file} line={} col={} chars={} ms={}",
                            line + 1,
                            utf16_column + 1,
                            hover.contents.len(),
                            started.elapsed().as_millis()
                        );
                        // 轻量悬停不给 range（它的 `range` 就是标识符本身，与上游
                        // `word_range` 的锚点重合），所以这里恒为 `None`。
                        return Ok(Some(to_lsp_hover(&hover, None)));
                    }
                    None => {
                        println!(
                            "S1_EDITOR_HOVER source=builtin-empty file={file} line={} col={} ms={}",
                            line + 1,
                            utf16_column + 1,
                            started.elapsed().as_millis()
                        );
                        return Ok(None);
                    }
                },
                Ok(None) => {
                    eprintln!("S1_EDITOR_HOVER source=builtin-empty-envelope file={file}");
                }
                Err(error) => {
                    eprintln!("S1_EDITOR_HOVER source=builtin-failed file={file} error={error}");
                }
            }
            Ok(None)
        })
    }
}

/// Core 归一化后的悬停 → 上游要的 `lsp_types::Hover`。
///
/// `isMarkdown` 走 `HoverContents::Markup`（上游 `hover_popover.rs:54` 取 `markup.value`
/// 交给 `render_markdown`）；纯文本包成 `MarkupKind::PlainText`（同一个渲染器，`PlainText`
/// 不会被当 Markdown 解析）。
///
/// ⚠️ **不用 `MarkedString`**：`lsp_types` 里那个变体是 deprecated（上游 `hover_popover.rs:42-53`
/// 仍然支持它只是为兼容老服务端），Core 也从来没给我们那一档。
fn to_lsp_hover(hover: &JavaHover, range: Option<Range>) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: if hover.is_markdown {
                MarkupKind::Markdown
            } else {
                MarkupKind::PlainText
            },
            value: hover.contents.clone(),
        }),
        range,
    }
}

/// 给单元测试用的一层薄封装：直接喂 Core 归一化后的 `hover` 值，断言上游形状。
///
/// 单独抽出来是因为 [`LitheHoverProvider::hover`] 需要一个真的 `App` + `Window` +
/// 后台执行器才跑得起来，而"Core 的值 → 上游的 `Hover`"这一步是**纯函数**：
/// 补全 / 快速修复两个模块也都是把纯的那一段单独测（见 `crate::completion` 的测试）。
#[cfg(test)]
pub(crate) fn lsp_hover_from_core(value: &serde_json::Value) -> Option<Hover> {
    let hover = lithe_gpui_java::parse_hover(value)?;
    Some(to_lsp_hover(&hover, None))
}

#[cfg(test)]
mod tests {
    use super::lsp_hover_from_core;
    use lsp_types::{HoverContents, MarkupKind};
    use serde_json::json;

    /// Markdown 内容必须走 `Markup(Markdown)`：上游只对 `Markup` 调 `render_markdown`
    /// （`gpui-component-0.7.0/src/input/popovers/hover_popover.rs:54-61`），落到
    /// `MarkedString` 那一档会按纯文本原样画出来（javadoc 的 `**粗体**` 会露出来）。
    #[test]
    fn markdown_hover_becomes_a_markdown_markup() {
        let hover = lsp_hover_from_core(&json!({
            "contents": "**greet** — 打招呼",
            "isMarkdown": true,
            "range": {
                "start": { "line": 1, "utf16Column": 5 },
                "end": { "line": 1, "utf16Column": 9 }
            }
        }))
        .expect("带 contents 的悬停必须能转成上游形状");
        assert_eq!(
            hover.contents,
            HoverContents::Markup(lsp_types::MarkupContent {
                kind: MarkupKind::Markdown,
                value: "**greet** — 打招呼".to_string(),
            })
        );
        // 纯函数这一层**不**填 range（换算要 rope，归 provider 里做）：显式钉住，
        // 免得后人以为这里漏了。
        assert_eq!(hover.range, None);
    }

    /// 轻量悬停给的是纯文本（`` `launch` ``）：走 `PlainText`，不按 Markdown 解析。
    #[test]
    fn plain_hover_becomes_plain_text_markup() {
        let hover = lsp_hover_from_core(&json!({
            "contents": "`launch`",
            "isMarkdown": false,
            "range": null
        }))
        .expect("轻量悬停也必须能转");
        match hover.contents {
            HoverContents::Markup(markup) => {
                assert_eq!(markup.kind, MarkupKind::PlainText);
                assert_eq!(markup.value, "`launch`");
            }
            other => panic!("必须是 Markup，得到 {other:?}"),
        }
    }

    /// 反证：没有内容 / 内容不是字符串 → `None`（**不**返回一个空浮层）。
    ///
    /// 这一条守的是"不假装能用"：上游对 `Some(Hover { contents: "" })` 会照常弹一个空框，
    /// 那比不弹更让人以为功能坏了。
    #[test]
    fn empty_or_malformed_hover_is_dropped() {
        assert_eq!(lsp_hover_from_core(&json!({ "contents": "" })), None);
        assert_eq!(lsp_hover_from_core(&json!({ "contents": 7 })), None);
        assert_eq!(lsp_hover_from_core(&json!({ "hover": null })), None);
    }
}
