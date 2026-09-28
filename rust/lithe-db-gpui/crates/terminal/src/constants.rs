//! 文案与度量常量。
//!
//! 从 `shell_probe/terminal.rs` 的「文案」「度量」两节原样拆出（逐字搬迁，只把 `const` 放宽到
//! `pub(crate)`，让本 crate 的兄弟模块共享同一份字面量）。所有数值的来源注释都保留在每一条上。
//!
//! ⚠️ **文案一律走函数，不再用 `&str` 常量**：`lithe_db_gpui_shared::tr` 必须在运行时读当前
//! locale，`const` 装不下 `SharedString`。所以下面每个 `*_KEY` 常量只保留**翻译键**，
//! 取文案的函数返回 `SharedString`；调用点写 `foo()`。键与原文的对应关系逐条登记在注释里
//! （locale 真源是 `windows/tauri/src/i18n/locale.ts`，经 `lithe-db-gpui/tools/extract-locale.mjs` 生成）。

use std::time::Duration;

use gpui_kit::SharedString;
use lithe_db_gpui_shared::tr;

// ---------------------------------------------------------------------------
// 文案键
// ---------------------------------------------------------------------------

/// `terminal.noTerminals` → 「没有终端」（`locale.ts:7544`）：空态条与空态页。
pub(crate) const NO_TERMINALS_KEY: &str = "lithe.terminal.noTerminals";
/// `terminal.chooseTerminalProfile` → 「选择终端配置文件」（`locale.ts:7543`）：配置文件下拉。
pub(crate) const CHOOSE_PROFILE_KEY: &str = "lithe.terminal.chooseTerminalProfile";
/// `terminal.contextClear` → 「清除终端」（`locale.ts:7510`）：真机在页签右键菜单里，本模块做成图标按钮。
pub(crate) const CLEAR_TERMINAL_KEY: &str = "lithe.terminal.contextClear";
/// `terminal.errorTitle` → 「终端错误」（`locale.ts:7549`）。
pub(crate) const ERROR_TITLE_KEY: &str = "lithe.terminal.errorTitle";
/// `terminal.errorFallback` → 「无法初始化终端」（`locale.ts:7550`）。
pub(crate) const ERROR_FALLBACK_KEY: &str = "lithe.terminal.errorFallback";
/// `terminal.retry` → 「重试」（`locale.ts:7551`）：失败/已退出后的重新拉起。
pub(crate) const RETRY_KEY: &str = "lithe.terminal.retry";
/// `run.running` → 「运行中」（`locale.ts:5930`）。
pub(crate) const RUNNING_KEY: &str = "lithe.run.running";
/// `run.succeeded` → 「成功」（`locale.ts:5931`）。
pub(crate) const SUCCEEDED_KEY: &str = "lithe.run.succeeded";
/// `run.failed` → 「失败」（`locale.ts:5932`）。
pub(crate) const FAILED_KEY: &str = "lithe.run.failed";
/// `git.console.exit` → 「退出码」（`locale.ts:4568`）：真机在 Git 控制台里这样拼
/// `退出码 {code}`（`features/git/components/log/git-console-entry.tsx:82-84`）。
pub(crate) const EXIT_CODE_KEY: &str = "lithe.git.console.exit";
/// `commandPalette.placeholder` → 「输入命令...」（`locale.ts:7905`）：命令输入框的占位。
///
/// 终端真机没有"输入行"（直接打进 xterm，`use-terminal-connection.ts:145-146`），
/// 所以这一条借的是命令面板的原文，而不是终端自己的键。
pub(crate) const INPUT_PLACEHOLDER_KEY: &str = "lithe.commandPalette.placeholder";
/// `git.console.scrollToEnd` → 「滚动到底部」（`locale.ts:4494`）：输出区"跳回最新"的提示。
pub(crate) const SCROLL_TO_END_KEY: &str = "lithe.git.console.scrollToEnd";
/// `terminal.terminals` → 「终端」（`locale.ts:7558`）：输出区的无障碍标签。
///
/// 取 `terminal.terminals` 而不是 `workbench.terminal`（两者 zh-CN 同值「终端」）：
/// 调用点是终端 Feature 自己的 a11y 名，按 feature 命名空间归属。
pub(crate) const TERMINALS_ARIA_KEY: &str = "lithe.terminal.terminals";

// ---------------------------------------------------------------------------
// 取文案
// ---------------------------------------------------------------------------

