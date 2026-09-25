//! 设置对话框：**模态对话框**（不是 Sheet、不是页面）。
//!
//! ## 布局真源
//!
//! 逐值照 `windows/tauri/src/features/settings/components/settings-dialog.tsx:88-157`
//! 与 `07-settings-ui.md` §1.3 的度量表：
//!
//! ```text
//! v_flex 820×620（居中、遮罩、Esc 可关）
//! ├─ 头部 44          h-11 border-b bg-surface px-3：[齿轮] 设置 ………… [关闭]
//! ├─ h_flex flex-1
//! │  ├─ 分类栏 190    w-47.5 border-r bg-surface p-2：项高 32、图标 16、选中 bg-primary/65
//! │  └─ 内容区 flex-1 bg-background p-6 overflow-y：页标题 text-xl font-semibold mb-5 + 分组
//! └─ 底部             justify-between px-4 py-3：左「恢复默认设置…」(ghost) 右「完成」(primary)
//! ```
//!
//! ## 组件选型：**不用** `component::setting::Settings`，自绘行
//!
//! `07-settings-ui.md` §9.3 推荐先用 `setting::{Settings, SettingPage, SettingGroup, SettingItem,
//! SettingField}`，但读完 0.6.6 的实现后判定**换不出这个布局**，三条硬证据：
//!
//! 1. **它自带搜索框，且无法关闭**：`Settings::render_sidebar` 的 header 永远塞一个
//!    `Input::new(&search_input)`（`gpui-component-0.6.6/src/setting/settings.rs:148-153`），
//!    没有任何开关；而 Windows 真源**没有搜索框**（§1.4：132 条搜索索引在真实对话框里没有渲染出口）。
//!    带着一个真源里没有的输入框，等于凭空多一个入口。
//! 2. **它的左栏是可拖拽分栏**：`h_resizable` + `resizable_panel().size(250)`（同文件 `:416-422`，
//!    宽度范围 160–360）；Windows 是**固定 190px、无拖拽把手**。
//! 3. **它的页面外壳不一样**：页头是 `p_4 + border_b_1`（`setting/page.rs:180-186`），
//!    Windows 是 `p-6` + 无下边框 + `text-xl font-semibold` 标题；分组/行的内边距也各自不同
//!    （`page.rs:236` 的 `py_4` vs Windows 的 `gap-3 p-3`）。
//!
//! 所以这里**复用它的行内控件**（`Switch` / `NumberInput` / `Button`，以及它自己的下拉实现方式
//! —— `Button + dropdown_menu_with_anchor + PopupMenuItem`，见
//! `setting/fields/dropdown.rs:60-87`）与 `Dialog` 容器，**自绘**左栏与行（[`crate::row`]）。
//! 代价：没有"白送"的搜索过滤（真源也没有）与页级重置按钮（真源只在底部有一个全局的）。
//!
//! ## 打开时机（硬约束）
//!
//! 浮层只能在事件回调或任务里打开；`render` 阶段调 `window.open_dialog` 会 panic
//! （`shell/overlays.md:203-213`、`gpui/crates/app/src/main.rs:14-16`）。本模块的两个入口
//! ——活动栏「设置」的点击回调、`Ctrl+,` 的**全局 action**（[`init`]）——都在事件回调里；
//! `--open-settings` 走 `window.on_next_frame`（首帧之后、非 render 阶段）。

use gpui_kit::assets::IconName;
use gpui_kit::base::{Selectable as _, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::input::{Input, InputEvent, InputState, NumberInput};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, Sizable as _, WindowExt as _};
use gpui_kit::{
    AbsoluteLength, Anchor, App, AppContext as _, Context, ElementId, Entity, FontWeight,
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, px, rems,
};

use lithe_gpui_shared::icons::{idea, idea_icon_svg};
use lithe_gpui_shared::tr;

use crate::identity::{
    GitIdentityPage, IdentityField, IdentityScope, IdentitySetup, git_identity_page,
    identity_value_is_valid, identity_value_rejection,
};
use crate::row::{ControlWidth, RowActivation, page_stack, page_title, settings_group, settings_row};
use crate::schema::{
    DISPLAY_LANGUAGES, EDITOR_FONT_SIZE_DEFAULT, EDITOR_FONT_SIZE_MAX, EDITOR_FONT_SIZE_MIN,
    SHELL_SYSTEM_DEFAULT, Settings, TAB_SIZES, TERMINAL_SHELL_IDS, UI_FONT_SIZE_DEFAULT,
    UI_FONT_SIZE_MAX, UI_FONT_SIZE_MIN, UI_FONT_SIZE_STEP,
};
use crate::store::{AppearanceMode, SettingsStore, store};
use crate::theme;

/// 打开设置对话框（`Ctrl+,`）。**只能从事件回调或任务里调用。**
///
/// 用 `actions!` + `App::on_action` 注册成**全局 action**，而不是挂在某个元素的
/// `on_action` 上：gpui 的按键派发在没有焦点元素时只走"窗口根节点"这一条路径
/// （`gpui-pre-0.3.6/src/window.rs:5815` 的 `focus_node_id_in_rendered_frame(None)` →
/// `root_node_id()`），而工作台视图是 `Root` 的**子**节点、不在那条路径上，所以挂在
/// `ShellWorkspace` 的根元素上会有"启动后没点过任何地方时 `Ctrl+,` 不响应"的死角。
/// `App::on_action` 注册的处理器在 action 冒泡阶段的最后一定会被调用
/// （`gpui-pre-0.3.6/src/app.rs:2360-2377`），与焦点无关。
pub fn install_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-,", OpenSettings, None)]);
    cx.on_action(|_: &OpenSettings, cx: &mut App| {
        let Some(window_handle) = cx.active_window() else {
            return;
        };
        let _ = window_handle.update(cx, |_, window, cx| open_settings_dialog(window, cx));
    });
}

gpui_kit::actions!(lithe_settings, [OpenSettings]);

/// 打开设置对话框。已经开着时（含确认子对话框）不叠第二层。
pub fn open_settings_dialog(window: &mut Window, cx: &mut App) {
    open_settings_dialog_at(window, cx, Category::DEFAULT);
}

/// 打开设置对话框并**直接停在某个分类**上。
///
/// 命令面板用这条入口（`gpui/crates/workbench/src/command_palette.rs` 的
/// `open-appearance-settings`）：真源里「首选项：打开 X 设置」会带一个页签
/// （`features/command-palette/constants/settings-actions.tsx:127-138` 的
/// `openSettingsDialog(tab)`），本侧只有两页，所以"到指定分类"就是全部差异。
pub fn open_settings_dialog_at(window: &mut Window, cx: &mut App, category: Category) {
    if window.has_active_dialog(cx) {
        return;
    }
    let store = store(cx);
    let view = cx.new(|cx| SettingsDialog::new(store, category, window, cx));
    println!("S1_SETTINGS dialog_opened category={}", category.id());

    window.open_dialog(cx, move |dialog, window, _cx| {
        // 820×620 用 rem 表达（`rems(P / 16.)`，1rem = 16px），再按窗口当前的 rem 基准求值：
        // `Dialog::width` / `margin_top` 只收 `Pixels`
        // （`gpui-component-0.6.6/src/dialog/dialog.rs:395,405`），而 gpui 没有
        // `impl From<Rems> for Pixels`，所以走 `AbsoluteLength::to_pixels(rem_size)`
        // （`gpui-pre-0.3.6/src/geometry.rs:3361`）。
        let rem = window.rem_size();
        let viewport = window.viewport_size();
        // 左右/下各留 16（Dialog 自己的边距也是 `spacing_tokens().lg` = 16，
        // `dialog/dialog.rs:528`）。
        let margin = rem;
        let width = rem_px(rem, DIALOG_WIDTH);
        let height = rem_px(rem, DIALOG_HEIGHT).min((viewport.height - margin * 2.).max(px(0.)));
        // 居中：Dialog 的 y 默认是视口的 1/10（`dialog/dialog.rs:529`），Windows 是垂直居中。
        let margin_top = ((viewport.height - height) / 2.).max(px(0.));

        dialog
            .close_button(false)
            .overlay(true)
            .overlay_closable(true)
            .keyboard(true)
            .width(width)
            .margin_top(margin_top)
            .h(height)
            // 弹层自身的内边距与间距归零：头部 / 内容 / 底部都由本视图自己排。
            .p_0()
            .content({
                let view = view.clone();
                // ⚠️ `.h_full()` **不是装饰，是这一页能不能滚的前提**：`DialogContent`
                // 默认是内容高度，不把它撑满对话框，`SettingsDialog` 根上的 `size_full()`
                // 就解析成 auto（百分比高度在 auto 高度的父级里等于 auto），整页按内容高度排下去 ——
                // 「Git」页那种比对话框高的页里，最后一个分组**够不到**、也滚不动
                // （实测滚轮前后像素 diff=0；同一注入器在编辑器上 diff=10969）。详见 `content()`。
                move |content, _, _| content.p_0().gap_0().h_full().child(view.clone())
            })
    });
}

/// 把规格值（px）按**当前 rem 基准**求值：`rems(P / 16.)`。
///
/// 写成 `/ 4.` 是错的 —— gpui 的档位 helper 后缀 `N` = `N × 0.25rem`，而这里的 `P` 是**像素**，
/// 1rem = 16px（主题的 `font.size`，`gpui/themes/lithe-dark.json:9`）。
fn rem_px(rem: gpui_kit::Pixels, spec_px: f32) -> gpui_kit::Pixels {
    AbsoluteLength::from(rems(spec_px / 16.)).to_pixels(rem)
}

/// 同一个换算的 `Length` 形式（`Styled::w` / `Styled::h` 收 `Length`，由布局期按窗口 rem 求值）。
fn rem_length(spec_px: f32) -> gpui_kit::Length {
    rems(spec_px / 16.).into()
}

