//! 文案与度量常量。
//!
//! 从 `shell_probe/terminal.rs` 的「文案」「度量」两节原样拆出（逐字搬迁，只把 `const` 放宽到
//! `pub(crate)`，让本 crate 的兄弟模块共享同一份字面量）。所有数值的来源注释都保留在每一条上。

use std::time::Duration;

// 文案（除 [`CAPABILITY_NOTICE`] 外全部逐字取 `windows/tauri/src/i18n/locale.ts`）
// ---------------------------------------------------------------------------

/// `terminal.noTerminals` → 「没有终端」（`locale.ts:7544`）：空态条与空态页。
pub(crate) const NO_TERMINALS: &str = "没有终端";
/// `terminal.newTerminal` → 「新建终端」（`locale.ts:7531`）：新建按钮的提示与无障碍名。
/// `terminal.chooseTerminalProfile` → 「选择终端配置文件」（`locale.ts:7543`）：配置文件下拉。
pub(crate) const CHOOSE_PROFILE: &str = "选择终端配置文件";
/// `terminal.contextClear` → 「清除终端」（`locale.ts:7510`）：真机在页签右键菜单里，本模块做成图标按钮。
pub(crate) const CLEAR_TERMINAL: &str = "清除终端";
/// `terminal.errorTitle` → 「终端错误」（`locale.ts:7549`）。
pub(crate) const ERROR_TITLE: &str = "终端错误";
/// `terminal.errorFallback` → 「无法初始化终端」（`locale.ts:7550`）。
pub(crate) const ERROR_FALLBACK: &str = "无法初始化终端";
/// `terminal.retry` → 「重试」（`locale.ts:7551`）：失败/已退出后的重新拉起。
pub(crate) const RETRY: &str = "重试";
/// `run.running` → 「运行中」（`locale.ts:5930`）。
pub(crate) const RUNNING: &str = "运行中";
/// `run.succeeded` → 「成功」（`locale.ts:5931`）。
pub(crate) const SUCCEEDED: &str = "成功";
/// `run.failed` → 「失败」（`locale.ts:5932`）。
pub(crate) const FAILED: &str = "失败";
/// `git.console.exit` → 「退出码」（`locale.ts:4568`）：真机在 Git 控制台里这样拼
/// `退出码 {code}`（`features/git/components/log/git-console-entry.tsx:82-84`）。
pub(crate) const EXIT_CODE: &str = "退出码";
/// `commandPalette.placeholder` → 「输入命令...」（`locale.ts:7905`）：命令输入框的占位。
///
/// 终端真机没有"输入行"（直接打进 xterm，`use-terminal-connection.ts:145-146`），
/// 所以这一条借的是命令面板的原文，而不是终端自己的键。
pub(crate) const INPUT_PLACEHOLDER: &str = "输入命令...";
/// `git.console.scrollToEnd` → 「滚动到底部」（`locale.ts:4494`）：输出区"跳回最新"的提示。
pub(crate) const SCROLL_TO_END: &str = "滚动到底部";
/// `terminal.terminals` → 「终端」（`locale.ts:7558`）：输出区的无障碍标签。
pub(crate) const TERMINALS_ARIA: &str = "终端";

/// 能力边界声明（**常驻显示**，不是可选提示）。
///
/// 本模块只做"输出查看器"外壳：没有 VT 模拟 → 全屏程序（`vim` / `top` / `less`）画不出画面；
/// 没有 PTY/控制台输入通道 → `Ctrl+C` 送不进子进程，命令无法中断。
///
/// ⚠️ `locale.ts` 的 `terminal.*` 全量（`:7503-7558`）里**没有**描述这两条限制的键，
/// 所以这一句是本模块唯一**不是**逐字取自 locale 的文案；原因与取舍见文件末尾未实现清单第 1 条。
pub(crate) const CAPABILITY_NOTICE: &str = "能力边界：不能运行 vim/top 等全屏程序；Ctrl+C 不可用";

// ---------------------------------------------------------------------------
// 度量（全部有出处；`px()` 直搬，`gpui/UI-MAP.md` §1.1 第 1 条）
// ---------------------------------------------------------------------------

