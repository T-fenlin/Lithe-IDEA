use std::cell::RefCell;
use std::io::{Read, Write as _};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::message_scroller::MessageScrollerState;
use gpui::{AppContext as _, Context, SharedString, WeakEntity, Window};

use crate::ansi::TerminalText;
use crate::constants::{
    EXIT_POLL_ATTEMPTS, EXIT_POLL_INTERVAL, MAX_LINES, READ_BUFFER, error_fallback,
    input_placeholder,
};
use crate::profile::{
    TerminalProfile, command_exists, default_profile, profile_for_shell_id, profiles,
};
use crate::terminal_view::{TabSession, TerminalPane};

pub(crate) enum SessionState {
    Running,
    Exited(i32),
    Failed(String),
}

pub(crate) enum SessionEvent {
    Output(Vec<u8>),
    StreamClosed,
    StreamError(String),
}

pub(crate) enum PumpStep {
    Continue,
    StreamsClosed,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalPaneEvent {
    LastTabClosed,
}

pub(crate) struct Session {
    pub(crate) child: Option<Child>,
    pub(crate) writer: Option<Sender<Vec<u8>>>,
    pub(crate) state: SessionState,
    pub(crate) text: TerminalText,
    pub(crate) closed_streams: usize,
    pub(crate) io_error: Option<String>,
    pub(crate) pid: Option<u32>,
}

impl Session {
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

pub(crate) fn blocking_recv(receiver: &Arc<Mutex<Receiver<SessionEvent>>>) -> Option<SessionEvent> {
    let guard = match receiver.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.recv().ok()
}

impl gpui_kit::EventEmitter<TerminalPaneEvent> for TerminalPane {}

impl TerminalPane {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let profiles = profiles();
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |state, cx| {
            state.set_placeholder(input_placeholder(), window, cx);
        });

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

        Self {
            tabs: Vec::new(),
            active: 0,
            next_id: 1,
            next_run: 1,
            profiles,
            active_profile: 0,
            default_shell_id: crate::profile::SHELL_SYSTEM_DEFAULT.to_string(),
            font_size: None,
            input,
            _input_events: input_events,
        }
    }

    pub fn ensure_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.tabs.is_empty() {
            return;
        }
        self.open_tab(cx);

        let shell: WeakEntity<Self> = cx.entity().downgrade();
        window.defer(cx, move |window, cx| {
            let _ = shell.update(cx, |this, cx| this.focus_input(window, cx));
        });
    }

    pub fn new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let created = self.open_tab(cx);
        if created {
            self.focus_input(window, cx);
        }
    }

    pub fn new_tab_in(
        &mut self,
        directory: impl Into<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let directory = directory.into();
        let profile = self
            .profiles
            .get(self.active_profile)
            .cloned()
            .or_else(default_profile)
            .map(|profile| profile.with_working_directory(directory));
        let created = self.open_tab_with(profile, cx);
        if created {
            self.focus_input(window, cx);
        }
    }

    pub fn close_active_tab(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(tab) = self.tabs.get(self.active) else {
            return false;
        };
        let id = tab.id;
        self.close_tab(id, cx);
        true
    }

    fn focus_input(&self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.input.clone();
        input.update(cx, |state, cx| state.focus(window, cx));
    }

    #[allow(dead_code)]
    pub fn profiles(&self) -> &[TerminalProfile] {
        &self.profiles
    }

    pub fn set_font_size(&mut self, size: Option<f32>, cx: &mut Context<Self>) {
        if self.font_size == size {
            return;
        }
        self.font_size = size;
        println!("S1_TERMINAL font_size={size:?}");
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn replace_profiles(&mut self, profiles: Vec<TerminalProfile>, cx: &mut Context<Self>) {
        self.profiles = profiles;
        if self.active_profile >= self.profiles.len() {
            self.active_profile = 0;
        }
        cx.notify();
    }

    #[allow(dead_code)]
    pub fn add_profile(&mut self, profile: TerminalProfile, cx: &mut Context<Self>) {
        self.profiles.push(profile);
        cx.notify();
    }

    pub fn set_default_shell(&mut self, shell_id: &str, cx: &mut Context<Self>) {
        if self.default_shell_id == shell_id {
            return;
        }
        self.default_shell_id = shell_id.to_string();

        let mut next = profiles();
        let outcome = match profile_for_shell_id(shell_id) {
            None => "systemDefault",
            Some(preferred) => {
                if command_exists(&preferred.program().to_string_lossy()) {
                    next.retain(|profile| profile.program() != preferred.program());
                    next.insert(0, preferred);
                    "applied"
                } else {
                    "unavailable"
                }
            }
        };
        self.replace_profiles(next, cx);
        self.active_profile = 0;
        println!(
            "S1_TERMINAL default_shell={} outcome={outcome} profiles={}",
            if shell_id.is_empty() {
                "(system)"
            } else {
                shell_id
            },
            self.profiles.len()
        );
        cx.notify();
    }

    pub(crate) fn open_tab(&mut self, cx: &mut Context<Self>) -> bool {
        self.open_tab_with(None, cx)
    }

    pub(crate) fn open_tab_with(
        &mut self,
        profile: Option<TerminalProfile>,
        cx: &mut Context<Self>,
    ) -> bool {
        let profile = profile
            .or_else(|| self.profiles.get(self.active_profile).cloned())
            .or_else(default_profile);
        let title: SharedString = match &profile {
            Some(profile) => profile.name().clone(),
            None => error_fallback(),
        };

        let id = self.next_id;
        self.next_id += 1;
        let run = self.next_run;
        self.next_run += 1;

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

    pub(crate) fn close_tab(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        let mut tab = self.tabs.remove(index);
        tab.session.shutdown();
        println!("S1_TERMINAL_CLOSED id={id}");

        if self.tabs.is_empty() {
            self.active = 0;
            cx.emit(TerminalPaneEvent::LastTabClosed);
        } else if index < self.active {
            self.active -= 1;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
        cx.notify();
    }

    pub(crate) fn retry_tab(&mut self, id: u64, cx: &mut Context<Self>) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        let run = self.next_run;
        self.next_run += 1;

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

        self.spawn_pump(id, run, receiver, cx);
        cx.notify();
    }

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
                let _ = state.splice(0..current - target, 0, cx);
            }
        });
    }

    pub(crate) fn settle_exit(&mut self, id: u64, run: u64, cx: &mut Context<Self>) -> bool {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return true;
        };
        if self.tabs[index].run != run {
            return true;
        }
        let settled = self.tabs[index].session.settle_exit();
        if settled {
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
