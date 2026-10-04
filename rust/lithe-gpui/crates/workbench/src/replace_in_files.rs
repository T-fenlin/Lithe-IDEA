//! Replace in Files 浮层（菜单「转到 → 在文件中替换」/ `Ctrl+Shift+R`）：
//! `workspace.replacePreview` 的**替换预览**。
//!
//! ## 这一件做了什么（U17 第四件，四件套的最后一件）
//!
//! 查询框 + 替换文本框 + 选项（区分大小写 / 全字匹配 / 正则 / 保留大小写 / 文件掩码）+
//! **按文件的 before / after 预览** + 四态（`idle` / `loading` / `ready` / `failed`）+
//! 底栏「预览」按钮与计数。纯逻辑（payload / 解析 / 分组 / 四态 / 代次）在
//! [`crate::search_model`]，本模块只做"把状态画出来"与"把事件接到 Core 上"。
//!
//! ## ⛔ 这一件**没有**做、而且在本分支**做不了**：应用替换
//!
//! 契约里与替换有关的命令**只有** `workspace.replacePreview` 一条
//! （`rust/lithe-core/src/protocol/command.rs:77-78,362`）——**没有** apply / replace 命令。
//! 所以"全部替换"在本分支**无法实现**，要等 main 那一侧补 Core 命令。宿主**不许**自己写盘：
//! 那会绕开 Core 的可见性 / 排除规则（`project/files.rs` 的 `VisibilityRules` 与
//! `parse_file_mask`），变成第二个真相源，而且 `file.write` 是"写一个整份文件"，
//! 拿它来拼替换等于把 Core 的匹配语义在宿主里再实现一遍。
//!
//! 界面上的表达是**不画那颗按钮** —— 真源同样没有：`ProjectReplaceView.swift:158-185`
//! 的底栏只有 `Button("Preview")` 与一行计数，"应用"在另一条通路
//! （`ProjectReplacementCoordinator.apply`，`macos/Sources/Lithe/Application/Features/
//! ProjectReplacementCoordinator.swift:60-81`）。本切片只做它面前那一段。
//!
//! ## 真源（`macos/Sources/Lithe/Views/Search/`）与规格出处
//!
//! | 规格 | 值 / 行为 | 出处 |
//! | --- | --- | --- |
//! | 真源是**浮动面板**，不是侧栏 | 工作台上的自绘模态层，居中 650×614、最小 520×400、可拖可缩放 | `ProjectReplaceFloatingPanel.swift:16-20,100`；`WorkbenchView.swift:589` |
//! | 头部行高 / 左右内衬 | 40 / 20 | `ProjectReplaceView.swift:96-97` |
//! | 标题 | 13 semibold「在项目中替换」/ "Replace in Files" | 同文件 `:74-75`；`macos/Resources/zh-Hans.lproj/Localizable.strings:65` |
//! | 控件区左右内衬 / 底部 / 间距 | 12 / 10 / 8 | 同文件 `:101,128,129` |
//! | 查找 + 替换两行输入 | 每个输入行里挂右侧选项开关 | 同文件 `:102-124` |
//! | 结果行高 | 46（Before / After 两行 + 行号） | 同文件 `:287,261-285` |
//! | 结果区左右内衬 | 8 | 同文件 `:225` |
//! | 底栏高 / 左右内衬 / 计数 | 52 / 20 /「N files, M matches」 | 同文件 `:177-178,172` |
//! | 预览按钮（真源的唯一主操作） | `Button("Preview")` + `isLoadingProjectReplacement` 时转 spinner | 同文件 `:158-176` |
//! | Esc 关闭 | `.onExitCommand(perform: close)` | 同文件 `:69` |
//! | 选项名 | Match Case / Whole Words / Regular Expression / Preserve Case | 同文件 `:108-123,387-394`；`Localizable.strings:1140-1142` |
//!
//! ## 为什么是浮层（而不是侧栏）
//!
//! 真源自己给了答案：`ProjectReplaceView` **只**由 `ProjectReplaceFloatingPanel` 包着用
//! （`WorkbenchView.swift:589` 的注释写明"Replace in Files 是工作台上的自绘模态层"），
//! 而 `SearchSidebarView` 是查找（Find in Files）的侧栏形态 —— 两者不是同一个形状。
//! 所以本模块照 [`crate::search_everywhere`] / [`crate::quick_open`] 的浮层写法
//! （`window.open_dialog` + gpui-kit `Dialog`），**不**挂进左活动栏。
//!
//! ## 复用（本任务最重要的工程要求）
//!
//! 1. **请求 / 解析 / 分组 / 代次一份都不用重写**：payload 是 `crate::search_model` 里
//!    新增的 [`replace_payload`]（字段名与"故意不传哪些"钉在同一模块的单测里），
//!    解析是 [`parse_replace_preview`]，四态是 [`ReplaceState`]，vintage 闸门与取消句柄是
//!    三个搜索界面共用的 [`QueryTimeline`] —— **本模块没有任何匹配 / 替换 / 排序逻辑**，
//!    那全在 Core 的 `project/files.rs:336-423`；
//! 2. **选项的四个布尔只是 `SearchOptions` 的转发**：`case_sensitive` / `whole_words` /
//!    `regular_expression` / `file_mask` 复用查找面板那几个字段，只有 `preserve_case`
//!    是替换独有（[`ReplaceOptions`]）。
//!
//! ## 与真源的有意差异（逐条）
//!
//! 1. **不画下半块的文件预览编辑器**：真源下半是 `ProjectReplacementSourcePreview`
//!    （`ProjectReplacementSourcePreview.swift:14-45` 挂 Monaco 编辑器 + 保存按钮 +
//!    查询高亮），gpui 侧没有对应的编辑器元素可嵌，也没有"未保存预览缓冲区"这条通路
//!    （`EditorPane` 只有真实文件缓冲区）。所以本切片画的是**上半块**（按文件的
//!    before / after 列表），并把 `ReplacementFile.replacementText` 整份文件正文
//!    **故意不解析**（见 `ReplacementFile` 的类型文档）；
//! 2. **不做查询高亮**：真源给 `before` 里的命中加黄底
//!    （`ProjectReplacementPreviewText.highlighted`，`:121-152`）。在宿主里给一段正文找
//!    命中等价于写**第二套匹配器**（大小写 / 全词 / 正则三档都要对），Core 的响应里
//!    也没有命中区间 —— 所以按"宿主不许自己算匹配"这条口径**不做**；
//! 3. **行不可点**：真源点一行只是把下面那块编辑器切过去（`:261-306` 的 `selectedResult`），
//!    本切片没有那块，所以行**不画点击态**（画了就成了"点了没反应"）。
//!    打开文件不在本切片范围；
//! 4. **`Preserve Case` 在正则打开时置灰**：真源是"勾了区分大小写就置灰"
//!    （`:119-123` 的 `isEnabled: !options.caseSensitive`），而 Core 的实际语义是
//!    **正则那一支直接返回、根本不看 `preserveCase`**（`project/files.rs:890-901`：`regex`
//!    存在时只走 `replace_all`）—— 所以本侧按 **Core 的实际语义**置灰，
//!    这是有意的偏离：置灰的判据要能被 Core 的行为解释；
//! 5. **不传 `paths` / `textOverrides`**：真源把"编辑器里未保存的缓冲区"一起送进 Core
//!    （`AppModel+SearchModule.swift:53-58`），gpui 宿主没有这条通路，
//!    所以预览的是**磁盘上的内容**（理由写在 [`replace_payload`] 的文档上）；
//! 6. **面板不可拖 / 不可缩放**：真源的浮动面板能拖动与四角缩放
//!    （`ProjectReplaceFloatingPanel.swift:23-53`），而 gpui-kit 的 `Dialog` 没有这两个交互
//!    （它是模态层，位置由 `width` / `margin_top` 决定）。本侧居中显示，尺寸照真源的 650×614。
//!
//! ## 浮层只能在事件回调或任务里打开
//!
//! [`open_dialog`] 里的 `window.open_dialog` **只能**从事件回调 / 任务里调
//! （`render` 阶段调会 panic）—— 所以入口是菜单那条事件回调、`Ctrl+Shift+R` 的全局
//! action 处理器。本模块的 `Render` 实现**只读**状态：不 mutate、不 notify、不起任务。
//!
//! ## 长操作的三件事：`operationID` / 取消 / stale 保护
//!
//! 与 [`crate::search_panel`] / [`crate::quick_open`] 完全同一条：
//!
//! 1. **`operationId`**：每次预览由 [`QueryTimeline::schedule`] 按代次生成
//!    （`lithe-gpui-search-<n>`），经 `CoreRequest::with_operation_id` 进信封；
//! 2. **取消**：换代时把上一条在飞请求的 `operationId` 交给
//!    `lithe_gpui_shared::cancel_operation`（**在后台任务里调**），Core 的协作式取消
//!    （`protocol/cancellation.rs` 的 `check()`）在 `replace_preview` 的逐文件 / 逐行循环里
//!    被调（`project/files.rs:385,400`）；
//! 3. **stale 保护**：结果回来先过代次闸门（[`QueryTimeline::accepts`]），旧代次一律丢弃
//!    并留一行 `stale=1`。**这一条不依赖"取消真的生效"**。
//!
//! ## 没有防抖，因为真源是显式「预览」
//!
//! 查找面板与两个浮层都是"输入停 N 毫秒就发一次"，而真源的替换面板要**点「预览」**
//! （`ProjectReplaceView.swift:160-165`），输入一变只是把上一次预览**作废**
//! （`:57-68` 的 `onChange { clearPreview() }`）。所以本模块**没有** `debounce_task`：
//! 只有「预览」按钮与两个输入框里的回车会真的发 Core 调用。这条差异让"一次查询可能扫
//! 整个工作区"（契约里没有 `maxResults`）只在用户明确要求时发生。
//!
//! ## 诊断（stderr）
//!
//! `S1_REPLACE_PREVIEW open|close|idle|issue|complete|stale|failed|cancel|option <fields>`。
//! 与 `S1_SEARCH` / `S1_QUICK_OPEN` / `S1_SEARCH_EVERYWHERE` 同族：浮层与键盘路径在本仓库
//! **没有**确定性单测（要真窗口 + 真键事件，而 `ShellWorkspace::new` 会起 JDTLS 预备 /
//! 会话恢复 / 索引 warm），所以交互路径的机器证据只有这些锚点；能确定性验证的部分全在
//! `crate::search_model` 的纯函数单测与本模块末尾的两个纯谓词单测里。

