use serde::Deserialize;
use serde_json::Value;

pub const CATEGORIES: &[&str] = &["debug", "fonts", "icons", "languages", "lsp", "themes"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionSnapshot {
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub description: Option<String>,
    pub publisher: String,
    pub entrypoint_kind: String,
    pub categories: Vec<String>,
    pub theme_names: Vec<String>,
    pub icons_names: Vec<String>,
    pub icons_paths: Vec<String>,
    pub font_families: Vec<String>,
    pub debug_adapter_names: Vec<String>,
    pub language_supports: Vec<LanguageSupportSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSupportSnapshot {
    pub id: String,
    pub display_name: String,
    pub file_extensions: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogSnapshot {
    pub extensions: Vec<ExtensionSnapshot>,
}

impl CatalogSnapshot {
    pub fn from_response(data: Value) -> Result<Self, serde_json::Error> {
        let response: CatalogResponse = serde_json::from_value(data)?;
        Ok(Self {
            extensions: response
                .plugins
                .iter()
                .map(ExtensionSnapshot::from_wire)
                .collect(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogResponse {
    plugins: Vec<WirePlugin>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WirePlugin {
    id: String,
    display_name: String,
    version: String,
    #[serde(default)]
    description: Option<String>,
    vendor: WireVendor,
    entrypoint: WireEntrypoint,
    categories: Vec<String>,
    #[serde(default)]
    themes: Vec<WireNamed>,
    #[serde(default)]
    icons: Vec<WireNamed>,
    #[serde(default)]
    fonts: Vec<WireFont>,
    #[serde(default)]
    debug_adapters: Vec<WireAdapter>,
    #[serde(default)]
    language_supports: Vec<WireLanguageSupport>,
}

#[derive(Debug, Deserialize)]
struct WireVendor {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct WireEntrypoint {
    kind: String,
}

#[derive(Debug, Deserialize)]
struct WireNamed {
    name: String,
    #[serde(default)]
    path: String,
}

#[derive(Debug, Deserialize)]
struct WireFont {
    family: String,
}

#[derive(Debug, Deserialize)]
struct WireAdapter {
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireLanguageSupport {
    id: String,
    #[serde(rename = "displayName")]
    display_name: String,
    #[serde(default)]
    file_extensions: Vec<String>,
}

impl ExtensionSnapshot {
    fn from_wire(wire: &WirePlugin) -> Self {
        let mut font_families = wire
            .fonts
            .iter()
            .map(|font| font.family.clone())
            .collect::<Vec<_>>();
        font_families.dedup();
        Self {
            id: wire.id.clone(),
            display_name: wire.display_name.clone(),
            version: wire.version.clone(),
            description: wire.description.clone(),
            publisher: wire.vendor.display_name.clone(),
            entrypoint_kind: wire.entrypoint.kind.clone(),
            categories: wire.categories.clone(),
            theme_names: wire.themes.iter().map(|theme| theme.name.clone()).collect(),
            icons_names: wire.icons.iter().map(|item| item.name.clone()).collect(),
            icons_paths: wire.icons.iter().map(|item| item.path.clone()).collect(),
            font_families,
            debug_adapter_names: wire
                .debug_adapters
                .iter()
                .map(|adapter| adapter.name.clone())
                .collect(),
            language_supports: wire
                .language_supports
                .iter()
                .map(|support| LanguageSupportSnapshot {
                    id: support.id.clone(),
                    display_name: support.display_name.clone(),
                    file_extensions: support.file_extensions.clone(),
                })
                .collect(),
        }
    }
}

pub fn matches_query(entry: &ExtensionSnapshot, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }
    entry.id.to_lowercase().contains(&query)
        || entry.display_name.to_lowercase().contains(&query)
        || entry.publisher.to_lowercase().contains(&query)
        || entry
            .description
            .as_deref()
            .is_some_and(|text| text.to_lowercase().contains(&query))
        || entry
            .theme_names
            .iter()
            .any(|name| name.to_lowercase().contains(&query))
        || entry
            .icons_names
            .iter()
            .any(|name| name.to_lowercase().contains(&query))
        || entry
            .font_families
            .iter()
            .any(|name| name.to_lowercase().contains(&query))
        || entry
            .debug_adapter_names
            .iter()
            .any(|name| name.to_lowercase().contains(&query))
        || entry
            .language_supports
            .iter()
            .any(|support| support.id.to_lowercase().contains(&query))
}

pub fn matches_category(entry: &ExtensionSnapshot, category: Option<&str>) -> bool {
    match category {
        None => true,
        Some(category) => entry.categories.iter().any(|item| item == category),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_response() -> Value {
        serde_json::json!({
            "schemaVersion": 2,
            "hostVersion": "0.0.1",
            "pluginAPIVersion": 1,
            "plugins": [
                {
                    "id": "lithe.builtinThemes",
                    "displayName": "Lithe 内置主题包",
                    "version": "0.0.1",
                    "apiVersion": 1,
                    "hostCompatibility": {"minimum": "0.0.1"},
                    "vendor": {"id": "lithe", "displayName": "Lithe Team", "signatureRequirement": "sameTeamAsHost"},
                    "entrypoint": {"kind": "builtIn", "targetName": "lithe-gpui-settings"},
                    "moduleIDs": ["lithe.theme.builtin"],
                    "categories": ["themes"],
                    "themes": [
                        {"name": "Gruvbox", "path": "themes/gruvbox.json"},
                        {"name": "Nord", "path": "themes/nord.json"}
                    ]
                },
                {
                    "id": "lithe.bundledFonts",
                    "displayName": "JetBrains Mono 字体包",
                    "version": "0.0.1",
                    "apiVersion": 1,
                    "hostCompatibility": {"minimum": "0.0.1"},
                    "vendor": {"id": "lithe", "displayName": "Lithe Team", "signatureRequirement": "sameTeamAsHost"},
                    "entrypoint": {"kind": "builtIn", "targetName": "lithe-gpui-app"},
                    "moduleIDs": ["lithe.font.jetbrainsMono"],
                    "categories": ["fonts"],
                    "fonts": [
                        {"family": "JetBrains Mono", "path": "fonts/JetBrainsMono-Bold.ttf"},
                        {"family": "JetBrains Mono", "path": "fonts/JetBrainsMono-Regular.ttf"}
                    ]
                }
            ]
        })
    }

    #[test]
    fn snapshot_normalizes_contributions_for_display() {
        let snapshot = CatalogSnapshot::from_response(sample_response())
            .expect("sample response should parse");
        assert_eq!(snapshot.extensions.len(), 2);

        let themes = &snapshot.extensions[0];
        assert_eq!(themes.id, "lithe.builtinThemes");
        assert_eq!(themes.publisher, "Lithe Team");
        assert_eq!(themes.entrypoint_kind, "builtIn");
        assert_eq!(themes.theme_names, vec!["Gruvbox", "Nord"]);

        let fonts = &snapshot.extensions[1];
        assert_eq!(fonts.font_families, vec!["JetBrains Mono"]);
    }

    #[test]
    fn query_matches_id_name_publisher_and_description_case_insensitively() {
        let snapshot = CatalogSnapshot::from_response(sample_response())
            .expect("sample response should parse");
        let themes = &snapshot.extensions[0];

        for query in ["lithe.builtinthemes", "内置主题", "lithe team", "gruvbox"] {
            assert!(matches_query(themes, query), "query {query} should match");
        }
        assert!(!matches_query(themes, "jetbrains mono"));
        assert!(matches_query(themes, "   "));
    }

    #[test]
    fn category_filter_matches_derived_categories_only() {
        let snapshot = CatalogSnapshot::from_response(sample_response())
            .expect("sample response should parse");
        let themes = &snapshot.extensions[0];

        assert!(matches_category(themes, None));
        assert!(matches_category(themes, Some("themes")));
        assert!(!matches_category(themes, Some("fonts")));
    }

    #[test]
    fn malformed_response_fails_with_a_describeable_error() {
        let error = CatalogSnapshot::from_response(serde_json::json!([]))
            .expect_err("array payload should not parse");
        assert!(error.is_data());
    }
}
