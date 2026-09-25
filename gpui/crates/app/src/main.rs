//! `Lithe` bin 目标的 crate 根，也是 **App Shell**（应用外壳）。
//!
//! ## 职责（《编码指南》「架构总览」）
//!
//! App Shell **只组合窗口与 Feature**，不承载任何具体 Feature 逻辑：命令行参数、首窗口尺寸、
//! 应用启动顺序、主题加载，以及把 `lithe-gpui-workbench` 的 `ShellWorkspace` 挂到 `Root` 下。
//!
//! 旧的 P1 冒烟宿主（`gpui/shell/src/main.rs`，bin 目标 `lithe-gpui-shell`）已删除：
//! 它的职责被本文件完全覆盖，而指南要求「一个应用一个 shell」。
//!
//! ## 启动顺序（0.6.6 只有这一种写法，顺序错会静默失败或 panic）
//!
//! `application().with_assets(..).run` → `gpui_kit::init` → `Theme::change` →
//! `cx.spawn` → `open_window` → `Root::new`。`Root` **必须是窗口的第一层**：它负责对话框、
//! 抽屉、通知与焦点归还；`Root::new` 之前不得打开任何浮层（那时窗口根还不是 `Root`，
//! `window.open_dialog` 会 panic）。
//!
//! ## 国际化（i18n）不在本文件
//!
//! `rust_i18n::i18n!` **必须出现在持有 `locales/` 的那个 crate 的根**：`rust_i18n::t!` 展开成
//! `crate::_rust_i18n_try_translate(..)`（`rust-i18n-macro-4.2.2/src/tr.rs:438,454`），
//! 而该函数由 `i18n!` 在**调用它的那个 crate** 里生成。本项目的 `t!` 包装
//! （`tr` / `tr_args`，含 `{name}` 插值）住在 `lithe-gpui-shared`，所以 locale 与 loader
//! 也落在那里（`gpui/crates/shared/locales/`，由 `gpui/tools/extract-locale.mjs` 从
//! `windows/tauri/src/i18n/locale.ts` 生成）。**不要在这里再调一次 `i18n!`** —— 那会生成
//! 第二份 backend，变成两个真相源。
//!
//! 运行：`cargo run --bin Lithe -- <workspace-root>`

// Windows 发布构建**不要弹控制台窗口**：GUI 程序带一个黑底控制台是明显的产品缺陷
// （Explorer 里双击会多出一个窗口，实测抓到的那个控制台是 1239x647）。
// **只在 release 关掉**，debug 保留控制台：`S1_*` 诊断行、`--help` 与 `parse_options`
// 的错误都走 stdout/stderr，开发和机器验证都要看得见；即使关掉控制台，只要启动方把
// stdout/stderr 重定向到文件，`println!` 仍然照常写入。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use gpui_kit::component::{Root, Theme, ThemeMode, TitleBar};
use gpui_kit::{
    App, AppContext as _, Bounds, Pixels, SharedString, Size, Window, WindowBounds, WindowOptions,
    point, px, size,
};
use lithe_gpui_settings::{Init, SettingsStore};
use lithe_gpui_workbench::ShellWorkspace;
// `RightToolWindowView` 走模块路径引入（`workbench` 的根只重导出 `ShellWorkspace`，
// 见 `crates/workbench/src/lib.rs:121`）。
use lithe_gpui_workbench::right_tool_window::RightToolWindowView;

/// 资源源与图标 helper（`gpui/assets/**` 的接线）。模块文档里有 `AssetSource` 委托顺序与
/// "为什么必须包装而不是注册两次"的论证。
mod assets;

/// 首窗口尺寸：**与同目录的 Dodona 保持同一口径** —— 取显示器可见区域的 **94%**、居中、
/// 普通窗口（不最大化）。
///
/// 出处：`D:\developmentProjects\rust\Dodona\crates\dodona\src\main.rs:91-108`
/// （GPUI Kit 的成熟参考应用，本机验证过界面正常）：
/// `visible = display.visible_bounds()`（排除任务栏，状态栏才够得着）→
/// `wanted = visible * 0.94` → 夹在 `MIN_WINDOW(1024×680)` 与 `visible` 之间 → 居中。
///
/// 为什么不用别的两种写法：
/// - **照抄 macOS 常量（1440×900）**：那是 macOS 的窗口习惯（带交通灯、不最大化），搬到 Windows
///   会得到一个比屏幕小的窗口，四周露出桌面；
/// - **启动即最大化**（`WindowBounds::Maximized` / `zoom_window()`）：`Maximized` 在这个 gpui
///   版本上只把 bounds 当还原尺寸、并不会真的最大化；Dodona 验证过的口径是"94% 的普通窗口"。
fn startup_window_bounds(cx: &App) -> WindowBounds {
    // 窗口能用的最小尺寸（与 Dodona 一致）。
    const MIN_WINDOW: Size<Pixels> = size(px(1024.), px(680.));
    let fallback = size(px(1280.), px(800.));

    let Some(display) = cx.primary_display() else {
        return WindowBounds::Windowed(Bounds::new(point(px(60.), px(40.)), fallback));
    };

    // `visible_bounds` 已经排除任务栏，所以状态栏始终够得着。
    let visible = display.visible_bounds();
    let wanted = size(visible.size.width * 0.94, visible.size.height * 0.94);
    let size = wanted.max(&MIN_WINDOW).min(&visible.size);
    let origin = point(
        visible.origin.x + (visible.size.width - size.width) / 2.0,
        visible.origin.y + (visible.size.height - size.height) / 2.0,
    );

    WindowBounds::Windowed(Bounds::new(origin, size))
}