/// 对话框尺寸（`settings-dialog.tsx:118-119` 的 `h-[620px] w-[820px]`）。
const DIALOG_WIDTH: f32 = 820.;
const DIALOG_HEIGHT: f32 = 620.;
/// 左栏宽度（`settings-dialog.tsx:126` 的 `w-47.5` = 190px）。
const NAV_WIDTH: f32 = 190.;
/// 头部高度（`settings-dialog.tsx:120` 的 `h-11` = 44px）——正好在 gpui 档位上，用 `h_11()`。

/// 左侧分类。**两类页面**：有真实内容的（常规 / 外观 / 编辑器 / 终端）与
/// **明确空态**的（其余 7 个 —— 子系统在 gpui 侧还不存在）。
///
/// 顺序 = Windows 分类表（`settings-dialog.tsx:35-48`）去掉 AI 两个分类之后的子序列，
/// **外加「外观」一页**：
///
/// ```text
/// 常规 → 外观* → 项目 · JDK 与 Maven → 运行配置 → 编辑器 → 快捷键 → 终端 → LSP → Git → 日志 → 更新
/// ```
///
/// ⚠️ `*` 「外观」是 **gpui 侧多出来的一页**：Windows 的真实对话框里**没有**这个分类
/// （`07-settings-ui.md` §5.1：`tabs/appearance-settings.tsx` 是死代码，`openSettingsDialog("appearance")`
/// 实际会落到「常规」），但主题必须有个落点，而死代码页签里那些项恰好是"能真生效"的一批。
/// v1 就把它做成了第 2 页；本阶段保持这个位置（紧挨「常规」，与主题在真源里属于"外观/常规"这一类相符）。
///
/// **为什么空态的也要列出来**（HANDOFF §4 第 2 项的口径）：用户点进一个分类期望看到
/// "这里能配什么、为什么现在没有"，而不是"这个分类不存在"。空态页只写一句前置条件，
/// **不画任何控件** —— 画一个永远不生效的开关比不画更容易骗人（`07-settings-ui.md` §7.3-D）。
///
/// 公开是因为命令面板要能"打开到指定分类"（[`open_settings_dialog_at`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// 常规（`settings.tabs.general` = 常规）。
    General,
    /// 外观（`settings.tabs.appearance` = 外观）。
    Appearance,
    /// 编辑器（`settings.tabs.editor` = 编辑器）。
    Editor,
    /// 终端（`settings.tabs.terminal` = 终端）。
    Terminal,
    /// 项目 · JDK 与 Maven（`settings.project.title`）—— 空态。
    Project,
    /// 运行配置（`settings.run.title`）—— 空态。
    Run,
    /// 快捷键（`settings.tabs.keyboard`）—— 空态。
    Keyboard,
    /// LSP（`settings.tabs.lsp`）—— 空态。
    Lsp,
    /// Git（`settings.tabs.git`）—— **条件实现**（阶段 15）：宿主登记了身份钩子就画
    /// 「提交身份」+ `confirmBeforeDiscard`，否则退回空态。
    Git,
    /// 日志（`settings.tabs.logs`）—— 空态。
    Logs,
    /// 更新（`settings.tabs.updates`）—— 空态。
    Updates,
}

impl Category {
    /// 左栏的渲染顺序（Windows 12 个分类去掉 AI 两个之后的子序列，外加 gpui 侧多出的「外观」）。
    pub const ALL: [Category; 11] = [
        Category::General,
        Category::Appearance,
        Category::Project,
        Category::Run,
        Category::Editor,
        Category::Keyboard,
        Category::Terminal,
        Category::Lsp,
        Category::Git,
        Category::Logs,
        Category::Updates,
    ];

    /// 有真实页面的分类（其余是 [`Self::prerequisite_key`] 的非空项 = 明确空态）。
    ///
    /// 测试用它把"空态页不许有控件、实现页必须有前置条件为 `None`"钉住；
    /// 把一页从空态升级成实现时，这张表和 `content()` 的分支要一起改。
    ///
    /// ⚠️ **「Git」页不在表里，但它有真实页面**（阶段 15）：它的能力取决于宿主有没有通过
    /// [`crate::identity::set_git_identity_host`] 登记钩子——登记了就画提交身份表单，
    /// 没登记（测试宿主 / 其它宿主）就退回 [`Self::empty_page`]。
    /// 这张表是**编译期常量**，表达不了"条件实现"，所以它记的是"这句话在没有任何宿主时也成立"
    /// 的那一面（= 空态需要一句前置条件）。`git_page_degrades_to_the_empty_state_without_a_host`
    /// 那条测试把这件事同时钉在两边。
    pub const IMPLEMENTED: [Category; 4] = [
        Category::General,
        Category::Appearance,
        Category::Editor,
        Category::Terminal,
    ];

    /// 不带参数打开设置时停在哪一页（真源默认是 `general`，`settings-dialog.tsx` 的初值）。
    pub const DEFAULT: Category = Category::General;

    /// 诊断行 `S1_SETTINGS dialog_opened category=…` 的取值。
    pub const fn id(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Appearance => "appearance",
            Self::Editor => "editor",
            Self::Terminal => "terminal",
            Self::Project => "project",
            Self::Run => "run",
            Self::Keyboard => "keyboard",
            Self::Lsp => "lsp",
            Self::Git => "git",
            Self::Logs => "logs",
            Self::Updates => "updates",
        }
    }

    /// 分类名文案键。**能复用真源既有键就复用**：
    /// 项目 / 运行配置页的标题在真源里是 `settings.project.title` / `settings.run.title`
    /// （`macos-settings-panels.tsx` 的分类标签就是这两条），其余走 `settings.tabs.*`。
    fn label_key(self) -> &'static str {
        match self {
            Self::General => "lithe.settings.tabs.general",
            Self::Appearance => "lithe.settings.tabs.appearance",
            Self::Editor => "lithe.settings.tabs.editor",
            Self::Terminal => "lithe.settings.tabs.terminal",
            Self::Project => "lithe.settings.project.title",
            Self::Run => "lithe.settings.run.title",
            Self::Keyboard => "lithe.settings.tabs.keyboard",
            Self::Lsp => "lithe.settings.tabs.lsp",
            Self::Git => "lithe.settings.tabs.git",
            Self::Logs => "lithe.settings.tabs.logs",
            Self::Updates => "lithe.settings.tabs.updates",
        }
    }

    /// 空态页那句"前置条件"的文案键；`None` = 这个分类**任何情况下都有真实页面**。
    ///
    /// ⚠️ 「Git」在这里返回 `Some`，但它在宿主登记了钩子时是**真实页面**
    /// （见 [`Category::IMPLEMENTED`] 的说明）：这一页的降级分支与其它分类的空态页共用
    /// 同一段渲染，所以它必须有一句可画的前置条件。
    ///
    /// 这些键**真源里没有**（Windows 这些页都有内容），由 `extract-locale.mjs` 的
    /// `GPUI_ONLY_KEYS` 提供，每条写了理由。
    fn prerequisite_key(self) -> Option<&'static str> {
        match self {
            Self::General | Self::Appearance | Self::Editor | Self::Terminal => None,
            Self::Project => Some("lithe.settings.gpui.prerequisiteProject"),
            Self::Run => Some("lithe.settings.gpui.prerequisiteRun"),
            Self::Keyboard => Some("lithe.settings.gpui.prerequisiteKeyboard"),
            Self::Lsp => Some("lithe.settings.gpui.prerequisiteLsp"),
            Self::Git => Some("lithe.settings.gpui.prerequisiteGit"),
            Self::Logs => Some("lithe.settings.gpui.prerequisiteLogs"),
            Self::Updates => Some("lithe.settings.gpui.prerequisiteUpdates"),
        }
    }

    /// 分类图标。Windows 用 `GearSixIcon` / `CodeBlockIcon` / `TerminalWindowIcon` 等
    /// **expui** 图标；本侧的分类图标继续走 Lucide（真源那几个字形已经搬进
    /// `gpui/assets/ui-icons/`，但分类栏这一列的图标不在本次范围内），逐个取语义最近的一个。
    fn icon(self) -> IconName {
        match self {
            Self::General => IconName::Settings,
            Self::Appearance => IconName::Palette,
            Self::Editor => IconName::Code,
            Self::Terminal => IconName::SquareTerminal,
            Self::Project => IconName::FolderCog,
            Self::Run => IconName::Play,
            Self::Keyboard => IconName::Keyboard,
            Self::Lsp => IconName::Braces,
            Self::Git => IconName::GitBranch,
            Self::Logs => IconName::ScrollText,
            Self::Updates => IconName::CloudDownload,
        }
    }
}

/// 「Git」页的运行期状态（阶段 15）。
///
/// 这一页的数据**不在设置文件里**（它读写的是 Git 的 `user.name` / `user.email`），
/// 所以整体单独一块，而不是加进 `Settings`。这里只放**可跨渲染帧保留**的状态；
/// 两个文本框的实体放在 [`SettingsDialog`] 上（与另外两个数字框同一处，
/// 视图状态集中在一个结构体里更好找）。
struct GitPageState {
    /// 宿主登记的钩子；`None` = 没有 Git 身份能力 → 这一页画**明确空态**
    /// （不是 panic、也不是画一半的控件）。
    host: Option<GitIdentityPage>,
    /// 当前作用域（真源 `git-identity-settings.tsx:21` 的初值也是 `local`）。
    scope: IdentityScope,
    /// 最近一次读到的快照；`None` = 还没读到（加载中 / 读失败）。
    setup: Option<IdentitySetup>,
    /// 读 / 写在飞。
    busy: bool,
    /// 最近一次失败的一句话（读或写；`None` = 没有失败）。
    error: Option<SharedString>,
    /// 刚保存成功的字段（真源 `git-identity-settings.tsx:27,158-162` 的 `saved`）。
    saved: Option<IdentityField>,
    /// 请求代次：切作用域 / 重新加载 / 保存之后，晚到的旧回包直接丢掉
    /// （真源 `git-identity-settings.tsx:29-33,45` 的 `generation` + `currentContext`）。
    generation: u64,
}

