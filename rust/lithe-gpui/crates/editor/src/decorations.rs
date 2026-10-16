//! 内嵌提示与 Code Lens：把语言服务的只读注记交给组件库的装饰层。
//!
//! 分工与诊断同一条：**取数**在 `lithe-gpui-java`、**画**在组件库（`CodeEditor::set_inlay_hints`
//! 与 `set_code_lenses` 各收一份完整快照），本文件只做「UTF-16 行列 → 编辑器字节偏移」的换算
//! 与形状转换。
//!
//! 两者都是**整份替换**：正文一变，之前那些偏移就全过期了，所以宿主先清空、再等下一份
//! （与诊断同一条规矩，见 `diagnostics.rs` 的模块注释）。

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{BackgroundExecutor, Context};
use lithe_gpui_java::{JavaCodeLens, JavaInlayHint, JavaLanguageService};
use lithe_ui::editor::{CodeEditor, CodeLens, InlayHint};

use crate::navigation::offset_for_utf16;

pub(crate) const CHANGE_DEBOUNCE: Duration = Duration::from_millis(400);

/// 请求里要覆盖的行数。
///
/// 按**换行符个数 + 1** 算，而不是 `lines().count()`：末尾有换行时后者少一行，而 LSP 的 range
/// 是半开区间，少一行会把最后一行整个漏掉。这个算法只在末尾换行上分叉，也正是最容易被
/// 「顺手改成 `lines().count()`」重构掉的地方，所以单独留一个函数给它上测试。
pub(crate) fn line_count(text: &str) -> u32 {
    text.matches('\n').count() as u32 + 1
}

pub(crate) struct Fetched {
    pub(crate) hints: Vec<JavaInlayHint>,
    pub(crate) lenses: Vec<JavaCodeLens>,
    /// 取数失败的原因；两个请求里任一个失败都会留在这里，但**已取到的另一半照常展示**
    /// （Code Lens 挂在 JDT 上偶尔会超时，不该把内嵌提示一起拖掉）。
    pub(crate) error: Option<String>,
    pub(crate) elapsed_ms: u128,
}

pub(crate) async fn fetch(
    service: Arc<JavaLanguageService>,
    path: PathBuf,
    text: String,
    executor: BackgroundExecutor,
    debounce: Duration,
) -> Fetched {
    if !debounce.is_zero() {
        executor.timer(debounce).await;
    }
    let started = Instant::now();

    let lines = line_count(&text);

    let mut error = None;
    let hints = match service.inlay_hints(&path, &text, lines) {
        Ok(hints) => hints,
        Err(reason) => {
            error = Some(reason);
            Vec::new()
        }
    };
    let lenses = match service.code_lenses(&path, &text) {
        Ok(lenses) => lenses,
        Err(reason) => {
            if error.is_none() {
                error = Some(reason);
            }
            Vec::new()
        }
    };

    Fetched {
        hints,
        lenses,
        error,
        elapsed_ms: started.elapsed().as_millis(),
    }
}

/// 把一份快照交给编辑器（整份替换），返回实际落下去的内嵌提示与 Code Lens 条数。
///
/// `text` 必须是**与这份快照同一版本**的正文：偏移是拿它换算出来的，正文换过就该由调用方
/// 按 revision 拦下来。
pub(crate) fn apply(
    state: &mut CodeEditor,
    hints: &[JavaInlayHint],
    lenses: &[JavaCodeLens],
    text: &str,
    cx: &mut Context<CodeEditor>,
) -> (usize, usize) {
    let hints: Vec<InlayHint> = hints
        .iter()
        .map(|hint| InlayHint {
            offset: offset_for_utf16(text, hint.line as usize, hint.utf16_column as usize),
            text: hint.label.clone().into(),
        })
        .collect();
    // 组件库的 CodeLens 是一行一条、每条若干项；服务端给的是一行一条一个标题，
    // 所以这里每条合成只有一个 item。同一行有多条时**不合并**——行号重复时组件库按行取用，
    // 合并会把后一条的标题丢掉。
    let lenses: Vec<CodeLens> = lenses
        .iter()
        .map(|lens| CodeLens {
            line: lens.line as usize,
            items: vec![lens.title.clone().into()],
        })
        .collect();
    let counts = (hints.len(), lenses.len());
    state.set_inlay_hints(hints, cx);
    state.set_code_lenses(lenses, cx);
    counts
}

/// 清掉一份快照。正文一变就必须调用：旧偏移指向的位置已经没有意义了。
pub(crate) fn clear(state: &mut CodeEditor, cx: &mut Context<CodeEditor>) {
    state.set_inlay_hints(Vec::new(), cx);
    state.set_code_lenses(Vec::new(), cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 行数算法只在末尾换行上分叉，而那正是 LSP 半开区间最容易漏掉最后一行的地方。
    #[test]
    fn the_requested_line_count_never_drops_the_last_line() {
        assert_eq!(line_count("a"), 1);
        assert_eq!(line_count("a\n"), 2, "末尾换行意味着第 2 行存在（虽然为空）");
        assert_eq!(line_count("a\nb"), 2);
        assert_eq!(line_count("a\nb\n"), 3);
        assert_eq!(line_count(""), 1);
    }
}
