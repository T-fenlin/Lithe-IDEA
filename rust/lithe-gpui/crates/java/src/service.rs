use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use lithe_gpui_shared::core_json;
use serde_json::{Value, json};

use crate::events::JavaDiagnostic;
use crate::jdtls::{self, JavaRuntime, JdtlsInstallation};
use crate::session::{Session, SessionSpec, SessionTimeouts};
use crate::workspace;

const READY_ABSOLUTE_TIMEOUT: Duration = Duration::from_secs(600);
const DEFINITION_TIMEOUT: Duration = Duration::from_secs(60);
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(15);

const HOVER_TIMEOUT: Duration = Duration::from_secs(5);

const CODE_ACTION_TIMEOUT: Duration = Duration::from_secs(15);

// 引用、重命名、格式化：与定义同一量级。JDT 在这三件事上要先做实参绑定与索引，
// 首次请求明显比后续慢，所以给的时间与 `DEFINITION_TIMEOUT` 看齐而不是向悬停看齐。
const REFERENCES_TIMEOUT: Duration = Duration::from_secs(60);
const RENAME_TIMEOUT: Duration = Duration::from_secs(60);
const FORMATTING_TIMEOUT: Duration = Duration::from_secs(60);

// 内嵌提示与代码镜头：只读、量小，跟悬停一个量级即可，不需要向定义看齐。
const INLAY_HINT_TIMEOUT: Duration = Duration::from_secs(10);
const CODE_LENS_TIMEOUT: Duration = Duration::from_secs(10);

const CODE_ACTION_DIAGNOSTIC_BACKOFF_MS: [u64; 2] = [250, 250];

#[derive(Clone, Debug, PartialEq)]
pub struct JavaCodeAction {
    pub title: String,
    pub kind: Option<String>,
    pub edits: Vec<JavaTextEdit>,
    pub other_file_edits: usize,
}

/// 一个内嵌提示（`textDocument/inlayHint`）：位置 + 要显示的那几个字。
///
/// 只留 `label`：Core 归一化里还有 `tooltip` / `paddingLeft` / `paddingRight` / `textEdits` / `data`，
/// 但组件库的 `InlayHint` 只画 `offset + text`（见 `lithe_ui::editor::decor`），多出的字段今天没有落点，
/// 带过来只会变成没人读的字段。要用时再在这里加。
#[derive(Clone, Debug, PartialEq)]
pub struct JavaInlayHint {
    pub line: u32,
    pub utf16_column: u32,
    pub label: String,
}

/// 一条 Code Lens（`textDocument/codeLens`）：所在行 + 命令标题。
///
/// `data` 是给 `codeLens/resolve` 用的回填句柄，而宿主今天**不发** resolve
/// （`SemanticRequest` 里没有 code_lens 字段），所以不接。
#[derive(Clone, Debug, PartialEq)]
pub struct JavaCodeLens {
    pub line: u32,
    pub title: String,
}

