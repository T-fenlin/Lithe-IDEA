//! 第一方**语言支持插件**：把 tree-sitter 语法注册进组件库的语言注册表。
//!
//! 组件库（`lithe-ui`）不认识任何具体语言，它只提供注册表与高亮引擎；本 crate 是第一个
//! "语言支持提供方"——声明语言、grammar 与高亮查询，并在宿主启动时注册一次。
//!
//! 现在的装载方式与 gpui-kit 时代的做法一致：grammar 作为依赖**静态编进二进制**，因此
//! "加一种语言"需要重新构建宿主。这与"语言支持可插拔"的目标不冲突——插件的**边界**就是
//! [`Language`] 与注册表；后续把装载源换成扩展目录里的动态库时，本 crate 的声明方式不变。
//!
//! 已知边界（不假装完整）：
//!
//! * **Markdown** 只注册了 block 语法与 block 查询。行内（强调、链接、代码片段）需要
//!   `tree_sitter_md::INLINE_LANGUAGE` + `HIGHLIGHT_QUERY_INLINE` 的第二次解析（术语上是
//!   injections），要等引擎支持注入后再补。
//! * **XML** 用 XML 语法；DTD 是另一个 grammar（`LANGUAGE_DTD`），尚未注册。

use lithe_ui::language::Language;

/// 本插件提供的语言。宿主在启动时通过 [`register`] 注册。
static LANGUAGES: [Language; 8] = [
    Language {
        id: "java",
        name: "Java",
        extensions: &["java"],
        grammar: || tree_sitter_java::LANGUAGE.into(),
        highlights: tree_sitter_java::HIGHLIGHTS_QUERY,
        line_comment: "//",
    },
    Language {
        id: "kotlin",
        name: "Kotlin",
        extensions: &["kt", "kts"],
        grammar: || tree_sitter_kotlin_sg::LANGUAGE.into(),
        highlights: tree_sitter_kotlin_sg::HIGHLIGHTS_QUERY,
        line_comment: "//",
    },
    Language {
        id: "xml",
        name: "XML",
        extensions: &["xml", "xsd"],
        grammar: || tree_sitter_xml::LANGUAGE_XML.into(),
        highlights: tree_sitter_xml::XML_HIGHLIGHT_QUERY,
        line_comment: "<!--",
    },
    Language {
        id: "yaml",
        name: "YAML",
        extensions: &["yaml", "yml"],
        grammar: || tree_sitter_yaml::LANGUAGE.into(),
        highlights: tree_sitter_yaml::HIGHLIGHTS_QUERY,
        line_comment: "#",
    },
    Language {
        id: "toml",
        name: "TOML",
        extensions: &["toml"],
        grammar: || tree_sitter_toml_ng::LANGUAGE.into(),
        highlights: tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
        line_comment: "#",
    },
    Language {
        id: "sql",
        name: "SQL",
        extensions: &["sql"],
        grammar: || tree_sitter_sequel::LANGUAGE.into(),
        highlights: tree_sitter_sequel::HIGHLIGHTS_QUERY,
        line_comment: "--",
    },
    Language {
        id: "shell",
        name: "Shell",
        extensions: &["sh", "bash", "zsh"],
        grammar: || tree_sitter_bash::LANGUAGE.into(),
        highlights: tree_sitter_bash::HIGHLIGHT_QUERY,
        line_comment: "#",
    },
    Language {
        id: "markdown",
        name: "Markdown",
        extensions: &["md", "markdown"],
        grammar: || tree_sitter_md::LANGUAGE.into(),
        highlights: tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
        // Markdown 没有行注释语法；留空而不是塞一个别的标记。
        line_comment: "",
    },
];

/// 注册本插件声明的全部语言。宿主启动时调用一次；重复调用只会记一条警告。
pub fn register() {
    lithe_ui::language::register(&LANGUAGES);
}

#[cfg(test)]
mod tests {
    use lithe_ui::editor::Buffer;
    use lithe_ui::language::{self, Highlighter, Language, Slot};

    use super::register;

    /// 每个测试都先注册：注册表是进程级的，测试并行跑，不能假设别人先注册过。
    fn ready() {
        register();
    }

    #[test]
    fn every_declared_language_is_reachable_by_its_extensions() {
        ready();
        for language in language::all() {
            assert!(
                !language.extensions.is_empty(),
                "{} 没声明扩展名",
                language.id
            );
            for extension in language.extensions {
                let found = language::by_extension(extension)
                    .unwrap_or_else(|| panic!("扩展名 `{extension}` 应该解析到 {}", language.id));
                assert_eq!(found.id, language.id);
            }
        }
        // 大小写不敏感：文件名大小写不该决定高亮。
        assert!(language::by_extension("JAVA").is_some());
        assert_eq!(
            language::by_path("src/main/java/A.java").map(|each| each.id),
            Some("java")
        );
    }

