//! 右侧工具窗「Maven」的**数据层**：`maven.scan` 的结论 → 一份可渲染的模块 / profile 视图。
//!
//! ## 为什么放在 workbench 而不是 `lithe-gpui-java`
//!
//! `maven.scan` 是 **Core 的项目命令**（契约 `shared/contracts/rust-core-api.md:107,1450+`），
//! 不是 LSP 会话的事实。`lithe-gpui-java` 拥有的是 JDTLS 会话（它为了 `mavenContext`
//! 也调过一次 `maven.scan`），而"右侧面板要显示什么"是**外壳的表现层问题** ——
//! 所以这一层归 workbench，用 `lithe_gpui_shared::core_json` 直接问 Core，
//! **不自己解析 `pom.xml`**（那是上游 m2e / `maven.scan` 的活）。
//!
//! ## 与真源的关系
//!
//! 真源 `features/maven/components/maven-pane.tsx` 的面板有工具栏、模块树、生命周期、
//! 依赖树、Profiles、构建输出六块；本侧**只做"有没有 Maven 项目 + 模块与 profile 一览"**，
//! 因为其余几块需要 `mvn` 执行与依赖解析的数据源（那属于下一批）。
//! 空态文案沿用真源的 `maven.notDetected`（「未检测到 Maven 项目」），
//! 判据从"写死的空态"改成 **`maven.scan` 返回 `null`**（那才是"没有可读 pom.xml"的权威答案）。

use std::path::Path;

use lithe_gpui_shared::core_json;
use serde_json::Value;

/// 一条源码根（`maven.scan` 的 `sourceRoots[]`，契约 `protocol/contracts.rs:198-205`）。
///
/// `kind` 是语义分档（main / test / generated…），原样保留：面板按它区分显示，
/// **不在这里翻译成文案**（渲染层管文案，数据层只管事实）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MavenSourceRootView {
    /// 模块相对路径（`/` 分隔）。
    pub path: String,
    /// 语义分档（camelCase 字符串，照 Core 给的原样）。
    pub kind: String,
}

/// 一个模块（`maven.scan` 的 `modules[]`，契约 `protocol/contracts.rs:185-195`）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MavenModuleView {
    /// 相对工作区根的路径（`/` 分隔，`.` = 根）。
    pub relative_path: String,
    /// `groupId` 缺失时给 `None`（不要编一个空串冒充有值）。
    pub group_id: Option<String>,
    pub artifact_id: String,
    pub version: Option<String>,
    pub packaging: String,
    /// 本模块的源码根（Maven 模型给的事实；**我们不猜目录**）。
    pub source_roots: Vec<MavenSourceRootView>,
    /// 子模块（**层级保留**：真机是模块树，扁平化会丢信息）。
    pub modules: Vec<MavenModuleView>,
}

impl MavenModuleView {
    /// 显示名：`artifactId` 为空时回落 `relativePath`（**渲染层不该判空**）。
    pub fn label(&self) -> &str {
        if self.artifact_id.is_empty() {
            &self.relative_path
        } else {
            &self.artifact_id
        }
    }

    /// 本模块 + 全部子孙模块的条数（面板头部要显示"几个模块"）。
    pub fn total(&self) -> usize {
        1 + self.modules.iter().map(Self::total).sum::<usize>()
    }
}

/// 一次扫描的结论（够右侧面板画出来）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MavenProjectView {
    /// reactor 根相对工作区根的路径（`.` = 工作区根就是 reactor）。
    pub relative_path: String,
    pub group_id: Option<String>,
    pub artifact_id: String,
    pub version: Option<String>,
    pub packaging: String,
    /// profile 的 id（顺序照 Core 给的稳定序）。
    pub profiles: Vec<String>,
    /// 默认激活的 profile（真机在 Profiles 区给它们打标）。
    pub active_profiles: Vec<String>,
    /// reactor 的子模块（递归）。
    pub modules: Vec<MavenModuleView>,
}

