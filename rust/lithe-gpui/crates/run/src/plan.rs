//! **启动计划 → 一条真命令行**：解析工具链引用、按平台分隔符合并 classpath、跑前置换算。
//!
//! ## 为什么这一步必须由宿主做
//!
//! Core 的 `runConfig.createLaunchPlan` 明确**不返回**可执行文件路径，也不返回 shell 命令
//! （契约 `shared/contracts/rust-core-api.md` 的 "It does not return a shell command or
//! platform executable path"）。它给的是三样东西，每一样都要宿主补上平台事实：
//!
//! | Core 给 | 宿主补 |
//! | --- | --- |
//! | `executable: { toolchain: "project-jdk" }` 或 `{ command: "npm" }` | 工具链引用 → 真实可执行文件（经 [`ToolchainResolver`]；`{command}` 走 `PATH`） |
//! | `classpath[]` / `modulepath[]`（结构化数组） | 用**平台路径分隔符**拼起来，再前置 `-cp` / `--module-path`（Core 明说它不拼，因为分隔符是平台相关的） |
//! | `environment`（工具链引用） / `env`（字面量） | 引用解析成真值（`property: "home"` → 工具链根目录） |
//! | `preLaunchSteps[]`（例如独立 Java 的 `javac`） | 按序跑完再跑主进程；非零退出就中止（契约明说这是宿主的事） |
//!
//! 所以本模块是**纯函数**（除工具链解析这一步外不碰任何环境）：给它一份计划与一个解析器，
//! 它给你一条能 `Command::new` 的命令行。纯的那一半可以直接单测。
//!
//! ## 分隔符与 `-m`：不在本模块做，也不在本模块猜
//!
//! - `-m module/name.Type` 是 **Core** 推的（`configuration.rs` 的 "the `java` CLI requires
//!   the module launcher flag"），本模块只把 `arguments` 原样搬过去；
//! - classpath 的拼接**只有一处**：`std::path::MAIN_SEPARATOR` 判断平台，
//!   `;`（Windows）/ `:`（POSIX）—— 与 `develop-lithe`「Keep shared contracts deterministic」
//!   同一条理由：这一份判断不能散落在两个 crate 里。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;

/// 工具链引用里代表 JDK 的那几个 id（Core 用 `config.toolchains` 的值）。
pub const TOOLCHAIN_JAVA: &str = "java";
/// 工具链引用里代表 Maven 的那几个 id。
pub const TOOLCHAIN_MAVEN: &str = "maven";

/// 一次工具链引用解析的结果。
///
/// 两种形态分开是必要的：JDK 给的是**安装根**（`bin/java` / `bin/javac` 在它下面，
/// `preLaunchSteps` 的 `tool: "javac"` 也靠它），Maven 给的是**可执行文件本身**。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolchainPath {
    /// JDK 安装根。
    JavaHome(PathBuf),
    /// 直接可执行的程序（Maven 一类）。
    Program(PathBuf),
}

/// 「工具链在哪」的解析器，由外壳登记（见 `crate::lib` 的模块文档）。
///
/// 参数是 Core 计划里的工具链引用 id（`"project-jdk"` / `"project-maven"` / `"java"` …）；
/// 返回 `None` 表示这台机器上没有可用的那个工具链 —— 调用方把它变成一条**可执行的**
/// 失败提示（"在设置里指定 JDK"），而不是静默换一个。
///
/// ⚠️ `Send + Sync` 是必需的，不是装饰：解析工具链引用发生在
/// [`crate::session::prepare`] 里，而那个函数**必须在后台线程跑**（它同时承载了
/// `preLaunchSteps` 的等待，见 `D10`）。外壳的实现是纯查表（捕获五个 `String` 覆盖值 +
/// 调 `settings::project::discover`），本来就满足这两条。
pub type ToolchainResolver = Arc<dyn Fn(&str) -> Option<ToolchainPath> + Send + Sync + 'static>;

/// 计划解析失败的原因（每条都直接给用户看，所以写清"缺什么、去哪补"）。
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

/// 一条可以交给 `std::process::Command` 的命令行。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    /// 要拉起的程序（已经是平台上的真实路径，或是 `{command}` 那个裸程序名）。
    pub program: PathBuf,
    /// 参数（顺序 = 计划给的顺序，classpath 已经前置）。
    pub args: Vec<String>,
    /// 工作目录（**绝对路径**：计划给的是工作区相对路径）。
    pub cwd: PathBuf,
    /// 要写进子进程的额外环境变量（`JAVA_HOME` 一类）。
    pub env: Vec<(String, String)>,
}

/// 把一份启动计划落成命令行；需要先跑完的步骤单独返回。
///
/// 返回 `(steps, main)`：`steps` 按计划顺序跑完（任一非零退出即中止整次运行），
/// 然后才是 `main`。
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

    // 主进程的 classpath / modulepath 拼在参数前面（契约："prepends as `["-cp", joined]`
    // to the relevant argument list"）；计划的 `arguments` 已经是完整参数表。
    let main = command_spec(root, plan, resolve)?;
    Ok((steps, main))
}

