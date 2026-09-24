//! 子进程会话：spawn 进程、两条读线程、stdin 写线程、`mpsc` 通道、kill/wait、代次。
//!
//! 从 `shell_probe/terminal.rs` 的「子进程会话」一节原样拆出（逐字搬迁，只调整可见性）。
//! 生命周期的硬要求：**关页签 / 面板销毁时都要 `kill()` + `wait()`**，绝不留下僵尸进程
//! （真机窗口销毁也杀全部 PTY，`windows/tauri/crates/terminal/src/manager.rs:99-113`）。
//!
//! `impl TerminalPane` 里"与进程有关"的方法也留在本文件（开/关/重试页签、泵、退出码），
//! 渲染方法在 `terminal_view.rs` —— 一个类型允许有多个 `impl` 块，分文件放不会改变语义。
//!
//! ## 与 Windows 前端的差异
//!
//! Windows 用 `portable-pty` 0.9 开 **ConPTY**（`windows/tauri/crates/terminal/Cargo.toml:11`、
//! `connection.rs:36-49`），本 crate 用 `std::process::Command` + 三个管道，**没有 PTY**，
//! 因此没有控制台、`Ctrl+C` 送不进去、全屏 TUI 程序不可用（能力边界在 `terminal_view.rs` 常驻显示）。

use std::cell::RefCell;
use std::io::{Read, Write as _};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::message_scroller::MessageScrollerState;
use gpui_kit::{AppContext as _, Context, SharedString, Window};

use crate::ansi::TerminalText;
use crate::constants::{
    EXIT_POLL_ATTEMPTS, EXIT_POLL_INTERVAL, MAX_LINES, READ_BUFFER, error_fallback,
    input_placeholder,
};
use crate::profile::{TerminalProfile, default_profile, profiles};
use crate::terminal_view::{TabSession, TerminalPane};

/// 会话状态（真机的三档：运行中 / 已退出（带退出码）/ 起不来）。
pub(crate) enum SessionState {
    Running,
    /// 已退出（退出码）。
    Exited(i32),
    /// 起不来（`Command::spawn` 失败或 shell 探测为空）。存的是操作系统错误原文。
    Failed(String),
}

/// 一条读线程 / 写线程发给 UI 的事件。
pub(crate) enum SessionEvent {
    /// 一段原始输出字节（stdout / stderr 各一条线程，混在一条通道里）。
    Output(Vec<u8>),
    /// 某条读线程读到 EOF。
    StreamClosed,
    /// 某条读线程读失败（`io::Error` 原文，进诊断）。
    StreamError(String),
}

/// 泵的下一步动作（决定要不要等退出码、要不要收工）。
pub(crate) enum PumpStep {
    Continue,
    /// 两条管道都 EOF 了 → 去收退出码。
    StreamsClosed,
    /// 实体已经没了 → 收工。
    Stop,
}

/// 一条终端会话：子进程 + 写线程句柄 + 已清洗的输出文本。
pub(crate) struct Session {
    /// 子进程。**一直留在这里**，`Drop` / 关页签时才有 `kill()` + `wait()` 的能力。
    pub(crate) child: Option<Child>,
    /// stdin 写入通道（写线程持有真正的 `ChildStdin`；这里的 `send` 永不阻塞 UI）。
    pub(crate) writer: Option<Sender<Vec<u8>>>,
    pub(crate) state: SessionState,
    pub(crate) text: TerminalText,
    /// 已经 EOF 的读线程数（到 2 就说明两条管道都关了）。
    pub(crate) closed_streams: usize,
    /// 读线程报回来的 IO 错误（有则显示在诊断行里）。
    pub(crate) io_error: Option<String>,
    /// 子进程 pid（诊断行）。
    pub(crate) pid: Option<u32>,
}

