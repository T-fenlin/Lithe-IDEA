//! 「设置 → 运行配置」页的**数据层**：Core 两条命令的一次往返 + 纯解析 + 纯分类。
//!
//! ## 这一页的数据从哪来
//!
//! 两步，都是**只读**的 Core 命令（`lithe-gpui/crates/shared/src/core_client.rs` 的信封）：
//!
//! ```text
//! ① workspace.snapshot { root }            → data.files（工作区相对路径，`/` 分隔）
//! ② runConfig.generate { root, paths }     → data.generated.configurations[]
//! ```
//!
//! - ① 的命令名 / 请求 / 响应逐条照 `lithe-gpui/crates/explorer/src/model.rs:211-261`（那里已经
//!   为项目树消费过一次），本模块只取 `data.files`：`runConfig.generate` 的 `paths` 用来
//!   推 Maven 根与 Java 生态（`rust/lithe-core/src/execution/configuration.rs:462-505`）。
//! - ② 是这一页的**唯一必需命令**：响应里直接带
//!   `generated.configurations[]`（`name` / `provider` / `execution` / `command` / `args` /
//!   `cwd` / `source` / `extensions`，`rust/lithe-core/src/execution/configuration.rs:286-325`
//!   的 `RunConfiguration`），**且不写任何文件** —— 契约
//!   `shared/contracts/rust-core-api.md:1536-1538` 明写"返回文档、由平台适配器决定要不要落盘"。
//!
//! ## 为什么用 `generate` 而不是 `resolve`
//!
//! `runConfig.resolve` 更接近真源（真源是 generate → resolve 两步，见
//! `windows/tauri/src/features/run/stores/run.store.ts:550-570`），但它**必须先有**
//! `.lithe/run/generated.json`：`resolve` 第一步就是
//! `read_document_value(root, "run/generated.json")`，读不到直接返回
//! `WorkspaceNotFound`（`rust/lithe-core/src/execution/configuration.rs:986-991`）。
//! gpui 侧目前**没有任何东西写过** `.lithe/run/*.json`（全仓库 grep `runConfig` 零命中），
//! 所以 `resolve` 只会稳定失败；`generate` 自己走文件树探测，一次调用就够。
//!
//! 代价（如实登记）：`generate` 的响应里 `source` 是**产生这条配置的清单文件**
//! （`detectors/maven.rs:100-104` 的 `pom.xml`），不是"来自哪一层"；只有 `resolve` 的
//! 合并步骤才会把 `source` 覆写成 `generated` / `project` / `local`
//! （`configuration.rs:2234-2245` 的 `merge_values`）。本模块因此按真源的判据把两者分开：
//! 见 [`split_source`]。
//!
//! ## 与真源（`components/run-configuration-settings.tsx`）的关系
//!
//! 真源那页的**列表态**（`:79-113`）只有三样东西：一句说明（`settings.run.description`）、
//! 一个「重新识别」按钮（`run.identifyAgain`）、以及每条配置一个可点的 ghost 按钮
//! （点进去是 `RunConfigurationEditor`）。本页照它的**事实字段**做，但：
//!
//! - 行**不可点**：gpui 侧没有配置编辑器（真源的行点进去是编辑器，本侧没有那个落点）；
//! - 「执行」整条链路不存在（无 run crate、Run 工具窗占位、Run 菜单为空），
//!   所以页面上**不画运行按钮**，改为一句如实说明（`lithe.settings.gpui.runNotWired`）。
//!
//! ## 诊断
//!
//! 每次真正取数都会打一行 [`RUN_DIAGNOSTIC_TAG`]，页面上显示多少条、都是什么来源，
//! 能在日志里逐项核对（[`RunProjectView::diagnostic_line`]）。

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use lithe_gpui_shared::core_json;
use serde_json::Value;

