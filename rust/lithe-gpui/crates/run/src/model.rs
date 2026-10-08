use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use lithe_gpui_shared::workspace_config::WorkspaceConfigPaths;
use lithe_gpui_shared::{core_json, previous_object, save_document};
use serde_json::Value;

const RUN_DOCUMENT_VERSION: u64 = 2;

pub const RUN_DIAGNOSTIC_TAG: &str = "S1_SETTINGS_RUN";

pub const MAX_PATHS: usize = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunLayer {
    Generated,
    Project,
    Local,
}

impl RunLayer {
    pub fn id(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::Project => "project",
            Self::Local => "local",
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Generated => "lithe.run.source.generated",
            Self::Project => "lithe.run.source.project",
            Self::Local => "lithe.run.source.local",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "project" => Self::Project,
            "local" => Self::Local,
            _ => Self::Generated,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunExecution {
    Application,
    Service,
    Task,
    Group,
}

impl RunExecution {
    pub fn id(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::Service => "service",
            Self::Task => "task",
            Self::Group => "group",
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Self::Application => "lithe.run.applications",
            Self::Service => "lithe.run.services",
            Self::Task => "lithe.run.tasks",
            Self::Group => "lithe.run.groups",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "service" => Self::Service,
            "task" => Self::Task,
            "group" => Self::Group,
            _ => Self::Application,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfigurationView {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub provider_title: String,
    pub execution: RunExecution,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub cwd: String,
    pub layer: RunLayer,
    pub manifest: Option<String>,
    pub main_class: Option<String>,
}

impl RunConfigurationView {
    pub fn command_line(&self) -> Option<String> {
        let command = self
            .command
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())?;
        if self.args.is_empty() {
            Some(command.to_string())
        } else {
            Some(format!("{command} {}", self.args.join(" ")))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunProjectView {
    pub configurations: Vec<RunConfigurationView>,
    pub entry_count: usize,
    pub java_entrypoints_origin: Option<String>,
}

impl RunProjectView {
    pub fn layer_counts(&self) -> BTreeMap<&'static str, usize> {
        let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        for configuration in &self.configurations {
            *counts.entry(configuration.layer.id()).or_default() += 1;
        }
        counts
    }

    pub fn provider_counts(&self) -> BTreeMap<String, usize> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for configuration in &self.configurations {
            *counts.entry(configuration.provider.clone()).or_default() += 1;
        }
        counts
    }

    pub fn diagnostic_line(
        &self,
        root: &Path,
        paths: usize,
        truncated: bool,
        elapsed_ms: u128,
    ) -> String {
        let sources = join_counts(self.layer_counts().into_iter());
        let providers = join_counts(self.provider_counts().into_iter());
        format!(
            "root={} paths={paths} truncated={truncated} configs={} entryCount={} sources={sources} providers={providers} origin={} ms={elapsed_ms}",
            root.display(),
            self.configurations.len(),
            self.entry_count,
            self.java_entrypoints_origin.as_deref().unwrap_or("-"),
        )
    }
}

fn join_counts<K: AsRef<str>>(entries: impl Iterator<Item = (K, usize)>) -> String {
    let joined = entries
        .map(|(key, count)| format!("{}:{count}", key.as_ref()))
        .collect::<Vec<_>>()
        .join(",");
    if joined.is_empty() {
        "-".to_string()
    } else {
        joined
    }
}

pub fn provider_title(provider: &str) -> String {
    const FRAMEWORK_TITLES: &[(&str, &str)] = &[
        ("spring-boot.maven", "Spring Boot"),
        ("quarkus.maven", "Quarkus"),
        ("micronaut.maven", "Micronaut"),
        ("java.main", "Java Application"),
        ("java.current-file", "Current File"),
        ("maven.module", "Maven Module"),
    ];
    if let Some((_, title)) = FRAMEWORK_TITLES.iter().find(|(id, _)| *id == provider) {
        return (*title).to_string();
    }
    let namespace = provider.split('.').next().unwrap_or(provider);
    let mut characters = namespace.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => provider.to_string(),
    }
}

fn split_source(source: Option<&str>) -> (RunLayer, Option<String>) {
    let Some(source) = source.map(str::trim).filter(|value| !value.is_empty()) else {
        return (RunLayer::Generated, None);
    };
    match source {
        "generated" | "project" | "local" => (RunLayer::from_id(source), None),
        manifest => (RunLayer::Generated, Some(manifest.to_string())),
    }
}

pub fn parse_snapshot(data: Option<&Value>) -> Result<(Vec<String>, bool), String> {
    let files = data
        .and_then(|data| data.get("files"))
        .and_then(Value::as_array)
        .ok_or_else(|| "响应的 data.files 不是数组".to_string())?;
    let truncated = files.len() > MAX_PATHS;
    Ok((
        files
            .iter()
            .filter_map(Value::as_str)
            .take(MAX_PATHS)
            .map(str::to_string)
            .collect(),
        truncated,
    ))
}

pub fn parse_generate(data: Option<&Value>) -> Result<RunProjectView, String> {
    let data = data.ok_or_else(|| "响应的 data 缺失".to_string())?;
    let generated = data
        .get("generated")
        .filter(|value| value.is_object())
        .ok_or_else(|| "响应的 data.generated 缺失".to_string())?;
    let items = generated
        .get("configurations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut configurations = Vec::with_capacity(items.len());
    for item in &items {
        let provider = text(item, "provider").unwrap_or_default();
        let raw_id = text(item, "id");
        let raw_name = text(item, "name");
        let id = raw_id
            .clone()
            .or(raw_name.clone())
            .unwrap_or_else(|| provider.clone());
        let name = raw_name.or(raw_id).unwrap_or_else(|| id.clone());
        if id.is_empty() {
            continue;
        }
        let (layer, manifest) = split_source(item.get("source").and_then(Value::as_str));
        configurations.push(RunConfigurationView {
            id,
            name,
            provider_title: provider_title(&provider),
            provider,
            execution: RunExecution::from_id(
                item.get("execution")
                    .and_then(Value::as_str)
                    .unwrap_or("application"),
            ),
            command: text(item, "command"),
            args: strings(item.get("args")),
            cwd: text(item, "cwd").unwrap_or_else(|| ".".to_string()),
            layer,
            manifest,
            main_class: item
                .get("extensions")
                .and_then(|extensions| extensions.get("maven"))
                .and_then(|maven| maven.get("mainClass"))
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }

    let entry_count = data
        .get("entryCount")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(configurations.len());

    Ok(RunProjectView {
        configurations,
        entry_count,
        java_entrypoints_origin: text(data, "javaEntrypointsOrigin"),
    })
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub fn load(root: &Path) -> Result<RunProjectView, String> {
    let started = Instant::now();
    let result = fetch(root);
    match &result {
        Ok((view, paths, truncated)) => println!(
            "{} result=ok {}",
            RUN_DIAGNOSTIC_TAG,
            view.diagnostic_line(root, *paths, *truncated, started.elapsed().as_millis())
        ),
        Err(error) => println!(
            "{} result=failed root={} ms={} error={error}",
            RUN_DIAGNOSTIC_TAG,
            root.display(),
            started.elapsed().as_millis()
        ),
    }
    result.map(|(view, _, _)| view)
}

fn fetch(root: &Path) -> Result<(RunProjectView, usize, bool), String> {
    let root_text = root
        .to_str()
        .ok_or_else(|| "工作区根不是合法 UTF-8 路径".to_string())?;

    let snapshot = core_json(
        "workspace.snapshot",
        serde_json::json!({ "root": root_text }),
    )
    .map_err(|error| core_error_text(&error))?;
    let (paths, truncated) = parse_snapshot(snapshot.as_ref())?;

    let generated = core_json(
        "runConfig.generate",
        serde_json::json!({ "root": root_text, "paths": paths, "modulePaths": [] }),
    )
    .map_err(|error| core_error_text(&error))?;
    persist_generated(root, generated.as_ref())?;

    let view = parse_generate(generated.as_ref())?;
    Ok((view, paths.len(), truncated))
}

fn persist_generated(root: &Path, data: Option<&Value>) -> Result<(), String> {
    let Some(document_value) = data
        .and_then(|data| data.get("generated"))
        .filter(|value| value.is_object())
    else {
        return Ok(());
    };
    let path = WorkspaceConfigPaths::new(root).run_generated();
    let version = document_value
        .get("version")
        .and_then(Value::as_u64)
        .unwrap_or(RUN_DOCUMENT_VERSION) as u32;
    let previous = previous_object(&path);
    let bytes = save_document(&path, document_value, previous.as_ref(), version)
        .map_err(|error| format!("写入 .lithe/run/generated.json 失败：{error}"))?;
    println!(
        "S1_RUN_GENERATED path={} bytes={bytes} configs={}",
        path.display(),
        document_value
            .get("configurations")
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0)
    );
    Ok(())
}

fn core_error_text(error: &lithe_gpui_shared::CoreError) -> String {
    match error.code() {
        Some(code) => format!("{code}: {error}"),
        None => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_data() -> Value {
        serde_json::json!({
            "generated": {
                "version": 2,
                "generator": { "fingerprint": "abc", "inputs": {} },
                "configurations": [
                    {
                        "id": "spring-boot.maven:.:demo",
                        "name": "demo",
                        "provider": "spring-boot.maven",
                        "execution": "service",
                        "command": null,
                        "args": [],
                        "cwd": ".",
                        "env": {},
                        "toolchains": { "java": "project-jdk", "maven": "project-maven" },
                        "debug": { "adapter": "jdwp" },
                        "extensions": { "maven": { "module": ".", "mainClass": "com.example.Demo" } },
                        "source": "pom.xml"
                    },
                    {
                        "id": "npm.script:web:dev",
                        "name": "dev",
                        "provider": "npm.script",
                        "execution": "task",
                        "command": "npm",
                        "args": ["run", "dev"],
                        "cwd": "web",
                        "source": "web/package.json"
                    }
                ]
            },
            "toolchainRequirements": {},
            "entryCount": 2,
            "javaEntrypointsOrigin": "previousGeneration"
        })
    }

    #[test]
    fn generate_response_parses_every_field() {
        let view = parse_generate(Some(&generate_data())).expect("合法响应必须能解析");
        assert_eq!(view.entry_count, 2);
        assert_eq!(
            view.java_entrypoints_origin.as_deref(),
            Some("previousGeneration")
        );
        assert_eq!(view.configurations.len(), 2);

        let maven = &view.configurations[0];
        assert_eq!(maven.name, "demo");
        assert_eq!(maven.provider, "spring-boot.maven");
        assert_eq!(maven.provider_title, "Spring Boot");
        assert_eq!(maven.execution, RunExecution::Service);
        assert_eq!(maven.command_line(), None);
        assert_eq!(maven.cwd, ".");
        assert_eq!(maven.layer, RunLayer::Generated);
        assert_eq!(maven.manifest.as_deref(), Some("pom.xml"));
        assert_eq!(maven.main_class.as_deref(), Some("com.example.Demo"));

        let npm = &view.configurations[1];
        assert_eq!(npm.provider_title, "Npm");
        assert_eq!(npm.execution, RunExecution::Task);
        assert_eq!(npm.command_line().as_deref(), Some("npm run dev"));
        assert_eq!(npm.cwd, "web");
        assert_eq!(npm.manifest.as_deref(), Some("web/package.json"));
        assert_eq!(npm.main_class, None);
    }

    #[test]
    fn malformed_responses_degrade_without_panicking() {
        assert!(parse_generate(None).is_err());
        assert!(parse_generate(Some(&serde_json::json!({ "entryCount": 0 }))).is_err());
        let empty = parse_generate(Some(&serde_json::json!({ "generated": { "version": 2 } })))
            .expect("缺 configurations 不应报错");
        assert!(empty.configurations.is_empty());
        assert_eq!(empty.entry_count, 0);
        let view = parse_generate(Some(&serde_json::json!({
            "generated": { "configurations": [ { "id": "x", "provider": "make.target" } ] }
        })))
        .expect("缺 entryCount 不应报错");
        assert_eq!(view.entry_count, 1);
        let view = parse_generate(Some(&serde_json::json!({
            "generated": { "configurations": [] },
            "entryCount": 0
        })))
        .expect("空数组必须能解析");
        assert!(view.configurations.is_empty());

        assert!(parse_snapshot(None).is_err());
        assert!(parse_snapshot(Some(&serde_json::json!({ "root": {} }))).is_err());
        let (files, truncated) = parse_snapshot(Some(&serde_json::json!({
            "files": ["pom.xml", 7, "src/Main.java"]
        })))
        .expect("合法 files 必须能解析");
        assert_eq!(
            files,
            vec!["pom.xml".to_string(), "src/Main.java".to_string()]
        );
        assert!(!truncated);
    }

    #[test]
    fn entries_without_identity_are_named_or_skipped() {
        let view = parse_generate(Some(&serde_json::json!({
            "generated": { "configurations": [
                { "id": "npm.script:.:dev", "provider": "npm.script" },
                { "name": "only-name", "provider": "make.target" },
                {},
                { "id": "", "name": "" }
            ] },
            "entryCount": 2
        })))
        .expect("兜底路径不应报错");
        assert_eq!(view.configurations.len(), 2);
        assert_eq!(view.configurations[0].name, "npm.script:.:dev");
        assert_eq!(view.configurations[1].id, "only-name");
    }

    #[test]
    fn source_is_split_into_layer_and_manifest() {
        assert_eq!(split_source(Some("generated")), (RunLayer::Generated, None));
        assert_eq!(split_source(Some("project")), (RunLayer::Project, None));
        assert_eq!(split_source(Some("local")), (RunLayer::Local, None));
        assert_eq!(
            split_source(Some("src/package.json")),
            (RunLayer::Generated, Some("src/package.json".to_string()))
        );
        assert_eq!(split_source(Some("  ")), (RunLayer::Generated, None));
        assert_eq!(split_source(None), (RunLayer::Generated, None));
        assert_eq!(RunLayer::Generated.id(), "generated");
        assert_eq!(RunLayer::Project.id(), "project");
        assert_eq!(RunLayer::Local.id(), "local");
        assert_eq!(RunLayer::from_id("local"), RunLayer::Local);
    }

    #[test]
    fn provider_titles_follow_the_truth_source() {
        assert_eq!(provider_title("spring-boot.maven"), "Spring Boot");
        assert_eq!(provider_title("quarkus.maven"), "Quarkus");
        assert_eq!(provider_title("micronaut.maven"), "Micronaut");
        assert_eq!(provider_title("java.main"), "Java Application");
        assert_eq!(provider_title("java.current-file"), "Current File");
        assert_eq!(provider_title("maven.module"), "Maven Module");
        assert_eq!(provider_title("npm.script"), "Npm");
        assert_eq!(provider_title("compose.service"), "Compose");
        assert_eq!(provider_title("just.recipe"), "Just");
        assert_eq!(provider_title("bare"), "Bare");
    }

    #[test]
    fn execution_labels_reuse_the_truth_source_sections() {
        assert_eq!(RunExecution::from_id("service"), RunExecution::Service);
        assert_eq!(RunExecution::from_id("task"), RunExecution::Task);
        assert_eq!(RunExecution::from_id("group"), RunExecution::Group);
        assert_eq!(RunExecution::from_id("nope"), RunExecution::Application);
        assert_eq!(RunExecution::Service.label_key(), "lithe.run.services");
        assert_eq!(
            RunExecution::Application.label_key(),
            "lithe.run.applications"
        );
        assert_eq!(RunExecution::Task.label_key(), "lithe.run.tasks");
        assert_eq!(RunExecution::Group.label_key(), "lithe.run.groups");
    }

    #[test]
    fn diagnostic_line_carries_the_page_counts() {
        let view = parse_generate(Some(&generate_data())).expect("合法响应必须能解析");
        let line = view.diagnostic_line(Path::new(r"D:\proj"), 812, false, 34);
        assert!(line.contains("paths=812"), "{line}");
        assert!(line.contains("truncated=false"), "{line}");
        assert!(line.contains("configs=2"), "{line}");
        assert!(line.contains("entryCount=2"), "{line}");
        assert!(line.contains("sources=generated:2"), "{line}");
        assert!(
            line.contains("providers=npm.script:1,spring-boot.maven:1"),
            "{line}"
        );
        assert!(line.contains("origin=previousGeneration"), "{line}");
        assert!(line.contains("ms=34"), "{line}");

        let empty = RunProjectView {
            configurations: Vec::new(),
            entry_count: 0,
            java_entrypoints_origin: None,
        };
        let line = empty.diagnostic_line(Path::new("."), 0, false, 1);
        assert!(line.contains("sources=- providers=- origin=-"), "{line}");
    }

    #[test]
    fn a_real_maven_fixture_yields_the_spring_boot_service() {
        let root =
            std::env::temp_dir().join(format!("lithe-settings-run-fixture-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src/main/java/com/example")).expect("建目录失败");
        std::fs::write(
            root.join("pom.xml"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0">
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.example</groupId>
  <artifactId>demo</artifactId>
  <version>0.0.1-SNAPSHOT</version>
  <packaging>jar</packaging>
  <build>
    <plugins>
      <plugin>
        <groupId>org.springframework.boot</groupId>
        <artifactId>spring-boot-maven-plugin</artifactId>
        <version>3.3.0</version>
      </plugin>
    </plugins>
  </build>
</project>
"#,
        )
        .expect("写 pom.xml 失败");
        std::fs::write(
            root.join("package.json"),
            r#"{ "name": "demo-web", "scripts": { "dev": "vite" } }"#,
        )
        .expect("写 package.json 失败");

        let (view, paths, truncated) = fetch(&root).expect("真实 Core 调用必须成功");
        assert_eq!(paths, 2);
        assert!(!truncated);
        println!(
            "{} fixture=real configs={} entryCount={} rows={}",
            RUN_DIAGNOSTIC_TAG,
            view.configurations.len(),
            view.entry_count,
            view.configurations
                .iter()
                .map(|configuration| format!(
                    "{}[{}] cmd={:?} cwd={} src={:?}",
                    configuration.name,
                    configuration.provider,
                    configuration.command_line(),
                    configuration.cwd,
                    configuration.manifest
                ))
                .collect::<Vec<_>>()
                .join(" | ")
        );
        assert!(
            view.configurations
                .iter()
                .any(|configuration| configuration.provider == "spring-boot.maven"),
            "Maven 探测器没认出 spring-boot-maven-plugin：{:#?}",
            view.configurations
        );
        assert!(
            view.configurations
                .iter()
                .any(|configuration| configuration.provider == "npm.script"),
            "npm 探测器没认出 package.json 的 scripts：{:#?}",
            view.configurations
        );
        assert_eq!(view.entry_count, view.configurations.len());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_root_is_an_error_not_an_empty_page() {
        let missing = std::env::temp_dir().join("lithe-settings-run-does-not-exist");
        let error = fetch(&missing).expect_err("不存在的根必须失败");
        assert!(!error.is_empty(), "失败必须带一句可排查的原因");
    }

    #[test]
    fn a_project_without_runnable_files_yields_an_empty_list() {
        let root =
            std::env::temp_dir().join(format!("lithe-settings-run-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("建目录失败");
        std::fs::write(root.join("README.md"), "# nothing runnable here\n").expect("写文件失败");

        let (view, paths, _) = fetch(&root).expect("真实 Core 调用必须成功");
        assert_eq!(paths, 1);
        assert!(
            view.configurations.is_empty(),
            "没有被识别的项目必须是空表：{:#?}",
            view.configurations
        );
        assert_eq!(view.entry_count, 0);

        let _ = std::fs::remove_dir_all(&root);
    }
}