/// `textDocument/rename` 的结果：按文件分的编辑。
///
/// 宿主今天只持有**当前这一个文件**，所以把结果分成两半：落在当前文件上的可以直接落进缓冲区，
/// 落在别的文件上的只能计数（`other_file_edits`）并由调用方如实报出来——静默丢掉就是「改了一半」。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct JavaWorkspaceEdits {
    /// 当前文件的编辑，顺序即服务端给的顺序。
    pub edits: Vec<JavaTextEdit>,
    /// 落在其它文件上的编辑条数。
    pub other_file_edits: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JavaCompletionItem {
    pub label: String,
    pub insert_text: String,
    pub kind: Option<i64>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub sort_text: Option<String>,
    pub filter_text: Option<String>,
    pub text_edit: Option<JavaTextEdit>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JavaTextEdit {
    pub start: JavaPosition,
    pub end: JavaPosition,
    pub new_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JavaPosition {
    pub line: u32,
    pub utf16_column: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JavaHover {
    pub contents: String,
    pub is_markdown: bool,
    pub range: Option<(JavaPosition, JavaPosition)>,
}

#[derive(Clone, Debug)]
pub enum JavaTarget {
    File {
        path: PathBuf,
        line: u32,
        utf16_column: u32,
    },
    Virtual {
        uri: String,
        display_path: String,
        text: String,
        line: u32,
        utf16_column: u32,
    },
}

enum State {
    Idle,
    Ready(Box<Session>),
    Failed(String),
    Closed,
}

/// 一次 Maven 配置推送的结果（Core 的 `lsp.updateMavenConfiguration` 原样转述）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MavenConfigurationUpdate {
    /// 设置文档变了，JDT LS 因此会重新导入每个 Maven 工程。
    pub settings_changed: bool,
    /// 设置没变，但按要求强制刷新了工程。
    pub projects_reloaded: bool,
    /// 变化的 profile 已经开始重新应用。
    pub profiles_updating: bool,
}

pub struct JavaLanguageService {
    workspace_root: PathBuf,
    state: Mutex<State>,
    installation: Mutex<Option<(JdtlsInstallation, JavaRuntime)>>,
    /// 会话是否**此刻**可以接请求。独立于 `state` 的锁：`state` 在 `hover()` 这类方法里
    /// 会被整段持有（跨越一次 LSP 往返），问「就绪了吗」不能去抢那把锁——
    /// 以前正是抢锁把十几次悬停排成了十几秒。
    ready: AtomicBool,
}

impl JavaLanguageService {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            state: Mutex::new(State::Idle),
            installation: Mutex::new(None),
            ready: AtomicBool::new(false),
        }
    }

    pub fn lsp_session_id(&self) -> Option<String> {
        let state = self.state.lock().ok()?;
        match &*state {
            State::Ready(session) => Some(session.session_id().to_string()),
            _ => None,
        }
    }

    /// 把当前工作区的 Maven 配置推给**已经在跑**的 JDTLS。
    ///
    /// `reload_projects` 是「显式重新加载」那一档：即便配置没变也让 JDT LS 重新解析每个
    /// Maven 工程（IDEA 的「重新加载所有 Maven 项目」）。不重启会话是有意的：JDT LS 重启
    /// 会复用工作区状态并跳过 pom 没变的工程，旧配置下没解析出来的构件会继续缺着
    /// （Core 侧 `lsp.updateMavenConfiguration` 的文档就是这么写的）。
    pub fn update_maven_configuration(
        &self,
        reload_projects: bool,
    ) -> Result<MavenConfigurationUpdate, String> {
        let session_id = self
            .lsp_session_id()
            .ok_or_else(|| "Java 语言服务尚未启动".to_string())?;
        let context = workspace::maven_context(&self.workspace_root)
            .ok_or_else(|| "工作区里没有可用的 Maven 工程".to_string())?;
        let data = core_json(
            "lsp.updateMavenConfiguration",
            json!({
                "sessionId": session_id,
                "mavenContext": context,
                "reloadProjects": reload_projects,
            }),
        )
        .map_err(|error| error.to_string())?;

        let read = |key: &str| {
            data.as_ref()
                .and_then(|data| data.get(key))
                .and_then(Value::as_bool)
                .unwrap_or(false)
        };
        Ok(MavenConfigurationUpdate {
            settings_changed: read("settingsChanged"),
            projects_reloaded: read("projectsReloaded"),
            profiles_updating: read("profilesUpdating"),
        })
    }

    pub fn prepare(&self) -> Result<bool, String> {
        if !self.is_java_workspace()? {
            println!("S1_JAVA_SKIPPED reason=not-a-java-workspace");
            return Ok(false);
        }
        self.start_session()?;
        Ok(true)
    }

    /// 语言服务是否**此刻**能接请求。只读一个原子标志：不启动、不等待、不抢锁。
    ///
    /// 功能请求（悬停、注记…）拿它做门禁：没就绪就直接走降级路径，而不是排队等。
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    pub fn definition(
        &self,
        file_path: &Path,
        text: &str,
        line: u32,
        utf16_column: u32,
    ) -> Result<Option<JavaTarget>, String> {
        self.navigate("definition", file_path, text, line, utf16_column)
    }

    /// 转到实现：与 [`Self::definition`] 同一形态，只是让 JDT LS 走
    /// `textDocument/implementation`。
    pub fn implementation(
        &self,
        file_path: &Path,
        text: &str,
        line: u32,
        utf16_column: u32,
    ) -> Result<Option<JavaTarget>, String> {
        self.navigate("implementation", file_path, text, line, utf16_column)
    }

    /// 转到类型定义：`textDocument/typeDefinition`。
    pub fn type_definition(
        &self,
        file_path: &Path,
        text: &str,
        line: u32,
        utf16_column: u32,
    ) -> Result<Option<JavaTarget>, String> {
        self.navigate("typeDefinition", file_path, text, line, utf16_column)
    }


    /// 转到引用：`textDocument/references`。
    ///
    /// 与三个 Go-to 的区别是**结果是一组**：定义那三个只取第一条落下去，引用要把全部位置交回宿主，
    /// 由它列出来给用户挑。Core 已经按 LSP 要求带上 `context.includeDeclaration`，这里不用重复。
    pub fn references(
        &self,
        file_path: &Path,
        text: &str,
        line: u32,
        utf16_column: u32,
    ) -> Result<Vec<JavaTarget>, String> {
        self.ensure_session()?;
        let uri = file_uri(file_path)?;
        let started = Instant::now();

        let locations = self.with_ready_session(|session| {
            let (version, changed) = session.sync_document(&uri, "java", text)?;
            println!(
                "S1_JAVA_SYNC uri={uri} version={version} changed={changed} bytes={}",
                text.len()
            );
            let result = session.request(
                "references",
                json!({
                    "uri": uri,
                    "position": { "line": line, "utf16Column": utf16_column },
                }),
                Instant::now() + REFERENCES_TIMEOUT,
            )?;
            Ok(result
                .get("locations")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default())
        })?;

        let mut targets = Vec::new();
        let mut skipped = 0usize;
        for location in &locations {
            match self.physical_target("REFERENCE", location)? {
                Some(target) => targets.push(target),
                None => skipped += 1,
            }
        }
        println!(
            "S1_JAVA_REFERENCES uri={uri} line={line} col={utf16_column} locations={} targets={} skipped={skipped} ms={}",
            locations.len(),
            targets.len(),
            started.elapsed().as_millis()
        );
        Ok(targets)
    }

    /// 重命名符号：`textDocument/rename`，`new_name` 是用户输入的新名字。
    ///
    /// 服务端回的是 `WorkspaceEdit`（可能横跨多个文件）。这里只把当前文件的编辑翻译出来，
    /// 其余文件用条数报回调用方——本仓今天没有「跨文件批量改写」的执行层，装作改了才是真问题。
    pub fn rename(
        &self,
        file_path: &Path,
        text: &str,
        line: u32,
        utf16_column: u32,
        new_name: &str,
    ) -> Result<JavaWorkspaceEdits, String> {
        self.ensure_session()?;
        let uri = file_uri(file_path)?;
        let started = Instant::now();

        let changes = self.with_ready_session(|session| {
            let (version, changed) = session.sync_document(&uri, "java", text)?;
            println!(
                "S1_JAVA_SYNC uri={uri} version={version} changed={changed} bytes={}",
                text.len()
            );
            let result = session.request(
                "rename",
                json!({
                    "uri": uri,
                    "position": { "line": line, "utf16Column": utf16_column },
                    "newName": new_name,
                }),
                Instant::now() + RENAME_TIMEOUT,
            )?;
            Ok(result.get("changes").cloned().unwrap_or(Value::Null))
        })?;

        let edits = workspace_edits(&changes, file_path);
        println!(
            "S1_JAVA_RENAME uri={uri} line={line} col={utf16_column} newName={new_name:?} edits={} other_file_edits={} ms={}",
            edits.edits.len(),
            edits.other_file_edits,
            started.elapsed().as_millis()
        );
        Ok(edits)
    }

    /// 格式化整篇文档：`textDocument/formatting`。
    ///
    /// 格式化选项（缩进宽度、是否用空格、要不要去尾随空白/补末尾换行）由 **Core 统一给死**
    /// （`feature_request_params`），宿主不另传——否则同一个工程在两处设置里会得到两种格式。
    pub fn formatting(
        &self,
        file_path: &Path,
        text: &str,
    ) -> Result<Vec<JavaTextEdit>, String> {
        self.ensure_session()?;
        let uri = file_uri(file_path)?;
        let started = Instant::now();

        let raw = self.with_ready_session(|session| {
            let (version, changed) = session.sync_document(&uri, "java", text)?;
            println!(
                "S1_JAVA_SYNC uri={uri} version={version} changed={changed} bytes={}",
                text.len()
            );
            let result = session.request(
                "formatting",
                json!({ "uri": uri }),
                Instant::now() + FORMATTING_TIMEOUT,
            )?;
            Ok(result
                .get("edits")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default())
        })?;

        let edits: Vec<JavaTextEdit> = raw.iter().filter_map(parse_text_edit).collect();
        println!(
            "S1_JAVA_FORMATTING uri={uri} raw={} edits={} ms={}",
            raw.len(),
            edits.len(),
            started.elapsed().as_millis()
        );
        Ok(edits)
    }

    /// 内嵌提示：`textDocument/inlayHint`。
    ///
    /// LSP 要求带 `range`；这里一次要**整篇**（`0:0` 到末行行首之后），因为组件库的提示是整份替换的，
    /// 分段请求反而要自己拼接与去重。要按可见区请求是以后的事（那时得处理区间边界上的重复）。
    pub fn inlay_hints(
        &self,
        file_path: &Path,
        text: &str,
        line_count: u32,
    ) -> Result<Vec<JavaInlayHint>, String> {
        self.ensure_session()?;
        let uri = file_uri(file_path)?;
        let started = Instant::now();

        let raw = self.with_ready_session(|session| {
            let (version, changed) = session.sync_document(&uri, "java", text)?;
            println!(
                "S1_JAVA_SYNC uri={uri} version={version} changed={changed} bytes={}",
                text.len()
            );
            let result = session.request(
                "inlayHints",
                json!({
                    "uri": uri,
                    "range": {
                        "start": { "line": 0, "utf16Column": 0 },
                        "end": { "line": line_count, "utf16Column": 0 },
                    },
                }),
                Instant::now() + INLAY_HINT_TIMEOUT,
            )?;
            Ok(result
                .get("hints")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default())
        })?;

        let hints: Vec<JavaInlayHint> = raw.iter().filter_map(parse_inlay_hint).collect();
        println!(
            "S1_JAVA_INLAY_HINTS uri={uri} raw={} hints={} ms={}",
            raw.len(),
            hints.len(),
            started.elapsed().as_millis()
        );
        Ok(hints)
    }

    /// 代码镜头：`textDocument/codeLens`。
    ///
    /// 只要**已经有标题**的那些：`codeLens` 允许服务端先回一个不带 command 的占位、再让客户端发
    /// `codeLens/resolve` 换标题。宿主今天不发 resolve（见 `JavaCodeLens` 的注释），所以没有标题的条目
    /// 拿不出可显示的文字，直接跳过并在日志里报数——画一行空白比不画更糟。
    pub fn code_lenses(
        &self,
        file_path: &Path,
        text: &str,
    ) -> Result<Vec<JavaCodeLens>, String> {
        self.ensure_session()?;
        let uri = file_uri(file_path)?;
        let started = Instant::now();

        let raw = self.with_ready_session(|session| {
            let (version, changed) = session.sync_document(&uri, "java", text)?;
            println!(
                "S1_JAVA_SYNC uri={uri} version={version} changed={changed} bytes={}",
                text.len()
            );
            let result = session.request(
                "codeLens",
                json!({ "uri": uri }),
                Instant::now() + CODE_LENS_TIMEOUT,
            )?;
            Ok(result
                .get("lenses")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default())
        })?;

        let mut unresolved = 0usize;
        let mut lenses = Vec::new();
        for lens in &raw {
            match parse_code_lens(lens) {
                Some(lens) => lenses.push(lens),
                None => unresolved += 1,
            }
        }
        println!(
            "S1_JAVA_CODE_LENS uri={uri} raw={} lenses={} unresolved={unresolved} ms={}",
            raw.len(),
            lenses.len(),
            started.elapsed().as_millis()
        );
        Ok(lenses)
    }

    /// 走到「拿到一个可用会话」这一步。
    ///
    /// 四档状态里只有 `Ready` 能发请求，其余三档各有各的说法；新加的语义操作走这一条，
    /// 不再各抄一遍 match。
    fn with_ready_session<T>(
        &self,
        body: impl FnOnce(&Session) -> Result<T, String>,
    ) -> Result<T, String> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match &*state {
            State::Ready(session) => body(session),
            State::Failed(reason) => Err(reason.clone()),
            State::Closed => Err("Java 语言服务已关闭".to_string()),
            State::Idle => Err("Java 语言服务尚未启动".to_string()),
        }
    }

    /// 三个 Go-to 目标共用的取数路径。
    ///
    /// `operation` 是 Core 的 `LspSemanticOperation` 名字（`definition` /
    /// `implementation` / `typeDefinition`）：Core 自己把它映射成 LSP 方法、并按服务端声明的
    /// 能力放行，所以这三者的差别只有这一个字符串，**不要**在宿主里另写三份请求体。
    fn navigate(
        &self,
        operation: &str,
        file_path: &Path,
        text: &str,
        line: u32,
        utf16_column: u32,
    ) -> Result<Option<JavaTarget>, String> {
        self.ensure_session()?;

        let uri = file_uri(file_path)?;
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let session = match &*state {
            State::Ready(session) => &**session,
            State::Failed(reason) => return Err(reason.clone()),
            State::Closed => return Err("Java 语言服务已关闭".to_string()),
            State::Idle => return Err("Java 语言服务尚未启动".to_string()),
        };

        let (version, changed) = session.sync_document(&uri, "java", text)?;
        println!(
            "S1_JAVA_SYNC uri={uri} version={version} changed={changed} bytes={}",
            text.len()
        );

        let result = session.request(
            operation,
            json!({
                "uri": uri,
                "position": { "line": line, "utf16Column": utf16_column },
            }),
            Instant::now() + DEFINITION_TIMEOUT,
        )?;

        let locations = result
            .get("locations")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let tag = navigation_tag(operation);
        let Some(first) = locations.first() else {
            println!("S1_JAVA_{tag} uri={uri} targets=0");
            return Ok(None);
        };

        if let Some(target) = self.physical_target(tag, first)? {
            return Ok(Some(target));
        }
        self.virtual_target(session, first).map(Some)
    }

    pub fn hover(
        &self,
        file_path: &Path,
        text: &str,
        line: u32,
        utf16_column: u32,
    ) -> Result<Option<JavaHover>, String> {
        self.ensure_session()?;

        let uri = file_uri(file_path)?;
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let session = match &*state {
            State::Ready(session) => &**session,
            State::Failed(reason) => return Err(reason.clone()),
            State::Closed => return Err("Java 语言服务已关闭".to_string()),
            State::Idle => return Err("Java 语言服务尚未启动".to_string()),
        };

        let (version, changed) = session.sync_document(&uri, "java", text)?;
        println!(
            "S1_JAVA_SYNC uri={uri} version={version} changed={changed} bytes={}",
            text.len()
        );

        let started = Instant::now();
        let result = session.request(
            "hover",
            json!({
                "uri": uri,
                "position": { "line": line, "utf16Column": utf16_column },
            }),
            Instant::now() + HOVER_TIMEOUT,
        )?;

        let hover = result.get("hover").and_then(parse_hover);
        match hover.as_ref() {
            Some(hover) => println!(
                "S1_JAVA_HOVER uri={uri} line={line} col={utf16_column} markdown={} range={} chars={} ms={}",
                hover.is_markdown,
                hover.range.is_some(),
                hover.contents.len(),
                started.elapsed().as_millis()
            ),
            None => println!(
                "S1_JAVA_HOVER uri={uri} line={line} col={utf16_column} contents=0 ms={}",
                started.elapsed().as_millis()
            ),
        }
        Ok(hover)
    }

    pub fn completion(
        &self,
        file_path: &Path,
        text: &str,
        line: u32,
        utf16_column: u32,
    ) -> Result<Vec<JavaCompletionItem>, String> {
        self.ensure_session()?;

        let uri = file_uri(file_path)?;
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let session = match &*state {
            State::Ready(session) => &**session,
            State::Failed(reason) => return Err(reason.clone()),
            State::Closed => return Err("Java 语言服务已关闭".to_string()),
            State::Idle => return Err("Java 语言服务尚未启动".to_string()),
        };

        let (version, changed) = session.sync_document(&uri, "java", text)?;
        println!(
            "S1_JAVA_SYNC uri={uri} version={version} changed={changed} bytes={}",
            text.len()
        );

        let started = Instant::now();
        let result = session.request(
            "completion",
            json!({
                "uri": uri,
                "position": { "line": line, "utf16Column": utf16_column },
            }),
            Instant::now() + COMPLETION_TIMEOUT,
        );

        let result = match result {
            Ok(result) => result,
            Err(error) if is_superseded(&error) => {
                println!("S1_JAVA_COMPLETION superseded reason={error}");
                return Ok(Vec::new());
            }
            Err(error) => return Err(error),
        };

        let items: Vec<JavaCompletionItem> = result
            .get("items")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(parse_completion_item).collect())
            .unwrap_or_default();
        println!(
            "S1_JAVA_COMPLETION uri={uri} line={line} col={utf16_column} items={} ms={}",
            items.len(),
            started.elapsed().as_millis()
        );
        Ok(items)
    }

    pub fn code_actions(
        &self,
        file_path: &Path,
        text: &str,
        start: JavaPosition,
        end: JavaPosition,
    ) -> Result<Vec<JavaCodeAction>, String> {
        self.ensure_session()?;

        let uri = file_uri(file_path)?;
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let session = match &*state {
            State::Ready(session) => &**session,
            State::Failed(reason) => return Err(reason.clone()),
            State::Closed => return Err("Java 语言服务已关闭".to_string()),
            State::Idle => return Err("Java 语言服务尚未启动".to_string()),
        };

        let before = session.latest_diagnostics(&uri);
        let (version, changed) = session.sync_document(&uri, "java", text)?;
        println!(
            "S1_JAVA_SYNC uri={uri} version={version} changed={changed} bytes={}",
            text.len()
        );
        let waited = self.settle_diagnostics(session, &uri, &before, changed);

        let snapshot = session.latest_diagnostics(&uri);
        let relevant = diagnostics_for(&snapshot, start, end);
        let context: Vec<Value> = relevant.iter().map(|d| diagnostic_context(d)).collect();

        let started = Instant::now();
        let result = session.request(
            "codeActions",
            json!({
                "uri": uri,
                "range": {
                    "start": { "line": start.line, "utf16Column": start.utf16_column },
                    "end": { "line": end.line, "utf16Column": end.utf16_column },
                },
                "diagnostics": context,
            }),
            Instant::now() + CODE_ACTION_TIMEOUT,
        );

        let result = match result {
            Ok(result) => result,
            Err(error) if is_superseded(&error) => {
                println!("S1_JAVA_CODE_ACTION superseded reason={error}");
                return Ok(Vec::new());
            }
            Err(error) => return Err(error),
        };

        let raw = result
            .get("actions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut actions = Vec::new();
        let mut other_file = 0usize;
        let mut commands = 0usize;
        let mut malformed = 0usize;
        for action in &raw {
            match parse_code_action(action, file_path) {
                CodeActionOutcome::Usable(parsed) => {
                    other_file += parsed.other_file_edits;
                    actions.push(parsed);
                }
                CodeActionOutcome::Unusable {
                    other_file_edits,
                    has_command,
                } => {
                    other_file += other_file_edits;
                    if has_command {
                        commands += 1;
                    }
                }
                CodeActionOutcome::Malformed => malformed += 1,
            }
        }
        let kinds: Vec<&str> = actions
            .iter()
            .filter_map(|action| action.kind.as_deref())
            .collect();
        println!(
            "S1_JAVA_CODE_ACTION uri={uri} line={} col={} raw={} actions={} other_file_edits={other_file} commands={commands} malformed={malformed} diagnostics={} waited_ms={} kinds={kinds:?} ms={}",
            start.line,
            start.utf16_column,
            raw.len(),
            actions.len(),
            context.len(),
            waited,
            started.elapsed().as_millis()
        );
        Ok(actions)
    }

    fn settle_diagnostics(
        &self,
        session: &Session,
        uri: &str,
        before: &[JavaDiagnostic],
        changed: bool,
    ) -> u128 {
        if !changed || before.is_empty() {
            return 0;
        }
        let started = Instant::now();
        for delay in CODE_ACTION_DIAGNOSTIC_BACKOFF_MS {
            std::thread::sleep(Duration::from_millis(delay));
            if session.latest_diagnostics(uri) != before {
                break;
            }
        }
        started.elapsed().as_millis()
    }

    pub fn diagnostics(&self, file_path: &Path) -> Vec<JavaDiagnostic> {
        let uri = match file_uri(file_path) {
            Ok(uri) => uri,
            Err(error) => {
                eprintln!(
                    "S1_JAVA_DIAGNOSTICS query_failed path={} error={error}",
                    file_path.display()
                );
                return Vec::new();
            }
        };
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match &*state {
            State::Ready(session) => session.latest_diagnostics(&uri),
            _ => Vec::new(),
        }
    }

    pub fn workspace_diagnostics(&self) -> Vec<JavaFileDiagnostics> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let State::Ready(session) = &*state else {
            return Vec::new();
        };
        let mut files: Vec<JavaFileDiagnostics> = session
            .diagnostics_by_uri()
            .into_iter()
            .filter_map(|(uri, diagnostics)| {
                file_path_from_uri(&uri).map(|path| JavaFileDiagnostics { path, diagnostics })
            })
            .collect();
        files.sort_by(|left, right| left.path.cmp(&right.path));
        let problems: usize = files.iter().map(|file| file.diagnostics.len()).sum();
        println!("S1_JAVA_PROBLEMS files={} problems={problems}", files.len());
        files
    }

    pub fn sync_document(&self, file_path: &Path, text: &str) -> Result<i64, String> {
        self.ensure_session()?;

        let uri = file_uri(file_path)?;
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let session = match &*state {
            State::Ready(session) => &**session,
            State::Failed(reason) => return Err(reason.clone()),
            State::Closed => return Err("Java 语言服务已关闭".to_string()),
            State::Idle => return Err("Java 语言服务尚未启动".to_string()),
        };

        let (version, changed) = session.sync_document(&uri, "java", text)?;
        println!(
            "S1_JAVA_SYNC uri={uri} version={version} changed={changed} bytes={}",
            text.len()
        );
        Ok(version)
    }

    pub fn shutdown(&self) {
        self.ready.store(false, Ordering::Release);
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let State::Ready(session) = std::mem::replace(&mut *state, State::Closed) {
            session.shutdown();
        } else {
            *state = State::Closed;
        }
    }

    /// 只取**已经存在**的会话：不启动、不等待。
    ///
    /// 启动是 [`Self::start_session`]（只有 [`Self::prepare`] 走）的职责。以前这里会
    /// 顺手启动，于是任何一个功能请求撞上「还没启动」都会在状态锁里等满就绪上限
    /// （600 秒），期间**其他所有请求都排在锁后面**：实测一次悬停挂了 600 秒、另一次 18 秒，
    /// 而它们本来都该立刻降级。
    fn ensure_session(&self) -> Result<(), String> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match &*state {
            State::Ready(_) => Ok(()),
            State::Failed(reason) => Err(reason.clone()),
            State::Closed => Err("Java 语言服务已关闭".to_string()),
            State::Idle => Err("Java 语言服务尚未启动".to_string()),
        }
    }

    /// 启动会话。这条路上等满就绪上限是合理的（它在后台执行器上跑），所以只给 prepare 用。
    fn start_session(&self) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match &*state {
            State::Ready(_) => return Ok(()),
            State::Failed(reason) => return Err(reason.clone()),
            State::Closed => return Err("Java 语言服务已关闭".to_string()),
            State::Idle => {}
        }

        let result = self.start_locked();
        match result {
            Ok(session) => {
                *state = State::Ready(Box::new(session));
                self.ready.store(true, Ordering::Release);
                Ok(())
            }
            Err(reason) => {
                println!("S1_JAVA_FAILED reason={reason}");
                *state = State::Failed(reason.clone());
                self.ready.store(false, Ordering::Release);
                Err(reason)
            }
        }
    }

    fn start_locked(&self) -> Result<Session, String> {
        let installation = jdtls::resolve(&self.workspace_root)?;
        let runtime = jdtls::resolve_runtime(installation_root(&installation.executable))?;
        println!(
            "S1_JAVA_JDTLS executable={} version={} java={} javaVersion={} launcher={} config={}",
            installation.executable.display(),
            installation.version,
            runtime.executable.display(),
            runtime.version,
            installation.launcher_jar_path.display(),
            installation.configuration_directory.display(),
        );

        let cache = workspace::plan(&self.workspace_root, &installation.version)?;
        println!(
            "S1_JAVA_CACHE directory={} key={} reuse={} stateDirectory={}",
            cache.cache_directory.display(),
            cache.workspace_key,
            cache.state_directory_existed,
            cache.state_directory().display(),
        );

        match workspace::cleanup_expired(&cache.cache_directory, &cache.workspace_key) {
            Ok(outcome) => println!(
                "S1_JAVA_CACHE_RECLAIM retentionDays={} removed={}",
                outcome.retention_days, outcome.removed
            ),
            Err(error) => println!("S1_JAVA_CACHE_RECLAIM_FAILED error={error}"),
        }

        let maven_context = workspace::maven_context(&self.workspace_root);
        match &maven_context {
            Some(context) => println!(
                "S1_JAVA_MAVEN context reactorPath={} profiles={} settings={}",
                context
                    .get("reactorPath")
                    .and_then(Value::as_str)
                    .unwrap_or("."),
                context
                    .get("profiles")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0),
                context.get("settingsPath").is_some(),
            ),
            None => println!("S1_JAVA_MAVEN context=none (no readable pom.xml)"),
        }

        let root_uri = directory_uri(&self.workspace_root)?;
        let spec = SessionSpec {
            workspace_root: &self.workspace_root,
            root_uri: &root_uri,
            installation: &installation,
            java_executable: &runtime.executable,
            java_home: &runtime.home,
            maven_context,
            java_runtimes: vec![(runtime.home.clone(), runtime.version.clone())],
            cache_directory: &cache.cache_directory,
            workspace_fingerprint: &cache.workspace_fingerprint,
            timeouts: SessionTimeouts::default(),
        };

        println!(
            "S1_JAVA_START rootUri={root_uri} workingDirectory={}",
            self.workspace_root.display()
        );
        let mut session = Session::start(&spec)?;
        let report = session.wait_ready(Instant::now() + READY_ABSOLUTE_TIMEOUT)?;
        println!(
            "S1_JAVA_READY elapsedMs={} logEvents={} serverInfo={}",
            report.elapsed.as_millis(),
            report.log_events,
            report.server_info.as_deref().unwrap_or("-"),
        );

        *self
            .installation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some((installation, runtime));
        Ok(session)
    }

    fn is_java_workspace(&self) -> Result<bool, String> {
        let data = core_json(
            "workspace.snapshot",
            json!({ "root": self.workspace_root.to_string_lossy() }),
        )
        .map_err(|error| match error.code() {
            Some(code) => format!("{code}：{error}"),
            None => error.to_string(),
        })?;
        let paths: Vec<String> = data
            .as_ref()
            .and_then(|data| data.get("files"))
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if paths.is_empty() {
            return Ok(false);
        }

        let plan = core_json(
            "java.workspacePolicy",
            json!({ "workspacePaths": paths, "changedPaths": [] }),
        )
        .map_err(|error| match error.code() {
            Some(code) => format!("{code}：{error}"),
            None => error.to_string(),
        })?;
        Ok(plan
            .as_ref()
            .and_then(|plan| plan.get("shouldStart"))
            .and_then(Value::as_bool)
            .unwrap_or(false))
    }

    fn physical_target(&self, tag: &str, location: &Value) -> Result<Option<JavaTarget>, String> {
        let uri = location
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| "定义结果缺少 uri".to_string())?;
        let Ok(url) = url::Url::parse(uri) else {
            return Ok(None);
        };
        if !url.scheme().eq_ignore_ascii_case("file") {
            return Ok(None);
        }
        let path = url
            .to_file_path()
            .map_err(|_| format!("无法把 file URI 还原成路径：{uri}"))?;
        let (line, utf16_column) = location_position(location)?;
        println!(
            "S1_JAVA_{tag} target=file path={} line={} col={}",
            path.display(),
            line + 1,
            utf16_column + 1
        );
        Ok(Some(JavaTarget::File {
            path,
            line,
            utf16_column,
        }))
    }

    fn virtual_target(&self, session: &Session, location: &Value) -> Result<JavaTarget, String> {
        let uri = location
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| "定义结果既没有 filePath 也没有 uri".to_string())?
            .to_string();
        let display_path = location
            .get("displayPath")
            .and_then(Value::as_str)
            .unwrap_or(&uri)
            .to_string();
        let (line, utf16_column) = location_position(location)?;

        let result = session.request(
            "virtualDocument",
            json!({ "virtualUri": uri }),
            Instant::now() + DEFINITION_TIMEOUT,
        )?;
        let text = result
            .get("text")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| format!("virtualDocument 没有返回源码：{uri}"))?
            .to_string();
        println!(
            "S1_JAVA_VIRTUAL uri={uri} displayPath={display_path} bytes={} line={} col={}",
            text.len(),
            line + 1,
            utf16_column + 1
        );
        Ok(JavaTarget::Virtual {
            uri,
            display_path,
            text,
            line,
            utf16_column,
        })
    }
}

