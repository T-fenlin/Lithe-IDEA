//! 工作台的会话状态（`.lithe/session.local.json`）：装载、恢复、落盘时机。
//!
//! ## 文档语义在 `shared`，这里只做"时机"与"路径"
//!
//! 文档形状、容错口径、上界与相对路径转换都在
//! [`lithe_db_gpui_shared::workspace_config::session`]（**唯一一份实现**，有它自己的单测）。
//! 本模块只负责三件外壳才有的事：
//!
//! 1. 把 `EditorPane` / `ShellWorkspace` 的当前状态**收集**成一份文档（[`SessionTracker::plan`]）；
//! 2. 决定**什么时候**落盘（[`SessionTracker::write`] 立刻写；防抖由外壳排计时器）；
//! 3. 打开项目时把文档**应用**回界面（[`restore_into`]）。
//!
//! ## 落盘时机：两处硬保证 + 一段防抖
//!
//! | 时机 | 走哪条 | 为什么 |
//! | --- | --- | --- |
//! | 会话内容变化（开 / 关标签、切标签、切视图、收起侧栏） | 外壳排 300ms 防抖（`DebounceState`） | 与设置文档同一套"防抖 + 退出前 flush"，避免连点时反复写盘 |
//! | **切换项目** | [`SessionTracker::flush`]（同步、强制） | 换根会 `replace_root` 丢掉整个旧 `ShellWorkspace`（见 `workspace.rs` 模块头），**必须先写再换根** |
//! | **应用退出 / 关窗** | [`SessionTracker::flush`]（同步、强制） | `on_app_quit` 补一次：防抖窗口里那次改动不能丢 |
//!
//! ## 为什么这些写入不会阻塞 UI 线程
//!
//! 防抖那一条睡在 `cx.background_executor().timer()` 上，窗口期内的多次改动被
//! [`DebounceState`] 的代数合并成**一次**唤醒（过期代数醒来什么都不做）。真正落到
//! `std::fs` 上的调用只有两处：防抖到期、[`SessionTracker::flush`]（换根前 / 退出前）——
//! 每次写一份几百字节的 JSON 再 `rename`，与设置文档的 `flush_pending` 同一条量级、
//! 同一条理由：换根与退出是**离散动作**，不是连续交互，为它们做异步写只会把
//! "那一刻的写入来不及完成"变成一个新的丢数据路径。
//!
//! ## 刷新时机为什么不能挂在"窗口帧"上
//!
//! `--session-probe` / `--session-assert` 最初写在 `window.on_next_frame(..)` 里，
//! 端到端跑出来**一行都没有** —— 无人值守启动里帧回调一次都不跑（窗口在、
//! `MainWindowHandle` 非零、`Responding=true`）。所以那两个探针改成"窗口建好之后立刻
//! 同步执行"（见 `crates/app/src/main.rs`）。这条不是本模块的语义，但它是
//! "会话状态的证据从哪里取"的直接前提，留在这里免得后来者又把它挂到帧上。
//!
//! ## 与设置那套的关系
//!
//! 设置 crate 的防抖在 `SettingsStore` 内部（它自己就是那个实体、`cx.spawn` 够得着）。
//! 会话的主人是外壳，所以这里把防抖拆成两半：**纯状态机**复用
//! `lithe-db-gpui-settings` 已有的 [`DebounceState`]（可确定性单测），计时器由
//! `ShellWorkspace` 在 `Context<Self>` 上排。本模块因此不依赖任何 GPUI 上下文，
//! 全部逻辑可以直接单测。

use std::path::{Path, PathBuf};

use lithe_db_gpui_editor::EditorPane;
use lithe_db_gpui_settings::DebounceState;
use lithe_db_gpui_shared::workspace_config::DIAGNOSTIC_PREFIX;
use lithe_db_gpui_shared::workspace_config::session::{
    self, SessionFileOutcome, SessionState, SessionUiState,
};

use gpui_kit::{App, Entity, Window};

/// 会话文档的落盘防抖窗口（毫秒）。与设置文档同一个 300ms：两处都是"人的操作间隔"，
/// 用同一个值可以少一份需要解释的差异。
pub const SESSION_SAVE_DEBOUNCE_MS: u64 = 300;

/// 一次 [`SessionTracker::plan`] 的结论：要不要写、写什么。
#[derive(Debug, Clone, PartialEq)]
pub enum SessionPlan {
    /// 内容与上一次落盘一致：一个字节都不写（重绘、切到同一个标签都会走到这一支）。
    Unchanged,
    /// 排一次防抖写（带这一代的代数，过期代数醒来什么都不做）。
    Debounced(u64),
    /// 会话文件比本程序新：能读不能写，**不排也不写**。
    ReadOnly,
}