impl Session {
    /// 起一个进程，并把它的 stdout / stderr / stdin 都接上管道。
    ///
    /// 起不来（`spawn` 失败）返回 `Err(系统错误原文)`；调用方把页签置成失败态
    /// （真机的失败态是 `TerminalErrorBoundary` + `terminal.errorTitle`，`terminal-error-boundary.tsx:40-51`）。
    pub(crate) fn start(
        profile: &TerminalProfile,
    ) -> Result<(Self, Receiver<SessionEvent>), String> {
        let mut command = Command::new(profile.program());
        command
            .args(profile.args())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(directory) = profile.working_directory() {
            command.current_dir(directory);
        }

        let mut child = command
            .spawn()
            .map_err(|error| format!("{}: {error}", profile.program().display()))?;
        let pid = child.id();

        let (events, receiver) = mpsc::channel::<SessionEvent>();

        // stdout / stderr 各一条读线程：管道下这两条流没有顺序保证，抵达顺序就是显示顺序
        // （真机的运行输出也是这样合并成一条流：`run.rs` 只发 `run-output { sessionId, chunk }`，
        // `windows/tauri/src-tauri/src/run.rs:1932-1965`）。stderr 未做单独着色，见未实现清单第 4 条。
        if let Some(stdout) = child.stdout.take() {
            spawn_reader(stdout, events.clone());
        }
        if let Some(stderr) = child.stderr.take() {
            spawn_reader(stderr, events.clone());
        }

        let (writer, write_requests) = mpsc::channel::<Vec<u8>>();
        let mut writer = Some(writer);
        if let Some(stdin) = child.stdin.take() {
            spawn_writer(stdin, write_requests);
        } else {
            writer = None;
        }

        Ok((
            Self {
                child: Some(child),
                writer,
                state: SessionState::Running,
                text: TerminalText::default(),
                closed_streams: 0,
                io_error: None,
                pid: Some(pid),
            },
            receiver,
        ))
    }

    /// 起不来的会话（探测不到 shell，或 `spawn` 失败）。
    pub(crate) fn failed(message: String) -> Self {
        Self {
            child: None,
            writer: None,
            state: SessionState::Failed(message),
            text: TerminalText::default(),
            closed_streams: 0,
            io_error: None,
            pid: None,
        }
    }

    pub(crate) fn is_running(&self) -> bool {
        matches!(self.state, SessionState::Running)
    }

    /// 收摊：先关 stdin（让 shell 自己收尾），再 `kill` + `wait`。
    ///
    /// **`wait()` 不能省**：只 `kill()` 不回收会在进程表里留僵尸进程。
    /// 这是"必须能关进程"的落点（真机窗口销毁时也会杀全部 PTY，
    /// `windows/tauri/crates/terminal/src/manager.rs:99-113`）。
    pub(crate) fn shutdown(&mut self) {
        self.writer = None;
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            match child.wait() {
                Ok(status) => self.state = SessionState::Exited(status.code().unwrap_or(-1)),
                Err(error) => self.state = SessionState::Failed(error.to_string()),
            }
        }
    }

    /// 如果进程已经退出，把退出码记下来（`Drop` 之外唯一改 `state` 的地方）。
    ///
    /// 返回 `true` 表示状态已经确定，泵可以收工。
    pub(crate) fn settle_exit(&mut self) -> bool {
        if !self.is_running() {
            return true;
        }
        let Some(child) = self.child.as_mut() else {
            return true;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.state = SessionState::Exited(status.code().unwrap_or(-1));
                true
            }
            Ok(None) => false,
            Err(error) => {
                self.state = SessionState::Failed(error.to_string());
                true
            }
        }
    }
}

/// 一条管道读线程：把字节按块丢进通道，EOF / 出错时各发一条事件。
///
/// 用 `read` 而不是 `read_line`：不按行切，避免"没有换行的输出（进度条、提示符）"卡住不显示。
pub(crate) fn spawn_reader<R: Read + Send + 'static>(mut reader: R, events: Sender<SessionEvent>) {
    std::thread::spawn(move || {
        let mut buffer = [0u8; READ_BUFFER];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    let _ = events.send(SessionEvent::StreamClosed);
                    break;
                }
                Ok(read) => {
                    if events
                        .send(SessionEvent::Output(buffer[..read].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    let _ = events.send(SessionEvent::StreamError(error.to_string()));
                    let _ = events.send(SessionEvent::StreamClosed);
                    break;
                }
            }
        }
    });
}