/// 命令行参数。
///
/// ⚠️ `--theme` / `--locale` / `--palette-keys` 是**显式覆盖**，不是设置的常规入口：
/// 「配色主题」与「显示语言」的正规来源是设置文件（`gpui/crates/settings`，设置界面的
/// 「外观 / 常规」两页）。这些开关的作用域只有"这一次启动"，**不写回设置文件**，
/// 保留它们是为了让主题 / 语言 / 键盘交互能在一个进程里被可复现地指定
/// （`.artifacts/verify-visual.ps1` 的 4 配置视觉验证依赖后者；
/// `--palette-keys` 的理由见下）。
struct Options {
    /// 工作区根，传给 `workspace.snapshot` 与 `git.*`。
    root: PathBuf,
    /// 本次启动要应用的主题名（覆盖设置文件；`themes/*.json` 里 `themes[].name`）。
    theme_override: Option<SharedString>,
    /// 本次启动的界面语言（覆盖设置文件）。gpui-kit 组件自带 `en` / `zh-CN` / `zh-HK`。
    locale_override: Option<String>,
    /// 启动后自动打开设置对话框（**验证/诊断用**：让机器能截到设置界面的图）。
    open_settings: bool,
    /// 启动后自动打开**命令面板**（**验证/诊断用**）。
    ///
    /// 为什么需要它：本机锁屏导致键盘注入到不了应用（理由见 [`Options::palette_keys`]），
    /// 而命令面板的**唯一**入口就是 `Ctrl+Shift+P`；没有这个开关，"面板打开长什么样"
    /// 这件事在无人值守环境里无法取证。它调的是与按键同一条链：
    /// `window.dispatch_action(OpenCommandPalette)` → `App::on_action` 的全局处理器
    /// （`crate::command_palette::install_actions`）→ `open_command_palette`。
    /// 与已有的 `--open-settings` 同一性质，不是产品能力。
    open_palette: bool,
    /// 启动后按顺序派发的一串串按键（**验证/诊断用**，可重复给多次 = 多串）。
    ///
    /// 用途与实测边界见 [`dispatch_next_key`] 的文档：这台工作站当前**锁屏**，
    /// `PostMessage` 投递的 `WM_KEYDOWN` 到不了 GPUI 的按键回调（同一通路上的
    /// `WM_LBUTTONDOWN` / `WM_CHAR` 照常生效，5 组对照实验见
    /// `.artifacts/command-palette/NOTES.md` §2），所以真实按键既打不开面板、也做不了
    /// 过滤。这个开关把按键交给 **gpui 自己的按键派发**
    /// （`Window::dispatch_keystroke`，`gpui-pre-0.3.6/src/window.rs:5373` 起：
    /// 构造成 `KeyDownEvent` 后走 `dispatch_event` 的完整路径）。
    ///
    /// ⚠️ 它**不能**代替真机验证"操作系统把 Ctrl+Shift+P 送进窗口"这一段；本轮实测连
    /// 已登记的 `ctrl-,` 都不命中（原因见上），所以本轮的键盘验证改走鼠标（`CommandItem`
    /// 的行点击与 `Enter` 在 gpui 里是同一个 `confirm`）。
    palette_keys: Vec<Vec<gpui_kit::Keystroke>>,
    /// 菜单栏走"左上角图标 + 浮动胶囊"形态（**验证/诊断用**）。
    ///
    /// 真源的默认值就是这一支（`default-settings.ts:105` 的 `compactMenuBar: true`），
    /// 而本侧默认取常驻（维护者要求"固定在界面上"）。两种形态是同一份菜单表、
    /// 只差容器与定位，所以这个开关让"两种形态长什么样"都能在无人值守环境里截到图
    /// （与 `--open-palette` 同一性质，不是产品能力）。
    compact_menu_bar: bool,
    /// `--menu-probe <顶级菜单 id> [动作名]`（**验证/诊断用**，见 [`run_menu_probe`]）。
    menu_probe: Option<(String, Option<String>)>,
    /// `--menu-probe` 里"打开菜单"与"执行动作"之间的等待毫秒数。
    ///
    /// 默认 2500：够截图脚本在**动作执行之前**抓到"下拉面板真的画出来了"那一帧。
    menu_probe_delay_ms: u64,
    /// `--menu-probe` 打开菜单之前等多久（默认 0 = 首帧就打开）。
    ///
    /// **为什么需要它**：无人值守环境里窗口不是前台窗口（工作站锁屏），GPUI 只在自己
    /// 认为脏的时候画一帧，而 `PrintWindow` 只把**上一次绘制的那一帧**画进位图 ——
    /// 首帧就打开的菜单会出现在首帧里（能截到），但"下一秒才打开"的菜单在窗口没有新
    /// 输入时**永远画不出来**（`.artifacts/p7/NOTES.md` §4 有实测对照）。
    /// 所以要把"菜单打开"对齐到**启动时的某一帧**上，让那一帧就是打开态。
    menu_probe_open_ms: u64,
    /// `--theme-probe <下标|主题名>`：执行「视图 → 主题」子菜单的一项（**验证/诊断用**）。
    ///
    /// 主题项是**动态生成**的（`gpui/themes/*.json` 与 gpui-kit 内置主题合起来才是注册表），
    /// 没有固定动作名，[`lithe_gpui_workbench::menu_bar::lookup_action`] 覆盖不到它。
    /// 走的仍是主题项 `on_click` 里那段同一份代码（见 `menu_bar::apply_theme_choice`）。
    theme_probe: Option<String>,
    /// `--project-menu-probe`：启动后把标题栏的**项目下拉**打开（**验证/诊断用**）。
    ///
    /// 见 [`run_project_menu_probe`]：走的是与"点触发器"完全相同的那段状态迁移。
    project_menu_probe: bool,
    /// `--branch-panel-probe`：启动后把标题栏的**分支弹窗**打开（**验证/诊断用**）。
    ///
    /// 见 `lithe_gpui_workbench::branch_panel::open_branch_panel`：走的是与"点标题栏分支项"
    /// 完全相同的那段代码（先 `BranchPanel::open` 重读数据，再 `window.open_dialog`），
    /// 被绕开的只有"操作系统把这次点击送进窗口"那一段。
    branch_panel_probe: bool,
    /// `--right-view <id>`：启动后把**右侧工具窗**切到指定视图并展开（**验证/诊断用**）。
    ///
    /// 见 [`lithe_gpui_workbench::ShellWorkspace::show_right_view_probe`]：走的是与"点右活动栏
    /// 那一项"完全相同的状态迁移（含懒扫与 `S1_RIGHT_PANEL` 诊断），被绕开的只有"操作系统把
    /// 那一下点击送进窗口"那一段。
    ///
    /// **为什么需要它**：本机（125% DPI）右活动栏一次点击会被处理成两次
    /// （`S1_RIGHT_PANEL … visible=true` 紧跟一行 `visible=false`，见 `gpui/HANDOFF.md` §2），
    /// 面板随即被自己收起 —— 于是"Spring 面板里到底画出了什么"在无人值守环境里无法取证。
    right_view: Option<String>,
    /// `--open-project-probe <目录>`：启动后走一遍「打开其他文件夹」的链路（**验证/诊断用**）。
    ///
    /// 见 [`lithe_gpui_workbench::ShellWorkspace::open_project_probe`]：它调的是与"在选择器里
    /// 选完一个文件夹"**同一个** `request_open_project` —— 被绕开的只有"操作系统把这次选择
    /// 送回来"那一段（那一段要真的弹 `IFileOpenDialog`，本机锁屏时点不了）。
    open_project_probe: Option<PathBuf>,
    /// `--open-project-destination <this-window|new-window>`：给 `--open-project-probe`
    /// 指定目的地（= 真源的 `explicitDestination`），于是**不弹对话框**、直接执行。
    ///
    /// **为什么需要它**：不指定时（照真源默认设置）`--open-project-probe` 会弹换项目对话框
    /// —— 那正是"截对话框那一帧"要的；而"换根真的生效"这条验收线要在无人值守下**点掉**那个
    /// 对话框，只能靠这个参数把它显式定下来。取值拼错时当场报错退出（与 `--palette-keys` 同口径）。
    open_project_destination: Option<String>,
    /// `--open-project-remember`：配合 `--open-project-probe`，把"用户勾了「不再询问」"这件事
    /// 写进对话框那个字段（**验证/诊断用**）。
    ///
    /// 本机工作站锁屏、点不了 checkbox，而"勾上之后点「此窗口」→ 换根 → 两个偏好键落盘 →
    /// 下一次不再弹对话框"这条链是要逐环取证的。它写的字段与 checkbox 的 `on_change`
    /// 写的是**同一个**（不是另一套状态）。
    open_project_remember: bool,
    /// `--open-project-delay-ms <毫秒>`：`--open-project-probe` 等多久才真的走那条链路。
    ///
    /// 与 `--menu-probe-delay` 同一个用途：给"换根**之前**先制造出要被释放的东西"留出窗口
    /// （验证无残留进程时，先让 `--menu-probe terminal …` 起一个终端页签 + JDTLS 会话，
    /// 再换根，然后对照进程数）。
    open_project_delay_ms: u64,
    /// `--left-view <下标|id>`：启动后把**左侧栏**切到指定视图（**验证/诊断用**）。
    ///
    /// 见 [`lithe_gpui_workbench::ShellWorkspace::show_left_view_probe`]。存在理由同
    /// `right_view`：验收要证明"换根后 Git 变更视图指向新根"，而
    /// `S1_SOURCE_CONTROL files=… root=…` 只在变更视图 `activate` 时打一行。
    left_view: Option<String>,
}

