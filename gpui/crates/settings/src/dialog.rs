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

use std::path::PathBuf;

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
    InteractiveElement as _, IntoElement, KeyBinding, ParentElement as _, PathPromptOptions,
    Render, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription, Window, div,
    prelude::FluentBuilder as _, rems,
};

use lithe_gpui_shared::icons::{idea, idea_icon_svg};
use lithe_gpui_shared::{tr, tr_args};

use crate::identity::{
    GitIdentityPage, IdentityField, IdentityScope, IdentitySetup, git_identity_page,
    identity_value_is_valid, identity_value_rejection,
};
use crate::project::{
    DetectedJdk, DetectedMaven, EffectiveToolchain, MINIMUM_JAVA_MAJOR, Overrides,
    ProjectEnvironment, ToolSource, discover,
};
use crate::row::{ControlWidth, RowActivation, page_stack, page_title, settings_group, settings_row};
use crate::run::{RunConfigurationView, RunProjectView};
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

/// 打开对话框时要不要**立刻**探一次数据（纯判据，可直接单测）。
///
/// 「项目」页要："探测"要起子进程，所以它跟「Git」页一样是**懒加载**（点进这一页才探），
/// 而懒加载的唯一触发器是左栏的点击回调 —— 如果对话框是**带着这一页**创建的
/// （[`open_settings_dialog_at`] 的 `category`），那条点击就永远不会发生，页面会停在
/// 「正在检测…」。
///
/// 「运行配置」页同理，但它不是"起子进程"而是"读整个工作区的文件树 + 跑一遍探测器"：
/// `runConfig.generate` 会 `scan(root)`（深度 ≤ 6、目录数 ≤ 4000），所以同样不该在构造期跑。
///
/// 「LSP」页（阶段 18）也进这张表：它的「检测到的语言服务器」那一组与「项目」页**共用同一份
/// 探测结果**（判据见 [`detected_language_servers`]），所以这一页同样要懒加载一次 ——
/// **不为它多探一次**：`project_load` 是共享的那一份（判据见 `SettingsDialog::lsp_page`）。
///
/// 判据单独抽出来，是为了让这条分支在测试里可见（它需要窗口才能端到端跑）。
fn page_loads_on_open(category: Category) -> bool {
    matches!(category, Category::Project | Category::Run | Category::Lsp)
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

    // 停在「项目」/「运行配置」页时**立刻探一次**。
    //
    // 这两页的数据都不在设置文件里，只有"点左栏那一项"那条路会触发取数 —— 从命令面板 /
    // 探针直接打开到这两页就会永远停在「正在检测…」。判据抽成纯函数 [`page_loads_on_open`]，
    // 所以这条分支有单测钉着（`--open-settings` 走的是默认分类，实测覆盖的是点击那条路）。
    if page_loads_on_open(category) {
        view.update(cx, |view, cx| view.load_page_on_open(category, cx));
    }

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
        let height = rem_px(rem, DIALOG_HEIGHT).min((viewport.height - margin * 2.).max(gpui_kit::Pixels::ZERO));
        // 居中：Dialog 的 y 默认是视口的 1/10（`dialog/dialog.rs:529`），Windows 是垂直居中。
        let margin_top = ((viewport.height - height) / 2.).max(gpui_kit::Pixels::ZERO);

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
    /// 项目 · JDK 与 Maven（`settings.project.title`）—— **实现页**（阶段 16）：
    /// 本机 JDK / Maven 的真实探测值 + 覆盖值 + 刷新。
    Project,
    /// 运行配置（`settings.run.title`）—— **实现页**（阶段 17）：Core 从项目文件识别出的
    /// 可运行目标 + 刷新；只有列表，**没有编辑与运行入口**（原因写在 `run_page`）。
    Run,
    /// 快捷键（`settings.tabs.keyboard`）—— 空态。
    Keyboard,
    /// LSP（`settings.tabs.lsp`）—— **实现页**（阶段 18）：真源三键里**唯一有真消费方**的
    /// `autoCompletion`（打字时是否自动弹补全菜单）+ 一组**真事实**的「已检测语言服务器」。
    /// 另两项（`parameterHints` / `semanticTokens`）与 JDTLS 运行时路径输入框**不画**
    /// （连置灰都不画），理由写在 `lsp_page` 的文档里。
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
    ///
    /// 「项目 · JDK 与 Maven」页（阶段 16）**没有任何条件**：探测是本进程自己做的
    /// （[`crate::project::discover`]），就算什么都没探测到，页面画的也是
    /// "每个字段各说各的为什么没有"（不是空态），所以它**进这张表**。
    ///
    /// 「运行配置」页（阶段 17）同理：数据是 Core 的 `runConfig.generate` 给的
    /// （[`crate::run`]），"一条都没识别到"与"识别失败"都由页面自己如实说明，
    /// **不是**"此分类尚未接入"那种空态，所以也进这张表。
    ///
    /// 「LSP」页（阶段 18）同理：`autoCompletion` 是**真的会改行为**的开关
    /// （落到 `editor/src/completion.rs` 的触发判据），"检测到的语言服务器"那一组画的也是
    /// 真探测值（[`detected_language_servers`]），所以它同样进这张表
    /// —— `prerequisiteLsp` 那句「前置条件：语言服务客户端（补全、参数提示、语义高亮）与
    /// JDTLS 运行时路径设置。」**已经有一半是假的**（补全客户端早就在了，见 `editor/src/completion.rs`），
    /// 这一批把它连同空态一起下线。
    pub const IMPLEMENTED: [Category; 7] = [
        Category::General,
        Category::Appearance,
        Category::Project,
        Category::Run,
        Category::Editor,
        Category::Terminal,
        Category::Lsp,
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
            // 「项目」页自阶段 16 起是**实现页**（真机探测 + 覆盖 + 刷新）：它的"什么都没探测到"
            // 由每个字段各自的「未找到 + 已排除的原因」表达，而不是整页的空态，
            // 所以这里返回 `None`（原来那条 `settings.gpui.prerequisiteProject` 已随之下线）。
            //
            // 「运行配置」页自阶段 17 起同样是实现页：它列出 Core 识别出的启动目标，
            // "一条都没有"与"识别失败"各有一句如实说明（`run_page`），整页空态不再适用 ——
            // 原来那条 `settings.gpui.prerequisiteRun` 也随之下线。
            //
            // 「LSP」页自阶段 18 起同样是实现页：`autoCompletion` 真生效、
            // 「已检测语言服务器」是真事实（`lsp_page`）—— 原来那条
            // `settings.gpui.prerequisiteLsp` 也随之下线（它那句"语言服务客户端"早就不成立了）。
            Self::General
            | Self::Appearance
            | Self::Project
            | Self::Run
            | Self::Editor
            | Self::Terminal
            | Self::Lsp => None,
            Self::Keyboard => Some("lithe.settings.gpui.prerequisiteKeyboard"),
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

/// 「项目 · JDK 与 Maven」页的运行期状态（阶段 16）。
///
/// 与 [`GitPageState`] 同一形状（数据不在设置文件里 → 单独一块），但少了 `host` 那一项：
/// 这一页的数据是**本进程自己探测出来的**（[`crate::project::discover`]），不依赖宿主登记钩子，
/// 所以它**没有"宿主没接线"这条降级分支** —— 只有"探测有没有回来"。
struct ProjectPageState {
    /// 最近一次探测的结论；`None` = 还没探测回来（首帧 / 刷新在飞）。
    environment: Option<ProjectEnvironment>,
    /// 探测在飞。
    busy: bool,
    /// 刚保存成功的提示（真源 `saved`）。
    saved: bool,
    /// 请求代次：刷新 / 保存后晚到的旧回包直接丢掉（与 `GitPageState::generation` 同一口径）。
    generation: u64,
}

impl ProjectPageState {
    fn new() -> Self {
        Self {
            environment: None,
            busy: false,
            saved: false,
            generation: 0,
        }
    }
}

/// 「运行配置」页的运行期状态（阶段 17）。
///
/// 与 [`ProjectPageState`] 同一形状（数据不在设置文件里 → 单独一块），差别只有两点：
///
/// 1. 它需要**工作区根**（`runConfig.generate` 的 `root`），而根只能从宿主登记的钩子里拿
///    （[`crate::identity::host_workspace_root`] 的文档写了为什么只有一个来源）；
///    没登记时 `root` 是 `None`，页面画"打开项目后才能识别"的空态，**不是**空列表。
/// 2. 它多一个 `error`：Core 往返会失败（目录不存在 / 响应形状不对），失败要显示**原因**，
///    与"确实没有可运行配置"（空态）是两件事，不能混成一句话。
struct RunPageState {
    /// 本次取数用的工作区根（宿主没登记时 `None`）。
    root: Option<PathBuf>,
    /// 已经为这一页取过数（区分"还没取"与"没有工作区根"这两件事）。
    requested: bool,
    /// 最近一次识别结果；`None` = 还没拿到（首帧 / 刷新在飞 / 失败）。
    view: Option<RunProjectView>,
    /// 取数在飞。
    busy: bool,
    /// 最近一次失败的一句话（带 Core 的错误码原文，可直接排查）。
    error: Option<SharedString>,
    /// 请求代次：刷新后晚到的旧回包直接丢掉（与 [`ProjectPageState::generation`] 同一口径）。
    generation: u64,
}

impl RunPageState {
    fn new() -> Self {
        Self {
            root: None,
            requested: false,
            view: None,
            busy: false,
            error: None,
            generation: 0,
        }
    }

    /// 这一页现在该画哪一支（**纯判据**，`run_page` 只负责把它画出来）。
    ///
    /// 拆出来是因为这几支很容易互相**冒充**，而它们说的是不同的事实：
    ///
    /// - 「取数失败」不能画成「没有识别到可运行配置」——后者是 Core 走完全部探测器之后的
    ///   结论，写成前者会让用户以为项目里真的没有可运行项；
    /// - 「还没取过数 / 正在取」也不能画成空态（那是在替 Core 下结论）。
    ///
    /// 单测直接钉住这张表（`run_page_never_confuses_a_failure_with_an_empty_result`）。
    fn mode(&self) -> RunPageMode {
        if !self.requested {
            return RunPageMode::Loading;
        }
        if self.root.is_none() {
            return RunPageMode::NoProject;
        }
        if self.error.is_some() {
            return RunPageMode::Failed;
        }
        match self.view {
            Some(_) => RunPageMode::Ready,
            None => RunPageMode::Loading,
        }
    }
}

/// [`RunPageState::mode`] 的取值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunPageMode {
    /// 宿主没登记工作区根 → 「打开项目后才能识别」。
    NoProject,
    /// 取数失败 → 只画原因（`runLoadFailed`），**不画**列表也不画空态。
    Failed,
    /// 还没拿到结果（首次 / 刷新在飞）→ 只画说明与动作行。
    Loading,
    /// 有结果：空表 → 「没有识别到可运行配置」，否则逐条画。
    Ready,
}

/// 生效值那一行里"名字"的取法（真源 `describeEffectiveToolchain` 的 `kind` 参数）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolKind {
    /// JDK：名字是 `toolchain.jdk`（`JDK {version}`），没有版本时退化成裸 `JDK`。
    Jdk,
    /// Maven：名字是 `toolchain.maven`（`Maven {version}`），没有版本时退化成裸 `Maven`。
    Maven,
}

impl ToolKind {
    /// 一个都没找到时那句话的键（真源 `effective-toolchain.ts:38`）。
    fn not_found_key(self) -> &'static str {
        match self {
            Self::Jdk => "lithe.toolchain.javaNotFound",
            Self::Maven => "lithe.toolchain.mavenNotFound",
        }
    }

    /// 有版本时的名字键（带 `{version}` 占位符）。
    fn name_key(self) -> &'static str {
        match self {
            Self::Jdk => "lithe.toolchain.jdk",
            Self::Maven => "lithe.toolchain.maven",
        }
    }

    /// 读不出版本时的裸名字（真源也是硬编码的 `"JDK"` / `"Maven"`，
    /// `effective-toolchain.ts:46-49`：那是**产品名**，不走 catalog）。
    fn bare_name(self) -> &'static str {
        match self {
            Self::Jdk => "JDK",
            Self::Maven => "Maven",
        }
    }
}

