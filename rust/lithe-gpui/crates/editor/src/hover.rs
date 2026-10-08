use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

use gpui::{App, AppContext as _, Task, Window};
use gpui_kit::component::input::{EditorState, HoverProvider, Rope};
use lithe_gpui_java::{JavaHover, JavaLanguageService};
use lithe_gpui_shared::core_json;
use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};
use serde_json::json;

pub(crate) fn install(state: &mut EditorState, provider: Option<Rc<dyn HoverProvider>>) {
    let Some(provider) = provider else {
        return;
    };
    state.lsp_mut().hover_provider = Some(provider);
}

pub(crate) struct LitheHoverProvider {
    java: Option<Arc<JavaLanguageService>>,
    file_path: PathBuf,
}

impl LitheHoverProvider {
    pub(crate) fn new(
        java: Option<Arc<JavaLanguageService>>,
        file_path: PathBuf,
    ) -> Rc<dyn HoverProvider> {
        Rc::new(Self { java, file_path })
    }
}

impl HoverProvider for LitheHoverProvider {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<gpui::Result<Option<Hover>>> {
        let (line, utf16_column) = crate::navigation::core_position(text, offset);
        let body = text.to_string();
        let file_path = self.file_path.clone();
        let java = self.java.clone();

        cx.background_spawn(async move {
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
                    Ok(None) => {
                        println!(
                            "S1_EDITOR_HOVER source=jdtls-empty file={file} line={} col={} ms={}",
                            line + 1,
                            utf16_column + 1,
                            started.elapsed().as_millis()
                        );
                        return Ok(None);
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
        assert_eq!(hover.range, None);
    }

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

    #[test]
    fn empty_or_malformed_hover_is_dropped() {
        assert_eq!(lsp_hover_from_core(&json!({ "contents": "" })), None);
        assert_eq!(lsp_hover_from_core(&json!({ "contents": 7 })), None);
        assert_eq!(lsp_hover_from_core(&json!({ "hover": null })), None);
    }
}
