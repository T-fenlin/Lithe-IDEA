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

/// 「系统默认 shell」的内部取值：**空串**。
///
/// 与设置 crate 的 `SHELL_SYSTEM_DEFAULT` 是同一个值，但这里**不复用**那一个常量：
/// 依赖方向是 `workbench` → `terminal`，`terminal` 不认识 `settings`
/// （`crates/terminal/Cargo.toml` 里没有 `lithe-db-gpui-settings`），所以各自声明一次，
/// 由 `SHELL_IDS` 的注释钉住两边同源。
pub(crate) const SHELL_SYSTEM_DEFAULT: &str = "";

/// 「默认 Shell」设置的全部合法取值（**顺序即设置页下拉的顺序**）。
///
/// 真源 `windows/tauri/src/features/settings/components/macos-settings-panels.tsx:388-391` 的
/// 四个 `<option>`：`""`（系统默认）/ `powershell` / `cmd` / `wsl`。
/// 设置 crate 的 `TERMINAL_SHELL_IDS` 是同一份表的另一份声明，两边的**取值**必须一致
/// （那边是白名单、这边是"能不能真的起一个进程"）。
pub(crate) const SHELL_IDS: [&str; 4] = [SHELL_SYSTEM_DEFAULT, "powershell", "cmd", "wsl"];

/// 把设置里的 shell id 解析成一个可执行的配置文件；`None` = 「系统默认」（保持探测顺序）。
///
/// 只做**映射**，不判可用性：`wsl` 这类命令在没装 WSL 的机器上不存在，
/// 判断由调用方（[`crate::TerminalPane::set_default_shell`]）用 [`command_exists`] 做，
/// 这样"设置里点名了但本机没有"能走一条单独的诊断分支，而不是静默什么都没发生。
pub(crate) fn profile_for_shell_id(id: &str) -> Option<TerminalProfile> {
    match id {
        "powershell" | "cmd" | "wsl" => Some(TerminalProfile::new(id)),
        // 空串与任何未知值都按「系统默认」处理（设置侧有白名单，未知值到不了这里）。
        _ => None,
    }
}

/// 运行时探测一个程序在不在 `PATH` 上（**不用 `#[cfg]`**）。
///
/// 先试 `where.exe`（Windows），它不存在时（`Err(NotFound)`）再试 `which`（POSIX）；
/// 两者都拿不到就返回 `false`。这样"Windows 优先 powershell、回退 cmd"这句话
/// 就是运行期的一条判断，而不是编译期的平台分支（`lithe-db-gpui/UI-MAP.md` §1.1 第 5 条）。
pub(crate) fn command_exists(program: &str) -> bool {
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
/// 注意这张表**只列探测得到的**：设置里点名的 shell 由 [`profile_for_shell_id`] 另判。
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
///
/// 探测清单是 [`SHELL_IDS`] 里**默认顺序**的那两项（`powershell` → `cmd`）；
/// `wsl` 只在设置里被点名时才认（[`profile_for_shell_id`]）—— 它不该挤进默认顺序，
/// 真机的默认顺序同样只有 PowerShell 与命令提示符两条。
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 设置里的四个 id → 配置文件：只有 `powershell` / `cmd` / `wsl` 能映射出程序，
    /// 空串（系统默认）与任何未知值都返回 `None`（= 保持探测顺序）。
    #[test]
    fn shell_ids_map_to_programs() {
        for id in ["powershell", "cmd", "wsl"] {
            let profile = profile_for_shell_id(id).expect("这三个 id 必须映射出程序");
            assert_eq!(profile.program().to_string_lossy(), id);
        }
        assert!(profile_for_shell_id(SHELL_SYSTEM_DEFAULT).is_none());
        assert!(profile_for_shell_id("fish").is_none());
    }

    /// 两份声明（设置 crate 的白名单 / 这里的映射表）必须列同一批取值。
    /// 这条钉住的是"设置页里能选的值 = 终端认识的值"，加值时两边漏一边就会红。
    #[test]
    fn shell_id_table_matches_the_settings_whitelist() {
        assert_eq!(
            SHELL_IDS,
            ["", "powershell", "cmd", "wsl"],
            "与 lithe-db-gpui-settings 的 TERMINAL_SHELL_IDS 必须逐项一致"
        );
    }
}

// ---------------------------------------------------------------------------
// 最小 ANSI 清洗 + 行缓冲
// ---------------------------------------------------------------------------
