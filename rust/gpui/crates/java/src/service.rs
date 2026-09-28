//! [`JavaLanguageService`]：把载荷发现、索引缓存与 Core LSP 会话组成**一个可被编辑器调用的门面**。
//!
//! ## 生命周期（谁在什么时候调）
//!
//! | 时机 | 调用 | 作用 |
//! | --- | --- | --- |
//! | 打开项目（外壳建好编辑区后） | [`JavaLanguageService::prepare`] | 用 `java.workspacePolicy` 判断是不是 Java 工作区；是就起 JDTLS 并**等它就绪**（= 生成 / 复用索引缓存） |
//! | `F12` / `Ctrl+单击` | [`JavaLanguageService::definition`] | 同步文档 → `textDocument/definition` → 归一化目标 |
//! | 打开 / 改动 Java 文件 | [`JavaLanguageService::sync_document`] + [`JavaLanguageService::diagnostics`] | 同步正文（让服务端重算）→ 读最近一次诊断快照（编辑器画波浪线） |
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

use crate::events::JavaDiagnostic;
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

/// 单次快速修复（code action）的死线。与补全同一个量级：它也是**交互式**请求
/// （用户按下 `Ctrl+.` 正等着菜单弹出来），只是没有兜底 —— 超时就是"菜单不弹"。
/// 15s 是上限而不是预期值：JDT 在已经 reconcile 过的文档上算一批 quick fix 是毫秒级
/// （实测 `S1_JAVA_CODE_ACTION … ms=<20`）。
const CODE_ACTION_TIMEOUT: Duration = Duration::from_secs(15);

/// 正文刚变过时，等"诊断追上正文"的**退避**（毫秒，逐次等待）。
///
/// 为什么需要它：`context.diagnostics` 是 JDT 算 quick fix 的**唯一输入**
/// （`CodeActionHandler.getCodeActionCommands` → `getProblemLocationCores`，
/// 反编译证据见 `code_actions` 的文档），而服务端推给我们的快照描述的是**上一次**
/// reconcile 的正文。`sync_document` 说"正文真的变了"（`changed=true`）时，
/// 手里那份快照的行列已经对不上新正文 —— 拿它去请求，JDT 会按错误的位置构造
/// {@code ProblemLocation}，给出的可能是**另一个**修复。所以这里等一次新发布（有界）。
///
/// 为什么是 2 档 250ms：实测 didChange → reconcile → publish 在 100ms 量级
/// （`Reconciled 1. Took 1 ms` 紧跟着 `1 problems reported`）。上限 500ms 只发生在
/// "用户刚敲完就按 Ctrl+."这一档，稳态（没改正文）`changed=false` 时一次都不等。
const CODE_ACTION_DIAGNOSTIC_BACKOFF_MS: [u64; 2] = [250, 250];

