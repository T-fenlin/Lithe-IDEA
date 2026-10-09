//! 语言支持：**语法高亮是可插拔的**。
//!
//! 本模块只提供两样东西——**注册表**（插件往里注册语言）与**高亮引擎**（tree-sitter 解析 +
//! 查询 + 映射到主题配色）。库**不认识任何具体语言**：grammar 与高亮查询都由语言支持插件提供，
//! 这样"加一种语言"不需要改组件库，也不必重新发版组件库。
//!
//! 分层与 Zed 的 `editor` / `language` / `theme` 三个 crate 一一对应：编辑平面在 [`crate::editor`]、
//! 语法与查询在本模块、配色在 [`crate::theme::Syntax`]。Zed 的 `language`/`theme`/`editor` 都是
//! GPL-3.0-or-later 且未发布（不可作库），所以这一层由我们自己实现，grammar 用 MIT/Apache 的
//! `tree-sitter` 生态。
//!
//! 缺失时的行为：**没有注册对应语言就留空**（回到调用方的退化路径），不假装能高亮。

use std::ops::Range;
use std::sync::OnceLock;

use gpui::Hsla;
use tree_sitter::{Language as Grammar, Parser, Query, QueryCursor, StreamingIterator as _, Tree};

use crate::theme::Syntax;

/// 一种语言的语法支持。语言支持插件声明它，组件库只按接口使用。
pub struct Language {
    /// 稳定标识（如 `java`、`bash`），**也是宿主"禁用某语言"开关的键**。
    ///
    /// 因此它必须与宿主的既有取值一致（例如 shell 脚本是 `bash` 而不是 `shell`）：id 一漂移，
    /// "卸载项目语言包"就再也关不掉对应高亮——这类不一致不会报错，只会静默失灵。
    pub id: &'static str,
    /// 展示名（如 `Java`）。
    pub name: &'static str,
    /// 归属的文件扩展名（小写、不带点）。
    pub extensions: &'static [&'static str],
    /// 归属的**文件名**（小写），给没有扩展名的脚本用（如 `mvnw`、`gradlew.bat`）。
    pub filenames: &'static [&'static str],
    /// 语法本体。用函数指针而不是值：`Language` 要能放进 `static`，而 grammar 的构造不是 const。
    pub grammar: fn() -> Grammar,
    /// 高亮查询（`highlights.scm` 源文本），通常直接取 grammar crate 自带的常量。
    pub highlights: &'static str,
    /// 行注释前缀（注释/反注释、折叠判断用）。
    pub line_comment: &'static str,
}

/// 语法槽位，与 [`Syntax`] 的 13 个颜色槽一一对应。
///
/// 这是 tree-sitter capture 名与主题配色之间的**唯一映射点**：查询里出现别的 capture 名就留空，
/// 不硬塞到某个颜色上。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Keyword,
    String,
    Number,
    Comment,
    Function,
    TypeName,
    Constant,
    Property,
    Tag,
    Attribute,
    Operator,
    Punctuation,
    Variable,
}

impl Slot {
    /// tree-sitter capture 名 → 槽位。按名称的**首段**判定，这样 `function.method`、
    /// `type.builtin`、`variable.parameter` 这些细化名都能落到同一个槽。
    #[must_use]
    pub fn from_capture(name: &str) -> Option<Self> {
        Some(match name.split('.').next()? {
            "keyword" => Self::Keyword,
            "string" => Self::String,
            "number" => Self::Number,
            "comment" => Self::Comment,
            "function" | "method" => Self::Function,
            "type" | "constructor" => Self::TypeName,
            "constant" | "boolean" => Self::Constant,
            "property" | "field" => Self::Property,
            "tag" => Self::Tag,
            "attribute" => Self::Attribute,
            "operator" => Self::Operator,
            "punctuation" | "delimiter" | "bracket" => Self::Punctuation,
            "variable" | "parameter" | "label" => Self::Variable,
            _ => return None,
        })
    }

    /// 槽位在当前配色下的颜色。
    #[must_use]
    pub fn color(self, syntax: &Syntax) -> Hsla {
        match self {
            Self::Keyword => syntax.keyword,
            Self::String => syntax.string,
            Self::Number => syntax.number,
            Self::Comment => syntax.comment,
            Self::Function => syntax.function,
            Self::TypeName => syntax.type_name,
            Self::Constant => syntax.constant,
            Self::Property => syntax.property,
            Self::Tag => syntax.tag,
            Self::Attribute => syntax.attribute,
            Self::Operator => syntax.operator,
            Self::Punctuation => syntax.punctuation,
            Self::Variable => syntax.variable,
        }
    }
}

