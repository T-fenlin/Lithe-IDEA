use std::path::Path;

use lithe_gpui_shared::core_json;
use serde_json::{Value, json};

use crate::spring_paths::collect_spring_source_paths;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpringEndpointView {
    pub id: String,
    pub http_methods: Vec<String>,
    pub route: String,
    pub controller: String,
    pub method: String,
    pub path: String,
    pub line: usize,
    pub column: usize,
}

impl SpringEndpointView {
    pub fn methods_label(&self) -> String {
        self.http_methods.join(",")
    }
}

pub fn filter_endpoints<'a>(
    endpoints: &'a [SpringEndpointView],
    query: &str,
) -> Vec<&'a SpringEndpointView> {
    let query = query.trim();
    if query.is_empty() {
        return endpoints.iter().collect();
    }
    let needle = query.to_lowercase();
    endpoints
        .iter()
        .filter(|endpoint| {
            let methods = endpoint.http_methods.join(" ");
            [
                &endpoint.route,
                &endpoint.controller,
                &endpoint.method,
                &methods,
            ]
            .iter()
            .any(|field| field.to_lowercase().contains(&needle))
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpringIndexView {
    pub endpoints: Vec<SpringEndpointView>,
    pub properties: usize,
    pub values: usize,
    pub property_references: usize,
    pub injections: usize,
    pub beans: usize,
    pub diagnostics: usize,
}

impl SpringIndexView {
    pub fn is_empty(&self) -> bool {
        self.endpoints.is_empty() && self.beans == 0 && self.properties == 0
    }
}

pub fn index(root: &Path) -> Option<SpringIndexView> {
    let collected = collect_spring_source_paths(root);
    let path_count = collected.paths.len();
    let truncated = collected.truncated;

    let response = core_json(
        "spring.index",
        json!({
            "root": root.to_string_lossy(),
            "paths": collected.paths,
            "refreshDependencyMetadata": true,
        }),
    );
    match response {
        Ok(response) => {
            let view = response.as_ref().and_then(parse);
            match &view {
                Some(view) => println!(
                    "S1_SPRING index=ok endpoints={} beans={} properties={} values={} refs={} injections={} diagnostics={} paths={path_count} truncated={truncated}",
                    view.endpoints.len(),
                    view.beans,
                    view.properties,
                    view.values,
                    view.property_references,
                    view.injections,
                    view.diagnostics
                ),
                None => println!(
                    "S1_SPRING index=none paths={path_count} truncated={truncated} (no Spring facts)"
                ),
            }
            view
        }
        Err(error) => {
            println!(
                "S1_SPRING index=failed paths={path_count} truncated={truncated} error={error}"
            );
            None
        }
    }
}

pub(crate) fn parse(response: &Value) -> Option<SpringIndexView> {
    if response.is_null() {
        return None;
    }
    let array = |key: &str| -> Vec<Value> {
        response
            .get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };

    let endpoints: Vec<SpringEndpointView> = array("endpoints")
        .iter()
        .filter_map(|endpoint| {
            let route = endpoint.get("route").and_then(Value::as_str)?;
            Some(SpringEndpointView {
                id: endpoint
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or(route)
                    .to_string(),
                http_methods: endpoint
                    .get("httpMethods")
                    .and_then(Value::as_array)
                    .map(|methods| {
                        methods
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default(),
                route: route.to_string(),
                controller: text(endpoint, "controller").unwrap_or_default(),
                method: text(endpoint, "method").unwrap_or_default(),
                path: text(endpoint, "path").unwrap_or_default(),
                line: endpoint.get("line").and_then(Value::as_u64).unwrap_or(0) as usize,
                column: endpoint.get("column").and_then(Value::as_u64).unwrap_or(0) as usize,
            })
        })
        .collect();

    Some(SpringIndexView {
        endpoints,
        properties: array("properties").len(),
        values: array("values").len(),
        property_references: array("propertyReferences").len(),
        injections: array("injections").len(),
        beans: array("beans").len(),
        diagnostics: array("diagnostics").len(),
    })
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::spring_paths::collect_spring_source_paths;

    #[test]
    fn null_response_means_no_spring_facts() {
        assert!(parse(&Value::Null).is_none());
    }

    #[test]
    fn counts_only_response_is_still_a_view() {
        let view = parse(&json!({
            "properties": [{ "name": "server.port" }],
            "values": [],
            "propertyReferences": [],
            "injections": [],
            "beans": [],
            "diagnostics": [{ "message": "x" }]
        }))
        .expect("非 null 即有视图");
        assert_eq!(view.properties, 1);
        assert_eq!(view.diagnostics, 1);
        assert!(view.endpoints.is_empty());
        assert!(!view.is_empty(), "有 properties 就不算空");
    }

    #[test]
    fn endpoints_keep_methods_route_and_location() {
        let view = parse(&json!({
            "endpoints": [{
                "id": "ep-1",
                "httpMethods": ["GET", "POST"],
                "route": "/api/users",
                "controller": "UserController",
                "method": "users",
                "path": "src/main/java/demo/UserController.java",
                "line": 42,
                "column": 5
            }]
        }))
        .expect("有 endpoints 就是视图");

        let endpoint = &view.endpoints[0];
        assert_eq!(endpoint.methods_label(), "GET,POST");
        assert_eq!(endpoint.route, "/api/users");
        assert_eq!(endpoint.controller, "UserController");
        assert_eq!(endpoint.path, "src/main/java/demo/UserController.java");
        assert_eq!(endpoint.line, 42);
        assert_eq!(endpoint.column, 5);
    }

    #[test]
    fn endpoints_without_a_route_are_dropped() {
        let view = parse(&json!({
            "endpoints": [
                { "id": "a", "httpMethods": ["GET"] },
                { "route": "/ok" }
            ]
        }))
        .expect("视图");
        assert_eq!(view.endpoints.len(), 1);
        assert_eq!(view.endpoints[0].route, "/ok");
        assert_eq!(view.endpoints[0].methods_label(), "");
        assert_eq!(view.endpoints[0].id, "/ok", "缺 id 时用 route 兜底");
    }

    #[test]
    fn filtering_matches_route_controller_method_or_methods() {
        let view = parse(&json!({
            "endpoints": [
                { "route": "/api/users", "controller": "UserController", "method": "list", "httpMethods": ["GET"] },
                { "route": "/api/orders", "controller": "OrderController", "method": "create", "httpMethods": ["POST"] },
            ]
        }))
        .expect("视图");
        let endpoints = &view.endpoints;

        assert_eq!(filter_endpoints(endpoints, "").len(), 2);
        assert_eq!(filter_endpoints(endpoints, "   ").len(), 2);
        assert_eq!(
            filter_endpoints(endpoints, "orders")[0].route,
            "/api/orders"
        );
        assert_eq!(filter_endpoints(endpoints, "usercontroller").len(), 1);
        assert_eq!(filter_endpoints(endpoints, "CREATE").len(), 1);
        assert_eq!(filter_endpoints(endpoints, "post").len(), 1);
        assert!(filter_endpoints(endpoints, "nonexistent").is_empty());
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "lithe-gpui-spring-index-{}-{label}",
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("临时工作区根");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn an_empty_workspace_still_sends_the_request_and_keeps_a_view() {
        let root = TempDir::new("empty-workspace");

        assert!(
            collect_spring_source_paths(root.path()).paths.is_empty(),
            "空工作区应当收集到 0 条路径"
        );

        let view = index(root.path()).expect("空工作区也有内置属性，响应不该是 null");
        assert!(
            !view.is_empty(),
            "内置属性让视图有权重：面板该显示内容而不是空态"
        );
        assert!(view.endpoints.is_empty(), "空工作区不可能有端点");
    }

    const SMOKE_ENV: &str = "LITHE_GPUI_SPRING_SMOKE";

    const FIXTURE_CANDIDATES: [&str; 2] =
        [".artifacts/p15/spring-fixture", ".artifacts/p14/fixture"];

    fn find_fixture_root() -> Option<PathBuf> {
        let mut anchors = Vec::new();
        if let Ok(current) = std::env::current_dir() {
            anchors.push(current);
        }
        if let Ok(executable) = std::env::current_exe()
            && let Some(parent) = executable.parent()
        {
            anchors.push(parent.to_path_buf());
        }
        for anchor in anchors {
            for ancestor in anchor.ancestors().take(8) {
                for candidate in FIXTURE_CANDIDATES {
                    let root = ancestor.join(candidate.replace('/', std::path::MAIN_SEPARATOR_STR));
                    if collect_spring_source_paths(&root)
                        .paths
                        .iter()
                        .any(|path| path.ends_with(".java"))
                    {
                        return Some(root);
                    }
                }
            }
        }
        None
    }

    #[test]
    fn real_core_round_trip_indexes_endpoints_from_a_fixture() {
        if std::env::var_os(SMOKE_ENV).is_none_or(|value| value.is_empty()) {
            return;
        }
        let Some(root) = find_fixture_root() else {
            println!("S1_SPRING smoke=skipped (no fixture under .artifacts)");
            return;
        };
        let collected = collect_spring_source_paths(&root);
        println!(
            "S1_SPRING smoke=fixture root={} paths={} truncated={}",
            root.display(),
            collected.paths.len(),
            collected.truncated
        );
        assert!(
            collected.paths.iter().any(|path| path.ends_with(".java")),
            "夹具里必须收集到至少一个 .java：{:?}",
            collected.paths
        );

        let legacy = core_json(
            "spring.index",
            json!({
                "root": root.to_string_lossy(),
                "refreshDependencyMetadata": true,
            }),
        )
        .expect("旧 payload 本身也是合法请求");
        let legacy_endpoints = legacy
            .as_ref()
            .and_then(parse)
            .map_or(0, |view| view.endpoints.len());
        println!("S1_SPRING_BEFORE payload=root-only endpoints={legacy_endpoints}");

        let view = index(&root).expect("夹具里有 Spring 事实，响应不该是 null");
        println!(
            "S1_SPRING_AFTER endpoints={} paths={}",
            view.endpoints.len(),
            collected.paths.len()
        );

        assert_eq!(
            legacy_endpoints, 0,
            "不带 `paths` 时 Core 不扫任何源码：这正是被修的 bug，不能被重新引入"
        );
        assert!(
            !view.endpoints.is_empty(),
            "带上 `paths` 之后必须扫出端点，否则本 bug 没修好"
        );
        assert!(
            view.endpoints
                .iter()
                .any(|endpoint| endpoint.route.contains("/api/users")),
            "端点里应当有夹具声明的路由：{:?}",
            view.endpoints
        );
    }
}