/// 解析 `<workspace-root> [--theme <名>] [--locale <tag>] [--open-settings] [--open-palette] [--compact-menu-bar] [--palette-keys <串>] [--right-view <id>]`。
fn parse_options() -> Result<Options, String> {
    const USAGE: &str = "用法：Lithe <workspace-root> [--theme <主题名>] [--locale <语言>] [--open-settings] [--open-palette] [--compact-menu-bar] [--palette-keys <按键串>] [--right-view <id>]\n\
         \x20 --theme <主题名>     本次启动使用的主题（覆盖设置文件；验证/诊断用）\n\
         \x20 --locale <语言>      本次启动使用的界面语言（覆盖设置文件；验证/诊断用）\n\
         \x20 --open-settings     启动后自动打开设置对话框（验证/诊断用）\n\
         \x20 --open-palette      启动后自动打开命令面板（验证/诊断用）\n\
         \x20 --compact-menu-bar  菜单栏走「左上角图标 + 浮动胶囊」形态（真源默认值；验证/诊断用）\n\
         \x20 --menu-probe <id> [动作名]\n\
         \x20                     打开某个顶级菜单，并在 2.5s 后执行指定动作（验证/诊断用；\n\
         \x20                     例：\"--menu-probe view lithe.menu.toggleTerminal\"）；动作名取\n\
         \x20                     `S1_MENU_RUN id=` 的值，省略动作名时只打开菜单\n\
         \x20 --theme-probe <下标> 执行「视图 → 主题」子菜单的第 N 项（验证/诊断用；主题项是\n\
         \x20                     动态生成的，没有固定动作名，所以单独一个开关）\n\
         \x20 --project-menu-probe  启动后打开标题栏的项目下拉（验证/诊断用；走的是与\n\
         \x20                     「点触发器」相同的那段状态迁移）\n\
         \x20 --branch-panel-probe  启动后打开标题栏的分支弹窗（验证/诊断用；走的是与\n\
         \x20                     「点标题栏分支项」相同的那段代码）\n\
         \x20 --right-view <id>     启动后把右侧工具窗切到 <id> 并展开（验证/诊断用；走的是与\n\
         \x20                     「点右活动栏那一项」相同的状态迁移）；<id> 取 extensions /\n\
         \x20                     notifications / maven / spring，未知 id 报一行错且不改启动状态\n\
         \x20 --open-project-probe <目录>\n\
         \x20                     启动后走一遍「打开文件夹」的链路并打开 <目录>（验证/诊断用；\n\
         \x20                     走的是与\"在选择器里选完一个文件夹\"相同的 request_open_project，\n\
         \x20                     被绕开的只有操作系统那个文件夹选择器）；不给\n\
         \x20                     --open-project-destination 时会照真源默认设置弹出换项目对话框\n\
         \x20 --open-project-destination <this-window|new-window>\n\
         \x20                     给 --open-project-probe 指定目的地（= 真源的 explicitDestination，\n\
         \x20                     于是不弹对话框、直接执行）；取值拼错时报错退出\n\
         \x20 --open-project-remember\n\
         \x20                     配合 --open-project-probe：把「勾了不再询问」写进对话框那个字段，\n\
         \x20                     于是换根成功之后会写两个偏好键（askWhereToOpenProjects /\n\
         \x20                     openFoldersInNewWindow）\n\
         \x20 --open-project-delay-ms <毫秒>  --open-project-probe 等多久才执行（默认 0 = 首帧之后；\n\
         \x20                     给「换根之前先造出要被释放的东西」留窗口）\n\
         \x20 --left-view <id>     启动后把左栏切到 <id>（验证/诊断用；files / changes / search）；\n\
         \x20                     「更改」还会顺带重读一次工作区状态，于是能拿到\n\
         \x20                     `S1_SOURCE_CONTROL files=… root=…` 那一行\n\
         \x20 --menu-probe-delay <毫秒>  `--menu-probe` 打开菜单后等多久才执行动作（默认 2500）\n\
         \x20 --palette-keys <串> 启动后按顺序派发一串按键，逗号分隔；可重复给多次 = 多串（验证/诊断用；\n\
         \x20                     例：\"ctrl-shift-p,n,down,enter,escape\"）";
    let mut args = std::env::args().skip(1);
    let mut root: Option<PathBuf> = None;
    let mut theme_override: Option<SharedString> = None;
    let mut locale_override: Option<String> = None;
    let mut open_settings = false;
    let mut open_palette = false;
    let mut compact_menu_bar = false;
    let mut menu_probe: Option<(String, Option<String>)> = None;
    let mut menu_probe_delay_ms: u64 = 2500;
    let mut menu_probe_open_ms: u64 = 0;
    let mut theme_probe: Option<String> = None;
    let mut palette_keys = Vec::new();
    let mut project_menu_probe = false;
    let mut branch_panel_probe = false;
    let mut right_view: Option<String> = None;
    let mut open_project_probe: Option<PathBuf> = None;
    let mut open_project_destination: Option<String> = None;
    let mut open_project_remember = false;
    let mut open_project_delay_ms: u64 = 0;
    let mut left_view: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--theme" => theme_override = Some(args.next().ok_or("--theme 缺少值")?.into()),
            "--locale" => locale_override = Some(args.next().ok_or("--locale 缺少值")?),
            "--open-settings" => open_settings = true,
            "--open-palette" => open_palette = true,
            "--compact-menu-bar" => compact_menu_bar = true,
            "--menu-probe" => {
                let menu = args.next().ok_or("--menu-probe 缺少顶级菜单 id")?;
                // 第二段是**可选**的：`--menu-probe view` 只打开菜单（用来截"下拉面板"那一帧），
                // `--menu-probe view lithe.menu.toggleTerminal` 才会接着执行动作。
                // 判据是"下一个参数看起来像不像一个动作名"（动作名一律以 `lithe.` 开头）；
                // 这里先把 `next()` 的**所有权**取出来，是 `libgit2` 之外唯一的读法 ——
                // `std::env::Args` 不实现 `Clone`（本轮实测 `args.clone()` 报 E0599）。
                let next = args.next();
                let action = match next {
                    Some(ref candidate) if candidate.starts_with("lithe.") => Some(candidate.clone()),
                    Some(candidate) => {
                        // 不像动作名 → 它一定是别的参数：迭代器是单向的、**放不回去**，
                        // 所以直接报错退出，避免把一个拼错的参数静默吃掉。
                        return Err(format!(
                            "--menu-probe 的第二个参数 {candidate:?} 不像动作名（应以 lithe. 开头）"
                        ));
                    }
                    None => None,
                };
                menu_probe = Some((menu, action));
            }
            "--menu-probe-delay" => {
                let raw = args.next().ok_or("--menu-probe-delay 缺少毫秒数")?;
                menu_probe_delay_ms = raw
                    .parse::<u64>()
                    .map_err(|error| format!("--menu-probe-delay 的 {raw:?} 不是毫秒数：{error}"))?;
            }
            "--menu-probe-open-ms" => {
                let raw = args.next().ok_or("--menu-probe-open-ms 缺少毫秒数")?;
                menu_probe_open_ms = raw
                    .parse::<u64>()
                    .map_err(|error| format!("--menu-probe-open-ms 的 {raw:?} 不是毫秒数：{error}"))?;
            }
            "--theme-probe" => {
                theme_probe = Some(args.next().ok_or("--theme-probe 缺少下标或主题名")?);
            }
            "--project-menu-probe" => project_menu_probe = true,
            "--branch-panel-probe" => branch_panel_probe = true,
            // `--right-view` 只**收下**这串字符，解析成视图放到窗口建好之后的那一段做：
            // 未知 id 要在启动状态还没被改动时报错（见 `main` 里的 `on_next_frame`）。
            "--right-view" => {
                right_view = Some(args.next().ok_or("--right-view 缺少视图 id")?);
            }
            // `--open-project-probe` 只**收下**路径，真正走的那一段放到窗口建好之后
            // （`on_next_frame`）：对话框只能在 `Root` 就位之后开。
            "--open-project-probe" => {
                open_project_probe =
                    Some(PathBuf::from(args.next().ok_or("--open-project-probe 缺少目录")?));
            }
            "--open-project-destination" => {
                let raw = args.next().ok_or("--open-project-destination 缺少取值")?;
                // 取值先在这里校验：拼错时当场退出，别让探针静默弹出一个没人点的对话框。
                if raw != "this-window" && raw != "new-window" {
                    return Err(format!(
                        "--open-project-destination 的 {raw:?} 不是 this-window / new-window"
                    ));
                }
                open_project_destination = Some(raw);
            }
            "--open-project-remember" => open_project_remember = true,
            "--open-project-delay-ms" => {
                let raw = args.next().ok_or("--open-project-delay-ms 缺少毫秒数")?;
                open_project_delay_ms = raw.parse::<u64>().map_err(|error| {
                    format!("--open-project-delay-ms 的 {raw:?} 不是毫秒数：{error}")
                })?;
            }
            "--left-view" => {
                left_view = Some(args.next().ok_or("--left-view 缺少视图 id")?);
            }
            "--palette-keys" => {
                let raw = args.next().ok_or("--palette-keys 缺少值")?;
                let mut sequence = Vec::new();
                for token in raw.split(',') {
                    let token = token.trim();
                    if token.is_empty() {
                        continue;
                    }
                    // 解析失败**直接报错退出**：静默跳过一个键会让验证脚本得出错误结论。
                    let keystroke = gpui_kit::Keystroke::parse(token)
                        .map_err(|error| format!("--palette-keys 里解析不了 {token:?}：{error}"))?;
                    sequence.push(keystroke);
                }
                palette_keys.push(sequence);
            }
            "-h" | "--help" => return Err(USAGE.to_string()),
            other if root.is_none() && !other.starts_with("--") => {
                root = Some(PathBuf::from(other));
            }
            other => return Err(format!("未知参数 {other}\n\n{USAGE}")),
        }
    }

    Ok(Options {
        root: root.ok_or_else(|| USAGE.to_string())?,
        theme_override,
        locale_override,
        open_settings,
        open_palette,
        compact_menu_bar,
        menu_probe,
        menu_probe_delay_ms,
        menu_probe_open_ms,
        theme_probe,
        palette_keys,
        project_menu_probe,
        branch_panel_probe,
        right_view,
        open_project_probe,
        open_project_destination,
        open_project_remember,
        open_project_delay_ms,
        left_view,
    })
}

