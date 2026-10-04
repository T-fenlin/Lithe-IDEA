use gpui_kit::SharedString;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum EscapeState {
    #[default]
    None,
    Esc,
    Csi,
    Osc,
    OscEsc,
}

#[derive(Default)]
pub(crate) struct TerminalText {
    pub(crate) completed: Vec<String>,
    pub(crate) current: Vec<char>,
    pub(crate) cursor: usize,
    pub(crate) escape: EscapeState,
    pub(crate) pending_bytes: Vec<u8>,
    pub(crate) replaced_bytes: usize,
}

impl TerminalText {
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
                        Some(len) => {
                            self.replaced_bytes += len;
                            self.push_str("\u{FFFD}");
                            self.pending_bytes = pending[valid + len..].to_vec();
                        }
                        None => {
                            self.pending_bytes = pending[valid..].to_vec();
                            break;
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn take_completed(&mut self) -> Vec<String> {
        std::mem::take(&mut self.completed)
    }

    pub(crate) fn current_text(&self) -> Option<SharedString> {
        if self.current.is_empty() {
            None
        } else {
            Some(SharedString::from(self.current.iter().collect::<String>()))
        }
    }

    pub(crate) fn clear(&mut self) {
        self.completed.clear();
        self.current.clear();
        self.cursor = 0;
    }

    pub(crate) fn push_str(&mut self, text: &str) {
        for ch in text.chars() {
            match self.escape {
                EscapeState::None => match ch {
                    '\u{1b}' => self.escape = EscapeState::Esc,
                    '\r' => self.cursor = 0,
                    '\n' => self.end_line(),
                    '\u{8}' | '\u{7f}' => self.backspace(),
                    '\t' => self.put(ch),
                    c if (c as u32) < 0x20 => {}
                    c => self.put(c),
                },
                EscapeState::Esc => {
                    self.escape = match ch {
                        '[' => EscapeState::Csi,
                        ']' => EscapeState::Osc,
                        _ => EscapeState::None,
                    };
                }
                EscapeState::Csi => {
                    let code = ch as u32;
                    if (0x40..=0x7e).contains(&code) || code > 0x7f {
                        self.escape = EscapeState::None;
                    }
                }
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

    pub(crate) fn put(&mut self, ch: char) {
        if self.cursor < self.current.len() {
            self.current[self.cursor] = ch;
        } else {
            self.current.push(ch);
        }
        self.cursor += 1;
    }

    pub(crate) fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.current.remove(self.cursor);
        }
    }

    pub(crate) fn end_line(&mut self) {
        self.completed.push(self.current.iter().collect());
        self.current.clear();
        self.cursor = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalText;

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
