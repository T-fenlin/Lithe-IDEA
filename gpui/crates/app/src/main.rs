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

use std::path::PathBuf;

use gpui_kit::component::{Root, Theme, ThemeMode, TitleBar};
use gpui_kit::{
    App, AppContext as _, Bounds, Pixels, SharedString, Size, WindowBounds, WindowOptions, point,
    px, size,
};
use lithe_gpui_workbench::ShellWorkspace;

/// 加载并监听 Lithe 主题目录，然后把 `Lithe Dark` 应用上去。
///
/// ⚠️ 主题文件的根是 **`ThemeSet`**（`{ name, author?, url?, themes: [ThemeConfig] }`），
/// **不是** `ThemeConfig`：官方文档里的 `{"colors": {…}}` 只是 colors **片段**，当整份文件写
/// 会得到空的 `themes` 数组、一个主题都载不进来
/// （`gpui-component-0.6.6/src/theme/schema.rs:22-34`；加载入口 `theme/registry.rs:98,152`）。
/// 更坑的是：**解析失败的文件会被整份静默忽略**（`registry.rs:252-258`），
/// 所以格式写错的表现只是"主题没生效"，不会有任何报错。
/// `theme_name` 是 `themes/*.json` 里 `themes[].name` 的值（主题按 name 去重、名字全局唯一，
/// 不能与内置的 `Default Light` / `Default Dark` 同名）。
fn apply_lithe_theme(cx: &mut App, theme_name: SharedString) {
    // 用 `CARGO_MANIFEST_DIR`（= `gpui/crates/app`）拼路径，不依赖进程的工作目录：
    // `watch_dir` 收的是真实路径，而"从哪个目录启动 exe"是会变的。
    let themes_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../themes"));
    // 回调是 `move` 的（`watch_dir` 要求 `F: Fn(&mut App) + 'static`），所以路径要各留一份。
    let callback_dir = themes_dir.clone();

    let result = gpui_kit::component::ThemeRegistry::watch_dir(themes_dir.clone(), cx, move |cx| {
        // `watch_dir` 会先 `reload_themes` 再调这个回调，所以这里 `themes()` 已经填好了。
        let theme = gpui_kit::component::ThemeRegistry::global(cx)
            .themes()
            .get(&theme_name)
            .cloned();
        match theme {
            Some(theme) => {
                // ⚠️ **必须先切 `ThemeMode` 再 `apply_config`**：`apply_config` 只把配置写进
                // `light_theme` / `dark_theme` 两个槽里**按 `config.mode` 对应的那一个**
                // （`theme/schema.rs:1060-1065`），而实际渲染用的是**当前 `ThemeMode`** 对应的槽。
                // 所以不切 mode 的话，加载一个 `"mode": "light"` 的主题会**完全没有效果** ——
                // 这是"主题看起来没生效"的第二个原因（第一个是文件解析失败被整份静默忽略）。
                let mode = theme.mode;
                gpui_kit::component::Theme::change(mode, None, cx);
                gpui_kit::component::Theme::global_mut(cx).apply_config(&theme);
                println!("S1_THEME applied={theme_name} dark={}", mode.is_dark());
            }
            None => eprintln!(
                "S1_THEME missing name={theme_name} dir={}",
                callback_dir.display()
            ),
        }
    });

    if let Err(error) = result {
        eprintln!(
            "S1_THEME watch_failed dir={} error={error}",
            themes_dir.display()
        );
    }
}

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
/// ⚠️ `--theme` / `--locale` 是**设置界面做好之前的临时开关**：Lithe 的产品设置里本来就有
/// 「配色主题」与「界面语言」两项（Windows 侧对应 `settings.theme_choice` 与语言设置）。
/// 这两个开关的唯一目的是让"主题与语言能在一个进程里被复现地切换"，好把视觉证据跑出来；
/// 设置界面（阶段 6）做好后应由设置接管，届时删掉它们。
struct Options {
    /// 工作区根，传给 `workspace.snapshot` 与 `git.*`。
    root: PathBuf,
    /// 要应用的主题名（`themes/*.json` 里 `themes[].name`）。
    theme: SharedString,
    /// 界面语言。gpui-kit 组件自带 `en` / `zh-CN` / `zh-HK`。
    locale: String,
}

/// 解析 `<workspace-root> [--theme <名>] [--locale <tag>]`。
fn parse_options() -> Result<Options, String> {
    const USAGE: &str = "用法：Lithe <workspace-root> [--theme <主题名>] [--locale <语言>]";
    let mut args = std::env::args().skip(1);
    let mut root: Option<PathBuf> = None;
    let mut theme: Option<SharedString> = None;
    let mut locale: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--theme" => theme = Some(args.next().ok_or("--theme 缺少值")?.into()),
            "--locale" => locale = Some(args.next().ok_or("--locale 缺少值")?),
            "-h" | "--help" => return Err(USAGE.to_string()),
            other if root.is_none() && !other.starts_with("--") => {
                root = Some(PathBuf::from(other));
            }
            other => return Err(format!("未知参数 {other}\n\n{USAGE}")),
        }
    }

    Ok(Options {
        root: root.ok_or_else(|| USAGE.to_string())?,
        theme: theme.unwrap_or_else(|| "Lithe Dark".into()),
        locale: locale.unwrap_or_else(|| "zh-CN".to_string()),
    })
}

/// bin 目标的入口点。
fn main() {
    let Options {
        root,
        theme,
        locale,
    } = match parse_options() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

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
            Theme::change(ThemeMode::Dark, None, cx);
            // **滚动条常显**：gpui-kit 默认是 `ScrollbarMode::Scrolling`（滚动时才出现、
            // 停下就淡出，`gpui-base/src/scrollbar.rs:48-56`），而 IDE 的观感是常显细条 + 可拖。
            Theme::set_scrollbar_mode(gpui_kit::component::scroll::ScrollbarMode::Always, cx);
            // 主题系统：加载 + 监听 `gpui/themes/`，把 `--theme` 指定的主题应用上去
            // （默认 `Lithe Dark`）；主题文件里的 `mode` 会一并决定明/暗。
            apply_lithe_theme(cx, theme);
            let bounds = startup_window_bounds(cx);

            cx.spawn(async move |cx| {
                let window_options = WindowOptions {
                    window_bounds: Some(bounds),
                    window_min_size: Some(size(px(1024.), px(680.))),
                    ..TitleBar::window_options()
                };

                cx.open_window(window_options, move |window, cx| {
                    let workspace = cx.new(|cx| ShellWorkspace::new(root, window, cx));
                    // `Root` 必须是窗口的第一层：它负责对话框、浮层与通知。
                    cx.new(|cx| Root::new(workspace, window, cx))
                })
                .expect("failed to open window");
            })
            .detach();
        });
}
