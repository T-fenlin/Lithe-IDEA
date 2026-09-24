//! 外壳入口：命令行参数、首窗口尺寸与**应用启动顺序**。
//!
//! 界面本身按"区域"拆在同目录的兄弟模块里，方便并行改动而互不冲突：
//!
//! - [`workspace`]：工作台骨架（标题栏 + 项目标签条 + 活动栏 + Dock 组装 + 状态栏）；
//! - [`panels`]：Dock 面板（`ShellPanel` / `PanelKind`）与各自的渲染；
//! - [`bottom_panel`]：底部 Git 工具窗（`提交记录`），自带头部与标签行；
//! - [`files`]：文件域逻辑（Core `workspace.snapshot` 调用与两层树构建）；
//! - [`stats`]：右侧项目统计表（`StatRow` / `StatsDelegate`）。
//!
//! 启动顺序固定为 `application().run` → `gpui_kit::init` → `cx.spawn` →
//! `open_window` → `Root::new`，与原单文件外壳完全一致。
//!
//! 运行：`cargo run --bin shell-probe -- <workspace-root>`

mod bottom_panel;
mod files;
mod panels;
mod stats;
mod workspace;

use std::path::PathBuf;

use gpui_kit::component::{Root, Theme, ThemeMode, TitleBar};
use gpui_kit::{
    App, AppContext as _, Bounds, Pixels, Size, WindowBounds, WindowOptions, point, px, size,
};

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
///   会得到一个比屏幕小的窗口，四周露出桌面 —— 这正是维护者看到的"没占满"；
/// - **启动即最大化**（`WindowBounds::Maximized` / `zoom_window()`）：`Maximized` 在这个 gpui
///   版本上只把 bounds 当还原尺寸、并不会真的最大化；而 Dodona 验证过的口径本来就是"94% 的
///   普通窗口"，不是最大化。
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
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            gpui_kit::init(cx);
            // `gpui_kit::init` 默认给**浅色**主题，而 Windows 产品默认是深色
            // （`docs/assets/screenshots/windows-zh-localization.png` 等真机截图全是深色），
            // 三端同步的目标界面对齐深色，所以这里显式切一次。
            Theme::change(ThemeMode::Dark, None, cx);
            // **滚动条常显**：gpui-kit 默认是 `ScrollbarMode::Scrolling`（滚动时才出现、
            // 停下就淡出，`gpui-base/src/scrollbar.rs:48-56`），而 IDE 的观感是常显细条 +
            // 可拖（macOS / IDEA 都是这样）。不常显时"内容多了看不出能滚、也抓不到滑块"，
            // 所以这里按真机改成 `Always`。
            Theme::set_scrollbar_mode(gpui_kit::component::scroll::ScrollbarMode::Always, cx);
            let bounds = startup_window_bounds(cx);

            cx.spawn(async move |cx| {
                let window_options = WindowOptions {
                    window_bounds: Some(bounds),
                    // 最小尺寸同样是 macOS 的项目窗口常量（`RootView.swift:351`）。
                    window_min_size: Some(size(px(980.), px(640.))),
                    ..TitleBar::window_options()
                };

                cx.open_window(window_options, move |window, cx| {
                    // 不做平台分支、不最大化：窗口口径与 Dodona 一致（可见区 94%、居中、普通窗口），
                    // 见 `startup_window_bounds` 的注释。窗口尺寸的正确性由"抓帧 + 与 Dodona 对账"验证，
                    // 不靠 Win32 的 `IsZoomed`/`GetWindowRect` 判断（那两者在 125% 缩放下会被虚拟化）。
                    let workspace =
                        cx.new(|cx| workspace::ShellWorkspace::new(root, window, cx));
                    // `Root` 必须是窗口的第一层：它负责对话框、浮层与通知。
                    cx.new(|cx| Root::new(workspace, window, cx))
                })
                .expect("failed to open window");
            })
            .detach();
        });
}
