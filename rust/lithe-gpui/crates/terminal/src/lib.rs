//! 终端 Feature：底部工具窗的「终端」页。
//!
//! ## 职责
//!
//! 从 `lithe-gpui/shell/src/bin/shell_probe/terminal.rs`（1753 行，单个 bin crate 里的扁平模块）
//! 拆出来，按《编码指南》「大型应用按业务能力组织 crate」做成一个**拥有独立状态与生命周期**
//! 的 Feature crate。四件事：**工具窗页签（新建 / 关闭 / 标题）+ 流式显示 stdout/stderr
//! + 一个发送命令的输入行 + 最小 ANSI 清洗**。终端进程由本 crate 直接在 Rust 宿主里用
//! `std::process::Command` 起（**不是** Windows 那套 ConPTY + `#[tauri::command]` 协议）。
//!
//! ## 公开边界
//!
//! 只有 [`TerminalPane`]（面板实体）、[`TerminalPaneEvent`]（面板事件）与 [`TerminalProfile`]
//! （配置文件）是公开 API，由 `lib.rs` 明确 re-export。`constants` / `ansi` / `session` /
//! `terminal_view` 这些实现路径**不**出现在 import 路径里，符合《编码指南》「内部重组时保持
//! public module path」的反面用法：新建 crate 直接发布 `pub use`，不把内部 folder 结构变成契约。
//!
//! ## ⚠️ 会话是**懒创建**的
//!
//! [`TerminalPane::new`] **不 spawn shell** —— 底部工具窗默认隐藏
//! （`features/window/stores/workspace-ui-defaults.ts:5` 的 `isBottomPaneVisible: false`），
//! 真机也是面板第一次可见时才建会话（`bottom-pane/bottom-pane.tsx:63-106` +
//! `terminal-container.tsx:212-219`）。宿主在第一次显示时调
//! [`TerminalPane::ensure_session`]（幂等，顺带把焦点交给输入行）；关掉**最后一个**页签时
//! 面板发 [`TerminalPaneEvent::LastTabClosed`]，宿主据此收起底部工具窗
//! （真机 `features/terminal/utils/terminal-pane-visibility.ts:14-26`）。
//!
//! ## ⚠️ 能力边界（**必须让用户看得见**，也在界面上以 `CAPABILITY_NOTICE` 常驻显示）
//!
//! - **不能运行 `vim` / `top` / `less` 这类全屏 TUI 程序**：没有 VT 模拟（无光标定位、无备用屏幕、
//!   无滚动区域），全屏程序画出来的字符网格只会被当成普通输出流铺开，画面不可读。
//! - **`Ctrl+C` 不可用**：本 crate 没有 PTY / 控制台输入通道，无法把 `CTRL_C_EVENT`（Windows）或
//!   `SIGINT`（POSIX）送进子进程；正在运行的命令只能靠**关闭页签**（`Child::kill`）中止。
//! - 只有"输出查看器"级别的显示：丢弃 SGR 之后**没有任何颜色**、没有选区、没有查找、没有链接、
//!   没有光标、没有鼠标输入（`terminal.tsx:267-289` 里 xterm.js 的那些能力全部缺失）。
//! - 终端**列宽 / 行高模型不存在**：长行按视口宽度软换行，而不是按终端的列数换行
//!   （真机是 xterm.js 的字符网格，`terminal.tsx:632-651` 还会把尺寸同步给 ConPTY）。
//!
//! ## 与 Windows 前端的差异（逐条写清，避免把这里当规格真源）
//!
//! | 项 | Windows 前端 | 本 crate |
//! | --- | --- | --- |
//! | 进程 | Rust host 用 `portable-pty` 0.9 开 **ConPTY**（`windows/tauri/crates/terminal/Cargo.toml:11`、`connection.rs:36-49`） | `std::process::Command` + 三个管道，**没有 PTY** |
//! | 通道 | 8 个 command + `ipc::Channel<TerminalEvent>`，`data: number[]`（`src-tauri/src/terminal.rs:85-181`） | 进程内 `std::sync::mpsc` + 读线程 |
//! | 渲染 | xterm.js 6 Canvas/WebGL（`terminal.tsx:267-289`） | 纯文本行 + 最小 ANSI 清洗（见 `ansi.rs`） |
//! | 输入 | 直接打进 xterm（`use-terminal-connection.ts:145-146`） | 底部单行输入框，**回车**发送一行（回车 = `\r\n`） |
//! | 回显 | ConPTY 回显 | **靠子进程自己回显**：实测 `cmd.exe` 与 `powershell.exe` 在管道下都会打印提示符与命令回显，所以本 crate **不做本地回显** |
//! | 编码 | host **零 ANSI 清洗**，可读性 100% 靠 xterm.js | 本 crate 做 ANSI 清洗 |
//!
//! ## 文件分工
//!
//! - `profile.rs`：`TerminalProfile` + `profiles()` + 运行时探测；
//! - `ansi.rs`：最小 ANSI 清洗与行缓冲（纯状态机，可单测）；
//! - `session.rs`：spawn 进程、读线程、`mpsc`、kill/wait、代次 + 与进程有关的 `TerminalPane` 方法；
//! - `terminal_view.rs`：`TerminalPane` 的 `Render` 与各区域渲染；
//! - `constants.rs`：文案与度量常量。
//!
//! 未实现清单、图标替代表、"设置界面接线点"说明都在 `terminal_view.rs` 末尾的注释块里。

mod ansi;
mod constants;
mod profile;
mod session;
mod terminal_view;

pub use profile::TerminalProfile;
pub use session::TerminalPaneEvent;
pub use terminal_view::TerminalPane;