/// `--palette-keys` 的派发计划：若干串按键 + 派发前要聚焦的根节点。
///
/// **为什么必须先聚焦**：`dispatch_key_event`（`gpui-pre-0.3.6/src/window.rs:5815-5816`）拿
/// `focus_node_id_in_rendered_frame(self.focus)` 定出派发路径，`self.focus` 为 `None` 时路径
/// 为空，`dispatch_tree.dispatch_key(..)`（同文件 `:5880`）就匹配不到任何 keymap 绑定。
/// 真机上这个焦点由"用户点过界面里的某个元素"建立，无人值守启动必须自己给
/// （`ShellWorkspace` 的根元素就是那个兜底锚点）。
///
/// ⚠️ 实测结论：**键盘派发没有让 keymap 命中** —— 同一个进程里 `ctrl-,`
/// （已登记的 `lithe_settings::OpenSettings`）与 `ctrl-shift-p` 都不触发 action，
/// 5 组对照实验（scan code / 直接投 `WM_GPUI_KEYDOWN` / `AttachThreadInput` / 显式聚焦）
/// 见 `.artifacts/command-palette/NOTES.md` §2。所以本轮验证**不依赖**这条路径，
/// 它只保留成"复现实验"的入口。
struct KeyPlan {
    focus: gpui_kit::FocusHandle,
    rounds: std::collections::VecDeque<std::collections::VecDeque<gpui_kit::Keystroke>>,
}