/// 能力边界声明（**常驻显示**，不是可选提示）。
///
/// 本模块只做"输出查看器"外壳：没有 VT 模拟 → 全屏程序（`vim` / `top` / `less`）画不出画面；
/// 没有 PTY/控制台输入通道 → `Ctrl+C` 送不进子进程，命令无法中断。
///
/// ⚠️ `locale.ts` 的 `terminal.*` 全量（`:7503-7558`）里**没有**描述这两条限制的键，
/// 所以这一句是本模块唯一**不是**逐字取自 locale 的文案；原因与取舍见文件末尾未实现清单第 1 条。
/// 它因此也**故意不接 i18n**（保留 `&str` 常量）：接了也没有 key 可用。
pub(crate) const CAPABILITY_NOTICE: &str = "能力边界：不能运行 vim/top 等全屏程序；Ctrl+C 不可用";

/// `terminal.noTerminals` → 「没有终端」。
pub(crate) fn no_terminals() -> SharedString {
    tr(NO_TERMINALS_KEY)
}

/// `terminal.chooseTerminalProfile` → 「选择终端配置文件」。
pub(crate) fn choose_profile() -> SharedString {
    tr(CHOOSE_PROFILE_KEY)
}

/// `terminal.contextClear` → 「清除终端」。
pub(crate) fn clear_terminal() -> SharedString {
    tr(CLEAR_TERMINAL_KEY)
}

/// `terminal.errorTitle` → 「终端错误」。
pub(crate) fn error_title() -> SharedString {
    tr(ERROR_TITLE_KEY)
}

/// `terminal.errorFallback` → 「无法初始化终端」。
pub(crate) fn error_fallback() -> SharedString {
    tr(ERROR_FALLBACK_KEY)
}

/// `terminal.retry` → 「重试」。
pub(crate) fn retry() -> SharedString {
    tr(RETRY_KEY)
}

/// `run.running` → 「运行中」。
pub(crate) fn running() -> SharedString {
    tr(RUNNING_KEY)
}

/// `run.succeeded` → 「成功」。
pub(crate) fn succeeded() -> SharedString {
    tr(SUCCEEDED_KEY)
}

/// `run.failed` → 「失败」。
pub(crate) fn failed() -> SharedString {
    tr(FAILED_KEY)
}

/// `git.console.exit` → 「退出码」。
pub(crate) fn exit_code() -> SharedString {
    tr(EXIT_CODE_KEY)
}

/// `commandPalette.placeholder` → 「输入命令...」。
pub(crate) fn input_placeholder() -> SharedString {
    tr(INPUT_PLACEHOLDER_KEY)
}

/// `git.console.scrollToEnd` → 「滚动到底部」。
pub(crate) fn scroll_to_end() -> SharedString {
    tr(SCROLL_TO_END_KEY)
}

/// `terminal.terminals` → 「终端」。
pub(crate) fn terminals_aria() -> SharedString {
    tr(TERMINALS_ARIA_KEY)
}

// ---------------------------------------------------------------------------
// 度量
// ---------------------------------------------------------------------------
//
// 布局度量一律用 gpui 的 rem-based helper，**生产代码里不再有直接 `px(...)` 的调用点**
// （`terminal_view.rs` 末尾的换算测试里用 `px(...)` 当期望值，那是断言不是布局）。rem base = 主题字号
// 16px，所以 helper 后缀 `N` = `N × 4px`，与 Windows 规格逐像素相等：
//
// | 规格 | 值 | 用到的 helper | 出处 |
// | --- | --- | --- | --- |
// | 页签条高 36 | 36 | `h_9()` | `UI-MAP-WINDOWS.md` §1.6、`ui/tab-bar.tsx:244-255` |
// | 单页签最小宽 80 | 80 | `min_w_20()` | `ui/tab-bar.tsx:257-267` |
// | 页签关闭按钮 24×24 | 24 | `size_6()` | `ui/button.tsx:27` 的 `icon-xs` |
// | 状态/边界提示行高 `--lithe-chrome-control-height` | 24 | `h_6()` | `styles/theme.css:126` |
// | 终端内容左右内边距 `pl-4` | 16 | `pl_4()` / `pr_4()` | `features/terminal/components/terminal.tsx:870` |
// | 输入行外框上下 6 / 左右 12、行内间距 6、输入框高 28 | 6 / 12 / 6 / 28 | `py_1p5()` / `px_3()` / `gap_1p5()` / `h_7()` | `features/run/components/run-pane.tsx:454,462` |
//
// **档位外的值**（页签最大宽 200、chrome 圆角 4、状态行 24 − 4 = 20）不在上面那张 helper 表里，
// 按《编码指南》写成 helper 底层的 `rems(P / 16.)`（`lithe-db-gpui/docs/gpui-kit/0.6.6/zh-CN/docs/coding-guides.md:253,288`）；
// 消费方只收 `Pixels` 的那几处（`Button::rounded` / `TabBar::max_width`）走 `terminal_view.rs`
// 的 `rem_px(rem, P)` —— 同形于 `settings/src/dialog.rs:189-199`，按**当前** rem 基准求值。
//
// 字号：`ui-text-sm` 是 **13px**、`--ui-text-caption` 是 **12px**（`styles/theme.css:114-116`），
// gpui 的档位是 `text_xs()`=12 / `text_sm()`=14：12 → `text_xs()`（等价）、
// 13 → `text_sm()`（14px，经维护者确认的**有意**视觉改动）、终端字号 14 → `text_sm()`（等价）。

