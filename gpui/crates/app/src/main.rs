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
}

/// 解析 `<workspace-root> [--theme <名>] [--locale <tag>] [--open-settings] [--open-palette] [--palette-keys <串>]`。
fn parse_options() -> Result<Options, String> {
    const USAGE: &str = "用法：Lithe <workspace-root> [--theme <主题名>] [--locale <语言>] [--open-settings] [--open-palette] [--palette-keys <按键串>]\n\
         \x20 --theme <主题名>     本次启动使用的主题（覆盖设置文件；验证/诊断用）\n\
         \x20 --locale <语言>      本次启动使用的界面语言（覆盖设置文件；验证/诊断用）\n\
         \x20 --open-settings     启动后自动打开设置对话框（验证/诊断用）\n\
         \x20 --open-palette      启动后自动打开命令面板（验证/诊断用）\n\
         \x20 --palette-keys <串> 启动后按顺序派发一串按键，逗号分隔；可重复给多次 = 多串（验证/诊断用；\n\
         \x20                     例：\"ctrl-shift-p,n,down,enter,escape\"）";
    let mut args = std::env::args().skip(1);
    let mut root: Option<PathBuf> = None;
    let mut theme_override: Option<SharedString> = None;
    let mut locale_override: Option<String> = None;
    let mut open_settings = false;
    let mut open_palette = false;
    let mut palette_keys = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--theme" => theme_override = Some(args.next().ok_or("--theme 缺少值")?.into()),
            "--locale" => locale_override = Some(args.next().ok_or("--locale 缺少值")?),
            "--open-settings" => open_settings = true,
            "--open-palette" => open_palette = true,
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
        palette_keys,
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
        palette_keys,
    } = match parse_options() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    // 设置文件必须在 `set_locale` 之前读：界面语言的常规来源就是它
    // （`--locale` 只是本次启动的显式覆盖）。这一步是纯文件读取，不需要 `App`。
    let loaded = lithe_gpui_settings::load();
    let locale = locale_override
        .clone()
        .unwrap_or_else(|| loaded.settings.gpui_locale().to_string());

    gpui_kit::application()
        // 用**全量**资产源（1830 个 Lucide 字形），不是默认的 101 个字形子集：
        // `gpui_kit::assets::Assets` 由 `gpui-kit-assets-0.6.6/build.rs` 按
        // `default-icons.txt` 过滤生成（`native_assets.rs:5` 的 `include!(default_assets.rs)`），
        // 里面**没有任何 `git-*` 字形**；而 Windows 规格的源码管理 / 提交记录 / 分支全要 git 字形。
        // `AllAssets` 是同一 crate 的 `#[folder = "assets"]` 全量嵌入
        // （`native_assets.rs:9-11`、`lib.rs:36`），注册它之后 `gpui_kit::assets::IconName::GitBranch`
        // 之类的字形才能真的画出来。
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
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

            let bounds = startup_window_bounds(cx);

            cx.spawn(async move |cx| {
                let window_options = WindowOptions {
                    window_bounds: Some(bounds),
                    window_min_size: Some(size(px(1024.), px(680.))),
                    ..TitleBar::window_options()
                };

                cx.open_window(window_options, move |window, cx| {
                    let workspace = cx.new(|cx| ShellWorkspace::new(root, window, cx));
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
