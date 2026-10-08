mod calendar;
mod canvas;
mod chart;
mod dashboard;
mod files;
mod generative;
mod interaction;
mod maps;
mod media;
mod messaging;
mod misc;
mod onboarding;
mod palette;
mod platform;
mod project;
mod syntax;
mod tokens;
mod tooling;

#[cfg(all(test, feature = "test-support"))]
mod tests;

use std::time::Duration;

use gpui::{App, Global, SharedString, WindowAppearance};
use web_time::Instant;

use crate::motion;
pub use calendar::CalendarSizes;
pub use canvas::CanvasSizes;
pub use chart::ChartSizes;
pub use dashboard::DashboardSizes;
pub use files::FileSizes;
pub use generative::GenerativeSizes;
pub use interaction::InteractionSizes;
pub use maps::MapSizes;
pub use media::MediaSizes;
pub use messaging::MessagingSizes;
pub use misc::MiscSizes;
pub use onboarding::OnboardingSizes;
pub use palette::{HUE_NAMES, Mix, Palette, Syntax};
pub use platform::Platform;
pub use project::ProjectSizes;
pub use syntax::{SyntaxTheme, syntax_themes};
pub use tokens::{
    AvatarSize, ContainerSize, Density, Elevation, IconSize, Radius, Sizable, Size, TextSize,
};
pub use tooling::ToolingSizes;

const FRAME: Duration = Duration::from_millis(8);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Light,
    Dark,
}

impl From<WindowAppearance> for Mode {
    fn from(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Mode::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => Mode::Light,
        }
    }
}

/// Design tokens. One global per app.
pub struct Theme {
    mode: Mode,
    high_contrast: bool,
    color_blind_safe: bool,
    transition: u64,
    custom: [Option<Palette>; 2],
    pub colors: Palette,
    pub density: Density,
    pub radius_scale: f32,
    /// 用户缩放系数（85%–130%）。**它不再自己乘 token 值**——见 `apply_rem`。
    pub font_scale: f32,
    /// 基准字号（px）：**尺寸缩放的唯一锚点**。100% 缩放下的窗口 rem 大小也是它。
    pub font_size: f32,
    pub font_family: SharedString,
    /// 等宽字体（编辑器）字号，px。与 `font_size` 是**两个不同的量**（UI 基准 vs 编辑器字号），
    /// 不是同一个量的两个副本；它不参与 rem 锚点。宿主设置是它的生产者。
    pub mono_font_size: f32,
    pub mono_family: SharedString,
    pub reduced_motion: bool,
    pub platform: Platform,
}

impl Global for Theme {}

pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}

