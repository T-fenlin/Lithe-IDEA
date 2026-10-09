use std::time::Duration;

use gpui::SharedString;
use lithe_gpui_shared::tr;

pub(crate) const NO_TERMINALS_KEY: &str = "lithe.terminal.noTerminals";
pub(crate) const CHOOSE_PROFILE_KEY: &str = "lithe.terminal.chooseTerminalProfile";
pub(crate) const CLEAR_TERMINAL_KEY: &str = "lithe.terminal.contextClear";
pub(crate) const ERROR_TITLE_KEY: &str = "lithe.terminal.errorTitle";
pub(crate) const ERROR_FALLBACK_KEY: &str = "lithe.terminal.errorFallback";
pub(crate) const RETRY_KEY: &str = "lithe.terminal.retry";
pub(crate) const RUNNING_KEY: &str = "lithe.run.running";
pub(crate) const SUCCEEDED_KEY: &str = "lithe.run.succeeded";
pub(crate) const FAILED_KEY: &str = "lithe.run.failed";
pub(crate) const EXIT_CODE_KEY: &str = "lithe.git.console.exit";
pub(crate) const INPUT_PLACEHOLDER_KEY: &str = "lithe.commandPalette.placeholder";
pub(crate) const SCROLL_TO_END_KEY: &str = "lithe.git.console.scrollToEnd";
pub(crate) const TERMINALS_ARIA_KEY: &str = "lithe.terminal.terminals";

pub(crate) const CAPABILITY_NOTICE: &str = "能力边界：不能运行 vim/top 等全屏程序；Ctrl+C 不可用";

pub(crate) fn no_terminals() -> SharedString {
    tr(NO_TERMINALS_KEY)
}

pub(crate) fn choose_profile() -> SharedString {
    tr(CHOOSE_PROFILE_KEY)
}

pub(crate) fn clear_terminal() -> SharedString {
    tr(CLEAR_TERMINAL_KEY)
}

pub(crate) fn error_title() -> SharedString {
    tr(ERROR_TITLE_KEY)
}

pub(crate) fn error_fallback() -> SharedString {
    tr(ERROR_FALLBACK_KEY)
}

pub(crate) fn retry() -> SharedString {
    tr(RETRY_KEY)
}

pub(crate) fn running() -> SharedString {
    tr(RUNNING_KEY)
}

pub(crate) fn succeeded() -> SharedString {
    tr(SUCCEEDED_KEY)
}

pub(crate) fn failed() -> SharedString {
    tr(FAILED_KEY)
}

pub(crate) fn exit_code() -> SharedString {
    tr(EXIT_CODE_KEY)
}

pub(crate) fn input_placeholder() -> SharedString {
    tr(INPUT_PLACEHOLDER_KEY)
}

pub(crate) fn scroll_to_end() -> SharedString {
    tr(SCROLL_TO_END_KEY)
}

pub(crate) fn terminals_aria() -> SharedString {
    tr(TERMINALS_ARIA_KEY)
}

// 下面三个几何规格现在**只有测试**在读（宿主原来的 `h(...)` / `rounded(...)` 随库
// `Button` / `Input` 的几何收口而消失，见 `terminal_view.rs` 的差异注释）。常量本身
// 仍是终端的规格真源，留在这里等对应的库片把入口补回来。
#[allow(dead_code)]
pub(crate) const CHROME_RADIUS: f32 = 4.;
#[allow(dead_code)]
pub(crate) const STATUS_LINE_HEIGHT: f32 = 24.;
pub(crate) const TERMINAL_LINE_HEIGHT: f32 = 1.;
/// 转录每行的横向内衬（终端的规格真源）。**当前没有读点**：迁到组件库的
/// `lists::MessageScroller` 之后，逐行内衬归库自己（每行 `px_3`），而库还没有
/// `with_row_style` 那样的入口把 16px 交回去——这个缺口登记在移植汇报里，
/// 库片补上后这里会重新被读到。
#[allow(dead_code)]
pub(crate) const TERMINAL_PADDING_INLINE: f32 = 16.;
pub(crate) const MAX_LINES: usize = 10_000;
pub(crate) const READ_BUFFER: usize = 4 * 1024;
pub(crate) const EXIT_POLL_ATTEMPTS: usize = 10;
pub(crate) const EXIT_POLL_INTERVAL: Duration = Duration::from_millis(20);

pub(crate) const NEW_TERMINAL_KEY: &str = "lithe.terminal.newTerminal";

pub(crate) fn new_terminal_label() -> SharedString {
    lithe_gpui_shared::tr(NEW_TERMINAL_KEY)
}