/// 生效值那一行的完整内容：主行 + 可选的详情行。
///
/// 拆出来是为了**可单测**：界面调用点与测试用的是同一个函数，
/// 不会出现"测试通过但页面画的是另一句话"这种漂移。
struct EffectiveLine {
    /// 主行文本（真源 `line.text`）。
    text: SharedString,
    /// 主行是不是失败色（真源 `line.tone === "error"`）。
    is_error: bool,
    /// 详情行：覆盖值原文（用不了时）或"试过哪些、为什么不行"（没找到时）。
    detail: Option<SharedString>,
}

/// 一个工具的「生效值」文案（逐条照 `features/run/utils/effective-toolchain.ts:23-58`）。
///
/// 拼法与真源一致：`{模式} → {名字} · {路径}`，来源**只在自动选出时**追加
/// （`configured` / `projectJdk` 这两档已经由模式本身说明了来源）。
fn describe_effective(kind: ToolKind, effective: &EffectiveToolchain) -> EffectiveLine {
    match effective {
        EffectiveToolchain::Resolved {
            mode,
            path,
            version,
            source,
        } => {
            let name = match version {
                Some(version) => tr_args(kind.name_key(), &[("version", version.as_str())]),
                None => SharedString::from(kind.bare_name()),
            };
            let mut parts = vec![name.to_string(), path.clone()];
            if let Some(source) = source {
                parts.push(source_label(*source).to_string());
            }
            EffectiveLine {
                text: SharedString::from(format!(
                    "{} → {}",
                    tr(mode.label_key()),
                    parts.join(" · ")
                )),
                is_error: false,
                detail: None,
            }
        }
        EffectiveToolchain::Unusable {
            path,
            reason,
            language_service_rejection,
        } => {
            // ⚠️ 低于 JDT LS 的 ≥ 21 闸门（`java/src/jdtls.rs` 的 `probe_java`）时，
            // 上面那句只说了"版本 X 低于 21"，用户真正要预判的是**语言服务起不起得来**，
            // 所以再补一句 —— 而且说的是"语言服务无法启动"，不是笼统的"无法使用"。
            // 判据与数值的唯一真源是 `project::MINIMUM_JAVA_MAJOR`（它的文档写了对应
            // `jdtls.rs` 的哪一条判据）。
            let minimum = MINIMUM_JAVA_MAJOR.to_string();
            let below_gate = SharedString::from(format!(
                "{path} · {}",
                tr_args(
                    "lithe.settings.gpui.jdkBelowLanguageServiceMinimum",
                    &[("minimum", minimum.as_str())],
                )
            ));
            EffectiveLine {
                text: tr_args("lithe.toolchain.invalid", &[("message", reason.as_str())]),
                is_error: true,
                // 用户填的原文（或自动选中那一个的路径）要显示出来，否则"用不了"这句话
                // 没说清是哪个值用不了。
                detail: Some(match language_service_rejection {
                    true => below_gate,
                    false => SharedString::from(path.clone()),
                }),
            }
        }
        EffectiveToolchain::NotFound { reason } => EffectiveLine {
            text: tr(kind.not_found_key()),
            is_error: true,
            // 试过哪些位置、各自为什么不行 —— 这就是"可排查的原因"。
            detail: Some(SharedString::from(reason.clone())),
        },
    }
}

/// 「来源」那一栏的文案：`JAVA_HOME` / `PATH` 用真源既有键，
/// 两个**环境变量**来源用带 `{name}` 占位符的新键填变量名。
fn source_label(source: ToolSource) -> SharedString {
    match source.label_arg() {
        Some(name) => tr_args(source.label_key(), &[("name", name)]),
        None => tr(source.label_key()),
    }
}

/// 一行「项目」页草稿输入框的当前值（去首尾空白）。
///
/// 「保存」那一段五个框走同一条口径：空 / 全空白 = 没选（`settings/src/project.rs` 的
/// `non_empty_opt`、`maven_configuration` 都是这个判据），所以空白必须在**写入设置之前**
/// 就抹掉，否则设置文件里会存一串空格、探测层再把它当"选了某个空路径"。
fn draft_value(input: &Entity<InputState>, cx: &App) -> String {
    input.read(cx).value().trim().to_string()
}

/// Maven 配置那一行（`settings.xml` / 本地仓库）的「生效值」：主行 + 是否失败色 + 说明。
///
/// **判定与画分开**：这一层只把 [`ProjectEnvironment`] 已有的事实翻成一句话，不重算路径
/// （重算就会与 `project.rs` 的探测漂移）。四条判据：
///
/// 1. **选了 `settings.xml` 但它不是一个文件** → 失败色 + 「这个路径不存在」（能不能生效
///    由存在性决定，不能让一个不存在的路径看起来像已生效）；
/// 2. **选了 `settings.xml` 且存在** → 正常色 + 「生效：交给 Java 语言服务」；
/// 3. **没选** → 报检测到的那一份：用户级优先，安装级那份只在**与生效值不同**时单列
///    （用户级不存在时生效值就是安装级，再列一次是重复 —— 旧版只读行也是这个口径）；
/// 4. **本地仓库没有覆盖值** → 报推导值 + 来源；**有覆盖值** → 报它并**如实标注尚未生效**
///    （Maven 执行通路还没有，判据见 `schema.rs` 的 `maven_local_repository_path` 文档）。
fn maven_config_effective(
    field: MavenConfigField,
    environment: Option<&ProjectEnvironment>,
) -> EffectiveLine {
    let Some(environment) = environment else {
        return EffectiveLine {
            text: tr("lithe.toolchain.detecting"),
            is_error: false,
            detail: None,
        };
    };
    let config = &environment.maven_config;

    match field {
        MavenConfigField::SettingsXml => match config.effective_settings() {
            Some(path) if config.override_settings_missing => EffectiveLine {
                text: SharedString::from(path.to_string()),
                is_error: true,
                detail: Some(tr("lithe.settings.gpui.overridePathMissing")),
            },
            Some(path) if config.settings_is_overridden() => EffectiveLine {
                text: SharedString::from(path.to_string()),
                is_error: false,
                detail: Some(tr("lithe.settings.gpui.mavenSettingsEffective")),
            },
            Some(path) => EffectiveLine {
                text: SharedString::from(path.to_string()),
                is_error: false,
                detail: config
                    .installation_settings
                    .as_deref()
                    .filter(|installation| *installation != path)
                    .map(SharedString::from),
            },
            None => EffectiveLine {
                text: tr("lithe.settings.gpui.mavenSettingsMissing"),
                is_error: true,
                detail: None,
            },
        },
        MavenConfigField::LocalRepository => {
            let overridden = environment.overrides.maven_local_repository.trim();
            if !overridden.is_empty() {
                return EffectiveLine {
                    text: SharedString::from(overridden.to_string()),
                    is_error: false,
                    detail: Some(tr("lithe.settings.gpui.mavenOverrideNotEffective")),
                };
            }
            match config.local_repository.as_ref() {
                Some(repository) => EffectiveLine {
                    text: SharedString::from(repository.path.clone()),
                    is_error: false,
                    // 这一份是怎么来的：settings.xml 里写的，还是 Maven 的默认位置。
                    detail: Some(tr(repository.source.label_key())),
                },
                None => EffectiveLine {
                    text: tr("lithe.settings.gpui.mavenLocalRepositoryUnknown"),
                    is_error: true,
                    detail: None,
                },
            }
        }
    }
}

/// 「运行配置」页里的一句整页级提示（无项目 / 没有识别到可运行配置）。
///
/// 用 `foreground` 而不是失败色：这两种都是**如实结论**，不是错误
/// （真正的失败走 `runLoadFailed` + `danger`）。
fn run_page_hint(text: SharedString, cx: &Context<SettingsDialog>) -> gpui_kit::AnyElement {
    div()
        .w_full()
        .text_sm()
        .text_color(cx.theme().foreground)
        .child(text)
        .into_any_element()
}

/// 「检测到的语言服务器」那一组的**真事实**（对应真源那句静态文本
/// `settings.mac.detectedServers` / `macos-settings-panels.tsx:431-435` 的 `SettingsGroup`）。
///
/// ## 为什么这里能给出真值
///
/// JDT LS 是 gpui 侧**唯一**的语言服务器（`java/src/service.rs` 一个门面，
/// 启动时打 `S1_JAVA_JDTLS executable=… version=… java=… javaVersion=…`），
/// 而它被"检测"的方式正是真源那句描述说的：**打开受支持的文件时才启动**
/// （`settings.mac.detectedServersDescription` =「语言服务器由已安装的语言扩展检测，
/// 并在打开受支持文件时启动。」）。
///
/// 所以这里读的是**项目页同一次探测**（[`crate::project::discover`]）里的 JDK 事实 ——
/// 那正是"跑 JDT LS 用的那个 JDK"（`java/src/jdtls.rs:440-495` 的 `resolve_runtime`，
/// 判据与闸门见 [`crate::project::MINIMUM_JAVA_MAJOR`]）：
///
/// - 生效值（自动或覆盖）过了 ≥ 21 闸门 ⇒ 报 `JDK 主目录 (版本) · 可执行文件`；
/// - 没过闸门 ⇒ 报同一行，但调用方会用失败色 + 那句"低于语言服务最低要求"；
/// - 一条候选都没有 ⇒ `None`（页面画"还没有可用的语言服务运行时"）。
///
/// ⚠️ **这不是编造**：值全部来自本次真实探测（`java -version` 的输出），
/// 本函数只做"挑选 + 拼一句话"，没有新的探测逻辑。
///
/// ⚠️ **与真源的差别（有意，登记在汇报里）**：真源那组是一句**静态文本**，
/// 本侧把它做成了真事实。而**已经跑起来的那个 JDTLS 会话**的安装信息
/// （`S1_JAVA_JDTLS` 那几项：`executable` / `version` / `launcher` / `config`）
/// 在本侧读不到 —— `JavaLanguageService.installation` 是 `lithe-gpui-java` 的**私有字段**，
/// 而本批的硬规则不允许改那个 crate（只读参考），所以那几项仍在日志里而不在页面上。
fn detected_language_servers(environment: &ProjectEnvironment) -> Option<SharedString> {
    let jdk = environment
        .jdk
        .detected()
        .or_else(|| environment.jdk.closest_below_requirement())?;
    let home = if jdk.home.is_empty() {
        jdk.executable.clone()
    } else {
        jdk.home.clone()
    };
    let version = if jdk.version.is_empty() {
        "-".to_string()
    } else {
        jdk.version.clone()
    };
    Some(SharedString::from(format!(
        "{home} ({version}) · {}",
        jdk.executable
    )))
}

/// 「检测到的安装」那一行的文本（真源 `settings.project.detected` + `path (version); …`）。
fn detected_installations(entries: Vec<String>) -> Option<SharedString> {
    if entries.is_empty() {
        return None;
    }
    Some(SharedString::from(format!(
        "{}: {}",
        tr("lithe.settings.project.detected"),
        entries.join("; ")
    )))
}

/// 「检测到的安装」里一条 JDK：`主目录 (版本)`。
///
/// 版本读不出来时只写路径（真源也是 `version ? \` (${version})\` : ""`，
/// `project-environment-settings.tsx:281`）——**不编一个版本**。
fn jdk_entry(jdk: &DetectedJdk) -> String {
    let path = if jdk.home.is_empty() {
        jdk.executable.clone()
    } else {
        jdk.home.clone()
    };
    if jdk.version.is_empty() {
        path
    } else {
        format!("{path} ({})", jdk.version)
    }
}

/// 「检测到的安装」里一条 Maven：`启动器 (版本)`。见 [`jdk_entry`]。
fn maven_entry(maven: &DetectedMaven) -> String {
    if maven.version.is_empty() {
        maven.executable.clone()
    } else {
        format!("{} ({})", maven.executable, maven.version)
    }
}

