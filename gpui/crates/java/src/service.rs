//! [`JavaLanguageService`]：把载荷发现、索引缓存与 Core LSP 会话组成**一个可被编辑器调用的门面**。
//!
//! ## 生命周期（谁在什么时候调）
//!
//! | 时机 | 调用 | 作用 |
//! | --- | --- | --- |
//! | 打开项目（外壳建好编辑区后） | [`JavaLanguageService::prepare`] | 用 `java.workspacePolicy` 判断是不是 Java 工作区；是就起 JDTLS 并**等它就绪**（= 生成 / 复用索引缓存） |
//! | `F12` / `Ctrl+单击` | [`JavaLanguageService::definition`] | 同步文档 → `textDocument/definition` → 归一化目标 |
//! | 应用退出 | [`JavaLanguageService::shutdown`] | `lsp.stopServer` + `lsp.destroyServer` |
//!
//! ## 失败为什么**退回第一批而不是报错**
//!
//! 第一批的轻量导航（`lsp.builtinNavigation`）不需要任何进程。JDTLS 起不来（没装 JDK 21、
//! 载荷不全、机器内存不够）时，**跳转不该整体失效**：`definition` 返回 `Err`，
//! 编辑器记一条 `S1_NAV_FAILED`，然后走第一批那条路。所以这里把"起不来"当成
//! **能力的降级**而不是错误状态 —— 但降级原因必须留在 `S1_JAVA_*` 里。
//!
//! ## 为什么失败后不自动重试
//!
//! 启动失败几乎都是环境问题（JDK 版本、载荷缺失）。每次 `F12` 都重试一次 30s+ 的启动
//! 会把每次跳转都变成一次长时间阻塞。所以失败只记一次，重启应用等于重试。

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lithe_gpui_shared::core_json;
use serde_json::{Value, json};

use crate::jdtls::{self, JdtlsInstallation, JavaRuntime};
use crate::session::{Session, SessionSpec, SessionTimeouts};
use crate::workspace;

/// 会话就绪的绝对上限：JDTLS 首次索引大项目可以很久，给足 10 分钟。
const READY_ABSOLUTE_TIMEOUT: Duration = Duration::from_secs(600);
/// 单次定义的死线：文档同步 + 一次语义请求。
const DEFINITION_TIMEOUT: Duration = Duration::from_secs(60);
/// 单次补全的死线。补全是**交互式**请求：用户正在等着敲下一个字符，不能像跳转那样给 60s。
/// JDTLS 首次索引进度未知时补全会慢，所以给 15s；超时由编辑器回落到 Core 的轻量补全
/// （`lsp.builtinCompletions`），用户至少能看到当前文件的标识符，而不是"什么都没有"。
const COMPLETION_TIMEOUT: Duration = Duration::from_secs(15);

/// 一条补全候选：字段与 Core `lsp.request{operation:"completion"}` **归一化后**的条目一一对应
/// （定义在 `rust/lithe-core/src/lsp/interface/client.rs:937-965`，信封是 `{ "items": [ … ] }`）。
///
/// 为什么在这里就转成结构化类型而不是把 JSON 透传给编辑器：Core 的线格式是本 crate 的契约
/// （crate 文档第一条），编辑器只该看到"标签 / 要插入什么 / 在哪替换"这三件事。
#[derive(Clone, Debug, PartialEq)]
pub struct JavaCompletionItem {
    /// 列表里显示的名字。
    pub label: String,
    /// 接受后插入的文本。**snippet 已在 [`JavaLanguageService::completion`] 里转成纯文本**。
    pub insert_text: String,
    /// LSP `CompletionItemKind` 的原始数值（1..25），未知为 `None`。
    pub kind: Option<i64>,
    /// 一行的补充说明（通常是签名）。
    pub detail: Option<String>,
    /// 文档（Core 已把 `string | MarkupContent` 归一成纯文本）。
    pub documentation: Option<String>,
    /// 服务端给的排序键（**排序要听服务端的**：它比标签更懂上下文）。
    pub sort_text: Option<String>,
    /// 过滤键（有些候选的显示名与匹配名不同，例如别名）。
    pub filter_text: Option<String>,
    /// 替换范围 + 新文本；`None` = Core 没给 `textEdit`（此时只能在光标处插入）。
    ///
    /// ⚠️ **这一项直接决定补全会不会"插重复"**：上游接受补全时优先用 `textEdit` 的 range
    /// 替换掉已经敲进去的前缀，没有 `textEdit` 就退化成"在光标处再插一段"
    /// （`gpui-base-0.6.6/src/input/editor/lsp/overlay.rs:166-197`），
    /// 于是 `Sys` + `System` 会变成 `SysSystem`。所以只要 Core 给了就必须带上。
    pub text_edit: Option<JavaTextEdit>,
}

