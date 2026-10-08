use std::path::Path;

use lithe_gpui_shared::core_json;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MavenSourceRootView {
    pub path: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MavenModuleView {
    pub relative_path: String,
    pub group_id: Option<String>,
    pub artifact_id: String,
    pub version: Option<String>,
    pub packaging: String,
    pub source_roots: Vec<MavenSourceRootView>,
    pub modules: Vec<MavenModuleView>,
}

impl MavenModuleView {
    pub fn label(&self) -> &str {
        if self.artifact_id.is_empty() {
            &self.relative_path
        } else {
            &self.artifact_id
        }
    }

    pub fn total(&self) -> usize {
        1 + self.modules.iter().map(Self::total).sum::<usize>()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MavenProjectView {
    pub relative_path: String,
    pub group_id: Option<String>,
    pub artifact_id: String,
    pub version: Option<String>,
    pub packaging: String,
    pub profiles: Vec<String>,
    pub active_profiles: Vec<String>,
    pub modules: Vec<MavenModuleView>,
}

impl MavenProjectView {
    pub fn module_count(&self) -> usize {
        1 + self
            .modules
            .iter()
            .map(MavenModuleView::total)
            .sum::<usize>()
    }
}

pub fn scan(root: &Path) -> Option<MavenProjectView> {
    match core_json(
        "maven.scan",
        serde_json::json!({ "root": root.to_string_lossy() }),
    ) {
        Ok(scan) => {
            let project = scan.as_ref().and_then(parse);
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
            println!("S1_MAVEN scan=failed error={error}");
            None
        }
    }
}

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

fn modules(value: Option<&Value>) -> Vec<MavenModuleView> {
    let Some(items) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|module| {
            Some(MavenModuleView {
                relative_path: module
                    .get("relativePath")
                    .and_then(Value::as_str)?
                    .to_string(),
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

    #[test]
    fn null_scan_means_no_project() {
        assert!(parse(&Value::Null).is_none());
        assert!(parse(&json!({ "artifactId": "demo" })).is_none());
    }

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
        assert_eq!(view.packaging, "jar");
    }

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
        assert_eq!(roots[1].kind, "generatedMain");
        assert!(parse(&json!({ "relativePath": ".", "modules": [{ "relativePath": "s", "artifactId": "s" }] }))
            .expect("reactor")
            .modules[0]
            .source_roots
            .is_empty());
    }

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
