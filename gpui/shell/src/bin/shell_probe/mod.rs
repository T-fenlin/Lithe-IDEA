//! 外壳入口：命令行参数、首窗口尺寸与**应用启动顺序**。
//!
//! 界面按"区域"拆在同目录的兄弟模块里，一个区域一个文件，便于并行改动互不冲突：
//!
//! - [`workspace`]：工作台骨架（标题栏 + 项目标签条 + 活动栏 + Dock 组装 + 状态栏）。
//!
//! 启动顺序固定为 `application().run` → `gpui_kit::init` → `cx.spawn` →
//! `open_window` → `Root::new`，与历史实现完全一致（顺序错会静默失败或 panic）。
//!
//! 运行：`cargo run --bin shell-probe -- <workspace-root>`

mod bottom_panel;
mod editor;
mod explorer;
mod shell;
mod terminal;
mod workspace;

use std::path::PathBuf;

use gpui_kit::component::{Root, Theme, ThemeMode, TitleBar};
use gpui_kit::{
    App, AppContext as _, Bounds, Pixels, SharedString, Size, WindowBounds, WindowOptions, point,
    px, size,
};

/// 加载并监听 Lithe 主题目录，然后把 `Lithe Dark` 应用上去。
///
/// ⚠️ 主题文件的根是 **`ThemeSet`**（`{ name, author?, url?, themes: [ThemeConfig] }`），
/// **不是** `ThemeConfig`：官方文档里的 `{"colors": {…}}` 只是 colors **片段**，当整份文件写
/// 会得到空的 `themes` 数组、一个主题都载不进来
/// （`gpui-component-0.6.6/src/theme/schema.rs:22-34`；加载入口 `theme/registry.rs:98,152`）。
/// 更坑的是：**解析失败的文件会被整份静默忽略**（`registry.rs:252-258`），
/// 所以格式写错的表现只是"主题没生效"，不会有任何报错。
fn apply_lithe_theme(cx: &mut App) {
    // `gpui/themes/lithe-dark.json` 里 `themes[].name` 的值。主题按 name 去重，
    // 名字全局唯一，不能与内置的 `Default Light` / `Default Dark` 同名。
    let theme_name: SharedString = "Lithe Dark".into();

    // 用 `CARGO_MANIFEST_DIR`（= `gpui/shell`）拼路径，不依赖进程的工作目录：
    // `watch_dir` 收的是真实路径，而"从哪个目录启动 exe"是会变的。
    let themes_dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../themes"));
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
                gpui_kit::component::Theme::global_mut(cx).apply_config(&theme);
                println!("S1_THEME applied={theme_name}");
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

fn workspace_root() -> Result<PathBuf, String> {
    let mut args = std::env::args().skip(1);
    let first = args.next().ok_or("用法：shell-probe <workspace-root>")?;
    Ok(PathBuf::from(first))
}

/// bin 目标的入口点。`pub` 是必需的：crate 根文件用 `pub use shell_probe::main;`
/// 把它再导出到根（入口点必须解析自 crate 根），私有函数无法被再导出。
pub fn main() {
    let root = match workspace_root() {
        Ok(root) => root,
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
            // 我们自己的文案走本 crate 的 `rust_i18n::t!("lithe.…")`。
            gpui_kit::component::set_locale("zh-CN");
            gpui_kit::init(cx);
            // `gpui_kit::init` 默认给**浅色**主题，而 Lithe 产品默认是深色，所以显式切一次。
            Theme::change(ThemeMode::Dark, None, cx);
            // **滚动条常显**：gpui-kit 默认是 `ScrollbarMode::Scrolling`（滚动时才出现、
            // 停下就淡出，`gpui-base/src/scrollbar.rs:48-56`），而 IDE 的观感是常显细条 + 可拖。
            Theme::set_scrollbar_mode(gpui_kit::component::scroll::ScrollbarMode::Always, cx);
            // 主题系统：加载 + 监听 `gpui/themes/`，把 `Lithe Dark` 应用上去。
            apply_lithe_theme(cx);
            let bounds = startup_window_bounds(cx);

            cx.spawn(async move |cx| {
                let window_options = WindowOptions {
                    window_bounds: Some(bounds),
                    window_min_size: Some(size(px(1024.), px(680.))),
                    ..TitleBar::window_options()
                };

                cx.open_window(window_options, move |window, cx| {
                    let workspace = cx.new(|cx| workspace::ShellWorkspace::new(root, window, cx));
                    // `Root` 必须是窗口的第一层：它负责对话框、浮层与通知。
                    cx.new(|cx| Root::new(workspace, window, cx))
                })
                .expect("failed to open window");
            })
            .detach();
        });
}