/// 本页诊断行的前缀（走 stdout，与 `S1_SETTINGS` / `S1_MAVEN` 一族同口径）。
///
/// 形如：
/// ```text
/// S1_SETTINGS_RUN root=D:\proj paths=812 truncated=false configs=2 entryCount=1 sources=generated:2 providers=spring-boot.maven:1,npm.script:1 origin=previousGeneration ms=34
/// ```
/// 页面上的行数 = `configs`，每条的来源分类 = `sources`，真正的可运行入口数 = `entryCount`
/// （Core 的 `entryCount` **不含**恒有的 "Current File" 兜底项，见
/// `rust/lithe-core/src/execution/configuration.rs:634-637`）。
pub const RUN_DIAGNOSTIC_TAG: &str = "S1_SETTINGS_RUN";

/// 传给 `runConfig.generate` 的 `paths` 上限。
///
/// 与 explorer 侧消费同一份快照时的 `RENDER_LIMIT` 同值（`lithe-gpui/crates/explorer/src/model.rs:35`）：
/// 截断只影响"极深目录里的 Java 生态判定"，**不影响 Maven 根的选择**（`maven_root` 只看路径的
/// 祖先链，`rust/lithe-core/src/project/maven.rs:832-850`）。被截断时诊断行里 `truncated=true`，
/// 不静默。
pub const MAX_PATHS: usize = 5_000;

/// 这条配置**生效于哪一层**（真源的 `run.source.*` 三档，`lithe.zh-CN.yml:3180-3185`）。
///
/// `generate` 的响应里这一栏恒为 [`RunLayer::Generated`]（只有 `resolve` 会合并出另两层），
/// 但仍然按真源的判据解析（`windows/tauri/src/features/run/utils/run-configuration.ts:30`），
/// 这样将来接上 `resolve` 时界面不用改。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunLayer {
    /// Core 从项目文件自动识别（`run.source.generated` = 自动生成）。
    Generated,
    /// 团队共享的项目配置（`.lithe/run/configurations.json`）。
    Project,
    /// 本机覆盖（`.lithe/run/local.json`）。
    Local,
}

impl RunLayer {
    /// 诊断行 `sources=…` 的取值（稳定 token，与真源的字面量逐字相同）。
    pub fn id(self) -> &'static str {
        match self {
            Self::Generated => "generated",
            Self::Project => "project",
            Self::Local => "local",
        }
    }

    /// 界面上那一栏的文案键（真源 `run.source.*`）。
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Generated => "lithe.run.source.generated",
            Self::Project => "lithe.run.source.project",
            Self::Local => "lithe.run.source.local",
        }
    }

    /// 从真源的分层字面量解析回来；不是这三档的一律当 `generated`
    /// （与 `run-configuration.ts:30` 的判据逐字一致）。
    pub fn from_id(id: &str) -> Self {
        match id {
            "project" => Self::Project,
            "local" => Self::Local,
            _ => Self::Generated,
        }
    }
}

/// 这条配置跑起来之后的行为（Core `Execution`，`rust/lithe-core/src/execution/types.rs:6-17`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunExecution {
    /// 面向用户的进程（可能交互，但不是后台服务）。
    Application,
    /// 一直服务到被显式停止的常驻进程。
    Service,
    /// 跑完就退出的有限命令。
    Task,
    /// 把其它配置编组一起启动的非进程项。
    Group,
}

impl RunExecution {
    /// 诊断行里的取值（Core 的 `#[serde(rename_all = "lowercase")]` 字面量）。
    pub fn id(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::Service => "service",
            Self::Task => "task",
            Self::Group => "group",
        }
    }

    /// 界面上那一栏的文案键：真源按这一档把列表**分组**
    /// （`run-pane.tsx:330,370,379` 的 `run.services` / `run.applications` / `run.tasks`），
    /// 本页不分组，但同一档的标题键可以直接当每一条的类别标签用。
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Application => "lithe.run.applications",
            Self::Service => "lithe.run.services",
            Self::Task => "lithe.run.tasks",
            Self::Group => "lithe.run.groups",
        }
    }

    /// 解析 Core 的字面量；认不出来的当 `application`（Core 的默认值，同文件 `:19-23`）。
    pub fn from_id(id: &str) -> Self {
        match id {
            "service" => Self::Service,
            "task" => Self::Task,
            "group" => Self::Group,
            _ => Self::Application,
        }
    }
}

