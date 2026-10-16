//! 悬停内容：向 JDTLS 取数，再交给组件库去画。
//!
//! 分工是本 crate 里最"反过来的"一处：**库只画**（`CodeEditor::present_hover` 挂在指针停下的
//! 那个符号上、`present_hover_at` 挂在调用方点名的偏移上），**取数、切分、什么时候收起来**
//! 都归宿主——库在正文变化后不会自己清掉旧内容，所以正文一变这里就把它收掉（见
//! `EditorPane::clear_hover`）。
//!
//! 请求按 LSP 口径报（行 0 基、列按 UTF-16 码元），换算是 `crate::navigation::core_position`。
//!
//! 取回来的正文还要过一道「有没有信息量」的关：内置回退对任何标识符都只回一句反引号包着的
//! 标识符本身，与用户眼里的代码逐字相同，弹出来纯粹挡视线，所以这种自我复述一律不弹
//! （见 `echoes_the_symbol`）。真正的签名、限定名和文档照旧。

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
    /// 其余合成说明，**内容一个字都不丢**（唯一例外是签名外面那对包裹用的反引号：那是
    /// 渲染记号而不是正文，见 `unquote_code_span`）。整份是空白的悬停当作"没有内容"。
    fn of(contents: &str) -> Option<Self> {
        let mut lines = contents
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.trim().is_empty());
        let signature = unquote_code_span(lines.next()?).to_string();
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
    // 指针底下那个词先在宿主侧记下来：正文回来时要拿它判断这次悬停是不是只在复述用户已经
    // 看见的东西（见 `echoes_the_symbol`）。
    let word = crate::navigation::word_range(&text, offset).map(|range| text[range].to_string());
    cx.background_spawn(async move { fetch(java, file_path, text, line, utf16_column, word).await })
}

async fn fetch(
    java: Option<Arc<JavaLanguageService>>,
    file_path: PathBuf,
    body: String,
    line: u64,
    utf16_column: usize,
    word: Option<String>,
) -> Option<Content> {
    let file = file_path.display().to_string();
    let started = Instant::now();

    if let Some(service) = java {
        match service.hover(&file_path, &body, line as u32, utf16_column as u32) {
            Ok(Some(hover)) => {
                let content = shown(Content::of(&hover.contents), word.as_deref());
                println!(
                    "S1_EDITOR_HOVER source=jdtls file={file} line={} col={} markdown={} chars={} shown={} ms={}",
                    line + 1,
                    utf16_column + 1,
                    hover.is_markdown,
                    hover.contents.len(),
                    content.is_some(),
                    started.elapsed().as_millis()
                );
                return content;
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
                let content = shown(Content::of(&hover.contents), word.as_deref());
                println!(
                    "S1_EDITOR_HOVER source=builtin file={file} line={} col={} chars={} shown={} ms={}",
                    line + 1,
                    utf16_column + 1,
                    hover.contents.len(),
                    content.is_some(),
                    started.elapsed().as_millis()
                );
                content
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

/// 把「取回来的正文」过滤成「值得弹出来的正文」。
fn shown(content: Option<Content>, word: Option<&str>) -> Option<Content> {
    content.filter(|content| !echoes_the_symbol(content, word))
}

/// 这次悬停是不是只在复述指针底下的那个词。
///
/// 内置回退（`lsp.builtinHover`）对任何标识符都给一句反引号包着的标识符本身，去掉反引号后
/// 与用户眼里的代码逐字相同，弹出来只是挡住代码，所以不弹。带说明段落的一律留着（有话说）；
/// 限定名（`java.net.UnknownHostException` 对 `UnknownHostException`）也不算复述，因为它
/// 给出了短名里没有的信息。
fn echoes_the_symbol(content: &Content, word: Option<&str>) -> bool {
    let Some(word) = word else {
        return false;
    };
    content.docs.is_empty() && content.signature.trim() == word
}

/// 去掉签名外面那对包裹用的反引号。
///
/// 内置回退的正文是 `` `name` ``，LSP 的代码签名也常整行包在反引号里；那对反引号是「这段是
/// 代码」的记号而不是正文，直接画出来就是用户眼前一对多余的符号。只有整行恰好是一个代码
/// 跨度时才动它——`` `a` 与 `b` `` 这种一句里多个跨度的留给组件库按散文处理。
fn unquote_code_span(signature: &str) -> &str {
    let trimmed = signature.trim();
    if trimmed.matches('`').count() == 2 && trimmed.starts_with('`') && trimmed.ends_with('`') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        signature
    }
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
        let content = Content::of("`void launch()`").expect("轻量悬停");
        assert_eq!(
            content.signature, "void launch()",
            "签名外面那对反引号是渲染记号，不是正文"
        );
        assert!(content.docs.is_empty(), "只有签名时不该多出一段空说明");
    }

    /// 悬停只在复述指针底下那个词时不弹：内置回退对任何标识符都回这一句，弹出来只是挡住代码。
    #[test]
    fn a_hover_that_only_repeats_the_symbol_is_not_shown() {
        let quoted = Content::of("`launch`").expect("内置回退的复述");
        assert!(
            shown(Some(quoted), Some("launch")).is_none(),
            "复述名字本身不弹"
        );
        let bare = Content::of("launch").expect("不带反引号的复述");
        assert!(shown(Some(bare), Some("launch")).is_none());

        let qualified = Content::of("`java.net.UnknownHostException`").expect("限定名");
        assert!(
            shown(Some(qualified), Some("UnknownHostException")).is_some(),
            "限定名给出了短名里没有的信息，要弹"
        );

        let documented = Content::of("`launch`\n\n把当前实现发到引用位置。").expect("带说明");
        assert!(
            shown(Some(documented), Some("launch")).is_some(),
            "有说明段落就留着"
        );

        let other = Content::of("`boot`").expect("另一个词");
        assert!(shown(Some(other), Some("launch")).is_some());

        let unknown = Content::of("`launch`").expect("算不出指针底下的词");
        assert!(
            shown(Some(unknown), None).is_some(),
            "算不出词就不敢当成复述"
        );

        let empty = shown(None, Some("launch"));
        assert!(empty.is_none(), "本来就没内容还是没内容");
    }

    /// 一句里多个代码跨度的签名不动它：整行不是一个跨度，交给组件库按散文处理。
    #[test]
    fn only_a_single_wrapping_code_span_is_unquoted() {
        assert_eq!(unquote_code_span("`a`"), "a");
        assert_eq!(unquote_code_span("`a` 与 `b`"), "`a` 与 `b`");
        assert_eq!(unquote_code_span("a`b"), "a`b");
        assert_eq!(unquote_code_span("void launch()"), "void launch()");
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