struct Registry {
    languages: Vec<&'static Language>,
    by_extension: Vec<(&'static str, &'static Language)>,
    by_filename: Vec<(&'static str, &'static Language)>,
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// 注册一批语言支持。语言支持插件在启动时调用一次。
///
/// 重复注册（第二次调用）不会 panic：记一条警告后忽略，因为宿主与插件都可能尝试注册。
/// 但**同一个 id 在一批里出现两次**是插件自身的错误，会 panic——否则就是静默丢一种语言。
pub fn register(languages: &'static [Language]) {
    let mut registered: Vec<&'static Language> = Vec::with_capacity(languages.len());
    let mut by_extension: Vec<(&'static str, &'static Language)> = Vec::new();
    let mut by_filename: Vec<(&'static str, &'static Language)> = Vec::new();
    for language in languages {
        assert!(
            !registered.iter().any(|each| each.id == language.id),
            "语言支持插件重复注册了 `{}`",
            language.id
        );
        registered.push(language);
        for extension in language.extensions {
            assert!(
                !by_extension
                    .iter()
                    .any(|(each, _)| each.eq_ignore_ascii_case(extension)),
                "扩展名 `{extension}` 被 `{}` 与另一种语言同时声明",
                language.id
            );
            by_extension.push((extension, language));
        }
        for filename in language.filenames {
            assert!(
                !by_filename
                    .iter()
                    .any(|(each, _)| each.eq_ignore_ascii_case(filename)),
                "文件名 `{filename}` 被 `{}` 与另一种语言同时声明",
                language.id
            );
            by_filename.push((filename, language));
        }
    }
    if REGISTRY
        .set(Registry {
            languages: registered,
            by_extension,
            by_filename,
        })
        .is_err()
    {
        log::warn!("language: 语言支持已经注册过，忽略这一次（插件重复初始化？）");
    }
}

/// 已注册的全部语言；没有注册过就是空表（不是错误）。
#[must_use]
pub fn all() -> &'static [&'static Language] {
    REGISTRY.get().map_or(&[], |registry| &registry.languages)
}

/// 按文件扩展名（不带点、大小写不敏感）找语言支持。
#[must_use]
pub fn by_extension(extension: &str) -> Option<&'static Language> {
    REGISTRY
        .get()?
        .by_extension
        .iter()
        .find(|(each, _)| each.eq_ignore_ascii_case(extension))
        .map(|(_, language)| *language)
}

/// 按**文件名**（不含目录、大小写不敏感）找语言支持——给 `mvnw`、`gradlew.bat` 这类
/// 没有扩展名的脚本用。
#[must_use]
pub fn by_filename(filename: &str) -> Option<&'static Language> {
    let name = filename.rsplit(['/', '\\']).next()?;
    REGISTRY
        .get()?
        .by_filename
        .iter()
        .find(|(each, _)| each.eq_ignore_ascii_case(name))
        .map(|(_, language)| *language)
}

/// 按路径找语言支持：**先看整个文件名**，再看最后一个 `.` 之后的扩展名。
///
/// 顺序有意义：`mvnw` 这类无扩展名脚本只能靠文件名认出来，而 `pom.xml` 只靠扩展名就够。
#[must_use]
pub fn by_path(path: &str) -> Option<&'static Language> {
    let name = path.rsplit(['/', '\\']).next()?;
    if let Some(language) = by_filename(name) {
        return Some(language);
    }
    by_extension(name.rsplit_once('.')?.1)
}

/// 一种语言的解析与高亮缓存。
///
/// 调用方（编辑器）**按行**取色，但 tree-sitter 必须**整篇解析**：所以这里持有整篇的
/// span 表，`spans_in` 按字节范围切出来。文本变化时调用 [`Highlighter::reparse`]。
pub struct Highlighter {
    language: &'static Language,
    parser: Parser,
    query: Query,
    /// 按 capture 下标查槽位；未识别的 capture 是 `None`（留空）。
    slots: Vec<Option<Slot>>,
    /// 已解析的整篇 span，按 `(起点升序, 终点降序)` 排——同起点时**外层在前**，
    /// 这样渲染时后画的里层覆盖外层，和语法嵌套一致。
    spans: Vec<(Range<usize>, Slot)>,
    tree: Option<Tree>,
}