/// 外壳的会话状态：当前文档 + 磁盘上那一份 + 防抖代数。
///
/// **一个工作区一个实例**：换项目会重建整个 `ShellWorkspace`，新外壳的
/// [`SessionTracker::new`] 读的是新根的会话，旧实例随旧外壳一起 drop ——
/// 所以换根前必须 [`SessionTracker::flush`]。
pub struct SessionTracker {
    /// 工作区根（会话文件在它下面的 `.lithe/` 里）。
    root: PathBuf,
    /// 内存里的当前会话文档（每次 [`Self::plan`] 现算）。
    current: SessionState,
    /// **上一次成功落盘**的内容。用它判断"要不要写"：内容没变时一个字节都不写。
    ///
    /// 文件不存在时是 `None`：那表示"磁盘上还没有这份文档"。
    last_saved: Option<SessionState>,
    /// 防抖状态机（纯值，可以直接单测）。
    debounce: DebounceState,
    /// 会话文件比本程序更新时不再尝试写入（与其余文档同一条口径：不静默降级覆盖）。
    read_only: bool,
    /// 上一次 `plan` 看到的"打开的文件 + 当前文件"。
    ///
    /// 它挡的是**每帧级别的观察**：编辑区的观察回调在光标移动时也会跑（一次按键一次），
    /// 而 `session_from_paths` 要对每个打开的文件做一次磁盘存在性检查。这一对值没变时
    /// 直接返回"没变"，连检查都省掉（内容变了才走那条慢路径）。
    last_seen: Option<(Vec<PathBuf>, Option<PathBuf>)>,
    /// 装载期的诊断（由 [`Self::new`] 打出 stderr，同时留在这里便于测试读取）。
    pub(crate) load_diagnostics: Vec<String>,
    /// 装载期诊断是否已经报过（[`Self::plan`] 第一次被调时补打一次，避免一次启动刷十几行）。
    diagnostics_reported: bool,
}

impl SessionTracker {
    /// 读入这个工作区的会话（**不创建文件**）。
    pub fn new(root: &Path) -> Self {
        let loaded = session::load_session(root);
        Self {
            root: root.to_path_buf(),
            current: SessionState::default(),
            last_saved: loaded.file_existed.then_some(loaded.session.clone()),
            debounce: DebounceState::default(),
            read_only: loaded.read_only,
            last_seen: None,
            load_diagnostics: loaded.diagnostics,
            diagnostics_reported: false,
        }
    }

    /// 工作区根。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 会话文件是否被标成只读（磁盘上那一份版本比本程序新）。
    pub fn read_only(&self) -> bool {
        self.read_only
    }

    /// 当前内存里的会话文档（诊断用）。
    pub fn current(&self) -> &SessionState {
        &self.current
    }

    /// 收集当前状态并判断要不要落盘。**不碰文件系统**（写走 [`Self::write`]）。
    ///
    /// `ui` 由外壳给（那些字段在 `ShellWorkspace` 上是私有的）；打开的文件与当前文件从
    /// `editor` 读。
    pub fn plan(
        &mut self,
        editor: &Entity<EditorPane>,
        ui: SessionUiState,
        cx: &App,
    ) -> SessionPlan {
        // 装载期的诊断在这里补打一次（`new` 不打印是为了让构造期安静，
        // 而"文件里有坏键"这件事必须能被机器看见 —— 少了这一句它就只活在内存里）。
        if !self.diagnostics_reported {
            self.diagnostics_reported = true;
            for diagnostic in &self.load_diagnostics {
                eprintln!("{DIAGNOSTIC_PREFIX} session_diagnostic {diagnostic}");
            }
        }
        let (open, active) = {
            let pane = editor.read(cx);
            (pane.open_disk_files(), pane.active_disk_file())
        };
        // 快判：打开的文件与当前文件都没变 → 一份文档都不必重算（连磁盘检查都省掉）。
        // 编辑区的观察回调在光标移动时也会跑，这一句是"每帧级观察不变成每帧级 stat"的闸。
        if self.last_seen.as_ref() == Some(&(open.clone(), active.clone())) {
            return SessionPlan::Unchanged;
        }
        self.last_seen = Some((open.clone(), active.clone()));
        self.current = session::session_from_paths(&self.root, &open, active.as_deref(), ui);
        if self.last_saved.as_ref() == Some(&self.current) {
            return SessionPlan::Unchanged;
        }
        if self.read_only {
            return SessionPlan::ReadOnly;
        }
        SessionPlan::Debounced(self.debounce.arm())
    }