    /// 真实 grammar 的端到端断言：Java 文本的关键字与数字各自落到对应槽位。
    #[test]
    fn java_text_colors_a_keyword_and_a_number() {
        ready();
        let java = language::by_extension("java").expect("java 已注册");
        let mut highlighter = Highlighter::new(java).expect("java 高亮器");
        let source = "class A { int x = 1; }";
        highlighter.reparse(source);
        assert!(highlighter.has_tree(), "解析树应该建好");
        let spans = highlighter.spans_in(0..source.len());
        assert!(!spans.is_empty(), "Java 片段应该至少有一个高亮 span");
        assert!(
            spans
                .iter()
                .any(|(range, slot)| *slot == Slot::Keyword && &source[range.clone()] == "class"),
            "`class` 应该是关键字：{spans:?}"
        );
        assert!(
            spans
                .iter()
                .any(|(range, slot)| *slot == Slot::Number && &source[range.clone()] == "1"),
            "`1` 应该是数字：{spans:?}"
        );
        // 切片必须落在请求范围内：编辑器按行取色，越界会串色到邻行。
        for (range, _) in highlighter.spans_in(0..9) {
            assert!(range.end <= 9, "{range:?} 越过了请求范围");
        }
    }

    /// 未注册的语言留空：没有 grammar 就不该硬画出颜色。
    #[test]
    fn an_unregistered_extension_stays_blank() {
        ready();
        assert!(language::by_extension("nope").is_none());
    }

    /// 端到端：组件库**自己的编辑器缓冲区**接上注册表后，Java 的颜色来自 tree-sitter。
    ///
    /// 与上面的断言不同，这一条走的是编辑器真正走的入口（`Buffer::set_language` +
    /// `Buffer::highlight_spans`），不是自己另建一个 `Highlighter`——所以它证明的是"接线接上了"，
    /// 而不只是"引擎能用"。
    #[test]
    fn the_editor_buffer_colors_java_through_tree_sitter() {
        ready();
        let java = language::by_extension("java").expect("java 已注册");
        let source = "class A { int x = 1; }";
        let mut buffer = Buffer::new(source);
        assert!(
            !buffer.has_highlighter(),
            "还没设置语言时，缓冲区不该有 tree-sitter 高亮器"
        );
        buffer.set_language(Some(java));
        assert_eq!(buffer.language().map(|each| each.id), Some("java"));
        assert!(
            buffer.has_highlighter(),
            "Java 的 grammar 与高亮查询都应该装得上"
        );
        let spans = buffer.highlight_spans(0..source.len());
        // 一个词可能同时落进多个 capture（`A` 既是 `variable` 又是 `type`），所以按词收集槽位。
        let slots = |word: &str| {
            spans
                .iter()
                .filter(|(range, _)| &source[range.clone()] == word)
                .map(|(_, slot)| *slot)
                .collect::<Vec<_>>()
        };
        assert!(
            slots("class").contains(&Slot::Keyword),
            "`class` 应该由 tree-sitter 认成关键字：{spans:?}"
        );
        assert!(
            slots("1").contains(&Slot::Number),
            "`1` 应该由 tree-sitter 认成数字：{spans:?}"
        );
        // 这一条才是"走的是语法而不是词法器"的证据：库内词法器对小写词只会给 `Variable`
        // （`int` 不在它的关键字表里），只有 Java 语法能把 `int` 认成内建类型。
        assert!(
            slots("int").contains(&Slot::TypeName),
            "`int` 应该由 Java 语法认成内建类型，词法器给不出这个槽位：{spans:?}"
        );
        // 编辑器按行/按可见片段取色：请求区间之外的字节一概不参与。
        let head = buffer.highlight_spans(0..5);
        assert!(!head.is_empty(), "`class` 正好落在这个区间里");
        assert!(
            head.iter().all(|(range, _)| range.end <= 5),
            "{head:?} 越过了请求区间"
        );
    }

    /// 语言声明了但高亮器装不上（查询编译不过）：不 panic、报"没有高亮器"、取 span 给空。
    ///
    /// 这正是"可见降级"的分叉点：编辑器看到 `has_highlighter()` 为假就回退到自己的简易词法器
    /// ——有颜色可看，而不是假装高亮，也不是每帧重试一次昂贵的查询编译。
    #[test]
    fn a_language_whose_query_will_not_compile_degrades_without_panicking() {
        /// 语法是真的，查询是坏的：这就是"插件声明了但装不上"的形态。
        static UNBUILDABLE: Language = Language {
            id: "unbuildable",
            name: "Unbuildable",
            extensions: &["unbuildable"],
            grammar: || tree_sitter_java::LANGUAGE.into(),
            highlights: "(this is not a query",
            line_comment: "//",
        };
        let mut buffer = Buffer::new("class A { int x = 1; }");
        buffer.set_language(Some(&UNBUILDABLE));
        assert_eq!(buffer.language().map(|each| each.id), Some("unbuildable"));
        assert!(!buffer.has_highlighter(), "装不上的语言必须报没有高亮器");
        assert!(buffer.highlight_spans(0..buffer.len()).is_empty());
        // 再问一次仍然是"没有"，而不是重试到 panic。
        assert!(!buffer.has_highlighter());
        // 撤回语言回到词法器路径，同样不 panic。
        buffer.set_language(None);
        assert!(!buffer.has_highlighter());
        assert!(buffer.highlight_spans(0..buffer.len()).is_empty());
    }
}
