#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use gpui_kit::component::{Root, Theme, ThemeMode, TitleBar};
use gpui_kit::{
    App, AppContext as _, Bounds, Pixels, SharedString, Size, Window, WindowBounds, WindowOptions,
    point, px, size,
};
use lithe_gpui_settings::{Init, SettingsStore};
use lithe_gpui_workbench::ShellWorkspace;
use lithe_gpui_workbench::WelcomeView;
use lithe_gpui_workbench::right_tool_window::RightToolWindowView;

mod assets;

const MIN_WINDOW: Size<Pixels> = size(px(1024.), px(680.));

const WELCOME_WINDOW_SIZE: Size<Pixels> = size(px(800.), px(650.));

fn startup_window_bounds(cx: &App) -> WindowBounds {
    let fallback = size(px(1280.), px(800.));

    let Some(display) = cx.primary_display() else {
        return WindowBounds::Windowed(Bounds::new(point(px(60.), px(40.)), fallback));
    };

    let visible = display.visible_bounds();
    let wanted = size(visible.size.width * 0.94, visible.size.height * 0.94);
    let size = wanted.max(&MIN_WINDOW).min(&visible.size);
    centered_window_bounds(&visible, size)
}

fn welcome_window_bounds(cx: &App) -> WindowBounds {
    let Some(display) = cx.primary_display() else {
        return WindowBounds::Windowed(Bounds::new(point(px(60.), px(40.)), WELCOME_WINDOW_SIZE));
    };
    let visible = display.visible_bounds();
    centered_window_bounds(&visible, WELCOME_WINDOW_SIZE.min(&visible.size))
}

fn centered_window_bounds(visible: &Bounds<Pixels>, size: Size<Pixels>) -> WindowBounds {
    let origin = point(
        visible.origin.x + (visible.size.width - size.width) / 2.0,
        visible.origin.y + (visible.size.height - size.height) / 2.0,
    );
    WindowBounds::Windowed(Bounds::new(origin, size))
}

struct Options {
    root: Option<PathBuf>,
    theme_override: Option<SharedString>,
    locale_override: Option<String>,
    open_settings: bool,
    open_palette: bool,
    palette_keys: Vec<Vec<gpui_kit::Keystroke>>,
    compact_menu_bar: bool,
    menu_probe: Option<(String, Option<String>)>,
    menu_probe_delay_ms: u64,
    menu_probe_open_ms: u64,
    theme_probe: Option<String>,
    project_menu_probe: bool,
    branch_panel_probe: bool,
    run_probe: bool,
    run_probe_debug: bool,
    right_view: Option<String>,
    open_project_probe: Option<PathBuf>,
    open_project_destination: Option<String>,
    open_project_remember: bool,
    open_project_delay_ms: u64,
    left_view: Option<String>,
    appearance_revert: Option<String>,
    session_probe: Option<Vec<PathBuf>>,
    session_active: Option<usize>,
    session_hide_sidebar: bool,
    session_assert: bool,
}