impl GitPageState {
    fn new() -> Self {
        Self {
            host: git_identity_page(),
            scope: IdentityScope::default(),
            setup: None,
            busy: false,
            error: None,
            saved: None,
            generation: 0,
        }
    }
}

/// 「Git」页的诊断前缀（与 `S1_SOURCE_CONTROL` 一族同口径，走 stderr）。
const GIT_IDENTITY_TAG: &str = "S1_GIT_IDENTITY";

/// 「Git」页里「保存」按钮可不可点的**纯判据**（不碰 `App`，所以可以直接单测）。
///
/// 四个禁用条件逐条照真源 `git-identity-settings.tsx:133-139`：
/// 读/写在飞、还没读到快照、`local` 而目标不是仓库、草稿为空 / 非法 / 与已保存值相同。
/// 界面调用点与测试用的是**同一个函数**，不会出现"测试通过但按钮永远可点"这种漂移。
fn git_save_enabled(
    setup: Option<&IdentitySetup>,
    scope: IdentityScope,
    field: IdentityField,
    draft: &str,
    busy: bool,
) -> bool {
    let Some(setup) = setup else {
        return false;
    };
    !busy && setup.can_save(scope, field, draft)
}

/// 「Git」页里「清除覆盖」按钮可不可点的纯判据（真源 `:147` 的 `configured == null`）。
fn git_clear_enabled(
    setup: Option<&IdentitySetup>,
    field: IdentityField,
    busy: bool,
) -> bool {
    let Some(setup) = setup else {
        return false;
    };
    !busy && setup.can_clear(field)
}

/// 设置对话框的内容视图。
pub struct SettingsDialog {
    store: Entity<SettingsStore>,
    category: Category,
    /// 「界面字体大小」的数字输入状态（`NumberInput` 是 Stateful 组件，状态由调用方持有）。
    font_size_input: Entity<InputState>,
    /// 「编辑器字体大小」的数字输入状态（同一个组件，另一份状态 —— 两个键互不相干）。
    editor_font_size_input: Entity<InputState>,
    /// 「Git」页的输入框与运行期状态（阶段 15；**不进设置文件**）。
    git_name_input: Entity<InputState>,
    git_email_input: Entity<InputState>,
    git: GitPageState,
    /// 订阅与观察（`store` 变了要重绘；输入框变了要写设置）。
    _subscriptions: Vec<Subscription>,
}