/// 诊断行里的动作名，与 Core 的语义操作名一一对应。
///
/// 三个 Go-to 共用 [`JavaLanguageService::navigate`]，日志若一律写 `DEFINITION` 就分不出
/// 「转到实现真的问了 JDT LS」还是「退回定义那条路」。
fn navigation_tag(operation: &str) -> &'static str {
    match operation {
        "implementation" => "IMPLEMENTATION",
        "typeDefinition" => "TYPE_DEFINITION",
        _ => "DEFINITION",
    }
}

fn location_position(location: &Value) -> Result<(u32, u32), String> {
    let start = location
        .get("range")
        .and_then(|range| range.get("start"))
        .ok_or_else(|| "定义结果缺少 range.start".to_string())?;
    let line = start
        .get("line")
        .and_then(Value::as_u64)
        .ok_or_else(|| "定义结果缺少 range.start.line".to_string())?;
    let utf16_column = start
        .get("utf16Column")
        .and_then(Value::as_u64)
        .ok_or_else(|| "定义结果缺少 range.start.utf16Column".to_string())?;
    Ok((line as u32, utf16_column as u32))
}

fn installation_root(executable: &Path) -> &Path {
    let parent = executable.parent().unwrap_or(Path::new(""));
    if parent
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("bin"))
    {
        parent.parent().unwrap_or(parent)
    } else {
        parent
    }
}