/// 一条 stdin 写线程：`send` 永不阻塞 UI；线程退出即关闭 stdin（子进程读到 EOF）。
pub(crate) fn spawn_writer(mut stdin: ChildStdin, requests: Receiver<Vec<u8>>) {
    std::thread::spawn(move || {
        while let Ok(bytes) = requests.recv() {
            if stdin.write_all(&bytes).is_err() {
                break;
            }
            if stdin.flush().is_err() {
                break;
            }
        }
    });
}

/// 阻塞地在后台线程上取一条事件。
///
/// 单独抽成一个**同步函数**（而不是把 `lock().recv()` 直接写进 `async` 块）是有意的：
/// `std::sync::MutexGuard` 是 `!Send`，只有让锁的作用域完全落在这个同步函数里，
/// 外面那个要交给 `background_spawn`（要求 `Send`）的 `async` 块才只捕获
/// `Arc<Mutex<Receiver<…>>>` 这一个 `Send` 值。
pub(crate) fn blocking_recv(receiver: &Arc<Mutex<Receiver<SessionEvent>>>) -> Option<SessionEvent> {
    let guard = match receiver.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.recv().ok()
}

// ---------------------------------------------------------------------------
// 组件
// ---------------------------------------------------------------------------

/// 一个页签：标题 + 会话 + 输出区状态。