/// 一条 `textEdit`：0 基行 + 0 基 **UTF-16 码元**列（LSP 口径，与编辑器字符列不同）。
#[derive(Clone, Debug, PartialEq)]
pub struct JavaTextEdit {
    /// 替换起点。
    pub start: JavaPosition,
    /// 替换终点。
    pub end: JavaPosition,
    /// 替换成什么（同样已经过 snippet 转换）。
    pub new_text: String,
}

/// 一个 LSP 位置（0 基行 + 0 基 UTF-16 码元列）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JavaPosition {
    /// 0 基行。
    pub line: u32,
    /// 0 基 UTF-16 码元列。
    pub utf16_column: u32,
}

/// 语义跳转的目标。
#[derive(Clone, Debug)]
pub enum JavaTarget {
    /// 磁盘上的普通源码文件（含跨文件 / 跨模块）。
    File {
        /// 目标文件的绝对路径。
        path: PathBuf,
        /// 0 基行。
        line: u32,
        /// 0 基 **UTF-16 码元**列（与编辑器的字符列不同口径，换算在编辑器侧）。
        utf16_column: u32,
    },
    /// JDT 的**虚拟源码**（`jdt://`，JDK / 依赖库的反编译结果）。
    ///
    /// `text` 是 Core 通过 `virtualDocument` 语义请求取回的只读源码，
    /// `uri` 是它的身份（同一个类永远同一个 URI，所以重复跳转只切标签）。
    Virtual {
        /// 不透明的 JDT 虚拟 URI（原样保留，**不解析**）。
        uri: String,
        /// 标签显示名，Core 归一化出来的源码形态路径，例如 `java.base/java/lang/String.java`。
        display_path: String,
        /// 只读源码正文。
        text: String,
        /// 0 基行。
        line: u32,
        /// 0 基 UTF-16 码元列。
        utf16_column: u32,
    },
}

/// 会话状态。
enum State {
    /// 还没尝试启动（项目刚打开，或工作区被策略判定为"不是 Java 项目"）。
    Idle,
    /// 会话就绪。
    Ready(Box<Session>),
    /// 启动失败过一次：记录原因，不再自动重试。
    Failed(String),
    /// 会话已关闭（应用退出）。
    Closed,
}

/// Java 语言服务门面。**所有方法都是阻塞的**，调用方负责放进后台任务。
pub struct JavaLanguageService {
    /// 工作区根：`rootUri` / `workingDirectory` / 缓存键都从它来。
    workspace_root: PathBuf,
    /// 会话与状态。`Mutex` 保证"启动"与"请求"不会同时操作同一个 Core 会话。
    state: Mutex<State>,
    /// 解析好的载荷与 JDK（启动成功后一直复用；诊断行也要用）。
    installation: Mutex<Option<(JdtlsInstallation, JavaRuntime)>>,
}