impl MavenProjectView {
    /// 整个 reactor 的模块总数（含根）。
    pub fn module_count(&self) -> usize {
        1 + self.modules.iter().map(MavenModuleView::total).sum::<usize>()
    }
}

/// 扫一个工作区；`None` = 没有可读的 `pom.xml`（`maven.scan` 的权威答案）。
pub fn scan(root: &Path) -> Option<MavenProjectView> {
    match core_json("maven.scan", serde_json::json!({ "root": root.to_string_lossy() })) {
        Ok(scan) => {
            let project = scan.as_ref().and_then(parse);
            // 诊断：面板里显示的每一个事实都能在这行里核对（可 grep）。
            match &project {
                Some(project) => println!(
                    "S1_MAVEN scan=ok artifactId={} version={} reactorPath={} modules={} profiles={}",
                    project.artifact_id,
                    project.version.as_deref().unwrap_or("-"),
                    project.relative_path,
                    project.module_count(),
                    project.profiles.len()
                ),
                None => println!("S1_MAVEN scan=none (no readable pom.xml)"),
            }
            project
        }
        Err(error) => {
            // 失败不静默，但不 panic：面板会回到"未检测到 Maven 项目"，
            // 日志里留下可追的原因。
            println!("S1_MAVEN scan=failed error={error}");
            None
        }
    }
}