/// 派发 `--palette-keys` 里的下一个按键，并把自己排到下一帧（**每帧一个**）。
///
/// `Window::dispatch_keystroke`（`gpui-pre-0.3.6/src/window.rs:5373-5396`）把按键交给
/// gpui 自己的派发路径；每帧一个是为了让上一个键的 `InputState` 变更（以及它触发的重绘）
/// 在这一帧结束前落地，与真实敲键的时序一致。
///
/// 计划是**按值传递**的（`on_next_frame` 的回调要 `'static`，借用逃不出去）。
fn dispatch_next_key(window: &mut Window, cx: &mut App, mut plan: KeyPlan) {
    if window.focused(cx).is_none() {
        window.focus(&plan.focus, cx);
    }

    let Some(keystroke) = plan.rounds.front_mut().and_then(|round| round.pop_front()) else {
        // 当前这一串发完了；还有下一串就继续（每帧一个键，串与串之间自然有帧间隔）。
        if plan.rounds.pop_front().is_none() || plan.rounds.is_empty() {
            println!("S1_KEYS done");
            return;
        }
        println!("S1_KEYS next_sequence");
        window.on_next_frame(move |window, cx| dispatch_next_key(window, cx, plan));
        return;
    };

    println!(
        "S1_KEYS dispatch={keystroke} focused={}",
        window.focused(cx).is_some()
    );
    window.dispatch_keystroke(keystroke, cx);
    if plan.rounds.is_empty() {
        println!("S1_KEYS done");
        return;
    }
    window.on_next_frame(move |window, cx| dispatch_next_key(window, cx, plan));
}

/// `--menu-probe` 的驱动函数：打开指定顶级菜单，并在 `delay_ms` 之后执行指定动作。
///
/// **为什么需要这个入口**（不是产品能力）：本机的工作站当前**锁屏**，鼠标注入的**两条路都无效**
/// —— `PostMessage(WM_LBUTTONDOWN/UP)` 与 `SetCursorPos + mouse_event` 实测都到不了应用
/// （`.artifacts/p7/NOTES.md` §3 有对照实验）。而"主菜单栏"这件东西的要紧处正好是
/// "点下去会不会开、会不会真的改到状态"，没有这个入口就无法在无人值守环境里取证。
///
/// ⚠️ 它**不绕开**菜单：打开走 [`gpui::WeakEntity<MenuBar>`] 上那个与点击回调同一个
/// `MenuBar::toggle`，执行走 `MenuBar::run_action` → `push_run`（点击回调里唯一干的事）
/// → 外壳下一帧 `drain_runs` + `apply_menu_action`。被绕开的只有"操作系统把这次点击
/// 送进窗口"那一段，与 `--open-palette` 绕开 `Ctrl+Shift+P` 是同一条口径。
fn run_menu_probe(
    menu_id: String,
    action_id: Option<String>,
    open_ms: u64,
    delay_ms: u64,
    window: &mut Window,
    cx: &mut App,
) {
    // 动作名先解析：拼错时**当场报错退出**，不静默跑一半（与 `--palette-keys` 同一条口径）。
    let action = match action_id {
        Some(id) => match lithe_gpui_workbench::menu_bar::lookup_action(&id) {
            Some(action) => Some(action),
            None => {
                eprintln!("--menu-probe：菜单里没有动作 {id}");
                return;
            }
        },
        None => None,
    };
    // `handle()` 返回的是句柄本身（不是 `Option`）：登记在 `ShellWorkspace::new` 里发生，
    // 而 `--menu-probe` 只在窗口建好之后才跑，走到这里一定已经登记过。
    let bar = lithe_gpui_workbench::menu_bar::handle();

    if open_ms == 0 {
        // 默认：**首帧**就打开。这样"菜单打开"会出现在启动时画的头几帧里，
        // 而无人值守环境里那几帧正是唯一能被 `PrintWindow` 拿到的新帧。
        let open_bar = bar.clone();
        window.on_next_frame(move |_window, cx| {
            let menu_id = menu_id.clone();
            let _ = open_bar.update(cx, |bar, cx| {
                if !bar.open_by_id(&menu_id, cx) {
                    eprintln!("--menu-probe：没有顶级菜单 {menu_id}");
                }
            });
        });
    } else {
        // `--menu-probe-open-ms N`：第 N 毫秒才打开。给"先截基线、再让菜单出现"这种
        // 需要**两次新鲜绘制**的场景用（此时截图脚本要在 N 之后先戳一次重绘再截）。
        let open_bar = bar.clone();
        let executor = cx.background_executor().clone();
        let opened = cx.spawn(async move |cx| {
            executor
                .timer(std::time::Duration::from_millis(open_ms))
                .await;
            cx.update(move |cx| {
                let _ = open_bar.update(cx, |bar, cx| {
                    if !bar.open_by_id(&menu_id, cx) {
                        eprintln!("--menu-probe：没有顶级菜单 {menu_id}");
                    }
                });
            });
        });
        opened.detach();
    }

    let Some(action) = action else {
        return;
    };
    // 再等 `delay_ms` 才执行动作：给截图脚本留出"下拉面板已经画出来"的取证窗口。
    //
    // ⚠️ 这里用 `App::spawn` 而**不是** `Context::spawn_in`：本函数在 `Root::new` **之前**
    // 被调用，那时还没有任何视图可以挂这个任务（`spawn_in` 定义在 `Context<T>` 上，
    // `App` 一侧只有 `spawn`；本轮实测 `cx.spawn_in(..)` 在 `&mut App` 上报 E0599）。
    // 计时器取 `background_spawn` 的池（**不能在 `foreground_spawn` 上阻塞地
    // `block_on` 一个 timer** —— 前台线程被占住时定时器永远不触发，任务会静默死掉）。
    let executor = cx.background_executor().clone();
    let delayed = cx.spawn(async move |cx| {
        executor
            .timer(std::time::Duration::from_millis(delay_ms))
            .await;
        println!("S1_MENU_PROBE run_after_ms={delay_ms}");
        cx.update(move |cx| {
            let _ = bar.update(cx, |bar, cx| bar.run_action(action, cx));
        });
    });
    delayed.detach();
}

/// `--project-menu-probe` 的驱动函数：**首帧之后**把标题栏的项目下拉打开。
///
/// **为什么需要这个入口**（不是产品能力）：与 [`run_menu_probe`] 完全同因 —— 本机工作站
/// 当前**锁屏**，鼠标/键盘注入到不了应用，而"项目下拉长什么样、当前项目行有没有高亮打勾、
/// 空态文案对不对"这些正是本任务的要紧处；没有它就无法在无人值守环境里取证。
///
/// ⚠️ 它**不绕开**面板：调的是 [`lithe_gpui_workbench::project_menu::ProjectMenu::open_by_probe`]，
/// 它写的那个 `open` 字段与"点触发器"回调（`ProjectMenu::toggle`）写的是**同一个**，
/// 渲染也是同一段代码。被绕开的只有"操作系统把这次点击送进窗口"那一段，
/// 与 `--menu-probe` 绕开点击、`--open-palette` 绕开 `Ctrl+Shift+P` 是同一条口径。
///
/// ⚠️ 必须等**首帧之后**：`Root::new` 还没返回时窗口根不是 `Root`，浮层只能在事件回调或
/// 任务里打开（`render` 阶段会 panic）；而 `PrintWindow` 只能拿到"已经画出来的帧"，
/// 启动期画的头几帧正是无人值守环境里唯一能截到的新帧（理由见 `menu_probe_open_ms` 的文档）。
fn run_project_menu_probe(window: &mut Window) {
    let menu = lithe_gpui_workbench::project_menu::handle();
    window.on_next_frame(move |_window, cx| {
        let _ = menu.update(cx, |menu, cx| menu.open_by_probe(cx));
    });
}