impl JavaLanguageService {
    /// 建一个服务门面，**不起进程**。
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            workspace_root,
            state: Mutex::new(State::Idle),
            installation: Mutex::new(None),
        }
    }

    /// 打开项目时调一次：判断工作区是不是 Java 项目，是就起 JDTLS 并等它就绪。
    ///
    /// "等它就绪"就是"生成索引"：JDT 的项目导入与索引在 `ServiceReady` 之后仍在继续，
    /// 但状态目录（`cacheDirectory/jdtls/<workspaceKey>`）在会话活着的时候就已经在写，
    /// 下一次用同一个键启动时会直接复用。
    ///
    /// 返回 `Ok(false)` 表示"策略判定不需要 Java 工具"（不是失败）。
    pub fn prepare(&self) -> Result<bool, String> {
        if !self.is_java_workspace()? {
            println!("S1_JAVA_SKIPPED reason=not-a-java-workspace");
            return Ok(false);
        }
        self.ensure_session()?;
        Ok(true)
    }

    /// 取光标处的**语义定义**。
    ///
    /// `text` 是编辑器当前的正文（可能未保存），`utf16_column` 是 Core 契约的列口径。
    /// 返回 `Ok(None)` = JDTLS 答了但没给位置（例如光标在空白处）；
    /// 返回 `Err` = 服务不可用，调用方应当退回第一批的轻量导航。
    pub fn definition(
        &self,
        file_path: &Path,
        text: &str,
        line: u32,
        utf16_column: u32,
    ) -> Result<Option<JavaTarget>, String> {
        // 契约 `:1287-1288`：策略说不启动时，"hosts still start a language server on demand
        // when the user opens a .java file"。所以这里**不**再问策略，直接按需启动。
        self.ensure_session()?;

        let uri = file_uri(file_path)?;
        // 锁必须在整个请求期间持有：Core 的会话是有状态的（文档版本、在飞请求），
        // 两个线程同时 syncDocument / request 会让版本号与结果对不上。
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
            "definition",
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
        let Some(first) = locations.first() else {
            println!("S1_JAVA_DEFINITION uri={uri} targets=0");
            return Ok(None);
        };

        if let Some(target) = self.physical_target(first)? {
            return Ok(Some(target));
        }
        self.virtual_target(session, first).map(Some)
    }

    /// 取光标处的补全候选（`textDocument/completion`，走 JDTLS 的**项目感知**结果）。
    ///
    /// 形状与 [`Self::definition`] 完全一致：`ensure_session` → 同步文档 → 发语义请求 → 解析结果。
    /// 只有两处不同，都是为了让用户**能直接用**：
    ///
    /// 1. **死线更短**（[`COMPLETION_TIMEOUT`]）：补全是交互式请求，用户正等着敲下一个字符。
    ///    超时由编辑器回落到 Core 的轻量补全（`lsp.builtinCompletions`），
    ///    至少还能看到当前文件的标识符 —— "什么都没有"才是最伤的；
    /// 2. **snippet 已经转成纯文本**：`insertTextFormat == 2` 的条目经 Core 的
    ///    `lsp.plainSnippet` 过一遍（`rust/lithe-core/src/lsp/lightweight/snippets.rs:14-53`）。
    ///    上游接受补全时是**把 `textEdit.newText` / `insertText` 原样写进正文**
    ///    （`gpui-base-0.6.6/src/input/editor/lsp/overlay.rs:166-197`），它不认识 snippet 语法；
    ///    不转的话用户会看到 `System.out.println(${1:...})` 这种东西。Core 那条命令是
    ///    **进程内同步调用**（`gpui/crates/shared/src/core_client.rs:189` → `lithe_core::execute_json`），
    ///    逐条转换的代价可以忽略；也避免我们在 gpui 侧再写一个转换器（那就是两个真相源）。
    pub fn completion(
        &self,
        file_path: &Path,
        text: &str,
        line: u32,
        utf16_column: u32,
    ) -> Result<Vec<JavaCompletionItem>, String> {
        // 与 `definition` 同口径：契约说"用户打开 .java 文件时可以按需起服务"，所以这里不再问策略。
        self.ensure_session()?;

        let uri = file_uri(file_path)?;
        // 锁必须在整个请求期间持有（理由同 `definition`）：Core 的会话有状态，
        // 两个线程同时 syncDocument / request 会让文档版本与结果对不上。
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

        // ⚠️ **"文档变旧了"不是失败**：Core 会在下一次 `syncDocument` 时自动取消版本变旧的补全
        // 请求（`rust/lithe-core/src/lsp/interface/engine.rs:4348-4360` 的
        // `is_stale_sensitive_method` 首位就是 `textDocument/completion`，取消走
        // `:4289-4346`），而快速打字时这**是常态**。所以这里把它翻成 `Ok(空)`：
        // 调用方（编辑器）就不会误判成"服务不可用"而回落到轻量补全 —— 否则菜单会在
        // jdtls 与 builtin 两种候选之间来回跳，看起来像"补全在乱跳"。
        // 错误字符串的形状见 `session.rs:351`（`"{code}@{stage}：{message}"`），只认前缀。
        let result = match result {
            Ok(result) => result,
            Err(error) if is_superseded(&error) => {
                println!("S1_JAVA_COMPLETION superseded reason={error}");
                return Ok(Vec::new());
            }
            Err(error) => return Err(error),
        };

        // Core 的信封是 `{ "items": [ … ] }`（`rust/lithe-core/src/lsp/interface/client.rs:871-876`）。
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

    /// 关闭会话（应用退出时调；`Idle` / `Failed` 时是空操作）。
    pub fn shutdown(&self) {
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let State::Ready(session) = std::mem::replace(&mut *state, State::Closed) {
            session.shutdown();
        } else {
            *state = State::Closed;
        }
    }

    // -----------------------------------------------------------------------
    // 内部
    // -----------------------------------------------------------------------

    /// 幂等启动：已经就绪直接返回；失败过就不再重试（见模块文档）。
    fn ensure_session(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
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
                Ok(())
            }
            Err(reason) => {
                println!("S1_JAVA_FAILED reason={reason}");
                *state = State::Failed(reason.clone());
                Err(reason)
            }
        }
    }

    /// 真正启动（调用方已持有 `state` 锁，`State::Idle` 已确认）。
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

        // 回收过期索引：失败只降级成一条警告 —— 它只是省磁盘，不该挡住跳转。
        match workspace::cleanup_expired(&cache.cache_directory, &cache.workspace_key) {
            Ok(outcome) => println!(
                "S1_JAVA_CACHE_RECLAIM retentionDays={} removed={}",
                outcome.retention_days, outcome.removed
            ),
            Err(error) => println!("S1_JAVA_CACHE_RECLAIM_FAILED error={error}"),
        }

        // Maven 项目上下文：有 `pom.xml` 才带（见 `workspace::maven_context`）。
        // 诊断行把结论说清楚 —— "生成源根有没有生效"这条链路的入口就在这里。
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
            // 只有一个候选：跑 JDTLS 的这个 JDK。项目要别的版本时由 Maven/项目设置决定，
            // 那属于 run/debug 的范围（本轮不做）。
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

    /// `java.workspacePolicy` + `workspace.snapshot`：这个工作区该不该起 Java 工具。
    ///
    /// ⚠️ 这里调了一次 `workspace.snapshot`（与项目树同一个 Core 命令）而不是复用项目树
    /// 已经拉到的那份：`Explorer` 的快照是 crate 私有状态，把它开成公开接口会让
    /// 项目树的数据层变成跨 feature 的契约。代价是打开项目时多扫一次工作区
    /// （Core 侧有界，实测本项目根 <1s），换来的是"Java 策略的输入来自 Core 而不是项目树"。
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

    /// 常规文件目标。
    ///
    /// ⚠️ **路径从 `uri` 推，不用 Core 给的 `filePath`**：Core 的 `filePath` 在 Windows 上
    /// 会多一个前导 `/`（`rust/lithe-core/src/lsp/interface/client.rs:1262-1273` 的
    /// `file_path_for_uri` 对 `file:///C:/...` 只剥两个斜杠，得到 `/C:/...`），
    /// 直接拿来当路径会指向一个不存在的文件。Windows host 同样**不信任** `filePath`：
    /// 它在每个使用点都重新 `filePathFromUri(location.uri)`
    /// （`windows/tauri/src/features/keymaps/commands/navigation-command-actions.ts:325,358,680`）。
    /// 这里用 `url` 的 `to_file_path()`，与那边的 `filePathFromUri` 同一件事。
    fn physical_target(&self, location: &Value) -> Result<Option<JavaTarget>, String> {
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
            "S1_JAVA_DEFINITION target=file path={} line={} col={}",
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

    /// `jdt://` 虚拟源码目标：先用 `virtualDocument` 把源码取回来，再交给编辑器。
    fn virtual_target(
        &self,
        session: &Session,
        location: &Value,
    ) -> Result<JavaTarget, String> {
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

/// 定义结果里的目标位置（Core 归一化成 `range.start.{line,utf16Column}`）。
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

/// 安装根：`.../bin/jdtls.bat` → `...`；已经在根上就是父目录。
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

/// 文件路径 → `file://` URI（百分号编码由 `url` 负责）。
fn file_uri(path: &Path) -> Result<String, String> {
    url::Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|_| format!("无法把路径转成 file URI：{}", path.display()))
}

/// 目录路径 → `file://` URI（LSP 的 `rootUri` 以 `/` 结尾）。
fn directory_uri(path: &Path) -> Result<String, String> {
    url::Url::from_directory_path(path)
        .map(|url| url.to_string())
        .map_err(|_| format!("无法把工作区路径转成 file URI：{}", path.display()))
}

// ---------------------------------------------------------------------------
// 补全条目的解析（Core 的线格式 → 本 crate 的结构化类型）
// ---------------------------------------------------------------------------

/// 这个错误是不是"请求被更新的文档版本取代了"（而不是真的失败）。
///
/// Core 的运行时错误码见 `rust/lithe-core/src/lsp/interface/engine.rs`：
/// `staleDocumentVersion`（`syncDocument` 之后旧请求被取消）与 `requestCancelled`
/// （宿主/服务端主动取消）。两者在**快速打字时是常态**，必须与"服务起不来 / 超时"区分开：
/// 前者静默丢弃（`Ok(空)`），后者才允许调用方降级到轻量补全。
///
/// 判据是错误字符串的前缀（`Session::request` 的形状是 `"{code}@{stage}：{message}"`，
/// `session.rs:351`）——本 crate 里只有这一处需要识别 Core 的运行时码，不额外引入枚举。
fn is_superseded(error: &str) -> bool {
    error.starts_with("staleDocumentVersion@") || error.starts_with("requestCancelled@")
}

/// 解析 Core 归一化后的一条补全条目。
///
/// 字段与 `rust/lithe-core/src/lsp/interface/client.rs:937-965` 一一对应；
/// **`label` 是唯一的必需字段**（Core 也是这么定的：没有 label 的条目直接丢掉），
/// 其余缺了就 `None`，绝不 panic —— 语言服务器的返回本来就随版本变化。
fn parse_completion_item(value: &Value) -> Option<JavaCompletionItem> {
    let label = value.get("label").and_then(Value::as_str)?.to_string();
    // Core 已经把 insertText 归一成"非空"（优先 insertText、否则 textEdit.newText、否则 label）。
    let insert_text_raw = value
        .get("insertText")
        .and_then(Value::as_str)
        .unwrap_or(label.as_str());
    let is_snippet = value.get("insertTextFormat").and_then(Value::as_u64) == Some(2);

    let mut text_edit = value.get("textEdit").and_then(parse_text_edit);
    let insert_text = if is_snippet {
        let plain = plain_snippet(insert_text_raw);
        // `textEdit` 才是上游真正用来替换前缀的那份文本，所以它也必须转。
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

/// 解析一条 `textEdit`（Core 的 `{ range: { start: {line, utf16Column}, end: {...} }, newText }`）。
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

/// 解析一个 Core 位置（`{ line, utf16Column }`）。
fn parse_position(value: &Value) -> Option<JavaPosition> {
    Some(JavaPosition {
        line: value.get("line").and_then(Value::as_u64)? as u32,
        utf16_column: value.get("utf16Column").and_then(Value::as_u64)? as u32,
    })
}

/// 把 LSP snippet 还原成可插入的纯文本（`System.out.println(${1:msg})` → `System.out.println(msg)`）。
///
/// **走 Core 的 `lsp.plainSnippet`，不在这里自己写规则**：那是 Core 拥有的转换
/// （`rust/lithe-core/src/lsp/lightweight/snippets.rs:20-53`，含 `${n:默认值}` /
/// `$n` / 转义 `$` 的全部规则），在 gpui 侧再实现一份就是两个真相源，
/// 而且一旦不一致，用户看到的候选与实际插入的文本会对不上。
///
/// 转换失败（Core 报错 / 返回非字符串）时**原样返回**：宁可在极少数情况下显示占位符，
/// 也不能因为一次转换失败让整条候选消失或插进空串 —— 并留一条诊断。
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

    /// `rootUri` 必须是可被 `url` 解析的 `file://` URI，且以 `/` 结尾；空格要编码。
    #[test]
    fn directory_uri_encodes_spaces() {
        // 用平台临时目录做基底：`url::Url::from_directory_path` 要求**平台原生**绝对路径，
        // 写一个 `/tmp/...` 在 Windows 上会被它直接拒绝（这条断言就是这么发现的）。
        let path = std::env::temp_dir().join("lithe java ws");
        let uri = directory_uri(&path).expect("uri");
        assert!(uri.starts_with("file:///"), "{uri}");
        assert!(uri.ends_with('/'), "{uri}");
        assert!(uri.contains("lithe%20java%20ws"), "{uri}");
    }

    /// **补全条目**的解析：字段与 Core 的归一化输出一一对应，尤其是 `textEdit` 的
    /// `utf16Column` 范围（决定"会不会插重复"）。
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
        assert_eq!(item.detail.as_deref(), Some("String Greeter.greet(String name)"));
        assert_eq!(item.sort_text.as_deref(), Some("0001"));
        // snippet 必须被 Core 的 `lsp.plainSnippet` 还原成纯文本（占位符一个都不许留），
        // 否则用户接受补全会把 `${1:name}` 原样插进源码。
        assert_eq!(item.insert_text, "greet(name)");
        let edit = item.text_edit.expect("必须带上 textEdit，否则会插重复");
        assert_eq!(edit.new_text, "greet(name)");
        assert_eq!(edit.start.line, 3);
        assert_eq!(edit.start.utf16_column, 28);
        assert_eq!(edit.end.utf16_column, 31);
    }

    /// 没有 `label` 的条目直接丢掉（Core 也是这个判据）；非 snippet 的文本**原样保留**。
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

    /// Core 归一化后的定义位置是 `range.start.{line,utf16Column}`；
    /// 缺字段要报错而不是默默跳到 1:1。
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

    // -----------------------------------------------------------------------
    // 选定式（opt-in）真实 JDTLS 端到端检查
    // -----------------------------------------------------------------------

    /// 开关环境变量。设了它才会跑真实 JDTLS —— 与 Core 的
    /// `real_jdtls_*`（`rust/lithe-core/src/lsp/interface/engine_real_jdt_tests.rs:1-10`）
    /// 同一约定：需要一个本地 JDTLS 载荷 + JDK 21+，CI 上不跑。
    ///
    /// 还需要 `LITHE_JDTLS_JAVA`（`crate::jdtls::JAVA_EXECUTABLE_ENV`）指出 JDK 21+ 的
    /// `java`：本机 PATH 上那个是 1.8，JDTLS 1.61 起不来。载荷本身由
    /// `crate::jdtls::resolve` 从当前 exe 向上找到 `.artifacts/jdtls`。
    const SMOKE_ENV: &str = "LITHE_GPUI_JDTLS_SMOKE";

    /// 极简 Java 工程的源文件：**跨文件**（`App` 调 `Greeter`）才是第二批的验收点，
    /// 同文件内的跳转第一批就能做。
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

    /// 级联临时目录：即使断言失败也会在 `Drop` 里删掉，不留 JDT 索引残骸。
    struct TempWorkspace(PathBuf);

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// 真实 JDTLS：`prepare` 起服务并等就绪，然后在 `App.java` 里对 `Greeter` 求定义，
    /// 必须落到 `Greeter.java` 的**声明行**。
    ///
    /// 这条测试同时守住三件事：载荷/JDK 解析、`lsp.startServer` 的字段能被 Core 接受、
    /// 以及跨文件语义结果能被归一化成 `JavaTarget::File`。
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
        // **标准 Maven 布局**（`src/main/java/...`）：本用例同时验证"`mavenContext` 被 Core 接受
        // 且真的改变了 JDT 的项目模型"—— 实测把源码放在非标准的 `src/demo/` 时，
        // 一旦带上 mavenContext（= Maven 源模型生效），`Greeter` 就解析不到了，
        // 这条断言因此也是"Maven 上下文确实生效"的证据。
        let sources = root.join("src").join("main").join("java").join("demo");
        std::fs::create_dir_all(&sources).expect("fixture 源码目录");
        std::fs::write(sources.join("Greeter.java"), GREETER).expect("Greeter.java");
        std::fs::write(sources.join("App.java"), APP).expect("App.java");
        // **Maven 工程**：本用例同时验证"`mavenContext` 被 Core 接受"这条链路 ——
        // Core 会校验 reactor（`project/maven.rs` 的 `MavenLaunchContextRequest`），
        // 形状错了 `lsp.startServer` 直接失败，下面的 `prepare()` 就会 panic。
        // 顺带证明 m2e 导入路径：JDT 拿到 sourcePaths 后补全/跳转照常工作。
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
        // 无论断言是否通过都要关掉 Core 会话（否则 java 子进程会活过测试进程）。
        struct Shutdown<'a>(&'a JavaLanguageService);
        impl Drop for Shutdown<'_> {
            fn drop(&mut self) {
                self.0.shutdown();
            }
        }
        let _shutdown = Shutdown(&service);

        let prepared = service.prepare().expect("真实 JDTLS 应当启动");
        assert!(prepared, "`src/demo/*.java` 的工程应当被判为 Java 工作区");

        // `greeter.greet("world")` 里的 `greet`：第 6 行（0 基 5）第 31 列（0 基 30）。
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
        assert_eq!(target_line, 3, "`greet` 声明在 Greeter.java 第 4 行（0 基 3）");

        // `jdt://`：`System` 是 JDK 里的类，JDT 只能给虚拟源码（反编译结果）。
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

        // 补全：`greeter.` 之后必须有候选，且**带替换范围**。
        //
        // 三条断言各有分工（照 `gpui/research/editor-lsp-completion.md` §7.1 的清单）：
        // 1. 非空 —— 证明 `operation:"completion"` 与响应解析一路通；
        // 2. `text_edit.is_some()` —— **唯一会把 `Sys` + `System` 变成 `SysSystem` 的路径**
        //    （上游没有 `textEdit` 就在光标处再插一段，`overlay.rs:166-197`），必须钉死；
        // 3. `insert_text` 里没有 `$` —— snippet 已经过 Core 的 `lsp.plainSnippet`，
        //    否则用户接受补全会把 `${1:name}` 原样插进源码。
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
}