pub fn file_uri(path: &Path) -> Result<String, String> {
    url::Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|_| format!("无法把路径转成 file URI：{}", path.display()))
}

fn file_path_from_uri(uri: &str) -> Option<PathBuf> {
    let url = url::Url::parse(uri).ok()?;
    if url.scheme() != "file" {
        return None;
    }
    url.to_file_path().ok()
}

#[derive(Clone, Debug, PartialEq)]
pub struct JavaFileDiagnostics {
    pub path: PathBuf,
    pub diagnostics: Vec<JavaDiagnostic>,
}

fn directory_uri(path: &Path) -> Result<String, String> {
    url::Url::from_directory_path(path)
        .map(|url| url.to_string())
        .map_err(|_| format!("无法把工作区路径转成 file URI：{}", path.display()))
}

fn is_superseded(error: &str) -> bool {
    error.starts_with("staleDocumentVersion@") || error.starts_with("requestCancelled@")
}

#[derive(Debug)]
enum CodeActionOutcome {
    Usable(JavaCodeAction),
    Unusable {
        other_file_edits: usize,
        has_command: bool,
    },
    Malformed,
}

fn parse_code_action(value: &Value, file_path: &Path) -> CodeActionOutcome {
    let Some(title) = value.get("title").and_then(Value::as_str) else {
        return CodeActionOutcome::Malformed;
    };
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let mut edits: Vec<JavaTextEdit> = Vec::new();
    let mut other_file_edits = 0usize;
    if let Some(changes) = value
        .get("edit")
        .and_then(|edit| edit.get("changes"))
        .and_then(Value::as_object)
    {
        for (key, list) in changes {
            let parsed: Vec<JavaTextEdit> = list
                .as_array()
                .map(|edits| edits.iter().filter_map(parse_text_edit).collect())
                .unwrap_or_default();
            if is_current_file(key, file_path) {
                edits.extend(parsed);
            } else {
                other_file_edits += parsed.len();
            }
        }
    }

    let has_command = value
        .get("command")
        .is_some_and(|command| !command.is_null());
    if edits.is_empty() || has_command {
        return CodeActionOutcome::Unusable {
            other_file_edits,
            has_command,
        };
    }

    edits.sort_by_key(|edit| std::cmp::Reverse((edit.start.line, edit.start.utf16_column)));

    CodeActionOutcome::Usable(JavaCodeAction {
        title: title.to_string(),
        kind,
        edits,
        other_file_edits,
    })
}

