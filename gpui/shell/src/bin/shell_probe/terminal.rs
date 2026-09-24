//! 底部工具窗 · 终端（**只做外壳，不做 VT 模拟**）。
//!
//! 对应 Windows 前端的 `TerminalContainer` → `TerminalTabBar` → `XtermTerminal`
//! （`windows/tauri/src/features/terminal/components/terminal-container.tsx:36,697-721`）。
//!
//! ## 本模块的范围（维护者 2026-09-25 拍板，不再扩大）
//!
//! 只做四件事：**工具窗页签（新建 / 关闭 / 标题）+ 流式显示 stdout/stderr + 一个发送命令的输入行
//! + 最小 ANSI 清洗**。终端进程由本模块直接在 Rust 宿主里用 `std::process::Command` 起
//! （**不是** Windows 那套 ConPTY + `#[tauri::command]` 协议，见下面「与 Windows 的差异」）。
//!
//! ## ⚠️ 能力边界（**必须让用户看得见**，也在界面上以 [`CAPABILITY_NOTICE`] 常驻显示）
//!
//! - **不能运行 `vim` / `top` / `less` 这类全屏 TUI 程序**：没有 VT 模拟（无光标定位、无备用屏幕、
//!   无滚动区域），全屏程序画出来的字符网格只会被当成普通输出流铺开，画面不可读。
//! - **`Ctrl+C` 不可用**：本模块没有 PTY / 控制台输入通道，无法把 `CTRL_C_EVENT`（Windows）或
//!   `SIGINT`（POSIX）送进子进程；正在运行的命令只能靠**关闭页签**（`Child::kill`）中止。
//! - 只有"输出查看器"级别的显示：丢弃 SGR 之后**没有任何颜色**、没有选区、没有查找、没有链接、
//!   没有光标、没有鼠标输入（`terminal.tsx:267-289` 里 xterm.js 的那些能力全部缺失）。
//! - 终端**列宽 / 行高模型不存在**：长行按视口宽度软换行，而不是按终端的列数换行
//!   （真机是 xterm.js 的字符网格，`terminal.tsx:632-651` 还会把尺寸同步给 ConPTY）。
//!
//! ## 与 Windows 前端的差异（逐条写清，避免把这里当规格真源）
//!
//! | 项 | Windows 前端 | 本模块 |
//! | --- | --- | --- |
//! | 进程 | Rust host 用 `portable-pty` 0.9 开 **ConPTY**（`windows/tauri/crates/terminal/Cargo.toml:11`、`connection.rs:36-49`） | `std::process::Command` + 三个管道，**没有 PTY** |
//! | 通道 | 8 个 command + `ipc::Channel<TerminalEvent>`，`data: number[]`（`src-tauri/src/terminal.rs:85-181`） | 进程内 `std::sync::mpsc` + 读线程 |
//! | 渲染 | xterm.js 6 Canvas/WebGL（`terminal.tsx:267-289`） | 纯文本行 + 最小 ANSI 清洗（见 [`TerminalText`]） |
//! | 输入 | 直接打进 xterm（`use-terminal-connection.ts:145-146`） | 底部单行输入框，**回车**发送一行（回车 = `\r\n`） |
//! | 回显 | ConPTY 回显 | **靠子进程自己回显**：实测 `cmd.exe` 与 `powershell.exe` 在管道下都会打印提示符与命令回显，所以本模块**不做本地回显**（见文件末尾「实测」） |
//! | 编码 | host **零 ANSI 清洗**，可读性 100% 靠 xterm.js | 本模块做 ANSI 清洗（见下） |
//!
//! ## 规格出处（每个数值/文案都能查到来源）
//!
//! - 页签条高 **36**、单页签 min-w **80** / max-w **200**、页签文字 **13px**、关闭按钮 **24×24**：
//!   `gpui/UI-MAP-WINDOWS.md` §1.6（引自 `windows/tauri/src/ui/tab-bar.tsx:244-267`、
//!   `features/terminal/components/terminal-tab-bar-item.tsx:102-107`）。
//! - 终端内容左内边距 **16**（`pl-4`，`features/terminal/components/terminal.tsx:870`）：同上 §1.6。
//! - 终端字号 **14** / 行高 **1**：`features/settings/config/default-settings.ts:76-77`
//!   （`terminalFontSize: DEFAULT_CODE_FONT_SIZE`，值为 14 见 `config/typography-defaults.ts:13`）。
//! - 输出保留上限 **10000 行**：真机默认 `terminalScrollback: 10000`
//!   （`features/settings/config/default-settings.ts:79`）。
//! - 状态文案「运行中 / 成功 / 失败」：`i18n/locale.ts:5930-5932`（`run.running/succeeded/failed`）；
//!   「退出码」`i18n/locale.ts:4568`（`git.console.exit`）；「终端错误 / 无法初始化终端 / 重试」：
//!   `i18n/locale.ts:7549-7551`；「没有终端 / 新建终端 / 关闭 {name} / 清除终端 / 选择终端配置文件」：
//!   `i18n/locale.ts:7544,7531,7519,7510,7543`；输入行占位「输入命令...」：
//!   `i18n/locale.ts:7905`（`commandPalette.placeholder`，「滚动到底部」`:4494`）。
//! - 页面/文案不含裸色值：颜色一律 `cx.theme()`（`gpui/UI-MAP.md` §1.1 第 2 条）。
//! - 不做平台分支：`profiles()` 用**运行时探测**（`where.exe` / `which`）决定 powershell → cmd
//!   （`gpui/UI-MAP.md` §1.1 第 5 条）。
//!
//! ## ANSI 清洗的参考实现（Windows 侧，逐条对照）
//!
//! - 丢弃 CSI（含 SGR）序列：`features/run/utils/run-output-style.ts:252-266`（`readCsi`）
//! - 丢弃 OSC 序列（BEL 或 `ESC \` 结束）：同文件 `:268-277`（`skipOsc`）
//! - 退格 `\b`（8）/ DEL（127）删掉一个字符：同文件 `:134-138`
//! - 其余 C0 控制字符（除 `\t` `\n`）丢弃：同文件 `:139-142`
//! - `\r` 回到行首、后续字符覆盖当前行：`features/run/utils/output-timestamper.ts:128-135`
//!   （`overwriteCarriageReturns`："较长的部分胜出"，与"光标覆盖写"在本模块里的实现等价）
//! - 转义序列/多字节字符被 chunk 切断时要跨块续上：同文件 `:202-238`
//!   （`controlSequenceEnd` / `advancePastIncompleteControl`）
//!
//! ## 文件末尾
//!
//! 未实现清单、图标替代表、"设置界面接线点"说明都在文件末尾的注释块里。

use std::cell::RefCell;
// ⚠️ `Read` 必须**按名字**导入（不能 `as _`）：它要出现在 `spawn_reader` 的 trait bound 里，
// 而 `use ... as _` 只让方法解析看得见它、名字本身并不进作用域。`Write` 只用于方法调用，
// 所以用 `as _` 就够。
use std::io::{Read, Write as _};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use gpui_kit::assets::IconName;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyTitle};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::message_scroller::{MessageScroller, MessageScrollerState};
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, Sizable as _, Size,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, ClickEvent, Context, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, Render, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled as _, Subscription, WeakEntity, Window, div, px, relative,
};

// ---------------------------------------------------------------------------
// 文案（除 [`CAPABILITY_NOTICE`] 外全部逐字取 `windows/tauri/src/i18n/locale.ts`）
// ---------------------------------------------------------------------------