impl Highlighter {
    /// 为一种语言建立高亮器。grammar 或查询装不上就返回 `None`（调用方据此走退化路径）。
    #[must_use]
    pub fn new(language: &'static Language) -> Option<Self> {
        let grammar = (language.grammar)();
        let mut parser = Parser::new();
        if let Err(error) = parser.set_language(&grammar) {
            log::warn!("language: `{}` 的 grammar 装不上：{error}", language.id);
            return None;
        }
        let query = match Query::new(&grammar, language.highlights) {
            Ok(query) => query,
            Err(error) => {
                log::warn!("language: `{}` 的高亮查询编译不过：{error}", language.id);
                return None;
            }
        };
        let slots = query
            .capture_names()
            .iter()
            .map(|name| Slot::from_capture(name))
            .collect();
        Some(Self {
            language,
            parser,
            query,
            slots,
            spans: Vec::new(),
            tree: None,
        })
    }

    #[must_use]
    pub fn language(&self) -> &'static Language {
        self.language
    }

    /// 整篇重新解析并重建 span 表。这是本模块唯一的"重活"，由调用方在文本变化后触发。
    pub fn reparse(&mut self, text: &str) {
        let Some(tree) = self.parser.parse(text, None) else {
            self.spans.clear();
            self.tree = None;
            return;
        };
        let mut spans: Vec<(Range<usize>, Slot)> = Vec::new();
        {
            let mut cursor = QueryCursor::new();
            let mut matches = cursor.matches(&self.query, tree.root_node(), text.as_bytes());
            while let Some(found) = matches.next() {
                for capture in found.captures {
                    let Some(slot) = self.slots.get(capture.index as usize).copied().flatten()
                    else {
                        continue;
                    };
                    let range = capture.node.byte_range();
                    if range.is_empty() {
                        continue;
                    }
                    spans.push((range, slot));
                }
            }
        }
        spans.sort_by(|(left, _), (right, _)| {
            left.start.cmp(&right.start).then(right.end.cmp(&left.end))
        });
        self.spans = spans;
        self.tree = Some(tree);
    }

    /// 与 `range` 相交的 span，已裁剪到 `range` 之内（编辑器渲染一行时用）。
    #[must_use]
    pub fn spans_in(&self, range: Range<usize>) -> Vec<(Range<usize>, Slot)> {
        if range.is_empty() {
            return Vec::new();
        }
        let first = self
            .spans
            .partition_point(|(span, _)| span.end <= range.start);
        self.spans[first..]
            .iter()
            .take_while(|(span, _)| span.start < range.end)
            .map(|(span, slot)| (span.start.max(range.start)..span.end.min(range.end), *slot))
            .collect()
    }

    /// 整篇 span 数；测试与诊断用。
    #[must_use]
    pub fn span_count(&self) -> usize {
        self.spans.len()
    }

    /// 解析树是否就绪（诊断用）。
    #[must_use]
    pub fn has_tree(&self) -> bool {
        self.tree.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_capture_lands_on_its_slot_by_first_segment() {
        assert_eq!(Slot::from_capture("keyword"), Some(Slot::Keyword));
        assert_eq!(Slot::from_capture("function.method"), Some(Slot::Function));
        assert_eq!(Slot::from_capture("type.builtin"), Some(Slot::TypeName));
        assert_eq!(
            Slot::from_capture("punctuation.bracket"),
            Some(Slot::Punctuation)
        );
        assert_eq!(
            Slot::from_capture("variable.parameter"),
            Some(Slot::Variable)
        );
        // 不认识的名字**留空**，不硬塞颜色。
        assert_eq!(Slot::from_capture("spell"), None);
        assert_eq!(Slot::from_capture(""), None);
    }

    #[test]
    fn a_slot_answers_with_its_theme_color() {
        let syntax = crate::theme::Palette::dark(false).syntax;
        assert_eq!(Slot::Keyword.color(&syntax), syntax.keyword);
        assert_eq!(Slot::TypeName.color(&syntax), syntax.type_name);
        assert_eq!(Slot::Punctuation.color(&syntax), syntax.punctuation);
    }

    /// 语言支持是插件提供的：查询一个没人声明的扩展名必须给出"没有"，而不是 panic 或瞎猜。
    #[test]
    fn a_path_the_plugins_do_not_claim_answers_nothing() {
        assert!(by_path("no-extension-file").is_none());
        assert!(by_path(r"a\b\c.unknownext").is_none());
        assert!(by_extension("unknownext").is_none());
    }

    /// 路径解析取最后一段的最后一个点：目录里的点不能当扩展名分隔符。
    #[test]
    fn a_path_takes_the_last_segment_and_extension() {
        assert!(by_path("dir.with.dots/plain").is_none());
    }
}