use std::path::PathBuf;

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Selectable as _, Sizable as _,
    StyledExt as _, WindowExt as _,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, AppContext as _, Context, ElementId, Entity, Focusable as _, Hsla,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render, SharedString,
    Styled as _, Subscription, Window, div, rems,
};
use lithe_gpui_shared::icons::{idea, idea_icon_image};
use lithe_gpui_shared::{CoreClient, CoreError, CoreRequest, tr, tr_args};
use serde_json::Value;

use crate::search_index::SearchIndexReadiness;
use crate::search_model::{
    QueryTimeline, REPLACE_PREVIEW_COMMAND, ReplaceOptions, ReplaceRow, ReplaceState,
    parse_replace_preview, replace_payload,
};

/// 诊断前缀（`eprintln!`，stderr）。与 `S1_SEARCH` 一族同口径。
const TAG: &str = "S1_REPLACE_PREVIEW";

/// 浮层宽 650、高 614：`ProjectReplaceFloatingPanel.swift:16-20` 的默认 frame。
///
/// `Dialog::width` 是**固有方法**、只吃 `Pixels`，所以调用点按当帧 rem 基准换算
/// （写死就是假 rem，见 `crate::rem_px`）。
const PANEL_WIDTH: f32 = 650.;
const PANEL_HEIGHT: f32 = 614.;

/// 结果行高 46：`ProjectReplaceView.swift:287` 的 `.frame(height: 46)`。
/// 46 不在 gpui 的档位上（档位是 …44 / 48…），所以写 `rems(46. / 16.)`。
const ROW_HEIGHT_SPEC: f32 = 46.;
/// 行内文件图标边长 14：同文件 `:502-505`；与 `search_everywhere` / `search_panel` 同值。
const ROW_ICON_SPEC: f32 = 14.;
/// 底栏高 52：同文件 `:178`。52 不在档位上（…48 / 64…）⇒ `rems(52. / 16.)`。
const FOOTER_HEIGHT_SPEC: f32 = 52.;

