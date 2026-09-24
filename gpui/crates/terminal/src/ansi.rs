//! 最小 ANSI 清洗 + 行缓冲（**纯状态机，不碰 GPUI，可单测**）。
//!
//! 从 `shell_probe/terminal.rs` 的「最小 ANSI 清洗 + 行缓冲」一节原样拆出（逐字搬迁）。
//!
//! 与真机的差别：真机把字节原样喂给 xterm.js（Rust host 零清洗，
//! `windows/tauri/src-tauri/src/terminal.rs`），由 xterm 解释 VT；本 crate 没有 VT，
//! 所以在这里把控制序列**丢弃**、把 `\r` `\b` 落实成字符级效果，剩下的就是可读的文本行。
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

use gpui_kit::SharedString;

/// 转义序列状态机的位置（跨 chunk 保留，见 `output-timestamper.ts:202-238`）。
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum EscapeState {
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
pub(crate) struct TerminalText {
    /// 本次 `feed` 里**刚完成**的行（每次取走，不长期堆积）。
    pub(crate) completed: Vec<String>,
    /// 当前还没换行的那一行。
    pub(crate) current: Vec<char>,
    /// 当前行的光标（字符下标）。
    pub(crate) cursor: usize,
    /// 转义序列状态（跨 chunk）。
    pub(crate) escape: EscapeState,
    /// 跨 chunk 的半个多字节字符。
    pub(crate) pending_bytes: Vec<u8>,
    /// 累计被 `from_utf8_lossy` 替换掉的字节数（非 UTF-8 输出的诊断依据）。
    pub(crate) replaced_bytes: usize,
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
    pub(crate) fn push_bytes(&mut self, bytes: &[u8]) {
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
    pub(crate) fn take_completed(&mut self) -> Vec<String> {
        std::mem::take(&mut self.completed)
    }

    /// 当前未完成行的文本（为空则 `None`）——它贴在输出区底部显示。
    pub(crate) fn current_text(&self) -> Option<SharedString> {
        if self.current.is_empty() {
            None
        } else {
            Some(SharedString::from(self.current.iter().collect::<String>()))
        }
    }

    /// 清空（「清除终端」）。
    pub(crate) fn clear(&mut self) {
        self.completed.clear();
        self.current.clear();
        self.cursor = 0;
    }

    /// 清洗后的字符流状态机。
    pub(crate) fn push_str(&mut self, text: &str) {
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
    pub(crate) fn put(&mut self, ch: char) {
        if self.cursor < self.current.len() {
            self.current[self.cursor] = ch;
        } else {
            self.current.push(ch);
        }
        self.cursor += 1;
    }

    /// 退格：删掉光标前一个字符并把光标前移。
    pub(crate) fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.current.remove(self.cursor);
        }
    }

    /// 换行：当前行封口。
    pub(crate) fn end_line(&mut self) {
        self.completed.push(self.current.iter().collect());
        self.current.clear();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalText;

    /// 纯函数测试：ANSI 清洗是终端能力里唯一不依赖进程与 GPUI 的一层，
    /// 所以在这里钉住它的行为（《编码指南》「测试策略」第 1 层 pure test）。
    #[test]
    fn drops_csi_sequences() {
        let mut text = TerminalText::default();
        text.push_bytes(b"\x1b[31mred\x1b[0m");
        text.push_bytes(b"\n");
        assert_eq!(text.take_completed(), vec!["red".to_string()]);
    }

    #[test]
    fn drops_osc_sequences() {
        let mut text = TerminalText::default();
        // OSC 以 BEL 结束。
        text.push_bytes(b"\x1b]0;title\x07body\n");
        assert_eq!(text.take_completed(), vec!["body".to_string()]);
    }

    #[test]
    fn carriage_return_overwrites_current_line() {
        let mut text = TerminalText::default();
        text.push_bytes(b"progress 10%");
        text.push_bytes(b"\rprogress 90%\n");
        assert_eq!(text.take_completed(), vec!["progress 90%".to_string()]);
    }

    #[test]
    fn backspace_removes_previous_character() {
        let mut text = TerminalText::default();
        text.push_bytes(b"abc\x08\n");
        assert_eq!(text.take_completed(), vec!["ab".to_string()]);
    }

    /// 多字节字符被 chunk 切断时不能产生假的 U+FFFD（跨块续上）。
    #[test]
    fn keeps_split_multibyte_character() {
        let mut text = TerminalText::default();
        let bytes = "终".as_bytes();
        text.push_bytes(&bytes[..1]);
        text.push_bytes(&bytes[1..]);
        text.push_bytes(b"\n");
        assert_eq!(text.take_completed(), vec!["终".to_string()]);
        assert_eq!(text.replaced_bytes, 0);
    }
}