/// `maven.scan` 的响应 → 视图（纯函数，便于单测）。
///
/// 只认**有证据**的字段：没有 `artifact_id` / `relative_path` 的响应视为"不是项目"
/// （契约：解析不出来时返回 `null`，所以走到这里的都该有这两个字段）。
pub(crate) fn parse(scan: &Value) -> Option<MavenProjectView> {
    if scan.is_null() {
        return None;
    }
    let relative_path = scan.get("relativePath").and_then(Value::as_str)?;
    let artifact_id = scan
        .get("artifactId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    let profiles: Vec<String> = scan
        .get("profiles")
        .and_then(Value::as_array)
        .map(|profiles| {
            profiles
                .iter()
                .filter_map(|profile| profile.get("id").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    // `activeByDefault` 是真机的标记依据（`MavenProfileResponse.is_active_by_default`）。
    let active_profiles: Vec<String> = scan
        .get("profiles")
        .and_then(Value::as_array)
        .map(|profiles| {
            profiles
                .iter()
                .filter(|profile| {
                    profile
                        .get("isActiveByDefault")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .filter_map(|profile| profile.get("id").and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    Some(MavenProjectView {
        relative_path: relative_path.to_string(),
        group_id: text(scan, "groupId"),
        artifact_id,
        version: text(scan, "version"),
        packaging: text(scan, "packaging").unwrap_or_else(|| "jar".to_string()),
        profiles,
        active_profiles,
        modules: modules(scan.get("modules")),
    })
}

/// 递归解析模块表（顺序照 Core 给的，**不重排**：契约保证稳定序）。
fn modules(value: Option<&Value>) -> Vec<MavenModuleView> {
    let Some(items) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|module| {
            Some(MavenModuleView {
                relative_path: module.get("relativePath").and_then(Value::as_str)?.to_string(),
                group_id: text(module, "groupId"),
                artifact_id: module
                    .get("artifactId")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                version: text(module, "version"),
                packaging: text(module, "packaging").unwrap_or_else(|| "jar".to_string()),
                source_roots: source_roots(module.get("sourceRoots")),
                modules: modules(module.get("modules")),
            })
        })
        .collect()
}

/// 解析源码根表（顺序照 Core 给的，不重排）。
fn source_roots(value: Option<&Value>) -> Vec<MavenSourceRootView> {
    let Some(items) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|root| {
            Some(MavenSourceRootView {
                path: root.get("path").and_then(Value::as_str)?.to_string(),
                kind: root
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("main")
                    .to_string(),
            })
        })
        .collect()
}

/// 取一个可选字符串字段（空串当没有：契约里空串不是合法 id）。
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
    use serde_json::json;

    /// 没有项目：`maven.scan` 的 `null` 就是权威答案（面板据此显示真源的「未检测到 Maven 项目」）。
    #[test]
    fn null_scan_means_no_project() {
        assert!(parse(&Value::Null).is_none());
        // 缺 relativePath 的响应也不当项目（契约里"解析不出来"就是 null，走到这里说明形状不对）。
        assert!(parse(&json!({ "artifactId": "demo" })).is_none());
    }

    /// 根 pom：字段逐个落位，profile 区分"列出"与"默认激活"。
    #[test]
    fn root_project_maps_every_field() {
        let view = parse(&json!({
            "relativePath": ".",
            "groupId": "demo",
            "artifactId": "lite-fixture",
            "version": "1.0.0",
            "packaging": "jar",
            "profiles": [
                { "id": "dev", "isActiveByDefault": true },
                { "id": "prod", "isActiveByDefault": false }
            ],
            "modules": []
        }))
        .expect("有 relativePath 就是项目");

        assert_eq!(view.relative_path, ".");
        assert_eq!(view.group_id.as_deref(), Some("demo"));
        assert_eq!(view.artifact_id, "lite-fixture");
        assert_eq!(view.profiles, vec!["dev".to_string(), "prod".to_string()]);
        assert_eq!(view.active_profiles, vec!["dev".to_string()]);
        assert_eq!(view.module_count(), 1, "只有 reactor 根");
    }

    /// 递归模块：层级要保留，计数含子孙；缺 groupId 时给 `None` 而不是空串。
    #[test]
    fn nested_modules_keep_the_hierarchy() {
        let view = parse(&json!({
            "relativePath": "backend",
            "artifactId": "backend",
            "modules": [{
                "relativePath": "backend/api",
                "artifactId": "api",
                "packaging": "jar",
                "modules": [{
                    "relativePath": "backend/api/model",
                    "artifactId": "model",
                    "packaging": "jar",
                    "modules": []
                }]
            }]
        }))
        .expect("reactor");

        assert_eq!(view.module_count(), 3, "backend + api + model");
        assert_eq!(view.modules.len(), 1);
        assert_eq!(view.modules[0].label(), "api");
        assert_eq!(view.modules[0].group_id, None);
        assert_eq!(view.modules[0].modules[0].label(), "model");
        // packaging 缺省是 jar（契约里 packaging 必有；缺了也不该崩）。
        assert_eq!(view.packaging, "jar");
    }

    /// 源码根：来自 Maven 模型的权威事实（`sourceRoots[]`），面板直接显示它们，
    /// **不猜目录、不扫文件系统**；`kind` 原样保留（main / test / generated…）。
    #[test]
    fn module_source_roots_come_from_the_model() {
        let view = parse(&json!({
            "relativePath": ".",
            "artifactId": "backend",
            "modules": [{
                "relativePath": "backend/api",
                "artifactId": "api",
                "packaging": "jar",
                "sourceRoots": [
                    { "path": "src/main/java", "kind": "main" },
                    { "path": "target/generated-sources/annotations", "kind": "generatedMain" }
                ],
                "modules": []
            }]
        }))
        .expect("reactor");
        let roots = &view.modules[0].source_roots;
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].path, "src/main/java");
        assert_eq!(roots[0].kind, "main");
        // 生成源根也留着 —— 它正是"注解处理器产出的代码能不能被补全看到"的那个目录。
        assert_eq!(roots[1].kind, "generatedMain");
        // 没有 sourceRoots 的模块给空表（不 panic、不编造）。
        assert!(parse(&json!({ "relativePath": ".", "modules": [{ "relativePath": "s", "artifactId": "s" }] }))
            .expect("reactor")
            .modules[0]
            .source_roots
            .is_empty());
    }

    /// 显示名：`artifactId` 为空时回落 `relativePath`（渲染层不判空）。
    #[test]
    fn label_falls_back_to_the_relative_path() {
        let view = parse(&json!({
            "relativePath": ".",
            "artifactId": "",
            "modules": [{ "relativePath": "sub", "artifactiId": "typo" }]
        }))
        .expect("reactor");
        assert_eq!(view.modules[0].label(), "sub");
    }
}