/// `terminal.noTerminals` → 「没有终端」（`locale.ts:7544`）：空态条与空态页。
const NO_TERMINALS: &str = "没有终端";
/// `terminal.newTerminal` → 「新建终端」（`locale.ts:7531`）：新建按钮的提示与无障碍名。
const NEW_TERMINAL: &str = "新建终端";
/// `terminal.chooseTerminalProfile` → 「选择终端配置文件」（`locale.ts:7543`）：配置文件下拉。
const CHOOSE_PROFILE: &str = "选择终端配置文件";
/// `terminal.contextClear` → 「清除终端」（`locale.ts:7510`）：真机在页签右键菜单里，本模块做成图标按钮。
const CLEAR_TERMINAL: &str = "清除终端";
/// `terminal.errorTitle` → 「终端错误」（`locale.ts:7549`）。
const ERROR_TITLE: &str = "终端错误";
/// `terminal.errorFallback` → 「无法初始化终端」（`locale.ts:7550`）。
const ERROR_FALLBACK: &str = "无法初始化终端";
/// `terminal.retry` → 「重试」（`locale.ts:7551`）：失败/已退出后的重新拉起。
const RETRY: &str = "重试";
/// `run.running` → 「运行中」（`locale.ts:5930`）。
const RUNNING: &str = "运行中";
/// `run.succeeded` → 「成功」（`locale.ts:5931`）。
const SUCCEEDED: &str = "成功";
/// `run.failed` → 「失败」（`locale.ts:5932`）。
const FAILED: &str = "失败";
/// `git.console.exit` → 「退出码」（`locale.ts:4568`）：真机在 Git 控制台里这样拼
/// `退出码 {code}`（`features/git/components/log/git-console-entry.tsx:82-84`）。
const EXIT_CODE: &str = "退出码";
/// `commandPalette.placeholder` → 「输入命令...」（`locale.ts:7905`）：命令输入框的占位。
///
/// 终端真机没有"输入行"（直接打进 xterm，`use-terminal-connection.ts:145-146`），
/// 所以这一条借的是命令面板的原文，而不是终端自己的键。
const INPUT_PLACEHOLDER: &str = "输入命令...";
/// `git.console.scrollToEnd` → 「滚动到底部」（`locale.ts:4494`）：输出区"跳回最新"的提示。
const SCROLL_TO_END: &str = "滚动到底部";
/// `terminal.terminals` → 「终端」（`locale.ts:7558`）：输出区的无障碍标签。
const TERMINALS_ARIA: &str = "终端";

/// 能力边界声明（**常驻显示**，不是可选提示）。
///
/// 本模块只做"输出查看器"外壳：没有 VT 模拟 → 全屏程序（`vim` / `top` / `less`）画不出画面；
/// 没有 PTY/控制台输入通道 → `Ctrl+C` 送不进子进程，命令无法中断。
///
/// ⚠️ `locale.ts` 的 `terminal.*` 全量（`:7503-7558`）里**没有**描述这两条限制的键，
/// 所以这一句是本模块唯一**不是**逐字取自 locale 的文案；原因与取舍见文件末尾未实现清单第 1 条。
const CAPABILITY_NOTICE: &str = "能力边界：不能运行 vim/top 等全屏程序；Ctrl+C 不可用";

// ---------------------------------------------------------------------------
// 度量（全部有出处；`px()` 直搬，`gpui/UI-MAP.md` §1.1 第 1 条）
// ---------------------------------------------------------------------------

/// 页签条高 36（`UI-MAP-WINDOWS.md` §1.6；`ui/tab-bar.tsx:244-255`）。
const TAB_BAR_HEIGHT: f32 = 36.;
/// 单个页签最小宽 80 / 最大宽 200（同上，`ui/tab-bar.tsx:257-267`）。
const TAB_MIN_WIDTH: f32 = 80.;
const TAB_MAX_WIDTH: f32 = 200.;
/// 页签文字 13px（`ui/tab-bar.tsx:170` 的 `ui-text-chrome`）。
const TAB_TEXT_SIZE: f32 = 13.;
/// 页签关闭按钮 24×24（`ui/button.tsx:27` 的 `icon-xs`）。
const TAB_CLOSE_BUTTON_SIZE: f32 = 24.;
/// chrome 圆角 4px（`styles/theme.css:133` 的 `--lithe-chrome-radius: 4px`）。
const CHROME_RADIUS: f32 = 4.;
/// 状态/边界提示行高 24px（`styles/theme.css:126` 的 `--lithe-chrome-control-height: 1.5rem`）。
const STATUS_LINE_HEIGHT: f32 = 24.;
/// 提示文字 12px（`styles/theme.css:114` 的 `--ui-text-caption: 12px`）。
const NOTICE_TEXT_SIZE: f32 = 12.;
/// 终端字号 14（`config/default-settings.ts:76` + `config/typography-defaults.ts:13`）。
const TERMINAL_FONT_SIZE: f32 = 14.;
/// 终端行高倍数 1（`config/default-settings.ts:77` 的 `terminalLineHeight: 1`）。
const TERMINAL_LINE_HEIGHT: f32 = 1.;
/// 终端内容左右内边距 16（`pl-4`，`features/terminal/components/terminal.tsx:870`）。
const TERMINAL_PADDING_INLINE: f32 = 16.;
/// 输入行：外框上下 6 / 左右 12、行内间距 6、输入框高 28、输入字号 12
/// （运行面板 stdin 行 `flex items-center gap-1.5 border-t px-3 py-1.5` + `h-7 font-mono text-[12px]`，
/// `features/run/components/run-pane.tsx:454,462`；`UI-MAP-WINDOWS.md` §3.5）。
const INPUT_ROW_PADDING_INLINE: f32 = 12.;
const INPUT_ROW_PADDING_BLOCK: f32 = 6.;
const INPUT_GAP: f32 = 6.;
const INPUT_HEIGHT: f32 = 28.;
const INPUT_TEXT_SIZE: f32 = 12.;
/// 输出保留上限 10000 行（真机默认 `terminalScrollback: 10000`，`config/default-settings.ts:79`）。
const MAX_LINES: usize = 10_000;
/// 管道单次读取缓冲 4 KiB（与真机 host 的读缓冲同量级：`crates/terminal/src/connection.rs:453-512`
/// 用 64 KiB；这里按行缓冲、上限 10000 行，4 KiB 足够）。
const READ_BUFFER: usize = 4 * 1024;
/// 两条读线程都 EOF 之后，用 `try_wait` 收退出码的轮询次数与间隔。
///
/// `cmd.exe` / `powershell.exe` 在管道下都会在退出前关掉两条管道，所以正常情况第一次
/// `try_wait` 就能拿到状态；这两次等待只是为了极端的时序（子进程已关管道但还没退出）。
const EXIT_POLL_ATTEMPTS: usize = 10;
const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(20);

// ---------------------------------------------------------------------------
// 终端配置文件（**"自定义 shell"就在这里接线**）
// ---------------------------------------------------------------------------