/// 把服务端回的 `WorkspaceEdit.changes` 拆成「当前文件能落的」与「别的文件的条数」。
///
/// Core 已经把 `changes` 的键从 URI 归一成了**文件路径**（`parse_workspace_edit` 走
/// `file_path_from_uri`），所以这里按路径比对，而不是再解一次 URI。
fn workspace_edits(changes: &Value, file_path: &Path) -> JavaWorkspaceEdits {
    let mut edits = Vec::new();
    let mut other_file_edits = 0usize;
    if let Some(entries) = changes.as_object() {
        for (key, list) in entries {
            let parsed: Vec<JavaTextEdit> = list
                .as_array()
                .map(|list| list.iter().filter_map(parse_text_edit).collect())
                .unwrap_or_default();
            if is_current_file(key, file_path) {
                edits.extend(parsed);
            } else {
                other_file_edits += parsed.len();
            }
        }
    }
    JavaWorkspaceEdits {
        edits,
        other_file_edits,
    }
}

/// 一条内嵌提示：位置必填，`label` 是字符串（Core 已把片段数组拼成字符串）。
fn parse_inlay_hint(value: &Value) -> Option<JavaInlayHint> {
    let position = value.get("position")?;
    let line = position.get("line").and_then(Value::as_u64)? as u32;
    let utf16_column = position.get("utf16Column").and_then(Value::as_u64)? as u32;
    let label = value.get("label").and_then(Value::as_str)?.to_string();
    if label.is_empty() {
        return None;
    }
    Some(JavaInlayHint {
        line,
        utf16_column,
        label,
    })
}

/// 一条 Code Lens：行号来自 `range.start`，标题来自 `command.title`。
///
/// 没有 `command`（或没有标题）的返回 `None`：那是等着 `codeLens/resolve` 回填的占位条目，
/// 而宿主今天不发 resolve，硬画会得到一行没有文字的东西。
fn parse_code_lens(value: &Value) -> Option<JavaCodeLens> {
    let line = value
        .get("range")
        .and_then(|range| range.get("start"))
        .and_then(|start| start.get("line"))
        .and_then(Value::as_u64)? as u32;
    let title = value
        .get("command")
        .and_then(|command| command.get("title"))
        .and_then(Value::as_str)
        .filter(|title| !title.is_empty())?
        .to_string();
    Some(JavaCodeLens { line, title })
}

fn is_current_file(key: &str, file_path: &Path) -> bool {
    let trimmed = key
        .strip_prefix('/')
        .filter(|rest| is_windows_drive_prefix(rest))
        .unwrap_or(key);
    Path::new(trimmed) == file_path
}

fn is_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn diagnostics_for(
    diagnostics: &[JavaDiagnostic],
    start: JavaPosition,
    end: JavaPosition,
) -> Vec<&JavaDiagnostic> {
    let empty_selection = start == end;
    diagnostics
        .iter()
        .filter(|diagnostic| {
            if empty_selection {
                let line = start.line;
                diagnostic.range.start.line <= line && line <= diagnostic.range.end.line
            } else {
                let a = (start.line, start.utf16_column);
                let b = (end.line, end.utf16_column);
                let d_start = (
                    diagnostic.range.start.line,
                    diagnostic.range.start.utf16_column,
                );
                let d_end = (diagnostic.range.end.line, diagnostic.range.end.utf16_column);
                !(d_end <= a || b <= d_start)
            }
        })
        .collect()
}

fn diagnostic_context(diagnostic: &JavaDiagnostic) -> Value {
    json!({
        "range": {
            "start": {
                "line": diagnostic.range.start.line,
                "utf16Column": diagnostic.range.start.utf16_column,
            },
            "end": {
                "line": diagnostic.range.end.line,
                "utf16Column": diagnostic.range.end.utf16_column,
            },
        },
        "severity": diagnostic.severity,
        "message": diagnostic.message,
        "source": diagnostic.source,
        "code": diagnostic.code,
    })
}

fn parse_completion_item(value: &Value) -> Option<JavaCompletionItem> {
    let label = value.get("label").and_then(Value::as_str)?.to_string();
    let insert_text_raw = value
        .get("insertText")
        .and_then(Value::as_str)
        .unwrap_or(label.as_str());
    let is_snippet = value.get("insertTextFormat").and_then(Value::as_u64) == Some(2);

    let mut text_edit = value.get("textEdit").and_then(parse_text_edit);
    let insert_text = if is_snippet {
        let plain = plain_snippet(insert_text_raw);
        if let Some(edit) = text_edit.as_mut() {
            edit.new_text = plain_snippet(&edit.new_text);
        }
        plain
    } else {
        insert_text_raw.to_string()
    };

    Some(JavaCompletionItem {
        label,
        insert_text,
        kind: value.get("kind").and_then(Value::as_i64),
        detail: value
            .get("detail")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        documentation: value
            .get("documentation")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        sort_text: value
            .get("sortText")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        filter_text: value
            .get("filterText")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        text_edit,
    })
}

fn parse_text_edit(value: &Value) -> Option<JavaTextEdit> {
    let range = value.get("range")?;
    Some(JavaTextEdit {
        start: parse_position(range.get("start")?)?,
        end: parse_position(range.get("end")?)?,
        new_text: value
            .get("newText")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
    })
}

pub(crate) fn parse_position(value: &Value) -> Option<JavaPosition> {
    Some(JavaPosition {
        line: value.get("line").and_then(Value::as_u64)? as u32,
        utf16_column: value.get("utf16Column").and_then(Value::as_u64)? as u32,
    })
}

pub fn parse_hover(value: &Value) -> Option<JavaHover> {
    let contents = value.get("contents").and_then(Value::as_str)?.to_string();
    if contents.is_empty() {
        return None;
    }
    let range = value.get("range").and_then(|range| {
        Some((
            parse_position(range.get("start")?)?,
            parse_position(range.get("end")?)?,
        ))
    });
    Some(JavaHover {
        contents,
        is_markdown: value
            .get("isMarkdown")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        range,
    })
}