impl SettingsDialog {
    fn new(
        store: Entity<SettingsStore>,
        category: Category,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let value = store.read(cx).settings().ui_font_size;
        let editor_value = store.read(cx).settings().font_size;
        // 数字框的引擎配置放在 `InputState` 上：`+`/`-` 与上下键都按 `step` 走、
        // 越界文本在输入期间被容忍、失焦时收敛到范围（`gpui-base-0.6.6/src/input/base/state.rs:9032-9052`）。
        let font_size_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(format_ui_font_size(value))
                .step(UI_FONT_SIZE_STEP)
                .min(UI_FONT_SIZE_MIN)
                .max(UI_FONT_SIZE_MAX)
        });
        // 编辑器字号的档位与 UI 字号不同：整数步长、10–22（真源控件属性，不是 `ui-font-size.ts` 那套）。
        let editor_font_size_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(format_editor_font_size(editor_value))
                .step(1.)
                .min(EDITOR_FONT_SIZE_MIN)
                .max(EDITOR_FONT_SIZE_MAX)
        });

        // 「Git」页的两个文本框。**初值是空的**：真源也是先渲染空输入框、等
        // `git.repositorySetup` 回来才 `setName` / `setEmail`
        // （`git-identity-settings.tsx:23-24,42-49`），这样"还没读到"与"读到了空值"不会混。
        let git_name_input = cx.new(|cx| InputState::new(window, cx));
        let git_email_input = cx.new(|cx| InputState::new(window, cx));

        let mut subscriptions = Vec::new();
        // 设置变了 → 重绘（主题/字号/开关的显示都跟着走）。
        subscriptions.push(cx.observe(&store, |_, _, cx| cx.notify()));
        // 输入框变了 → 写设置。照 `setting/fields/number.rs:96-111` 的口径：
        // 解析不了的中间态（`""`、`"1."`）与越界值都**不写**设置，等下一次按键补全，
        // 免得"输入 18 的过程中先被夹成 10"。
        subscriptions.push(cx.subscribe_in(
            &font_size_input,
            window,
            |this: &mut Self, input, event: &InputEvent, _window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let Ok(parsed) = input.read(cx).value().parse::<f64>() else {
                    return;
                };
                if !(UI_FONT_SIZE_MIN..=UI_FONT_SIZE_MAX).contains(&parsed) {
                    return;
                }
                let store = this.store.clone();
                store.update(cx, |store, cx| store.set_ui_font_size(parsed, cx));
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &editor_font_size_input,
            window,
            |this: &mut Self, input, event: &InputEvent, _window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let Ok(parsed) = input.read(cx).value().parse::<f64>() else {
                    return;
                };
                if !(EDITOR_FONT_SIZE_MIN..=EDITOR_FONT_SIZE_MAX).contains(&parsed) {
                    return;
                }
                let store = this.store.clone();
                store.update(cx, |store, cx| store.set_editor_font_size(parsed, cx));
            },
        ));
        // 「Git」页的两个文本框：**只重绘**（保存按钮的禁用态跟着草稿走），
        // 不写任何东西 —— 写入只发生在「保存」按钮被点的那一刻（真源同样是显式保存，
        // `git-identity-settings.tsx:131-143`）。这是与上面两个数字框的关键差别：
        // 那两个是设置项（改即生效），这两个是 Git 配置的**草稿**。
        for input in [&git_name_input, &git_email_input] {
            subscriptions.push(cx.subscribe_in(
                input,
                window,
                |_this: &mut Self, _input, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                },
            ));
        }

        Self {
            store,
            category,
            font_size_input,
            editor_font_size_input,
            git_name_input,
            git_email_input,
            git: GitPageState::new(),
            _subscriptions: subscriptions,
        }
    }

    /// 头部 44px：齿轮 + 「设置」+ 关闭。`settings-dialog.tsx:120` + `ui/dialog.tsx:261-283`。
    fn header(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .h_11()
            .flex_shrink_0()
            .items_center()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .text_color(cx.theme().foreground)
            // 头部齿轮：改用**Windows 真源**的 IntelliJ `expui` 图标
            // （`ui-icons/idea/expui/general/settings.svg`，深色另有 `settings_dark.svg`），
            // 不再用 Lucide 的 `IconName::Settings`。这是"图标资源接线"的第一处真实调用点：
            // 真源文件已在 `gpui/assets/ui-icons/`，注册（`LitheAssets`）与清单
            // （`shared::icons::idea`）已就位，helper 按主题挑明暗。
            // 详细的四步引用规范见 `gpui/research/icon-asset-inventory.md` 第 7 节。
            .child(idea_icon_svg(&idea::GEAR_ICON, cx))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(tr("lithe.workbench.settings")),
            )
            .child(div().flex_1())
            .child(
                // 只有图标的按钮必须有 tooltip 与可读名称（设计指南「字体与图标」）。
                Button::new("settings-close")
                    .small()
                    .ghost()
                    .icon(IconName::Close)
                    .accessibility_label(tr("lithe.tabs.close"))
                    .tooltip(tr("lithe.tabs.close"))
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            )
    }

    /// 左栏：固定宽、无拖拽把手，选中的分类有**颜色 + 字重**两种可辨别信号。
    fn nav(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .id("settings-nav")
            .h_full()
            .w(rem_length(NAV_WIDTH))
            .flex_shrink_0()
            .gap_0p5()
            .p_2()
            .border_r_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .aria_label(tr("lithe.settings.mac.categories"))
            .children(Category::ALL.iter().enumerate().map(|(index, category)| {
                let selected = *category == self.category;
                Button::new(("settings-category", index))
                    .ghost()
                    .selected(selected)
                    .w_full()
                    // 内容整体左对齐：`Button` 的内容层默认 `justify_center`
                    // （`button/button.rs:700-705`），所以要塞一个 `w_full` 的自定义内容，
                    // 而不是用 `.icon()/.label()`（那两者会被居中）。
                    .child(
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .items_center()
                            .gap_2p5()
                            .child(Icon::new(category.icon()).size_4())
                            .child(
                                div()
                                    .min_w_0()
                                    .truncate()
                                    .when(selected, |this| this.font_weight(FontWeight::MEDIUM))
                                    .child(tr(category.label_key())),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.category = *category;
                        // 「Git」页第一次被打开时读一次身份。**不在构造期读**：那会把首帧卡住，
                        // 与 `Explorer` / `ChangesView` 的"构造期不取数据"同一条口径。
                        // 钩子没登记时 `git_load` 直接返回（`host` 是 `None`）。
                        if *category == Category::Git && this.git.setup.is_none() && !this.git.busy {
                            this.git_load(window, cx);
                        }
                        cx.notify();
                    }))
            }))
    }

    /// 右栏：页标题 + 分组，只有这一列滚动（滚动条贴住它自己的边缘）。
    ///
    /// ⚠️ 这一列能不能滚，取决于**父级有没有确定高度**：`overflow_y_scrollbar()`
    /// （gpui-component 的 `Scrollable` 包装元素，`scroll/scrollable.rs:128-186`）
    /// 把"滚动"放在它自己 `size_full()` 的内层，而 `size_full()` 是**百分比高度** ——
    /// 父级是内容高度时它解析成 auto，于是整列按内容排下去、既没有可滚的余地也不裁剪。
    /// 实测（2026-09-26）：Git 页比对话框高时，"集成"分组**根本够不到**（滚轮前后像素 diff=0），
    /// 而同一注入器在编辑器上 diff=10969。修在调用点：给 `DialogContent` 加 `.h_full()`
    /// （见 [`open_settings_dialog_at`] 的 `.content(..)`）。
    fn content(&self, cx: &Context<Self>) -> impl IntoElement {
        let settings = self.store.read(cx).settings().clone();
        div()
            .id("settings-content-scroll")
            .flex_1()
            .min_w_0()
            .h_full()
            .p_6()
            .bg(cx.theme().background)
            .overflow_y_scrollbar()
            .child(page_stack().child(page_title(tr(self.category.label_key()))).children(
                match self.category {
                    Category::General => self.general_page(&settings, cx),
                    Category::Appearance => self.appearance_page(&settings, cx),
                    Category::Editor => self.editor_page(&settings, cx),
                    Category::Terminal => self.terminal_page(&settings, cx),
                    // 「Git」页（阶段 15）：提交身份 + 一个真有消费方的开关。
                    // 钩子没登记时 `git_page` 自己退回明确空态。
                    Category::Git => self.git_page(&settings, cx),
                    // 其余分类是**明确空态**：只有一句前置条件，没有任何控件。
                    Category::Project
                    | Category::Run
                    | Category::Keyboard
                    | Category::Lsp
                    | Category::Logs
                    | Category::Updates => self.empty_page(cx),
                },
            ))
    }

    /// 「常规」页：v1 只做「语言」一项。
    fn general_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        vec![
            settings_group(
                tr("lithe.settings.mac.language"),
                vec![settings_row(
                    "settings-row-language",
                    tr("lithe.settings.mac.language"),
                    // ⚠️ **不照抄** Windows 的 `settings.mac.languageDescription`
                    // （「界面语言会立即生效。默认语言为英文。」）—— gpui 侧 `set_locale` 只在启动早期
                    // 调用一次，而界面里有构造期就 `tr()` 过的文案（活动栏项、状态栏），运行中换语言会
                    // 变成一半新一半旧。本侧的做法是**选完就用相同参数重启自己**（见下），所以描述写
                    // "重启后生效"——它确实会重启，只是不用用户自己动手。这条是 gpui 侧新增的键
                    // （Windows 缺失，本侧补）。
                    Some(tr("lithe.settings.gpui.languageRestartDescription")),
                    self.dropdown(
                        "settings-language",
                        language_label(&settings.display_language),
                        DISPLAY_LANGUAGES
                            .iter()
                            .map(|tag| {
                                let value = SharedString::from(*tag);
                                (
                                    value.clone(),
                                    language_label(tag),
                                    settings.display_language == *tag,
                                )
                            })
                            .collect(),
                        {
                            let store = self.store.clone();
                            Box::new(move |value, _, cx| {
                                // 改语言 → 立即落盘（`set_display_language` 不等防抖）→ 用相同参数
                                // **重启自己**，新进程直接以新语言起来。只有**真的变了**才重启：
                                // 选回当前语言不该把应用重启一次。
                                //
                                // 为什么不走"热切"：`set_locale` 只在启动早期调一次，界面里又有构造期
                                // 就 `tr()` 过的文案（活动栏项、状态栏），热切只会"一半新一半旧"；
                                // 而"只提示、让用户自己重启"会让界面长期停在半生效状态。重启最干净。
                                // ⚠️ 代价：当前进程里未保存的编辑器内容与终端会话会随之结束
                                // （编辑器接上保存后，这里要重新评估是否加确认）。
                                let changed = store.update(cx, |store, cx| {
                                    store.set_display_language(value.to_string(), cx)
                                });
                                if changed {
                                    crate::restart::restart_application(cx);
                                }
                            })
                        },
                    ),
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    /// 「外观」页：v1 做**能立刻生效**的 4 项。
    fn appearance_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let mode = AppearanceMode::from_settings(settings);
        // 显示**当前生效**的主题（跟随系统时就是系统对应的那一支），而不是 Windows 的
        // `settings.theme`：同步模式下 Windows 会显示一个并非生效中的值
        // （`macos-settings-panels.tsx:141` 读 `settings.theme`，而 `:122` 写的是 `autoTheme*`）。
        // 这是有意的偏离，避免界面显示的值与实际生效的不一致。
        let applied_theme = self.store.read(cx).applied_theme();
        let themes = theme_choices(cx, applied_theme.clone());

        vec![
            settings_group(
                tr("lithe.settings.appearance.theme"),
                vec![
                    settings_row(
                        "settings-row-theme",
                        tr("lithe.settings.appearance.colorTheme"),
                        None,
                        self.dropdown(
                            "settings-theme",
                            applied_theme,
                            themes,
                            {
                                let store = self.store.clone();
                                Box::new(move |value, _, cx| {
                                    store.update(cx, |store, cx| store.set_theme(value, cx));
                                })
                            },
                        ),
                        None,
                        cx,
                    ),
                    settings_row(
                        "settings-row-appearance-mode",
                        tr("lithe.settings.mac.appearanceMode"),
                        Some(tr("lithe.settings.mac.appearanceDescription")),
                        self.dropdown(
                            "settings-appearance-mode",
                            mode_label(mode),
                            vec![
                                (
                                    SharedString::from(MODE_SYSTEM),
                                    tr("lithe.settings.mac.followSystem"),
                                    mode == AppearanceMode::System,
                                ),
                                (
                                    SharedString::from(MODE_LIGHT),
                                    tr("lithe.settings.mac.light"),
                                    mode == AppearanceMode::Light,
                                ),
                                (
                                    SharedString::from(MODE_DARK),
                                    tr("lithe.settings.mac.dark"),
                                    mode == AppearanceMode::Dark,
                                ),
                            ],
                            {
                                let store = self.store.clone();
                                Box::new(move |value, _, cx| {
                                    let mode = match value.as_ref() {
                                        MODE_LIGHT => AppearanceMode::Light,
                                        MODE_DARK => AppearanceMode::Dark,
                                        _ => AppearanceMode::System,
                                    };
                                    store.update(cx, |store, cx| {
                                        store.set_appearance_mode(mode, cx)
                                    });
                                })
                            },
                        ),
                        None,
                        cx,
                    ),
                ],
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.appearance.typography"),
                vec![settings_row(
                    "settings-row-ui-font-size",
                    tr("lithe.settings.appearance.uiFontSize"),
                    // Windows 的 `settings.appearance.uiFontSizeDescription`
                    // （「以 0.5 像素为单位调整界面文本和图标缩放」）正好说明了步长与作用范围。
                    Some(tr("lithe.settings.appearance.uiFontSizeDescription")),
                    NumberInput::new(&self.font_size_input)
                        .w(ControlWidth::Number.length())
                        .into_any_element(),
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.appearance.interface"),
                vec![settings_row(
                    "settings-row-status-bar",
                    tr("lithe.settings.appearance.showStatusBar"),
                    None,
                    {
                        let store = self.store.clone();
                        Switch::new("settings-show-status-bar")
                            .small()
                            .checked(settings.show_status_bar)
                            .on_change(move |checked, _, cx| {
                                let checked = *checked;
                                store.update(cx, |store, cx| {
                                    store.set_show_status_bar(checked, cx)
                                });
                            })
                            .into_any_element()
                    },
                    // 开关行支持"点整行 = 切换"（Windows `SettingsRow` 的整行激活）。
                    // 开关自己会吃掉点击并切换，所以行回调只在点标签/空白处触发，不会触发两次。
                    Some({
                        let activation: RowActivation = {
                            let store = self.store.clone();
                            let next = !settings.show_status_bar;
                            Box::new(move |_, _, cx| {
                                store.update(cx, |store, cx| {
                                    store.set_show_status_bar(next, cx)
                                });
                            })
                        };
                        activation
                    }),
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    /// 「编辑器」页：**只放真的有消费方的两项**。
    ///
    /// 真源 `macos-settings-panels.tsx:275-328` 的「编辑器」页有 4 项，分三组
    /// （显示 / 编辑器标签页 / 缩进）：
    ///
    /// | 真源项 | 本侧 | 为什么 |
    /// | --- | --- | --- |
    /// | `fontSize`（数字，10–22） | ✅ 做了 | 落点是主题的 `mono_font_size`，编辑器正文当帧就变（[`crate::store::SettingsStore::set_editor_font_size`]） |
    /// | `tabSize`（2/4/8） | ✅ 做了 | 落点是每个 `EditorState` 的 `TabSize`，由外壳转发给 `EditorPane::set_tab_size` |
    /// | `codeLens`（「显示用法与 Git 作者」） | ❌ 不做 | gpui 侧**没有 LSP 的 code lens 通路**，也没有行内 Git blame（`enableInlineGitBlame` 同理）；放上去就是"点了没反应" |
    /// | `horizontalTabScroll`（缓冲区轮播） | ❌ 不做 | 本侧标签条没有轮播形态（`TabBar` 自带横向滚动，但没有"单行 / 多行换行"这一档设置）；同上，不做假控件 |
    ///
    /// 被砍掉的两项**不画置灰控件**：`07-settings-ui.md` §7.3-D 对"没有对应物"的项给的
    /// 处理是"应隐藏或标注"，而画一个永远不生效的开关比不画更容易骗人。
    fn editor_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        vec![
            settings_group(
                tr("lithe.settings.mac.display"),
                vec![settings_row(
                    "settings-row-editor-font-size",
                    tr("lithe.settings.mac.fontSize"),
                    // 真源这一行**没有**描述；这里补一句是因为 gpui 侧的字号作用于主题的
                    // 等宽字号（编辑器正文），而终端正文也用它 —— 说清楚作用范围，
                    // 免得用户以为它是"界面字号"（那是外观页的 `uiFontSize`）。
                    Some(tr("lithe.settings.gpui.editorFontSizeDescription")),
                    NumberInput::new(&self.editor_font_size_input)
                        .w(ControlWidth::Number.length())
                        .into_any_element(),
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.mac.indentation"),
                vec![settings_row(
                    "settings-row-tab-size",
                    tr("lithe.settings.mac.tabWidth"),
                    None,
                    self.dropdown(
                        "settings-tab-size",
                        tab_size_label(settings.tab_size),
                        TAB_SIZES
                            .iter()
                            .map(|size| {
                                let value = SharedString::from(size.to_string());
                                (value, tab_size_label(*size), settings.tab_size == *size)
                            })
                            .collect(),
                        {
                            let store = self.store.clone();
                            Box::new(move |value, _, cx| {
                                let Ok(size) = value.parse::<u32>() else {
                                    return;
                                };
                                store.update(cx, |store, cx| store.set_tab_size(size, cx));
                            })
                        },
                    ),
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    /// 「终端」页：真源只有一项（`macos-settings-panels.tsx:372-396`，分组 `Shell`）。
    ///
    /// 描述逐字用真源的 `settings.mac.defaultShellDescription`（「用于新的终端会话。」）——
    /// 这句在 gpui 侧同样成立：已开的会话不会换 shell，只有新建页签才按新值解析
    /// （`terminal/src/session.rs` 的 `open_tab_with` 读 `active_profile`）。
    fn terminal_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        vec![
            settings_group(
                tr("lithe.settings.mac.shell"),
                vec![settings_row(
                    "settings-row-terminal-shell",
                    tr("lithe.settings.mac.defaultShell"),
                    Some(tr("lithe.settings.mac.defaultShellDescription")),
                    self.dropdown(
                        "settings-terminal-shell",
                        shell_label(&settings.terminal_default_shell_id),
                        TERMINAL_SHELL_IDS
                            .iter()
                            .map(|id| {
                                let value = SharedString::from(*id);
                                (
                                    value,
                                    shell_label(id),
                                    settings.terminal_default_shell_id == *id,
                                )
                            })
                            .collect(),
                        {
                            let store = self.store.clone();
                            Box::new(move |value, _, cx| {
                                // 只写设置：把它推给终端面板是**外壳**的事（订阅本实体后调
                                // `TerminalPane::set_default_shell`）。设置 crate 不认识终端 crate
                                // —— 依赖方向见 `crate::lib.rs` 的模块文档。
                                store.update(cx, |store, cx| {
                                    store.set_terminal_default_shell_id(value.to_string(), cx)
                                });
                            })
                        },
                    ),
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    // ---- 「Git」页（阶段 15） ----

    /// 该字段对应的输入框实体。
    fn git_input(&self, field: IdentityField) -> &Entity<InputState> {
        match field {
            IdentityField::Name => &self.git_name_input,
            IdentityField::Email => &self.git_email_input,
        }
    }

    /// 输入框里的当前草稿。
    fn git_draft(&self, field: IdentityField, cx: &Context<Self>) -> SharedString {
        self.git_input(field).read(cx).value()
    }

    /// 把快照里"该作用域已保存的值"写回输入框。
    ///
    /// 只在**读回包 / 保存回包 / 重新加载**之后调用（真源 `git-identity-settings.tsx:47-48,76-77`）。
    /// ⚠️ **成功保存 name 时不要动 email 的草稿**（真源 `:75` 的注释就是这件事）：
    /// 调用点只传自己那一个字段。
    fn git_fill_input(
        &self,
        field: IdentityField,
        setup: &IdentitySetup,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = setup.configured(field).unwrap_or("").to_string();
        self.git_input(field).update(cx, |input, cx| {
            input.set_value(SharedString::from(text), window, cx);
        });
    }

    /// 读一次身份快照（切作用域 / 重新加载 / 第一次进这一页）。
    ///
    /// **异步**：Core 调用是同步阻塞的，所以走宿主钩子里的 `background_spawn` + 回前台回写
    /// （照 `changes.rs` / `explorer` 的现成写法）。回包要过**代次校验**：
    /// 用户在看结果之前又切了一次作用域时，旧结果必须丢掉。
    ///
    /// 不收 `Window`：回包那一层拿到的 `window` 由 `WeakEntity::update` 提供
    /// （见 [`Self::git_fill_input`]）。
    fn git_load(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(host) = self.git.host.clone() else {
            return;
        };
        self.git.generation = self.git.generation.wrapping_add(1);
        let generation = self.git.generation;
        let scope = self.git.scope;
        self.git.busy = true;
        self.git.error = None;
        self.git.saved = None;
        self.git.setup = None;
        cx.notify();

        self.git_diagnose(&format!(
            "run=load scope={} root={}",
            scope.id(),
            host.workspace_root.display()
        ));

        let entity = cx.entity().downgrade();
        host.load(
            scope,
            move |setup, window, cx| {
                let _ = entity.update(cx, |this, cx| {
                    // `WeakEntity<SettingsDialog>::update(cx: &mut App, ..)` 把 `window`
                    // 交给闭包，所以这一层不捕获外层的 `window`（捕获了就不是 `'static`）。
                    if this.git.generation != generation {
                        return;
                    }
                    this.git.busy = false;
                    match setup {
                        Some(setup) => {
                            this.git.setup = Some(setup.clone());
                            this.git_diagnose(&format!(
                                "run=load result=ok scope={} repository={} commits={} configured_name={} configured_email={} effective_name={} effective_email={}",
                                scope.id(),
                                setup.is_repository,
                                setup.has_commits,
                                setup.configured_name.is_some(),
                                setup.configured_email.is_some(),
                                setup.effective_name.is_some(),
                                setup.effective_email.is_some(),
                            ));
                            // 两个输入框都填上"该作用域已保存的值"。
                            this.git_fill_input(IdentityField::Name, &setup, window, cx);
                            this.git_fill_input(IdentityField::Email, &setup, window, cx);
                        }
                        None => {
                            // 宿主已经打过诊断（`S1_GIT_IDENTITY run=… result=failed`）；
                            // 这里只把"读失败"这句话画出来（真源 `git.setup.readFailed`）。
                            this.git_diagnose(&format!(
                                "run=load result=failed scope={}",
                                scope.id()
                            ));
                            this.git.error = Some(tr("lithe.git.setup.readFailed"));
                            this.git_fill_input(
                                IdentityField::Name,
                                &IdentitySetup::default(),
                                window,
                                cx,
                            );
                            this.git_fill_input(
                                IdentityField::Email,
                                &IdentitySetup::default(),
                                window,
                                cx,
                            );
                        }
                    }
                    cx.notify();
                });
            },
            window,
            cx,
        );
    }

    /// 保存一个字段（`clear` = 清除该作用域的覆盖）。
    ///
    /// 与真源同一口径（`git-identity-settings.tsx:63-85`）：**逐字段保存**，
    /// 一个字段成功不会顺手写另一个；就算另一个字段的草稿还没保存也不会被覆盖。
    fn git_save(&mut self, field: IdentityField, clear: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(host) = self.git.host.clone() else {
            return;
        };
        let scope = self.git.scope;
        let draft = self.git_draft(field, cx);
        let value = if clear {
            None
        } else {
            let value = draft.trim().to_string();
            if !identity_value_is_valid(&value) {
                // 界面上的按钮此时是禁用的，所以走到这里只可能是"键盘/程序路径绕过了按钮"。
                // 不静默：留一行诊断，然后什么都不做（绝不给 Core 发一个必定被拒的请求）。
                self.git_diagnose(&format!(
                    "run=save field={} result=blocked reason={}",
                    field.id(),
                    identity_value_rejection(&value).unwrap_or("invalid")
                ));
                return;
            }
            Some(value)
        };

        self.git.generation = self.git.generation.wrapping_add(1);
        let generation = self.git.generation;
        self.git.busy = true;
        self.git.error = None;
        self.git.saved = None;
        cx.notify();

        self.git_diagnose(&format!(
            "run=save scope={} field={} action={} value_len={}",
            scope.id(),
            field.id(),
            if clear { "clear" } else { "set" },
            value.as_ref().map(|value| value.len()).unwrap_or(0)
        ));

        let entity = cx.entity().downgrade();
        host.save(
            scope,
            field,
            value,
            move |setup, window, cx| {
                let _ = entity.update(cx, |this, cx| {
                    if this.git.generation != generation {
                        return;
                    }
                    this.git.busy = false;
                    match setup {
                        Some(setup) => {
                            this.git.setup = Some(setup.clone());
                            this.git.saved = Some(field);
                            this.git_diagnose(&format!(
                                "run=save field={} result=ok configured={} effective={}",
                                field.id(),
                                setup.configured(field).is_some(),
                                setup.effective(field).is_some(),
                            ));
                            // **只回写这一个字段**（真源 `:75-77` 的注释：保存 name 不能
                            // 抹掉还没保存的 email 草稿）。
                            this.git_fill_input(field, &setup, window, cx);
                        }
                        None => {
                            this.git_diagnose(&format!(
                                "run=save field={} result=failed",
                                field.id()
                            ));
                            this.git.error = Some(tr("lithe.git.setup.save"));
                        }
                    }
                    cx.notify();
                });
            },
            window,
            cx,
        );
    }

    /// 一行 `S1_GIT_IDENTITY` 诊断（可 grep；走 stderr，与 `S1_SETTINGS` 一族同口径）。
    fn git_diagnose(&self, detail: &str) {
        eprintln!("{GIT_IDENTITY_TAG} {detail}");
    }

    /// 「Git」页：**提交身份**（真源 `components/git-identity-settings.tsx`，规格
    /// `07-settings-ui.md` §3.10 的「子面板 2」）+ 一个**真的有消费方**的开关。
    ///
    /// 做完的（真源项 → 本侧落点）：
    ///
    /// | 真源项 | 本侧 | 落点 |
    /// | --- | --- | --- |
    /// | 作用域下拉（`local` / `global`） | ✅ | 切一次重新读一次（`git_load`），并让旧回包作废 |
    /// | 姓名 / 邮箱两个字段，各自「保存」+「清除覆盖」+「当前生效值」 | ✅ | `git.repositorySetup` / `git.configureIdentity`（经宿主钩子） |
    /// | 非仓库 + `local` 禁用输入并提示 | ✅ | 两个输入框与保存按钮都禁用 + `git.setup.initializeFirst` |
    /// | `settings.git.confirmDiscard`（丢弃前确认） | ✅ | `ChangesView::set_confirm_before_discard`（真的被丢弃路径消费） |
    ///
    /// **没做完的逐条与理由**（`07-settings-ui.md` §7.3-D 的口径：没有消费方的一律不画，
    /// 画一个永远不生效的开关比不画更容易骗人）：
    ///
    /// | 真源项 | 为什么不做 |
    /// | --- | --- |
    /// | `gitExecutable` / `gitUseCredentialHelper` / 整个 `GitExecutionSettings` 子面板 | 它们写的是 **Git 配置文件**（`git.executionConfigure`）或凭据助手，gpui 侧没有这两条通路，也没有对应的 Core 命令落在本侧的命令集里 |
    /// | `gitFetchPrune` / `gitFetchSubmodules` / `gitFetchTags` | 要 `git.fetchPlan` + 远程 Fetch 子系统；gpui 侧没有 Fetch 入口（`changes.rs` 的写操作只有 stage/commit/discard 一族） |
    /// | `coreFeatures.git`（Git 集成总开关） | 关掉它意味着"整个源代码管理页消失"，而 gpui 侧没有任何地方读这个分组位；**另开一类破坏性状态**超出本页范围 |
    /// | `autoRefreshGitStatus` | gpui 侧**刻意没有 watcher**（`git/src/lib.rs` 的 `07` 条），自动刷新只有"切到本视图时刷新一次"这一条，开关关掉等于把唯一一条自动通路也关死 |
    /// | `gitChangesFolderView` | 变更列表第一版就是扁平列表（没有目录树折叠），开关无处生效 |
    /// | `showUntrackedFiles` / `showStagedFirst` / `openDiffOnClick` / `compactGitStatusBadges` / `collapseEmptyGitSections` | 都要改**变更列表的行分类与布局**；第一版的分类头与行样式是固定的（真源那几项各自对应一处渲染分支，本侧还没有那些分支） |
    /// | `rememberLastGitPanelMode` | gpui 侧的底部 Git 面板还没有"分区模式"这个概念 |
    /// | `gitDefaultDiffView` | gpui 侧**没有差异视图**（要 `git.diff` 与富渲染） |
    /// | `enableInlineGitBlame` | 编辑区没有行内 blame（同「编辑器」页 `codeLens` 的理由） |
    fn git_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        // 钩子没登记 → **明确空态**（与其它未接入分类同一页）。
        // ⚠️ 这一条是 HANDOFF 要求的降级分支，也是"设置对话框在别的宿主里也能开"的唯一出口：
        // 不许 panic、不许假装成功。
        if self.git.host.is_none() {
            return self.empty_page(cx);
        }
        let host_root = self
            .git
            .host
            .as_ref()
            .map(|host| host.workspace_root.display().to_string())
            .unwrap_or_default();
        // 每次渲染打一行（可 grep）：它同时证明"设置对话框**读到了**宿主钩子"与
        // "这一页真的走到了实现分支而不是空态"。**走 stderr**（stdout 重定向到文件时
        // 是块缓冲，见 HANDOFF §2）。`git_diagnose` 是 `&self` 方法，这里正好可用。
        self.git_diagnose(&format!("run=render host=present root={host_root}"));

        let mut rows: Vec<gpui_kit::AnyElement> = Vec::new();

        // ① 作用域：一行下拉 + 一句按作用域变的说明（真源 `:90-104`）。
        rows.push(settings_row(
            "settings-row-git-scope",
            tr("lithe.git.setup.scope"),
            Some(tr(if self.git.scope == IdentityScope::Local {
                "lithe.git.setup.localDescription"
            } else {
                "lithe.git.setup.globalDescription"
            })),
            self.dropdown(
                "settings-git-scope",
                git_scope_label(self.git.scope),
                IdentityScope::ALL
                    .iter()
                    .map(|scope| {
                        let value = SharedString::from(scope.id());
                        (value, git_scope_label(*scope), self.git.scope == *scope)
                    })
                    .collect(),
                {
                    let entity = cx.entity().downgrade();
                    Box::new(move |value, window, cx| {
                        let scope = match value.as_ref() {
                            "global" => IdentityScope::Global,
                            _ => IdentityScope::Local,
                        };
                        let _ = entity.update(cx, |this, cx| {
                            if this.git.scope == scope {
                                return;
                            }
                            this.git.scope = scope;
                            // 切作用域 = 重新读一次（真源 `:34-61` 的 effect 依赖 `scope`）。
                            this.git_load(window, cx);
                        });
                    })
                },
            ),
            None,
            cx,
        ));

        // ② 工作区根：真源把 `root` 原文画出来（`git-identity-settings.tsx:109`）。
        // 没有它，用户不知道"当前仓库"指的是哪个目录。
        rows.push(
            div()
                .w_full()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(SharedString::from(host_root))
                .into_any_element(),
        );

        // ③ 非仓库 + `local`：禁用两个输入并给提示（真源 `:110-112`）。
        let blocked = self
            .git
            .setup
            .as_ref()
            .is_some_and(|setup| !setup.editable_in(self.git.scope));
        if blocked {
            rows.push(
                div()
                    .w_full()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(tr("lithe.git.setup.initializeFirst"))
                    .into_any_element(),
            );
        }

        // ④ 姓名 / 邮箱：各一行（标签 + 输入框 + 保存 + 清除覆盖 + 当前生效值）。
        for field in [IdentityField::Name, IdentityField::Email] {
            rows.push(self.git_identity_row(field, cx));
        }

        // ⑤ 底部三句：逐字段保存的说明、重新加载、加载中 / 失败 / 已保存。
        rows.push(
            div()
                .w_full()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(tr("lithe.git.setup.separateSave"))
                .into_any_element(),
        );
        rows.push(
            h_flex()
                .w_full()
                .items_center()
                .gap_3()
                .child(
                    Button::new("settings-git-reload")
                        .small()
                        .ghost()
                        .label(tr("lithe.git.setup.reload"))
                        .disabled(self.git.busy)
                        .on_click(cx.listener(|this, _, window, cx| this.git_load(window, cx))),
                )
                .when(self.git.busy, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(tr("lithe.git.setup.loading")),
                    )
                })
                .when_some(self.git.error.clone(), |this, error| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().danger)
                            .child(error),
                    )
                })
                .when(self.git.saved.is_some(), |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(tr("lithe.git.setup.saved")),
                    )
                })
                .into_any_element(),
        );

        vec![
            settings_group(
                tr("lithe.git.setup.identity"),
                // 分组第一句是那行说明（真源 `:89`），它是**整组的描述**而不是某一行的。
                {
                    let mut group_rows = vec![
                        div()
                            .w_full()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(tr("lithe.git.setup.identityDescription"))
                            .into_any_element(),
                    ];
                    group_rows.extend(rows);
                    group_rows
                },
                cx,
            )
            .into_any_element(),
            // ⑥ 真的有消费方的设置项（见本函数文档的表）：`confirmBeforeDiscard`。
            settings_group(
                tr("lithe.settings.git.integration"),
                vec![settings_row(
                    "settings-row-git-confirm-discard",
                    tr("lithe.settings.git.confirmDiscard"),
                    Some(tr("lithe.settings.git.confirmDiscardDescription")),
                    {
                        let store = self.store.clone();
                        Switch::new("settings-git-confirm-discard")
                            .small()
                            .checked(settings.confirm_before_discard)
                            .on_change(move |checked, _, cx| {
                                let checked = *checked;
                                // 只写设置：把它推给左栏「更改」视图是**外壳**的事
                                // （订阅 `SettingsStore` 后调
                                // `ChangesView::set_confirm_before_discard`）。
                                store.update(cx, |store, cx| {
                                    store.set_confirm_before_discard(checked, cx)
                                });
                            })
                            .into_any_element()
                    },
                    None,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    /// 「Git」页里的一个身份字段行（真源 `git-identity-settings.tsx:113-165`）。
    ///
    /// 布局与其它页的行不同：输入框 + 两个按钮横排，下面一行「当前生效值」。
    /// 输入框要吃掉整行剩余宽度（144px 的控件档装不下"输入框 + 两个按钮"），
    /// 所以这里**不走** `settings_row` 的左右两栏，而是自己排一个竖排块
    /// （与其它页的行在视觉上仍然同族：同样的 14px 标签、12px 弱化文字、12px 间隔）。
    fn git_identity_row(&self, field: IdentityField, cx: &Context<Self>) -> gpui_kit::AnyElement {
        let draft = self.git_draft(field, cx);
        let save_enabled = git_save_enabled(
            self.git.setup.as_ref(),
            self.git.scope,
            field,
            draft.as_ref(),
            self.git.busy,
        );
        let clear_enabled = git_clear_enabled(self.git.setup.as_ref(), field, self.git.busy);
        // 输入框的禁用条件：读/写在飞、还没读到快照、或非仓库 + `local`（真源 `:129`）。
        let input_enabled = !self.git.busy
            && self
                .git
                .setup
                .as_ref()
                .is_some_and(|setup| setup.editable_in(self.git.scope));
        let effective = self
            .git
            .setup
            .as_ref()
            .and_then(|setup| setup.effective(field))
            .map(str::to_string);

        v_flex()
            .id(("settings-git-field", field.index()))
            .w_full()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(tr(field.label_key())),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        div().flex_1().min_w_0().child(
                            // ⚠️ 单行输入的实际高度来自 `input_h`（`Input` 的同名固有方法
                            // `h` 只对多行输入生效，理由见 `explorer_view.rs:467-473`），
                            // 所以这里不显式写高度、用默认档。
                            Input::new(self.git_input(field)),
                        ),
                    )
                    .child(
                        Button::new(("settings-git-save", field.index()))
                            .small()
                            .label(tr("lithe.git.setup.save"))
                            .disabled(!save_enabled)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.git_save(field, false, window, cx)
                            })),
                    )
                    .child(
                        Button::new(("settings-git-clear", field.index()))
                            .small()
                            .ghost()
                            .label(tr("lithe.git.setup.clear"))
                            .disabled(!clear_enabled)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.git_save(field, true, window, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(match effective {
                        // 真源拼的是 `${t("git.setup.effective")}: ${effective}`
                        // （`git-identity-settings.tsx:155`）。
                        Some(value) => SharedString::from(format!(
                            "{}: {value}",
                            tr("lithe.git.setup.effective")
                        )),
                        None => tr("lithe.git.setup.unconfigured"),
                    }),
            )
            .when(!input_enabled, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.git.setup.initializeFirst")),
                )
            })
            .into_any_element()
    }

    /// **明确空态**页：一句「此分类尚未接入」+ 一句前置条件，**没有任何控件**。
    /// 1. 分类**留在左栏**（用户点得到，不会以为"这个分类不存在"）；
    /// 2. 标题是分类自己的名字（页面标题照常画），正文说明缺的是**什么子系统**；
    /// 3. **不画假控件**（`07-settings-ui.md` §7.3-D）：一个永远不生效的开关比不画更容易骗人，
    ///    所以这里连"置灰的开关"都没有，只有说明文字。
    ///
    /// 图标沿用左栏那一列；`Empty` 的虚线边框按编辑区空状态同一口径关掉
    /// （`empty.rs:74-75` 硬编码了 `border_dashed`，真机界面没有这圈线）。
    fn empty_page(&self, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let prerequisite = self
            .category
            .prerequisite_key()
            .expect("空态分类必须登记一条前置条件文案（`Category::prerequisite_key`）");
        vec![
            v_flex().w_full().py_8().child(
                Empty::new()
                    .border_color(cx.theme().transparent)
                    .gap_3()
                    .px_6()
                    .header(
                        EmptyHeader::new()
                            .max_w_112()
                            .gap_3()
                            .media(
                                EmptyMedia::new().child(
                                    Icon::new(self.category.icon())
                                        .size_10()
                                        .text_color(cx.theme().muted_foreground),
                                ),
                            )
                            .title(
                                EmptyTitle::new()
                                    .text_sm()
                                    .child(tr("lithe.settings.gpui.pageNotAvailableTitle")),
                            )
                            .description(
                                EmptyDescription::new().text_sm().child(tr(prerequisite)),
                            ),
                    ),
            )
            .into_any_element(),
        ]
    }

    /// 底部：左「恢复默认设置…」+ 右「完成」。
    fn footer(&self, cx: &Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .flex_shrink_0()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                // 会打开确认对话框的命令：文案末尾带省略号（设计指南「大小写、标点与符号」）。
                Button::new("settings-restore-defaults")
                    .ghost()
                    .label(tr("lithe.settings.gpui.restoreDefaultsOpen"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.confirm_restore_defaults(window, cx)
                    })),
            )
            .child(
                Button::new("settings-done")
                    .primary()
                    .label(tr("lithe.settings.mac.done"))
                    .on_click(cx.listener(|this, _, window, cx| {
                        // 关闭前把防抖窗口里的最后一次改动落盘，避免"改了没生效就退出"。
                        let store = this.store.clone();
                        store.update(cx, |store, _| store.flush_pending());
                        window.close_dialog(cx);
                    })),
            )
    }

    /// 「恢复默认设置」的确认对话框。
    ///
    /// ⚠️ **有意不照抄 Windows 的文案**：真源是
    /// `settings.mac.restoreDefaultsConfirm` = 「⚠️确认恢复所有配置吗？」
    /// （`windows/tauri/src/i18n/locale.ts`，渲染点 `settings-dialog.tsx:99-107`），
    /// 而设计指南 `docs/design-guides.md:434` 明确点名"您确定要……吗"这类套话是反例。
    /// 所以按指南写成"标题=决策、正文=后果、按钮=结果词"：
    /// 标题「恢复默认设置？」、正文「所有设置都会回到默认值。」、按钮「取消」+「恢复默认设置」。
    /// 这三条里只有按钮文案沿用 Windows 原句，另两条是 gpui 侧新增键（Windows 缺失，本侧补）。
    fn confirm_restore_defaults(&self, window: &mut Window, cx: &mut Context<Self>) {
        let store = self.store.clone();
        let input = self.font_size_input.clone();
        let editor_input = self.editor_font_size_input.clone();

        window.open_dialog(cx, move |dialog, _, cx| {
            let store = store.clone();
            let input = input.clone();
            // 两个数字框都要在确认后回写显示值，所以两个实体都得在**每次重绘**时各克隆一份
            // （闭包是 `Fn`：直接 move 外层捕获的实体做不到，`Entity` 不是 `Copy`）。
            let editor_input = editor_input.clone();
            dialog
                .title(tr("lithe.settings.gpui.restoreDefaultsTitle"))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.settings.gpui.restoreDefaultsBody")),
                )
                .footer(
                    h_flex()
                        .w_full()
                        .gap_2()
                        .justify_end()
                        .child(
                            Button::new("settings-restore-cancel")
                                .label(tr("lithe.ui.cancel"))
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            // 破坏性结果用破坏性样式标记（设计指南「破坏性操作要区分可逆与不可逆」），
                            // 但样式不替代准确用词 —— 文案仍是结果词。
                            Button::new("settings-restore-confirm")
                                .danger()
                                .label(tr("lithe.settings.mac.restoreDefaults"))
                                .on_click(move |_, window, cx| {
                                    store.update(cx, |store, cx| store.restore_defaults(cx));
                                    // 数字框的文字不会自己跟着回落（设置层不回写控件），
                                    // 恢复默认后显式同步一次，免得界面显示一个已经不是当前的数值。
                                    input.update(cx, |input, cx| {
                                        input.set_value(
                                            SharedString::from(format_ui_font_size(
                                                UI_FONT_SIZE_DEFAULT,
                                            )),
                                            window,
                                            cx,
                                        );
                                    });
                                    editor_input.update(cx, |input, cx| {
                                        input.set_value(
                                            SharedString::from(format_editor_font_size(
                                                EDITOR_FONT_SIZE_DEFAULT,
                                            )),
                                            window,
                                            cx,
                                        );
                                    });
                                    window.close_dialog(cx);
                                }),
                        ),
                )
        });
    }

    /// 一个下拉：`Button` + `dropdown_menu_with_anchor`。
    ///
    /// 这就是 gpui-kit `setting::SettingField::dropdown` 自己的实现方式
    /// （`gpui-component-0.6.6/src/setting/fields/dropdown.rs:60-87`）：
    /// `Select` + `SelectState` 更适合"可搜索的长列表"，而这里最多十来个主题。
    fn dropdown(
        &self,
        id: impl Into<ElementId>,
        current_label: SharedString,
        options: Vec<(SharedString, SharedString, bool)>,
        on_select: Box<dyn Fn(SharedString, &mut Window, &mut App) + 'static>,
    ) -> gpui_kit::AnyElement {
        let on_select = std::rc::Rc::new(on_select);
        Button::new(id)
            .outline()
            .w(ControlWidth::Default.length())
            .justify_between()
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .child(current_label),
            )
            .dropdown_caret(true)
            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
                options.iter().fold(menu, |menu, (value, label, checked)| {
                    let on_select = on_select.clone();
                    let value = value.clone();
                    menu.item(
                        PopupMenuItem::new(label.clone())
                            .checked(*checked)
                            .on_click(move |_, window, cx| {
                                on_select(value.clone(), window, cx);
                            }),
                    )
                })
                // 主题可能有十几个，允许滚动（Windows 的下拉也是可滚动列表）。
                .scrollable(true)
            })
            .into_any_element()
    }
}

impl Render for SettingsDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // ⚠️ 三个区域函数都收 `&Context<Self>`（不可变借用）：edition 2024 的 RPIT 会捕获
        // 签名里的生命周期，收 `&mut` 就会让"同一个表达式里连调两次"报 E0499
        // （同 `gpui/crates/workbench/src/lib.rs:63-74` 的结论）。
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.header(cx))
            .child(
                h_flex()
                    .w_full()
                    .flex_1()
                    .min_h_0()
                    .child(self.nav(cx))
                    .child(self.content(cx)),
            )
            .child(self.footer(cx))
    }
}

/// 外观模式下拉的取值常量（内部值，不是界面文案）。
const MODE_SYSTEM: &str = "system";
const MODE_LIGHT: &str = "light";
const MODE_DARK: &str = "dark";

/// 主题下拉的候选：注册表里全部主题名；**当前生效的名字不在列表里时补在第一位**
/// （照 Windows 的 `normalizedThemeOptions`，`macos-settings-panels.tsx:102-113`），
/// 否则用户在设置文件里写了一个已卸载的主题时，下拉会显示一个不在候选里的空值。
///
/// 返回 `(value, label, checked)` 三元组：value 是主题名（设置文件里存的就是它），
/// label 是显示名 —— gpui 的主题没有单独的展示名，两者都是 `themes[].name`。
fn theme_choices(cx: &App, applied: SharedString) -> Vec<(SharedString, SharedString, bool)> {
    let mut names = theme::theme_names(cx);
    if !names.iter().any(|name| name == applied.as_ref()) {
        names.insert(0, applied.to_string());
    }
    names
        .into_iter()
        .map(|name| {
            let label = SharedString::from(name.clone());
            let checked = name == applied.as_ref();
            (SharedString::from(name), label, checked)
        })
        .collect()
}

/// 语言下拉的显示名。
///
/// Windows 的两个选项是**硬编码**的 `English` / `简体中文`
/// （`macos-settings-panels.tsx:187-188`），**不在 i18n catalog 里**
/// （`gpui/crates/shared/locales/README.md` §7 第 5 条登记过这件事），死代码页签
/// `tabs/general-settings.tsx` 用的中文是「英语」/「简体中文」（`07-settings-ui.md` §3.1）。
/// 界面上不许出现字面量，所以这两条是 gpui 侧新增的键（Windows 缺失，本侧补），
/// 中文取死代码页签的「英语」/「简体中文」，英文按语言自称写。
fn language_label(tag: &str) -> SharedString {
    match tag {
        "en-US" => tr("lithe.settings.gpui.languageEnglish"),
        _ => tr("lithe.settings.gpui.languageChinese"),
    }
}

/// 外观模式的下拉显示名。
fn mode_label(mode: AppearanceMode) -> SharedString {
    match mode {
        AppearanceMode::System => tr("lithe.settings.mac.followSystem"),
        AppearanceMode::Light => tr("lithe.settings.mac.light"),
        AppearanceMode::Dark => tr("lithe.settings.mac.dark"),
    }
}

/// UI 字号显示文本（`13` / `16.5`，不补 `.00`）。
fn format_ui_font_size(value: f64) -> String {
    let normalized = crate::schema::normalize_ui_font_size(value);
    if normalized.fract() == 0.0 {
        format!("{}", normalized as i64)
    } else {
        format!("{normalized}")
    }
}

/// 编辑器字号显示文本。它恒为整数档（[`crate::schema::normalize_editor_font_size`]），
/// 所以只有整数一种形态；分开一个函数是为了让"两个字号的口径不同"这件事在调用点看得见。
fn format_editor_font_size(value: f64) -> String {
    format!("{}", crate::schema::normalize_editor_font_size(value) as i64)
}

/// 制表符宽度的显示文本：`2 个空格`（真源 `macos-settings-panels.tsx:321` 的
/// `` `${size} ${t("settings.mac.spaces")}` `` —— 数字在前、单位词在后，中间一个空格）。
fn tab_size_label(size: u32) -> SharedString {
    SharedString::from(format!("{size} {}", tr("lithe.settings.mac.spaces")))
}

/// 终端默认 Shell 下拉的显示名。四个取值与真源
/// `macos-settings-panels.tsx:388-391` 的四个 `<option>` 一一对应。
fn shell_label(id: &str) -> SharedString {
    match id {
        "powershell" => tr("lithe.settings.mac.shellPowerShell"),
        "cmd" => tr("lithe.settings.mac.shellCommandPrompt"),
        "wsl" => tr("lithe.settings.mac.shellWsl"),
        // 空串 = 「系统默认」；未知值不可能出现在界面上（`normalize` 有白名单），
        // 真出现了也按系统默认显示，不 panic。
        _ => {
            debug_assert_eq!(id, SHELL_SYSTEM_DEFAULT);
            tr("lithe.settings.mac.systemDefault")
        }
    }
}

/// 身份作用域下拉的显示名。两个取值与真源
/// `git-identity-settings.tsx:98-99` 的 `<option value="local">` / `value="global"` 一一对应。
fn git_scope_label(scope: IdentityScope) -> SharedString {
    match scope {
        IdentityScope::Local => tr("lithe.git.setup.local"),
        IdentityScope::Global => tr("lithe.git.setup.global"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 字号显示文本：整数不补 `.00`，半步长保留一位小数。
    #[test]
    fn font_size_labels_are_compact() {
        assert_eq!(format_ui_font_size(13.0), "13");
        assert_eq!(format_ui_font_size(16.5), "16.5");
        assert_eq!(format_ui_font_size(13.24), "13");
        // 编辑器字号只有整数档：小数先被四舍五入，不会出现 "14.5"。
        assert_eq!(format_editor_font_size(14.0), "14");
        assert_eq!(format_editor_font_size(14.6), "15");
    }

    /// 分类清单必须正好是 Windows 去掉 AI 之后的 10 个分类 + gpui 侧的「外观」。
    ///
    /// ⚠️ 这条断言是"左栏不许和真源漂移"的守卫：加分类要同时有页面（内容或空态）与文案。
    #[test]
    fn categories_match_the_windows_list_without_ai() {
        let ids: Vec<&str> = Category::ALL.iter().map(|category| category.id()).collect();
        assert_eq!(
            ids,
            [
                "general",
                // 真源对话框里**没有** appearance（那是死代码页签，§5.1）；本侧必须留着它，
                // 因为主题要有落点。它是这张表与真源唯一的差集。
                "appearance",
                "project",
                "run",
                "editor",
                "keyboard",
                "terminal",
                "lsp",
                "git",
                "logs",
                "updates",
            ],
            "顺序与取值都要跟 settings-dialog.tsx:35-48 一致（AI 两个分类 + 本侧新增的 appearance 除外）"
        );
        assert_eq!(Category::DEFAULT, Category::General);
    }

    /// **每个分类要么是"实现页"、要么有一条前置条件**，两者互斥且必居其一。
    ///
    /// 这条守的是空态页的诚实性：漏登记前置条件的空态页会画出一个只有标题的空页
    /// （`empty_page` 里那句 `expect` 会在运行期炸），而实现页如果被记成空态，
    /// `content()` 的分支与 `IMPLEMENTED` 就会各说各话。
    #[test]
    fn every_category_is_implemented_or_declares_a_prerequisite() {
        for category in Category::ALL {
            let implemented = Category::IMPLEMENTED.contains(&category);
            let prerequisite = category.prerequisite_key();
            assert_eq!(
                implemented,
                prerequisite.is_none(),
                "{:?} 的实现状态与前置条件登记不一致",
                category
            );
        }
        // `IMPLEMENTED` 里的每一项都必须在 `ALL` 里（反过来由上一条断言覆盖）。
        for category in Category::IMPLEMENTED {
            assert!(Category::ALL.contains(&category), "{category:?} 不在 ALL 里");
        }
    }

    /// 分类 id 是诊断行 `S1_SETTINGS dialog_opened category=…` 的取值，也是命令面板
    /// "打开到指定分类"要传的值 —— 改它就是改可 grep 的契约。
    #[test]
    fn category_ids_are_probe_tokens() {
        assert_eq!(Category::General.id(), "general");
        assert_eq!(Category::Appearance.id(), "appearance");
        assert_eq!(Category::Editor.id(), "editor");
        assert_eq!(Category::Terminal.id(), "terminal");
        assert_eq!(Category::Lsp.id(), "lsp");
        assert_eq!(Category::Updates.id(), "updates");
    }

    /// rem 换算：`rems(P / 16.)` 在 16px 基准下必须等于规格像素值。
    #[test]
    fn rem_conversion_matches_the_spec_pixels() {
        assert_eq!(rem_px(px(16.), DIALOG_WIDTH), px(820.));
        assert_eq!(rem_px(px(16.), DIALOG_HEIGHT), px(620.));
        assert_eq!(rem_px(px(16.), NAV_WIDTH), px(190.));
        // 基准字号翻倍（uiFontSize 26）时长度等比例放大：这就是用 rem 的意义。
        assert_eq!(rem_px(px(32.), NAV_WIDTH), px(380.));
    }

    /// **钩子没登记 → 明确空态**（阶段 15 的降级分支）。
    ///
    /// 三件事一起钉住：
    /// 1. `git_identity_page()` 在没人登记时返回 `None`（`GitPageState::new` 因此拿到 `None`）；
    /// 2. 「Git」分类**仍然登记着前置条件**（空态页要有一句话可画，`empty_page` 的 `expect`
    ///    才不会在运行期炸）—— 所以它不在 `IMPLEMENTED` 里，这一点与实现页的差别是**有意的**：
    ///    Git 页的能力取决于宿主有没有登记钩子，而那张表是编译期常量。
    /// 3. 分类 id 仍是命令面板"打开到指定分类"要传的值。
    #[test]
    fn git_page_degrades_to_the_empty_state_without_a_host() {
        crate::identity::set_git_identity_host(None);
        assert!(
            git_identity_page().is_none(),
            "没有宿主登记钩子时，Git 页必须走空态分支"
        );
        assert!(
            !Category::IMPLEMENTED.contains(&Category::Git),
            "Git 页是**条件实现**：宿主没登记钩子时它就是空态，不能记进 IMPLEMENTED"
        );
        assert_eq!(
            Category::Git.prerequisite_key(),
            Some("lithe.settings.gpui.prerequisiteGit")
        );
        assert_eq!(Category::Git.id(), "git");
        assert_eq!(Category::Git.label_key(), "lithe.settings.tabs.git");
    }

    /// 「保存 / 清除」两个按钮的禁用判据（真源 `git-identity-settings.tsx:133-139,147`）。
    ///
    /// 界面调用点与这里用的是**同一个函数**（`git_save_enabled` / `git_clear_enabled`），
    /// 所以这条测试同时是"按钮不会永远可点、也不会永远禁用"的守卫。
    #[test]
    fn git_buttons_follow_the_truth_source_gates() {
        let setup = IdentitySetup {
            is_repository: true,
            configured_name: Some("Lithe Dev".to_string()),
            effective_name: Some("Lithe Dev".to_string()),
            ..IdentitySetup::default()
        };
        // 有仓库 + 新值 → 可保存；同一个值 / 空值 / 非法值 → 不可保存。
        assert!(git_save_enabled(
            Some(&setup),
            IdentityScope::Local,
            IdentityField::Name,
            "Other",
            false
        ));
        assert!(!git_save_enabled(
            Some(&setup),
            IdentityScope::Local,
            IdentityField::Name,
            "Lithe Dev",
            false
        ));
        assert!(!git_save_enabled(
            Some(&setup),
            IdentityScope::Local,
            IdentityField::Name,
            "  ",
            false
        ));
        // 读/写在飞 → 两个按钮都禁用（真源 `busy` 挡在最前面）。
        assert!(!git_save_enabled(
            Some(&setup),
            IdentityScope::Local,
            IdentityField::Name,
            "Other",
            true
        ));
        assert!(!git_clear_enabled(Some(&setup), IdentityField::Name, true));
        // 还没读到快照 → 两个按钮都禁用（`state === null`）。
        assert!(!git_save_enabled(
            None,
            IdentityScope::Local,
            IdentityField::Name,
            "Other",
            false
        ));
        assert!(!git_clear_enabled(None, IdentityField::Name, false));
        // 「清除覆盖」只在**该作用域里写着值**时可点。
        assert!(git_clear_enabled(Some(&setup), IdentityField::Name, false));
        assert!(!git_clear_enabled(Some(&setup), IdentityField::Email, false));
    }
}
