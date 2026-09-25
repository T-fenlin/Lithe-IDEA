//! 右侧「Spring」视图的**数据层**：Core 的 `spring.index` → 可渲染的端点 / 计数视图。
//!
//! ## 为什么用 Core 的索引而不是自己找注解
//!
//! `spring.index` 已经在 Core 里做了整套 Spring 语义索引
//! （契约 `shared/contracts/rust-core-api.md:1737-1758`）：读工作区 JSON 元数据与依赖 JAR 里的
//! `spring-configuration-metadata.json`、索引配置文档与 Java 源码，产出**确定性有序**的
//! `properties` / `values` / `propertyReferences` / `diagnostics` / `beans` / `injections` / `endpoints`。
//! 依赖元数据在 Rust 进程里缓存，`refreshDependencyMetadata` 只在"打开项目"时置真。
//! 所以这里**一行注解解析都不写**（那正是仓库"复用成熟上游"红线：
//! `scripts/verify-java-semantic-ownership.mjs` 把 `is_main_method` 一类 token 都列成了禁区）。
//!
//! ## 本层只做两件事
//!
//! 1. 把 `endpoints[]` 摊成能直接画的行（HTTP 方法集 / 路由 / 控制器 / 方法 / 源码位置）；
//! 2. 把其余集合**只报条数** —— 它们是"索引是否健康"的直观信号，
//!    而逐条渲染（属性补全、注入链、`@Value` 引用）需要各自的 UI，属于后续批次。
//!
//! 位置口径照契约：**相对路径 + 1 基行列**（不是 LSP 的 0 基），本层原样保留，不换算。

use std::path::Path;

use lithe_gpui_shared::core_json;
use serde_json::{Value, json};

/// 一条 Spring 端点（`SpringEndpointResponse`，`protocol/contracts.rs:931-941`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpringEndpointView {
    /// 稳定 id（Core 给的，用于去重与选中态）。
    pub id: String,
    /// 声明的 HTTP 方法集（契约：**保留精确集合**，所以是 `GET+POST` 这种多值情形也要能显示）。
    pub http_methods: Vec<String>,
    /// 路由（`route`）——用户最想看的那一列。
    pub route: String,
    /// 控制器类名。
    pub controller: String,
    /// 处理方法名。
    pub method: String,
    /// 源码相对路径。
    pub path: String,
    /// **1 基**行 / 列（契约口径，本层不换算成编辑器口径）。
    pub line: usize,
    pub column: usize,
}

impl SpringEndpointView {
    /// 显示用的一行文本（HTTP 方法 + 路由）。渲染层直接用它，避免各画各的。
    pub fn label(&self) -> String {
        if self.http_methods.is_empty() {
            self.route.clone()
        } else {
            format!("{} {}", self.http_methods.join("/"), self.route)
        }
    }
}

/// 一次索引的结论。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpringIndexView {
    /// 端点（顺序照 Core 给的确定性序，**不重排**）。
    pub endpoints: Vec<SpringEndpointView>,
    /// 配置属性条数。
    pub properties: usize,
    /// 配置值条数。
    pub values: usize,
    /// `@Value` 引用条数。
    pub property_references: usize,
    /// 依赖注入点条数。
    pub injections: usize,
    /// Bean 条数。
    pub beans: usize,
    /// 索引自带的诊断条数（例如元数据缺失）。
    pub diagnostics: usize,
}

impl SpringIndexView {
    /// 有没有任何 Spring 事实 —— 面板据此决定显示内容还是"未检测到"。
    pub fn is_empty(&self) -> bool {
        self.endpoints.is_empty() && self.beans == 0 && self.properties == 0
    }
}

/// 索引一个工作区。返回 `None` = Core 明确说这里没有 Spring 事实（响应为 `null`）。
pub fn index(root: &Path) -> Option<SpringIndexView> {
    // `refreshDependencyMetadata: true` 只在"打开项目"这条路径上置真（契约原话），
    // 所以本函数应该**每次打开面板调一次**，不要挂在每次按键上。
    let response = core_json(
        "spring.index",
        json!({ "root": root.to_string_lossy(), "refreshDependencyMetadata": true }),
    );
    match response {
        Ok(response) => {
            let view = response.as_ref().and_then(parse);
            match &view {
                Some(view) => println!(
                    "S1_SPRING index=ok endpoints={} beans={} properties={} values={} refs={} injections={} diagnostics={}",
                    view.endpoints.len(),
                    view.beans,
                    view.properties,
                    view.values,
                    view.property_references,
                    view.injections,
                    view.diagnostics
                ),
                None => println!("S1_SPRING index=none (no Spring facts)"),
            }
            view
        }
        Err(error) => {
            // 失败不静默也不 panic：面板回到空态，日志留下可追的原因。
            println!("S1_SPRING index=failed error={error}");
            None
        }
    }
}

/// `spring.index` 的响应 → 视图（纯函数，便于单测）。
///
/// 关键字段缺失时**不猜**：端点没有 `route` 就整条丢掉（画一条没有路由的端点没有意义）；
/// 计数缺了就是 0。Core 的字段名逐字见 `protocol/contracts.rs:931-941` 与 `:943+`。
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

/// 取一个可选字符串字段（空串当没有）。
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

    /// 空响应：`null` = 没有 Spring 事实（面板显示空态），不是错误。
    #[test]
    fn null_response_means_no_spring_facts() {
        assert!(parse(&Value::Null).is_none());
    }

    /// 只有计数、没有端点：视图仍然成立（索引健康度也要能显示），并报 `is_empty`。
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

    /// 端点：HTTP 方法集、路由、控制器、1 基行列原样落位；`label()` 拼出显示串。
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
        assert_eq!(endpoint.label(), "GET/POST /api/users");
        assert_eq!(endpoint.controller, "UserController");
        assert_eq!(endpoint.path, "src/main/java/demo/UserController.java");
        // 契约是**1 基**行列：本层不换算（换算是编辑器侧的事）。
        assert_eq!(endpoint.line, 42);
        assert_eq!(endpoint.column, 5);
    }

    /// 没有 `route` 的端点整条丢掉（画一条没有路由的端点没意义）；缺字段不 panic。
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
        // 没有 httpMethods 时不 panic，`label()` 退化成纯路由。
        assert_eq!(view.endpoints[0].label(), "/ok");
        assert_eq!(view.endpoints[0].id, "/ok", "缺 id 时用 route 兜底");
    }
}