/// 一条 JDT 的快速修复（Code Action）。
///
/// 字段与 Core 归一化的 codeAction 条目一一对应
/// （`rust/lithe-core/src/lsp/interface/client.rs:1158-1178` 的 `parse_code_action`）：
/// `{ title, kind, isPreferred, edit: { changes: { <路径>: [ {range, newText} ] } }, command, data }`。
/// 这里刻意**只留编辑器真正要用的三样**：
///
/// - `title`：菜单里显示的名字（上游 `CodeActionMenu` 只渲染 `action.title`）；
/// - `kind`：只进诊断行（`kind=quickfix` 一眼看出这是修复而不是重构）；
/// - `edits`：**落在当前文件上的**替换，UTF-16 口径，已按位置**倒序**排好。
///
/// ⚠️ **带 `command` 的 action 在这里就被丢掉**（不返回给编辑器）：
/// 本批不实现 `workspace/executeCommand`（理由见 [`JavaLanguageService::code_actions`] 的文档），
/// 返回一个选中后什么都不做的菜单项比不返回它更糟。
#[derive(Clone, Debug, PartialEq)]
pub struct JavaCodeAction {
    /// 菜单里显示的名字（JDT 原样给的，例如 `Import 'List' (java.util)`）。
    pub title: String,
    /// LSP `CodeActionKind`（JDT 给的原始字符串，例如 `quickfix` / `source.generate.accessors`）。
    pub kind: Option<String>,
    /// 落在**当前文件**上的替换，已按位置倒序（先应用后面的，前面的范围才不会失效）。
    pub edits: Vec<JavaTextEdit>,
    /// 落在**其它文件**上的编辑条数。非 0 = 这条修复要"创建 / 改另一个文件"
    /// （例如 `Create class 'List<T>'` 会往 `demo/List.java` 写内容）。
    /// 本批的编辑器只有一个 buffer、也没有"新建文件"这条路径，所以这类 action **不返回**；
    /// 这个数字只用于诊断行（证明它们是被**有意**挡掉的，而不是没解析出来）。
    pub other_file_edits: usize,
}

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

    /// 取某个位置上的**快速修复**（`textDocument/codeAction`，JDT 的项目感知结果）。
    ///
    /// ## 同步正文的顺序：**先 `sync_document`，再请求**（不能反）
    ///
    /// 两条理由，缺一条这个功能就是坏的：
    ///
    /// 1. **Core 会拒绝没打开的文档**：`engine.rs:1686-1691` 对不在 `open_documents` 里的
    ///    uri 直接回 `invalid_request`（`"The document is not open in the language server."`），
    ///    而"打开"正是 `sync_document` 做的（首次 `didOpen`，之后递增版本的 `didChange`）；
    /// 2. **服务端算出的行列是针对它手上那一版正文的**：编辑区里的正文可能还没保存、也可能
    ///    刚被改过，不先同步，JDT 给回的替换范围落到当前正文上就是错位的 —— 用户点
    ///    "Import 'List'"，结果覆盖掉的是别的地方。
    ///
    /// ## `context.diagnostics` 是**必需**的（这是本方法最容易被写漏的一条）
    ///
    /// JDT 不是"按光标位置猜修复"，而是拿请求里的诊断反查出 problem location 再算修复：
    /// `CodeActionHandler.getCodeActionCommands` 里
    /// `CodeActionContext.getDiagnostics()` → `lambda$0`（只留 `source == "Java"`）→
    /// `getProblemLocationCores(unit, diagnostics)` → `QuickFixProcessor.getCorrections(..)`。
    /// **实测**：同一位置、同一 range，`diagnostics: []` 只回 3 条 source action
    /// （`Generate Getters/Setters`），带上快照之后才回 `Import 'List' (java.util)` 等 quick fix
    /// （`.artifacts/p17/probe.log` / `probe2.log`）。`getProblemId` 还会把 `code` 当数字解析
    /// （`Integer.parseInt(getCode().getLeft())`），所以 `code` 必须原样带上。
    ///
    /// 客户端只该送"与这次请求相关"的诊断，所以这里送**光标行 / 选区范围内**的那些
    /// （[`diagnostics_for`]）；快照里其它行的 error 不会变成这个菜单里的候选。
    ///
    /// ## 为什么本批**不**执行 `command`
    ///
    /// 带 `command` 的 action（JDT 的 `source.organizeImports` / `overrideMethods` /
    /// `generate.constructors` 等，见 `SourceAssistProcessor` 的 `CodeAction.setCommand`）
    /// 要客户端发 `workspace/executeCommand` 再应用服务端回的工作区编辑。本批不做：
    /// 实测 JDT 1.61 在**默认 kind 集合**下（Core 的 codeAction 参数构造没有 `only` 字段）
    /// 返回的 action 全部带 `edit.changes`，`command` 恒为 `null`；实现一整条执行链
    /// 是投机性抽象。这类 action 在 [`parse_code_action`] 里被**丢掉**并在诊断行里计数。
    ///
    /// 返回值已过滤：只留"能在当前 buffer 里落地"的 action（见 [`JavaCodeAction`]）。
    pub fn code_actions(
        &self,
        file_path: &Path,
        text: &str,
        start: JavaPosition,
        end: JavaPosition,
    ) -> Result<Vec<JavaCodeAction>, String> {
        // 与 `definition` / `completion` 同口径：用户打开 .java 文件时可以按需起服务。
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

        // 同步**之前**那份快照：`changed=true` 时它描述的是旧正文，必须等新的发布（见常量文档）。
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
        // 请求字段名逐字照 Core 的 `SemanticRequest`（`engine.rs:326-349`，camelCase）与
        // `feature_request_params`（`client.rs:682-692`）：`uri` / `range{start,end}` / `diagnostics`。
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

        // 与补全同一口径："文档变旧"不是失败（虽然 `textDocument/codeAction` 不在 Core 的
        // `is_stale_sensitive_method` 名单里，这一档目前不会走到，但形状必须一致）。
        let result = match result {
            Ok(result) => result,
            Err(error) if is_superseded(&error) => {
                println!("S1_JAVA_CODE_ACTION superseded reason={error}");
                return Ok(Vec::new());
            }
            Err(error) => return Err(error),
        };

        // Core 的信封是 `{ "actions": [ … ] }`（`client.rs:1151-1178`）。
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

    /// `sync_document` 说正文变过时，**有界**等一次新的诊断发布，返回等了多少毫秒。
    ///
    /// 收手判据是"快照和同步前那份不一样"——新正文的 reconcile 结论到了。
    /// 为什么不判"非空"：改好一个错误之后新快照就该是**空**的，那也是新结论。
    /// 为什么可能等满：新正文的错误与旧正文逐条相同（例如只是把 `List` 改了几个字母），
    /// 两次发布逐条相等 ⇒ 分不出来，只能等满预算；此时手上那份快照**仍然是对的**
    /// （行列都没变），所以等满不是错误路径，只是白等 500ms。
    ///
    /// 同步前那份是空的（`changed=true` 但还没有过任何发布）时**不等**：没有基线可比，
    /// 而且第一次 Alt+Enter 之前编辑器本来就已经取过一轮诊断（波浪线要先出现）。
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

    /// 某个文件的**最近一次**诊断快照（`textDocument/publishDiagnostics` 推送）。
    ///
    /// 这是**查询**，不是请求：不起会话、不同步文档。诊断由服务端推送，只有同步过（或同步过
    /// 又改过）的文档才会有发布 —— Core 会丢掉"文档不在 `open_documents` 里"的那些
    /// （`rust/lithe-core/src/lsp/interface/client.rs:350-352`），所以没打开过的文件返回空是
    /// 正常结果，不是失败。为了不让一次查询把 JDTLS 拖起来（`ensure_session` 的启动链是
    /// 10s 级），这里**不**调 `ensure_session`。
    ///
    /// ⚠️ **空数组有两种含义**（查询口径下不可区分）：服务端刚发布了一次"清空"
    /// （错误都改好了），或者**还没发布过**。编辑器侧靠"同步正文之后有界重取"
    /// 把这两者分开（见 gpui 侧 `editor/src/diagnostics.rs`），本方法不做任何等待 ——
    /// 它必须保持"纯查询"（不起会话、不阻塞），否则每帧查一次就会把 UI 拖住。
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
            // `Idle` / `Failed` / `Closed` 都还没有（或不再有）事件来源 → 空。
            _ => Vec::new(),
        }
    }

    /// 把一个文档的**当前正文**同步给服务端（首次 `didOpen`，之后 `didChange`），**不发语义请求**。
    ///
    /// ## 为什么需要这条（而不是只调 [`Self::diagnostics`]）
    ///
    /// 诊断**不是请求的结果**：它是服务端在 `didOpen` / `didChange` / 构建完成之后
    /// **推送**的（`textDocument/publishDiagnostics` → Core 的 `diagnostics` 事件 → 泵按 uri 归档），
    /// 而 [`Self::diagnostics`] 只回"最近一次快照"。所以改完正文之后如果不同步过去，
    /// 服务端永远不会为新正文重算，用户看到的就是"波浪线停在旧位置"或"错误改好了波浪线还在"。
    /// `definition` / `completion` 里的 `sync_document` 是**顺带**发生的（用户得有那两种动作），
    /// 诊断刷新不能依赖它们。
    ///
    /// ## 口径
    ///
    /// - 与 `definition` / `completion` 同一条：**按需启动会话**（契约 `:1287-1288`：
    ///   "hosts still start a language server on demand when the user opens a .java file"）；
    /// - 锁在整个同步期间持有（理由同 `definition`：Core 的会话有状态，版本号不能乱）；
    /// - 返回服务端分配的文档版本号。Core 对"正文没变"的重复同步返回 `changed: false`
    ///   且**不重发** `didChange`（契约 `:1303-1310`）—— 那是正常结果，
    ///   诊断仍可能在路上，调用方按"有界重取"处理，不要把 `changed=false` 当失败。
    ///
    /// 诊断行复用既有形状（`S1_JAVA_SYNC`）：这条链路的证据是"正文真的递到了服务端"。
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
/// 文件路径 → `file://` URI（百分号编码由 `url` 负责）。
///
/// `pub`：编辑器的快速修复 provider 也用它 —— `edit.changes` 的键必须是这条 URI
/// （`crate::code_actions` 与 `perform_code_action` 两侧要对上）。**路径 → URI 只该有一处实现**，
/// 否则两侧各拼一次，某个字符的编码方式不同就会让编辑"找不到自己的文件"。
pub fn file_uri(path: &Path) -> Result<String, String> {
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

/// [`parse_code_action`] 的结论。
///
/// 三种情况要分开，是因为它们的**原因不同**、诊断行里也要分得开：
/// `Unusable` 是"能解析但本批落不了地"（要执行命令 / 要写别的文件），
/// `Malformed` 是"结构不认识"（连标题都没有）。混成一个 `None` 就没法判断
/// 某个菜单为空到底是"服务端没给"还是"我们挡掉了"。
#[derive(Debug)]
enum CodeActionOutcome {
    /// 能在当前 buffer 里落地。
    Usable(JavaCodeAction),
    /// 有标题但落不了地：`other_file_edits` 是落在别的文件上的编辑条数，
    /// `has_command` 说明它需要 `workspace/executeCommand`（本批不执行）。
    Unusable {
        other_file_edits: usize,
        has_command: bool,
    },
    /// 没有标题：结构不认识（Core 的 `parse_code_action` 也是这个判据）。
    Malformed,
}

/// 解析一条 Core 归一化后的 codeAction 条目（形状见 [`JavaCodeAction`]）。
///
/// `edit.changes` 的键是**路径**而不是 URI：Core 的 `parse_workspace_edit` 用
/// `file_path_from_uri`（`client.rs:1192-1231`）把 `file:///C:/x` 变成 `/C:/x`。
/// Windows 上这个前导斜杠必须去掉才能与编辑器手上的盘符路径比（[`is_current_file`]）。
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

    // 带 command 的 action 整条不返回：LSP 的语义是"先应用 edit、再执行 command"，
    // 而命令我们执行不了（理由见 `code_actions` 的文档）—— 只做一半会留下一个半成品修复。
    let has_command = value
        .get("command")
        .is_some_and(|command| !command.is_null());
    if edits.is_empty() || has_command {
        return CodeActionOutcome::Unusable {
            other_file_edits,
            has_command,
        };
    }

    // 倒序：一次 action 可能带多条编辑，先应用靠后的，靠前那条的范围才不会被前一次替换挪走
    // （Core 的 `lsp.applyTextEdits` 也是这个顺序，`lightweight/edits.rs:48-61`）。
    edits.sort_by_key(|edit| std::cmp::Reverse((edit.start.line, edit.start.utf16_column)));

    CodeActionOutcome::Usable(JavaCodeAction {
        title: title.to_string(),
        kind,
        edits,
        other_file_edits,
    })
}