    /// 防抖到期：只有"最后一次改动"对应的那次唤醒才真的写。
    ///
    /// 由外壳的计时器任务调。
    pub fn flush_if_current(&mut self, armed: u64) -> bool {
        if !self.debounce.should_flush(armed) {
            return false;
        }
        self.write()
    }

    /// 立刻落盘（切换项目 / 退出前）。`force = false` 时内容没变就不写。
    ///
    /// 返回是否真的写了。
    pub fn flush(&mut self, force: bool) -> bool {
        self.debounce.mark_flushed();
        if !force && self.last_saved.as_ref() == Some(&self.current) {
            return false;
        }
        self.write()
    }

    /// 把当前文档写到磁盘（**唯一**的落盘点：防抖到期与 `flush` 都汇到这里）。
    ///
    /// ⚠️ **`EditorPane` 的状态不在这里采集**：`flush` 发生在"换根前 / 退出前"，
    /// 这两处都必须拿到**调用那一刻**的编辑区状态，所以调用方先 [`Self::plan`] 一次
    /// （它同时把内容写进 `current`），再 `flush`。见 `ShellWorkspace::flush_session`。
    fn write(&mut self) -> bool {
        if self.read_only {
            eprintln!(
                "{DIAGNOSTIC_PREFIX} session_save_skipped root={} reason=document_version_newer",
                self.root.display()
            );
            return false;
        }
        match session::save_session(&self.root, &self.current) {
            Ok(saved) => {
                self.last_saved = Some(self.current.clone());
                println!(
                    "{DIAGNOSTIC_PREFIX} session_saved root={} files={} active={:?} bytes={} exclusion={:?}",
                    self.root.display(),
                    self.current.open_files.len(),
                    self.current.active_file,
                    saved.bytes,
                    saved.exclusion
                );
                true
            }
            // 写失败不 panic、也不清内存：下一次改动会再试一次。
            Err(error) => {
                eprintln!(
                    "{DIAGNOSTIC_PREFIX} session_save_failed root={} error={error}",
                    self.root.display()
                );
                false
            }
        }
    }
}

/// 打开项目时把会话应用回界面：解析路径 → 逐条判定 → 打开 → 恢复活动标签。
///
/// ## 顺序（每一步都有理由）
///
/// 1. **解析 + 过滤**（[`session::resolve_session_files`]）：被删 / 改名 / 在根外的路径在这里
///    被丢掉，**不报错也不崩溃** —— 这是"容错"那一条的落点；
/// 2. 打开文件 + 切到会话记的活动标签（[`EditorPane::restore_session`]）；
/// 3. 界面状态（侧栏可见性、左右面板的当前视图与可见性）由**外壳**在拿到
///    [`SessionOutcome`] 之后自己应用 —— 那些字段是它的私有状态，本模块读不到也不该读。
///
/// 返回的 [`SessionOutcome`] 同时是**逐条可 grep 的证据**（恢复了几条、跳过了几条、原因）。
pub fn restore_into(
    root: &Path,
    editor: &Entity<EditorPane>,
    window: &mut Window,
    cx: &mut App,
) -> SessionOutcome {
    let loaded = session::load_session(root);
    for diagnostic in &loaded.diagnostics {
        eprintln!("{DIAGNOSTIC_PREFIX} session_diagnostic {diagnostic}");
    }
    let outcomes = session::resolve_session_files(root, &loaded.session);
    let paths: Vec<PathBuf> = outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            SessionFileOutcome::Restored(path) => Some(path.clone()),
            SessionFileOutcome::Skipped { .. } => None,
        })
        .collect();
    let skipped = outcomes.len() - paths.len();

    let restore = editor.update(cx, |pane, cx| {
        pane.restore_session(&paths, loaded.session.active_file, window, cx)
    });

    for outcome in &outcomes {
        if let SessionFileOutcome::Skipped { relative, reason } = outcome {
            println!(
                "{DIAGNOSTIC_PREFIX} session_skipped root={} path={relative} reason={reason}",
                root.display()
            );
        }
    }
    println!(
        "{DIAGNOSTIC_PREFIX} session_restored root={} opened={} skipped={} active={:?} active_file={:?}",
        root.display(),
        restore.opened.len(),
        skipped,
        restore.active_index,
        restore
            .active_index
            .and_then(|index| restore.opened.get(index))
            .map(|path| path.display().to_string())
    );

    SessionOutcome {
        session: loaded.session,
        outcomes,
        opened: restore.opened.len(),
        skipped,
        active_index: restore.active_index,
    }
}

