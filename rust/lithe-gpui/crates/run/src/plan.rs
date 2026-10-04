use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;

pub const TOOLCHAIN_JAVA: &str = "java";
pub const TOOLCHAIN_MAVEN: &str = "maven";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolchainPath {
    JavaHome(PathBuf),
    Program(PathBuf),
}

pub type ToolchainResolver = Arc<dyn Fn(&str) -> Option<ToolchainPath> + Send + Sync + 'static>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanError(pub String);

impl std::fmt::Display for PlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl PlanError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
}

pub fn launch_command(
    root: &Path,
    plan: &Value,
    resolve: &ToolchainResolver,
) -> Result<(Vec<CommandSpec>, CommandSpec), PlanError> {
    let steps = plan
        .get("preLaunchSteps")
        .and_then(Value::as_array)
        .map(|steps| {
            steps
                .iter()
                .map(|step| command_spec(root, step, resolve))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    let main = command_spec(root, plan, resolve)?;
    Ok((steps, main))
}

fn command_spec(
    root: &Path,
    value: &Value,
    resolve: &ToolchainResolver,
) -> Result<CommandSpec, PlanError> {
    let executable = value
        .get("executable")
        .ok_or_else(|| PlanError::new("启动计划缺少 executable"))?;
    let program = resolve_executable(executable, resolve)?;
    let mut env: Vec<(String, String)> = Vec::new();

    let mut args: Vec<String> = Vec::new();
    if let Some(joined) = join_paths(value.get("classpath"))? {
        args.push("-cp".to_string());
        args.push(joined);
    }
    if let Some(joined) = join_paths(value.get("modulepath"))? {
        args.push("--module-path".to_string());
        args.push(joined);
    }
    args.extend(
        value
            .get("arguments")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    );

    for (key, target) in value
        .get("environment")
        .and_then(Value::as_object)
        .map(|entries| {
            entries
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
        })
        .into_iter()
        .flatten()
    {
        let resolved = resolve_environment_value(&target, resolve)?;
        push_env(&mut env, key, resolved);
    }
    for (key, target) in value
        .get("env")
        .and_then(Value::as_object)
        .map(|entries| {
            entries
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
        })
        .into_iter()
        .flatten()
    {
        if let Some(text) = target.as_str() {
            push_env(&mut env, key, text.to_string());
        }
    }

    let cwd = value
        .get("workingDirectory")
        .and_then(Value::as_str)
        .filter(|directory| !directory.is_empty())
        .map(|directory| root.join(directory.trim_start_matches('/')))
        .unwrap_or_else(|| root.to_path_buf());

    Ok(CommandSpec {
        program,
        args,
        cwd,
        env,
    })
}

fn push_env(env: &mut Vec<(String, String)>, key: String, value: String) {
    if let Some(existing) = env.iter_mut().find(|(name, _)| *name == key) {
        existing.1 = value;
    } else {
        env.push((key, value));
    }
}

fn resolve_executable(
    executable: &Value,
    resolve: &ToolchainResolver,
) -> Result<PathBuf, PlanError> {
    if let Some(command) = executable
        .get("command")
        .and_then(Value::as_str)
        .filter(|command| !command.is_empty())
    {
        return Ok(PathBuf::from(command));
    }

    let reference = executable
        .get("toolchain")
        .and_then(Value::as_str)
        .filter(|reference| !reference.is_empty())
        .ok_or_else(|| PlanError::new("启动计划的 executable 既没有 command 也没有 toolchain"))?;
    let tool = executable.get("tool").and_then(Value::as_str);
    let Some(resolved) = resolve(reference) else {
        return Err(PlanError::new(format!(
            "找不到工具链「{reference}」：请在设置里指定对应的 JDK / Maven 路径"
        )));
    };

    match resolved {
        ToolchainPath::JavaHome(home) => {
            let tool = tool.unwrap_or("java");
            Ok(home.join("bin").join(executable_name(tool)))
        }
        ToolchainPath::Program(program) => Ok(program),
    }
}

fn executable_name(tool: &str) -> String {
    if cfg!(windows) && !tool.to_ascii_lowercase().ends_with(".exe") {
        format!("{tool}.exe")
    } else {
        tool.to_string()
    }
}

fn resolve_environment_value(
    value: &Value,
    resolve: &ToolchainResolver,
) -> Result<String, PlanError> {
    let Some(reference) = value.get("toolchain").and_then(Value::as_str) else {
        return Err(PlanError::new(format!(
            "启动计划里的 environment 取值无法解析：{value}"
        )));
    };
    let property = value
        .get("property")
        .and_then(Value::as_str)
        .unwrap_or("home");
    let resolved = resolve(reference).ok_or_else(|| {
        PlanError::new(format!(
            "找不到工具链「{reference}」：请在设置里指定对应的 JDK / Maven 路径"
        ))
    })?;
    match (property, resolved) {
        ("home", ToolchainPath::JavaHome(home)) => Ok(home.to_string_lossy().into_owned()),
        ("home", ToolchainPath::Program(program)) => program
            .parent()
            .map(|parent| parent.to_string_lossy().into_owned())
            .ok_or_else(|| PlanError::new(format!("工具链「{reference}」没有可用的 home 目录"))),
        (other, _) => Err(PlanError::new(format!(
            "启动计划请求了不支持的 environment 属性「{other}」"
        ))),
    }
}

fn join_paths(value: Option<&Value>) -> Result<Option<String>, PlanError> {
    let Some(items) = value.and_then(Value::as_array) else {
        return Ok(None);
    };
    if items.is_empty() {
        return Ok(None);
    }
    let mut paths = Vec::with_capacity(items.len());
    for item in items {
        let Some(text) = item.as_str() else {
            return Err(PlanError::new("classpath / modulepath 里有非字符串项"));
        };
        if text.is_empty() {
            continue;
        }
        paths.push(text);
    }
    if paths.is_empty() {
        return Ok(None);
    }
    let separator = if cfg!(windows) { ';' } else { ':' };
    Ok(Some(paths.join(&separator.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolver(java_home: &str, maven: Option<&str>) -> ToolchainResolver {
        let java_home = PathBuf::from(java_home);
        let maven = maven.map(PathBuf::from);
        Arc::new(move |reference: &str| match reference {
            "project-jdk" | "java" => Some(ToolchainPath::JavaHome(java_home.clone())),
            "project-maven" | "maven" => maven.clone().map(ToolchainPath::Program),
            _ => None,
        })
    }

    #[test]
    fn java_plan_becomes_a_real_command_line() {
        let plan = serde_json::json!({
            "executable": { "toolchain": "project-jdk" },
            "arguments": ["-Dfile.encoding=UTF-8", "-m", "demo/com.example.Main"],
            "workingDirectory": "demo",
            "environment": { "JAVA_HOME": { "toolchain": "project-jdk", "property": "home" } },
            "classpath": ["/ws/demo/target/classes", "/home/user/.m2/x.jar"],
            "modulepath": ["/ws/demo/target/classes"]
        });
        let (steps, main) =
            launch_command(Path::new("/ws"), &plan, &resolver("/jdk-21", None)).unwrap();

        assert!(steps.is_empty());
        assert_eq!(
            main.program,
            PathBuf::from(if cfg!(windows) {
                "/jdk-21\\bin\\java.exe"
            } else {
                "/jdk-21/bin/java"
            })
        );
        let separator = if cfg!(windows) { ';' } else { ':' };
        assert_eq!(
            main.args,
            vec![
                "-cp".to_string(),
                format!("/ws/demo/target/classes{separator}/home/user/.m2/x.jar"),
                "--module-path".to_string(),
                "/ws/demo/target/classes".to_string(),
                "-Dfile.encoding=UTF-8".to_string(),
                "-m".to_string(),
                "demo/com.example.Main".to_string(),
            ]
        );
        assert_eq!(main.cwd, Path::new("/ws").join("demo"));
        assert_eq!(
            main.env,
            vec![("JAVA_HOME".to_string(), "/jdk-21".to_string())]
        );
    }

    #[test]
    fn prelaunch_steps_resolve_their_own_tool_and_classpath() {
        let plan = serde_json::json!({
            "executable": { "toolchain": "project-jdk" },
            "arguments": ["com.example.Main"],
            "workingDirectory": ".",
            "environment": { "JAVA_HOME": { "toolchain": "project-jdk", "property": "home" } },
            "classpath": ["/ws/.lithe/run/classes/java.main"],
            "preLaunchSteps": [{
                "executable": { "toolchain": "project-jdk", "tool": "javac" },
                "arguments": ["-d", "/ws/.lithe/run/classes/java.main", "src/Main.java"],
                "classpath": ["/ws/.lithe/run/classes/java.main"]
            }]
        });
        let (steps, _) =
            launch_command(Path::new("/ws"), &plan, &resolver("/jdk-21", None)).unwrap();

        assert_eq!(steps.len(), 1);
        let step = &steps[0];
        assert!(
            step.program.to_string_lossy().contains("javac"),
            "编译步骤必须是 javac，实际是 {}",
            step.program.display()
        );
        assert_eq!(step.args[0], "-cp");
        assert_eq!(step.args[1], "/ws/.lithe/run/classes/java.main");
        assert_eq!(step.args[2], "-d");
    }

    #[test]
    fn command_plans_use_the_bare_program_name() {
        let plan = serde_json::json!({
            "executable": { "command": "npm" },
            "arguments": ["run", "dev"],
            "workingDirectory": "web",
            "environment": {},
            "env": { "NODE_ENV": "development" }
        });
        let (_, main) =
            launch_command(Path::new("/ws"), &plan, &resolver("/jdk-21", None)).unwrap();
        assert_eq!(main.program, PathBuf::from("npm"));
        assert_eq!(main.args, vec!["run".to_string(), "dev".to_string()]);
        assert_eq!(main.cwd, Path::new("/ws").join("web"));
        assert_eq!(
            main.env,
            vec![("NODE_ENV".to_string(), "development".to_string())]
        );
    }

    #[test]
    fn maven_plans_use_the_maven_executable() {
        let plan = serde_json::json!({
            "executable": { "toolchain": "project-maven" },
            "arguments": ["-pl", "demo", "spring-boot:run"],
            "workingDirectory": ".",
            "environment": {}
        });
        let (_, main) = launch_command(
            Path::new("/ws"),
            &plan,
            &resolver("/jdk-21", Some("/maven/bin/mvn")),
        )
        .unwrap();
        assert_eq!(main.program, PathBuf::from("/maven/bin/mvn"));
        assert!(main.env.is_empty());
        assert_eq!(main.cwd, Path::new("/ws"));
    }

    #[test]
    fn missing_toolchain_reports_what_to_configure() {
        let plan = serde_json::json!({
            "executable": { "toolchain": "project-jdk" },
            "arguments": [],
            "workingDirectory": ".",
            "environment": {}
        });
        let empty: ToolchainResolver = Arc::new(|_| None);
        let error = launch_command(Path::new("/ws"), &plan, &empty).unwrap_err();
        assert!(error.0.contains("project-jdk"), "实际：{}", error.0);
        assert!(error.0.contains("设置"), "提示要指向设置页：{}", error.0);
    }

    #[test]
    fn unsupported_environment_property_is_an_error() {
        let plan = serde_json::json!({
            "executable": { "toolchain": "project-jdk" },
            "arguments": [],
            "workingDirectory": ".",
            "environment": { "MAVEN_HOME": { "toolchain": "project-jdk", "property": "prefix" } }
        });
        let error =
            launch_command(Path::new("/ws"), &plan, &resolver("/jdk-21", None)).unwrap_err();
        assert!(error.0.contains("prefix"), "实际：{}", error.0);
    }
}