/// bin 目标的入口点。
/// 启动顺序（0.6.6 只有这一种写法，顺序错会静默失败或 panic）：
/// `application().with_assets(..).run` → `set_locale` → `gpui_kit::init` → `Theme::change` →
/// 设置（读文件 + 主题监听 + 动作）→ `cx.spawn` → `open_window` → `Root::new`。
/// `Root` **必须是窗口的第一层**：它负责对话框、抽屉、通知与焦点归还；`Root::new` 之前不得
/// 打开任何浮层（那时窗口根还不是 `Root`，`window.open_dialog` 会 panic）。
fn main() {
    let Options {
        root,
        theme_override,
        locale_override,
        open_settings,
        open_palette,
        compact_menu_bar,
        menu_probe,
        menu_probe_delay_ms,
        menu_probe_open_ms,
        theme_probe,
        palette_keys,
        project_menu_probe,
        branch_panel_probe,
        right_view,
        open_project_probe,
        open_project_destination,
        open_project_remember,
        open_project_delay_ms,
        left_view,
    } = match parse_options() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    // `--right-view <id>`：**先解析**，未知 id 在这里就报一行错并**不改动启动状态**
    // （解析成 `None` 之后，窗口那段 `if let Some(view)` 整段不执行，右工具窗保持
    // "隐藏 + `Maven`"的默认值，与不带这个参数时完全一样）。
    // 报错走 stderr：stdout 重定向到文件时是块缓冲的，进程还在跑时可能一行都看不到
    // （理由与本文件开头的 `S1_ASSETS` 一致）。
    let right_view_probe = right_view.and_then(|id| match RightToolWindowView::from_id(&id) {
        Some(view) => Some(view),
        None => {
            eprintln!(
                "--right-view：未知视图 {id}（可用取值：extensions / notifications / maven / spring）"
            );
            None
        }
    });
    // `--left-view <id>`：同一套（未知取值报一行错，不改动启动状态）。
    let left_view_probe =
        left_view
            .as_deref()
            .and_then(|selector| match lithe_gpui_workbench::left_activity_index(selector) {
                Some(index) => Some(index),
                None => {
                    eprintln!(
                        "--left-view：未知视图 {selector}（可用取值：files / changes / search）"
                    );
                    None
                }
            });

    // 设置文件必须在 `set_locale` 之前读：界面语言的常规来源就是它
    // （`--locale` 只是本次启动的显式覆盖）。这一步是纯文件读取，不需要 `App`。
    let loaded = lithe_gpui_settings::load();
    let locale = locale_override
        .clone()
        .unwrap_or_else(|| loaded.settings.gpui_locale().to_string());

    gpui_kit::application()
        // 资源源分两层（`src/assets.rs`）：
        //
        // 1. **我们自己搬进来的资源**（`gpui/assets/**`，用 `rust-embed` 编译期内嵌）：
        //    `ui-icons/**`（157 个 IntelliJ `expui` SVG + 1 个旧前端生成物）、
        //    `icon-themes/**`（4 套文件类型图标包，1 027 个 SVG）、`icons/**` 与
        //    `images/logo.png`（应用图标/brand 图）。
        // 2. **回落 gpui-kit 的全量 Lucide 字形**（1830 个，`gpui_kit::assets::AllAssets`）。
        //
        // ⚠️ 只能 `with_assets` **一次**：它签名是 `impl AssetSource`，第二次调用是**覆盖**
        // 而不是叠加（`gpui-pre-0.3.6/src/app.rs:198-206`）。所以两者由 `LitheAssets` 组合。
        //
        // 为什么必须是 `AllAssets`（全量）而不是默认的 `Assets`（101 个字形子集）：
        // `Assets` 由 `gpui-kit-assets-0.6.6/build.rs` 按 `default-icons.txt` 过滤生成
        // （`native_assets.rs:5` 的 `include!(default_assets.rs)`），里面**没有任何 `git-*`
        // 字形**；而 Windows 规格的源码管理 / 提交记录 / 分支全要 git 字形
        // （`IconName::GitBranch`、`IconName::GitGraph`）。`AllAssets` 是同一 crate 的
        // `#[folder = "assets"]` 全量嵌入（`native_assets.rs:9-11`、`lib.rs:36`）。
        .with_assets(assets::LitheAssets)
        .run(move |cx| {
            // S1_ASSETS：启动诊断，证明包装层被注册、并且两侧资源都能列出来。
            // 它同时是"回落没坏"的最早证据：`fallback_icons` 是 Lucide 一侧的数量，
            // `embedded` 是我们自己嵌进来的文件数，`ui_icons` 是其中的图标子集。
            //
            // ⚠️ 走 **stderr**（`eprintln!`），不走 stdout：stdout 在重定向到文件时是
            // **块缓冲**的，进程还在跑时日志文件里可能一行都看不到（本机实测：同一份
            // 二进制 + 同一个参数，日志里出现过 3 行、也出现过 0 行，纯粹取决于缓冲区
            // 有没有被填满/刷新）。stderr 是无缓冲的，验证脚本 grep 它才稳定。
            // 这也与 `crates/settings` 的 `S1_SETTINGS` 诊断同一口径
            // （`persistence.rs:13` 记的就是"统一走 stderr"）。
            #[cfg(debug_assertions)]
            {
                eprintln!(
                    "S1_ASSETS embedded={} ui_icons={} fallback_icons={}",
                    assets::LitheAssets::embedded_count(),
                    assets::LitheAssets::embedded_count_under("ui-icons/"),
                    assets::LitheAssets::fallback_count()
                );
                // 关键路径探针：证明 `AssetSource::load` 真的能取到我们清单里写的那个键
                // （`ui-icons/idea/expui/general/settings.svg`）。只是 print 一批路径名是
                // 不够的 —— 名字对但 `load` 语义错（例如回落把 Err 折叠成 Ok(None) 之后
                // 上层当成"空 SVG"）时，界面会**静默画不出东西**。
                for probe in [
                    "ui-icons/idea/expui/general/settings.svg",
                    "ui-icons/idea/expui/general/settings_dark.svg",
                    "icons/settings.svg",
                    // 文件类型图标主题（`icon-themes/idea/**`，任务 B 的接线点）：这一条是
                    // "查找层算出的路径确实能 `load` 到字节"的证据。没有它，主题图标取不到
                    // 字节时界面会**静默回落到 Lucide**（`FileIcon::render` 的 `else` 分支），
                    // 截图上看不出区别 —— 这正是 S1_ASSETS 存在的理由。
                    "icon-themes/idea/icons/expui/fileTypes/gitignore.svg",
                ] {
                    let hit = assets::probe_len(probe);
                    eprintln!(
                        "S1_ASSETS probe path={probe} bytes={}",
                        hit.map_or_else(|| "MISSING".to_string(), |n| n.to_string())
                    );
                }
            }
            // 语言：Lithe 的产品默认是简体中文（`windows/tauri/src/i18n/locale.ts` 的 zh-CN
            // 目录就是本仓库的文案真源）。gpui-kit **组件自己**的文案用它自带的 zh-CN，
            // 所以这里不需要 `rust_i18n::extend!(gpui_component)` —— 那个宏要求把
            // `gpui-component` 加成**直接依赖**（它展开成 `gpui_component::_rust_i18n_extend(..)`，
            // 并用 `stringify!($target)` 当 namespace，见 `rust-i18n-4.2.2/src/lib.rs:214-219`），
            // 而 gpui-kit 的编码规范要求"应用只依赖 gpui-kit 一个 crate"。两者冲突以规范为准；
            // 我们自己的文案走 `lithe_gpui_shared::{tr, tr_args}`。
            gpui_kit::component::set_locale(&locale);
            gpui_kit::init(cx);
            // `gpui_kit::init` 默认给**浅色**主题，而 Lithe 产品默认是深色，所以显式切一次。
            // 主题文件加载是异步的（`ThemeRegistry::watch_dir` 内部 `cx.spawn`），真正生效的主题
            // 由下面 `watch_lithe_themes` 的回调按设置里的值应用；这一句只是让"主题还没到位"
            // 的那一帧也是产品默认的深色，而不是浅色。
            Theme::change(ThemeMode::Dark, None, cx);
            // **滚动条常显**：gpui-kit 默认是 `ScrollbarMode::Scrolling`（滚动时才出现、
            // 停下就淡出，`gpui-base/src/scrollbar.rs:48-56`），而 IDE 的观感是常显细条 + 可拖。
            Theme::set_scrollbar_mode(gpui_kit::component::scroll::ScrollbarMode::Always, cx);

            // 设置：登记状态（含读文件结果与 `--theme` 覆盖）→ 主题目录装载/监听 → `Ctrl+,` 动作。
            // 顺序有要求：`watch_lithe_themes` 的回调要用到 `SettingsStore`，而主题的**应用**
            // 只在那个回调里发生（见 `lithe_gpui_settings::lib.rs` 的启动顺序表）。
            let store: gpui_kit::Entity<SettingsStore> = lithe_gpui_settings::init_store(
                cx,
                Init {
                    loaded,
                    theme_override,
                },
            );
            lithe_gpui_settings::watch_lithe_themes(cx);
            lithe_gpui_settings::install_actions(cx);
            let _ = store;

            // 外壳的启动期形态登记成 `App::Global`：换项目会**重建整个 `ShellWorkspace`**，
            // 而重建点在 `window.replace_root` 的闭包里 —— 那里捕获不到下面的局部值
            // （理由见 `lithe_gpui_workbench::ShellStartup` 的文档）。
            //
            // `--right-view` / `--left-view` 也一起登记：它们在 `ShellWorkspace::new` 里应用，
            // 于是**换根重建出来的外壳也会应用** —— 那正是"换根后右栏面板与 Git 变更视图
            // 指向新根"这条验收线要的证据（视图不打开就不会去扫 / 去读）。
            lithe_gpui_workbench::set_shell_startup(
                cx,
                compact_menu_bar,
                right_view_probe,
                left_view_probe,
            );
            // `Ctrl+O` = 「文件 → 打开文件夹」（真源 `file.open`，Q18）。
            // ⚠️ 在**这里**登记（App 级、一次），不在 `ShellWorkspace::new` 里：
            // `App::on_action` 是累加的，而 `new` 每次换根都会再跑一遍 ——
            // 换一次项目就多一个处理器，同一个键会被处理多次。
            lithe_gpui_workbench::install_open_project_action(cx);
            // B2 的六条「菜单项也有的键位入口」：`Ctrl+Shift+T` 重新打开已关闭标签页 /
            // `Ctrl+B` 侧栏 / `Ctrl+J` 终端 / `Ctrl+=` `Ctrl+-` `Ctrl+0` 缩放。
            //
            // ⚠️ 与上面那条**同一条理由**（也必须在这里、只在这里）：`App::on_action` 是累加的，
            // 而 `ShellWorkspace::new` 每换一次项目就会再跑一遍 —— 放那里会让同一个键
            // 被处理 N 次。键位字面量在 `lithe_gpui_workbench::menu_bar` 的常量里
            // （与菜单项显示的那颗键同一份），登记时逐条读它们。
            lithe_gpui_workbench::menu_bar::install_key_actions(cx);

            let bounds = startup_window_bounds(cx);

            cx.spawn(async move |cx| {
                let window_options = WindowOptions {
                    window_bounds: Some(bounds),
                    window_min_size: Some(size(px(1024.), px(680.))),
                    ..TitleBar::window_options()
                };

                cx.open_window(window_options, move |window, cx| {
                    let workspace =
                        cx.new(|cx| {
                            ShellWorkspace::new(
                                root,
                                compact_menu_bar,
                                branch_panel_probe,
                                window,
                                cx,
                            )
                        });
                    // 系统外观监听要在有窗口之后注册（`Context::observe_window_appearance`
                    // 收 `&mut Window`）；设置里的「跟随系统」才需要它。
                    if let Some(store) = lithe_gpui_settings::try_store(cx) {
                        store.update(cx, |store, cx| store.attach_window(window, cx));
                    }
                    // `--open-settings`：**首帧之后**再开对话框。这里用 `on_next_frame`
                    // （`gpui-pre-0.3.6/src/window.rs:2610`）而不是立即调用，因为
                    // `Root::new` 还没返回、窗口根还不是 `Root`，而浮层只能在事件回调或任务里
                    // 打开（`render` 阶段会 panic）。
                    if open_settings {
                        window.on_next_frame(|window, cx| {
                            lithe_gpui_settings::open_settings_dialog(window, cx);
                        });
                    }
                    // `--open-palette`：同上，只是换成命令面板。它调的是**按键最终落到的那
                    // 同一个函数**（`open_command_palette`，也就是 `OpenCommandPalette` 全局
                    // 处理器里面的那句）；被绕开的只有"操作系统把 `Ctrl+Shift+P` 送进窗口"这一段。
                    //
                    // ⚠️ 这里**不用** `window.dispatch_action(..)`：`Window::dispatch_action`
                    // 走的是 `dispatch_action_on_node(node_id, ..)`（`window.rs:2442-2454`），
                    // 只派发给**焦点节点及其祖先**上的处理器，**不触发** `App::on_action` 注册的
                    // 全局监听器 —— 本轮实测它一次都没进 `open_command_palette`（无
                    // `S1_COMMAND_PALETTE` 行）。全局 action 的官方路径是按键派发
                    // （`dispatch_key_event` → `dispatch_action_on_node` 后冒泡到全局），
                    // 或直接调用处理函数本身。
                    if open_palette {
                        window.on_next_frame(|window, cx| {
                            lithe_gpui_workbench::command_palette::open_command_palette(window, cx);
                        });
                    }
                    // `--menu-probe`：打开某个顶级菜单，必要时再执行一条菜单动作。
                    // 同样放在首帧之后（理由同上），并且**不经过命令行之外任何新通路**
                    // （见 [`run_menu_probe`] 的说明）。
                    if let Some((menu_id, action_id)) = menu_probe {
                        run_menu_probe(
                            menu_id,
                            action_id,
                            menu_probe_open_ms,
                            menu_probe_delay_ms,
                            window,
                            cx,
                        );
                    }
                    // `--theme-probe`：执行「视图 → 主题」子菜单的一项。同样在首帧之后。
                    if let Some(selector) = theme_probe {
                        window.on_next_frame(move |_window, cx| {
                            let bar = lithe_gpui_workbench::menu_bar::handle();
                            let _ = bar.update(cx, |bar, cx| {
                                if !bar.run_theme_by(&selector, cx) {
                                    eprintln!("--theme-probe：注册表里没有主题 {selector}");
                                }
                            });
                        });
                    }
                    // `--project-menu-probe`：打开标题栏的项目下拉。同样在首帧之后。
                    if project_menu_probe {
                        run_project_menu_probe(window);
                    }
                    // `--right-view` / `--left-view`：**不在这里**处理。
                    // 它们已经在 `set_shell_startup` 里登记进 `ShellStartup`，由
                    // `ShellWorkspace::new` 在构造期应用 —— 那样**换根重建出来的外壳也会应用**，
                    // 而这里的一次性 `on_next_frame` 只够得着第一个外壳（理由见那个 Global 的文档）。
                    //
                    // `--open-project-probe`：**首帧之后**走一遍「打开文件夹」。
                    //
                    // ⚠️ 必须在 `Root` 就位前后都行（这里在 `Root::new` **之前**）：它只是
                    // "请求打开"，真正的对话框 / 换根发生在 `request_open_project` 里 ——
                    // 而那一段要 `window.open_dialog`，所以它会经由 `on_next_frame` 再排一帧，
                    // 那时 `Root` 已经装好了。
                    if let Some(target) = open_project_probe {
                        let workspace = workspace.clone();
                        let explicit = open_project_destination.clone();
                        window.on_next_frame(move |window, cx| {
                            let explicit = explicit.as_deref().map(|raw| match raw {
                                "new-window" => {
                                    lithe_gpui_workbench::OpenDestination::NewWindow
                                }
                                _ => lithe_gpui_workbench::OpenDestination::ThisWindow,
                            });
                            if open_project_delay_ms == 0 {
                                workspace.update(cx, |this, cx| {
                                    this.open_project_probe(
                                        target.clone(),
                                        explicit,
                                        open_project_remember,
                                        window,
                                        cx,
                                    )
                                });
                                return;
                            }
                            // 延时档：与 `--menu-probe-delay` 同一实现（**后台执行器**上的
                            // timer；不能在前台线程 `block_on` 一个 timer —— 前台被占住时
                            // 定时器永远不触发，任务会静默死掉）。
                            //
                            // ⚠️ 这里只能用 `WeakEntity` + `App::active_window()` 拿回
                            // `&mut Window`：`cx.update(..)` 给的是 `&mut App`，
                            // 而换项目那条链要窗口（弹对话框 / 换根）。与
                            // `install_open_project_action` 的取窗口方式同一条路。
                            println!("S1_OPEN_PROJECT_PROBE delay_ms={open_project_delay_ms}");
                            let shell = workspace.downgrade();
                            let executor = cx.background_executor().clone();
                            cx.spawn(async move |cx| {
                                executor
                                    .timer(std::time::Duration::from_millis(
                                        open_project_delay_ms,
                                    ))
                                    .await;
                                cx.update(move |cx| {
                                    let Some(handle) = cx.active_window() else {
                                        eprintln!(
                                            "S1_OPEN_PROJECT_PROBE state=no_window"
                                        );
                                        return;
                                    };
                                    let _ = handle.update(cx, |_, window, cx| {
                                        let _ = shell.update(cx, |this, cx| {
                                            this.open_project_probe(
                                                target.clone(),
                                                explicit,
                                                open_project_remember,
                                                window,
                                                cx,
                                            )
                                        });
                                    });
                                });
                            })
                            .detach();
                        });
                    }
                    // `Root` 必须是窗口的第一层：它负责对话框、浮层与通知。
                    let root_entity = cx.new(|cx| Root::new(workspace, window, cx));

                    // `--palette-keys`：**在 `Root` 装好之后**、首帧之后再开始派发按键。
                    //
                    // ⚠️ 两个"不能更早"都有实测理由：
                    // 1. `dispatch_keystroke` 会同步走一次 `Window::draw`（`window.rs:5810-5813`），
                    //    而 `draw` 里 `self.root.as_ref().unwrap()`（`window.rs:3540`）**要求窗口
                    //    根已就位** —— 本轮第一次尝试就是在这里 panic 的（`None` unwrap）；
                    // 2. `on_next_frame`（`window.rs:2610`）保证不在 render 阶段改窗口状态。
                    if !palette_keys.is_empty() {
                        println!("S1_KEYS plan={}", palette_keys.len());
                        let Some(focus) = lithe_gpui_workbench::command_palette::shell_focus()
                        else {
                            eprintln!("S1_KEYS no_shell_focus（外壳还没登记焦点锚点）");
                            return root_entity;
                        };
                        let plan = KeyPlan {
                            focus,
                            rounds: palette_keys
                                .into_iter()
                                .map(|sequence| sequence.into_iter().collect())
                                .collect(),
                        };
                        root_entity.update(cx, |_root, cx| {
                            cx.spawn_in(window, async move |_root, cx| {
                                // 首帧之后再开始：与 `--open-settings` 同一条理由。
                                cx.background_executor()
                                    .timer(std::time::Duration::from_millis(400))
                                    .await;
                                let _ = cx.update(|window, _cx| {
                                    window.on_next_frame(move |window, cx| {
                                        dispatch_next_key(window, cx, plan);
                                    });
                                });
                            })
                            .detach();
                        });
                    }
                    root_entity
                })
                .expect("failed to open window");
            })
            .detach();
        });
}