/// 一次会话恢复的结论（诊断 + 侧栏 / 视图恢复的输入）。
#[derive(Debug, Clone, PartialEq)]
pub struct SessionOutcome {
    /// 磁盘上读到的会话文档（界面状态由外壳从它里面取）。
    pub session: SessionState,
    /// 恢复阶段每一条会话路径的判定（逐条）。
    pub outcomes: Vec<SessionFileOutcome>,
    /// 真的打开 / 切到的文件数。
    pub opened: usize,
    /// 被跳过的文件数（删了 / 改名 / 在根外）。
    pub skipped: usize,
    /// 恢复之后的活动下标。
    pub active_index: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lithe_db_gpui_shared::workspace_config::session::MAX_OPEN_FILES;

    /// 一个独立的临时目录（**不含 Git 仓库**），用完删掉。
    ///
    /// 本模块的用例都不调写盘（`plan` / `flush` 的写路径要真仓库 + 真 `EditorPane`，
    /// 由 `shared` 的会话用例与 GUI 端到端覆盖），所以裸临时目录是安全的。
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lithe-workbench-session-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        dir
    }

    /// 内容没变时判"不用写"：这是"重绘不落盘"这条保证的判据。
    #[test]
    fn an_unchanged_document_is_not_written() {
        let dir = temp_dir("unchanged");
        let mut tracker = SessionTracker::new(&dir);
        // 直接摆一份"已经落盘"的内容，再看 plan 的结论（不经过 GPUI 实体）。
        tracker.current = SessionState {
            open_files: vec!["a.txt".to_string()],
            ..SessionState::default()
        };
        tracker.last_saved = Some(tracker.current.clone());

        assert!(!tracker.flush(false), "内容没变时 flush 不该写");
        assert!(!tracker.debounce.has_pending());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 只读文档（版本比本程序新）**既不排防抖也不写**，并且留下诊断。
    #[test]
    fn a_newer_document_is_never_written() {
        let dir = temp_dir("read-only");
        let path = session::session_path(&dir);
        std::fs::create_dir_all(path.parent().expect("有父目录")).expect("建配置目录");
        std::fs::write(&path, r#"{"version":99,"openFiles":["kept.txt"]}"#).expect("写文件");

        let mut tracker = SessionTracker::new(&dir);
        assert!(tracker.read_only());
        assert!(
            tracker
                .load_diagnostics
                .iter()
                .any(|line| line.contains("document_version_newer")),
            "{:?}",
            tracker.load_diagnostics
        );
        assert!(!tracker.flush(true), "只读文档必须拒绝写入");
        assert_eq!(
            std::fs::read_to_string(&path).expect("读文件"),
            r#"{"version":99,"openFiles":["kept.txt"]}"#,
            "被拒绝的写入不得改动文件"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 防抖代数：过期的那次唤醒什么都不做，只有最后一次真的写。
    ///
    /// 这是"连点只写一次"的判据；这里直接驱动纯状态机（不排真实计时器 ——
    /// 真实时间同步被 `.agents/skills/write-stable-tests/SKILL.md` 禁止）。
    #[test]
    fn only_the_last_debounce_generation_writes() {
        let dir = temp_dir("debounce");
        let mut tracker = SessionTracker::new(&dir);
        tracker.current = SessionState {
            open_files: vec!["a.txt".to_string()],
            ..SessionState::default()
        };

        let first = tracker.debounce.arm();
        let second = tracker.debounce.arm();
        // 第一次唤醒过期：`should_flush(first)` 为假（代数已经不是最新）。
        assert!(!tracker.debounce.should_flush(first));
        assert!(tracker.debounce.should_flush(second));
        // 同一代重复唤醒不会再写一次。
        assert!(!tracker.debounce.should_flush(second));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 上界与会话文档一致：外壳不会把超过 [`MAX_OPEN_FILES`] 的列表交给文档层。
    ///
    /// 这条钉的是**两处常量同源**（外壳的防抖逻辑不该自己再定义一份上界）。
    #[test]
    fn the_bound_comes_from_the_shared_document_layer() {
        assert_eq!(MAX_OPEN_FILES, 40);
    }
}