/// 浮层根元素 id（业务身份，不是下标）。
const OVERLAY_ID: &str = "lithe-replace-in-files";
/// 结果列表的元素 id。
const LIST_ID: &str = "lithe-replace-preview-results";
/// 「预览」按钮的元素 id。
const PREVIEW_BUTTON_ID: &str = "lithe-replace-preview-run";

/// `Ctrl+Shift+R`：真源的 Windows 平台键位就是它
/// （`macos/Sources/Lithe/Models/Keymap/LitheCommandCatalog.swift:61` 的
/// `"replace-in-project": shortcut("r", [.control, .shift])`；macOS 那一侧是 `:99` 的
/// `[.command, .shift]`）。本仓 `mod` → `ctrl` 的既有口径下两者是同一个键。
pub const REPLACE_KEY: &str = "ctrl-shift-r";

/// 登记 `Ctrl+Shift+R`（`ShellWorkspace::new` 调一次）。
///
/// 处理器里的三段与 [`crate::quick_open::install_actions`] 逐字同源：拿活动窗口 →
/// 拿外壳句柄 → 调外壳的 [`crate::workspace::ShellWorkspace::open_replace_in_files`]。
/// 菜单那一项经 `CommandId::OpenReplaceInFiles` 落到**同一个方法**
/// ——"一个逻辑命令只建模一次"。
pub fn install_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new(REPLACE_KEY, OpenReplaceInFiles, None)]);
    cx.on_action(|_: &OpenReplaceInFiles, cx: &mut App| {
        let Some(window_handle) = cx.active_window() else {
            return;
        };
        let _ = window_handle.update(cx, |_, window, cx| {
            let Some(shell) = crate::command_palette::shell().and_then(|shell| shell.upgrade())
            else {
                return;
            };
            let _ = shell.update(cx, |shell, cx| shell.open_replace_in_files(window, cx));
        });
    });
}

gpui_kit::actions!(lithe_workbench, [OpenReplaceInFiles]);

/// 打开浮层（**只能从事件回调或任务里调用**；`render` 阶段调 `window.open_dialog` 会 panic）。
///
/// 已经开着别的对话框时不叠第二层 —— 与 `open_command_palette` / `open_quick_open` /
/// `open_search_everywhere` / `open_branch_panel` 同一条口径。
pub(crate) fn open_dialog(panel: Entity<ReplaceInFiles>, window: &mut Window, cx: &mut App) {
    if window.has_active_dialog(cx) {
        ReplaceInFiles::diagnose("open", "false reason=active_dialog");
        return;
    }
    // 三个入口（菜单「转到 → 在文件中替换」、`Ctrl+Shift+R`）都只到这一个开浮层的点，
    // 所以这一行就是"浮层真的被要求打开"的锚点。
    ReplaceInFiles::diagnose("open", "true");
    let handle = panel.clone();
    window.open_dialog(cx, move |dialog, window, _cx| {
        // 650×614 与居中：`Dialog` 的 y 默认是视口的 1/10，本侧照真源的"居中"排
        // （`ProjectReplaceFloatingPanel.swift:54` 的 `.position(x: midX, y: midY)`）。
        // 高度还要夹进视口（真源也有 `constrained`，同文件 `:102-110`）。
        let rem = window.rem_size();
        let viewport = window.viewport_size();
        let height = crate::rem_px(rem, PANEL_HEIGHT)
            .min((viewport.height - rem * 2.).max(gpui_kit::Pixels::ZERO));
        let margin_top = ((viewport.height - height) / 2.).max(gpui_kit::Pixels::ZERO);
        dialog
            .close_button(false)
            .overlay(true)
            .overlay_closable(true)
            .keyboard(true)
            .width(crate::rem_px(rem, PANEL_WIDTH))
            .h(height)
            .margin_top(margin_top)
            .p_0()
            // 关闭的唯一收尾点：`Esc` / 点遮罩都经过它 —— 在飞的 Core 预览在这里取消。
            .on_close({
                let handle = handle.clone();
                move |_event, window, cx| {
                    let _ = handle.update(cx, |panel, cx| panel.close(window, cx));
                }
            })
            .content({
                let handle = handle.clone();
                move |content, _, _| content.p_0().gap_0().h_full().child(handle.clone())
            })
    });
    // ⚠️ 焦点必须在 `open_dialog` **之后**要一次：它**无条件**把焦点交给 `Dialog` 自己
    // （`gpui-component-0.7.0/src/root.rs` 的 `open_dialog` 里 `focus_handle.focus(window, cx)`），
    // 而不是"只在没人持有焦点时才给"。没有焦点就没有输入处理器，查询框看得见却打不进字。
    let _ = panel.update(cx, |panel, cx| panel.focus_query(window, cx));
}

/// 一个替换预览的选项（真源 `ProjectReplaceOptionButton` 的四颗开关，
/// `ProjectReplaceView.swift:108-123`）。
///
/// 做成 enum 而不是"四处字符串 match"：`id` / 文案键 / 取哪一格 / 写哪一格 /
/// 什么时候置灰**只有这一份**，表里加一档忘了改 match 会在编译期红
/// （与 `crate::search_panel::QueryOption` 同一条取舍）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReplaceOption {
    /// `caseSensitive`。
    CaseSensitive,
    /// `wholeWords`。
    WholeWords,
    /// `regularExpression`。
    RegularExpression,
    /// `preserveCase`（[`ReplaceOptions::preserve_case`]，替换独有）。
    PreserveCase,
}

impl ReplaceOption {
    /// 全部四档。**只有单测用**：它守"`id` 互不相同 + 每一档都能读能写"；
    /// 界面上的分组由 [`Self::QUERY_ROW`] 与 `PreserveCase` 决定（真源就是这么分的，
    /// `ProjectReplaceView.swift:108-124`），所以生产构建里不需要这份完整清单。
    #[cfg(test)]
    const ALL: [ReplaceOption; 4] = [
        Self::CaseSensitive,
        Self::WholeWords,
        Self::RegularExpression,
        Self::PreserveCase,
    ];

    /// 查找框那一行的三颗（真源挂在 Find 输入行右侧，`:108-111`）。
    const QUERY_ROW: [ReplaceOption; 3] = [
        Self::CaseSensitive,
        Self::WholeWords,
        Self::RegularExpression,
    ];

