mod buffer;
mod code_actions;
mod completion;
mod decorations;
mod diagnostics;
mod editor_view;
mod go_to_line;
mod hover;
pub mod local_history;
mod navigation;
mod problems;
mod symbol_actions;

pub use editor_view::{
    BufferWriteHost, CursorPosition, DebugGutterHost, EDITOR_HOST_ELEMENT_ID, EditorPane,
    FIND_BAR_KEY_CONTEXT, GutterBreakpoint, JavaServicePhase, SessionRestore, TabMenuHostActions,
};
pub use problems::{BuildProblem, ProblemsPane};

use gpui::{App, KeyBinding};

// 三个 Go-to 目标（Go 菜单）。只有「转到定义」绑了 `f12`；实现与类型定义今天**不绑键**，
// 只给菜单入口 —— 与文件生命周期五项同一口径（执行分支在宿主，先不铺键位面）。
gpui::actions!(
    lithe_editor,
    [
        SaveBuffer,
        NavigateToDefinition,
        NavigateToImplementation,
        NavigateToTypeDefinition,
        // 符号级动作（Go ▸ 转到引用 / 重命名符号，Edit ▸ 格式化文档）。
        // 只有「转到引用」绑键：真源自己写着「使用 Shift+F12 查找引用」
        // （`references.findReferencesHint`），不绑这条提示就是假的。另外两条与
        // 「转到实现 / 类型定义」同口径——只给菜单入口，不铺键位面。
        GoToReferences,
        RenameSymbol,
        FormatDocument,
        CloseActiveTab,
        ShowCodeActions
    ]
);

// 查找 / 替换的动作。声明在 editor（渲染方持有查找条与其 key context），workbench 绑键时引用。
// `Escape` 的 handler 挂在查找条容器上，`F3` / `Shift+F3` 用全窗绑定，所以四个动作都在这里。
gpui::actions!(
    lithe_editor,
    [OpenFind, OpenFindReplace, FindNext, FindPrevious, CloseFind]
);

pub fn install_actions(cx: &mut App) {
    // 字面量一律 `secondary-`：它是 gpui 的平台归一化开关（macOS → Command，其他 → Control），
    // 也是 `settings/src/keymap.rs::DEFAULT_BINDINGS` 已经假设的形态。旧写的 `ctrl-` 在 macOS 上
    // 让「设置页显示的（⌘S）」与「真能按出来的（Ctrl+S）」分成了两套（F2）。
    let mut bindings = vec![
        KeyBinding::new("secondary-s", SaveBuffer, None),
        KeyBinding::new("f12", NavigateToDefinition, None),
        KeyBinding::new("secondary-w", CloseActiveTab, None),
        // 代码动作：与上游 `gpui-base` 的 `ToggleCodeActions` 同一个键（`state.rs` 里
        // `secondary-.` 绑在编辑器的上下文上）。组件库没有"请求代码动作"的事件 ——
        // 它不认识语言服务 —— 所以这个入口由宿主自己绑、自己取数（见 `src/code_actions.rs`）。
        KeyBinding::new("secondary-.", ShowCodeActions, None),
        // 真源那句提示是「使用 Shift+F12 查找引用」，所以这一条必须有键才成立。
        KeyBinding::new("shift-f12", GoToReferences, None),
    ];
    if cfg!(target_os = "macos") {
        bindings.push(KeyBinding::new("cmd-.", ShowCodeActions, None));
    }
    cx.bind_keys(bindings);
}
