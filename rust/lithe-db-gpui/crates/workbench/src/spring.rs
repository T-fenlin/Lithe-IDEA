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
//! ## 本层只做三件事
//!
//! 1. **收集要索引哪些文件**（[`crate::spring_paths`]）—— `spring.index` 只索引请求里
//!    `paths` 给出的路径，`paths` 缺省是空 `Vec`，所以"扫哪些文件"必须由本层给出；
//! 2. 把 `endpoints[]` 摊成能直接画的行（HTTP 方法集 / 路由 / 控制器 / 方法 / 源码位置）；
//! 3. 把其余集合**只报条数** —— 它们是"索引是否健康"的直观信号，
//!    而逐条渲染（属性补全、注入链、`@Value` 引用）需要各自的 UI，属于后续批次。
//!
//! 位置口径照契约：**相对路径 + 1 基行列**（不是 LSP 的 0 基），本层原样保留，不换算。

use std::path::Path;

use lithe_db_gpui_shared::core_json;
use serde_json::{Value, json};

use crate::spring_paths::collect_spring_source_paths;

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
    // **`paths` 必须显式给出**：`spring.index` 只索引请求里 `paths` 列出的工作区相对路径
    // （`rust/lithe-core/src/languages/spring.rs:82-89` 取 `.java` 进 `endpoint_index`，
    // `:93-111` 取 `application*` 进 `values`），而 `paths` 是 `#[serde(default)] Vec<String>`
    // （同文件 `:36-51`）：**不传 = 一个文件都不扫**，响应里只剩 `built_in_properties()`
    // （`:324-386`）那批内置属性，于是诊断行显示成 `endpoints=0 properties=10`——
    // "索引成功"却什么都没扫。收集口径见 [`crate::spring_paths`]（对齐 Windows 参考实现）。
    let collected = collect_spring_source_paths(root);
    let path_count = collected.paths.len();
    let truncated = collected.truncated;

    // `refreshDependencyMetadata: true` 只在"打开项目"这条路径上置真（契约原话），
    // 所以本函数应该**每次打开面板调一次**，不要挂在每次按键上。
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
            // 失败不静默也不 panic：面板回到空态，日志留下可追的原因。
            // `paths` 一起打出来：失败时"到底有没有收集到输入"是第一个要排除的变量
            // （工作区为空与 Core 报错长得完全不同，但两者都会让面板空掉）。
            println!(
                "S1_SPRING index=failed paths={path_count} truncated={truncated} error={error}"
            );
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
    use std::path::PathBuf;

    use crate::spring_paths::collect_spring_source_paths;

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

    /// 级联临时目录：断言失败也会在 `Drop` 里删掉，不留工作区残骸。
    ///
    /// 名字里的进程号 + 建目录前先删一次，让上次崩溃留下的同名目录不会污染本次结果。
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(label: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "lithe-db-gpui-spring-index-{}-{label}",
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

    /// 空工作区：`paths` 收集结果是空，但**请求仍然要发**。
    ///
    /// 为什么这条必须守：`paths: []` 时 Core 依然会回 `built_in_properties()`
    /// （`rust/lithe-core/src/languages/spring.rs:324-386`）那批内置属性，于是响应非 `null`、
    /// 面板拿到"有权重"的视图（`is_empty() == false`）。如果哪天为了"省一次调用"
    /// 在空 `paths` 时提前 return `None`，面板就会从"显示内置属性"静默退化成
    /// "未检测到 Spring 组件与端点" —— 那是与本次修复反向的回归。
    ///
    /// 这条是**真实往返**（进程内调 `spring.index`），但没有外部依赖：
    /// 只用一个空的临时目录，不扫任何工作区源码，所以是例行测试而不是 opt-in。
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

    // -----------------------------------------------------------------------
    // 选定式（opt-in）真实往返：`spring::index` → `spring.index` → Core
    // -----------------------------------------------------------------------

    /// 开关环境变量。设了它才会跑真实夹具 —— 与 `lithe-db-gpui-java` 的
    /// `LITHE_GPUI_JDTLS_SMOKE`（`crates/java/src/service.rs:839`）同一约定：
    /// 需要机器上的本地夹具，CI 与例行 `cargo test` 上不跑。
    ///
    /// 与 JDTLS 那条不同的是：这条**不 spawn 任何 java/JDTLS 进程**，
    /// Core 的 `spring.index` 是纯正则 + 文本的进程内调用（`endpoint_index`），
    /// 所以没有需要 watchdog 的子进程，退出时也没有残留进程要清。
    const SMOKE_ENV: &str = "LITHE_GPUI_SPRING_SMOKE";

    /// 真实夹具的**仓库相对**候选路径，按优先级排列。
    ///
    /// 夹具由本轮的验证批次生成、位于不入库的 `.artifacts/`，所以路径只允许出现在这条
    /// opt-in 测试里（不能进例行单测：那是"机器相关硬编码"），而且**找不到就跳过、不失败**。
    const FIXTURE_CANDIDATES: [&str; 2] =
        [".artifacts/p15/spring-fixture", ".artifacts/p14/fixture"];

    /// 从当前目录与测试可执行文件所在目录向上找仓库根（各最多 8 层），
    /// 返回第一个"确实能收集到 `.java`"的候选夹具目录。
    ///
    /// 走两个锚点是因为 `cargo test` 的工作目录是包根
    /// （`lithe-db-gpui/crates/workbench`），而直接跑测试二进制时不是；两个锚点各向上找，
    /// 都不用把仓库根写死。
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
                    // 候选写成 `/` 分隔的仓库相对路径（可读、跨平台一致），拼到锚点上时换成
                    // 平台分隔符，免得诊断行里出现 `….artifacts/p15/spring-fixture` 这种混用形式。
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

    /// 本 bug 的验收点：同一个夹具，**不带 `paths`** 与 **带 `paths`** 的对照。
    ///
    /// ① 旧 payload（只有 `root` + `refreshDependencyMetadata`）必须给出 `endpoints == 0`：
    ///    这是把"`paths` 才是扫源码的开关"钉成回归断言 —— 一旦哪天又有人把 `paths` 从
    ///    payload 里去掉，这条测试会直接失败，而不是等到实机截图才发现端点恒为 0；
    /// ② 修复后的 `spring::index` 必须给出 `endpoints > 0`，且端点里能看到夹具的路由。
    ///
    /// 断言用 `.contains("/api/users")` 而不是全等：两个夹具（p15 / p14）的控制器方法名不同，
    /// 但它们都声明了 `@GetMapping("/api/users")` + `@PostMapping("/api/users")`。
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

        // ① 复现旧 payload（就是修复前的 `index()`）：只给 root。
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

        // ② 修复后的路径（`index()` 自己会打 `S1_SPRING index=ok … paths=… endpoints=…`）。
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