/// 「项目」页里三个**可覆盖**的字段。
///
/// 顺序即页面顺序（真源也是 JDK → Maven → Maven JDK，`project-environment-settings.tsx:141-195`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectField {
    /// `javaHomePath` = JDK 主目录。
    Jdk,
    /// `mavenExecutablePath` = Maven 主目录 / 可执行文件。
    Maven,
    /// `mavenJavaHomePath` = Maven 使用的 JDK 主目录。
    MavenJdk,
}

impl ProjectField {
    /// 页面顺序。
    const ALL: [ProjectField; 3] = [Self::Jdk, Self::Maven, Self::MavenJdk];

    /// 诊断行里的稳定 token。
    fn id(self) -> &'static str {
        match self {
            Self::Jdk => "jdk",
            Self::Maven => "maven",
            Self::MavenJdk => "maven_jdk",
        }
    }

    /// 元素 id 用的序号。
    fn index(self) -> usize {
        match self {
            Self::Jdk => 0,
            Self::Maven => 1,
            Self::MavenJdk => 2,
        }
    }

    /// 行标签（真源既有键）。
    fn label_key(self) -> &'static str {
        match self {
            Self::Jdk => "lithe.run.jdkHome",
            Self::Maven => "lithe.run.mavenExecutable",
            Self::MavenJdk => "lithe.run.mavenJdkHome",
        }
    }

    /// 行提示（真源既有键；Maven 那条是 gpui 侧如实改写的新键，理由见脚本）。
    fn hint_key(self) -> &'static str {
        match self {
            Self::Jdk => "lithe.run.jdkHomeHint",
            Self::Maven => "lithe.settings.gpui.mavenExecutableHint",
            Self::MavenJdk => "lithe.run.mavenJdkHomeHint",
        }
    }

    /// 「这一项在 gpui 侧**还没有消费方**」时那句如实标注的键；`None` = 有真消费方。
    ///
    /// ## 为什么 Maven 的两项要标"尚未生效"（已核实，不是猜）
    ///
    /// 1. gpui 全仓**没有任何地方执行 `mvn`**：`maven.scan`（`workbench/src/maven.rs`）
    ///    是 Core 进程内的项目描述符解析，右侧 Maven 工具窗只呈现它的结论；
    ///    `runConfig.createLaunchPlan` 在 Core 里存在，但 gpui 侧没有采购它的一页
    ///    （`settings` 的运行配置页只读 `runConfig.generate`，页面上也写明了"点击启动尚未接入"）。
    /// 2. 语言服务那一侧**故意不登记**这两个键：`java/src/toolchain.rs` 的模块文档写明
    ///    "后两个键故意不放进这个槽：登记了没人读只会把'存了不生效'从设置页搬到这一层"。
    /// 3. 所以这两个键今天的状态是"**存了没人读**"。页面的口径是
    ///    **不藏起来、但要标出来**（HANDOFF §4：用户需要看到自己填的值，也必须知道它现在不起作用）。
    ///
    /// ⚠️ 与 `javaHomePath` 的区别就在这里：那一个自阶段 16 起**真的生效**
    /// （`workbench/src/workspace.rs` 的 `register_java_toolchain` → `java/src/toolchain.rs`
    /// → `jdtls::resolve_runtime`），所以它**不带**这个标注。
    fn pending_key(self) -> Option<&'static str> {
        match self {
            Self::Maven | Self::MavenJdk => {
                Some("lithe.settings.gpui.mavenOverrideNotEffective")
            }
            Self::Jdk => None,
        }
    }
}

/// 「项目 · JDK 与 Maven」页里 **Maven 自己那份配置**的两个可覆盖字段（本批新增）。
///
/// 与 [`ProjectField`] 分开的原因：那三个走"探测一个工具链"那条判定
/// （[`EffectiveToolchain`]，`Resolved` / `Unusable` / `NotFound` 三态），
/// 这两个走"用哪份 settings.xml、哪个本地仓库"（[`crate::project::MavenConfiguration`]），
/// 形状、判定与消费方都不同 —— 塞进同一个枚举会让每个 `match` 都要写"这一支不可能"。
///
/// 顺序即页面顺序（真源 `project-environment-settings.tsx:287-314` 也是 settings.xml → 本地仓库）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum MavenConfigField {
    /// 用户 `settings.xml`（真源同名项目级字段 `maven.settingsPath`）。
    SettingsXml,
    /// 本地仓库（真源 `maven.localRepositoryPath`）。
    LocalRepository,
}

impl MavenConfigField {
    /// 页面顺序。
    const ALL: [MavenConfigField; 2] = [Self::SettingsXml, Self::LocalRepository];