fn plain_snippet(value: &str) -> String {
    match core_json("lsp.plainSnippet", json!({ "value": value })) {
        Ok(Some(response)) => match response.get("text").and_then(Value::as_str) {
            Some(text) => text.to_string(),
            None => {
                eprintln!("S1_JAVA_COMPLETION plain_snippet=missing_text_field");
                value.to_string()
            }
        },
        Ok(None) => value.to_string(),
        Err(error) => {
            eprintln!("S1_JAVA_COMPLETION plain_snippet=failed error={error}");
            value.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 功能请求**不许**顺手启动语言服务。
    ///
    /// 旧实现里每个功能请求都会在拿不到会话时去启动：那条路要在状态锁里等满就绪上限，
    /// 期间同一把锁上的所有请求（悬停、诊断、注记…）一起排队。实测一次悬停挂了 600 秒、
    /// 另一次 18 秒。这里断言两件事：立刻失败、且没有把状态推进成已就绪。
    #[test]
    fn feature_requests_do_not_start_the_language_service() {
        let service = JavaLanguageService::new(PathBuf::from("C:/lithe-no-such-workspace"));
        assert!(!service.is_ready(), "新建的服务不该是就绪的");

        let hover = service.hover(Path::new("Foo.java"), "class Foo {}", 0, 0);
        assert!(
            hover.as_ref().is_err_and(|reason| reason.contains("尚未启动")),
            "未启动时悬停应当立刻报未启动，实际：{hover:?}"
        );
        let sync = service.sync_document(Path::new("Foo.java"), "class Foo {}");
        assert!(sync.is_err_and(|reason| reason.contains("尚未启动")));
        assert!(!service.is_ready(), "功能请求不该把服务启动起来");
    }

    #[test]
    fn installation_root_skips_a_bin_directory() {
        assert_eq!(
            installation_root(Path::new("/opt/jdtls/bin/jdtls")),
            Path::new("/opt/jdtls")
        );
        assert_eq!(
            installation_root(Path::new("/opt/jdtls/jdtls")),
            Path::new("/opt/jdtls")
        );
    }

    #[test]
    fn directory_uri_encodes_spaces() {
        let path = std::env::temp_dir().join("lithe java ws");
        let uri = directory_uri(&path).expect("uri");
        assert!(uri.starts_with("file:///"), "{uri}");
        assert!(uri.ends_with('/'), "{uri}");
        assert!(uri.contains("lithe%20java%20ws"), "{uri}");
    }

    #[test]
    fn hover_reads_the_normalized_fields() {
        let hover = parse_hover(&json!({
            "contents": "**greet**(String name) — 打招呼",
            "isMarkdown": true,
            "range": {
                "start": { "line": 3, "utf16Column": 28 },
                "end": { "line": 3, "utf16Column": 33 }
            }
        }))
        .expect("带 contents 的悬停必须能解析");
        assert_eq!(hover.contents, "**greet**(String name) — 打招呼");
        assert!(hover.is_markdown);
        let (start, end) = hover.range.expect("给了 range 就必须解析出来");
        assert_eq!(start.line, 3);
        assert_eq!(start.utf16_column, 28);
        assert_eq!(end.utf16_column, 33);

        let hover = parse_hover(&json!({
            "contents": "`launch`",
            "isMarkdown": false,
            "range": {
                "start": { "line": 0, "utf16Column": 3 },
                "end": { "line": 0, "utf16Column": 9 }
            }
        }))
        .expect("轻量悬停同一份解析");
        assert_eq!(hover.contents, "`launch`");
        assert!(!hover.is_markdown);
        assert_eq!(hover.range.map(|(start, _)| start.utf16_column), Some(3));

        let hover =
            parse_hover(&json!({ "contents": "x", "range": null })).expect("没有 range 也要能解析");
        assert_eq!(hover.range, None);
        assert!(!hover.is_markdown, "缺 isMarkdown 时按纯文本处理");

        assert_eq!(parse_hover(&json!({ "contents": "" })), None);
        assert_eq!(parse_hover(&json!({ "contents": 3 })), None);
        assert_eq!(parse_hover(&json!({ "hover": null })), None);
    }

    #[test]
    fn completion_item_reads_the_normalized_fields() {
        let item = parse_completion_item(&json!({
            "label": "greet(String)",
            "insertText": "greet(${1:name})",
            "insertTextFormat": 2,
            "kind": 2,
            "detail": "String Greeter.greet(String name)",
            "documentation": "打招呼",
            "sortText": "0001",
            "filterText": "greet",
            "textEdit": {
                "range": {
                    "start": { "line": 3, "utf16Column": 28 },
                    "end": { "line": 3, "utf16Column": 31 }
                },
                "newText": "greet(${1:name})"
            }
        }))
        .expect("有 label 的条目必须能解析");

        assert_eq!(item.label, "greet(String)");
        assert_eq!(item.kind, Some(2));
        assert_eq!(
            item.detail.as_deref(),
            Some("String Greeter.greet(String name)")
        );
        assert_eq!(item.sort_text.as_deref(), Some("0001"));
        assert_eq!(item.insert_text, "greet(name)");
        let edit = item.text_edit.expect("必须带上 textEdit，否则会插重复");
        assert_eq!(edit.new_text, "greet(name)");
        assert_eq!(edit.start.line, 3);
        assert_eq!(edit.start.utf16_column, 28);
        assert_eq!(edit.end.utf16_column, 31);
    }

    #[test]
    fn completion_item_requires_a_label_and_keeps_plain_text() {
        assert!(parse_completion_item(&json!({ "insertText": "x" })).is_none());

        let item = parse_completion_item(&json!({
            "label": "println",
            "insertText": "println",
            "insertTextFormat": 1,
        }))
        .expect("普通文本条目");
        assert_eq!(item.insert_text, "println");
        assert!(item.text_edit.is_none(), "Core 没给 textEdit 就该是 None");
    }

    #[test]
    fn location_position_reads_the_normalized_range() {
        let location = json!({
            "uri": "file:///ws/src/demo/Greeter.java",
            "filePath": "/ws/src/demo/Greeter.java",
            "range": {
                "start": { "line": 2, "utf16Column": 15 },
                "end": { "line": 2, "utf16Column": 22 }
            },
            "isReadOnly": false,
            "displayPath": Value::Null,
        });
        assert_eq!(location_position(&location), Ok((2, 15)));
        assert!(location_position(&json!({ "range": { "start": { "line": 2 } } })).is_err());
    }

    #[test]
    fn superseded_matches_the_event_pump_error_shape() {
        assert!(is_superseded(
            "staleDocumentVersion@document：文档版本已变旧"
        ));
        assert!(is_superseded("requestCancelled@request：（无消息）"));
        assert!(!is_superseded("requestTimeout@request：语义请求超时"));
        assert!(!is_superseded(
            "事件泵已退出（waitEvents: process_failed：会话已不在运行）"
        ));
    }

    fn code_action_fixture() -> Value {
        json!({
            "title": "Import 'List' (java.util)",
            "kind": "quickfix",
            "isPreferred": false,
            "edit": {
                "changes": {
                    "/C:/ws/src/main/java/demo/ImportFix.java": [{
                        "range": {
                            "start": { "line": 0, "utf16Column": 0 },
                            "end": { "line": 2, "utf16Column": 0 }
                        },
                        "newText": "package demo;\n\nimport java.util.List;\n\n"
                    }]
                }
            },
            "command": Value::Null,
            "data": Value::Null
        })
    }

    #[test]
    fn code_action_reads_edits_for_the_current_file() {
        let path = PathBuf::from(r"C:\ws\src\main\java\demo\ImportFix.java");
        let CodeActionOutcome::Usable(action) = parse_code_action(&code_action_fixture(), &path)
        else {
            panic!(
                "这条 action 能落地：{:?}",
                parse_code_action(&code_action_fixture(), &path)
            );
        };
        assert_eq!(action.title, "Import 'List' (java.util)");
        assert_eq!(action.kind.as_deref(), Some("quickfix"));
        assert_eq!(action.other_file_edits, 0);
        assert_eq!(action.edits.len(), 1);
        let edit = &action.edits[0];
        assert_eq!(edit.start.line, 0);
        assert_eq!(edit.start.utf16_column, 0);
        assert_eq!(edit.end.line, 2);
        assert_eq!(edit.new_text, "package demo;\n\nimport java.util.List;\n\n");
    }

    #[test]
    fn code_action_for_another_file_is_reported_not_returned() {
        let action = json!({
            "title": "Create class 'List<T>'",
            "kind": "quickfix",
            "edit": {
                "changes": {
                    "/C:/ws/src/main/java/demo/List.java": [{
                        "range": {
                            "start": { "line": 0, "utf16Column": 0 },
                            "end": { "line": 0, "utf16Column": 0 }
                        },
                        "newText": "package demo;\n"
                    }]
                }
            },
            "command": Value::Null
        });
        match parse_code_action(
            &action,
            Path::new(r"C:\ws\src\main\java\demo\ImportFix.java"),
        ) {
            CodeActionOutcome::Unusable {
                other_file_edits,
                has_command,
            } => {
                assert_eq!(
                    other_file_edits, 1,
                    "别的文件的编辑条数要报出来（诊断行靠它）"
                );
                assert!(!has_command, "这条不是 command 型");
            }
            other => panic!("只落在别人的文件上就该判 Unusable：{other:?}"),
        }
    }

    #[test]
    fn code_action_tolerates_missing_and_malformed_fields() {
        let path = Path::new(r"C:\ws\demo\A.java");
        for broken in [
            json!({ "kind": "quickfix", "edit": Value::Null }),
            json!({}),
        ] {
            assert!(
                matches!(
                    parse_code_action(&broken, path),
                    CodeActionOutcome::Malformed
                ),
                "缺 title 应当判 Malformed：{broken}"
            );
        }
        for broken in [
            json!({ "title": "t" }),
            json!({ "title": "t", "edit": Value::Null }),
            json!({ "title": "", "edit": { "changes": Value::Null } }),
            json!({ "title": "t", "edit": { "changes": { "/C:/ws/demo/A.java": "nope" } } }),
            json!({ "title": "t", "edit": { "changes": { "/C:/ws/demo/A.java": [{ "newText": "x" }] } } }),
        ] {
            assert!(
                matches!(
                    parse_code_action(&broken, path),
                    CodeActionOutcome::Unusable { .. }
                ),
                "没有可落地编辑的应当判 Unusable：{broken}"
            );
        }
        let with_command = json!({
            "title": "Organize Imports",
            "kind": "source.organizeImports",
            "edit": {
                "changes": {
                    "/C:/ws/demo/A.java": [{
                        "range": {
                            "start": { "line": 0, "utf16Column": 0 },
                            "end": { "line": 0, "utf16Column": 0 }
                        },
                        "newText": "x"
                    }]
                }
            },
            "command": { "title": "Organize Imports", "command": "java.edit.organizeImports", "arguments": [] }
        });
        assert!(matches!(
            parse_code_action(&with_command, Path::new(r"C:\ws\demo\A.java")),
            CodeActionOutcome::Unusable {
                has_command: true,
                ..
            }
        ));
    }

    #[test]
    fn code_action_edits_are_ordered_back_to_front() {
        let action = json!({
            "title": "two edits",
            "kind": "quickfix",
            "edit": {
                "changes": {
                    "/C:/ws/demo/A.java": [
                        { "range": { "start": { "line": 1, "utf16Column": 0 }, "end": { "line": 1, "utf16Column": 0 } }, "newText": "first" },
                        { "range": { "start": { "line": 9, "utf16Column": 4 }, "end": { "line": 9, "utf16Column": 8 } }, "newText": "second" }
                    ]
                }
            }
        });
        let CodeActionOutcome::Usable(parsed) =
            parse_code_action(&action, Path::new(r"C:\ws\demo\A.java"))
        else {
            panic!("能落地");
        };
        let order: Vec<u32> = parsed.edits.iter().map(|edit| edit.start.line).collect();
        assert_eq!(order, vec![9, 1], "必须按位置倒序");
    }

    #[test]
    fn relevant_diagnostics_follow_the_cursor_line() {
        let snapshot = vec![
            diagnostic_at(3, 4, 3, 8),
            diagnostic_at(7, 0, 7, 5),
            diagnostic_at(9, 2, 11, 3),
        ];
        let picked = diagnostics_for(&snapshot, position(3, 0), position(3, 0));
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].range.start.utf16_column, 4);

        let picked = diagnostics_for(&snapshot, position(10, 0), position(10, 0));
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].range.start.line, 9);

        assert!(diagnostics_for(&snapshot, position(5, 0), position(5, 0)).is_empty());
    }

    #[test]
    fn relevant_diagnostics_follow_a_selection() {
        let snapshot = vec![diagnostic_at(3, 4, 3, 8), diagnostic_at(7, 0, 7, 5)];
        let picked = diagnostics_for(&snapshot, position(7, 0), position(7, 5));
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].range.start.line, 7);

        let picked = diagnostics_for(&snapshot, position(0, 0), position(9, 0));
        assert_eq!(picked.len(), 2);

        assert!(diagnostics_for(&snapshot, position(8, 0), position(8, 2)).is_empty());
    }

    #[test]
    fn diagnostic_context_keeps_the_fields_jdt_reads() {
        let value = diagnostic_context(&diagnostic_at(3, 4, 3, 8));
        assert_eq!(value["source"], "Java");
        assert_eq!(value["code"], "16777218");
        assert_eq!(value["severity"], 1);
        assert_eq!(value["range"]["start"]["utf16Column"], 4);
        assert_eq!(value["range"]["end"]["line"], 3);
        assert!(!value["message"].as_str().unwrap_or_default().is_empty());
    }

    fn diagnostic_at(
        start_line: u32,
        start_column: u32,
        end_line: u32,
        end_column: u32,
    ) -> JavaDiagnostic {
        JavaDiagnostic {
            range: crate::events::JavaDiagnosticRange {
                start: position(start_line, start_column),
                end: position(end_line, end_column),
            },
            severity: Some(1),
            message: "List cannot be resolved to a type".to_string(),
            source: Some("Java".to_string()),
            code: Some("16777218".to_string()),
            tags: Vec::new(),
        }
    }

    fn position(line: u32, utf16_column: u32) -> JavaPosition {
        JavaPosition { line, utf16_column }
    }

    const SMOKE_ENV: &str = "LITHE_GPUI_JDTLS_SMOKE";

    const GREETER: &str = concat!(
        "package demo;\n",
        "\n",
        "public class Greeter {\n",
        "    public String greet(String name) {\n",
        "        return \"Hello, \" + name;\n",
        "    }\n",
        "}\n",
    );
    const APP: &str = concat!(
        "package demo;\n",
        "\n",
        "public class App {\n",
        "    public static void main(String[] args) {\n",
        "        Greeter greeter = new Greeter();\n",
        "        System.out.println(greeter.greet(\"world\"));\n",
        "    }\n",
        "}\n",
    );

    struct TempWorkspace(PathBuf);

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn real_jdtls_resolves_a_cross_file_definition() {
        if std::env::var_os(SMOKE_ENV).is_none_or(|value| value.is_empty()) {
            return;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时钟应在 Unix 纪元之后")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "lithe-gpui-java-smoke-{}-{stamp}",
            std::process::id()
        ));
        let _cleanup = TempWorkspace(root.clone());
        let sources = root.join("src").join("main").join("java").join("demo");
        std::fs::create_dir_all(&sources).expect("fixture 源码目录");
        std::fs::write(sources.join("Greeter.java"), GREETER).expect("Greeter.java");
        std::fs::write(sources.join("App.java"), APP).expect("App.java");
        std::fs::write(
            root.join("pom.xml"),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                "<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n",
                "  <modelVersion>4.0.0</modelVersion>\n",
                "  <groupId>demo</groupId>\n",
                "  <artifactId>lithe-smoke</artifactId>\n",
                "  <version>1.0.0</version>\n",
                "  <properties><maven.compiler.release>21</maven.compiler.release></properties>\n",
                "</project>\n",
            ),
        )
        .expect("pom.xml");

        let service = JavaLanguageService::new(root.clone());
        struct Shutdown<'a>(&'a JavaLanguageService);
        impl Drop for Shutdown<'_> {
            fn drop(&mut self) {
                self.0.shutdown();
            }
        }
        let _shutdown = Shutdown(&service);

        let prepared = service.prepare().expect("真实 JDTLS 应当启动");
        assert!(prepared, "`src/demo/*.java` 的工程应当被判为 Java 工作区");

        let line = APP
            .lines()
            .position(|line| line.contains("greeter.greet"))
            .expect("fixture 里有 greet 调用") as u32;
        let utf16_column = APP
            .lines()
            .nth(line as usize)
            .and_then(|line| line.find("greet("))
            .expect("fixture 里有 greet 调用") as u32;
        let app_path = sources.join("App.java");

        let target = service
            .definition(&app_path, APP, line, utf16_column)
            .unwrap_or_else(|error| panic!("语义定义请求失败：{error}"))
            .unwrap_or_else(|| panic!("JDT 没有给出 `greet` 的定义位置"));

        let JavaTarget::File {
            path,
            line: target_line,
            ..
        } = target
        else {
            panic!("`greet` 的定义在工程源码里，不该是虚拟源码");
        };
        assert_eq!(
            path.canonicalize().ok(),
            sources.join("Greeter.java").canonicalize().ok(),
            "跨文件定义必须落在 Greeter.java"
        );
        assert_eq!(
            target_line, 3,
            "`greet` 声明在 Greeter.java 第 4 行（0 基 3）"
        );

        let system_line = APP
            .lines()
            .position(|line| line.contains("System.out.println"))
            .expect("fixture 里有 System 使用") as u32;
        let system_column = APP
            .lines()
            .nth(system_line as usize)
            .and_then(|line| line.find("System"))
            .expect("fixture 里有 System 使用") as u32;
        let target = service
            .definition(&app_path, APP, system_line, system_column)
            .unwrap_or_else(|error| panic!("JDK 类的语义定义请求失败：{error}"))
            .unwrap_or_else(|| panic!("JDT 没有给出 `System` 的定义位置"));
        let JavaTarget::Virtual {
            uri,
            display_path,
            text,
            ..
        } = target
        else {
            panic!("JDK 类不可能落在工程源码里，必须是虚拟源码");
        };
        assert!(uri.starts_with("jdt://"), "{uri}");
        assert!(display_path.ends_with("System.java"), "{display_path}");
        assert!(
            text.contains("class System"),
            "虚拟文档应当是 System 的源码：{}",
            &text[..text.len().min(200)]
        );

        let dot_line = APP
            .lines()
            .position(|line| line.contains("greeter.greet"))
            .expect("fixture 里有 greeter 调用") as u32;
        let dot_column = APP
            .lines()
            .nth(dot_line as usize)
            .and_then(|line| line.find("greeter."))
            .map(|index| index + "greeter.".len())
            .expect("fixture 里有 greeter.") as u32;
        let items = service
            .completion(&app_path, APP, dot_line, dot_column)
            .unwrap_or_else(|error| panic!("补全请求失败：{error}"));
        assert!(
            !items.is_empty(),
            "`greeter.` 之后 JDT 应当给出成员候选（哪怕是空的也要先确认链路）"
        );
        assert!(
            items.iter().any(|item| item.label.contains("greet")),
            "候选里应当有 greet：{:?}",
            items.iter().map(|item| &item.label).collect::<Vec<_>>()
        );
        for item in &items {
            assert!(
                item.text_edit.is_some(),
                "候选 {:?} 没有 textEdit：接受它就会插重复（`Sys`+`System` → `SysSystem`）",
                item.label
            );
            assert!(
                !item.insert_text.contains('$'),
                "候选 {:?} 的 insert_text 仍是 snippet：{:?}",
                item.label,
                item.insert_text
            );
        }
    }

    #[test]
    fn real_jdtls_publishes_diagnostics_after_a_sync() {
        if std::env::var_os(SMOKE_ENV).is_none_or(|value| value.is_empty()) {
            return;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时钟应在 Unix 纪元之后")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "lithe-gpui-java-diag-{}-{stamp}",
            std::process::id()
        ));
        let _cleanup = TempWorkspace(root.clone());
        let sources = root.join("src").join("main").join("java").join("demo");
        std::fs::create_dir_all(&sources).expect("fixture 源码目录");
        std::fs::write(
            root.join("pom.xml"),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                "<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n",
                "  <modelVersion>4.0.0</modelVersion>\n",
                "  <groupId>demo</groupId>\n",
                "  <artifactId>lithe-diag-smoke</artifactId>\n",
                "  <version>1.0.0</version>\n",
                "  <properties><maven.compiler.release>21</maven.compiler.release></properties>\n",
                "</project>\n",
            ),
        )
        .expect("pom.xml");

        const BROKEN: &str = concat!(
            "package demo;\n",
            "\n",
            "public class Broken {\n",
            "    void run() {\n",
            "        int x = \"not an int\";\n",
            "    }\n",
            "}\n",
        );
        let path = sources.join("Broken.java");
        std::fs::write(&path, BROKEN).expect("Broken.java");

        let service = JavaLanguageService::new(root.clone());
        struct Shutdown<'a>(&'a JavaLanguageService);
        impl Drop for Shutdown<'_> {
            fn drop(&mut self) {
                self.0.shutdown();
            }
        }
        let _shutdown = Shutdown(&service);

        assert!(service.prepare().expect("真实 JDTLS 应当启动"));
        let version = service
            .sync_document(&path, BROKEN)
            .unwrap_or_else(|error| panic!("同步正文失败：{error}"));

        let error_line = BROKEN
            .lines()
            .position(|line| line.contains("not an int"))
            .expect("fixture 里有错误行") as u32;

        let deadline = Instant::now() + Duration::from_secs(30);
        let mut diagnostics = Vec::new();
        while Instant::now() < deadline {
            diagnostics = service.diagnostics(&path);
            if !diagnostics.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }

        assert!(
            !diagnostics.is_empty(),
            "同步（文档版本 {version}）之后 30s 内没有收到诊断：\
             要么 `lsp.syncDocument` 没让服务端重算，要么泵没把 `diagnostics` 事件归档"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Some(1)),
            "`String` 赋给 `int` 至少该有一条 Error（severity=1）：{diagnostics:?}"
        );
        let on_error_line = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.range.start.line == error_line)
            .unwrap_or_else(|| panic!("没有一条诊断落在错误行 {error_line}：{diagnostics:?}"));
        assert!(
            on_error_line.range.start.utf16_column < on_error_line.range.end.utf16_column,
            "诊断范围不能是空区间（空区间的波浪线画不出来）：{on_error_line:?}"
        );
    }

    #[test]
    fn real_jdtls_returns_an_import_quick_fix_for_an_unresolved_type() {
        if std::env::var_os(SMOKE_ENV).is_none_or(|value| value.is_empty()) {
            return;
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("系统时钟应在 Unix 纪元之后")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "lithe-gpui-java-codeaction-{}-{stamp}",
            std::process::id()
        ));
        let _cleanup = TempWorkspace(root.clone());
        let sources = root.join("src").join("main").join("java").join("demo");
        std::fs::create_dir_all(&sources).expect("fixture 源码目录");
        std::fs::write(
            root.join("pom.xml"),
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                "<project xmlns=\"http://maven.apache.org/POM/4.0.0\">\n",
                "  <modelVersion>4.0.0</modelVersion>\n",
                "  <groupId>demo</groupId>\n",
                "  <artifactId>lithe-codeaction-smoke</artifactId>\n",
                "  <version>1.0.0</version>\n",
                "  <properties><maven.compiler.release>21</maven.compiler.release></properties>\n",
                "</project>\n",
            ),
        )
        .expect("pom.xml");

        const BROKEN: &str = concat!(
            "package demo;\n",
            "\n",
            "public class ImportFix {\n",
            "    List<String> names = new java.util.ArrayList<>();\n",
            "}\n",
        );
        let path = sources.join("ImportFix.java");
        std::fs::write(&path, BROKEN).expect("ImportFix.java");

        let service = JavaLanguageService::new(root.clone());
        struct Shutdown<'a>(&'a JavaLanguageService);
        impl Drop for Shutdown<'_> {
            fn drop(&mut self) {
                self.0.shutdown();
            }
        }
        let _shutdown = Shutdown(&service);

        assert!(service.prepare().expect("真实 JDTLS 应当启动"));
        let version = service
            .sync_document(&path, BROKEN)
            .unwrap_or_else(|error| panic!("同步正文失败：{error}"));

        let error_line = BROKEN
            .lines()
            .position(|line| line.contains("List<String>"))
            .expect("fixture 里有错误行") as u32;
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut diagnostics = Vec::new();
        while Instant::now() < deadline {
            diagnostics = service.diagnostics(&path);
            if !diagnostics.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        assert!(
            !diagnostics.is_empty(),
            "同步（文档版本 {version}）之后 30s 内没有诊断，快速修复就无从谈起"
        );

        let column = BROKEN
            .lines()
            .nth(error_line as usize)
            .and_then(|line| line.find("List"))
            .expect("fixture 里有 List") as u32;
        let caret = JavaPosition {
            line: error_line,
            utf16_column: column,
        };
        let actions = service
            .code_actions(&path, BROKEN, caret, caret)
            .unwrap_or_else(|error| panic!("codeAction 请求失败：{error}"));

        assert!(
            !actions.is_empty(),
            "`List` 没有 import，JDT 至少该给出 `Import 'List' (…)`：诊断 {diagnostics:?}"
        );
        let import_util = actions
            .iter()
            .find(|action| action.title == "Import 'List' (java.util)")
            .unwrap_or_else(|| {
                panic!(
                    "候选中应当有 `Import 'List' (java.util)`：{:?}",
                    actions.iter().map(|a| &a.title).collect::<Vec<_>>()
                )
            });
        for action in &actions {
            assert!(
                !action.edits.is_empty(),
                "返回给编辑器的 action 必须带可落地的编辑：{action:?}"
            );
        }
        assert_eq!(import_util.kind.as_deref(), Some("quickfix"));
        assert_eq!(import_util.other_file_edits, 0);
        assert_eq!(import_util.edits.len(), 1);
        let edit = &import_util.edits[0];
        assert_eq!(edit.start.line, 0);
        assert_eq!(edit.start.utf16_column, 0);
        assert!(
            edit.new_text.contains("import java.util.List;"),
            "这条编辑必须真的插入 import：{edit:?}"
        );
    }

    /// 内嵌提示：位置与文本逐字保留。
    #[test]
    fn inlay_hints_keep_their_position_and_label() {
        let hint = parse_inlay_hint(&json!({
            "position": { "line": 3, "utf16Column": 12 },
            "label": "count:",
        }))
        .expect("合法提示");
        assert_eq!(hint.line, 3);
        assert_eq!(hint.utf16_column, 12);
        assert_eq!(hint.label, "count:");
    }

    /// 缺位置或文本为空都不是可画的提示：前者没有落点，后者画出来是一块看不见的占位。
    #[test]
    fn an_inlay_hint_without_a_position_or_text_is_dropped() {
        assert!(parse_inlay_hint(&json!({ "label": "x" })).is_none());
        assert!(
            parse_inlay_hint(&json!({
                "position": { "line": 0, "utf16Column": 0 },
                "label": "",
            }))
            .is_none()
        );
    }

    /// Code Lens：行号与命令标题都要留下。
    #[test]
    fn code_lenses_keep_the_line_and_the_command_title() {
        let lens = parse_code_lens(&json!({
            "range": { "start": { "line": 7, "utf16Column": 0 }, "end": { "line": 7, "utf16Column": 4 } },
            "command": { "title": "3 references", "command": "java.show.references" },
        }))
        .expect("有标题");
        assert_eq!(lens.line, 7);
        assert_eq!(lens.title, "3 references");
    }

    /// 等 `codeLens/resolve` 回填的占位条目没有标题，画出来是一行空白，所以要跳过。
    #[test]
    fn code_lenses_without_a_title_are_skipped() {
        assert!(
            parse_code_lens(&json!({
                "range": { "start": { "line": 1, "utf16Column": 0 } },
            }))
            .is_none()
        );
        assert!(
            parse_code_lens(&json!({
                "range": { "start": { "line": 1, "utf16Column": 0 } },
                "command": { "title": "" },
            }))
            .is_none()
        );
    }
}