    /// 元素 id 的片段（`ElementId` 由它拼出来，所以必须互不相同）。
    fn id(self) -> &'static str {
        match self {
            Self::CaseSensitive => "case-sensitive",
            Self::WholeWords => "whole-words",
            Self::RegularExpression => "regular-expression",
            Self::PreserveCase => "preserve-case",
        }
    }

    /// 文案键。
    ///
    /// 前三颗**复用**查找面板那几个真源既有键（`locales/source/locale.ts:1484-1486`
    /// 的 `search.matchCase` / `search.matchWholeWord` / `search.useRegex`）；
    /// `Preserve Case` 真源的中文只存在于 macOS 的 SwiftUI 字面量里
    /// （`ProjectReplaceView.swift:392`），catalog 里没有，所以是 gpui 自有键。
    fn label_key(self) -> &'static str {
        match self {
            Self::CaseSensitive => "lithe.search.matchCase",
            Self::WholeWords => "lithe.search.matchWholeWord",
            Self::RegularExpression => "lithe.search.useRegex",
            Self::PreserveCase => "lithe.search.gpui.preserveCase",
        }
    }

    /// 读这一格。
    fn get(self, options: &ReplaceOptions) -> bool {
        match self {
            Self::CaseSensitive => options.search.case_sensitive,
            Self::WholeWords => options.search.whole_words,
            Self::RegularExpression => options.search.regular_expression,
            Self::PreserveCase => options.preserve_case,
        }
    }

    /// 写这一格；返回**是否真的变了**（没变就不用作废旧预览）。
    fn set(self, options: &mut ReplaceOptions, value: bool) -> bool {
        let slot = match self {
            Self::CaseSensitive => &mut options.search.case_sensitive,
            Self::WholeWords => &mut options.search.whole_words,
            Self::RegularExpression => &mut options.search.regular_expression,
            Self::PreserveCase => &mut options.preserve_case,
        };
        let changed = *slot != value;
        *slot = value;
        changed
    }

    /// 这一颗今天能不能拨。
    ///
    /// **只有 `Preserve Case` 会被置灰，判据来自 Core 而不是真源**：正则那一支
    /// `replace_with_options` 直接 `return`（`project/files.rs:890-901`），`preserveCase`
    /// 传进去也不会被读 —— 画一颗"拨了不影响任何东西"的开关就是骗人。
    /// 真源置灰的是"勾了区分大小写"，而 Core 在大小写敏感时**照样**应用
    /// `apply_case_pattern`（`:937-941`），所以那一档本侧**不**置灰（有意差异，见模块文档第 4 条）。
    fn enabled(self, options: &ReplaceOptions) -> bool {
        match self {
            Self::PreserveCase => !options.search.regular_expression,
            Self::CaseSensitive | Self::WholeWords | Self::RegularExpression => true,
        }
    }
}

/// 浮层的交互状态。
///
/// 与 `QuickOpen` / `SearchEverywhere` 同一口径：`open_dialog` 的 builder **每帧重跑**，
/// 状态必须存在 `Entity` 里。这里持有三个输入框（查找 / 替换 / 文件掩码）、
/// 拼好的选项、四态与代次。
pub struct ReplaceInFiles {
    /// 工作区根：Core 的 `root`。
    root: PathBuf,
    /// 查找框。
    query: Entity<InputState>,
    /// 替换文本框。
    replacement: Entity<InputState>,
    /// 文件掩码框（Core 的 `fileMask`，逗号分隔 glob）。
    mask: Entity<InputState>,
    /// 三个框的当前值。`Input` 没有 `on_change`，只能听 `InputEvent::Change` 各存一份
    /// （与 `crate::search_panel` 同一做法）。
    query_text: SharedString,
    replacement_text: SharedString,
    mask_text: SharedString,
    /// 五个选项拼成 Core 的请求选项。
    options: ReplaceOptions,
    /// 四态状态机。
    state: ReplaceState,
    /// 代次与在飞 `operationId`（stale 保护 + 取消句柄），四个搜索界面共用同一个类型。
    timeline: QueryTimeline,
    /// 输入框订阅。**必须持有**，掉了就取消订阅。
    _subscriptions: Vec<Subscription>,
    /// 搜索索引的就绪状态（`U18` 收尾）。**只读的展示值**：owner 是 `crate::search_index`，
    /// 浮层在**打开时**由外壳读一次推进来（构造参数），之后由外壳在状态变化时推
    /// （[`Self::set_search_index_readiness`]）。浮层不据此禁用 / 排队 / 改预览语义。
    search_index_readiness: SearchIndexReadiness,
}