/// 一个终端配置文件：起哪个程序、带什么参数、在哪个目录。
///
/// 字段照 Windows 的 `ResolvedTerminalLaunch`（`features/terminal/utils/terminal-profiles.ts:9-15`：
/// `shell / workingDirectory / initialCommand / name / profileId`）裁剪，只留本外壳用得到的三个。
///
/// ## 🔌 设置界面的接线点在这里
///
/// 1. 用户在设置里改「终端默认配置文件 / shell 路径 / 参数 / 启动目录」→ 调
///    [`TerminalPane::add_profile`] 或 [`TerminalPane::replace_profiles`] 把新的
///    [`TerminalProfile`] 送进来（Windows 侧的存储是 `features/terminal/stores/profiles.store.ts`
///    与 `settings.terminalDefaultProfileId`，`terminal-profiles.ts:56-88` 做优先级解析）；
/// 2. 默认配置文件由 [`default_profile`] 给：Windows 优先 `powershell`、回退 `cmd`，
///    **用运行时探测**决定（`where.exe` 失败再试 `which`，不写 `#[cfg(target_os)]`）；
/// 3. 页签条右端的 ⌄ 菜单（[`TerminalPane::render_tab_bar`]）就是"选择终端配置文件"的落点，
///    对应真机 `terminal-tab-bar.tsx:442-451,557-565`；
/// 4. 真机的新配置文件可以带 `startupCommands`（`terminal-profiles.ts:77`）；本模块**没有**初始
///    命令通路（没有 PTY、也没有"先注入一行"的落点），要接的话就在这里加字段并在
///    `Session::start` 之后往 stdin 写一行。
#[derive(Clone)]
pub struct TerminalProfile {
    /// 展示名（真机取 `TerminalProfile.name`，`terminal-profiles.ts:11-13`）。
    pub name: SharedString,
    /// 要执行的程序。**只写程序名或用户配置的绝对路径**，不要在代码里写机器路径。
    pub program: PathBuf,
    /// 程序参数（本模块默认给空：真机的"系统默认"profile 不带参数，
    /// `terminal-profiles.ts:19-23` 的 `SYSTEM_DEFAULT_PROFILE_ID` 只有 id + name）。
    pub args: Vec<String>,
    /// 工作目录；`None` = 继承宿主进程的当前目录。
    ///
    /// 真机的终端默认落在工作区根（`terminal-profiles.ts:78` 的 `workingDirectory`），
    /// 本模块的 `new(window, cx)` 拿不到工作区根，所以留 `None` 作为接线点：
    /// 外壳接好之后把工作区根填进来即可。
    pub working_directory: Option<PathBuf>,
}

impl TerminalProfile {
    /// 一个用程序名构造的 profile（名字就是程序名）。
    fn from_program(program: &str) -> Self {
        Self {
            name: SharedString::from(program.to_string()),
            program: PathBuf::from(program),
            args: Vec::new(),
            working_directory: None,
        }
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
pub fn profiles() -> Vec<TerminalProfile> {
    detected_profiles().to_vec()
}

/// 默认配置文件 = 探测到的第一个（Windows 优先 `powershell`）。
///
/// 缓存探测结果：`profiles()` 与 `default_profile()` 是同一条规则的两种问法，
/// 不该因此多敲一次 `where.exe`。
pub fn default_profile() -> Option<TerminalProfile> {
    detected_profiles().first().cloned()
}

/// 真正做探测的地方（只跑一次，结果进 [`OnceLock`]）。
fn detected_profiles() -> &'static [TerminalProfile] {
    static DETECTED: OnceLock<Vec<TerminalProfile>> = OnceLock::new();
    DETECTED.get_or_init(|| {
        let mut detected = Vec::new();
        for program in ["powershell", "cmd"] {
            if command_exists(program) {
                detected.push(TerminalProfile::from_program(program));
            }
        }
        detected
    })
}

// ---------------------------------------------------------------------------
// 最小 ANSI 清洗 + 行缓冲
// ---------------------------------------------------------------------------

/// 转义序列状态机的位置（跨 chunk 保留，见 `output-timestamper.ts:202-238`）。
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum EscapeState {
    /// 普通文本。
    #[default]
    None,
    /// 刚读到 `ESC`，还不知道是 CSI / OSC / 其它两字符序列。
    Esc,
    /// `ESC [ …`（CSI）：吃到 final byte（`0x40..=0x7E`）为止。
    Csi,
    /// `ESC ] …`（OSC）：吃到 BEL 或 `ESC \` 为止。
    Osc,
    /// OSC 里读到了 `ESC`，等一个字符判断是不是 `\`（ST）。
    OscEsc,
}

/// 一个终端的输出文本缓冲：**清洗 + 按行组织**。
///
/// 与真机的差别：真机把字节原样喂给 xterm.js（Rust host 零清洗，
/// `windows/tauri/src-tauri/src/terminal.rs`），由 xterm 解释 VT；本模块没有 VT，
/// 所以在这里把控制序列**丢弃**、把 `\r` `\b` 落实成字符级效果，剩下的就是可读的文本行。
///
/// 光标模型：`current` 是"当前行"的字符数组，`cursor` 是字符下标。`\r` 把 `cursor` 归零，
/// 之后写入的字符**覆盖**原位置——这与真机 `overwriteCarriageReturns`
/// （`output-timestamper.ts:128-135`）的"逐段取较长者"在进度条这类场景下等价，
/// 但更接近"回车覆盖当前行"的字面语义。
#[derive(Default)]
struct TerminalText {
    /// 本次 `feed` 里**刚完成**的行（每次取走，不长期堆积）。
    completed: Vec<String>,
    /// 当前还没换行的那一行。
    current: Vec<char>,
    /// 当前行的光标（字符下标）。
    cursor: usize,
    /// 转义序列状态（跨 chunk）。
    escape: EscapeState,
    /// 跨 chunk 的半个多字节字符。
    pending_bytes: Vec<u8>,
    /// 累计被 `from_utf8_lossy` 替换掉的字节数（非 UTF-8 输出的诊断依据）。
    replaced_bytes: usize,
}

impl TerminalText {
    /// 喂一段原始字节（stdout / stderr 都走这里）。
    ///
    /// UTF-8 处理：合法的多字节字符被 chunk 切断时，尾部**留到下一块**（不产生假 `U+FFFD`）；
    /// 真正的非法序列（例如 Windows 控制台的 **GBK/CP936** 输出）按 `String::from_utf8_lossy`
    /// 兜底，替换成 `U+FFFD` 并计入 [`TerminalText::replaced_bytes`]。
    ///
    /// **已知限制**：`cmd.exe` 默认代码页是 936 时，中文输出会是满屏 `�`。真机靠
    /// ConPTY + xterm 的 UTF-8 解码规避（`windows/tauri/src-tauri/src/run.rs:1864-1903`
    /// 甚至按 ANSI 代码页解码），本模块没有"取当前代码页"的依赖，故不做转换。
    fn push_bytes(&mut self, bytes: &[u8]) {
        self.pending_bytes.extend_from_slice(bytes);
        loop {
            let pending = std::mem::take(&mut self.pending_bytes);
            match std::str::from_utf8(&pending) {
                Ok(text) => {
                    let owned = text.to_string();
                    self.push_str(&owned);
                    break;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    if valid > 0 {
                        let head = String::from_utf8_lossy(&pending[..valid]).into_owned();
                        self.push_str(&head);
                    }
                    match error.error_len() {
                        // 真正的非法序列：按 lossy 兜底，消耗掉这一段继续。
                        Some(len) => {
                            self.replaced_bytes += len;
                            self.push_str("\u{FFFD}");
                            self.pending_bytes = pending[valid + len..].to_vec();
                        }
                        // 尾部是"未完成的多字节字符"：留到下一块再拼。
                        None => {
                            self.pending_bytes = pending[valid..].to_vec();
                            break;
                        }
                    }
                }
            }
        }
    }

    /// 取走本次新完成的行。
    fn take_completed(&mut self) -> Vec<String> {
        std::mem::take(&mut self.completed)
    }

    /// 当前未完成行的文本（为空则 `None`）——它贴在输出区底部显示。
    fn current_text(&self) -> Option<SharedString> {
        if self.current.is_empty() {
            None
        } else {
            Some(SharedString::from(self.current.iter().collect::<String>()))
        }
    }

    /// 清空（「清除终端」）。
    fn clear(&mut self) {
        self.completed.clear();
        self.current.clear();
        self.cursor = 0;
    }

