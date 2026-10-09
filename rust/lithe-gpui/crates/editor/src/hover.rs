//! 悬停内容：向 JDTLS 取数，再交给组件库去画。
//!
//! 分工是本 crate 里最"反过来的"一处：**库只画**（`CodeEditor::present_hover` 挂在指针停下的
//! 那个符号上、`present_hover_at` 挂在调用方点名的偏移上），**取数、切分、什么时候收起来**
//! 都归宿主——库在正文变化后不会自己清掉旧内容，所以正文一变这里就把它收掉（见
//! `EditorPane::clear_hover`）。
//!
//! 请求按 LSP 口径报（行 0 基、列按 UTF-16 码元），换算是 `crate::navigation::core_position`。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use gpui::{App, AppContext as _, ElementId, SharedString, Task};
use lithe_gpui_java::JavaLanguageService;
use lithe_gpui_shared::core_json;
use lithe_ui::editor::HoverInfo;
use serde_json::json;

use crate::navigation::core_position;

/// 一次悬停的正文，已经切成"签名"与说明段落两半。
pub(crate) struct Content {
    signature: SharedString,
    docs: Vec<SharedString>,
}

impl Content {
    /// 服务端的悬停正文 → 签名 + 说明。
    ///
    /// 组件库的画法就是这两样（一行代码签名 + 若干段说明），所以取第一段非空行当签名、
    /// 其余合成说明，**内容一个字都不丢**。整份是空白的悬停当作"没有内容"。
    fn of(contents: &str) -> Option<Self> {
        let mut lines = contents
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.trim().is_empty());
        let signature = lines.next()?.to_string();
        let rest = lines.collect::<Vec<_>>().join("\n");
        Some(Self {
            signature: signature.into(),
            docs: (!rest.is_empty())
                .then(|| rest.into())
                .into_iter()
                .collect(),
        })
    }
}

/// Asks for the hover at a byte offset; `None` when the tools have nothing to say there.
pub(crate) fn request(
    java: Option<Arc<JavaLanguageService>>,
    file_path: PathBuf,
    text: String,
    offset: usize,
    cx: &App,
) -> Task<Option<Content>> {
    let (line, utf16_column) = core_position(&text, offset);
    cx.background_spawn(async move { fetch(java, file_path, text, line, utf16_column).await })
}

async fn fetch(
    java: Option<Arc<JavaLanguageService>>,
    file_path: PathBuf,
    body: String,
    line: u64,
    utf16_column: usize,
) -> Option<Content> {
    let file = file_path.display().to_string();
    let started = Instant::now();

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
                return Content::of(&hover.contents);
            }
            Ok(None) => {
                println!(
                    "S1_EDITOR_HOVER source=jdtls-empty file={file} line={} col={} ms={}",
                    line + 1,
                    utf16_column + 1,
                    started.elapsed().as_millis()
                );
                return None;
            }
            Err(error) => eprintln!(
                "S1_EDITOR_HOVER source=jdtls-failed file={file} line={} col={} error={error}",
                line + 1,
                utf16_column + 1
            ),
        }
    }

    match core_json(
        "lsp.builtinHover",
        json!({
            "filePath": file_path.display().to_string(),
            "text": body,
            "position": { "line": line, "utf16Column": utf16_column },
        }),
    ) {
        Ok(Some(response)) => match response.get("hover").and_then(lithe_gpui_java::parse_hover) {
            Some(hover) => {
                println!(
                    "S1_EDITOR_HOVER source=builtin file={file} line={} col={} chars={} ms={}",
                    line + 1,
                    utf16_column + 1,
                    hover.contents.len(),
                    started.elapsed().as_millis()
                );
                Content::of(&hover.contents)
            }
            None => {
                println!(
                    "S1_EDITOR_HOVER source=builtin-empty file={file} line={} col={} ms={}",
                    line + 1,
                    utf16_column + 1,
                    started.elapsed().as_millis()
                );
                None
            }
        },
        Ok(None) => {
            eprintln!("S1_EDITOR_HOVER source=builtin-empty-envelope file={file}");
            None
        }
        Err(error) => {
            eprintln!("S1_EDITOR_HOVER source=builtin-failed file={file} error={error}");
            None
        }
    }
}

/// 取回来的正文包成组件库要的形状；`id` 由调用方给（它知道这是哪一个编辑器、第几次内容）。
pub(crate) fn info(content: Content, id: impl Into<ElementId>) -> HoverInfo {
    let mut info = HoverInfo::new(id, content.signature);
    for paragraph in content.docs {
        info = info.docs(paragraph);
    }
    info
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_hover_becomes_a_signature_and_its_docs() {
        let content = Content::of("**greet** — 打招呼\n\n参数：`name`").expect("有内容");
        assert_eq!(content.signature, "**greet** — 打招呼");
        assert_eq!(content.docs.len(), 1);
        assert_eq!(content.docs[0], "参数：`name`");
    }

    #[test]
    fn a_single_line_hover_has_no_docs() {
        let content = Content::of("`launch`").expect("轻量悬停");
        assert_eq!(content.signature, "`launch`");
        assert!(content.docs.is_empty(), "只有签名时不该多出一段空说明");
    }

    #[test]
    fn blank_or_empty_hover_is_dropped() {
        assert!(Content::of("").is_none());
        assert!(Content::of("   \n\n  ").is_none(), "全是空白等于没有内容");
    }

    #[test]
    fn leading_blank_lines_do_not_become_the_signature() {
        let content = Content::of("\n\nint value()").expect("第二行才是签名");
        assert_eq!(content.signature, "int value()");
    }

    /// 取数与切分都不碰 LSP 的形状：解析出来的悬停正文逐字进签名/说明，一个字不丢。
    #[test]
    fn the_core_hover_envelope_keeps_its_text() {
        let hover = lithe_gpui_java::parse_hover(&json!({
            "contents": "**greet** — 打招呼",
            "isMarkdown": true,
            "range": null
        }))
        .expect("带 contents 的悬停必须能转成上游形状");
        let content = Content::of(&hover.contents).expect("有内容");
        assert_eq!(content.signature, "**greet** — 打招呼");

        assert!(
            lithe_gpui_java::parse_hover(&json!({ "contents": "" })).is_none(),
            "空正文在解析那一层就被丢掉了"
        );
        assert!(lithe_gpui_java::parse_hover(&json!({ "contents": 7 })).is_none());
        assert!(lithe_gpui_java::parse_hover(&json!({ "hover": null })).is_none());
    }
}