impl ReplaceInFiles {
    /// 建浮层状态（`open_replace_in_files` 那条回调里建一次，之后由 `Entity` 跨帧持有）。
    ///
    /// `search_index_readiness`：**打开这一刻**的索引就绪状态（外壳从 `search_index` 读，
    /// 唯一 owner 是它）。
    pub(crate) fn new(
        root: PathBuf,
        search_index_readiness: SearchIndexReadiness,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let query = cx.new(|cx| {
            InputState::new(window, cx).placeholder(tr("lithe.search.gpui.queryPlaceholder"))
        });
        let replacement =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("lithe.search.replaceWith")));
        let mask =
            cx.new(|cx| InputState::new(window, cx).placeholder(tr("lithe.search.filesToInclude")));

        // 三个框共用一个处理器：它一次读三个框的当前值，所以"改了哪个"不影响结果；
        // 回车也一样（在哪个框里回车都立即预览）。
        let subscribe = |state: &Entity<InputState>, cx: &mut Context<Self>| {
            cx.subscribe_in(
                state,
                window,
                |this: &mut Self, _state: &Entity<InputState>, event: &InputEvent, _, cx| {
                    this.on_input_event(event, cx);
                },
            )
        };
        let subscriptions = vec![
            subscribe(&query, cx),
            subscribe(&replacement, cx),
            subscribe(&mask, cx),
        ];

        Self {
            root,
            query,
            replacement,
            mask,
            query_text: SharedString::default(),
            replacement_text: SharedString::default(),
            mask_text: SharedString::default(),
            options: ReplaceOptions::default(),
            state: ReplaceState::Idle,
            timeline: QueryTimeline::default(),
            _subscriptions: subscriptions,
            search_index_readiness,
        }
    }

    /// 外壳推来一个新的索引就绪状态（**只在真的变了时** `cx.notify()`）。
    pub(crate) fn set_search_index_readiness(
        &mut self,
        readiness: SearchIndexReadiness,
        cx: &mut Context<Self>,
    ) {
        if self.search_index_readiness == readiness {
            return;
        }
        self.search_index_readiness = readiness;
        cx.notify();
    }

    /// 把焦点交给查找框。`open_dialog` 之后必须调一次（理由见 [`open_dialog`] 的注释）。
    fn focus_query(&self, window: &mut Window, cx: &mut App) {
        self.query.read(cx).focus_handle(cx).focus(window, cx);
    }

    /// 输入事件（三个框共用）。
    fn on_input_event(&mut self, event: &InputEvent, cx: &mut Context<Self>) {
        match event {
            InputEvent::Change => {
                self.query_text = self.query.read(cx).value();
                self.replacement_text = self.replacement.read(cx).value();
                self.mask_text = self.mask.read(cx).value();
                self.options.search.file_mask = self.mask_text.to_string();
                // 输入一变，上一次的预览**立刻作废**：真源同一句
                // （`ProjectReplaceView.swift:57-68` 的 `onChange { clearPreview() }`）。
                // 留着上一次的 before/after 会让人以为那是新输入的结果。
                self.discard_preview(cx);
            }
            // 回车 = "就是现在"：真源没有这一条（它要点「预览」），但按了没反应是更差的手感。
            InputEvent::PressEnter { .. } => self.issue(cx),
            InputEvent::Focus | InputEvent::Blur => {}
        }
    }

    /// 作废当前预览（输入变了 / 选项变了）：换代 + 尽力取消，状态回到 `Idle`。
    fn discard_preview(&mut self, cx: &mut Context<Self>) {
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        if !matches!(self.state, ReplaceState::Idle) {
            self.state = ReplaceState::Idle;
            cx.notify();
        }
    }

    /// 真的发一次预览（点「预览」/ 回车）。
    ///
    /// 与另外三个搜索界面的差别只有一处：**没有防抖**，因为这个入口是显式按钮
    /// （理由见模块文档）。
    fn issue(&mut self, cx: &mut Context<Self>) {
        let query = self.query_text.trim().to_string();
        if query.is_empty() {
            // 空查询**不发 Core 调用**：`replace_preview` 对空查询直接回空
            // （`project/files.rs:341-344`），发过去只是白跑一趟信封。
            self.state = ReplaceState::Idle;
            Self::diagnose("idle", "query=empty");
            cx.notify();
            return;
        }
        let scheduled = self.timeline.schedule();
        if let Some(superseded) = scheduled.superseded.clone() {
            Self::cancel_in_background(superseded, cx);
        }
        let generation = scheduled.generation;
        let operation_id = scheduled.operation_id.clone();
        let root = self.root.to_string_lossy().to_string();
        let replacement = self.replacement_text.to_string();
        let options = self.options.clone();
        let previous = self.state.files().to_vec();
        self.state = ReplaceState::loading(query.clone(), &previous);
        Self::diagnose(
            "issue",
            &format!(
                "generation={generation} operation={operation_id} query={query} \
                 replacement_len={} case={} whole={} regex={} preserve={} mask={}",
                replacement.len(),
                options.search.case_sensitive,
                options.search.whole_words,
                options.search.regular_expression,
                options.preserve_case,
                options.search.file_mask,
            ),
        );
        cx.notify();

        // 同步的 Core 调用只在 `background_spawn` 里跑（UI 线程不做阻塞工作）。
        // 任务 detach 掉：代次闸门才是采纳判据，外壳被换掉时 `WeakEntity::update` 自然失败。
        let background_query = query.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move {
                    let request = CoreRequest::command(REPLACE_PREVIEW_COMMAND)
                        .with_payload(replace_payload(
                            &root,
                            &background_query,
                            &replacement,
                            &options,
                        ))
                        .with_operation_id(operation_id);
                    CoreClient::new().execute(&request)
                })
                .await;
            let _ = this.update(cx, |panel, cx| panel.adopt(generation, &query, result, cx));
        })
        .detach();
    }

    /// 后台结果回到前台：**先过代次闸门**，再进 `ready` / `failed`。
    fn adopt(
        &mut self,
        generation: u64,
        query: &str,
        result: Result<Option<Value>, CoreError>,
        cx: &mut Context<Self>,
    ) {
        if !self.timeline.accepts(generation) {
            // 过期结果：**不采纳**（契约要求），也不改任何状态。留一行证据。
            Self::diagnose(
                "stale",
                &format!(
                    "generation={generation} current_query_len={}",
                    self.query_text.len()
                ),
            );
            return;
        }
        let next = match result {
            Ok(data) => match parse_replace_preview(data.as_ref()) {
                Ok(files) => ReplaceState::ready(query, &files),
                // 契约异常（缺 data / 形状不对）：给稳定 token，不给句子 ——
                // 宿主不自己编界面文案（见 `search_model::ParseError`）。
                Err(error) => ReplaceState::failed(query, error.token()),
            },
            // Core 的失败原文是**数据**：`CoreError` 的 Display 给 `code: message`。
            Err(error) => ReplaceState::failed(query, error.to_string()),
        };
        let (files, occurrences) = next.counts();
        match &next {
            ReplaceState::Failed { reason, .. } => Self::diagnose(
                "failed",
                &format!("generation={generation} query={query} reason={reason}"),
            ),
            ReplaceState::Ready { .. } => Self::diagnose(
                "complete",
                &format!(
                    "generation={generation} query={query} files={files} matches={occurrences} state={}",
                    // 空结果与"有命中"必须是两个可区分的状态词（契约要求分开）：
                    // 0 个文件 = `empty`，否则 `ready`。
                    if files == 0 { "empty" } else { "ready" }
                ),
            ),
            // `Loading` / `Idle` 不会从这条路径产生：`issue` 先置 `Loading`，
            // 这里只会把它推进到 `Ready` 或 `Failed`。
            _ => Self::diagnose("unexpected_state", &format!("generation={generation}")),
        }
        self.state = next;
        self.timeline.settle(generation);
        cx.notify();
    }

    /// 把一颗选项拨到 `value`；变了就作废旧预览（**不自动重发**，见模块文档）。
    fn set_option(&mut self, option: ReplaceOption, value: bool, cx: &mut Context<Self>) {
        if !option.enabled(&self.options) {
            // 置灰的那一颗：gpui 的 `Button::disabled` 已经不会派发点击，这里是第二道保险。
            Self::diagnose("option", &format!("id={} ignored=disabled", option.id()));
            return;
        }
        if !option.set(&mut self.options, value) {
            // 值没变（gpui 可能重复发同一个值）：不重搜。
            return;
        }
        Self::diagnose("option", &format!("id={} value={value}", option.id()));
        self.discard_preview(cx);
    }

    /// 浮层关闭（`Esc` / 点遮罩的唯一收尾点）：在飞的预览取消掉。
    fn close(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(superseded) = self.timeline.abandon() {
            Self::cancel_in_background(superseded, cx);
        }
        Self::diagnose("close", "true");
    }

    /// 把 `operationId` 交给后台取消。
    ///
    /// **不在 UI 线程调**：它要拿 Core 的操作注册表锁（`cancellation.rs:84`）。
    fn cancel_in_background(operation_id: String, cx: &mut Context<Self>) {
        cx.background_spawn(async move {
            let requested = lithe_gpui_shared::cancel_operation(&operation_id);
            // `false` = 这个 id 没有在飞的操作（已经跑完了）—— 正常情况，不是失败。
            Self::diagnose(
                "cancel",
                &format!("operation={operation_id} requested={requested}"),
            );
        })
        .detach();
    }

    /// 一行 stderr 诊断（`S1_REPLACE_PREVIEW <event> <fields>`）。
    fn diagnose(event: &str, fields: &str) {
        eprintln!("{TAG} {event} {fields}");
    }

    // ---- 渲染 ----

    /// 头部：标题 + 文件掩码框。`flex_shrink_0`：它不参与结果区的伸缩。
    fn header(&self, cx: &App) -> AnyElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            // 高 40（`ProjectReplaceView.swift:97`）恰是档位内的 `h_10()`；左右内衬 20 ⇒ `px_5()`。
            .h_10()
            .px_5()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .text_color(cx.theme().foreground)
                    .child(tr("lithe.search.gpui.replaceInFiles")),
            )
            .child(div().flex_1())
            // 掩码框：真源有一颗 "File mask:" 勾选框 + 96 宽输入框（`:79-93`）。
            // 本侧**不画那颗勾选框**：Core 里空掩码就是"不过滤"（`file_mask_allows` 对空
            // 掩码表恒真），勾选框只是把一份非空掩码临时关掉 —— 清空输入框是同一个效果，
            // 少一个会漂移的状态。placeholder 复用查找面板的 `search.filesToInclude`。
            .child(
                Input::new(&self.mask)
                    .small()
                    .cleanable(true)
                    .w(rems(96. / 16.))
                    .aria_label(tr("lithe.search.filesToInclude")),
            )
            .into_any_element()
    }

    /// 控件区：查找框（右侧三颗选项）+ 替换框（右侧一颗）。
    fn controls(&self, cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w_full()
            .flex_shrink_0()
            .gap_2()
            .px_3()
            // 底部 10（`:129` 的 `.padding(.bottom, 10)`）恰是档位内的 `pb_2p5()`。
            .pb_2p5()
            .child(
                Input::new(&self.query)
                    .small()
                    .cleanable(true)
                    .w_full()
                    .prefix(idea_icon_image(
                        &idea::MAGNIFYING_GLASS_ICON,
                        cx,
                        rems(ROW_ICON_SPEC / 16.),
                    ))
                    // 无障碍名：`placeholder` 不是可访问名（与查找面板同一条）。
                    .aria_label(tr("lithe.search.gpui.queryPlaceholder")),
            )
            .child(self.option_row(ReplaceOption::QUERY_ROW.as_slice(), cx))
            .child(
                Input::new(&self.replacement)
                    .small()
                    .cleanable(true)
                    .w_full()
                    .aria_label(tr("lithe.search.replaceWith")),
            )
            .child(self.option_row(&[ReplaceOption::PreserveCase], cx))
            .into_any_element()
    }

    /// 一行选项开关（真源 `ProjectReplaceOptionButton`：选中有底色、置灰半透明）。
    fn option_row(&self, options: &[ReplaceOption], cx: &mut Context<Self>) -> AnyElement {
        let mut row = h_flex().w_full().flex_shrink_0().gap_1();
        for option in options {
            let option = *option;
            row = row.child(
                // ⚠️ `.small()` + `.selected(..)` + `.disabled(..)` 就是真源那颗开关的语义
                // （`:359-395`：`isOn` 决定底色、`isEnabled` 决定能不能拨）。
                // `Button::disabled` 是上游 `Disableable` 的实现，真的不派发点击。
                Button::new(ElementId::from(format!(
                    "lithe-replace-option-{}",
                    option.id()
                )))
                .small()
                .selected(option.get(&self.options))
                .disabled(!option.enabled(&self.options))
                .label(tr(option.label_key()))
                .on_click(cx.listener(move |this, _, _, cx| {
                    let next = !option.get(&this.options);
                    this.set_option(option, next, cx);
                })),
            );
        }
        row.into_any_element()
    }

    /// 结果区：四态各画各的。
    ///
    /// 判据只看 [`ReplaceState`]，**不重算任何东西**（`render` 里不做匹配 / 替换 / 排序，
    /// 那是 Core 与 `search_model` 的事；before/after 逐字来自 Core 的响应）。
    fn results(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match &self.state {
            ReplaceState::Idle => Self::empty_state(
                Some(IconName::Search),
                tr("lithe.search.gpui.replaceInFiles"),
                Some(tr("lithe.search.gpui.replaceEmptyDescription")),
                cx,
            ),
            ReplaceState::Loading { .. } => h_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .justify_center()
                .gap_2()
                .child(Spinner::new().color(cx.theme().muted_foreground))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.fileExplorer.searchingFiles")),
                )
                .into_any_element(),
            // 失败：标题一句固定的、原因一行 Core 给的原文（或契约异常的稳定 token）。
            // **空结果不走这一支** —— 两者必须分得开。
            ReplaceState::Failed { reason, .. } => Self::empty_state(
                Some(IconName::TriangleAlert),
                tr("lithe.search.failed"),
                Some(SharedString::from(reason.clone())),
                cx,
            ),
            // 空结果 = `Ready` 但一个文件都没有：**不是失败**，标题带上查询串
            // （真源 `search.noResultsFor` 的 `{query}` 占位符）。
            ReplaceState::Ready { files, .. } if files.is_empty() => Self::empty_state(
                None,
                tr_args(
                    "lithe.search.noResultsFor",
                    &[("query", self.state.query())],
                ),
                None,
                cx,
            ),
            ReplaceState::Ready { .. } => {
                let rows = self.state.rows();
                self.list(&rows, cx)
            }
        }
    }

    /// 结果列表：文件头 + before/after 行，可滚动。
    ///
    /// ⚠️ 不用 gpui-kit 的 `List`：它要求**所有行等高**，而文件头与 before/after 行高度不同
    /// （与 `crate::search_panel` 同一条取舍）。
    fn list(&self, rows: &[ReplaceRow], cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .id(LIST_ID)
            .w_full()
            .flex_1()
            .min_h_0()
            .gap_0()
            .overflow_y_scrollbar()
            .children(rows.iter().map(|row| self.row(row, cx)))
            .into_any_element()
    }

    /// 列表里的一行。
    ///
    /// ⚠️ 元素 id 用**业务身份**（路径 / 路径 + 行号），不用下标：下标会在结果变化时把
    /// 状态串到别的行上。
    ///
    /// 两行都**不可点**：真源点一行只是把下半块的编辑器切过去
    /// （`ProjectReplaceView.swift:261-266`），本切片没有那块（见模块文档第 3 条），
    /// 所以不画点击态 —— 画一个点了没反应的行比不画更糟。
    fn row(&self, row: &ReplaceRow, cx: &App) -> AnyElement {
        match row {
            // 文件头：路径（工作区相对、`/` 分隔）+ 这个文件里会被替换几处。
            ReplaceRow::File { path, occurrences } => h_flex()
                .id(ElementId::from(format!("replace-file-{path}")))
                .w_full()
                .gap_2()
                .px_2()
                .py_1()
                .child(idea_icon_image(
                    &idea::FILE_ICON,
                    cx,
                    rems(ROW_ICON_SPEC / 16.),
                ))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_xs()
                        .font_medium()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_color(cx.theme().foreground)
                        .child(SharedString::from(path.clone())),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        // 次数是数字（数据，不是文案），不走 `tr`。
                        .child(SharedString::from(occurrences.to_string())),
                )
                .into_any_element(),
            // 一行 before / after：行号 + 两行等宽正文。
            ReplaceRow::Line {
                path,
                line,
                before,
                after,
                ..
            } => h_flex()
                .id(ElementId::from(format!("replace-line-{path}-{line}")))
                .w_full()
                // 高 46（`ProjectReplaceView.swift:287`）不在档位上 ⇒ `rems(46. / 16.)`。
                .h(rems(ROW_HEIGHT_SPEC / 16.))
                .gap_2()
                .pl_4()
                .pr_2()
                .py_1()
                .child(
                    div()
                        .w_8()
                        .flex_shrink_0()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        // 行号是数据（1 基），不是文案。
                        .child(SharedString::from(line.to_string())),
                )
                .child(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap_0()
                        .child(Self::text_pair(
                            tr("lithe.search.gpui.before"),
                            before,
                            cx.theme().muted_foreground,
                            cx,
                        ))
                        .child(Self::text_pair(
                            tr("lithe.search.gpui.after"),
                            after,
                            cx.theme().foreground,
                            cx,
                        )),
                )
                .into_any_element(),
        }
    }

    /// before / after 各一行：固定宽度的标签 + 等宽正文（正文超长就截断）。
    ///
    /// **没有查询高亮**：那需要第二套匹配器（见模块文档第 2 条）。
    fn text_pair(label: SharedString, text: &str, color: Hsla, cx: &App) -> AnyElement {
        h_flex()
            .w_full()
            .min_w_0()
            .gap_2()
            .child(
                div()
                    .w(rems(48. / 16.))
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(label),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    // 真源的预览是等宽 12px（`ProjectReplaceView.swift:276`），
                    // `text_xs()` = 12px，字族走主题的等宽族。
                    .font_family(cx.theme().mono_font_family.clone())
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_color(color)
                    .child(SharedString::from(text.to_string())),
            )
            .into_any_element()
    }

    /// 底栏：「预览」按钮 + 计数。**没有「全部替换」**（本分支做不了，见模块文档）。
    fn footer(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let (files, matches) = self.state.counts();
        let loading = matches!(self.state, ReplaceState::Loading { .. });
        let can_preview = preview_enabled(&self.state, &self.query_text);
        h_flex()
            .w_full()
            .flex_shrink_0()
            // 高 52（`ProjectReplaceView.swift:178`）不在档位上 ⇒ `rems(52. / 16.)`；左右 20 ⇒ `px_5()`。
            .h(rems(FOOTER_HEIGHT_SPEC / 16.))
            .px_5()
            .gap_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                Button::new(PREVIEW_BUTTON_ID)
                    .primary()
                    .label(tr("lithe.search.gpui.preview"))
                    // 查询为空 / 正在跑：禁用而不是"点了没反应"（真源同一判据，
                    // `ProjectReplaceView.swift:165`）。
                    .disabled(!can_preview)
                    .on_click(cx.listener(|this, _, _, cx| this.issue(cx))),
            )
            .child(div().flex_1())
            .child(div().w_4().flex_shrink_0().when(loading, |slot| {
                slot.child(Spinner::new().color(cx.theme().muted_foreground))
            }))
            .child(
                div()
                    .flex_shrink_0()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    // 真源那句 "N files, M matches"（`:172`）的两个数；
                    // `M` 是每个文件 `matchCount` 之和，不是命中行数。
                    .child(tr_args(
                        "lithe.search.gpui.replacePreviewCounts",
                        &[
                            ("files", &files.to_string()),
                            ("matches", &matches.to_string()),
                        ],
                    )),
            )
            .into_any_element()
    }

    /// 空态 / 失败态 —— 结构与度量由 [`lithe_gpui_shared::empty_state`] 这个**共同 owner** 提供
    /// （真源 `LitheUnavailableView.swift:19-35`），这里只把本面板的参数转过去。
    ///
    /// 改前本函数与 `crate::search_panel::SearchPanel::empty_state` 里那一份**逐字相同**
    /// （图标 `size_8()` = 32 而真源 28、标题漏 `font_semibold()`、描述还错用了标题档 `text_sm`）
    /// —— 同一个设计量被复制成两份，任一侧改动都会静默分叉。现在两份都只写一次。
    fn empty_state(
        icon: Option<IconName>,
        title: SharedString,
        description: Option<SharedString>,
        cx: &App,
    ) -> AnyElement {
        lithe_gpui_shared::empty_state::unavailable(
            icon.map(|icon| {
                Icon::new(icon).size(rems(
                    lithe_gpui_shared::empty_state::UNAVAILABLE_ICON_SIZE_SPEC / 16.,
                ))
            }),
            title,
            description,
            cx,
        )
    }
}

