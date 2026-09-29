//! `plugins.listCatalog` 的 Core 桥：拼请求、解信封、归一错误。

use crate::model::CatalogSnapshot;
use lithe_gpui_shared::{HOST_VERSION, core_json};

/// 拉取内置扩展目录的展示快照。
///
/// **同步**：Core 调用是进程内的确定性执行，但仍然要走完整信封；调用方必须
/// 放在后台任务里（GPUI 的 UI 线程不做阻塞工作）。失败以字符串返回——目录是
/// 打包数据，失败只可能是宿主版本不匹配或打包损坏，展示层如实呈现即可。
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
        // 官方目录的 hostVersion 是 0.0.1；如果哪天升版本，这里会先在测试里失败，
        // 提醒同步改 fixture，而不是等到运行时才出现 parse_failed。
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