/// 界面上一条配置显示的全部事实。字段名与 Core 的 `RunConfiguration` 对齐。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunConfigurationView {
    /// Core 的稳定 id（团队 / 本机覆盖层的连接键）。
    pub id: String,
    /// 条目的显示名（真源列表行的主文本就是它）。
    pub name: String,
    /// 命名空间判别符，例如 `spring-boot.maven` / `npm.script`。
    pub provider: String,
    /// [`provider`](Self::provider) 的显示名（真源 `configurationTitle`，
    /// `windows/tauri/src/features/run/utils/run-configuration.ts:18-25,59-63`）。
    pub provider_title: String,
    /// 启动后的行为（真源的列表就是按它分组的）。
    pub execution: RunExecution,
    /// 要拉起的程序；由工具链接管可执行文件的条目为 `None`。
    pub command: Option<String>,
    /// 程序参数（顺序照 Core 给的）。
    pub args: Vec<String>,
    /// 命令的工作目录（工作区相对路径，Core 的缺省是 `.`）。
    pub cwd: String,
    /// 生效层级（见 [`RunLayer`]）。
    pub layer: RunLayer,
    /// 产生这条配置的清单文件（`pom.xml` / `package.json` …）；`generate` 的响应里就是它。
    pub manifest: Option<String>,
    /// JVM 配置的主类（Core `extensions.maven.mainClass`，`configuration.rs:342-344`）。
    pub main_class: Option<String>,
}

impl RunConfigurationView {
    /// 命令行（`command` + `args`）；`None` = 这条配置没有自己的命令行。
    ///
    /// `None` 不是缺失，而是**语义**：Maven 框架服务把可执行文件交给工具链
    /// （`detectors/mod.rs:156-171` 的 `with_toolchains` 会把 `command` 置空），
    /// 真正的命令行由启动计划按工具链组装，所以界面上如实说"没有固定命令行"。
    pub fn command_line(&self) -> Option<String> {
        let command = self.command.as_deref().map(str::trim).filter(|c| !c.is_empty())?;
        if self.args.is_empty() {
            Some(command.to_string())
        } else {
            Some(format!("{command} {}", self.args.join(" ")))
        }
    }
}

/// 一页的完整事实（= 一次 `runConfig.generate` 的解析结果）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunProjectView {
    /// 配置表，**顺序照 Core 给的**（Core 保证确定序：Java 入口在前，探测器结果按
    /// 目录深度 / 目录 / 名字排序，`detectors/mod.rs:279-286`）。不重排。
    pub configurations: Vec<RunConfigurationView>,
    /// Core 认出的**真实入口数**（不含恒有的 "Current File" 兜底项）。
    pub entry_count: usize,
    /// Java 入口从哪来：`languageService`（语言服务本次的回答）或 `previousGeneration`
    /// （上一次生成的记录，冷启动时不让列表清空，`configuration.rs:492-495`）。
    pub java_entrypoints_origin: Option<String>,
}

impl RunProjectView {
    /// 按生效层级计数（诊断行的 `sources=`）。
    pub fn layer_counts(&self) -> BTreeMap<&'static str, usize> {
        let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        for configuration in &self.configurations {
            *counts.entry(configuration.layer.id()).or_default() += 1;
        }
        counts
    }

    /// 按 provider 计数（诊断行的 `providers=`）；`BTreeMap` 保证这一行也是确定序。
    pub fn provider_counts(&self) -> BTreeMap<String, usize> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for configuration in &self.configurations {
            *counts.entry(configuration.provider.clone()).or_default() += 1;
        }
        counts
    }

    /// 一行可 grep 的诊断：页面上显示多少条、各是什么来源 / provider，都能在日志里核对。
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

/// `a:1,b:2` 形式的计数摘要（`BTreeMap` 的迭代序 = 键的字典序，确定）。
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

/// Core `provider` → 显示名。
///
/// 逐字照真源 `configurationTitle`（`windows/tauri/src/features/run/utils/run-configuration.ts:18-25,59-63`）：
/// 六条框架 / Java 专用名走表，其余取命名空间首段并首字母大写（`npm.script` → `Npm`）。
/// 真源也没给这些名字建文案键（它们是技术名，不是界面用词），本侧同样不建。
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