/// 页签条高 36（`UI-MAP-WINDOWS.md` §1.6；`ui/tab-bar.tsx:244-255`）。
pub(crate) const TAB_BAR_HEIGHT: f32 = 36.;
/// 单个页签最小宽 80 / 最大宽 200（同上，`ui/tab-bar.tsx:257-267`）。
pub(crate) const TAB_MIN_WIDTH: f32 = 80.;
pub(crate) const TAB_MAX_WIDTH: f32 = 200.;
/// 页签文字 13px（`ui/tab-bar.tsx:170` 的 `ui-text-chrome`）。
pub(crate) const TAB_TEXT_SIZE: f32 = 13.;
/// 页签关闭按钮 24×24（`ui/button.tsx:27` 的 `icon-xs`）。
pub(crate) const TAB_CLOSE_BUTTON_SIZE: f32 = 24.;
/// chrome 圆角 4px（`styles/theme.css:133` 的 `--lithe-chrome-radius: 4px`）。
pub(crate) const CHROME_RADIUS: f32 = 4.;
/// 状态/边界提示行高 24px（`styles/theme.css:126` 的 `--lithe-chrome-control-height: 1.5rem`）。
pub(crate) const STATUS_LINE_HEIGHT: f32 = 24.;
/// 提示文字 12px（`styles/theme.css:114` 的 `--ui-text-caption: 12px`）。
pub(crate) const NOTICE_TEXT_SIZE: f32 = 12.;
/// 终端字号 14（`config/default-settings.ts:76` + `config/typography-defaults.ts:13`）。
pub(crate) const TERMINAL_FONT_SIZE: f32 = 14.;
/// 终端行高倍数 1（`config/default-settings.ts:77` 的 `terminalLineHeight: 1`）。
pub(crate) const TERMINAL_LINE_HEIGHT: f32 = 1.;
/// 终端内容左右内边距 16（`pl-4`，`features/terminal/components/terminal.tsx:870`）。
pub(crate) const TERMINAL_PADDING_INLINE: f32 = 16.;
/// 输入行：外框上下 6 / 左右 12、行内间距 6、输入框高 28、输入字号 12
/// （运行面板 stdin 行 `flex items-center gap-1.5 border-t px-3 py-1.5` + `h-7 font-mono text-[12px]`，
/// `features/run/components/run-pane.tsx:454,462`；`UI-MAP-WINDOWS.md` §3.5）。
pub(crate) const INPUT_ROW_PADDING_INLINE: f32 = 12.;
pub(crate) const INPUT_ROW_PADDING_BLOCK: f32 = 6.;
pub(crate) const INPUT_GAP: f32 = 6.;
pub(crate) const INPUT_HEIGHT: f32 = 28.;
pub(crate) const INPUT_TEXT_SIZE: f32 = 12.;
/// 输出保留上限 10000 行（真机默认 `terminalScrollback: 10000`，`config/default-settings.ts:79`）。
pub(crate) const MAX_LINES: usize = 10_000;
/// 管道单次读取缓冲 4 KiB（与真机 host 的读缓冲同量级：`crates/terminal/src/connection.rs:453-512`
/// 用 64 KiB；这里按行缓冲、上限 10000 行，4 KiB 足够）。
pub(crate) const READ_BUFFER: usize = 4 * 1024;
/// 两条读线程都 EOF 之后，用 `try_wait` 收退出码的轮询次数与间隔。
///
/// `cmd.exe` / `powershell.exe` 在管道下都会在退出前关掉两条管道，所以正常情况第一次
/// `try_wait` 就能拿到状态；这两次等待只是为了极端的时序（子进程已关管道但还没退出）。
pub(crate) const EXIT_POLL_ATTEMPTS: usize = 10;
pub(crate) const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// `terminal.newTerminal` 的翻译键（`i18n/locale.ts:7531`）。
pub(crate) const NEW_TERMINAL_KEY: &str = "lithe.terminal.newTerminal";

/// `terminal.newTerminal` 的当前语言文案。
///
/// 这是 `lithe_gpui_shared::{tr, tr_args}` 在终端 Feature 里的**真实调用点**：
/// locale 的 `lithe.terminal.newTerminal` 在 zh-CN 下就是「新建终端」，与原来写死的字面量
/// 逐字相同，所以换成翻译键之后界面文案不变。其余文案的大规模改写不在本轮范围。
pub(crate) fn new_terminal_label() -> gpui_kit::SharedString {
    lithe_gpui_shared::tr(NEW_TERMINAL_KEY)
}