/// `edit.changes` 的键是不是**这个 buffer 的文件**。
///
/// Core 给的键是 `file_path_from_uri` 的结果：`file:///C:/x.java` → `/C:/x.java`
/// （Windows 上带一个前导斜杠），`file:///Users/x.java` → `/Users/x.java`（macOS 上本就是绝对路径）。
/// 所以去掉前导斜杠**仅当**后面紧跟 `X:` 形状的盘符时才做；随后交给 `Path` 比 ——
/// Windows 上 `D:/a/b` 与 `D:\a\b` 的分量相同（`Path` 的分隔符解析对 `/` 与 `\` 一视同仁），
/// 所以这两侧的分隔符差异不需要自己归一化。
fn is_current_file(key: &str, file_path: &Path) -> bool {
    let trimmed = key
        .strip_prefix('/')
        .filter(|rest| is_windows_drive_prefix(rest))
        .unwrap_or(key);
    Path::new(trimmed) == file_path
}

/// `C:` / `c:` 形状的盘符前缀（Windows 路径被 URI 化之后会多一个前导斜杠）。
fn is_windows_drive_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// 挑出这次请求**相关**的诊断：与选区相交的那些，光标是空选区时取**光标所在行**上的那些。
///
/// 为什么空选区按"行"而不是按"点"：`Ctrl+.` 的直觉是"修这一行的问题"，而光标常常停在
/// 行首或标识符之外（缩进、行尾），用零宽区间去判相交会把这些情况全判成"没有相关诊断"，
/// 菜单里就只剩 source action —— 用户看到的是"快速修复没了"。
///
/// 只认 JDT 自己推来的诊断（`source` 字段由 JDT 给 `"Java"`）：`code_actions` 把它们原样
/// 送回 `context.diagnostics`，而 `getProblemId` 会把 `code` 当数字解析。
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
                // 两个区间相交：`!(a.end <= b.start || b.end <= a.start)`，
                // 行列按 LSP 的"行优先"序比较（先比行，行相同再比列）。
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

