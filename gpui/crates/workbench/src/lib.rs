//! 工作台外壳（Feature：窗口 chrome + 各区域的组装）。
//!
//! ## 职责
//!
//! 从 `gpui/shell/src/bin/shell_probe/` 下的 `workspace.rs`（534 行）与 `shell/`
//! 四个区域文件整体上移，按《编码指南》「大型应用按业务能力组织 crate」做成一个 Feature crate。
//! 它拥有窗口的可见结构：标题栏 + 项目标签条 + 左右活动栏 + 各面板槽位 + 状态栏，
//! 并把 `explorer` / `editor` / `git` / `terminal` 四个 Feature 的 `Entity` 组装进三栏布局。
//!
//! ## 文件分工
//!
//! - [`workspace`]：`ShellWorkspace` 组装（三栏 + 底部窗 + 右工具窗 + 状态栏）与 `Render`；
//! - [`command_palette`]：命令面板浮层（`Ctrl+Shift+P`；`Dialog` + `Command`，阶段 6 第二半）；
//! - [`menu_bar`]：主菜单栏（9 个顶级菜单 + 两种形态；度量与"只列可执行项"的口径见其模块文档）；
//! - [`title_bar`]：标题栏（40px）+ 自绘窗口三键 + 菜单栏 / 项目下拉 / 分支项的落位；
//! - [`project_menu`]：标题栏的项目下拉面板（3 段 + 2 分隔线；徽标算法与 v1 范围见其模块文档）；
//! - [`branch_panel`]：标题栏的分支项 + 分支弹窗（`Dialog` + `Command`；只读 v1 与三段自绘的理由
//!   见其模块文档）；数据来自 `lithe-gpui-git` 的 `BranchSnapshot`（`git.status` + `git.references`）；
//! - [`project_tabs`]：项目标签条；
//! - [`activity_bar`]：左右活动栏（38px）；
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
//! `gpui/themes/*.json` 的 `font.size` + `gpui-component-0.6.6/src/root.rs:582` 的
//! `window.set_rem_size(cx.theme().font_size)`）。规格值来自 Windows 前端源码
//! （`windows/tauri/src/styles/theme.css` 的 `--lithe-*` 令牌与各组件里的 Tailwind 值），
//! 逐值搬过来才能与真机并排核对：标题栏 **40**（`h_10()`）、项目标签条 **32**（`h_8()`）、
//! 活动栏 **38**（不在档位上，保留 `px(...)`）、状态栏 **24**（`h_6()`）、左栏 **320**（`w_80()`）、
//! 右工具窗 **400**（`rems(400. / 16.)`，档位外但可换算成 rem）、工作区间隔 **4**（`gap_1()`）、
//! 底部窗 **320**（`h_80()`）。
//! 每个值的出处写在使用点或 `crates/*/src/*.rs` 顶部的度量映射表里。
//!
//! **三类仍写 `px(...)` 的值**（每处都有注释说明，不是漏改）：
//!
//! 1. **不在 gpui 固定档位上的长度**：gpui 的档位表是编译期生成的固定列表
//!    （`gpui-pre-macros-0.3.6/src/styles.rs:926-1158`），例如 38 / 42 / 56 / 144 / 200 /
//!    240 / 400 …Tailwind v4 的任意整数档在 gpui 里没有对应 helper，不能自己发明一个；
//!    这类值按《编码指南》写成 helper 底层的 `rems(P / 16.)`（400 = `rems(25.)`），
//!    rem base 16px 时与 `px(P)` 逐像素相等，同时随主题字号缩放；
//! 2. **圆角**：Lithe 的圆角阶梯是 `--radius: 8px` 派生的 `calc(--radius × k)`
//!    （`theme.css:6-12`：`sm` = 4.8、`md` = 6.4、`lg` = 8、`xl` = 11.2），**不在** 4px 网格上。
//!    也不能改成读主题 —— `ThemeConfig.radius` 是 `usize`
//!    （`gpui-component-0.6.6/src/theme/schema.rs:67-68`），装不下 4.8 / 6.4；而把主题半径设成 8
//!    会让**所有** gpui-kit 组件的圆角从 6 变成 8，反而离 Lithe 的 `rounded-md`(6.4) 更远。
//!    所以圆角一律走应用层具名常量（`TAB_RADIUS` / `CHIP_RADIUS` / `ISLAND_RADIUS` / …），值不变；
//! 3. **运行时算术**（例如活动栏的 `38 + 4 = 42`、树行的 `10 + depth × 16`）：没有固定的档位
//!    helper 可套；其中档位内的部分写成 helper 底层的 `rems(N / 16.)`（不是 `/ 4.`：
//!    rem base = 16px，所以 `rems(8. / 16.)` = 8px，值随主题基准字号缩放）。
//!
//! **字号**：Lithe 的 UI 基准是 13px（`--app-ui-font-size` / `--ui-text-chrome` = 13px，
//! `theme.css:112,115`），gpui 的字号位位只有 `text_xs()`=12 与 `text_sm()`=14，13 不在位位上。
//! 按《编码指南》统一用 **`text_sm()`（14px）**——13 → 14 是经维护者确认的**有意**视觉改动
//! （所有 chrome 文字 +1px），不是等价换算；位位内等价换算的是 12 → `text_xs()`、
//! 16 → `text_base()`；10 / 11（徽章、日期列、等宽字号）不在位位上，保留 `px(...)`。
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
pub mod maven;
pub mod menu_bar;
pub mod project_menu;
pub mod spring;
pub mod spring_paths;
pub mod project_tabs;
pub mod right_tool_window;
pub mod status_bar;
pub mod title_bar;
pub mod workspace;

pub use workspace::ShellWorkspace;