    /// 诊断行 / 元素 id 里的稳定 token。
    fn id(self) -> &'static str {
        match self {
            Self::SettingsXml => "settings_xml",
            Self::LocalRepository => "local_repository",
        }
    }

    /// 元素 id 用的序号。
    fn index(self) -> usize {
        match self {
            Self::SettingsXml => 0,
            Self::LocalRepository => 1,
        }
    }

    /// 行标签。
    ///
    /// `settings.xml` 是**文件名**（代码标识符，不是文案，所以不走 i18n —— 真源同一行也是
    /// 字面量，`project-environment-settings.tsx:290`）；本地仓库用真源既有键。
    fn label(self) -> SharedString {
        match self {
            Self::SettingsXml => SharedString::from("settings.xml"),
            Self::LocalRepository => tr("lithe.maven.localRepository"),
        }
    }

    /// 行提示（两条都是 gpui 侧新增键：真源只有 placeholder「自动检测」，没有说明会挑哪一份）。
    fn hint_key(self) -> &'static str {
        match self {
            Self::SettingsXml => "lithe.settings.gpui.mavenSettingsHint",
            Self::LocalRepository => "lithe.settings.gpui.mavenLocalRepositoryHint",
        }
    }

    /// 系统对话框里选**文件**还是**目录**。
    ///
    /// 照真源 `:305` 的 `choose(key !== "settingsPath", …)`：`settings.xml` 是文件，
    /// 本地仓库是目录。
    fn picks_file(self) -> bool {
        matches!(self, Self::SettingsXml)
    }
}

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
    /// 「项目 · JDK 与 Maven」页的五个覆盖值输入框（阶段 16 + 本批的 Maven 配置两行）。值是**草稿**：
    /// 只有点「保存」才写进设置文件并重新探测（与真源的显式保存同一条口径）。
    java_home_input: Entity<InputState>,
    maven_executable_input: Entity<InputState>,
    maven_java_home_input: Entity<InputState>,
    /// Maven 用户 `settings.xml`（`mavenSettingsPath`）。
    maven_settings_input: Entity<InputState>,
    /// Maven 本地仓库（`mavenLocalRepositoryPath`）。
    maven_local_repository_input: Entity<InputState>,
    /// 「项目」页的运行期状态（探测结论）。
    project: ProjectPageState,
    /// 「运行配置」页的运行期状态（Core 识别出的启动目标）。
    run: RunPageState,
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
        // 构造期读一次设置快照：三个「项目」页输入框的初值来自它（`Settings` 是纯数据，克隆代价可忽略）。
        let saved = store.read(cx).settings().clone();
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

        // 「项目」页的五个覆盖值输入框：初值 = 设置文件里已经保存的值（真源也是拿
        // `inspected.toolchain` 填输入框，`project-environment-settings.tsx:52-76`）。
        // 它们同样是**草稿**：改动只重绘，写入发生在「保存」被点的那一刻。
        let java_home_input =
            cx.new(|cx| InputState::new(window, cx).default_value(saved.java_home_path.clone()));
        let maven_executable_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(saved.maven_executable_path.clone())
        });
        let maven_java_home_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(saved.maven_java_home_path.clone())
        });
        let maven_settings_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(saved.maven_settings_path.clone())
        });
        let maven_local_repository_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(saved.maven_local_repository_path.clone())
        });

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

        // 「项目」页的五个覆盖值输入框：**只重绘**（「保存」按钮的可点性跟着草稿走），
        // 不写任何东西 —— 与上面两个 Git 输入框同一条理由：它们是草稿，
        // 写入发生在「保存」被点的那一刻（真源同样是显式保存）。
        for input in [
            &java_home_input,
            &maven_executable_input,
            &maven_java_home_input,
            &maven_settings_input,
            &maven_local_repository_input,
        ] {
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
            java_home_input,
            maven_executable_input,
            maven_java_home_input,
            maven_settings_input,
            maven_local_repository_input,
            project: ProjectPageState::new(),
            run: RunPageState::new(),
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
                        // 「项目」页同理：**第一次切到这一页才探测**。探测要起子进程
                        // （每个 JDK 候选一次 `java -version`、Maven 一次 `mvn -version`），
                        // 挂在构造期会让"打开设置对话框"为一张没被看过的页付钱。
                        if *category == Category::Project
                            && this.project.environment.is_none()
                            && !this.project.busy
                        {
                            this.project_load(cx);
                        }
                        // 「运行配置」页也是懒加载，代价换成"读一遍工作区文件树 + 跑探测器"
                        // （`workspace.snapshot` + `runConfig.generate`）。
                        //
                        // 失败之后**不再自动重试**（`error.is_some()` 就不取数了）：失败原因已经
                        // 画在页面上，重试是用户按「重新识别」的显式动作，来回切分类不该反复跑 Core。
                        if *category == Category::Run
                            && this.run.view.is_none()
                            && this.run.error.is_none()
                            && !this.run.busy
                        {
                            this.run_load(cx);
                        }
                        // ⚠️ 「LSP」页也要走这一条：它那组「已检测语言服务器」与「项目」页
                        // **共用同一份探测结果**（判据见 `detected_language_servers`），
                        // 而 `page_loads_on_open` 只在"带着 LSP 分类打开对话框"时兜住
                        // （命令面板 / 启动探针那条路）。只点左栏进来时这里就是唯一的触发器 ——
                        // 不写这一条页面会**永远停在「正在检测…」**（实测：`lsp-page-top.png`
                        // 第一版就是这一格）。
                        if *category == Category::Lsp
                            && this.project.environment.is_none()
                            && !this.project.busy
                        {
                            this.project_load(cx);
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
                    // 「项目 · JDK 与 Maven」页（阶段 16）：真机探测出的 JDK / Maven 生效值
                    // + 可覆盖 + 可刷新。探测不到时**不是整页空态**，而是每个字段各说各的原因。
                    Category::Project => self.project_page(cx),
                    // 「运行配置」页（阶段 17）：Core 从项目文件识别出的启动目标（只读列表）
                    // + 刷新。没有编辑与运行入口 —— 理由画在页面上（`run_page` 的最后一段）。
                    Category::Run => self.run_page(cx),
                    // 「Git」页（阶段 15）：提交身份 + 一个真有消费方的开关。
                    // 钩子没登记时 `git_page` 自己退回明确空态。
                    Category::Git => self.git_page(&settings, cx),
                    // 「LSP」页（阶段 18）：真源三键里唯一有真消费方的那一个（`autoCompletion`）
                    // + 一组真事实的「已检测语言服务器」。另两项与 JDTLS 路径输入框**不画**
                    // （理由写在 `lsp_page` 的文档里）。
                    Category::Lsp => self.lsp_page(&settings, cx),
                    // 其余分类是**明确空态**：只有一句前置条件，没有任何控件。
                    Category::Keyboard | Category::Logs | Category::Updates => {
                        self.empty_page(cx)
                    }
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

    /// 「项目 · JDK 与 Maven」页里的一个可覆盖字段。
    ///
    /// 三个字段的**键名与真源逐字相同**（`javaHomePath` / `mavenExecutablePath` /
    /// `mavenJavaHomePath`，`project-environment-settings.tsx:141-195`），
    /// 标签 / 提示也直接复用真源既有键（含 `-Hint` 那一组）。
    fn project_field_input(&self, field: ProjectField) -> &Entity<InputState> {
        match field {
            ProjectField::Jdk => &self.java_home_input,
            ProjectField::Maven => &self.maven_executable_input,
            ProjectField::MavenJdk => &self.maven_java_home_input,
        }
    }

    /// Maven 配置那两行的输入框。
    fn maven_config_input(&self, field: MavenConfigField) -> &Entity<InputState> {
        match field {
            MavenConfigField::SettingsXml => &self.maven_settings_input,
            MavenConfigField::LocalRepository => &self.maven_local_repository_input,
        }
    }

    /// 「选择…」（工具链那三行）：系统对话框挑一个**目录**。
    ///
    /// 三个字段都为目录：真源同一处的 `choose(true, …)` 就是 `directory: true`
    /// （`project-environment-settings.tsx:197-260`）；`javaHomePath` 也允许直接填
    /// `java` 可执行文件（探测层认），但**挑**的时候给目录 —— 允许两者会让
    /// `IFileOpenDialog` 的行为在平台上不一致。
    fn project_pick_toolchain(
        &self,
        field: ProjectField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_pick(
            self.project_field_input(field).clone(),
            false,
            tr(field.label_key()),
            field.id(),
            window,
            cx,
        );
    }

    /// 「选择…」（Maven 配置那两行）：`settings.xml` 挑文件、本地仓库挑目录。
    fn project_pick_maven_config(
        &self,
        field: MavenConfigField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_pick(
            self.maven_config_input(field).clone(),
            field.picks_file(),
            field.label(),
            field.id(),
            window,
            cx,
        );
    }

    /// 五个「选择…」按钮的公共实现：开系统对话框 → 把选中的路径写进这一行的**草稿**。
    ///
    /// 用 gpui **自带**的 `App::prompt_for_paths`（`gpui-pre-0.3.6/src/app.rs:1687-1692`；
    /// Windows 实现是真的 `IFileOpenDialog`，跑在专用线程上，本线程只拿一个 `Receiver` 就返回），
    /// 零新增依赖 —— 与「文件 → 打开文件夹」是同一条通路（`workbench/src/workspace.rs` 的
    /// `ShellWorkspace::open_project_picker`）。
    ///
    /// ⚠️ 本页早先的注释写"gpui 侧没有文件对话框依赖"，那是**不完整**的结论
    /// （`project_menu.rs:108-113` 只查了 `rfd` / `tinyfiledialogs` / `native-dialog` 三个 crate，
    /// 漏了 gpui 自带的这一条；`workspace.rs:1740-1743` 已经推翻过它）。
    ///
    /// 写入的只是草稿：真正落盘仍由「保存」负责（与真源"选择器只写 draft"一致）。
    /// 选完打一行 `S1_SETTINGS_PROJECT run=pick …`，与 `discover` 的数据行、`run=save` 合起来
    /// 就能回答"这个路径是手打、挑来的还是探测到的"。
    fn project_pick(
        &self,
        input: Entity<InputState>,
        files: bool,
        label: SharedString,
        target: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files,
            // 文件与目录**互斥**：`files` 为真时只选文件（`settings.xml`），否则只选目录。
            directories: !files,
            // 单选：每一行只有一个路径，多选没有语义（真源同样是 `multiple: false`）。
            multiple: false,
            prompt: Some(label),
        });
        cx.spawn_in(window, async move |this, async_cx| {
            // 两层 `Result`：外层是 oneshot 通道（送信端被丢），内层是平台侧的错误
            // （Linux 打不开选择器时会给）。两者都不是"用户取消"，所以各自打一行。
            let result = match picked.await {
                Ok(Ok(result)) => result,
                Ok(Err(error)) => {
                    eprintln!(
                        "S1_SETTINGS_PROJECT run=pick target={target} state=failed error={error}"
                    );
                    return;
                }
                Err(_) => {
                    eprintln!(
                        "S1_SETTINGS_PROJECT run=pick target={target} state=cancelled reason=channel-closed"
                    );
                    return;
                }
            };
            let Some(path) = result.and_then(|paths| paths.into_iter().next()) else {
                eprintln!("S1_SETTINGS_PROJECT run=pick target={target} state=cancelled");
                return;
            };
            let value = path.to_string_lossy().to_string();
            println!("S1_SETTINGS_PROJECT run=pick target={target} state=picked path={value}");
            // 选中之后照常是**草稿**：与手打一个字符等价，只是省掉粘贴。
            let _ = async_cx.update(move |window, cx| {
                let _ = this.update(cx, |this, cx| {
                    input.update(cx, |input, cx| {
                        input.set_value(SharedString::from(value), window, cx);
                    });
                    this.project.saved = false;
                    cx.notify();
                });
            });
        })
        // `detach()` 而不是丢掉：gpui 的 `Task` 一 drop 就**取消**（选择器刚打开就被取消）。
        .detach();
    }

    /// 「清空」：把这一行的覆盖值草稿置空 = 回到自动检测（真源 `:261-273` 的「清除」）。
    ///
    /// 只清草稿，不写设置：写入仍由「保存」负责 —— 与真源"清除按钮改的是同一个 draft"一致。
    fn project_clear(&mut self, field: ProjectField, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.project_field_input(field).clone();
        input.update(cx, |input, cx| {
            input.set_value(SharedString::from(""), window, cx);
        });
        self.project.saved = false;
        self.project_diagnose(&format!("run=clear field={}", field.id()));
        cx.notify();
    }

    /// 「清空」（Maven 配置那两行）：同样只清草稿 = 回到自动检测/推导。
    fn project_clear_maven_config(
        &mut self,
        field: MavenConfigField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = self.maven_config_input(field).clone();
        input.update(cx, |input, cx| {
            input.set_value(SharedString::from(""), window, cx);
        });
        self.project.saved = false;
        self.project_diagnose(&format!("run=clear field={}", field.id()));
        cx.notify();
    }

    /// 一行 `S1_SETTINGS_PROJECT` 诊断（可 grep；走 stdout，与 `discover` 的数据行同前缀）。
    ///
    /// 数据本身那一行由 [`crate::project::discover`] 打
    /// （`jdk=… source=… version=… maven=… localRepo=…`）；这里补的是"页面做了什么"。
    /// 两者合起来就能回答"截图里的值来自哪一次探测"。
    fn project_diagnose(&self, detail: &str) {
        println!("{} {detail}", crate::project::PROJECT_DIAGNOSTIC_TAG);
    }

    /// 重新探测（真源 `load()`，`project-environment-settings.tsx:47-82`）。
    ///
    /// **用设置文件里已保存的覆盖值**，不是输入框草稿：真源的「重新加载并检测」也是先丢掉草稿
    /// 再 `load()`。
    ///
    /// 为什么整段探测要进 `background_spawn`：`discover` 会同步起子进程
    /// （每个 JDK 候选一次 `java -version`，Maven 一次 `mvn -version`；Windows 上 `mvn.cmd`
    /// 还要先拉起 `cmd.exe`），在 UI 线程上跑会让整个对话框卡住 ——
    /// 与 `lithe_gpui_shared::core_client` 的调用方同一条口径。
    fn project_load(&mut self, cx: &mut Context<Self>) {
        self.project.generation = self.project.generation.wrapping_add(1);
        let generation = self.project.generation;
        self.project.busy = true;
        self.project.saved = false;
        cx.notify();

        let settings = self.store.read(cx).settings().clone();
        let overrides = Overrides {
            java_home: settings.java_home_path.clone(),
            maven_executable: settings.maven_executable_path.clone(),
            maven_java_home: settings.maven_java_home_path.clone(),
            maven_settings: settings.maven_settings_path.clone(),
            maven_local_repository: settings.maven_local_repository_path.clone(),
        };
        self.project_diagnose(&format!("run=discover overrides={}", overrides.count()));

        cx.spawn(async move |this, cx| {
            let environment = cx
                .background_spawn(async move { discover(&overrides) })
                .await;
            let empty = environment.has_no_toolchain();
            let _ = this.update(cx, |this, cx| {
                // 晚到的旧回包丢掉（刷新/保存各推一次代次，与 Git 页的 `generation` 同一口径）。
                if this.project.generation != generation {
                    return;
                }
                this.project.busy = false;
                this.project.environment = Some(environment);
                this.project_diagnose(&format!(
                    "run=discover result=ok generation={generation} no_toolchain={empty}"
                ));
                cx.notify();
            });
        })
        .detach();
    }

    /// 「保存」：把五个草稿写进**全局设置文件**，然后按新值重新探测。
    ///
    /// ⚠️ 真源的保存写的是**项目级**文件（`.lithe/run/local.json`，经
    /// `runConfig.updateOptions`；Maven 那两行写的是 Maven 工具窗的项目本地配置）；
    /// 本侧没有项目级存储与那条 Core 通路，所以写的是全局设置文件，
    /// 页面上也用 [`Self::project_page`] 的第一句如实说明（`settings.gpui.projectScopeGlobal`）。
    fn project_save(&mut self, cx: &mut Context<Self>) {
        let java_home = draft_value(&self.java_home_input, cx);
        let maven_executable = draft_value(&self.maven_executable_input, cx);
        let maven_java_home = draft_value(&self.maven_java_home_input, cx);
        let maven_settings = draft_value(&self.maven_settings_input, cx);
        let maven_local_repository = draft_value(&self.maven_local_repository_input, cx);
        let state = |value: &str| if value.is_empty() { "-" } else { "set" };
        self.project_diagnose(&format!(
            "run=save javaHome={} mavenExecutable={} mavenJavaHome={} mavenSettings={} mavenLocalRepo={}",
            state(&java_home),
            state(&maven_executable),
            state(&maven_java_home),
            state(&maven_settings),
            state(&maven_local_repository),
        ));

        let store = self.store.clone();
        store.update(cx, |store, cx| {
            store.set_java_home_path(java_home, cx);
            store.set_maven_executable_path(maven_executable, cx);
            store.set_maven_java_home_path(maven_java_home, cx);
            store.set_maven_settings_path(maven_settings, cx);
            store.set_maven_local_repository_path(maven_local_repository, cx);
        });
        // 保存之后立刻按新值重探一次（真源保存后也会刷新运行侧的视图）；`saved` 要在
        // `project_load` 之后置位，否则会被它清掉。
        self.project_load(cx);
        self.project.saved = true;
    }

    /// 「项目 · JDK 与 Maven」页（阶段 16；真源
    /// `components/project-environment-settings.tsx` + `features/run/components/effective-toolchain.tsx`）。
    ///
    /// 画出来的每一格都能在这两个地方找到出处：
    /// - **生效值**：`{模式} → {名字} · {路径} · {来源}`，逐条照 `describeEffectiveToolchain`；
    /// - **检测到的安装**：真源同名那一行（`settings.project.detected`），列出**全部**候选。
    ///
    /// 没找到时的表现：这一格变成失败色的「未找到 JDK/Maven」，**下面紧跟一行"试过哪些位置、
    /// 各自为什么不行"**（`ProjectEnvironment` 的 `rejected`）—— 这就是任务要的"可排查的原因"。
    /// 页面级的 `Empty` 空态只在**连一个工具链都没有**时出现（[`ProjectEnvironment::has_no_toolchain`]），
    /// 文案也不是「此分类尚未接入」。
    ///
    /// 与真源的**有意差异**（逐条登记在这里与汇报里）：
    /// - 真源画「浏览」按钮（系统目录对话框），**本批已对齐**：五个路径行都有「选择…」
    ///   （gpui 自带的 `prompt_for_paths`，零新增依赖，判据见 [`Self::project_pick`]）；
    /// - 真源的 `settings.xml` / 本地仓库两行写进 Maven 工具窗的**项目本地配置**，
    ///   本侧没有那条通路，改成**全局设置里的覆盖值**（`mavenSettingsPath` /
    ///   `mavenLocalRepositoryPath`），并如实标注哪一个真的生效：
    ///   `settings.xml` 会经 `mavenContext.settingsPath` 交给语言服务
    ///   （`java.configuration.maven.userSettings`），本地仓库今天还没有消费方；
    /// - 真源有"项目 Maven Wrapper"这一级候选，本侧拿不到工作区根 → 没做（提示文案也如实改写）。
    fn project_page(&self, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let environment = self.project.environment.as_ref();
        let detecting = || EffectiveLine {
            text: tr("lithe.toolchain.detecting"),
            is_error: false,
            detail: None,
        };

        let mut toolchain_rows: Vec<gpui_kit::AnyElement> = Vec::new();

        // ① 作用域：本侧是**全局**覆盖值（真源那句 `settings.project.scope` 说的是"当前项目"，
        //    照抄会撒谎）。理由逐条写在 `extract-locale.mjs` 的 GPUI_ONLY_KEYS 里。
        toolchain_rows.push(
            div()
                .w_full()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(tr("lithe.settings.gpui.projectScopeGlobal"))
                .into_any_element(),
        );

        // ② 一个工具链都没探测到 → 一句如实的前提说明（**不是**「此分类尚未接入」）。
        if environment.is_some_and(ProjectEnvironment::has_no_toolchain) {
            toolchain_rows.push(
                div()
                    .w_full()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(tr("lithe.settings.gpui.projectNothingDetected"))
                    .into_any_element(),
            );
        }

        // ③ 三个字段（标签 / 提示全部复用真源既有键）。
        let jdk_candidates = || {
            environment.and_then(|environment| {
                detected_installations(environment.jdk.candidates.iter().map(jdk_entry).collect())
            })
        };
        for field in ProjectField::ALL {
            let effective = match (field, environment) {
                (ProjectField::Jdk, Some(environment)) => {
                    describe_effective(ToolKind::Jdk, &environment.jdk.effective())
                }
                (ProjectField::Maven, Some(environment)) => {
                    describe_effective(ToolKind::Maven, &environment.maven.effective())
                }
                (ProjectField::MavenJdk, Some(environment)) => {
                    describe_effective(ToolKind::Jdk, &environment.maven_jdk_effective())
                }
                (_, None) => detecting(),
            };
            let detected = match field {
                // Maven 的候选表是"检测到的 Maven 安装"，另两行是"检测到的 JDK 安装"
                // （真源 `:172-175` 与 `:189-192` 用的是同一份 `discovered.java`）。
                ProjectField::Maven => environment.and_then(|environment| {
                    detected_installations(
                        environment.maven.candidates.iter().map(maven_entry).collect(),
                    )
                }),
                ProjectField::Jdk | ProjectField::MavenJdk => jdk_candidates(),
            };
            toolchain_rows.push(self.project_toolchain_row(field, effective, detected, cx));
        }

        // ④ 底部动作：重新探测 + 保存 + 加载中 / 已保存（真源 `:316-336`）。
        toolchain_rows.push(self.project_actions(cx));

        // ⑤ Maven 自己的两个配置文件：可选择的覆盖值（选的那份优先，下面写清谁在生效）。
        let maven_rows: Vec<gpui_kit::AnyElement> = MavenConfigField::ALL
            .into_iter()
            .map(|field| self.project_maven_config_row(field, cx))
            .collect();
        // 五个覆盖值走的是输入框草稿（真源也是同一条路），所以这一页不读 `Settings`：
        // 输入框的初值在构造期就从设置里取好了。

        vec![
            settings_group(
                tr("lithe.settings.project.detected"),
                toolchain_rows,
                cx,
            )
            .into_any_element(),
            settings_group(tr("lithe.maven.settings"), maven_rows, cx).into_any_element(),
        ]
    }

    /// 「项目」页的一行工具链：标签 + 覆盖值输入框 + 「清空」+ 提示 + 生效值 + 检测到的安装。
    ///
    /// 不走 `settings_row` 的左右两栏：这一行的输入框要吃掉整行剩余宽度，
    /// 下面还要跟三行说明（与 [`Self::git_identity_row`] 同一条理由）。
    fn project_toolchain_row(
        &self,
        field: ProjectField,
        effective: EffectiveLine,
        detected: Option<SharedString>,
        cx: &Context<Self>,
    ) -> gpui_kit::AnyElement {
        v_flex()
            .id(("settings-project-row", field.index()))
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
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(self.project_field_input(field))),
                    )
                    .child(
                        Button::new(("settings-project-pick", field.index()))
                            .small()
                            .ghost()
                            .label(tr("lithe.settings.gpui.pickPath"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.project_pick_toolchain(field, window, cx)
                            })),
                    )
                    .child(
                        Button::new(("settings-project-clear", field.index()))
                            .small()
                            .ghost()
                            .label(tr("lithe.ui.clear"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.project_clear(field, window, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr(field.hint_key())),
            )
            // 「尚未生效」标注：Maven 的两个覆盖值今天**没有任何消费方**（判据见
            // [`ProjectField::pending_key`]）。用 warning 色而不是 danger：它不是错误，
            // 而是"这一格现在不改变任何行为"的事实；但也不能弱化到看不见。
            .children(field.pending_key().map(|key| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().warning)
                    .child(tr(key))
            }))
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(if effective.is_error {
                        cx.theme().danger
                    } else {
                        cx.theme().foreground
                    })
                    .child(effective.text),
            )
            .children(effective.detail.map(|detail| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(detail)
            }))
            .children(detected.map(|detected| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(detected)
            }))
            .into_any_element()
    }

    /// 一行**只读事实**（标签 + 值 + 备注）。
    ///
    /// 调用点：**「LSP」页**的「已检测语言服务器」（那一格只是把项目页同一次探测的 JDK 事实
    /// 复述出来，没有可编辑的覆盖值）。「项目」页的 `settings.xml` / 本地仓库从本批起
    /// 不再是只读事实，走 [`Self::project_maven_config_row`]。
    fn project_fact_row(
        &self,
        id: &'static str,
        label: SharedString,
        value: SharedString,
        note: Option<SharedString>,
        is_error: bool,
        cx: &Context<Self>,
    ) -> gpui_kit::AnyElement {
        v_flex()
            .id(id)
            .w_full()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(label),
            )
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(if is_error {
                        cx.theme().danger
                    } else {
                        cx.theme().foreground
                    })
                    .child(value),
            )
            .children(note.map(|note| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(note)
            }))
            .into_any_element()
    }

    /// 「项目」页 Maven 自己那份配置的一行（`settings.xml` / 本地仓库）：
    /// 标签 + 草稿输入框 + 「选择…」+「清空」+ 提示 + **谁在生效**那一行。
    ///
    /// 判定（哪一份生效、要不要标失败色）全在纯函数 [`maven_config_effective`] 里，
    /// 这里只画 —— 与 [`Self::project_toolchain_row`] 同一条分工。
    fn project_maven_config_row(
        &self,
        field: MavenConfigField,
        cx: &Context<Self>,
    ) -> gpui_kit::AnyElement {
        let effective = maven_config_effective(field, self.project.environment.as_ref());
        v_flex()
            .id(("settings-maven-config-row", field.index()))
            .w_full()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(field.label()),
            )
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(Input::new(self.maven_config_input(field))),
                    )
                    .child(
                        Button::new(("settings-maven-pick", field.index()))
                            .small()
                            .ghost()
                            .label(tr("lithe.settings.gpui.pickPath"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.project_pick_maven_config(field, window, cx)
                            })),
                    )
                    .child(
                        Button::new(("settings-maven-clear", field.index()))
                            .small()
                            .ghost()
                            .label(tr("lithe.ui.clear"))
                            .disabled(self.project.busy)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.project_clear_maven_config(field, window, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(tr(field.hint_key())),
            )
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(if effective.is_error {
                        cx.theme().danger
                    } else {
                        cx.theme().foreground
                    })
                    .child(effective.text),
            )
            .children(effective.detail.map(|detail| {
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(detail)
            }))
            .into_any_element()
    }

    /// 「项目」页底部的动作行：重新探测 + 保存 + 加载中 / 已保存（真源 `:322-336`）。
    fn project_actions(&self, cx: &Context<Self>) -> gpui_kit::AnyElement {
        h_flex()
            .w_full()
            .items_center()
            .gap_3()
            .pt_2()
            .child(
                Button::new("settings-project-refresh")
                    .small()
                    .ghost()
                    .label(tr("lithe.settings.project.refresh"))
                    .disabled(self.project.busy)
                    .on_click(cx.listener(|this, _, _window, cx| this.project_load(cx))),
            )
            .child(
                Button::new("settings-project-save")
                    .small()
                    .label(tr("lithe.ui.save"))
                    .disabled(self.project.busy)
                    .on_click(cx.listener(|this, _, _window, cx| this.project_save(cx))),
            )
            .when(self.project.busy, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.toolchain.detecting")),
                )
            })
            .when(self.project.saved && !self.project.busy, |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr("lithe.settings.project.saved")),
                )
            })
            .into_any_element()
    }

    /// 从 [`open_settings_dialog_at`] 直接落到懒加载页时的那一次取数（判据见 [`page_loads_on_open`]）。
    fn load_page_on_open(&mut self, category: Category, cx: &mut Context<Self>) {
        match category {
            Category::Project => self.project_load(cx),
            Category::Run => self.run_load(cx),
            // 「LSP」页共用「项目」页那一份探测结果（判据见 `detected_language_servers`）：
            // 已经有值就不重复探测（用户切页不该反复起 `java -version` 子进程）。
            Category::Lsp => {
                if self.project.environment.is_none() && !self.project.busy {
                    self.project_load(cx);
                }
            }
            // 其余分类没有懒加载（`page_loads_on_open` 已经把它们挡在外面）。
            _ => {}
        }
    }

    /// 「LSP」页（阶段 18；真源 `macos-settings-panels.tsx:398-438` 的 `LspPanel`）。
    ///
    /// ## 这一页画什么、为什么只有这些
    ///
    /// | 真源项 | 处置 | 判据 |
    /// | --- | --- | --- |
    /// | `autoCompletion`（`settings.mac.autoCompletion`） | **画成真开关** | 唯一有真消费方的项：落到 `editor/src/completion.rs` 的 `is_completion_trigger`（经外壳转发，模板是 `tabSize`）。关掉之后 `S1_JAVA_COMPLETION` **一行都不会出现** |
    /// | `parameterHints` | **不画，连置灰都不画** | 上游 `gpui-base-0.6.6` 的 `Lsp` 结构体没有签名帮助接口（`input/editor/lsp/mod.rs:39-69` 的 provider 清单里没有，全 crate `SignatureHelp` 零命中），没有任何可挂载的浮层 |
    /// | `semanticTokens`（标签键是 `semanticHighlighting`） | **不画**（本批不做） | 上游 trait 在（`lsp/semantic_tokens.rs:36-55`）但 Java 侧零实现（`java/src/service.rs` 没有 `semantic_tokens`）、零装载点 —— 那是"新做一个特性"，不是"接一个开关" |
    /// | JDTLS / JDK 运行时路径输入框 | **不画** | ① 真源**没有**这个控件；② Windows 已主动退役同类键（`settings-normalization.ts:550` + 守卫测试 `:26-33`）；③ 「项目 · JDK 与 Maven」页的 JDK 覆盖值**真的生效**（`d87f3b0a`），同一个概念不该有两个入口 |
    /// | 「已检测语言服务器」分组 | **做成真事实** | 真源只是一句静态文本（`:431-435`）。本侧读的是「项目」页同一次探测里的 JDK 事实 —— 那正是跑 JDT LS 用的那个 JDK（判据与边界见 [`detected_language_servers`]） |
    ///
    /// ## 关于描述句（有意改写，不照抄）
    ///
    /// 真源的描述是「显示活动语言服务器提供的补全建议。」——它只提"语言服务器"，
    /// 而本侧的 `autoCompletion` 关掉时**连 Core 的轻量兜底也一起不再自动弹出**
    /// （兜底是同一个 provider 的第二个数据源）。照抄会让用户以为兜底还在，
    /// 所以描述句用 gpui 侧新增的键（理由写在 `extract-locale.mjs` 的 `GPUI_ONLY_KEYS` 里），
    /// 如实写明"关掉之后不再自动弹出"。
    fn lsp_page(&self, settings: &Settings, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let environment = self.project.environment.as_ref();
        let detecting = || SharedString::from(tr("lithe.toolchain.detecting"));

        // ① 「语言服务」分组：真源三个开关里唯一有真消费方的那一个。
        let mut service_rows: Vec<gpui_kit::AnyElement> = vec![settings_row(
            "settings-row-lsp-auto-completion",
            tr("lithe.settings.mac.autoCompletion"),
            Some(tr("lithe.settings.gpui.autoCompletionDescription")),
            {
                let store = self.store.clone();
                Switch::new("settings-lsp-auto-completion")
                    .small()
                    .checked(settings.auto_completion)
                    .on_change(move |checked, _, cx| {
                        let checked = *checked;
                        // 只写设置：把它推给编辑区是**外壳**的事（订阅 `SettingsStore` 后调
                        // `EditorPane::set_auto_completion`）。设置 crate 不认识编辑区、
                        // 更不认识 `lithe-gpui-java` —— 依赖方向见 `crate::lib.rs` 的模块文档。
                        store.update(cx, |store, cx| store.set_auto_completion(checked, cx));
                    })
                    .into_any_element()
            },
            None,
            cx,
        )];
        // ② 作用域说明：为什么这一页只有一项（真源另两项在本侧没有可挂载的接口）。
        service_rows.push(
            div()
                .w_full()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(tr("lithe.settings.gpui.lspOnlyAutoCompletion"))
                .into_any_element(),
        );

        // ③ 「已检测语言服务器」：真事实（或"还没有可用的运行时"那句）。
        let minimum = MINIMUM_JAVA_MAJOR.to_string();
        let (server_value, server_is_error) = match environment {
            None => (detecting(), false),
            Some(environment) => match detected_language_servers(environment) {
                Some(servers) => {
                    // 生效 JDK 过不了 ≥ 21 闸门时，这句话要说清"检测到了、但语言服务起不来"
                    // —— 与「项目」页那行同一句、同一个判据（`project::MINIMUM_JAVA_MAJOR`）。
                    let usable = environment.jdk.detected().is_some();
                    let text = if usable {
                        servers
                    } else {
                        SharedString::from(format!(
                            "{} · {}",
                            servers,
                            tr_args(
                                "lithe.settings.gpui.jdkBelowLanguageServiceMinimum",
                                &[("minimum", minimum.as_str())],
                            )
                        ))
                    };
                    (text, !usable)
                }
                None => (
                    SharedString::from(tr("lithe.settings.gpui.noLanguageServer")),
                    true,
                ),
            },
        };

        vec![
            settings_group(
                tr("lithe.settings.mac.languageServices"),
                service_rows,
                cx,
            )
            .into_any_element(),
            settings_group(
                tr("lithe.settings.mac.detectedServers"),
                vec![self.project_fact_row(
                    "settings-lsp-detected-servers",
                    SharedString::from(tr("lithe.settings.mac.detectedServers")),
                    server_value,
                    Some(tr("lithe.settings.mac.detectedServersDescription")),
                    server_is_error,
                    cx,
                )],
                cx,
            )
            .into_any_element(),
        ]
    }

    /// 一行 `S1_SETTINGS_RUN` 诊断（可 grep；走 stdout，与 `S1_SETTINGS_PROJECT` 同口径）。
    ///
    /// **数据**那一行由 [`crate::run::load`] 打
    /// （`result=ok root=… configs=N sources=generated:N providers=… ms=…`）；
    /// 这里补的是"页面做了什么"（第几个代次、结果是几条）。两者合起来就能回答
    /// "截图里那几行来自哪一次取数"。
    fn run_diagnose(&self, detail: &str) {
        println!("{} {detail}", crate::run::RUN_DIAGNOSTIC_TAG);
    }

    /// 识别一次（真源 `run.store` 的 `generate`，`run-configuration-settings.tsx:97-99` 的按钮）。
    ///
    /// 整段取数进 `background_spawn`：`workspace.snapshot` 要遍历整个工作区，
    /// `runConfig.generate` 还要再 `scan(root)` 一遍（深度 ≤ 6、目录数 ≤ 4000）并解析清单文件，
    /// 在 UI 线程上跑会让对话框卡住 —— 与「项目」页的 `discover`、`maven`/`spring` 的懒扫同一口径。
    fn run_load(&mut self, cx: &mut Context<Self>) {
        self.run.generation = self.run.generation.wrapping_add(1);
        let generation = self.run.generation;
        let root = crate::identity::host_workspace_root();
        self.run.root = root.clone();
        self.run.requested = true;
        self.run.busy = true;
        self.run.error = None;
        cx.notify();

        let Some(root) = root else {
            // 没有工作区根（宿主没登记钩子）**不等于**"没有可运行配置"：前者是"看不到项目"，
            // 后者是"项目里确实没有可运行项"。两句话不能混，所以这一支只画 `runNoProject`。
            self.run.busy = false;
            self.run_diagnose(&format!(
                "run=load generation={generation} result=skipped reason=no_workspace_root"
            ));
            cx.notify();
            return;
        };

        self.run_diagnose(&format!(
            "run=load generation={generation} root={}",
            root.display()
        ));

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { crate::run::load(&root) })
                .await;
            let _ = this.update(cx, |this, cx| {
                // 晚到的旧回包丢掉（每次刷新推一代，与 Git / 项目页同一口径）。
                if this.run.generation != generation {
                    return;
                }
                this.run.busy = false;
                match result {
                    Ok(view) => {
                        // 页面上的行数就是这里的 `rows`：截图与日志可以逐项对照。
                        this.run_diagnose(&format!(
                            "run=loaded generation={generation} rows={} entryCount={}",
                            view.configurations.len(),
                            view.entry_count
                        ));
                        this.run.view = Some(view);
                    }
                    Err(error) => {
                        this.run_diagnose(&format!("run=failed generation={generation}"));
                        this.run.error = Some(tr_args(
                            "lithe.settings.gpui.runLoadFailed",
                            &[("reason", &error)],
                        ));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// 「运行配置」页（阶段 17；真源 `components/run-configuration-settings.tsx:79-113` 的列表态）。
    ///
    /// 画出来的每一格都能在这两个地方找到出处：
    /// - **「重新识别」/ 已识别的入口数**：真源列表态同一段
    ///   （`run.identifyAgain` `:97-99`、`run.generatedEntries` `:92-96`）；
    /// - **每条配置的事实**：真源 `run-configuration-editor.tsx:248-258` 的
    ///   `grid-cols-[7.5rem_1fr]` 事实块（`run.type` / `run.effectiveSource` / `run.mainClass`），
    ///   本页按任务要求补上 `run.command` 与 `run.workingDirectory`。
    ///
    /// 与真源的**有意差异**（逐条登记在汇报里）：
    /// - **第一句说明是自己的键**（`settings.gpui.runReadOnlyDescription`）：真源的
    ///   `settings.run.description` 承诺"配置启动参数 / 环境变量 / 覆盖项 + 点击保存生效"，
    ///   而本页没有配置编辑器、没有保存、也没有运行子系统；照抄就是"说了做不到"。
    /// - 真源的行是**可点按钮**（点进去是 `RunConfigurationEditor`），本侧没有配置编辑器、
    ///   也没有项目级存储，所以行做成**只读事实块**：画一个点不动或点了没反应的按钮更骗人
    ///   （`07-settings-ui.md` §7.3-D 的口径）；
    /// - 真源每条行尾有「编辑」，本侧不画（没有那个落点）；
    /// - **不画任何运行按钮**（连置灰也不画）：gpui 侧没有 run crate、Run 工具窗是占位、
    ///   Run 菜单是空的、命令面板没有 Run 动作 —— 所以页面最后一段如实写出这件事
    ///   （`lithe.settings.gpui.runNotWired`），而不是给一个假控件。
    fn run_page(&self, cx: &Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let mut rows: Vec<gpui_kit::AnyElement> = Vec::new();

        // ① 说明。**故意不用**真源的 `settings.run.description`：那句是「选择服务或任务，
        //    配置启动参数…点击保存后生效」，承诺的是配置编辑器与保存 —— 本页三样都没有
        //    （只读列表 + 刷新），照抄就是"说了做不到"（理由写在 `GPUI_ONLY_KEYS` 里）。
        rows.push(
            div()
                .w_full()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(tr("lithe.settings.gpui.runReadOnlyDescription"))
                .into_any_element(),
        );

        // ② 失败态：**显示可排查的原因**（里面带 Core 的错误码原文）。
        if let Some(error) = self.run.error.as_ref() {
            rows.push(
                div()
                    .w_full()
                    .text_sm()
                    .text_color(cx.theme().danger)
                    .child(error.clone())
                    .into_any_element(),
            );
        }

        // ③ 已识别的入口数（真源 `run.generatedEntries`）。
        //
        // 数字用 Core 的 `entryCount`（与真源同一口径：它**不含**恒有的 "Current File" 兜底项），
        // 而列表长度可能比它多 1 —— 那一行的名字就是 "Current File"，两者不会看起来矛盾。
        // 只有拿到结果那一支才画这一行：失败 / 还没取到数时显示一个计数等于替 Core 下结论。
        if self.run.mode() == RunPageMode::Ready {
            if let Some(view) = self.run.view.as_ref() {
                rows.push(
                    div()
                        .w_full()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr_args(
                            "lithe.run.generatedEntries",
                            &[("count", &view.entry_count.to_string())],
                        ))
                        .into_any_element(),
                );
            }
        }

        // ④ 动作行：「重新识别」（生成中显示「处理中…」，照真源 `:97-99` 的三元表达式）。
        rows.push(
            h_flex()
                .w_full()
                .items_center()
                .gap_3()
                .pt_2()
                .child(
                    Button::new("settings-run-refresh")
                        .small()
                        .ghost()
                        .label(if self.run.busy {
                            tr("lithe.settings.project.loading")
                        } else {
                            tr("lithe.run.identifyAgain")
                        })
                        .disabled(self.run.busy)
                        .on_click(cx.listener(|this, _, _window, cx| this.run_load(cx))),
                )
                .into_any_element(),
        );

        // ⑤ 列表 / 空态 / 无项目。**画哪一支由 [`RunPageState::mode`] 决定**（纯判据，有单测）：
        //    失败与"还没有结果"两支在这里什么都不画 —— 失败原因已经在 ② 那一行里。
        match self.run.mode() {
            RunPageMode::NoProject => {
                rows.push(run_page_hint(tr("lithe.settings.gpui.runNoProject"), cx));
            }
            RunPageMode::Failed | RunPageMode::Loading => {}
            RunPageMode::Ready => {
                if let Some(view) = self.run.view.as_ref() {
                    if view.configurations.is_empty() {
                        // "一条都没识别到"是**真的结论**（Core 走完了全部探测器），不是"没接入"。
                        rows.push(run_page_hint(
                            tr("lithe.settings.gpui.runNothingDetected"),
                            cx,
                        ));
                    } else {
                        for (index, configuration) in view.configurations.iter().enumerate() {
                            rows.push(self.run_configuration_row(index, configuration, cx));
                        }
                    }
                }
            }
        }

        // ⑥ 最后一段：如实说明"执行尚未接入"，以及缺的是什么。**任何状态下都画**。
        rows.push(
            div()
                .w_full()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(tr("lithe.settings.gpui.runNotWired"))
                .into_any_element(),
        );

        rows
    }

    /// 「运行配置」页里**一条配置**的只读事实块。
    ///
    /// 标签 / 值的排版照真源事实块：左列固定 `7.5rem`、右列占满（`run-configuration-editor.tsx:248`
    /// 的 `grid-cols-[7.5rem_1fr]`）。字段与取值全部来自 Core 的 `RunConfiguration`，
    /// 一个都不猜：`run.type` = 显示名 + 行为类别 + provider 原名，`run.command`、
    /// `run.workingDirectory`、`run.effectiveSource`、`run.mainClass`（有才画）。
    fn run_configuration_row(
        &self,
        index: usize,
        configuration: &RunConfigurationView,
        cx: &Context<Self>,
    ) -> gpui_kit::AnyElement {
        let fact = |label_key: &'static str, value: SharedString, muted: bool| {
            h_flex()
                .w_full()
                .items_start()
                .gap_2()
                .child(
                    div()
                        .w(rems(7.5))
                        .flex_shrink_0()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(tr(label_key)),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_sm()
                        .text_color(if muted {
                            cx.theme().muted_foreground
                        } else {
                            cx.theme().foreground
                        })
                        .child(value),
                )
        };

        // `run.type` 的取值和真源一样是 `configurationTitle`（`run-configuration-editor.tsx:250`），
        // 后面括注 provider 原名与行为类别：前两者是显示名，后者让"这是服务还是任务"一眼可见。
        let kind = format!(
            "{} · {}（{}）",
            configuration.provider_title,
            tr(configuration.execution.label_key()),
            configuration.provider
        );

        // 没有命令行不是缺字段：Maven 框架服务把可执行文件交给工具链，真正的命令行由启动计划组装。
        let (command, command_muted) = match configuration.command_line() {
            Some(line) => (SharedString::from(line), false),
            None => (tr("lithe.settings.gpui.runNoCommand"), true),
        };

        // `run.effectiveSource` 的取值照真源（`run-configuration-editor.tsx:252` 的
        // `run.source.${source}`），并补上产生它的清单文件 —— 那是"为什么这里会有这一条"的答案。
        let mut source = tr(configuration.layer.label_key()).to_string();
        if let Some(manifest) = configuration.manifest.as_ref() {
            source.push_str(" · ");
            source.push_str(manifest);
        }

        v_flex()
            .id(("settings-run-row", index))
            .w_full()
            .gap_1()
            .p_3()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover.opacity(0.35))
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().foreground)
                    .child(configuration.name.clone()),
            )
            .child(fact("lithe.run.type", SharedString::from(kind), false))
            .child(fact("lithe.run.command", command, command_muted))
            .child(fact(
                "lithe.run.workingDirectory",
                SharedString::from(configuration.cwd.clone()),
                false,
            ))
            .child(fact("lithe.run.effectiveSource", SharedString::from(source), false))
            .children(configuration.main_class.as_ref().map(|main_class| {
                fact(
                    "lithe.run.mainClass",
                    SharedString::from(main_class.clone()),
                    false,
                )
            }))
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
    // `px` 只在这里当**期望值**用（换算断言：`rem_px(px(16.), W) == px(820.)`），
    // 生产代码里已没有直接 `px(...)` 调用点，所以 import 落在测试模块内，
    // 否则顶层 `use` 会报 `unused import`（`check-ui-px.mjs` 只扫生产代码，不看测试）。
    use gpui_kit::px;

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

    /// 「项目 · JDK 与 Maven」页的**行标签与提示全部复用真源既有键**，
    /// 只有 Maven 那条提示是如实改写的新键（真源那句会承诺一个本侧没有的 Maven Wrapper）。
    #[test]
    fn project_field_keys_follow_the_truth_source() {
        assert_eq!(ProjectField::ALL.len(), 3);
        assert_eq!(ProjectField::Jdk.label_key(), "lithe.run.jdkHome");
        assert_eq!(ProjectField::Maven.label_key(), "lithe.run.mavenExecutable");
        assert_eq!(
            ProjectField::MavenJdk.label_key(),
            "lithe.run.mavenJdkHome"
        );
        assert_eq!(ProjectField::Jdk.hint_key(), "lithe.run.jdkHomeHint");
        assert_eq!(
            ProjectField::Maven.hint_key(),
            "lithe.settings.gpui.mavenExecutableHint"
        );
        assert_eq!(
            ProjectField::MavenJdk.hint_key(),
            "lithe.run.mavenJdkHomeHint"
        );
        // 三个字段的 token 与序号都唯一（诊断行与元素 id 都靠它定位）。
        let ids: Vec<&str> = ProjectField::ALL.iter().map(|field| field.id()).collect();
        assert_eq!(ids, ["jdk", "maven", "maven_jdk"]);
        let indices: Vec<usize> = ProjectField::ALL.iter().map(|field| field.index()).collect();
        assert_eq!(indices, [0, 1, 2]);
    }

    /// **A2 的守卫**：Maven 的两个覆盖值必须带「尚未生效」标注，JDK 那一个**不许**带。
    ///
    /// 判据（已核实，见 [`ProjectField::pending_key`] 的文档）：gpui 侧不执行 `mvn`、
    /// 也没采购 `runConfig.createLaunchPlan`，所以 `mavenExecutablePath` / `mavenJavaHomePath`
    /// 今天存了没人读；而 `javaHomePath` 自阶段 16 起真的会改 JDT LS 用的 JVM。
    /// 这条测试同时钉住"两边不许写反"—— 写反了就是**又一处"显示 ≠ 实际"**。
    #[test]
    fn only_the_maven_overrides_are_marked_as_not_effective() {
        assert_eq!(ProjectField::Jdk.pending_key(), None);
        assert_eq!(
            ProjectField::Maven.pending_key(),
            Some("lithe.settings.gpui.mavenOverrideNotEffective")
        );
        assert_eq!(
            ProjectField::MavenJdk.pending_key(),
            Some("lithe.settings.gpui.mavenOverrideNotEffective")
        );
    }

    /// 「项目」页自阶段 16 起是**实现页**：不再有空态前置条件（那句「尚未接入」已下线）。
    #[test]
    fn project_page_is_implemented_not_an_empty_state() {
        assert!(Category::IMPLEMENTED.contains(&Category::Project));
        assert_eq!(Category::Project.prerequisite_key(), None);
        assert_eq!(Category::Project.label_key(), "lithe.settings.project.title");
        assert_eq!(Category::Project.id(), "project");
    }

    /// Maven 配置那两行的键、token、序号与**选文件/选目录**逐条钉住。
    ///
    /// `picks_file` 写反的后果是实机才能发现的：`settings.xml` 会开成"选目录"（永远选不到
    /// 文件），本地仓库会开成"选文件"（永远选不到目录）—— 两行都变成走不通的按钮。
    /// 判据照真源 `project-environment-settings.tsx:305` 的 `choose(key !== "settingsPath", …)`。
    #[test]
    fn maven_config_fields_follow_the_truth_source() {
        assert_eq!(MavenConfigField::ALL.len(), 2);
        assert_eq!(MavenConfigField::SettingsXml.label().as_ref(), "settings.xml");
        assert_eq!(
            MavenConfigField::LocalRepository.label(),
            tr("lithe.maven.localRepository")
        );
        assert_eq!(
            MavenConfigField::SettingsXml.hint_key(),
            "lithe.settings.gpui.mavenSettingsHint"
        );
        assert_eq!(
            MavenConfigField::LocalRepository.hint_key(),
            "lithe.settings.gpui.mavenLocalRepositoryHint"
        );
        let ids: Vec<&str> = MavenConfigField::ALL.iter().map(|field| field.id()).collect();
        assert_eq!(ids, ["settings_xml", "local_repository"]);
        let indices: Vec<usize> = MavenConfigField::ALL
            .iter()
            .map(|field| field.index())
            .collect();
        assert_eq!(indices, [0, 1]);
        // `settings.xml` 是文件、本地仓库是目录。
        assert!(MavenConfigField::SettingsXml.picks_file());
        assert!(!MavenConfigField::LocalRepository.picks_file());
    }

    /// `maven_config_effective` 的四条判据：选了且存在 → 生效说明；选了但不存在 → 失败色 +
    /// 「这个路径不存在」；没选 → 报检测到的那一份（安装级那份只在不同时单列）；
    /// 本地仓库有覆盖值 → 如实标注尚未生效。
    #[test]
    fn maven_config_lines_report_who_is_effective() {
        use crate::project::{LocalRepository, LocalRepositorySource, MavenConfiguration};

        let environment = |config: MavenConfiguration, overrides: Overrides| ProjectEnvironment {
            overrides,
            jdk: crate::project::JdkDiscovery::default(),
            maven: crate::project::MavenDiscovery::default(),
            maven_config: config,
            maven_jdk: crate::project::JdkDiscovery::default(),
        };

        // 没探测到（环境还没有结论）：两行都是"正在检测"，且不是失败色。
        for field in MavenConfigField::ALL {
            let line = maven_config_effective(field, None);
            assert_eq!(line.text, tr("lithe.toolchain.detecting"));
            assert!(!line.is_error);
        }

        // 自动检测到用户级 settings.xml，安装级那份不同 → 单列一行备注（旧口径保留）。
        let auto = environment(
            MavenConfiguration {
                user_settings: Some(r"C:\Users\x\.m2\settings.xml".to_string()),
                installation_settings: Some(r"C:\maven\conf\settings.xml".to_string()),
                local_repository: Some(LocalRepository {
                    path: r"D:\repo".to_string(),
                    source: LocalRepositorySource::SettingsXml,
                }),
                ..MavenConfiguration::default()
            },
            Overrides::default(),
        );
        let line = maven_config_effective(MavenConfigField::SettingsXml, Some(&auto));
        assert_eq!(line.text.as_ref(), r"C:\Users\x\.m2\settings.xml");
        assert!(!line.is_error);
        assert_eq!(
            line.detail.as_deref(),
            Some(r"C:\maven\conf\settings.xml"),
            "安装级那份只在与生效值不同时出现"
        );
        let line = maven_config_effective(MavenConfigField::LocalRepository, Some(&auto));
        assert_eq!(line.text.as_ref(), r"D:\repo");
        assert_eq!(
            line.detail,
            Some(tr(LocalRepositorySource::SettingsXml.label_key()))
        );

        // 用户选了一份存在的 settings.xml → 生效值是它，并说明交给语言服务。
        let picked = environment(
            MavenConfiguration {
                override_settings: Some(r"D:\ci\settings.xml".to_string()),
                override_settings_missing: false,
                local_repository: Some(LocalRepository {
                    path: r"D:\ci-repo".to_string(),
                    source: LocalRepositorySource::SettingsXml,
                }),
                ..MavenConfiguration::default()
            },
            Overrides::default(),
        );
        let line = maven_config_effective(MavenConfigField::SettingsXml, Some(&picked));
        assert_eq!(line.text.as_ref(), r"D:\ci\settings.xml");
        assert!(!line.is_error);
        assert_eq!(
            line.detail,
            Some(tr("lithe.settings.gpui.mavenSettingsEffective"))
        );

        // 选的那份不存在 → 失败色 + 那句"这个路径不存在"（不能看起来像已生效）。
        let missing = environment(
            MavenConfiguration {
                override_settings: Some(r"D:\nope\settings.xml".to_string()),
                override_settings_missing: true,
                ..MavenConfiguration::default()
            },
            Overrides::default(),
        );
        let line = maven_config_effective(MavenConfigField::SettingsXml, Some(&missing));
        assert_eq!(line.text.as_ref(), r"D:\nope\settings.xml");
        assert!(line.is_error);
        assert_eq!(
            line.detail,
            Some(tr("lithe.settings.gpui.overridePathMissing"))
        );

        // 一份都没检测到 → 失败色 + 「未检测到 settings.xml」。
        let none = environment(MavenConfiguration::default(), Overrides::default());
        let line = maven_config_effective(MavenConfigField::SettingsXml, Some(&none));
        assert_eq!(line.text, tr("lithe.settings.gpui.mavenSettingsMissing"));
        assert!(line.is_error);

        // 本地仓库有覆盖值 → 报它，但**如实标注尚未生效**（Maven 执行通路还没有）。
        let overridden = environment(
            MavenConfiguration {
                local_repository: Some(LocalRepository {
                    path: r"D:\repo".to_string(),
                    source: LocalRepositorySource::MavenDefault,
                }),
                ..MavenConfiguration::default()
            },
            Overrides {
                maven_local_repository: " D:\\my-repo ".to_string(),
                ..Overrides::default()
            },
        );
        let line = maven_config_effective(MavenConfigField::LocalRepository, Some(&overridden));
        assert_eq!(line.text.as_ref(), "D:\\my-repo");
        assert!(!line.is_error);
        assert_eq!(
            line.detail,
            Some(tr("lithe.settings.gpui.mavenOverrideNotEffective"))
        );

        // 本地仓库既没覆盖值、也推不出来（拿不到用户主目录）→ 失败色 + 「未知」。
        let unknown = environment(MavenConfiguration::default(), Overrides::default());
        let line = maven_config_effective(MavenConfigField::LocalRepository, Some(&unknown));
        assert_eq!(
            line.text,
            tr("lithe.settings.gpui.mavenLocalRepositoryUnknown")
        );
        assert!(line.is_error);
    }

    /// 「运行配置」页自阶段 17 起是**实现页**：它列出 Core 识别出的启动目标，
    /// 不再有空态前置条件（那句「尚未接入」已下线）。
    #[test]
    fn run_page_is_implemented_not_an_empty_state() {
        assert!(Category::IMPLEMENTED.contains(&Category::Run));
        assert_eq!(Category::Run.prerequisite_key(), None);
        assert_eq!(Category::Run.label_key(), "lithe.settings.run.title");
        assert_eq!(Category::Run.id(), "run");
    }

    /// 带着分类打开对话框时，只有**懒加载页**需要立刻取一次数
    /// （否则它们会停在「正在检测…」）。判据与 `load_page_on_open` 的分派必须一致 ——
    /// 两边都说"这一页要懒加载"，`content()` 的分支才不会漏。
    #[test]
    fn only_the_lazy_pages_load_on_open() {
        assert!(page_loads_on_open(Category::Project));
        assert!(page_loads_on_open(Category::Run));
        // 「LSP」页共用「项目」页那份探测结果（见 `detected_language_servers`），
        // 所以它也要懒加载一次 —— 否则从命令面板直接打开到这一页会永远停在"正在检测…"。
        assert!(page_loads_on_open(Category::Lsp));
        for category in Category::ALL {
            if matches!(
                category,
                Category::Project | Category::Run | Category::Lsp
            ) {
                continue;
            }
            assert!(
                !page_loads_on_open(category),
                "{category:?} 不该在打开时读盘 / 起探测子进程"
            );
        }
    }

    /// 「LSP」页自阶段 18 起是**实现页**：`autoCompletion` 真生效，
    /// 「已检测语言服务器」是真事实 —— 那句「此分类尚未接入」的空态前置条件必须下线。
    #[test]
    fn lsp_page_is_implemented_not_an_empty_state() {
        assert!(Category::IMPLEMENTED.contains(&Category::Lsp));
        assert_eq!(Category::Lsp.prerequisite_key(), None);
        assert_eq!(Category::Lsp.label_key(), "lithe.settings.tabs.lsp");
        assert_eq!(Category::Lsp.id(), "lsp");
    }

    /// 「运行配置」页的四支不能互相冒充：**失败**不许画成"没有识别到可运行配置"
    /// （那是 Core 走完全部探测器后的结论），**还没取到数**也不许替 Core 下结论。
    ///
    /// 这一支判据决定了页面上唯一那句话是哪一句，所以四组状态逐条钉住。
    #[test]
    fn run_page_never_confuses_a_failure_with_an_empty_result() {
        // 还没点过这一页：不画任何结论（第一次进入时它会被 `run_load` 立刻改掉）。
        let mut state = RunPageState::new();
        assert_eq!(state.mode(), RunPageMode::Loading);

        // 宿主没登记工作区根：是"看不到项目"，不是"项目里没有可运行项"。
        state.requested = true;
        assert_eq!(state.mode(), RunPageMode::NoProject);

        // 有根、但失败：只画原因。
        state.root = Some(PathBuf::from(r"D:\proj"));
        state.error = Some(SharedString::from("识别运行配置失败"));
        assert_eq!(state.mode(), RunPageMode::Failed);

        // 失败之后**不清**这一支：清掉就等于把"没识别到"画在失败上面。
        assert_ne!(state.mode(), RunPageMode::Ready);

        // 拿到结果（哪怕是空表）才是 Ready —— 空表由页面画"没有识别到可运行配置"。
        state.error = None;
        state.view = Some(RunProjectView {
            configurations: Vec::new(),
            entry_count: 0,
            java_entrypoints_origin: None,
        });
        assert_eq!(state.mode(), RunPageMode::Ready);

        // 刷新在飞（busy）时仍是 Ready：旧结果继续画，按钮变禁用 + 显示「处理中…」。
        state.busy = true;
        assert_eq!(state.mode(), RunPageMode::Ready);
    }

    /// 生效值那一行的四种形态（逐条对照真源 `describeEffectiveToolchain`）。
    ///
    /// 最关键的一条是**来源只在自动选出时出现**：覆盖值的"来源"由模式（「已选择」）表达，
    /// 再拼一次来源就是重复信息（真源 `effective-toolchain.ts:51-54`）。
    #[test]
    fn effective_lines_follow_the_truth_source_wording() {
        /// 一条"自动选中"的 JDK 生效值（测试里反复改一两个字段用）。
        fn resolved(
            mode: crate::project::ToolMode,
            version: Option<&str>,
            source: Option<ToolSource>,
        ) -> EffectiveToolchain {
            EffectiveToolchain::Resolved {
                mode,
                path: r"D:\ProgramData\java\openjdk-21".to_string(),
                version: version.map(str::to_string),
                source,
            }
        }

        let automatic = resolved(
            crate::project::ToolMode::Automatic,
            Some("21.0.2"),
            Some(ToolSource::JavaHome),
        );
        let line = describe_effective(ToolKind::Jdk, &automatic);
        assert!(!line.is_error, "自动选中的值是正常态");
        assert!(line.text.contains(" → "), "{}", line.text);
        assert!(line.text.contains(r"D:\ProgramData\java\openjdk-21"));
        assert!(line.text.contains("21.0.2"), "{}", line.text);
        assert!(
            line.text.contains(source_label(ToolSource::JavaHome).as_ref()),
            "自动选中必须报来源：{}",
            line.text
        );
        assert!(line.detail.is_none());

        let configured = resolved(crate::project::ToolMode::Configured, Some("21.0.2"), None);
        let line = describe_effective(ToolKind::Jdk, &configured);
        assert!(!line.is_error);
        assert!(
            !line.text.contains(source_label(ToolSource::JavaHome).as_ref()),
            "覆盖值不该再拼来源：{}",
            line.text
        );
        assert!(!line.text.contains(source_label(ToolSource::Path).as_ref()));

        // 读不出版本时退化成裸名字（真源 `"JDK"` / `"Maven"`），**不编一个版本**。
        let line = describe_effective(
            ToolKind::Jdk,
            &resolved(crate::project::ToolMode::Automatic, None, Some(ToolSource::Path)),
        );
        assert!(line.text.contains(ToolKind::Jdk.bare_name()), "{}", line.text);
        let line = describe_effective(
            ToolKind::Maven,
            &EffectiveToolchain::Resolved {
                mode: crate::project::ToolMode::Automatic,
                path: r"D:\tools\apache-maven-3.9.9\bin\mvn.cmd".to_string(),
                version: None,
                source: Some(ToolSource::MavenHome),
            },
        );
        assert!(
            line.text.contains(ToolKind::Maven.bare_name()),
            "{}",
            line.text
        );
        assert!(
            line.text
                .contains(source_label(ToolSource::MavenHome).as_ref()),
            "Maven 的来源要报出 MAVEN_HOME：{}",
            line.text
        );

        // 覆盖值用不了：失败色 + **详情行回显用户填的原文**。
        let unusable = EffectiveToolchain::Unusable {
            path: r"D:\nope".to_string(),
            reason: "找不到 java.exe".to_string(),
            // 路径不存在这一类**不是**版本闸门问题（理由见 `project.rs::below_requirement_reason`）。
            language_service_rejection: false,
        };
        let line = describe_effective(ToolKind::Jdk, &unusable);
        assert!(line.is_error);
        assert!(line.text.contains("找不到 java.exe"), "{}", line.text);
        assert_eq!(line.detail.as_deref(), Some(r"D:\nope"));

        // 低于 JDT LS 的 ≥ 21 闸门：详情行要**同时**给出路径与"语言服务无法启动"，
        // 这样用户既能看清是哪个值不行，也能预判语言服务起不来（A1 的核心口径）。
        let below_gate = EffectiveToolchain::Unusable {
            path: r"D:\ProgramData\java\openjdk-17".to_string(),
            reason: crate::project::below_requirement_reason("17.0.16"),
            language_service_rejection: true,
        };
        let line = describe_effective(ToolKind::Jdk, &below_gate);
        assert!(line.is_error);
        assert!(line.text.contains("17.0.16"), "{}", line.text);
        let detail = line.detail.as_deref().unwrap_or_default();
        assert!(detail.contains("openjdk-17"), "{detail}");
        // ⚠️ 断言**不写死中文**：`tr` 读的是进程级 locale（测试二进制里可能是 en），
        // 所以这里钉的是"那句话里必须有的三样事实"：闸门数值、语言服务、JDT LS。
        assert!(
            detail.contains(&crate::project::MINIMUM_JAVA_MAJOR.to_string()),
            "详情行必须报出闸门数值：{detail}"
        );
        assert!(detail.contains("JDT LS"), "详情行必须点明是哪个语言服务：{detail}");
        assert!(
            detail.contains("language service") || detail.contains("语言服务"),
            "详情行必须说清后果落在语言服务上：{detail}"
        );

        // 没找到：失败色 + 详情行是"试过哪些、为什么不行"。
        let not_found = EffectiveToolchain::NotFound {
            reason: "JAVA_HOME=未设置；已排除：C:\\jdk8（版本 1.8.0_402）".to_string(),
        };
        let line = describe_effective(ToolKind::Jdk, &not_found);
        assert!(line.is_error);
        assert_eq!(line.text, tr(ToolKind::Jdk.not_found_key()));
        let detail = line.detail.expect("没找到时必须给出已排除清单");
        assert!(detail.contains("已排除"), "{detail}");
    }

    /// 「检测到的安装」那两行：有版本写 `路径 (版本)`，没有版本只写路径。
    #[test]
    fn detected_installations_omit_a_missing_version() {
        let jdk = DetectedJdk {
            home: r"D:\ProgramData\java\openjdk-21".to_string(),
            executable: r"D:\ProgramData\java\openjdk-21\bin\java.exe".to_string(),
            version: "21.0.2".to_string(),
            meets_language_service: true,
            source: ToolSource::JavaHome,
        };
        assert_eq!(
            jdk_entry(&jdk),
            r"D:\ProgramData\java\openjdk-21 (21.0.2)"
        );
        // 没有 home 时退回可执行文件路径；没有版本时不加空括号。
        assert_eq!(
            jdk_entry(&DetectedJdk {
                home: String::new(),
                ..jdk.clone()
            }),
            r"D:\ProgramData\java\openjdk-21\bin\java.exe (21.0.2)"
        );
        assert_eq!(
            jdk_entry(&DetectedJdk {
                version: String::new(),
                ..jdk.clone()
            }),
            r"D:\ProgramData\java\openjdk-21"
        );

        let maven = DetectedMaven {
            executable: r"D:\tools\apache-maven-3.9.9\bin\mvn.cmd".to_string(),
            version: "3.9.9".to_string(),
            home: Some(r"D:\tools\apache-maven-3.9.9".to_string()),
            java_runtime: Some("21.0.2".to_string()),
            source: ToolSource::MavenHome,
        };
        assert_eq!(
            maven_entry(&maven),
            r"D:\tools\apache-maven-3.9.9\bin\mvn.cmd (3.9.9)"
        );
        assert_eq!(
            maven_entry(&DetectedMaven {
                version: String::new(),
                ..maven
            }),
            r"D:\tools\apache-maven-3.9.9\bin\mvn.cmd"
        );

        // 一个候选都没有 → 那一行**不画**（不是画一句空话）。
        assert!(detected_installations(Vec::new()).is_none());
    }
}
