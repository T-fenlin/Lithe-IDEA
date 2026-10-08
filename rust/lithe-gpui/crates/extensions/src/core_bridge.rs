use crate::model::CatalogSnapshot;
use lithe_gpui_shared::{HOST_VERSION, core_json};

pub fn load_catalog() -> Result<CatalogSnapshot, String> {
    let data = core_json(
        "plugins.listCatalog",
        serde_json::json!({ "hostVersion": HOST_VERSION }),
    )
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "plugins.listCatalog 的响应缺少 data".to_string())?;
    CatalogSnapshot::from_response(data).map_err(|error| format!("插件目录响应无法解析：{error}"))
}

#[cfg(test)]
mod tests {
    use crate::{CATEGORIES, matches_category};

    #[test]
    fn load_catalog_matches_the_bundled_fixture_host_version() {
        let snapshot = crate::load_catalog().expect("bundled catalog should load");
        assert!(!snapshot.extensions.is_empty());
    }

    #[test]
    fn bundled_catalog_covers_every_filter_category() {
        let snapshot = crate::load_catalog().expect("bundled catalog should load");
        for category in CATEGORIES {
            assert!(
                snapshot
                    .extensions
                    .iter()
                    .any(|extension| matches_category(extension, Some(category))),
                "bundled catalog should cover the {category} category"
            );
        }
    }
}