/// 单个页签最大宽 200（`ui/tab-bar.tsx:257-267`）。
///
/// 200 不在 gpui 的固定 rem 档位上（档位里 48 → 192、56 → 224），没有 `max_w_50()`，所以它保持
/// **规格像素值**这个身份，由调用点换算成 rem：`TabBar::max_width` 只收 `Pixels`，所以走
/// `terminal_view.rs` 的 `rem_px(rem, TAB_MAX_WIDTH)`（= `rems(TAB_MAX_WIDTH / 16.)`）。
/// 写成 `/ 4.` 就错了 —— 1rem = 16px，不是 4px。
pub(crate) const TAB_MAX_WIDTH: f32 = 200.;
/// chrome 圆角 4px（`styles/theme.css:133` 的 `--lithe-chrome-radius: 4px`）。
///
/// Lithe 的圆角阶梯（`--radius × k`）一律走应用层具名常量，不套 gpui 的 `rounded_sm()`/
/// `rounded_md()`（语义不同：Lithe 的 `sm` 是 4.8、`md` 是 6.4），也不能从主题读 ——
/// `ThemeConfig.radius` 是 `usize`（`gpui-component-0.6.6/src/theme/schema.rs:67-68`），
/// 装不下 4.8 / 6.4。
///
/// 它是**规格像素值**（4 不在 gpui 档位上），调用点不再写 `px(4.)`：能吃 `AbsoluteLength` 的
/// 直接写 `rems(CHROME_RADIUS / 16.)`，`Button::rounded`（只收 `Pixels`）走 `rem_px(rem, CHROME_RADIUS)`。
pub(crate) const CHROME_RADIUS: f32 = 4.;
/// 状态/边界提示行高 24px（`styles/theme.css:126` 的 `--lithe-chrome-control-height: 1.5rem`）。
///
/// 行高本身用 `h_6()`；常量保留是因为「重试」按钮的高度是运行时算术 `24 − 4`
/// （让出状态行自己的上下内边距，见 [`crate::terminal_view`] 的 `render_status_line`）——
/// 20 不在档位上，所以那处写 `rems((STATUS_LINE_HEIGHT - 4.) / 16.)`，规格值集中在这个常量里。
pub(crate) const STATUS_LINE_HEIGHT: f32 = 24.;
/// 终端行高倍数 1（`config/default-settings.ts:77` 的 `terminalLineHeight: 1`）——
/// 是倍数不是长度，`line_height(relative(..))` 原样保留。
pub(crate) const TERMINAL_LINE_HEIGHT: f32 = 1.;
/// 终端内容左右内边距 16（`pl-4`，`features/terminal/components/terminal.tsx:870`）。
///
/// 16 在 rem 档位上（样式调用点用 `pl_4()` / `pr_4()`）；常量保留是因为输出行的行样式直接改
/// `StyleRefinement`（见 `output_row_style`），那里没有 `Styled` 的 helper 可用，
/// 只能写 `rems(TERMINAL_PADDING_INLINE / 16.)`。
pub(crate) const TERMINAL_PADDING_INLINE: f32 = 16.;
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
/// 这是 `lithe_db_gpui_shared::{tr, tr_args}` 在终端 Feature 里的**真实调用点**：
/// locale 的 `lithe.terminal.newTerminal` 在 zh-CN 下就是「新建终端」，与原来写死的字面量
/// 逐字相同，所以换成翻译键之后界面文案不变。
pub(crate) fn new_terminal_label() -> SharedString {
    lithe_db_gpui_shared::tr(NEW_TERMINAL_KEY)
}
