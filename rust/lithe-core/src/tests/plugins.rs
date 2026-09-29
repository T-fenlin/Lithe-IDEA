use crate::plugins::{
    validate_plugin_catalog_json, PluginValidationError, PluginVersion,
    PLUGIN_MANIFEST_SCHEMA_VERSION,
};
use serde_json::Value;

const OFFICIAL_PLUGINS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../shared/fixtures/plugins/official-v2.json"
));
const LANGUAGE_SUPPORT_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../shared/fixtures/plugins/language-support-v2.json"
));

const HOST_VERSION: PluginVersion = PluginVersion {
    major: 0,
    minor: 0,
    patch: 1,
};

#[test]
fn official_plugin_catalog_is_valid_and_covers_every_builtin_category() {
    let owners = validate_plugin_catalog_json(OFFICIAL_PLUGINS, HOST_VERSION)
        .expect("official plugin fixture should validate");
    assert!(!owners.is_empty());

    let catalog: Value =
        serde_json::from_str(OFFICIAL_PLUGINS).expect("official plugin fixture should be JSON");
    assert_eq!(
        catalog["schemaVersion"], PLUGIN_MANIFEST_SCHEMA_VERSION,
        "fixture schema version must track the Core build"
    );
    // Categories are derived from non-empty contributions, so a category is
    // covered exactly when some plugin carries that contribution list.
    let mut covered = Vec::new();
    for plugin in catalog["plugins"].as_array().expect("plugins array") {
        if !plugin["themes"]
            .as_array()
            .unwrap_or(&Vec::new())
            .is_empty()
        {
            covered.push("themes");
        }
        if !plugin["iconThemes"]
            .as_array()
            .unwrap_or(&Vec::new())
            .is_empty()
        {
            covered.push("icon-themes");
        }
        if !plugin["fonts"].as_array().unwrap_or(&Vec::new()).is_empty() {
            covered.push("fonts");
        }
        if !plugin["debugAdapters"]
            .as_array()
            .unwrap_or(&Vec::new())
            .is_empty()
        {
            covered.push("debug");
        }
        if !plugin["languageSupports"]
            .as_array()
            .unwrap_or(&Vec::new())
            .is_empty()
        {
            covered.push("languages");
            if plugin["languageSupports"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|support| support["languageServerModuleID"].is_string())
            {
                covered.push("lsp");
            }
        }
    }
    for category in [
        "themes",
        "icon-themes",
        "fonts",
        "debug",
        "languages",
        "lsp",
    ] {
        assert!(
            covered.contains(&category),
            "official catalog should cover the {category} category"
        );
    }
}

#[test]
fn language_support_catalog_allows_execution_and_testing_to_share_a_module() {
    let owners = validate_plugin_catalog_json(LANGUAGE_SUPPORT_FIXTURE, HOST_VERSION)
        .expect("language support fixture should validate");
    assert_eq!(owners.len(), 2);
    assert_eq!(
        owners.get("dev.lithe.fixture.go.language-server"),
        Some(&"dev.lithe.fixture.go-support".to_string())
    );
    assert_eq!(
        owners.get("dev.lithe.fixture.go.execution"),
        Some(&"dev.lithe.fixture.go-support".to_string())
    );
}

#[test]
fn incompatible_host_is_rejected_deterministically() {
    let error = validate_plugin_catalog_json(
        OFFICIAL_PLUGINS,
        PluginVersion {
            major: 0,
            minor: 1,
            patch: 0,
        },
    )
    .unwrap_err();
    assert_eq!(
        error,
        PluginValidationError::IncompatibleHost {
            plugin: "catalog".into()
        }
    );
}

#[test]
fn schema_v1_catalog_is_rejected_with_unsupported_schema() {
    let legacy = r#"{
        "schemaVersion": 1,
        "hostVersion": "0.0.1",
        "pluginAPIVersion": 1,
        "plugins": []
    }"#;
    let error = validate_plugin_catalog_json(legacy, HOST_VERSION).unwrap_err();
    assert_eq!(
        error,
        PluginValidationError::UnsupportedSchema {
            plugin: "catalog".into(),
            version: 1,
        }
    );
}

/// 找到某个 id 在 plugins 数组里的下标（变异测试要改某一条的贡献项）。
fn index_of(catalog: &Value, id: &str) -> usize {
    catalog["plugins"]
        .as_array()
        .expect("plugins array")
        .iter()
        .position(|plugin| plugin["id"] == id)
        .unwrap_or_else(|| panic!("official catalog must contain {id}"))
}

#[test]
fn unsorted_theme_contribution_is_rejected_deterministically() {
    let catalog: Value = serde_json::from_str(OFFICIAL_PLUGINS).expect("official catalog");
    let mut unsorted = catalog.clone();
    unsorted["plugins"][index_of(&catalog, "lithe.gruvboxTheme")]["themes"] = serde_json::json!([
        {"name": "Nord", "path": "extensions/themes/nord.json"},
        {"name": "Gruvbox", "path": "extensions/themes/gruvbox.json"}
    ]);
    let error = validate_plugin_catalog_json(&unsorted.to_string(), HOST_VERSION).unwrap_err();
    assert_eq!(
        error,
        PluginValidationError::InvalidContribution {
            plugin: "lithe.gruvboxTheme".into(),
            category: "themes",
        }
    );
}

