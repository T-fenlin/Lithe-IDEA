//! `plugins.listCatalog` 响应的展示快照与过滤逻辑。
//!
//! 快照只保留扩展页要显示的字段；贡献项归一为名称列表（字体家族去重）。
//! 分类（`categories`）由 Core 从非空贡献项推导，快照原样携带。

use serde::Deserialize;
use serde_json::Value;

/// 分类过滤器的稳定 id，与 Core 推导出的分类名一一对应，按规范顺序排列。
pub const CATEGORIES: &[&str] = &[
    "debug",
    "fonts",
    "icon-themes",
    "languages",
    "lsp",
    "themes",
];

/// 目录里一个扩展条目的展示快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionSnapshot {
    /// 稳定包标识（如 `lithe.builtinThemes`），也用作 UI 的稳定 ElementId。
    pub id: String,
    /// 展示名称（来自清单 `displayName`，数据自带，不走 i18n）。
    pub display_name: String,
    /// 严格三段式版本号。
    pub version: String,
    /// 清单里的简短描述；缺省时展示层回退到空态文案。
    pub description: Option<String>,
    /// 发布者展示名（清单 `vendor.displayName`）。
    pub publisher: String,
    /// 入口点类型：`builtIn` 或 `nativeBundle`（v1 目录只有 `builtIn`）。
    pub entrypoint_kind: String,
    /// Core 推导并校验过的分类，规范顺序。
    pub categories: Vec<String>,
    /// 贡献的主题族名，按清单顺序。
    pub theme_names: Vec<String>,
    /// 贡献的图标主题名，按清单顺序。
    pub icon_theme_names: Vec<String>,
    /// 贡献的图标主题**清单路径**（`icon-themes/<包>/extension.json`）。
    /// 扩展页从这里取包名 —— 它是设置键 `iconTheme` 的取值。
    pub icon_theme_paths: Vec<String>,
    /// 贡献的字体家族，去重后按规范顺序。
    pub font_families: Vec<String>,
    /// 贡献的调试适配器名，按清单顺序。
    pub debug_adapter_names: Vec<String>,
    /// 贡献的语言支持，按清单顺序。
    pub language_supports: Vec<LanguageSupportSnapshot>,
}

/// 一个语言支持的展示摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageSupportSnapshot {
    /// 小写稳定语言标识（如 `java`）。
    pub id: String,
    /// 语言展示名。
    pub display_name: String,
    /// 识别的文件扩展名（无点前缀），按清单顺序。
    pub file_extensions: Vec<String>,
}

/// 一次 `plugins.listCatalog` 的完整快照。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogSnapshot {
    /// 校验通过的扩展条目，Core 保证按稳定 id 排序。
    pub extensions: Vec<ExtensionSnapshot>,
}

impl CatalogSnapshot {
    /// 把 Core 响应的 `data` 解析成快照。响应形状由 Core 契约保证；解析失败
    /// 返回 serde 错误原文，由展示层决定怎么呈现。
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
    icon_themes: Vec<WireNamed>,
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
        // 字体家族按 (family, path) 有序存储但同一族有多个字重文件；展示层
        // 只需要族名，这里去重并保持稳定顺序。
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
            icon_theme_names: wire
                .icon_themes
                .iter()
                .map(|item| item.name.clone())
                .collect(),
            icon_theme_paths: wire
                .icon_themes
                .iter()
                .map(|item| item.path.clone())
                .collect(),
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

/// 搜索框过滤：对 id、名称、发布者、描述与贡献项名称做大小写不敏感的子串匹配
/// （搜 "gruvbox" 应能命中内置主题包）。
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
            .icon_theme_names
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

/// 分类过滤：`None` 表示「全部」。
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