/// 一条诊断 → Core 的 `LspClientDiagnostic` 请求形状
/// （`rust/lithe-core/src/lsp/interface/types.rs:172-185`，`camelCase`）。
///
/// 三个字段是 JDT 侧真正会被读到的，一个都不能省：
/// `source`（`CodeActionHandler` 只要 `"Java"` 的）、`code`（被 `Integer.parseInt` 当 problem id）、
/// `range`（反查 `ProblemLocation` 的偏移）。`severity`/`message` 是契约里的必填项，
/// 原样带上（`tags` 有 `#[serde(default)]`，不必写）。
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
///
/// `pub(crate)`：诊断事件（`crate::events`）用的是同一份位置口径 ——
/// "0 基行 + 0 基 UTF-16 码元列"这个契约字段名只该有一处解析。
pub(crate) fn parse_position(value: &Value) -> Option<JavaPosition> {
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

    /// `is_superseded` 的判据是错误文本的**前缀**，而那段文本由事件泵格式化
    /// （`crate::events::completion_outcome`）。两处形状必须钉在一起：改动任何一边都该红。
    #[test]
    fn superseded_matches_the_event_pump_error_shape() {
        assert!(is_superseded("staleDocumentVersion@document：文档版本已变旧"));
        assert!(is_superseded("requestCancelled@request：（无消息）"));
        assert!(!is_superseded("requestTimeout@request：语义请求超时"));
        assert!(!is_superseded("事件泵已退出（waitEvents: process_failed：会话已不在运行）"));
    }

    /// 一条 codeAction 条目（形状逐字取真实 JDTLS 的响应，见 `.artifacts/p17/probe2.log`：
    /// `Import 'List' (java.util)` 的 `edit.changes` 键是 Core 归一化后的**路径**）。
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

    /// 真实形状必须能解析出"要替换的范围 + 新文本"，并且 Windows 下 `/C:/…` 这个键
    /// 要能对上盘符路径 `C:\…`（少去一个前导斜杠就会**静默**丢掉所有 quick fix）。
    #[test]
    fn code_action_reads_edits_for_the_current_file() {
        let path = PathBuf::from(r"C:\ws\src\main\java\demo\ImportFix.java");
        let CodeActionOutcome::Usable(action) = parse_code_action(&code_action_fixture(), &path)
        else {
            panic!("这条 action 能落地：{:?}", parse_code_action(&code_action_fixture(), &path));
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

    /// 落在**别的文件**上的 action（`Create class 'List<T>'`）不返回，但把条数报出来 ——
    /// 编辑器只有一个 buffer，也没有"新建文件"这条路径，返回它等于给一个点了没反应的菜单项。
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
        match parse_code_action(&action, Path::new(r"C:\ws\src\main\java\demo\ImportFix.java")) {
            CodeActionOutcome::Unusable {
                other_file_edits,
                has_command,
            } => {
                assert_eq!(other_file_edits, 1, "别的文件的编辑条数要报出来（诊断行靠它）");
                assert!(!has_command, "这条不是 command 型");
            }
            other => panic!("只落在别人的文件上就该判 Unusable：{other:?}"),
        }
    }

    /// 缺字段 / 脏数据**不 panic**（服务端输出随版本变化）：没有 `title`、没有 `edit`、
    /// `edit` 是 `null`、`changes` 是数组、`range` 缺一半 —— 一律 `Malformed`/`Unusable`，绝不炸。
    #[test]
    fn code_action_tolerates_missing_and_malformed_fields() {
        let path = Path::new(r"C:\ws\demo\A.java");
        // 没有 `title` 的：结构不认识（Core 也是这个判据 —— 标题是菜单里唯一要显示的东西）。
        for broken in [json!({ "kind": "quickfix", "edit": Value::Null }), json!({})] {
            assert!(
                matches!(
                    parse_code_action(&broken, path),
                    CodeActionOutcome::Malformed
                ),
                "缺 title 应当判 Malformed：{broken}"
            );
        }
        // 有 title 但没有可落地的编辑：Unusable（不是崩溃）。
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
        // 有 `edit` 但同时有 `command`：整条丢掉（执行链本批不做）。
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

    /// 同一 action 里的多条编辑必须**倒序**返回：先应用靠后的，靠前那条的范围才没有被挪走。
    /// （正序应用会让前面那条的列号指向已经被替换过的文本 —— 表现是"修复把代码改乱了"。）
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

    /// `diagnostics_for`：光标（空选区）取**光标所在行**的诊断 —— 光标停在缩进 / 行尾也要能
    /// 拿到那一行的错误，否则 `Ctrl+.` 会只剩 source action（"快速修复没了"）。
    #[test]
    fn relevant_diagnostics_follow_the_cursor_line() {
        let snapshot = vec![
            diagnostic_at(3, 4, 3, 8),
            diagnostic_at(7, 0, 7, 5),
            diagnostic_at(9, 2, 11, 3),
        ];
        // 光标在错误行的行首（列 0，正好在诊断范围之外）。
        let picked = diagnostics_for(&snapshot, position(3, 0), position(3, 0));
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].range.start.utf16_column, 4);

        // 跨行诊断（9..11 行）：光标落在中间那一行也算。
        let picked = diagnostics_for(&snapshot, position(10, 0), position(10, 0));
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].range.start.line, 9);

        // 没有诊断的行：空。
        assert!(diagnostics_for(&snapshot, position(5, 0), position(5, 0)).is_empty());
    }

    /// `diagnostics_for`：有选区时按**相交**取，与选区不沾边的错误不进这次菜单。
    #[test]
    fn relevant_diagnostics_follow_a_selection() {
        let snapshot = vec![diagnostic_at(3, 4, 3, 8), diagnostic_at(7, 0, 7, 5)];
        let picked = diagnostics_for(&snapshot, position(7, 0), position(7, 5));
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].range.start.line, 7);

        // 选区覆盖两条：都要。
        let picked = diagnostics_for(&snapshot, position(0, 0), position(9, 0));
        assert_eq!(picked.len(), 2);

        // 选区在两行之间（7:5 之后、没有诊断的地方）：空。
        assert!(diagnostics_for(&snapshot, position(8, 0), position(8, 2)).is_empty());
    }

    /// 送回 `context.diagnostics` 的 JSON 里三个 JDT 真正会读的字段一个都不能少
    /// （`source` 必须是 `"Java"`，`code` 要被 `Integer.parseInt` 解析，`range` 用来反查偏移）——
    /// 少任何一个，quick fix 就会静默消失。
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

    /// 造一条诊断（UTF-16 列口径，与 Core 给的一致）。
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

    /// 一个 LSP 位置。
    fn position(line: u32, utf16_column: u32) -> JavaPosition {
        JavaPosition {
            line,
            utf16_column,
        }
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

    /// 真实 JDTLS：**同步一份有错的正文之后，服务端会推 `publishDiagnostics`**。
    ///
    /// 编辑器侧的波浪线只有这一个数据来源，而这条链路有两段容易写错：
    /// 1. [`JavaLanguageService::diagnostics`] 是**纯查询**（不起会话、不阻塞），
    ///    它自己**不会**让服务端重算 —— 必须先把正文同步过去（[`JavaLanguageService::sync_document`]，
    ///    首次 `didOpen`、之后 `didChange`）。少这一步，波浪线会停在旧位置或永远不出现；
    /// 2. 诊断是**异步推送**的，同步一返回就查几乎必然拿到空快照。
    ///
    /// 所以这条测试同时钉住"同步 → 推送 → 泵归档 → 查询"整条链路，以及
    /// [`JavaLanguageService::diagnostics`] 的空快照语义（同步之后**最终**必须非空）。
    ///
    /// 等待是**有界轮询**（单调时钟死线 30s，本地实测 <2s）：服务端在另一个进程里，
    /// 宿主侧可观测的边界只有"最近一次快照"这一个，没有可注入的时钟或事件源。
    /// 超时的失败信息带上文档版本号与已等时长，便于区分"同步没生效"与"服务端慢"。
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

        // 一个**必然报错**的正文：`String` 赋给 `int` 是类型错（JDT 报
        // `Type mismatch: cannot convert from String to int`，severity=1）。
        // 第 5 行（0 基 4）是错误所在行，下面的范围断言用它。
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
            // 有界轮询里唯一允许的等待：外部进程异步推送，宿主侧没有事件源可等。
            std::thread::sleep(Duration::from_millis(200));
        }

        assert!(
            !diagnostics.is_empty(),
            "同步（文档版本 {version}）之后 30s 内没有收到诊断：\
             要么 `lsp.syncDocument` 没让服务端重算，要么泵没把 `diagnostics` 事件归档"
        );
        assert!(
            diagnostics.iter().any(|diagnostic| diagnostic.severity == Some(1)),
            "`String` 赋给 `int` 至少该有一条 Error（severity=1）：{diagnostics:?}"
        );
        let on_error_line = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.range.start.line == error_line)
            .unwrap_or_else(|| {
                panic!("没有一条诊断落在错误行 {error_line}：{diagnostics:?}")
            });
        assert!(
            on_error_line.range.start.utf16_column < on_error_line.range.end.utf16_column,
            "诊断范围不能是空区间（空区间的波浪线画不出来）：{on_error_line:?}"
        );
    }

    /// 真实 JDTLS：**没写 import 的类型必须能拿到 `Import '…' (…)` 快速修复，而且带 `edit`**。
    ///
    /// 这条用例守的是本批最关键的一段链路，四件事缺一都会静默失败（菜单里没有修复）：
    ///
    /// 1. **先同步正文再请求**：不同步，Core 会以"文档没打开"拒绝，或者 JDT 按旧正文算范围；
    /// 2. **`context.diagnostics` 必须带上**：实测（`.artifacts/p17/probe.log`）不带诊断时
    ///    同一位置只回 3 条 source action（`Generate Getters/Setters`），一条 quick fix 都没有 ——
    ///    JDT 是拿诊断反查 `ProblemLocation` 的，不是按光标猜；
    /// 3. **诊断的 `source` 必须是 `"Java"`、`code` 必须是可解析的数字**：`CodeActionHandler` 的
    ///    过滤器只要 `"Java"`，`getProblemId` 会 `Integer.parseInt(code)`；
    /// 4. **`edit.changes` 的键是 Core 归一化后的路径**（`/C:/…`），
    ///    [`is_current_file`] 少去一个前导斜杠就会把整条 action 判成"别的文件"而丢掉。
    ///
    /// 断言只看"至少有一个能落地的 action 且它落到本文件上"，不断言具体标题
    /// （JDT 会同时给出 `Import 'List' (java.util)` 与 `(com.sun.tools.javac.util)` 等多个候选，
    /// 以及 `Create class 'List<T>'` 这种要新建文件的 —— 后者由 [`parse_code_action`] 挡掉）。
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

        // `List` 没写 import（`ArrayList` 用全限定名，保证错误只有一个）。
        // 同一个文件也放在 `.artifacts/p17/codeaction-fixture/` 下供实机验证用。
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

        // 等诊断发布（有界轮询，理由见上一条用例）。
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

        // 光标停在 `List` 的起点（与诊断范围同列），与该行的诊断一起送过去。
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
        // 真实的 `Import 'List' (java.util)` 编辑：把 `package demo;\n\n` 换成
        // `package demo;\n\nimport java.util.List;\n\n`（范围 0:0..2:0，见 probe2.log）。
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
}
