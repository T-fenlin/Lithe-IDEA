use std::sync::Once;

use gpui_kit::component::highlighter::{GrammarConfig, LanguageRegistry};

pub fn ensure_extra_languages() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let registry = LanguageRegistry::singleton();
        registry.register(
            "xml",
            &GrammarConfig::new(
                "xml",
                tree_sitter_xml::LANGUAGE_XML.into(),
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

    #[test]
    fn xml_is_registered_with_a_grammar() {
        ensure_extra_languages();
        let config = LanguageRegistry::singleton()
            .language("xml")
            .expect("注册后 registry 里应当有 xml");
        assert!(config.has_grammar(), "xml 必须带 grammar，否则不上色");
    }

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
