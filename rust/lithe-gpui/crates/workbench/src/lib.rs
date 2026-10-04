//! 工作台外壳（Feature：窗口 chrome + 各区域的组装）。
//!
//! ## 职责
//!
//! 从 `lithe-gpui/shell/src/bin/shell_probe/` 下的 `workspace.rs`（534 行）与 `shell/`
//! 四个区域文件整体上移，按《编码指南》「大型应用按业务能力组织 crate」做成一个 Feature crate。
//! 它拥有窗口的可见结构：标题栏 + 项目标签条 + 左右活动栏 + 各面板槽位 + 状态栏，
//! 并把 `explorer` / `editor` / `git` / `terminal` 四个 Feature 的 `Entity` 组装进三栏布局。
//!
//! ## 文件分工
//!
//! - [`workspace`]：`ShellWorkspace` 组装（三栏 + 底部窗 + 右工具窗 + 状态栏）与 `Render`；
//! - [`command_palette`]：命令面板浮层（`Ctrl+Shift+P`；`Dialog` + `Command`，阶段 6 第二半）；
//! - [`menu_bar`]：主菜单栏（9 个顶级菜单 + 两种形态；度量与"只列可执行项"的口径见其模块文档）；
//! - [`title_bar`]：标题栏（**两行：菜单行 30 + 主行 38**，2026-09-27 起）+ 自绘窗口三键 +
//!   菜单栏 / 项目下拉 / 分支项 / 运行控件的落位；
//! - [`project_menu`]：标题栏的项目下拉面板（3 段 + 2 分隔线；徽标算法与 v1 范围见其模块文档）；
//! - [`branch_panel`]：标题栏的分支项 + 分支弹窗（`Dialog` + `Command`；只读 v1 与三段自绘的理由
//!   见其模块文档）；数据来自 `lithe-gpui-git` 的 `BranchSnapshot`（`git.status` + `git.references`）；
//! - [`project_tabs`]：项目标签条；
//! - [`activity_bar`]：左右活动栏（40px，真源 `WorkbenchView.swift:7`（右）/`:12`（左））；
//! - [`right_tool_window`]：右侧工具窗（400px，可收起；Maven / 通知 / 扩展三个视图）；
//! - [`spring`] / [`spring_paths`]：右侧「Spring」视图的数据层与它**必须**给出的输入清单
//!   （`spring.index` 只索引请求里 `paths` 列出的文件；`paths` 缺省为空 =
//!   一个工作区文件都不扫、端点恒为 0。收集口径与排除目录见 [`spring_paths`] 的模块文档）；
//! - [`status_bar`]：状态栏（24px）。
//!
//! ## 设置（阶段 8）
//!
//! 本 crate 只做设置的两个**消费点**，设置本身（模型 / 持久化 / 主题应用 / 对话框）在
//! `lithe-gpui-settings`：
//!
//! 1. 活动栏「设置」项 → `open_settings_dialog`（真机是模态对话框，所以它不改选中态、
//!    也不换底部窗）；
//! 2. 状态栏按 `Settings::show_status_bar` 条件渲染，订阅 `SettingsStore` 后自动跟随。
//!
//! 活动栏的选中态是**三组独立的**（左栏两组 + 右栏一组）：左栏顶部组看 `ShellWorkspace` 的
//! `top_activity_view`（默认第 0 项「项目」），左栏底部组看 `bottom_visible` + `bottom_kind`，
//! 右栏看 `right_visible` + `right_view`；左栏两组可以同时高亮（真机
//! `features/window/stores/workspace-ui-defaults.ts:5-7` 与
//! `features/layout/components/sidebar/main-sidebar.tsx:643-652`），右栏是第三套状态
//! （`plugin-activity-rail.tsx:24-26`）。两个工具窗都**默认隐藏**：底部窗
//! `workspace-ui-defaults.ts:5` 的 `isBottomPaneVisible: false`、右工具窗
//! `stores/ui-state/panel-slice.ts:28` 的 `isRightSidebarVisible: false`；终端会话在第一次
//! 可见时才由 `TerminalPane::ensure_session` 懒创建。
//!
//! 四个区域文件都是**无状态渲染函数**（`-> impl IntoElement`），状态由 [`workspace::ShellWorkspace`]
//! 持有并通过参数传入；需要独立生命周期的内容（项目树 / 编辑区 / Git / 终端）各自是 Feature
//! crate 里的 `Entity`。右工具窗的三个视图本轮都只有头部 + 空态，所以是纯渲染函数，
//! 没有单独的 `Entity`。
//!
//! ## 区域度量：用 GPUI 的 rem-based helper，不写裸 `px(...)`
//!
//! 《编码指南》「主题与样式」要求应用布局用 GPUI 的 rem-based helper（`p_2()` / `gap_3()` /
//! `text_sm()`）而不是直接写 `px(...)`；「基础字号控制应用缩放」解释了收益（type、whitespace、
//! control、icon 一起随 base font 缩放）。**本 crate 的区域度量按这条执行**，换算规则与例外如下。
//!
//! **换算规则**：helper 后缀 `N` = `N × 0.25rem` = `N × 4px`（rem base = 主题字号 16px，
//! `lithe-gpui/themes/*.json` 的 `font.size` + `gpui-component-0.6.6/src/root.rs:582` 的
//! `window.set_rem_size(cx.theme().font_size)`）。**尺寸真源是 macOS**
//! （`macos/Sources/Lithe/Views/`，`windows/tauri/**` 那份前端已删除、其数值一律作废）：
//! 标题栏 **40**（单行，`LitheTheme.swift:478`）、项目标签条 **38**（条高）/ **30**（标签高，
//! 见 `crate::project_tabs`）、活动栏 **40**（真源 `WorkbenchView.swift:7`（右）/`:12`（左）；
//! B14 之前是 38）、状态栏 **24**（`h_6()`）、左栏 **320**
//! （`WorkbenchLayoutStore.swift:21`；可拖区间 30…`min(520, 容器 − 400)`）、
//! 右工具窗 **360**（`WorkbenchLayoutStore.swift:5`）、工作区间隔 **4**（`gap_1()`）、
//! 底部窗 **260**（`WorkbenchView.swift:2019` 的最小高；2026-10-03 按真源把常量从 240
//! 对齐到它 —— 视觉无变化，因为 clamp 早已强制 260）。
//! 每个值的出处写在使用点或 `crates/*/src/*.rs` 顶部的度量映射表里。
//!
//! **档位外的长度一律写 helper 底层的 `rems(P / 16.)`**（**不是** `/ 4.`）：gpui 的档位表是
//! 编译期生成的固定列表（`gpui-pre-macros-0.3.6/src/styles.rs:926-1158`），例如
//! 26 / 180 / 240 / 260 / 360 / 520 …Tailwind v4 的任意整数档在 gpui 里没有对应
//! helper，不能自己发明一个 `w_100()`；但 `rems(P / 16.)` 在默认 16px 基准下与 `px(P)`
//! 逐像素相等，同时随主题字号缩放。**不在档位上从来不是保留 `px(...)` 的理由** ——
//! 它正是该写 `rems(P / 16.)` 的理由（`coding-guides.md:288` 的四类例外里没有这一条）。
//!
//! **必须先按运行时基准换算成 `Pixels` 的位置**走 [`rem_px`]（不要写 `px(N.)`）：
//!
//! 1. **固有方法只吃 `Pixels`**：`Dialog::width` / `Dialog::margin_top`、`PopupMenu::min_w` /
//!    `max_w` / `max_h`、`Button::rounded`、`Sizable::with_size`、`Positioner::offset` /
//!    `margin`。这些固有方法会**遮蔽** `Styled` 上的同名方法（`Styled` 那一版收
//!    `impl Into<AbsoluteLength>`，本来可以直接写 `rems`），而 gpui **没有**
//!    `impl From<Rems> for Pixels`，所以只能先按运行时 rem 基准求值一次；
//! 2. **圆角**：Lithe 的圆角阶梯是 `--radius: 8px` 派生的 `calc(--radius × k)`
//!    （`theme.css:6-12`：`sm` = 4.8、`md` = 6.4、`lg` = 8、`xl` = 11.2），**不在** 4px 网格上。
//!    也不能改成读主题 —— `ThemeConfig.radius` 是 `usize`
//!    （`gpui-component-0.6.6/src/theme/schema.rs:67-68`），装不下 4.8 / 6.4；而把主题半径设成 8
//!    会让**所有** gpui-kit 组件的圆角从 6 变成 8，反而离 Lithe 的 `rounded-md`(6.4) 更远。
//!    所以圆角一律走应用层具名规格常量（`TAB_RADIUS_SPEC` / `CHIP_RADIUS_SPEC` /
//!    `CLOSE_BUTTON_RADIUS_SPEC` / …），调用点写 `rems(C / 16.)`（`Styled::rounded*` 收
//!    `impl Into<AbsoluteLength>`，档位外的圆角也一样不需要 `px`）；
//! 3. **运行时算术**（例如活动栏的 `40 + 4 = 44`、树行的 `10 + depth × 16`）：没有固定的档位
//!    helper 可套；同样写成 helper 底层的 `rems(N / 16.)`（`rems(8. / 16.)` = 8px），
//!    值随主题基准字号缩放。
//!
//! **字号**：Lithe 的 UI 基准是 13px（`--app-ui-font-size` / `--ui-text-chrome` = 13px，
//! `theme.css:112,115`），gpui 的字号位位只有 `text_xs()`=12 与 `text_sm()`=14，13 不在位位上。
//! 按《编码指南》统一用 **`text_sm()`（14px）**——13 → 14 是经维护者确认的**有意**视觉改动
//! （所有 chrome 文字 +1px），不是等价换算；位位内等价换算的是 12 → `text_xs()`、
//! 16 → `text_base()`；10 / 11 / 13（徽章、日期列、等宽字号）不在位位上，写
//! `rems(10. / 16.)` 这类换算（见 `crate::branch_panel` 的 ahead/behind 箭头）。
//!
//! 颜色仍然一律走 `cx.theme()`（不写裸色值），这条没有偏离。
//!
//! ## 公开边界
//!
//! 只有 [`ShellWorkspace`]（工作台根视图）是公开 API；区域模块对外只暴露它们的渲染函数
//! （右工具窗另外暴露 [`right_tool_window::RightToolWindowView`]：它是"当前视图"的类型，
//! 与真机 `activeRightSidebarView` 同物）。
//!
//! ## ⚠️ 区域渲染函数必须收 `&Window` / `&App`，不要收 `&mut`
//!
//! 本 crate 是 **edition 2024**，而 edition 2024 的 RPIT 规则会**捕获签名里所有在作用域的
//! 生命周期**（2021 及以前只捕获出现在 `impl Trait` 里的那些）。于是：
//!
//! - 参数写成 `&mut Window` / `&mut App` 时，返回的 `impl IntoElement` 会捕获这个**可变借用**；
//! - 同一个表达式里连续调用两次（例如"左右两条活动栏"共用 `cx`）就报
//!   `E0499 cannot borrow *cx as mutable more than once at a time`。
//!
//! 本项目实测：第一次构建时这一条一次报了 **12 个** E0499/E0502。修法不是拆表达式，而是
//! 让这些函数收 `&Window` / `&App` —— 它们本来就只读主题色与窗口状态，不需要可变借用，
//! 而不可变借用可以同时存在。**新增区域模块请遵守这条。**

