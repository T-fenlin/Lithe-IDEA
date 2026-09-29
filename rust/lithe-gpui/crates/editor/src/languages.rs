//! 注册 gpui-kit **没有内置**的语法 —— 目前只有 XML。
//!
//! ## 为什么需要这一层（gpui-kit 的语法集是固定的）
//!
//! gpui-kit 的语法集来自 `gpui-component` 的 **Cargo feature**：`Language` 枚举按
//! `#[cfg(feature = "tree-sitter-<语言>")]` 门控，`LanguageRegistry::singleton()` 用
//! `Language::all()` 一次性建表（`gpui-component-0.7.0/src/highlighter/registry.rs:517`）。
//! 也就是说：**开了 feature 的语言自动注册，没开的语言在那张表里根本不存在** ——
//! 而那 35 个语法里**没有 XML**，`pom.xml`（Spring Boot 工程里最该高亮的非 Java 文件）
//! 因此无从点亮。
//!
//! ## 我们怎么补上
//!
//! gpui-kit 把注册接口公开了，所以缺的不是能力而是**语法本身**：
//!
//! - `LanguageRegistry::register(name, &GrammarConfig::new(name, language, injections,
//!   highlights, injections_src, locals))` —— 查询以**字符串**形式传入，懒编译；
//! - `GrammarConfig` 要一个 `tree_sitter::Language`，而 `tree-sitter-xml` 同时给出
//!   `LANGUAGE_XML`（`LanguageFn`）与 `XML_HIGHLIGHT_QUERY`（查询文本）。
//!
//! 于是**不需要 unsafe、不需要加载器、不需要自己收 `.scm`**：把两者拼起来注册即可。
//!
//! ## 这条路径就是"运行期加载语法"的注册缝
//!
//! 上面的 `register(...)` 调用与"从扩展目录加载一个语法再注册"**是同一个 API**，
//! 唯一差别是 `Language` 从哪来：现在是 `tree_sitter_xml::LANGUAGE_XML.into()`（链接进
//! 二进制），将来若改成运行期加载，这里换成"用加载器造出来的 parser"即可，
//! 其它代码（查询字符串、语言名、`buffer.rs` 的映射、主题配色）一行都不用动。
//!
//! 而"真正运行期加载"要补的是**加载器**本身，那是另一个决策（原生 `dlopen` + `unsafe`，
//! 或 WASM + wasmtime 沙箱），取舍见
//! `.agents/notes/implemented/feature/2026-09-29-plugin-catalog-v2-and-extensions-page.md`。

use std::sync::Once;

use gpui_kit::component::highlighter::{GrammarConfig, LanguageRegistry};

/// 注册所有"gpui-kit 没内置、我们自己带"的语法。
///
/// **幂等**（`Once`）：注册表是进程全局单例，重复注册只是覆盖同一份配置；
/// 但每次覆盖都会 bump 它的 generation 并让已开编辑器的缓存失效，所以这里只做一次。
///
/// 调用时机：**给编辑器设语言之前**（`editor_view.rs` 的两处 `state.language(..)`）。
/// 上游明确说注册后已打开的编辑器不会自动换高亮（`gpui-base-0.7.0/src/input/editor/
/// language.rs:25-27` 要求重设语言），所以"先注册、后建编辑器"是唯一没有窗口期的顺序。
pub fn ensure_extra_languages() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let registry = LanguageRegistry::singleton();
        registry.register(
            "xml",
            &GrammarConfig::new(
                "xml",
                tree_sitter_xml::LANGUAGE_XML.into(),
                // XML 没有要注入的其它语言（DTD 是同一个 crate 里的另一套语法，
                // 由 `XML_HIGHLIGHT_QUERY` 自己覆盖，不需要 injection 机制）。
                Vec::new(),
                tree_sitter_xml::XML_HIGHLIGHT_QUERY,
                "",
                "",
            ),
        );
        println!("S1_LANGUAGES registered=1 names=xml");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 注册之后，注册表里真的能查到 `xml`，且**带着语法**（`has_grammar`）——
    /// 只注册一个空名字的话编辑器仍然不上色，那与不注册没有区别。
    #[test]
    fn xml_is_registered_with_a_grammar() {
        ensure_extra_languages();
        let config = LanguageRegistry::singleton()
            .language("xml")
            .expect("注册后 registry 里应当有 xml");
        assert!(config.has_grammar(), "xml 必须带 grammar，否则不上色");
    }

    /// 幂等：重复调用不改变结果（注册表里仍是同一份可用配置）。
    #[test]
    fn registration_is_idempotent() {
        ensure_extra_languages();
        ensure_extra_languages();
        assert!(
            LanguageRegistry::singleton()
                .language("xml")
                .is_some_and(|config| config.has_grammar())
        );
    }
}