impl Theme {
    /// Installs the component library's theme global. A host calls this once at
    /// startup, before it renders any of the library's components; the palette
    /// can then follow the host's own with [`Theme::set_palette`].
    pub fn init(cx: &mut App) {
        cx.set_global(Theme {
            mode: Mode::Light,
            high_contrast: false,
            color_blind_safe: false,
            transition: 0,
            custom: [None, None],
            colors: Palette::light(false),
            density: Density::Standard,
            radius_scale: 1.0,
            font_scale: 1.0,
            font_size: 16.0,
            font_family: "Inter".into(),
            mono_font_size: 13.0,
            mono_family: "JetBrains Mono".into(),
            reduced_motion: false,
            platform: Platform::current(),
        });
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn high_contrast(&self) -> bool {
        self.high_contrast
    }

    pub fn color_blind_safe(&self) -> bool {
        self.color_blind_safe
    }

    pub fn is_dark(&self) -> bool {
        self.mode == Mode::Dark
    }

    /// Cross-fades the palette to `mode`.
    pub fn set_mode(mode: Mode, cx: &mut App) {
        log::info!("theme: mode -> {mode:?}");
        cx.global_mut::<Theme>().mode = mode;
        Self::fade_to_target(cx);
    }

    /// Sets `mode` at once, stopping any fade under way.
    pub fn set_mode_now(mode: Mode, cx: &mut App) {
        log::info!("theme: mode -> {mode:?}, at once");
        let theme = cx.global_mut::<Theme>();
        theme.mode = mode;
        theme.transition += 1;
        theme.colors = theme.target();
        cx.refresh_windows();
    }

    pub fn set_high_contrast(on: bool, cx: &mut App) {
        log::info!("theme: high contrast -> {on}");
        cx.global_mut::<Theme>().high_contrast = on;
        Self::fade_to_target(cx);
    }

    /// Chart hues that stay apart for color-blind eyes, in Ely's own palettes.
    pub fn set_color_blind_safe(on: bool, cx: &mut App) {
        log::info!("theme: color-blind safe charts -> {on}");
        cx.global_mut::<Theme>().color_blind_safe = on;
        Self::fade_to_target(cx);
    }

    /// The owner's palette for `mode`, as an edited or imported theme gives, or none to bring Ely's back. The colors fade to it when `mode` shows. High contrast and color-blind safe charts change Ely's own palettes alone.
    pub fn set_palette(mode: Mode, palette: Option<Palette>, cx: &mut App) {
        log::info!(
            "theme: {mode:?} palette -> {}",
            if palette.is_some() {
                "the owner's"
            } else {
                "Ely's"
            }
        );
        cx.global_mut::<Theme>().custom[mode as usize] = palette;
        Self::fade_to_target(cx);
    }

    /// The palette the shown mode fades to: the owner's, or Ely's.
    pub fn palette(&self) -> Palette {
        self.target()
    }

    /// Shows `mode` with `palette` at once: the host's own colors, or none for
    /// the library's. A host that has its own theme calls this whenever that
    /// theme changes, so the library's components share its colors.
    pub fn apply(mode: Mode, palette: Option<Palette>, cx: &mut App) {
        let theme = cx.global_mut::<Theme>();
        theme.mode = mode;
        theme.custom[mode as usize] = palette;
        theme.transition += 1;
        theme.colors = theme.target();
        cx.refresh_windows();
    }

    /// Edits non-color tokens, then repaints.
    pub fn update(cx: &mut App, edit: impl FnOnce(&mut Theme)) {
        edit(cx.global_mut::<Theme>());
        // 尺寸缩放的**单一杠杆**落点：主题是唯一真源，每个窗口的 rem 大小由它推出。
        // `Window::set_rem_size` 只是一次赋值（不标脏），所以这里统一重标是安全的，
        // 也不会与"每帧渲染"互相触发。
        for handle in cx.windows() {
            let _ = handle.update(cx, |_, window, cx| {
                let rem = cx.theme().base_rem() * cx.theme().font_scale;
                window.set_rem_size(rem);
            });
        }
        cx.refresh_windows();
    }

    /// UI 基准字号的像素形态。**只是 [`Theme::font_size`] 的换算**，不是第二份数据——
    /// 给"需要一个 `Pixels` 值"的调用点（如宿主的 `rem_px(base, x)`）用。
    pub fn font_size_px(&self) -> gpui::Pixels {
        gpui::px(self.font_size)
    }

    /// 等宽（编辑器）字号的像素形态；同 [`Theme::font_size_px`]，只是换算。
    pub fn mono_font_size_px(&self) -> gpui::Pixels {
        gpui::px(self.mono_font_size)
    }

    /// 把当前主题的基准字号（乘用户缩放）标到窗口的 rem 大小上。
    ///
    /// 这是全套尺寸唯一的缩放入口：token 只把设计值换算成 rem，不再各自乘缩放系数。
    pub fn apply_rem(&self, window: &mut gpui::Window) {
        window.set_rem_size(self.base_rem() * self.font_scale);
    }

    fn target(&self) -> Palette {
        if let Some(palette) = &self.custom[self.mode as usize] {
            return palette.clone();
        }
        let mut palette = match self.mode {
            Mode::Light => Palette::light(self.high_contrast),
            Mode::Dark => Palette::dark(self.high_contrast),
        };
        if self.color_blind_safe {
            palette.chart = palette::color_blind_chart(self.mode);
        }
        palette
    }

    fn fade_to_target(cx: &mut App) {
        let duration = motion::duration(motion::THEME, cx);
        let theme = cx.global_mut::<Theme>();
        theme.transition += 1;
        let (id, from, to) = (theme.transition, theme.colors.clone(), theme.target());
        let start = Instant::now();
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor().timer(FRAME).await;
                let t = (start.elapsed().as_secs_f32() / duration.as_secs_f32()).min(1.0);
                let running = cx.update(|cx| {
                    let theme = cx.global_mut::<Theme>();
                    if theme.transition != id {
                        return false;
                    }
                    theme.colors = from.mix(&to, motion::ease_in_out_cubic(t));
                    cx.refresh_windows();
                    t < 1.0
                });
                if !running {
                    return;
                }
            }
        })
        .detach();
    }
}