pub mod activity_bar;
pub mod branch_panel;
pub mod command_palette;
// 调试工具窗（U19）：宿主断点模型 + 调用栈 / 变量树 / 监视 / 求值 / 单步控制五块 + 诚实空态。
// **为什么在 workbench 而不在新 crate**：本仓不许新建 crate（本任务硬约束），而调试面板
// 要放进底部工具窗、还要读编辑区的行号槽事件 —— 那两样都归 `ShellWorkspace`。
// 真接上的 Core 命令只有 `debug.steppingFilters` 与 `debug.relocateBreakpoints`（都无会话依赖）；
// 缺什么、为什么缺，逐条写在模块文档上。
pub mod debug;
// 中央扩展页（`ShellWorkspace` 的中央页槽位，Zed 式扩展管理入口）。
// 数据层在 `lithe-gpui-extensions`（model + Core 桥），本模块只做视图。
pub mod extensions_page;
pub mod maven;
pub mod menu_bar;
pub mod notifications;
pub mod project_menu;
pub mod project_tabs;
// Quick Open 浮层（`Ctrl+P`；`workspace.searchEverywhere` 的 Files scope）。浮层形状照
// `command_palette`（`Dialog` + `Command`，本地过滤关掉），请求 / 解析 / 代次复用
// `search_model`；规格出处与有意差异见其模块文档。
pub mod quick_open;
// Replace in Files 浮层（菜单「转到 → 在文件中替换」/ `Ctrl+Shift+R`）：
// `workspace.replacePreview` 的**替换预览**（只读，不写盘）。浮层形状照 `search_everywhere`，
// 请求 / 解析 / 四态 / 代次复用 `search_model`；规格出处、以及"应用替换在本分支做不了
// （Core 没有替换命令）"这条边界见其模块文档。
pub mod replace_in_files;
pub mod right_tool_window;
// Search Everywhere 浮层（菜单「视图 → 全局搜索」；`workspace.searchEverywhere` 的
// All / Classes / Files / Symbols / Actions / Text 六个 scope）。浮层形状照 `command_palette` / `quick_open`
// （`Dialog` + `Command`，本地过滤关掉），请求 / 解析 / 代次 / 六个 scope 的投影全部复用
// `search_model`；规格出处、`maxResults` 的推导与有意差异见其模块文档。
pub mod search_everywhere;
// 搜索索引在宿主侧的接线（warm / update / invalidate 三条 Core 命令的时机与代次）。
// **不是**渲染模块：它只负责"索引什么时候建、什么时候更新"，界面在下面两个模块里。
pub mod search_index;
// 查找面板（U17 第一件）的两层：纯逻辑（payload / 解析 / 分组 / 代次，可确定性单测）
// 与视图（左活动栏「搜索」项那块侧栏）。
pub mod search_model;
pub mod search_panel;
pub mod session;
pub mod spring;
pub mod spring_panel;
pub mod spring_paths;
pub mod status_bar;
pub mod title_bar;
// 欢迎页 / 无项目状态（U10）：窗口根的另一支（`Root` + `WelcomeView`），真源是
// `macos/Sources/Lithe/Views/App/WelcomeView.swift`。触发条件（`choose_launch_root` 的
// `None` 支）在 `crates/app/src/main.rs` 里判。
pub mod welcome;
pub mod workspace;