/// 一条命令（主进程或一个前置步骤）的解析：`executable` + `arguments` +（可选）
/// `classpath` / `modulepath` / `environment` / `env` / `workingDirectory`。
///
/// 两种形状共用这一个函数：前置步骤与主进程的字段名在契约里是**同一套**
/// （`{ executable, arguments, classpath? }`），分开实现迟早会漂移。
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
    // classpath / modulepath 前置（`-cp` / `--module-path`）：
    // 顺序按契约（classpath 在前，modulepath 在后），只有一个时也不省掉标志。
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

    // 环境：`environment` 是"工具链引用"（要解析），`env` 是字面量（照抄）。
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
            // 字面量 `env` 后写：同一个键同时出现在两处时以用户显式设的那个为准。
            push_env(&mut env, key, text.to_string());
        }
    }

    // 工作目录：计划的相对路径 → 相对**工作区根**的绝对路径（Core 只允许 `/` 与相对路径，
    // 所以这里不会有"相对谁"的歧义）。
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

/// 写一个环境变量；同名的**替换**而不是追加（重复的 `JAVA_HOME` 会让子进程继承哪一份
/// 取决于平台实现，而契约里 `environment` 与 `env` 是"派生值"与"字面量"两个来源）。
fn push_env(env: &mut Vec<(String, String)>, key: String, value: String) {
    if let Some(existing) = env.iter_mut().find(|(name, _)| *name == key) {
        existing.1 = value;
    } else {
        env.push((key, value));
    }
}

/// `{ executable }` → 真实程序。
///
/// **不在这里派生 `JAVA_HOME`**：环境变量由计划的 `environment` 段声明（Core 对 Java 计划
/// 会发 `{"JAVA_HOME": {toolchain, property: "home"}}`），两处都设会产生重复项。
fn resolve_executable(
    executable: &Value,
    resolve: &ToolchainResolver,
) -> Result<PathBuf, PlanError> {
    if let Some(command) = executable
        .get("command")
        .and_then(Value::as_str)
        .filter(|command| !command.is_empty())
    {
        // `{command}` 是**裸程序名**（Core 拒绝带路径的命令），所以交给 `PATH` 解析：
        // `Command::new("npm")` 在 Windows 上会按 PATHEXT 找到 `npm.cmd`。
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
        // `tool: "javac"` 取同一个工具链 bin 目录里的兄弟程序（编译前置步骤用它）。
        ToolchainPath::JavaHome(home) => {
            let tool = tool.unwrap_or("java");
            Ok(home.join("bin").join(executable_name(tool)))
        }
        ToolchainPath::Program(program) => Ok(program),
    }
}

/// 工具链里的程序名：Windows 要补 `.exe`（`java` → `java.exe`）。
fn executable_name(tool: &str) -> String {
    if cfg!(windows) && !tool.to_ascii_lowercase().ends_with(".exe") {
        format!("{tool}.exe")
    } else {
        tool.to_string()
    }
}

/// `environment` 里的一项：`{ toolchain, property }` → 真值。
///
/// 目前 Core 只发 `property: "home"`（工具链根目录）。别的 property **报错而不是忽略**：
/// 静默丢掉一个环境变量会让"跑起来但行为不对"变成查不出来的问题。
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
        // Maven 那一档 Core 只把可执行文件交给我们；`home` 取它的父目录（`bin` 的上一层）
        // 是平台惯例，但**不编造**：属性对不上就报错。
        ("home", ToolchainPath::Program(program)) => program
            .parent()
            .map(|parent| parent.to_string_lossy().into_owned())
            .ok_or_else(|| PlanError::new(format!("工具链「{reference}」没有可用的 home 目录"))),
        (other, _) => Err(PlanError::new(format!(
            "启动计划请求了不支持的 environment 属性「{other}」"
        ))),
    }
}

/// 结构化路径数组 → 平台分隔符拼起来的一行；空数组/缺失给 `None`。
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
    // ⚠️ 平台分隔符只在这一处判断：Windows `;`、其它 `:`。
    let separator = if cfg!(windows) { ';' } else { ':' };
    Ok(Some(paths.join(&separator.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一个只认 `project-jdk` 的解析器（用假路径，绝不碰真机）。
    fn resolver(java_home: &str, maven: Option<&str>) -> ToolchainResolver {
        let java_home = PathBuf::from(java_home);
        let maven = maven.map(PathBuf::from);
        Arc::new(move |reference: &str| match reference {
            "project-jdk" | "java" => Some(ToolchainPath::JavaHome(java_home.clone())),
            "project-maven" | "maven" => maven.clone().map(ToolchainPath::Program),
            _ => None,
        })
    }

    /// 直接 Java 启动：工具链引用解析成 `<home>/bin/java`，`JAVA_HOME` 跟着设上，
    /// classpath 用平台分隔符拼好并前置 `-cp`。
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
                // `-m` 是 **Core** 推的，宿主只搬运：这条断言就是在钉这件事。
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

    /// 工具链里的 `tool: "javac"` 取同一个 bin 目录里的兄弟程序，并带上 classpath ——
    /// 这正是"独立 Java 先编译再运行"那一档（Core 的 `preLaunchSteps`）。
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

    /// `{command}` 形态（npm 一类）不碰工具链：直接用裸程序名，让 `PATH` 去解析。
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

    /// Maven 计划：`{toolchain: "project-maven"}` 解析成可执行文件本身（不是 `bin/mvn` 拼法），
    /// 且 `environment` 为空时完全没有额外环境变量。
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

    /// 机器上没有那个工具链时给**可执行的**提示（而不是静默换一个或 panic）。
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

    /// 不认识的 `environment.property` 报错而不是忽略：静默丢掉环境变量会让
    /// "跑起来了但行为不对"变得查不出来。
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