    /// 清洗后的字符流状态机。
    fn push_str(&mut self, text: &str) {
        for ch in text.chars() {
            match self.escape {
                EscapeState::None => match ch {
                    '\u{1b}' => self.escape = EscapeState::Esc,
                    // 回车：回到行首，后续字符覆盖当前行。
                    '\r' => self.cursor = 0,
                    '\n' => self.end_line(),
                    // 退格 / DEL：删掉光标前一个字符（run-output-style.ts:134-138）。
                    '\u{8}' | '\u{7f}' => self.backspace(),
                    '\t' => self.put(ch),
                    // 其余 C0（以及 C1 区以外的）控制字符一律丢弃（run-output-style.ts:139-142）。
                    c if (c as u32) < 0x20 => {}
                    c => self.put(c),
                },
                // ESC 引导：CSI 是 `[`、OSC 是 `]`，其余两字符序列（如 `ESC ( B`、`ESC 7`）整体丢弃。
                EscapeState::Esc => {
                    self.escape = match ch {
                        '[' => EscapeState::Csi,
                        ']' => EscapeState::Osc,
                        _ => EscapeState::None,
                    };
                }
                // CSI：参数 0x30-0x3F、中间字节 0x20-0x2F，final byte 0x40-0x7E 结束
                // （readCsi，run-output-style.ts:252-266）。SGR（`…m`）与光标移动（`…H`）都被丢掉。
                EscapeState::Csi => {
                    let code = ch as u32;
                    if (0x40..=0x7e).contains(&code) || code > 0x7f {
                        self.escape = EscapeState::None;
                    }
                }
                // OSC：BEL 或 ST（`ESC \`）结束（skipOsc，run-output-style.ts:268-277）。
                EscapeState::Osc => {
                    if ch == '\u{7}' {
                        self.escape = EscapeState::None;
                    } else if ch == '\u{1b}' {
                        self.escape = EscapeState::OscEsc;
                    }
                }
                EscapeState::OscEsc => {
                    self.escape = if ch == '\\' {
                        EscapeState::None
                    } else {
                        EscapeState::Osc
                    };
                }
            }
        }
    }

    /// 在光标处写入一个字符（覆盖或追加），光标前进一格。
    fn put(&mut self, ch: char) {
        if self.cursor < self.current.len() {
            self.current[self.cursor] = ch;
        } else {
            self.current.push(ch);
        }
        self.cursor += 1;
    }

    /// 退格：删掉光标前一个字符并把光标前移。
    fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.current.remove(self.cursor);
        }
    }

    /// 换行：当前行封口。
    fn end_line(&mut self) {
        self.completed.push(self.current.iter().collect());
        self.current.clear();
        self.cursor = 0;
    }
}

// ---------------------------------------------------------------------------
// 子进程会话
// ---------------------------------------------------------------------------

/// 会话状态（真机的三档：运行中 / 已退出（带退出码）/ 起不来）。
enum SessionState {
    Running,
    /// 已退出（退出码）。
    Exited(i32),
    /// 起不来（`Command::spawn` 失败或 shell 探测为空）。存的是操作系统错误原文。
    Failed(String),
}

/// 一条读线程 / 写线程发给 UI 的事件。
enum SessionEvent {
    /// 一段原始输出字节（stdout / stderr 各一条线程，混在一条通道里）。
    Output(Vec<u8>),
    /// 某条读线程读到 EOF。
    StreamClosed,
    /// 某条读线程读失败（`io::Error` 原文，进诊断）。
    StreamError(String),
}

/// 泵的下一步动作（决定要不要等退出码、要不要收工）。
enum PumpStep {
    Continue,
    /// 两条管道都 EOF 了 → 去收退出码。
    StreamsClosed,
    /// 实体已经没了 → 收工。
    Stop,
}

/// 一条终端会话：子进程 + 写线程句柄 + 已清洗的输出文本。
struct Session {
    /// 子进程。**一直留在这里**，`Drop` / 关页签时才有 `kill()` + `wait()` 的能力。
    child: Option<Child>,
    /// stdin 写入通道（写线程持有真正的 `ChildStdin`；这里的 `send` 永不阻塞 UI）。
    writer: Option<Sender<Vec<u8>>>,
    state: SessionState,
    text: TerminalText,
    /// 已经 EOF 的读线程数（到 2 就说明两条管道都关了）。
    closed_streams: usize,
    /// 读线程报回来的 IO 错误（有则显示在诊断行里）。
    io_error: Option<String>,
    /// 子进程 pid（诊断行）。
    pid: Option<u32>,
}