#[test]
fn theme_path_escaping_the_bundle_is_rejected() {
    let catalog: Value = serde_json::from_str(OFFICIAL_PLUGINS).expect("official catalog");
    let mut escaping = catalog.clone();
    escaping["plugins"][index_of(&catalog, "lithe.gruvboxTheme")]["themes"] = serde_json::json!([
        {"name": "Gruvbox", "path": "../secrets/gruvbox.json"},
        {"name": "Nord", "path": "extensions/themes/nord.json"}
    ]);
    let error = validate_plugin_catalog_json(&escaping.to_string(), HOST_VERSION).unwrap_err();
    assert_eq!(
        error,
        PluginValidationError::InvalidContribution {
            plugin: "lithe.gruvboxTheme".into(),
            category: "themes",
        }
    );
}

#[test]
fn list_catalog_returns_validated_entries_with_derived_categories() {
    let request = serde_json::json!({
        "id": "catalog",
        "command": "plugins.listCatalog",
        "payload": {"hostVersion": "0.0.1"}
    });
    let response: Value = serde_json::from_str(&crate::execute_json(
        &serde_json::to_string(&request).expect("catalog request should encode"),
    ))
    .expect("catalog response should be JSON");
    assert_eq!(response["ok"], true, "{response}");

    let plugins = response["data"]["plugins"]
        .as_array()
        .expect("plugins array should encode");
    assert_eq!(plugins.len(), 12);
    let by_id: Vec<(&str, Vec<&str>)> = plugins
        .iter()
        .map(|plugin| {
            let categories: Vec<&str> = plugin["categories"]
                .as_array()
                .expect("categories should encode")
                .iter()
                .map(|value| value.as_str().expect("category should be a string"))
                .collect();
            (plugin["id"].as_str().expect("plugin id"), categories)
        })
        .collect();
    assert_eq!(
        by_id,
        vec![
            ("lithe.gruvboxTheme", vec!["themes"]),
            ("lithe.ibmPlexMonoFont", vec!["fonts"]),
            ("lithe.ideaFont", vec!["fonts"]),
            ("lithe.ideaIconTheme", vec!["icon-themes"]),
            ("lithe.ideaTheme", vec!["themes"]),
            ("lithe.javaDebugAdapter", vec!["debug"]),
            ("lithe.javaLanguageSupport", vec!["languages", "lsp"]),
            ("lithe.litheTheme", vec!["themes"]),
            ("lithe.nordTheme", vec!["themes"]),
            ("lithe.oneTheme", vec!["themes"]),
            ("lithe.sourceCodeProFont", vec!["fonts"]),
            ("lithe.vscodeTheme", vec!["themes"]),
        ]
    );
    // The listing must echo the manifest fields the extensions page renders:
    // - one `builtIn` font (the IDEA default) and two `bundled` font families,
    // - one `builtIn` theme (IDEA, the default) and five `bundled` theme groups.
    let idea_font = &plugins[2];
    assert_eq!(idea_font["displayName"], "IDEA 字体（内置）");
    assert_eq!(idea_font["entrypoint"]["kind"], "builtIn");
    assert_eq!(idea_font["fonts"][0]["family"], "JetBrains Mono");
    assert_eq!(
        idea_font["fonts"][0]["path"],
        "assets/fonts/JetBrainsMono-Bold.ttf"
    );
    let plex = &plugins[1];
    assert_eq!(plex["entrypoint"]["kind"], "bundled");
    assert_eq!(
        plex["fonts"][0]["path"],
        "extensions/fonts/ibm-plex-mono/IBMPlexMono-Bold.ttf"
    );
    assert_eq!(
        plugins
            .iter()
            .filter(|plugin| {
                plugin["entrypoint"]["kind"] == "bundled"
                    && !plugin["fonts"].as_array().expect("fonts").is_empty()
            })
            .count(),
        2,
        "两个可卸载的字体扩展"
    );
    let idea = &plugins[4];
    assert_eq!(idea["displayName"], "IDEA 主题（内置）");
    assert_eq!(idea["entrypoint"]["kind"], "builtIn");
    let lithe = &plugins[7];
    assert_eq!(lithe["entrypoint"]["kind"], "bundled");
    assert_eq!(lithe["themes"][0]["path"], "extensions/themes/lithe.json");
    assert_eq!(
        plugins
            .iter()
            .filter(|plugin| !plugin["themes"].as_array().expect("themes").is_empty())
            .count(),
        6,
        "六个主题组（IDEA 内置 + 五个可卸载）"
    );
    let java = &plugins[6];
    assert_eq!(java["displayName"], "Java 语言支持");
    assert_eq!(java["languageSupports"][0]["id"], "java");
}

#[test]
fn list_catalog_rejects_mismatched_host_version() {
    let request = serde_json::json!({
        "id": "catalog",
        "command": "plugins.listCatalog",
        "payload": {"hostVersion": "9.9.9"}
    });
    let response: Value = serde_json::from_str(&crate::execute_json(
        &serde_json::to_string(&request).expect("catalog request should encode"),
    ))
    .expect("catalog response should be JSON");
    assert_eq!(response["ok"], false);
    assert_eq!(response["error"]["code"], "parse_failed");
}

#[test]
fn list_catalog_rejects_malformed_host_version() {
    let request = serde_json::json!({
        "id": "catalog",
        "command": "plugins.listCatalog",
        "payload": {"hostVersion": "not-a-version"}
    });
    let response: Value = serde_json::from_str(&crate::execute_json(
        &serde_json::to_string(&request).expect("catalog request should encode"),
    ))
    .expect("catalog response should be JSON");
    assert_eq!(response["ok"], false);
    assert_eq!(response["error"]["code"], "invalid_request");
}