pub use welcome::{ProjectOpener, WelcomeRow, WelcomeView};
pub use workspace::{
    OpenDestination, ShellWorkspace, install_open_project_action, left_activity_index,
    set_shell_startup,
};

/// 登记本 crate（及其子视图 crate）的快捷键。
///
/// **为什么由 workbench 转出**：`crates/app` 只依赖 `lithe-gpui-workbench` 与
/// `lithe-gpui-settings`（`app/Cargo.toml:36`），不直接依赖 `explorer` / `editor` 这些
/// 子视图 crate。所以子 crate 的 `install_actions` 要经由本函数转到 app，
/// 而不是给 app 新增依赖边。
pub fn install_actions(cx: &mut gpui_kit::App) {
    // 项目树：`⌘A` / `Ctrl+A` = 真源 `ProjectTreeSelection.selectAll`
    // （选**焦点行的同级兄弟**，`ProjectTreeSelection.swift:59-70`）。
    lithe_gpui_explorer::install_actions(cx);
}

/// 规格像素 → `Pixels` 的换算，**唯一实现在 [`lithe_gpui_shared::rem_px`]**。
///
/// 原先这段换算在 `editor` / `git` / `settings` / `terminal` / `workbench` 五个 crate 里
/// 各有一份逐字相同的实现（外加三份重复单测），改口径就会分叉。现已收敛到 `shared`，
/// 本 crate 只做 `pub(crate)` 转发，让既有的 `crate::rem_px(..)` 调用点与 `[`crate::rem_px`]`
/// 文档链接继续成立。
///
/// 换算口径、为什么必须按**运行时** rem 基准、以及"什么槽才需要绕这一层"的判据，
/// 全部写在被转发的那个函数上（本 crate 不再重复一遍，避免两处说明漂移）。
pub(crate) use lithe_gpui_shared::rem_px;