impl TerminalPane {
    /// 起一个默认 shell（Windows 优先 `powershell`，回退 `cmd`）的终端面板，并开第一个页签。
    ///
    /// 探测不到任何 shell 时也会建一个页签，但它是失败态并显示 [`error_fallback`] +
    /// 「重试」（真机的失败态文案见 `i18n/locale.ts:7549-7551`）。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let profiles = profiles();
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |state, cx| {
            state.set_placeholder(input_placeholder(), window, cx);
        });

        // `subscribe_in`（而不是 `subscribe`）：发送之后要用 `window` 清空输入框，
        // 而 `InputState::set_value` 需要 `&mut Window`（`gpui-base-0.6.6/src/input/base/state.rs:897`）。
        let input_events = cx.subscribe_in(
            &input,
            window,
            |this: &mut Self,
             _input,
             event: &InputEvent,
             window: &mut Window,
             cx: &mut Context<Self>| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.submit_input(window, cx);
                }
            },
        );

        let mut pane = Self {
            tabs: Vec::new(),
            active: 0,
            next_id: 1,
            next_run: 1,
            profiles,
            active_profile: 0,
            input,
            _input_events: input_events,
        };
        pane.open_tab(cx);
        pane
    }

    /// 新开一个页签（默认配置文件）。
    ///
    /// 真机的新建入口是 `terminal.new`（`cmd+t`，`command-registry.ts:258-267`）与
    /// 页签条右端的 `+`（`terminal-tab-bar.tsx:724-735`）。
    pub fn new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let created = self.open_tab(cx);
        if created {
            self.focus_input(window, cx);
        }
    }

    /// 让命令输入行拿焦点。
    ///
    /// **刻意不在构造期调用**：`new` 里窗口根视图还不是 `Root`、输入框也还没上树，
    /// 那时抢焦点容易变成空焦点（`gpui/UI-MAP.md` §1.3 记过构造期碰焦点/浮层的坑）。
    /// 宿主在首次渲染之后调一次即可。
    pub fn focus_input(&self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.input.clone();
        input.update(cx, |state, cx| state.focus(window, cx));
    }

    /// 当前的配置文件列表（"自定义 shell"接线点，见 [`TerminalProfile`] 的文档）。
    ///
    /// ⚠️ 这三个方法目前**没有调用方** —— 它们是设置界面接进来之前的**预留接线点**，
    /// 所以显式 `allow(dead_code)`，而不是删掉（删了就等于"自定义 shell"这个需求没落点）。
    #[allow(dead_code)]
    pub fn profiles(&self) -> &[TerminalProfile] {
        &self.profiles
    }

    /// 换一套配置文件（设置界面接线点）。会保留现有页签，只影响之后新建的页签。
    #[allow(dead_code)]
    pub fn replace_profiles(&mut self, profiles: Vec<TerminalProfile>, cx: &mut Context<Self>) {
        self.profiles = profiles;
        if self.active_profile >= self.profiles.len() {
            self.active_profile = 0;
        }
        cx.notify();
    }

    /// 追加一个自定义 shell（设置界面接线点）。
    #[allow(dead_code)]
    pub fn add_profile(&mut self, profile: TerminalProfile, cx: &mut Context<Self>) {
        self.profiles.push(profile);
        cx.notify();
    }

    /// 开一个页签并选中它。返回是否真的建了页签。
    pub(crate) fn open_tab(&mut self, cx: &mut Context<Self>) -> bool {
        // 列表为空时回落到 `default_profile()`：设置界面可能把列表清空过，而"默认 shell"
        // 永远是同一条规则（Windows 优先 powershell、回退 cmd）。
        let profile = self
            .profiles
            .get(self.active_profile)
            .cloned()
            .or_else(default_profile);
        let title: SharedString = match &profile {
            Some(profile) => profile.name().clone(),
            None => error_fallback(),
        };

        let id = self.next_id;
        self.next_id += 1;
        let run = self.next_run;
        self.next_run += 1;

        // 起进程：这是唯一会失败的步骤，失败也不吞掉，直接落到页签状态里。
        let (session, receiver) = match &profile {
            Some(profile) => match Session::start(profile) {
                Ok(started) => started,
                Err(error) => {
                    println!("S1_TERMINAL_FAILED tab={id} error={error}");
                    (Session::failed(error), mpsc::channel::<SessionEvent>().1)
                }
            },
            None => {
                println!("S1_TERMINAL_FAILED tab={id} error=no-shell-detected");
                (
                    Session::failed(error_fallback().to_string()),
                    mpsc::channel::<SessionEvent>().1,
                )
            }
        };

        if let (SessionState::Running, Some(pid)) = (&session.state, session.pid) {
            println!("S1_TERMINAL_TAB id={id} profile={} pid={pid}", title);
        }

        let scroller = cx.new(|cx| MessageScrollerState::new(0, cx));
        self.tabs.push(TabSession {
            id,
            run,
            title,
            profile: profile.unwrap_or_else(|| TerminalProfile::new("cmd")),
            session,
            scroller,
            rows: Rc::new(RefCell::new(Vec::new())),
        });
        self.active = self.tabs.len() - 1;

        self.spawn_pump(id, run, receiver, cx);
        cx.notify();
        true
    }

    /// 关一个页签：**杀进程 + 回收**（"必须能关进程"的落点之一）。
    ///
    /// 关闭后的选中位置照真机：关闭活动页签后落到原下标位置，越界则落到前一个
    /// （`terminal-container.tsx:230-250`）。
    pub(crate) fn close_tab(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        let mut tab = self.tabs.remove(index);
        tab.session.shutdown();
        println!("S1_TERMINAL_CLOSED id={id}");

        if self.tabs.is_empty() {
            self.active = 0;
        } else if index < self.active {
            self.active -= 1;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
        cx.notify();
    }

    /// 重新拉起当前页签的进程（失败/已退出后的「重试」）。
    pub(crate) fn retry_tab(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        let run = self.next_run;
        self.next_run += 1;

        // 先把旧会话收干净，再起新的；旧泵靠代次失配自行丢弃残留事件（见 [`TabSession::run`]）。
        let (session, receiver) = {
            let tab = &mut self.tabs[index];
            tab.session.shutdown();
            tab.session.text.clear();
            tab.rows.borrow_mut().clear();
            match Session::start(&tab.profile) {
                Ok(started) => started,
                Err(error) => {
                    println!("S1_TERMINAL_FAILED id={id} error={error}");
                    (Session::failed(error), mpsc::channel::<SessionEvent>().1)
                }
            }
        };

        let scroller = self.tabs[index].scroller.clone();
        scroller.update(cx, |state, cx| state.reset(0, cx));
        self.tabs[index].session = session;
        self.tabs[index].run = run;

        // 旧泵的通道在旧读线程退出后断开（`recv()` 返回 `Err`），会自己收工；
        // 新泵用新通道新代次，互不干扰。
        self.spawn_pump(id, run, receiver, cx);
        cx.notify();
    }

    /// 清空活动页签的输出（真机 `terminal.contextClear`，`terminal-tab-bar.tsx:959-964`）。
    pub(crate) fn clear_active_output(&mut self, cx: &mut Context<Self>) {
        let active = self.active;
        let Some(tab) = self.tabs.get_mut(active) else {
            return;
        };
        tab.session.text.clear();
        tab.rows.borrow_mut().clear();
        let scroller = tab.scroller.clone();
        scroller.update(cx, |state, cx| state.reset(0, cx));
        cx.notify();
    }

    /// 把输入行里的一行命令送进活动页签的 stdin。
    ///
    /// 结尾用 `\r\n`：Windows 上 `cmd.exe` / `powershell.exe` 都按 CRLF 断行
    /// （实测见文件末尾）。**不做本地回显** —— 这两个 shell 在管道下自己会打印提示符与命令回显。
    pub(crate) fn submit_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let value = self.input.read(&**cx).value();
        let line = value.trim_end_matches(['\r', '\n']).to_string();
        if line.trim().is_empty() {
            return;
        }

        let mut delivered = false;
        if let Some(tab) = self.tabs.get(self.active)
            && let Some(writer) = &tab.session.writer
        {
            let mut bytes = line.into_bytes();
            bytes.extend_from_slice(b"\r\n");
            delivered = writer.send(bytes).is_ok();
        }
        if !delivered {
            println!("S1_TERMINAL_STDIN_DROPPED tab_index={}", self.active);
        }

        let input = self.input.clone();
        input.update(cx, |state, cx| {
            state.set_value(SharedString::default(), window, cx);
        });
        cx.notify();
    }

    /// 起一个泵任务：把这条会话的事件搬到 UI 上。
    ///
    /// 为什么这样写（`std::sync::mpsc` 没有异步接收端，而 `gpui/shell/Cargo.toml` **没有**
    /// `async_channel` 依赖、也不允许改 Cargo.toml）：
    /// 阻塞式 `recv()` 放进 `cx.background_spawn`（`gpui-pre-0.3.6/src/app.rs:3073`，
    /// 要求 `Send + 'static`），前台任务 `await` 它的结果 —— 每来一块输出才醒一次，
    /// 既不轮询也不用阻塞 UI 线程。`Mutex<Receiver>` 只是为了把 `!Sync` 的 `Receiver`
    /// 搬进那次 `background_spawn`（同一时刻只有一个等待者，不会争锁）。
    pub(crate) fn spawn_pump(
        &self,
        id: u64,
        run: u64,
        receiver: Receiver<SessionEvent>,
        cx: &mut Context<Self>,
    ) {
        let receiver = Arc::new(Mutex::new(receiver));
        cx.spawn(async move |this, cx| {
            loop {
                let receiver = receiver.clone();
                let event = cx
                    .background_spawn(async move { blocking_recv(&receiver) })
                    .await;

                let Some(event) = event else { break };
                let step = match this.update(cx, |this, cx| this.handle_event(id, run, event, cx)) {
                    Ok(step) => step,
                    Err(_) => break,
                };
                match step {
                    PumpStep::Continue => {}
                    PumpStep::Stop => break,
                    PumpStep::StreamsClosed => {
                        // 两条管道都 EOF：进程基本已经结束，`try_wait` 收退出码。
                        for _ in 0..EXIT_POLL_ATTEMPTS {
                            let settled = this
                                .update(cx, |this, cx| this.settle_exit(id, run, cx))
                                .unwrap_or(true);
                            if settled {
                                break;
                            }
                            cx.background_spawn(async { std::thread::sleep(EXIT_POLL_INTERVAL) })
                                .await;
                        }
                        break;
                    }
                }
            }
        })
        .detach();
    }

    /// 处理一条会话事件（UI 线程）。
    pub(crate) fn handle_event(
        &mut self,
        id: u64,
        run: u64,
        event: SessionEvent,
        cx: &mut Context<Self>,
    ) -> PumpStep {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return PumpStep::Stop;
        };
        // 代次失配 = 这是重试之前那条会话的残留事件，直接丢弃并收工。
        if self.tabs[index].run != run {
            return PumpStep::Stop;
        }

        match event {
            SessionEvent::Output(bytes) => {
                let fresh = {
                    let tab = &mut self.tabs[index];
                    tab.session.text.push_bytes(&bytes);
                    tab.session.text.take_completed()
                };
                self.append_rows(index, fresh, cx);
                cx.notify();
                PumpStep::Continue
            }
            SessionEvent::StreamError(message) => {
                println!("S1_TERMINAL_IO_ERROR id={id} error={message}");
                self.tabs[index].session.io_error = Some(message);
                cx.notify();
                PumpStep::Continue
            }
            SessionEvent::StreamClosed => {
                let closed = {
                    let tab = &mut self.tabs[index];
                    tab.session.closed_streams += 1;
                    tab.session.closed_streams
                };
                if closed >= 2 {
                    PumpStep::StreamsClosed
                } else {
                    PumpStep::Continue
                }
            }
        }
    }

    /// 把新完成的行并进输出区，并让 `MessageScrollerState` 的行数与之一致。
    ///
    /// 行数上限 [`MAX_LINES`] 只从**最前面**丢：`MessageScrollerState::splice` 支持删区间
    /// （`gpui-component-0.6.6/src/message_scroller.rs:80-106`），锚点由组件自己保。
    pub(crate) fn append_rows(&mut self, index: usize, fresh: Vec<String>, cx: &mut Context<Self>) {
        let tab = &mut self.tabs[index];
        {
            let mut rows = tab.rows.borrow_mut();
            rows.extend(fresh.into_iter().map(SharedString::from));
            let overflow = rows.len().saturating_sub(MAX_LINES);
            if overflow > 0 {
                rows.drain(..overflow);
            }
        }

        let target = tab.rows.borrow().len();
        let scroller = tab.scroller.clone();
        scroller.update(cx, |state, cx| {
            let current = state.item_count();
            if target > current {
                let _ = state.append(target - current, cx);
            } else if target < current {
                // 只有"清空/截断"会走到这里。
                let _ = state.splice(0..current - target, 0, cx);
            }
        });
    }

    /// 两条管道 EOF 之后收退出码。返回 `true` 表示状态已确定。
    pub(crate) fn settle_exit(&mut self, id: u64, run: u64, cx: &mut Context<Self>) -> bool {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return true;
        };
        if self.tabs[index].run != run {
            return true;
        }
        let settled = self.tabs[index].session.settle_exit();
        if settled {
            // `replacedBytes > 0` 就是"输出里有非 UTF-8 字节（例如 GBK 中文）"的证据。
            let replaced = self.tabs[index].session.text.replaced_bytes;
            let state = &self.tabs[index].session.state;
            match state {
                SessionState::Exited(code) => {
                    println!("S1_TERMINAL_EXIT id={id} code={code} replacedBytes={replaced}");
                }
                SessionState::Failed(error) => {
                    println!("S1_TERMINAL_EXIT id={id} error={error} replacedBytes={replaced}");
                }
                SessionState::Running => {}
            }
            cx.notify();
        }
        settled
    }
}