/// 「预览」按钮能不能按：查询非空、而且没有一次预览在飞。
///
/// 抽成纯函数是为了能被确定性单测钉住（视图 / 键盘路径在本仓库没有确定性单测，
/// 见模块文档的"诊断"一节）。判据与真源逐条一致：`isEmpty || isLoading` 就禁用
/// （`ProjectReplaceView.swift:165`）。
fn preview_enabled(state: &ReplaceState, query: &str) -> bool {
    !query.trim().is_empty() && !matches!(state, ReplaceState::Loading { .. })
}

impl Render for ReplaceInFiles {
    /// ⚠️ 这里**只读**状态：不 mutate、不 notify、不起任务。
    /// 进 `loading` / 换代 / 取消都发生在事件回调与后台任务里（见模块文档）。
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 索引就绪那一行（`Ready` 时是 `None`）：判据与元素都归 `crate::search_index`，
        // 这里只把它画在控件区与结果区之间。浮层不因此禁用 / 排队 / 改预览语义。
        let index_notice = crate::search_index::readiness_notice(self.search_index_readiness, cx);
        v_flex()
            .id(OVERLAY_ID)
            .size_full()
            .child(self.header(cx))
            .child(self.controls(cx))
            .children(index_notice)
            .child(self.results(cx))
            .child(self.footer(cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_model::{ReplacementFile, ReplacementLine};

    /// 四个选项的 `id` 必须互不相同、而且覆盖每一档。重复会让两个 `Button` 抢同一个
    /// `ElementId` —— gpui 会把点击串到另一个上（`notifications.rs:677-689` 记过这条）。
    #[test]
    fn option_ids_are_distinct_and_cover_every_option() {
        let mut ids: Vec<&str> = ReplaceOption::ALL
            .iter()
            .map(|option| option.id())
            .collect();
        assert_eq!(ids.len(), 4, "四颗开关一个不多一个不少");
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "选项 id 不能重复");
        // 查找框那一行是真源挂三颗的位置（正则不重复出现在替换行里）。
        assert_eq!(ReplaceOption::QUERY_ROW.len(), 3);
        assert!(!ReplaceOption::QUERY_ROW.contains(&ReplaceOption::PreserveCase));
    }

    /// `Preserve Case` 的置灰判据**来自 Core 而不是真源**：正则那一支
    /// `replace_with_options` 直接返回（`project/files.rs:890-901`），`preserveCase`
    /// 传进去也不会被读；而大小写敏感那一档 Core **照样**应用
    /// `apply_case_pattern`（`:937-941`），所以本侧不照真源置灰（有意差异第 4 条）。
    #[test]
    fn preserve_case_is_disabled_only_while_the_regex_option_is_on() {
        let mut options = ReplaceOptions::default();
        assert!(ReplaceOption::PreserveCase.enabled(&options));
        assert!(ReplaceOption::CaseSensitive.enabled(&options));
        options.search.regular_expression = true;
        assert!(
            !ReplaceOption::PreserveCase.enabled(&options),
            "正则打开时 Core 根本不读 preserveCase —— 画一颗拨了没用的开关就是骗人"
        );
        assert!(
            ReplaceOption::CaseSensitive.enabled(&options),
            "大小写敏感在正则打开时仍然有意义"
        );
    }

    /// 「预览」按钮的判据：空查询 → 禁用；一次预览在飞 → 禁用；只有"真的有活可干"时能按。
    #[test]
    fn preview_button_needs_a_query_and_no_preview_in_flight() {
        let files = vec![ReplacementFile {
            path: "src/A.java".to_string(),
            lines: vec![ReplacementLine {
                line: 1,
                before: "a".to_string(),
                after: "b".to_string(),
                occurrences: 1,
            }],
        }];
        assert!(!preview_enabled(&ReplaceState::Idle, ""));
        assert!(
            !preview_enabled(&ReplaceState::Idle, "   "),
            "只有空白也算空查询（Core 会 trim，`files.rs:341`）"
        );
        assert!(preview_enabled(&ReplaceState::Idle, "needle"));
        assert!(
            !preview_enabled(&ReplaceState::loading("needle", &files), "needle"),
            "一次预览在路上时不许再点（真源同一判据）"
        );
        assert!(preview_enabled(
            &ReplaceState::ready("needle", &files),
            "needle"
        ));
        assert!(preview_enabled(
            &ReplaceState::failed("needle", "invalid_request"),
            "needle"
        ));
    }

    /// 每个选项的读写落在 [`ReplaceOptions`] 的**不同**格子上：这是"表里加一档忘了改
    /// match"的机器判据，也钉住"四个布尔真的进 payload"这件事。
    #[test]
    fn each_option_reads_and_writes_its_own_slot() {
        let mut options = ReplaceOptions::default();
        for option in ReplaceOption::ALL {
            assert!(!option.get(&options), "默认全关");
            assert!(option.set(&mut options, true), "第一次拨要报 changed");
            assert!(option.get(&options), "{} 必须真的写进它那一格", option.id());
            assert!(!option.set(&mut options, true), "同一个值不算 changed");
            assert!(option.set(&mut options, false), "拨回去也要报 changed");
            assert!(!option.get(&options));
        }
        // 全部拨回来之后是干净的默认值（没有两档共用一个格子）。
        assert_eq!(options, ReplaceOptions::default());
    }
}