fn parse_options() -> Result<Options, String> {
    const USAGE: &str = "用法：Lithe [<workspace-root>] [--theme <主题 id 或名>] [--locale <语言>] [--open-settings] [--open-palette] [--compact-menu-bar] [--palette-keys <按键串>] [--right-view <id>]\n\
         \x20 省略 <workspace-root> 时：打开「最近项目」里最近且仍然存在的那个（已失效的条目标为失效）；\n\
         \x20 一个都没有时打开**欢迎页**（800×650；挑一个文件夹或点一条最近项目即进入工作区）——\n\
         \x20 双击启动走的就是这一条，窗口一定会开出来。\n\
         \x20 --theme <id|名>      本次启动使用的主题（id 如 lithe-dark，名如 Lithe Dark；覆盖设置文件；验证/诊断用）\n\
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
         \x20                     「点右活动栏那一项」相同的状态迁移）；<id> 取 notifications /\n\
         \x20                     maven / spring，或 extensions（打开中央扩展页），\n\
         \x20                     未知 id 报一行错且不改启动状态\n\
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
         \x20 --appearance-revert <键名>\n\
         \x20                     启动后执行一次「改回我的全局外观」（验证/诊断用；键名取外观键的\n\
         \x20                     JSON 名，例如 fontSize / theme；走的是与设置页那个按钮相同的调用）\n\
         \x20 --menu-probe-delay <毫秒>  `--menu-probe` 打开菜单后等多久才执行动作（默认 2500）\n\
         \x20 --session-probe <文件>[,...]\n\
         \x20                     启动后打开这些文件（逗号分隔，验证/诊断用；走的是与资源管理器里\n\
         \x20                     点文件相同的 EditorPane::open），随后立刻写一次会话并可用\n\
         \x20 --session-active <下标>  --session-probe 打开的文件里把第 N 个设为当前文件（0 起）\n\
         \x20 --session-hide-sidebar  --session-probe 之后收起左侧栏\n\
         \x20 --session-assert   首帧之后打印恢复出来的会话状态（验证/诊断用；恢复本身发生在\n\
         \x20                     构造期，这一行是\"三个文件回来了、当前文件是第二个\"的判据）\n\
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
    let mut run_probe = false;
    let mut run_probe_debug = false;
    let mut right_view: Option<String> = None;
    let mut open_project_probe: Option<PathBuf> = None;
    let mut open_project_destination: Option<String> = None;
    let mut open_project_remember = false;
    let mut open_project_delay_ms: u64 = 0;
    let mut left_view: Option<String> = None;
    let mut appearance_revert: Option<String> = None;
    let mut session_probe: Option<Vec<PathBuf>> = None;
    let mut session_active: Option<usize> = None;
    let mut session_hide_sidebar = false;
    let mut session_assert = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--theme" => theme_override = Some(args.next().ok_or("--theme 缺少值")?.into()),
            "--locale" => locale_override = Some(args.next().ok_or("--locale 缺少值")?),
            "--open-settings" => open_settings = true,
            "--open-palette" => open_palette = true,
            "--compact-menu-bar" => compact_menu_bar = true,
            "--menu-probe" => {
                let menu = args.next().ok_or("--menu-probe 缺少顶级菜单 id")?;
                let next = args.next();
                let action = match next {
                    Some(ref candidate) if candidate.starts_with("lithe.") => {
                        Some(candidate.clone())
                    }
                    Some(candidate) => {
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
                menu_probe_delay_ms = raw.parse::<u64>().map_err(|error| {
                    format!("--menu-probe-delay 的 {raw:?} 不是毫秒数：{error}")
                })?;
            }
            "--menu-probe-open-ms" => {
                let raw = args.next().ok_or("--menu-probe-open-ms 缺少毫秒数")?;
                menu_probe_open_ms = raw.parse::<u64>().map_err(|error| {
                    format!("--menu-probe-open-ms 的 {raw:?} 不是毫秒数：{error}")
                })?;
            }
            "--theme-probe" => {
                theme_probe = Some(args.next().ok_or("--theme-probe 缺少下标或主题名")?);
            }
            "--project-menu-probe" => project_menu_probe = true,
            "--branch-panel-probe" => branch_panel_probe = true,
            "--run-probe" => run_probe = true,
            "--run-probe-debug" => {
                run_probe = true;
                run_probe_debug = true;
            }
            "--session-probe" => {
                let raw = args.next().ok_or("--session-probe 缺少文件列表")?;
                let paths: Vec<PathBuf> = raw
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(PathBuf::from)
                    .collect();
                if paths.is_empty() {
                    return Err("--session-probe 的文件列表是空的".to_string());
                }
                session_probe = Some(paths);
            }
            "--session-active" => {
                let raw = args.next().ok_or("--session-active 缺少下标")?;
                session_active =
                    Some(raw.parse::<usize>().map_err(|error| {
                        format!("--session-active 的 {raw:?} 不是下标：{error}")
                    })?);
            }
            "--session-hide-sidebar" => session_hide_sidebar = true,
            "--session-assert" => session_assert = true,
            "--right-view" => {
                right_view = Some(args.next().ok_or("--right-view 缺少视图 id")?);
            }
            "--open-project-probe" => {
                open_project_probe = Some(PathBuf::from(
                    args.next().ok_or("--open-project-probe 缺少目录")?,
                ));
            }
            "--open-project-destination" => {
                let raw = args.next().ok_or("--open-project-destination 缺少取值")?;
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
            "--appearance-revert" => {
                let raw = args.next().ok_or("--appearance-revert 缺少键名")?;
                if lithe_gpui_settings::AppearanceKey::from_json_key(&raw).is_none() {
                    return Err(format!(
                        "--appearance-revert 的 {raw:?} 不是外观键（可用：theme / syncSystemTheme / autoThemeLight / autoThemeDark / uiFontSize / fontSize / fontFamily / monoFontFamily / terminalFontSize）"
                    ));
                }
                appearance_revert = Some(raw);
            }
            "--palette-keys" => {
                let raw = args.next().ok_or("--palette-keys 缺少值")?;
                let mut sequence = Vec::new();
                for token in raw.split(',') {
                    let token = token.trim();
                    if token.is_empty() {
                        continue;
                    }
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
        run_probe,
        run_probe_debug,
        right_view,
        open_project_probe,
        open_project_destination,
        open_project_remember,
        open_project_delay_ms,
        left_view,
        appearance_revert,
        session_probe,
        session_active,
        session_hide_sidebar,
        session_assert,
    })
}

struct KeyPlan {
    focus: gpui_kit::FocusHandle,
    rounds: std::collections::VecDeque<std::collections::VecDeque<gpui_kit::Keystroke>>,
}

fn dispatch_next_key(window: &mut Window, cx: &mut App, mut plan: KeyPlan) {
    if window.focused(cx).is_none() {
        window.focus(&plan.focus, cx);
    }

    let Some(keystroke) = plan.rounds.front_mut().and_then(|round| round.pop_front()) else {
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

fn run_menu_probe(
    menu_id: String,
    action_id: Option<String>,
    open_ms: u64,
    delay_ms: u64,
    window: &mut Window,
    cx: &mut App,
) {
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
    let bar = lithe_gpui_workbench::menu_bar::handle();

    if open_ms == 0 {
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

fn run_appearance_revert_probe(key: String, window: &mut Window, _cx: &mut App) {
    let Some(key) = lithe_gpui_settings::AppearanceKey::from_json_key(&key) else {
        eprintln!("--appearance-revert：未知外观键 {key}");
        return;
    };
    window.on_next_frame(move |_window, cx| {
        let Some(store) = lithe_gpui_settings::try_store(cx) else {
            eprintln!("--appearance-revert：没有设置状态（这个宿主里没有 SettingsStore）");
            return;
        };
        let keys: Vec<lithe_gpui_settings::AppearanceKey> = match key {
            lithe_gpui_settings::AppearanceKey::Theme
            | lithe_gpui_settings::AppearanceKey::SyncSystemTheme
            | lithe_gpui_settings::AppearanceKey::AutoThemeLight
            | lithe_gpui_settings::AppearanceKey::AutoThemeDark => vec![
                lithe_gpui_settings::AppearanceKey::Theme,
                lithe_gpui_settings::AppearanceKey::SyncSystemTheme,
                lithe_gpui_settings::AppearanceKey::AutoThemeLight,
                lithe_gpui_settings::AppearanceKey::AutoThemeDark,
            ],
            single => vec![single],
        };
        println!(
            "S1_APPEARANCE_PROBE revert key={} keys={}",
            key.as_str(),
            keys.iter()
                .map(|key| key.as_str())
                .collect::<Vec<_>>()
                .join(",")
        );
        store.update(cx, |store, cx| {
            store.revert_appearance_group_to_global(&keys, cx)
        });
    });
}

fn run_project_menu_probe(window: &mut Window) {
    let menu = lithe_gpui_workbench::project_menu::handle();
    window.on_next_frame(move |_window, cx| {
        let _ = menu.update(cx, |menu, cx| menu.open_by_probe(cx));
    });
}

const LAUNCH_TAG: &str = "S1_WORKSPACE_LAUNCH";

enum LaunchPlan {
    Workspace(PathBuf),
    Welcome(lithe_gpui_settings::LoadedRecentProjects),
}

fn choose_launch_root(explicit: Option<PathBuf>, recent_root: Option<String>) -> Option<PathBuf> {
    explicit.or_else(|| recent_root.map(PathBuf::from))
}

fn resolve_launch_plan(explicit: Option<PathBuf>) -> LaunchPlan {
    if explicit.is_some() {
        let root = choose_launch_root(explicit, None).expect("刚判过 explicit 是 Some");
        return LaunchPlan::Workspace(root);
    }

    let mut loaded = lithe_gpui_settings::load_recent_projects();
    let pick = loaded
        .projects
        .select_launch_root(|path| std::path::Path::new(path).is_dir());

    let missing = pick.missing.len();
    if missing > 0 {
        for path in &pick.missing {
            loaded.projects.set_missing(path, true);
        }
        match loaded.path.as_deref() {
            Some(file) => match lithe_gpui_settings::save_recent_projects(file, &loaded.projects) {
                Ok(bytes) => eprintln!(
                    "{LAUNCH_TAG} recent_marked_missing={missing} saved bytes={bytes} path={}",
                    file.display()
                ),
                Err(error) => eprintln!("{LAUNCH_TAG} recent_save_failed error={error}"),
            },
            None => eprintln!("{LAUNCH_TAG} recent_mark_skipped reason=no_path"),
        }
    }

    let candidates = loaded.projects.len();
    match choose_launch_root(None, pick.root) {
        Some(root) => {
            eprintln!(
                "{LAUNCH_TAG} source=recent root={} candidates={candidates} missing={missing}",
                root.display()
            );
            LaunchPlan::Workspace(root)
        }
        None => {
            eprintln!("{LAUNCH_TAG} source=welcome candidates={candidates} missing={missing}");
            LaunchPlan::Welcome(loaded)
        }
    }
}

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
        run_probe,
        run_probe_debug,
        right_view,
        open_project_probe,
        open_project_destination,
        open_project_remember,
        open_project_delay_ms,
        left_view,
        appearance_revert,
        session_probe,
        session_active,
        session_hide_sidebar,
        session_assert,
    } = match parse_options() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    let plan = resolve_launch_plan(root);

    let extensions_page_probe = right_view.as_deref() == Some("extensions");
    let right_view_probe = right_view
        .as_deref()
        .filter(|id| *id != "extensions")
        .and_then(|id| match RightToolWindowView::from_id(id) {
            Some(view) => Some(view),
            None => {
                eprintln!(
                    "--right-view：未知视图 {id}（可用取值：extensions（扩展页）/ notifications / maven / spring）"
                );
                None
            }
        });
    let left_view_probe = left_view.as_deref().and_then(|selector| {
        match lithe_gpui_workbench::left_activity_index(selector) {
            Some(index) => Some(index),
            None => {
                eprintln!("--left-view：未知视图 {selector}（可用取值：files / changes / search）");
                None
            }
        }
    });

    let loaded = lithe_gpui_settings::load();
    let locale = locale_override
        .clone()
        .unwrap_or_else(|| loaded.settings.gpui_locale().to_string());

    gpui_kit::application()
        .with_assets(assets::LitheAssets)
        .run(move |cx| {
            #[cfg(debug_assertions)]
            {
                eprintln!(
                    "S1_ASSETS embedded={} idea_icons={} fallback_icons={}",
                    assets::LitheAssets::embedded_count(),
                    assets::LitheAssets::embedded_count_under("icons/idea/"),
                    assets::LitheAssets::fallback_count()
                );
                for &probe in assets::PROBE_PATHS {
                    let hit = assets::probe_len(probe);
                    eprintln!(
                        "S1_ASSETS probe path={probe} bytes={}",
                        hit.map_or_else(|| "MISSING".to_string(), |n| n.to_string())
                    );
                }
            }
            gpui_kit::component::set_locale(&locale);
            gpui_kit::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);
            Theme::set_scrollbar_mode(gpui_kit::component::scroll::ScrollbarMode::Always, cx);
            // 组件库（`lithe-gpui`）读的是它自己的主题全局：外壳在这里装一次，
            // 之后宿主每次换主题都会把颜色重新映射过去（见 `component_theme`）。
            lithe_gpui_workbench::component_theme::install_component_theme(cx);

            let store: gpui_kit::Entity<SettingsStore> = lithe_gpui_settings::init_store(
                cx,
                Init {
                    loaded,
                    theme_override,
                },
            );
            let uninstalled = store.read(cx).uninstalled_extensions().to_vec();
            let report = lithe_gpui_settings::prepare_bundled_extensions(&uninstalled);
            println!("S1_EXTENSIONS prepared {}", report.summary());
            lithe_gpui_settings::register_fonts(cx);
            lithe_gpui_settings::watch_lithe_themes(cx);
            lithe_gpui_settings::install_actions(cx);
            lithe_gpui_workbench::install_actions(cx);
            let _ = store;

            lithe_gpui_workbench::set_shell_startup(
                cx,
                compact_menu_bar,
                right_view_probe,
                extensions_page_probe,
                left_view_probe,
            );
            lithe_gpui_workbench::install_open_project_action(cx);
            lithe_gpui_workbench::menu_bar::install_key_actions(cx);

            // 多窗口：窗口尺寸与选项归组合根，所以「给这个工程再开一个窗口」也由它提供。
            // 外壳只在 `OpenDestination::NewWindow`（真源 `ProjectOpenPlacement.newWindow`）时调它；
            // 每个窗口一份自己的 `ShellWorkspace`（自己的 Dock、树、编辑器标签与会话写入）。
            lithe_gpui_workbench::set_open_window_host(std::sync::Arc::new(
                move |root: &std::path::Path, _window: &mut Window, cx: &mut App| {
                    let options = WindowOptions {
                        window_bounds: Some(startup_window_bounds(cx)),
                        window_min_size: Some(MIN_WINDOW),
                        ..TitleBar::window_options()
                    };
                    let root = root.to_path_buf();
                    let opened = cx.open_window(options, move |window, cx| {
                        let workspace = cx.new(|cx| {
                            ShellWorkspace::new(
                                root,
                                compact_menu_bar,
                                branch_panel_probe,
                                run_probe,
                                run_probe_debug,
                                window,
                                cx,
                            )
                        });
                        if let Some(store) = lithe_gpui_settings::try_store(cx) {
                            store.update(cx, |store, cx| store.attach_window(window, cx));
                        }
                        workspace
                    });
                    match opened {
                        Ok(handle) => {
                            eprintln!("S1_OPEN_WINDOW state=opened handle={handle:?}");
                        }
                        Err(error) => eprintln!("S1_OPEN_WINDOW state=failed error={error}"),
                    }
                },
            ));

            let (bounds, min_size) = match &plan {
                LaunchPlan::Welcome(_) => (welcome_window_bounds(cx), WELCOME_WINDOW_SIZE),
                LaunchPlan::Workspace(_) => (startup_window_bounds(cx), MIN_WINDOW),
            };

            cx.spawn(async move |cx| {
                let window_options = WindowOptions {
                    window_bounds: Some(bounds),
                    window_min_size: Some(min_size),
                    ..TitleBar::window_options()
                };

                cx.open_window(window_options, move |window, cx| {
                    let workspace_root = match plan {
                        LaunchPlan::Welcome(loaded) => {
                            let content = cx.new(|cx| {
                                let welcome = cx.new(|cx| {
                                    WelcomeView::new(loaded.projects, loaded.path, window, cx)
                                });
                                Root::new(welcome, window, cx)
                            });
                            if let Some(store) = lithe_gpui_settings::try_store(cx) {
                                store.update(cx, |store, cx| store.attach_window(window, cx));
                            }
                            return content;
                        }
                        LaunchPlan::Workspace(root) => root,
                    };

                    let workspace = cx.new(|cx| {
                        ShellWorkspace::new(
                            workspace_root,
                            compact_menu_bar,
                            branch_panel_probe,
                            run_probe,
                            run_probe_debug,
                            window,
                            cx,
                        )
                    });
                    if let Some(store) = lithe_gpui_settings::try_store(cx) {
                        store.update(cx, |store, cx| store.attach_window(window, cx));
                    }
                    if open_settings {
                        window.on_next_frame(|window, cx| {
                            lithe_gpui_settings::open_settings_dialog(window, cx);
                        });
                    }
                    if open_palette {
                        window.on_next_frame(|window, cx| {
                            lithe_gpui_workbench::command_palette::open_command_palette(window, cx);
                        });
                    }
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
                    if project_menu_probe {
                        run_project_menu_probe(window);
                    }
                    if let Some(key) = appearance_revert.clone() {
                        run_appearance_revert_probe(key, window, cx);
                    }
                    if let Some(target) = open_project_probe {
                        let workspace = workspace.clone();
                        let explicit = open_project_destination.clone();
                        window.on_next_frame(move |window, cx| {
                            let explicit = explicit.as_deref().map(|raw| match raw {
                                "new-window" => lithe_gpui_workbench::OpenDestination::NewWindow,
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
                            println!("S1_OPEN_PROJECT_PROBE delay_ms={open_project_delay_ms}");
                            let shell = workspace.downgrade();
                            let executor = cx.background_executor().clone();
                            cx.spawn(async move |cx| {
                                executor
                                    .timer(std::time::Duration::from_millis(open_project_delay_ms))
                                    .await;
                                cx.update(move |cx| {
                                    let Some(handle) = cx.active_window() else {
                                        eprintln!("S1_OPEN_PROJECT_PROBE state=no_window");
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
                    if session_probe.is_some() || session_assert {
                        let _ = workspace.update(cx, |this, cx| {
                            if let Some(paths) = session_probe.clone() {
                                this.session_probe(
                                    &paths,
                                    session_active,
                                    session_hide_sidebar,
                                    window,
                                    cx,
                                );
                            }
                            if session_assert {
                                this.session_assert(cx);
                            }
                        });
                    }
                    let root_entity = cx.new(|cx| Root::new(workspace, window, cx));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_root_wins_over_the_recent_project() {
        let root = choose_launch_root(
            Some(PathBuf::from(r"D:\explicit")),
            Some(r"D:\recent".to_string()),
        );
        assert_eq!(root, Some(PathBuf::from(r"D:\explicit")));
    }

    #[test]
    fn recent_project_is_used_when_there_is_no_explicit_root() {
        let root = choose_launch_root(None, Some(r"D:\recent".to_string()));
        assert_eq!(root, Some(PathBuf::from(r"D:\recent")));
    }

    #[test]
    fn no_root_at_all_shows_the_welcome_page() {
        assert_eq!(choose_launch_root(None, None), None);
    }

    #[test]
    fn welcome_page_requires_both_candidates_missing() {
        let explicit = PathBuf::from(r"D:\explicit");
        let recent = r"D:\recent".to_string();
        let cases = [
            (None, None, None),
            (Some(explicit.clone()), None, Some(explicit.clone())),
            (None, Some(recent.clone()), Some(PathBuf::from(&recent))),
            (Some(explicit.clone()), Some(recent), Some(explicit)),
        ];
        for (explicit, recent, expected) in cases {
            assert_eq!(
                choose_launch_root(explicit, recent),
                expected,
                "启动根判定的真值表在这一格上不对"
            );
        }
    }
}
