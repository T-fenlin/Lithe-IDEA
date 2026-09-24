//! 终端配置文件：起哪个程序、带什么参数、在哪个目录（**"自定义 shell"就在这里接线**）。
//!
//! 从 `shell_probe/terminal.rs` 的「终端配置文件」一节原样拆出（逐字搬迁，只调整可见性）。
//!
//! 字段照 Windows 的 `ResolvedTerminalLaunch`（`features/terminal/utils/terminal-profiles.ts:9-15`：
//! `shell / workingDirectory / initialCommand / name / profileId`）裁剪，只留本外壳用得到的三个。
//!
//! ## 🔌 设置界面的接线点在这里
//!
//! 1. 用户在设置里改「终端默认配置文件 / shell 路径 / 参数 / 启动目录」→ 调
//!    `TerminalPane::add_profile` 或 `TerminalPane::replace_profiles` 把新的
//!    `TerminalProfile` 送进来（Windows 侧的存储是 `features/terminal/stores/profiles.store.ts`
//!    与 `settings.terminalDefaultProfileId`，`terminal-profiles.ts:56-88` 做优先级解析）；
//! 2. 默认配置文件由 `default_profile` 给：Windows 优先 `powershell`、回退 `cmd`，
//!    **用运行时探测**决定（`where.exe` 失败再试 `which`，不写 `#[cfg(target_os)]`）；
//! 3. 页签条右端的 ⌄ 菜单（`TerminalPane::render_tab_bar`）就是"选择终端配置文件"的落点，
//!    对应真机 `terminal-tab-bar.tsx:442-451,557-565`；
//! 4. 真机的新配置文件可以带 `startupCommands`（`terminal-profiles.ts:77`）；本 crate **没有**初始
//!    命令通路（没有 PTY、也没有"先注入一行"的落点），要接的话就在这里加字段并在
//!    `Session::start` 之后往 stdin 写一行。

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use gpui_kit::SharedString;

#[derive(Clone)]
#[non_exhaustive]
pub struct TerminalProfile {
    /// 展示名（真机取 `TerminalProfile.name`，`terminal-profiles.ts:11-13`）。
    name: SharedString,
    /// 要执行的程序。**只写程序名或用户配置的绝对路径**，不要在代码里写机器路径。
    program: PathBuf,
    /// 程序参数（本模块默认给空：真机的"系统默认"profile 不带参数，
    /// `terminal-profiles.ts:19-23` 的 `SYSTEM_DEFAULT_PROFILE_ID` 只有 id + name）。
    args: Vec<String>,
    /// 工作目录；`None` = 继承宿主进程的当前目录。
    ///
    /// 真机的终端默认落在工作区根（`terminal-profiles.ts:78` 的 `workingDirectory`），
    /// 本模块的 `new(window, cx)` 拿不到工作区根，所以留 `None` 作为接线点：
    /// 外壳接好之后把工作区根填进来即可。
    working_directory: Option<PathBuf>,
}

impl TerminalProfile {
    /// 一个用程序名构造的 profile（展示名就是程序名）。
    ///
    /// 字段私有 + `#[non_exhaustive]`（《编码指南》「公共 API 设计」）：跨 crate 之后
    /// 结构体字面量不再合法，新增字段也不会破坏调用方。需要更多参数时用
    /// [`TerminalProfile::with_args`] / [`TerminalProfile::with_working_directory`]
    /// （non-boolean builder 用 `with_` 前缀，《编码指南》词汇表）。
    pub fn new(program: impl Into<PathBuf>) -> Self {
        let program = program.into();
        Self {
            name: SharedString::from(program.to_string_lossy().to_string()),
            program,
            args: Vec::new(),
            working_directory: None,
        }
    }

    /// 附加程序参数（non-boolean builder）。
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// 指定工作目录（non-boolean builder）。
    pub fn with_working_directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(directory.into());
        self
    }

    /// 展示名（页签标题用）。
    pub fn name(&self) -> &SharedString {
        &self.name
    }

    /// 要执行的程序（crate 内部：`Session::start` 用）。
    pub(crate) fn program(&self) -> &PathBuf {
        &self.program
    }

    /// 程序参数（crate 内部）。
    pub(crate) fn args(&self) -> &[String] {
        &self.args
    }

    /// 工作目录（crate 内部）。
    pub(crate) fn working_directory(&self) -> Option<&PathBuf> {
        self.working_directory.as_ref()
    }
}

/// 运行时探测一个程序在不在 `PATH` 上（**不用 `#[cfg]`**）。
///
/// 先试 `where.exe`（Windows），它不存在时（`Err(NotFound)`）再试 `which`（POSIX）；
/// 两者都拿不到就返回 `false`。这样"Windows 优先 powershell、回退 cmd"这句话
/// 就是运行期的一条判断，而不是编译期的平台分支（`gpui/UI-MAP.md` §1.1 第 5 条）。
fn command_exists(program: &str) -> bool {
    for probe in ["where.exe", "which"] {
        let Ok(status) = Command::new(probe)
            .arg(program)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        else {
            continue;
        };
        if status.success() {
            return true;
        }
    }
    false
}

/// 本机可用的终端配置文件：Windows 优先 `powershell`，回退 `cmd`。
///
/// 两个都探不到时返回空表，[`TerminalPane::new`] 会把页签置成失败态并显示
/// [`ERROR_FALLBACK`]（真机在探测不到 shell 时也显示「暂未检测到」，
/// `terminal.shellUnavailable`，`locale.ts:7542`）。
pub(crate) fn profiles() -> Vec<TerminalProfile> {
    detected_profiles().to_vec()
}

/// 默认配置文件 = 探测到的第一个（Windows 优先 `powershell`）。
///
/// 缓存探测结果：`profiles()` 与 `default_profile()` 是同一条规则的两种问法，
/// 不该因此多敲一次 `where.exe`。
pub(crate) fn default_profile() -> Option<TerminalProfile> {
    detected_profiles().first().cloned()
}

/// 真正做探测的地方（只跑一次，结果进 [`OnceLock`]）。
fn detected_profiles() -> &'static [TerminalProfile] {
    static DETECTED: OnceLock<Vec<TerminalProfile>> = OnceLock::new();
    DETECTED.get_or_init(|| {
        let mut detected = Vec::new();
        for program in ["powershell", "cmd"] {
            if command_exists(program) {
                detected.push(TerminalProfile::new(program));
            }
        }
        detected
    })
}

// ---------------------------------------------------------------------------
// 最小 ANSI 清洗 + 行缓冲
// ---------------------------------------------------------------------------