/// Core 的 `source` 字段拆成「哪一层」+「哪个清单文件」。
///
/// `generate` 的响应里 `source` 是清单文件（`pom.xml`），只有 `resolve` 的合并层会把
/// 它覆写成 `generated` / `project` / `local`。真源的判据（`run-configuration.ts:30`）
/// 正是"三层字面量之一就是层，否则算 generated"，本函数照它实现，两个信息都不丢。
fn split_source(source: Option<&str>) -> (RunLayer, Option<String>) {
    let Some(source) = source.map(str::trim).filter(|value| !value.is_empty()) else {
        return (RunLayer::Generated, None);
    };
    match source {
        "generated" | "project" | "local" => (RunLayer::from_id(source), None),
        manifest => (RunLayer::Generated, Some(manifest.to_string())),
    }
}

/// `workspace.snapshot` 的响应 → 扁平的文件路径表。
///
/// 响应形状逐条照 `lithe-gpui/crates/explorer/src/model.rs:220-223` 的契约注释：
/// `{ root: WorkspaceNode, files: String[] }`，本函数只用 `files`。
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

/// `runConfig.generate` 的 `data` → 一页的事实（**纯函数**，直接单测）。
///
/// 兜底口径：
/// - `data` 或 `data.generated` 缺失 → `Err`（那是信封级/契约级失败，不是"没有配置"）；
/// - `generated.configurations` 缺失或不是数组 → **空表**（契约保证它存在，但画"没识别到"
///   比 panic 或报错更接近事实）；
/// - 单条缺 `id` / `name` → 用对方兜底，两个都没有才跳过（无法标识的条目不画）。
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
        // 兜底顺序：自己的 → 对方的 → provider。两个都没有时 `id` 还是空串，下面直接跳过。
        let id = raw_id
            .clone()
            .or(raw_name.clone())
            .unwrap_or_else(|| provider.clone());
        let name = raw_name.or(raw_id).unwrap_or_else(|| id.clone());
        if id.is_empty() {
            // 既没有 id 也没有 name：画出来也没法称呼它，跳过而不是造一个假名字。
            continue;
        }
        let (layer, manifest) = split_source(item.get("source").and_then(Value::as_str));
        configurations.push(RunConfigurationView {
            id,
            name,
            provider_title: provider_title(&provider),
            provider,
            execution: RunExecution::from_id(
                item.get("execution").and_then(Value::as_str).unwrap_or("application"),
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
        // 契约里这一项恒在；缺失时退回"表里有多少条"，比报错或显示 0 都更接近事实。
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

/// 取一次数：`workspace.snapshot` → `runConfig.generate` → 解析。
///
/// **同步**（两次 Core 调用都是同步读盘 / 遍历），调用方必须放进
/// `Context::background_spawn`（与 `explorer` / `maven` 的调用点同一条口径）。
/// 成功与失败都打一行 [`RUN_DIAGNOSTIC_TAG`]。
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

/// [`load`] 的实现（拆出来是为了在测试里绕开那次诊断打印）。
fn fetch(root: &Path) -> Result<(RunProjectView, usize, bool), String> {
    let root_text = root
        .to_str()
        .ok_or_else(|| "工作区根不是合法 UTF-8 路径".to_string())?;

    let snapshot = core_json("workspace.snapshot", serde_json::json!({ "root": root_text }))
        .map_err(|error| core_error_text(&error))?;
    let (paths, truncated) = parse_snapshot(snapshot.as_ref())?;

    let generated = core_json(
        "runConfig.generate",
        serde_json::json!({ "root": root_text, "paths": paths, "modulePaths": [] }),
    )
    .map_err(|error| core_error_text(&error))?;
    let view = parse_generate(generated.as_ref())?;
    Ok((view, paths.len(), truncated))
}

/// Core 失败 → 可排查的一句话。
///
/// 有错误码时**带上错误码原文**（`workspace_not_found` / `parse_failed` …）：
/// 页面上那句"识别失败：…"里的原因就是它，用户据此能自己往下查。
fn core_error_text(error: &lithe_gpui_shared::CoreError) -> String {
    match error.code() {
        Some(code) => format!("{code}: {error}"),
        None => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一条完整的 `runConfig.generate` 响应（字段照 Core 的 `RunConfiguration` 序列化形状，
    /// `configuration.rs:286-325`），用来钉住逐字段解析。
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

    /// 逐字段解析：名称 / 类型 / 命令行 / 工作目录 / 来源 / 主类。
    #[test]
    fn generate_response_parses_every_field() {
        let view = parse_generate(Some(&generate_data())).expect("合法响应必须能解析");
        assert_eq!(view.entry_count, 2);
        assert_eq!(view.java_entrypoints_origin.as_deref(), Some("previousGeneration"));
        assert_eq!(view.configurations.len(), 2);

        let maven = &view.configurations[0];
        assert_eq!(maven.name, "demo");
        assert_eq!(maven.provider, "spring-boot.maven");
        assert_eq!(maven.provider_title, "Spring Boot");
        assert_eq!(maven.execution, RunExecution::Service);
        // 工具链接管的条目没有命令行 —— 这是语义，不是缺字段。
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

    /// 缺字段 / 空数组 / 非法形状都不 panic：缺 `generated` 是错误，缺 `configurations` 是空表。
    #[test]
    fn malformed_responses_degrade_without_panicking() {
        // 完全没有 data → 错误（契约级失败）。
        assert!(parse_generate(None).is_err());
        // 有 data 但没有 generated → 错误。
        assert!(parse_generate(Some(&serde_json::json!({ "entryCount": 0 }))).is_err());
        // generated 里没有 configurations → 空表 + 空态。
        let empty = parse_generate(Some(&serde_json::json!({ "generated": { "version": 2 } })))
            .expect("缺 configurations 不应报错");
        assert!(empty.configurations.is_empty());
        // entryCount 缺失时退回表长（不报错、也不假装是 0）。
        assert_eq!(empty.entry_count, 0);
        let view = parse_generate(Some(&serde_json::json!({
            "generated": { "configurations": [ { "id": "x", "provider": "make.target" } ] }
        })))
        .expect("缺 entryCount 不应报错");
        assert_eq!(view.entry_count, 1);
        // 空数组就是空表（页面据此画"没识别到"）。
        let view = parse_generate(Some(&serde_json::json!({
            "generated": { "configurations": [] },
            "entryCount": 0
        })))
        .expect("空数组必须能解析");
        assert!(view.configurations.is_empty());

        // 快照响应缺 files / 不是数组 → 错误；不是字符串的项被丢掉。
        assert!(parse_snapshot(None).is_err());
        assert!(parse_snapshot(Some(&serde_json::json!({ "root": {} }))).is_err());
        let (files, truncated) = parse_snapshot(Some(&serde_json::json!({
            "files": ["pom.xml", 7, "src/Main.java"]
        })))
        .expect("合法 files 必须能解析");
        assert_eq!(files, vec!["pom.xml".to_string(), "src/Main.java".to_string()]);
        assert!(!truncated);
    }

    /// 缺 `name` / `id` 的条目用对方兜底；两个都没有的跳掉（不造名字）。
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

    /// 来源分类：三层字面量归层、其它字符串是清单文件、缺失算 generated。
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
        // 三层字面量与真源（`run.source.*`）逐字对齐。
        assert_eq!(RunLayer::Generated.id(), "generated");
        assert_eq!(RunLayer::Project.id(), "project");
        assert_eq!(RunLayer::Local.id(), "local");
        assert_eq!(RunLayer::from_id("local"), RunLayer::Local);
    }

    /// provider → 显示名逐条对齐真源的 `FRAMEWORK_TITLES` + 命名空间规则。
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

    /// 分类标签用真源的分组标题键（`run-pane.tsx:330,370,379`）。
    #[test]
    fn execution_labels_reuse_the_truth_source_sections() {
        assert_eq!(RunExecution::from_id("service"), RunExecution::Service);
        assert_eq!(RunExecution::from_id("task"), RunExecution::Task);
        assert_eq!(RunExecution::from_id("group"), RunExecution::Group);
        // 认不出来的当 application（Core 的默认值）。
        assert_eq!(RunExecution::from_id("nope"), RunExecution::Application);
        assert_eq!(RunExecution::Service.label_key(), "lithe.run.services");
        assert_eq!(RunExecution::Application.label_key(), "lithe.run.applications");
        assert_eq!(RunExecution::Task.label_key(), "lithe.run.tasks");
        assert_eq!(RunExecution::Group.label_key(), "lithe.run.groups");
    }

    /// 诊断行必须能回答"页面上那几条是从哪来的"：条数、来源、provider、耗时都在。
    #[test]
    fn diagnostic_line_carries_the_page_counts() {
        let view = parse_generate(Some(&generate_data())).expect("合法响应必须能解析");
        let line = view.diagnostic_line(Path::new(r"D:\proj"), 812, false, 34);
        assert!(line.contains("paths=812"), "{line}");
        assert!(line.contains("truncated=false"), "{line}");
        assert!(line.contains("configs=2"), "{line}");
        assert!(line.contains("entryCount=2"), "{line}");
        assert!(line.contains("sources=generated:2"), "{line}");
        assert!(line.contains("providers=npm.script:1,spring-boot.maven:1"), "{line}");
        assert!(line.contains("origin=previousGeneration"), "{line}");
        assert!(line.contains("ms=34"), "{line}");

        // 一条都没有时 sources / providers 是 `-`，不是空串（grep 时列数不飘）。
        let empty = RunProjectView {
            configurations: Vec::new(),
            entry_count: 0,
            java_entrypoints_origin: None,
        };
        let line = empty.diagnostic_line(Path::new("."), 0, false, 1);
        assert!(line.contains("sources=- providers=- origin=-"), "{line}");
    }

    /// **真实 Core 往返**：造一个带 `spring-boot-maven-plugin` 的最小 Maven 夹具，
    /// 走 [`fetch`]（真的调 `workspace.snapshot` + `runConfig.generate`）并断言拿回了那条服务。
    ///
    /// 这条测试保护的是"探测链真的通"：Core 的 Maven 探测器只认**声明式 reactor 里应用了
    /// 框架插件的模块**（`rust/lithe-core/src/execution/detectors/maven.rs:16-20,31-48`），
    /// 所以夹具必须同时有 `<modules>` 之外的合法 pom 与那个插件。
    #[test]
    fn a_real_maven_fixture_yields_the_spring_boot_service() {
        let root = std::env::temp_dir().join(format!(
            "lithe-settings-run-fixture-{}",
            std::process::id()
        ));
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
        // 夹具只有两个文件，远不到上限。
        assert_eq!(paths, 2);
        assert!(!truncated);
        // 与页面/应用同前缀的一行诊断：`cargo test … -- --nocapture` 就能看到
        // "这个夹具被识别成了哪几条"，与截图里的行逐项对得上。
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
        // Core 的 `entryCount` 不含恒有的 "Current File" 兜底项，夹具里没有 Java 源码，
        // 所以它就是探测器条数。
        assert_eq!(view.entry_count, view.configurations.len());

        let _ = std::fs::remove_dir_all(&root);
    }

    /// 打不开的目录要以**错误**收场（不是"没有配置"）：页面据此画失败态 + 原因。
    #[test]
    fn a_missing_root_is_an_error_not_an_empty_page() {
        let missing = std::env::temp_dir().join("lithe-settings-run-does-not-exist");
        let error = fetch(&missing).expect_err("不存在的根必须失败");
        assert!(!error.is_empty(), "失败必须带一句可排查的原因");
    }

    /// **空态的真实来源**：一个没有被任何探测器认出的目录，真实 Core 会返回 0 条
    /// （没有 Java 源码时也不会补那条恒有的 "Current File"）。
    ///
    /// 这条测试保护页面那句"没有识别到可运行配置"**只在真的没识别到时**出现：
    /// 它证明空表是 Core 的结论，而不是"取数失败"或"还没取到"被画成了空态。
    #[test]
    fn a_project_without_runnable_files_yields_an_empty_list() {
        let root = std::env::temp_dir().join(format!(
            "lithe-settings-run-empty-{}",
            std::process::id()
        ));
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
