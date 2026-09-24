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
//! - [`workspace`]：`ShellWorkspace` 组装（三栏 + 底部窗 + 状态栏）与 `Render`；
//! - [`title_bar`]：标题栏（40px）+ 自绘窗口三键；
//! - [`project_tabs`]：项目标签条；
//! - [`activity_bar`]：左右活动栏（38px）；
//! - [`status_bar`]：状态栏（24px）。
//!
//! 四个区域文件都是**无状态渲染函数**（`-> impl IntoElement`），状态由 [`workspace::ShellWorkspace`]
//! 持有并通过参数传入；需要独立生命周期的内容（项目树 / 编辑区 / Git / 终端）各自是 Feature
//! crate 里的 `Entity`。
//!
//! ## ⚠️ 有意的偏离：区域度量用 `px(...)` 而不是 rem helper
//!
//! 《编码指南》「主题与样式」要求应用布局用 GPUI 的 rem-based helper（`p_2()` / `gap_3()` /
//! `text_sm()`），而不是直接写 `px(...)`；「基础字号控制应用缩放」还解释了这样做的收益
//! （type、whitespace、control、icon 一起随 base font 缩放）。
//!
//! **本 crate 的区域度量有意不遵守这一条**，理由有三条，都是可核对的：
//!
//! 1. **规格真源给的就是 px**。本项目的界面规格来自 Windows 前端源码
//!    （`windows/tauri/src/styles/theme.css` 的 `--lithe-*` 令牌与各组件里的 Tailwind 值），
//!    逐值搬过来才能与真机并排核对；例如标题栏 **40**（`--lithe-title-bar-height: 2.5rem`）、
//!    项目标签条 **32**、活动栏 **38**、状态栏 **24**（`--lithe-footer-height: 1.5rem`）、
//!    左栏 **320**（`settings.sidebarWidth`）、右工具窗 **400**、工作区间隔 **4**
//!    （`--lithe-workbench-gap`）、编辑器岛圆角 **11.2**（`--radius * 1.4`）、
//!    底部窗 **320**。这些数字每一个都能在源码里指到出处。
//! 2. **改 rem 会破坏"逐值搬"的可核对性**。`p_2()` 之类的值取决于 theme base font，
//!    换算之后规格文档里的 `40` / `24` / `320` 就对不上了，评审时无法一眼验证；
//!    而 `UI-MAP-WINDOWS.md` 的逐区域对照表正是按这些 px 值写的。
//! 3. **本项目的缩放策略还没定**。指南描述的是"以 base font 为应用缩放基准"的模型；
//!    在决定 Lithe 是否要用这套缩放之前，把度量改成 rem 会提前锁死一个未评审的设计选择。
//!
//! 因此：**保留 px，但把它当作一条已登记的偏离**，而不是"忘了按指南做"。
//! 区域文件里每个 `const` 都带出处注释；将来若确定采用 rem 缩放，应按区域整体换算并在
//! `UI-MAP-WINDOWS.md` 里同步更新对照表，而不是零散改几个值。
//!
//! 颜色仍然一律走 `cx.theme()`（不写裸色值），这条没有偏离。
//!
//! ## 公开边界
//!
//! 只有 [`ShellWorkspace`]（工作台根视图）是公开 API；四个区域模块对外只暴露它们的渲染函数。
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
pub mod project_tabs;
pub mod status_bar;
pub mod title_bar;
pub mod workspace;

pub use workspace::ShellWorkspace;