impl Session {
    /// 起一个进程，并把它的 stdout / stderr / stdin 都接上管道。
    ///
    /// 起不来（`spawn` 失败）返回 `Err(系统错误原文)`；调用方把页签置成失败态
    /// （真机的失败态是 `TerminalErrorBoundary` + `terminal.errorTitle`，`terminal-error-boundary.tsx:40-51`）。
    fn start(profile: &TerminalProfile) -> Result<(Self, Receiver<SessionEvent>), String> {
        let mut command = Command::new(&profile.program);
        command
            .args(&profile.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(directory) = &profile.working_directory {
            command.current_dir(directory);
        }

        let mut child = command
            .spawn()
            .map_err(|error| format!("{}: {error}", profile.program.display()))?;
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
    fn failed(message: String) -> Self {
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

    fn is_running(&self) -> bool {
        matches!(self.state, SessionState::Running)
    }

    /// 收摊：先关 stdin（让 shell 自己收尾），再 `kill` + `wait`。
    ///
    /// **`wait()` 不能省**：只 `kill()` 不回收会在进程表里留僵尸进程。
    /// 这是"必须能关进程"的落点（真机窗口销毁时也会杀全部 PTY，
    /// `windows/tauri/crates/terminal/src/manager.rs:99-113`）。
    fn shutdown(&mut self) {
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
    fn settle_exit(&mut self) -> bool {
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
fn spawn_reader<R: Read + Send + 'static>(mut reader: R, events: Sender<SessionEvent>) {
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
fn spawn_writer(mut stdin: ChildStdin, requests: Receiver<Vec<u8>>) {
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
fn blocking_recv(receiver: &Arc<Mutex<Receiver<SessionEvent>>>) -> Option<SessionEvent> {
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
struct TabSession {
    /// 稳定 id（页签数组下标会随关闭变动，泵任务按 id 找页签）。
    id: u64,
    /// 本次运行的**代次**：重试会 +1，旧泵的残留事件靠它丢弃。
    ///
    /// 真机用同一套路做过期保护：`run.rs` 用 `execution_id` 校验"旧适配器不得回收新 Run"
    /// （`windows/tauri/src-tauri/src/run.rs:2016-2045`，前端 `run.store.ts:1039-1067`）。
    run: u64,
    title: SharedString,
    /// 起这个页签时用的配置文件（重试时复用）。
    profile: TerminalProfile,
    session: Session,
    /// 输出区的贴底跟随状态（每个页签一份：`MessageScrollerState` 自带 `FollowMode::Tail`，
    /// `gpui-component-0.6.6/src/message_scroller.rs:38,65`）。
    scroller: Entity<MessageScrollerState>,
    /// 已完成行（输出区显示的内容）。`RefCell` 是为了让行渲染闭包按需读，
    /// 避免每个 chunk 都克隆整个 Vec。
    rows: Rc<RefCell<Vec<SharedString>>>,
}

/// 终端面板（底部工具窗的"终端"页）。
///
/// 对外只暴露 [`TerminalPane::new`]（主代理按这个签名接线）与几个公开方法；字段全私有。
pub struct TerminalPane {
    tabs: Vec<TabSession>,
    /// 当前活动页签的下标（空表时无意义）。
    active: usize,
    next_id: u64,
    /// 下一个运行代次（见 [`TabSession::run`]）。
    next_run: u64,
    /// 可选配置文件（[`profiles`] 或 [`TerminalPane::replace_profiles`] 灌进来的）。
    profiles: Vec<TerminalProfile>,
    /// 新建页签用哪个配置文件。
    active_profile: usize,
    /// 底部命令输入行。
    input: Entity<InputState>,
    /// 输入行的事件订阅（`PressEnter` → 发送一行）。订阅器一 drop 就失效，所以要存住。
    _input_events: Subscription,
}

impl TerminalPane {
    /// 起一个默认 shell（Windows 优先 `powershell`，回退 `cmd`）的终端面板，并开第一个页签。
    ///
    /// 探测不到任何 shell 时也会建一个页签，但它是失败态并显示 [`ERROR_FALLBACK`] +
    /// 「重试」（真机的失败态文案见 `i18n/locale.ts:7549-7551`）。
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let profiles = profiles();
        let input = cx.new(|cx| InputState::new(window, cx));
        input.update(cx, |state, cx| {
            state.set_placeholder(INPUT_PLACEHOLDER, window, cx);
        });

        // `subscribe_in`（而不是 `subscribe`）：发送之后要用 `window` 清空输入框，
        // 而 `InputState::set_value` 需要 `&mut Window`（`gpui-base-0.6.6/src/input/base/state.rs:897`）。
        let input_events = cx.subscribe_in(
            &input,
            window,
            |this: &mut Self, _input, event: &InputEvent, window: &mut Window, cx: &mut Context<Self>| {
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
    fn open_tab(&mut self, cx: &mut Context<Self>) -> bool {
        // 列表为空时回落到 `default_profile()`：设置界面可能把列表清空过，而"默认 shell"
        // 永远是同一条规则（Windows 优先 powershell、回退 cmd）。
        let profile = self
            .profiles
            .get(self.active_profile)
            .cloned()
            .or_else(default_profile);
        let title = match &profile {
            Some(profile) => profile.name.clone(),
            None => SharedString::from(ERROR_FALLBACK),
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
                    Session::failed(ERROR_FALLBACK.to_string()),
                    mpsc::channel::<SessionEvent>().1,
                )
            }
        };

        if let (SessionState::Running, Some(pid)) = (&session.state, session.pid) {
            println!(
                "S1_TERMINAL_TAB id={id} profile={} pid={pid}",
                title
            );
        }

        let scroller = cx.new(|cx| MessageScrollerState::new(0, cx));
        self.tabs.push(TabSession {
            id,
            run,
            title,
            profile: profile.unwrap_or_else(|| TerminalProfile::from_program("cmd")),
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
    fn close_tab(&mut self, id: u64, cx: &mut Context<Self>) {
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
    fn retry_tab(&mut self, id: u64, cx: &mut Context<Self>) {
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
    fn clear_active_output(&mut self, cx: &mut Context<Self>) {
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
    fn submit_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
    fn spawn_pump(
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
    fn handle_event(
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
    fn append_rows(&mut self, index: usize, fresh: Vec<String>, cx: &mut Context<Self>) {
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
    fn settle_exit(&mut self, id: u64, run: u64, cx: &mut Context<Self>) -> bool {
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

    // ---- 渲染 ----
    //
    // ⚠️ 只读的渲染辅助函数一律返回**具体类型**（`AnyElement` / `Button`），并且只借
    // `&self` + `&mut Context<Self>`：edition 2024 下 `-> impl IntoElement` 会把作用域里的
    // 生命周期捕进 opaque 类型，同一个表达式连续调用两次就报 E0499（仓库已踩过）。

    /// 页签条（高 36）。
    ///
    /// 用 gpui-kit 的 `TabBar` + `Tab`（`gpui/UI-MAP-WINDOWS.md` §2⑤「终端页签行」判定为可一比一）：
    /// `TabVariant::Underline` = 活动页签下方一条主色指示条（真机是同语义的 3px 强调条，
    /// `ui/tab-bar.tsx:206-210`）。关闭按钮真机的 `Tab` 没有内建（`UI-MAP-WINDOWS.md` §2⑤），
    /// 所以按建议用 `Tab::suffix(Button)` 自己塞一个。
    ///
    /// 没有页签时退化成真机的"空态条"：终端图标 + 「没有终端」+ `+`
    /// （`terminal-tab-bar.tsx:708-755`）。
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let shell: WeakEntity<Self> = cx.entity().downgrade();

        if self.tabs.is_empty() {
            return h_flex()
                .w_full()
                .flex_shrink_0()
                .h(px(TAB_BAR_HEIGHT))
                .gap(px(INPUT_GAP))
                .px(px(8.))
                .bg(cx.theme().tab_bar)
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    Icon::new(IconName::SquareTerminal)
                        .w(px(16.))
                        .h(px(16.))
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_size(px(TAB_TEXT_SIZE))
                        .text_color(cx.theme().muted_foreground)
                        .child(SharedString::from(NO_TERMINALS)),
                )
                .child(self.new_tab_button(shell))
                .into_any_element();
        }

        let tabs: Vec<Tab> = self
            .tabs
            .iter()
            .map(|tab| {
                let id = tab.id;
                let name = tab.title.clone();
                let close_shell = shell.clone();
                // 「关闭 {name}」：`terminal.tabClose`，`locale.ts:7519`。
                let close_label = SharedString::from(format!("关闭 {name}"));
                let close = Button::new(SharedString::from(format!("terminal-tab-close-{id}")))
                    .ghost()
                    .icon(IconName::Close)
                    .tab_stop(false)
                    .w(px(TAB_CLOSE_BUTTON_SIZE))
                    .h(px(TAB_CLOSE_BUTTON_SIZE))
                    .rounded(px(CHROME_RADIUS))
                    .tooltip(close_label.clone())
                    .accessibility_label(close_label)
                    .on_click(
                        move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                            // 关闭按钮嵌在 `Tab::suffix` 里，`Button` 的点击默认会继续冒泡到
                            // 页签的 `on_click`（那会去切页签），所以这里显式掐断
                            // （`App::stop_propagation`，`gpui-pre-0.3.6/src/app.rs:2383`）。
                            cx.stop_propagation();
                            let _ = close_shell.update(cx, |this, cx| this.close_tab(id, cx));
                        },
                    );

                // 用 `child` 而不是 `label`：`Tab` 会在自己的根上写 `text_sm()`
                // （14px，`gpui-component-0.6.6/src/tab/tab.rs:803-807`），而用户样式在它之前
                // 写入、会被覆盖；把文字放进自己的 div 才能拿到真机的 13px
                // （`ui/tab-bar.tsx:170` 的 `ui-text-chrome`）。
                Tab::new()
                    .aria_label(name.clone())
                    .min_w(px(TAB_MIN_WIDTH))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(TAB_TEXT_SIZE))
                            .child(name),
                    )
                    .suffix(close)
            })
            .collect();

        let switch_shell = shell.clone();
        let bar = TabBar::new("terminal-tab-bar")
            .underline()
            // 真机单页签高 28（`--lithe-tab-height: 1.75rem`，`styles/theme.css:122`；
            // 用法见 `ui/tab-bar.tsx:257-267`）；gpui 的 `Tab` 高度由 `TabVariant::height(size)`
            // 决定，且 `Tab::render` 会在用户样式之后重写 `.h(...)`
            // （`gpui-component-0.6.6/src/tab/tab.rs:24-43,801`），改不动。
            // 表里能选的是 Small+Underline=**30** 与 Medium+Underline=36，取更接近的 30。
            .with_size(Size::Small)
            .selected_index(self.active)
            .max_width(px(TAB_MAX_WIDTH))
            .children(tabs)
            .suffix(
                h_flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap(px(2.))
                    .child(self.clear_button(shell.clone()))
                    .child(self.new_tab_button(shell.clone()))
                    .child(self.profile_menu_button(cx)),
            )
            .on_click(
                move |index: &usize, window: &mut Window, cx: &mut App| {
                    let index = *index;
                    let _ = switch_shell.update(cx, |this, cx| {
                        this.active = index;
                        // 切页签后把焦点还给输入行（真机切页签也会重新聚焦 xterm，
                        // `terminal.tsx:662-687` 的"验证式重试"就是在做这件事）。
                        let input = this.input.clone();
                        input.update(cx, |state, cx| state.focus(window, cx));
                        cx.notify();
                    });
                },
            );

        v_flex()
            .w_full()
            .flex_shrink_0()
            .h(px(TAB_BAR_HEIGHT))
            .bg(cx.theme().tab_bar)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(bar)
            .into_any_element()
    }

    /// 状态 + 能力边界一行（高 24）。
    ///
    /// 左边是会话状态：运行中 / 成功·退出码 N / 失败·退出码 N / 终端错误（起不来），
    /// 起不来或已退出时右侧多一个「重试」。右边常驻 [`CAPABILITY_NOTICE`]。
    fn render_status_line(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(tab) = self.tabs.get(self.active) else {
            return div().into_any_element();
        };

        let (label, color) = match &tab.session.state {
            SessionState::Running => {
                // 读线程报过 IO 错误就显示出来（正常管道不会走到这里）。
                let label = match &tab.session.io_error {
                    Some(error) => SharedString::from(format!("{RUNNING} · {error}")),
                    None => SharedString::from(RUNNING),
                };
                (label, cx.theme().success)
            }
            SessionState::Exited(0) => (SharedString::from(SUCCEEDED), cx.theme().success),
            SessionState::Exited(code) => (
                SharedString::from(format!("{FAILED} · {EXIT_CODE} {code}")),
                cx.theme().danger,
            ),
            SessionState::Failed(message) => (
                SharedString::from(format!("{ERROR_TITLE}：{message}")),
                cx.theme().danger,
            ),
        };

        let id = tab.id;
        let retryable = !tab.session.is_running();

        let mut line = h_flex()
            .w_full()
            .flex_shrink_0()
            .h(px(STATUS_LINE_HEIGHT))
            .gap(px(INPUT_GAP))
            .px(px(INPUT_ROW_PADDING_INLINE))
            .bg(cx.theme().muted)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .max_w(px(320.))
                    .truncate()
                    .text_size(px(NOTICE_TEXT_SIZE))
                    .text_color(color)
                    .child(label),
            );

        if retryable {
            line = line.child(
                Button::new(SharedString::from(format!("terminal-retry-{id}")))
                    .ghost()
                    .icon(IconName::Play)
                    .label(RETRY)
                    .tab_stop(false)
                    .h(px(STATUS_LINE_HEIGHT - 4.))
                    .rounded(px(CHROME_RADIUS))
                    .on_click(cx.listener(
                        move |this: &mut Self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                            this.retry_tab(id, cx);
                        },
                    )),
            );
        }

        line.child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(NOTICE_TEXT_SIZE))
                .text_color(cx.theme().muted_foreground)
                .child(SharedString::from(CAPABILITY_NOTICE)),
        )
        .into_any_element()
    }

    /// 输出区：贴底跟随的虚拟列表 + 底部"未完成行"。
    ///
    /// `MessageScroller`（`gpui-component-0.6.6/src/message_scroller.rs:185,284`）自带
    /// `FollowMode::Tail`、滚动条与"跳回最新"按钮（`jump_button`），正是
    /// `gpui/UI-MAP.md` §1.3 表格里推荐的"贴底跟随的输出区"。
    ///
    /// ⚠️ 它的行容器默认是**聊天记录**的排版（每行间 `pb_8()` = 32px、左右 `px_3()`，
    /// `message_scroller.rs:349-355`），终端要贴行显示，所以用 `with_row_style` 覆写内边距
    /// （`refine_style` 在那些内建 padding **之后**执行，能盖掉它们）。
    fn render_output(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(tab) = self.tabs.get(self.active) else {
            return self.render_empty(cx);
        };
        if let SessionState::Failed(message) = &tab.session.state {
            return self.render_failed(message.clone(), cx);
        }

        let rows = tab.rows.clone();
        let mono = cx.theme().mono_font_family.clone();
        let foreground = cx.theme().foreground;
        // 行渲染闭包是 `move` 的，会把字体名搬走；下面"未完成行"还要用，所以先留一份。
        let row_mono = mono.clone();

        let scroller = MessageScroller::new(
            SharedString::from(format!("terminal-output-{}", tab.id)),
            tab.scroller.clone(),
            move |index, _window, cx| {
                let text = rows.borrow().get(index).cloned().unwrap_or_default();
                div()
                    .w_full()
                    .min_w_0()
                    .font_family(row_mono.clone())
                    .text_size(px(TERMINAL_FONT_SIZE))
                    .line_height(relative(TERMINAL_LINE_HEIGHT))
                    .text_color(cx.theme().foreground)
                    .child(text)
            },
        )
        .scrollbar(true)
        .jump_button(true)
        .with_jump_button_label(SCROLL_TO_END)
        .with_row_style(output_row_style());

        // 未完成行贴在列表**下面**：进度条（`\r` 反复覆盖同一行）与提示符都停在底部，
        // 与真机的观感一致，也省掉了"每来一块就替换列表最后一行"的整表重排。
        let partial = tab.session.text.current_text();

        v_flex()
            .flex_1()
            .min_h_0()
            .w_full()
            .bg(cx.theme().background)
            .child(
                // 输出区的无障碍标签：`terminal.terminals`「终端」（`locale.ts:7558`）。
                // `MessageScroller` 自己挂 `Role::Log` 但不设标签
                // （`gpui-component-0.6.6/src/message_scroller.rs:364-368`），所以在包裹层补一个。
                // ⚠️ `.role()` / `.aria_label()` 在 `StatefulInteractiveElement` 上，而它只对
                // `Stateful<E>` 实现（`gpui-pre-0.3.6/src/elements/div.rs:1300-1326,4074`），
                // 所以必须先 `.id(...)`。
                div()
                    .id("terminal-output-viewport")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .role(Role::Log)
                    .aria_label(TERMINALS_ARIA)
                    // `MessageScroller` 的根是 `size_full()`，所以外面这层必须给出确定高度。
                    .child(div().size_full().child(scroller)),
            )
            .when_some(partial, |this, line| {
                this.child(
                    h_flex()
                        .w_full()
                        .flex_shrink_0()
                        .pl(px(TERMINAL_PADDING_INLINE))
                        .pr(px(TERMINAL_PADDING_INLINE))
                        .font_family(mono.clone())
                        .text_size(px(TERMINAL_FONT_SIZE))
                        .line_height(relative(TERMINAL_LINE_HEIGHT))
                        .text_color(foreground)
                        .child(line),
                )
            })
            .into_any_element()
    }

    /// 空态（没有终端页签）：真机是「没有终端」+ `+`（`terminal-tab-bar.tsx:708-755`）。
    fn render_empty(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex().size_full().child(
            Empty::new()
                .header(
                    EmptyHeader::new()
                        .media(
                            gpui_kit::component::empty::EmptyMedia::new()
                                .child(Icon::new(IconName::SquareTerminal).w(px(32.)).h(px(32.))),
                        )
                        .title(EmptyTitle::new().text_size(px(TAB_TEXT_SIZE)).child(SharedString::from(NO_TERMINALS))),
                )
                .content(EmptyContent::new().child(
                    Button::new("terminal-empty-new")
                        .icon(IconName::Plus)
                        .label(NEW_TERMINAL)
                        .h(px(TAB_CLOSE_BUTTON_SIZE))
                        .px(px(8.))
                        .rounded(px(CHROME_RADIUS))
                        .on_click(cx.listener(
                            |this: &mut Self, _event: &ClickEvent, window: &mut Window, cx: &mut Context<Self>| {
                                this.new_tab(window, cx);
                            },
                        )),
                )),
        )
        .into_any_element()
    }

    /// 失败态（子进程起不来）：终端错误 + 系统错误原文 + 「重试」。
    fn render_failed(&self, message: String, cx: &mut Context<Self>) -> AnyElement {
        let id = self.tabs.get(self.active).map(|tab| tab.id).unwrap_or_default();
        v_flex().size_full().child(
            Empty::new()
                .header(
                    EmptyHeader::new()
                        .media(
                            gpui_kit::component::empty::EmptyMedia::new()
                                .child(Icon::new(IconName::SquareTerminal).w(px(32.)).h(px(32.))),
                        )
                        .title(
                            EmptyTitle::new()
                                .text_size(px(TAB_TEXT_SIZE))
                                .child(SharedString::from(ERROR_TITLE)),
                        )
                        .description(
                            EmptyDescription::new()
                                .text_size(px(NOTICE_TEXT_SIZE))
                                .child(SharedString::from(message)),
                        ),
                )
                .content(EmptyContent::new().child(
                    Button::new("terminal-failed-retry")
                        .icon(IconName::Play)
                        .label(RETRY)
                        .h(px(TAB_CLOSE_BUTTON_SIZE))
                        .px(px(8.))
                        .rounded(px(CHROME_RADIUS))
                        .on_click(cx.listener(
                            move |this: &mut Self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>| {
                                this.retry_tab(id, cx);
                            },
                        )),
                )),
        )
        .into_any_element()
    }

    /// 底部命令输入行（真机没有这一行，是本外壳的替代：见文件头「与 Windows 的差异」）。
    fn render_input_row(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tab = self.tabs.get(self.active)?;
        if !tab.session.is_running() {
            return None;
        }

        // ⚠️ 必须走 `Styled::h` 的全限定写法：`Input` 有**同名固有方法**
        // `Input::h(impl Into<DefiniteLength>)`，它只写 `self.height`，而那个字段只在多行输入里
        // 生效（`gpui-component-0.6.6/src/input/input.rs:256-260`）；单行输入的实际高度来自
        // `input_h(size)` = 32px。全限定调用写的是样式表，而 `refine_style` 在 `input_h` 之后
        // 执行（`input.rs:703,719`），能覆写成真机 stdin 输入框的 28px。
        let input = gpui_kit::Styled::h(Input::new(&self.input), px(INPUT_HEIGHT))
            .w_full()
            .min_w_0()
            .rounded(px(CHROME_RADIUS))
            .font_family(cx.theme().mono_font_family.clone())
            .text_size(px(INPUT_TEXT_SIZE));

        Some(
            h_flex()
                .w_full()
                .flex_shrink_0()
                .gap(px(INPUT_GAP))
                .px(px(INPUT_ROW_PADDING_INLINE))
                .py(px(INPUT_ROW_PADDING_BLOCK))
                .bg(cx.theme().background)
                .border_t_1()
                .border_color(cx.theme().border)
                .child(input)
                .into_any_element(),
        )
    }

    /// `+` 新建终端（真机 `terminal-tab-bar.tsx:724-735`，图标 `Plus`、提示 `terminal.newTerminal`）。
    fn new_tab_button(&self, shell: WeakEntity<Self>) -> Button {
        Button::new("terminal-new-tab")
            .ghost()
            .icon(IconName::Plus)
            .tab_stop(false)
            .w(px(TAB_CLOSE_BUTTON_SIZE))
            .h(px(TAB_CLOSE_BUTTON_SIZE))
            .rounded(px(CHROME_RADIUS))
            .tooltip(NEW_TERMINAL)
            .accessibility_label(NEW_TERMINAL)
            .on_click(move |_event: &ClickEvent, window: &mut Window, cx: &mut App| {
                let _ = shell.update(cx, |this, cx| this.new_tab(window, cx));
            })
    }

    /// 清除输出（真机 `terminal.contextClear`，`terminal-tab-bar.tsx:959-964`）。
    ///
    /// ⚠️ 图标用 `Trash` 而不是真机的 `Trash2`：本仓库注册的 `gpui_kit::assets::AllAssets`
    /// 里**没有 `trash-2.svg`**（全量目录只有 `trash.svg` / `trash-off.svg`），
    /// `IconName` 是按 svg 文件名生成的（`gpui-kit-assets-0.6.6/build.rs:20-51`），
    /// 所以 `IconName::Trash2` 这个变体根本不存在。
    fn clear_button(&self, shell: WeakEntity<Self>) -> Button {
        Button::new("terminal-clear-output")
            .ghost()
            .icon(IconName::Trash)
            .tab_stop(false)
            .w(px(TAB_CLOSE_BUTTON_SIZE))
            .h(px(TAB_CLOSE_BUTTON_SIZE))
            .rounded(px(CHROME_RADIUS))
            .tooltip(CLEAR_TERMINAL)
            .accessibility_label(CLEAR_TERMINAL)
            .disabled(self.tabs.is_empty())
            .on_click(move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                let _ = shell.update(cx, |this, cx| this.clear_active_output(cx));
            })
    }

    /// 配置文件下拉（真机 `terminal-tab-bar.tsx:442-451,1031-1051`）。
    ///
    /// 这就是 [`TerminalProfile`] 文档里说的"设置界面接线点"在界面上的落点：菜单里列出
    /// [`TerminalPane::profiles`] 的内容，选中即改"新建页签用哪个 shell"。
    fn profile_menu_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let shell: WeakEntity<Self> = cx.entity().downgrade();
        let profiles: Vec<SharedString> = self
            .profiles
            .iter()
            .map(|profile| profile.name.clone())
            .collect();
        let active = self.active_profile;

        Button::new("terminal-profile-menu")
            .ghost()
            .icon(IconName::ChevronDown)
            .tab_stop(false)
            .w(px(TAB_CLOSE_BUTTON_SIZE))
            .h(px(TAB_CLOSE_BUTTON_SIZE))
            .rounded(px(CHROME_RADIUS))
            .tooltip(CHOOSE_PROFILE)
            .accessibility_label(CHOOSE_PROFILE)
            // `DropdownMenu`（`gpui-component-0.6.6/src/menu/dropdown_menu.rs:14-19`）；
            // 菜单项用 `PopupMenuItem::new(label).checked(..).on_click(..)`
            // （`menu/popup_menu.rs:71,180,196`）。
            .dropdown_menu(move |mut menu, _window, _cx| {
                menu = menu.scrollable(true);
                if profiles.is_empty() {
                    menu = menu.item(PopupMenuItem::new(ERROR_FALLBACK).disabled(true));
                    return menu;
                }
                for (index, name) in profiles.iter().enumerate() {
                    let shell = shell.clone();
                    menu = menu.item(
                        PopupMenuItem::new(name.clone())
                            .checked(index == active)
                            .on_click(move |_event, _window, cx| {
                                let _ = shell.update(cx, |this, cx| {
                                    this.active_profile = index;
                                    cx.notify();
                                });
                            }),
                    );
                }
                menu
            })
            .into_any_element()
    }
}

impl Render for TerminalPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 每个 `render_*` 都返回具体类型（`AnyElement` / `Option<AnyElement>`），
        // 所以这里可以顺序调用它们，再在最后一条表达式里 `cx.theme()`。
        let tab_bar = self.render_tab_bar(cx);
        let status = self.render_status_line(cx);
        let output = self.render_output(cx);
        let input_row = self.render_input_row(cx);

        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(tab_bar)
            .child(status)
            .child(output)
            .children(input_row)
    }
}

impl Drop for TerminalPane {
    /// 面板销毁时把**所有**子进程 kill + wait，绝不留下僵尸进程（真机窗口销毁也杀全部 PTY，
    /// `windows/tauri/crates/terminal/src/manager.rs:99-113`）。
    fn drop(&mut self) {
        for tab in &mut self.tabs {
            tab.session.shutdown();
        }
        if !self.tabs.is_empty() {
            println!("S1_TERMINAL_DROP tabs={}", self.tabs.len());
        }
    }
}

/// 输出区行容器的内边距覆写（见 [`TerminalPane::render_output`] 的说明）。
///
/// 左右 16 = 真机终端内容左内边距（`pl-4`，`terminal.tsx:870`）；上下 0 = 终端行必须贴行显示。
fn output_row_style() -> StyleRefinement {
    let mut style = StyleRefinement::default();
    let inline = px(TERMINAL_PADDING_INLINE);
    let zero = px(0.);
    style.padding.left = Some(inline.into());
    style.padding.right = Some(inline.into());
    style.padding.top = Some(zero.into());
    style.padding.bottom = Some(zero.into());
    style
}

// ---------------------------------------------------------------------------
// 图标替代表
//
// | 用途 | Windows 真机的字形 | 本项目用的字形 | 说明 |
// | --- | --- | --- | --- |
// | 终端（空态条 / 空态页） | `TerminalIcon` | `SquareTerminal`（`square-terminal.svg`） | 同语义，全量目录里确有 |
// | 新建终端 | `Plus` | `Plus`（`plus.svg`） | 同名 1:1 |
// | 关闭页签 | `X` | `Close`（`close.svg`） | 默认集没有 `x`，`close` 是同一个叉 |
// | 重新拉起 | —（真机是错误兜底的「重试」文字按钮） | `Play`（`play.svg`） | 表达"再跑一次" |
// | 清除输出 | `Trash2` | `Trash`（`trash.svg`） | **全量 Lucide 目录里没有 `trash-2.svg`**，`IconName::Trash2` 不存在 |
// | 配置文件下拉 | `ChevronDown` | `ChevronDown`（`chevron-down.svg`） | 同名 1:1 |
// | 跳回最新（输出区） | —（真机是 xterm 自己的滚动条） | `MessageScroller` 内建的 `ArrowDown` | 组件自带 |
//
// 未实现清单（本轮**不做**的，逐条写明卡在哪）：
//
// 1. **能力边界文案不是逐字取自 locale**：[`CAPABILITY_NOTICE`] 是本模块唯一自写的用户可见文案。
//    卡点：`locale.ts` 的 `terminal.*`（`:7503-7558`）里没有任何描述"没有 VT / Ctrl+C 不可用"的键，
//    而维护者的范围决定要求"界面上写清"。取舍：写一句中文并在这里登记，而不是假装它来自 locale。
//    建议：等 Windows 前端补了这类键（例如 `terminal.capabilityNotice`）再换成同一个键。
// 2. **VT 模拟**（光标定位、备用屏幕、滚动区域、真彩 SGR 着色、选区、查找、链接、IME、鼠标）：
//    全部不做 —— 这是维护者 2026-09-25 明确划出的范围（`gpui/UI-MAP-WINDOWS.md` §4 缺口清单第 1 条
//    也说 gpui-kit 三 crate 里 `portable_pty|conpty|alacritty|termwiz|vte|vt100` 零命中）。
//    本模块只做"丢弃控制序列 + `\r`/`\b` 落实"这一层，够"输出查看器"用（文件头列了参考实现）。
// 3. **ANSI 颜色**：SGR 被整段丢弃，所以 `ls --color` 之类的彩色输出全是单色。要上色得先有
//    "按 run 解析 SGR → 分段 span"的实现（真机是 `run-output-style.ts:92-199` 那套），
//    以及 16 色板（gpui-kit 没有终端色 token，`UI-MAP-WINDOWS.md` §3.1 第 24-39 行说要自建 `[Hsla; 16]`）。
// 4. **stderr 不单独着色**：真机的**运行**输出也是 stdout/stderr 混成一条流
//    （`run.rs:1932-1965` 只发 `run-output {sessionId, chunk}`），只有调试器控制台分色
//    （`debugger-view.tsx:671-672` 的 `text-destructive`）。本模块跟运行输出一致。
// 5. **GBK/CP936 输出**：`cmd.exe` 在 936 代码页下的中文会变成 `�`（[`TerminalText::push_bytes`]
//    有详述）。要修需要读系统 ANSI 代码页并做编码转换（真机 host 就是按 ANSI 代码页解码的，
//    `run.rs:1864-1903`），本仓库的 `gpui/shell` 没有这个依赖（`Cargo.toml` 只有 gpui-kit /
//    lithe-core / serde_json，且本轮**不允许**改 Cargo.toml）。
// 6. **无 PTY**：子进程没有控制台，因此 (a) 全屏程序不可用，(b) `Ctrl+C` 送不到，
//    (c) 行缓冲/回显/宽度协商都由子进程自己决定。要真做终端必须引 `portable-pty`
//    （真机就是它，`windows/tauri/crates/terminal/Cargo.toml:11`）—— 属于下一轮的决策，不在本轮范围。
// 7. **页签拖拽重排 / 固定 / 重命名 / 右键菜单（关闭其他、关闭全部、导出输出）**、
//    **分屏**（真机 `cmd+d` / `cmd+shift+d`）、**垂直页签布局**、**全屏**：`TabBar` 不支持拖拽，
//    垂直布局它也没有（`UI-MAP-WINDOWS.md` §2⑤），分屏要自建布局。本轮只做"新建 / 关闭 / 标题"。
// 8. **终端宽度模式**（`terminalWidthMode: "full" | "editor"`，默认 `"editor"`，
//    `features/terminal/stores/terminal.store.ts:29`）与**页签布局/位置**：属于底部工具窗容器的
//    职责（真机在 `main-layout.tsx:318-322`），不在这个面板模块里。
// 9. **搜索（`terminal.find`）、复制/粘贴、字号缩放、OSC 标题/目录跟随、导出输出**：未做。
//    真机的这些能力分别依赖 xterm 的 selection / addon 与 OSC 流解析
//    （`terminal-osc-stream.ts:93-104`），没有 VT 就没有落点。
// 10. **构造期不抢焦点**：[`TerminalPane::focus_input`] 要宿主在首次渲染后调一次
//    （原因见该方法的注释）。此外没有 ⌘/Ctrl 级快捷键（`terminal.new` = `cmd+t`、
//    `terminal.close` = `cmd+w`，`default-keymaps.ts:122-139`）—— 快捷键要挂在有焦点的根元素上，
//    属于外壳的接线，不在本模块。
//
// 实测记录（本文件里"不做本地回显""按 CRLF 断行"两条结论的依据，2026-09-25，Windows 10 19045）：
//
// - `"echo hello-from-pipe`r`ndir /b`r`nexit`r`n" | & cmd.exe /Q`（PowerShell 造管道）：
//   输出是 `Microsoft Windows [版本 …]` 横幅 + `D:\…>hello-from-pipe`（**提示符 + 命令回显**）
//   + 目录列表 + 下一条提示符；命令逐行执行、输出实时流出，`exit` 后进程自行退出。
// - 用 `Start-Job` 起
//   `cmd /c "(echo Write-Output MARKER-1 & ping -n 6 127.0.0.1 >nul & echo Write-Output MARKER-2) | powershell -NoLogo -NoProfile"`
//   并把 stdout 重定向到文件：**t=2s**（第一条命令之后、生产者还没写下第二条时）文件里已经有
//   `PS D:\…> Write-Output MARKER-1` 与 `MARKER-1`；t=8s 才看到 `MARKER-2`。
//   → PowerShell 在管道下是**逐行**读 stdin 并执行（不是"读完整个 stdin 当脚本跑"），
//   而且同样打印 `PS D:\…>` 提示符与命令回显。顺带测了 `powershell -Command -`：也逐行执行，
//   但**不**打印提示符。
// - 因此：本模块不做本地回显（会重复两遍），并且发送时用 `\r\n` 结尾。
// - ⚠️ 上面两条测的是**带 `-NoLogo -NoProfile` 的 powershell**；[`profiles`] 默认**不带参数**
//   （对齐真机的"系统默认"profile，`terminal-profiles.ts:19-23`），差别只是多一条启动横幅。
