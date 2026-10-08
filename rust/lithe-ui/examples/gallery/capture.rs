use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context as _, Result, bail};
use gpui::{App, AsyncApp, Window, WindowHandle, px};
use lithe_ui::theme::Theme;

use crate::{
    pages, script,
    shell::{Choice, Gallery},
};

const SETTLE: Duration = Duration::from_millis(700);
const SCROLL_SETTLE: Duration = Duration::from_millis(250);

/// Shoots one page, or all, in both modes, then quits; exits 1 on failure.
pub fn run(
    window: WindowHandle<Gallery>,
    dir: PathBuf,
    only: Option<usize>,
    scripted: bool,
    cx: &mut App,
) {
    Theme::update(cx, |theme| theme.reduced_motion = true);
    cx.spawn(async move |cx| {
        let outcome = shoot_all(window, &dir, only, scripted, cx).await;
        if let Err(error) = outcome.map(|()| cx.update(|cx| cx.quit())) {
            log::error!("capture failed: {error:#}");
            std::process::exit(1);
        }
    })
    .detach();
}

async fn shoot_all(
    window: WindowHandle<Gallery>,
    dir: &Path,
    only: Option<usize>,
    scripted: bool,
    cx: &mut AsyncApp,
) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let chosen = pages::ALL
        .iter()
        .enumerate()
        .filter(|(ix, _)| only.is_none_or(|only| only == *ix));
    let gallery = window.update(cx, |_, window, _| number(window))??;
    window.update(cx, |_, window, _| take_posted_moves(window))??;
    for (ix, page) in chosen {
        for (choice, name) in [(Choice::Light, "light"), (Choice::Dark, "dark")] {
            window.update(cx, |gallery, window, cx| {
                gallery.select(ix, cx);
                gallery.fresh(cx);
                gallery.choose(choice, window, cx);
            })?;
            script::park(window, cx)?;
            cx.background_executor().timer(SETTLE).await;
            let (viewport, max) = window.update(cx, |gallery, _, _| gallery.scroll_extent())?;
            if viewport <= px(0.0) {
                bail!("page {} has no viewport", page.slug);
            }
            let mut offset = px(0.0);
            for segment in 0.. {
                let suffix = if segment == 0 {
                    String::new()
                } else {
                    format!("-{segment}")
                };
                let path = dir.join(format!("{}-{name}{suffix}.png", page.slug));
                script::shown(window.into(), cx).await?;
                snapshot(gallery, &path)?;
                log::info!("capture: wrote {}", path.display());
                if offset >= max {
                    break;
                }
                offset = (offset + viewport).min(max);
                window.update(cx, |gallery, _, cx| gallery.scroll_to(offset, cx))?;
                cx.background_executor().timer(SCROLL_SETTLE).await;
            }
            if scripted && !page.script.is_empty() {
                window.update(cx, |gallery, _, cx| gallery.select(ix, cx))?;
                cx.background_executor().timer(SETTLE).await;
                let shot = |step: &str| dir.join(format!("{}-{step}-{name}.png", page.slug));
                script::play(window, page.script, shot, cx).await?;
            }
        }
    }
    Ok(())
}

/// A gpui window's AppKit window.
#[cfg(target_os = "macos")]
fn native(window: &Window) -> Result<cocoa::base::id> {
    use cocoa::base::id;
    use objc::{msg_send, sel, sel_impl};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window)
        .map_err(|error| anyhow::anyhow!("window has no native handle: {error:?}"))?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        bail!("window is not an AppKit window");
    };
    Ok(unsafe { msg_send![appkit.ns_view.as_ptr() as id, window] })
}

/// A gpui window's number, as AppKit and CoreGraphics know it.
#[cfg(target_os = "macos")]
pub fn number(window: &Window) -> Result<u32> {
    use objc::{msg_send, sel, sel_impl};

    let number: i64 = unsafe { msg_send![native(window)?, windowNumber] };
    u32::try_from(number).context("window number out of range")
}

/// Takes posted mouse moves; a tracking area takes real ones.
#[cfg(target_os = "macos")]
fn take_posted_moves(window: &Window) -> Result<()> {
    use cocoa::{appkit::NSWindow, base::YES};

    unsafe { native(window)?.setAcceptsMouseMovedEvents_(YES) };
    log::info!("capture: the window takes posted mouse moves");
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn take_posted_moves(_: &Window) -> Result<()> {
    bail!("capture needs macOS window APIs")
}

/// This process's frontmost window above the normal level: a popover or menu.
#[cfg(target_os = "macos")]
pub fn popup_number() -> Result<u32> {
    use core_foundation::{
        base::{CFType, TCFType},
        dictionary::CFDictionary,
        number::CFNumber,
        string::CFString,
    };
    use core_graphics::window;

    let read = |dict: &CFDictionary<CFString, CFType>, key: &'static str| {
        dict.find(CFString::from_static_string(key))
            .and_then(|value| value.downcast::<CFNumber>())
            .and_then(|value| value.to_i64())
    };
    let pid = i64::from(std::process::id());
    let list = window::copy_window_info(
        window::kCGWindowListOptionOnScreenOnly,
        window::kCGNullWindowID,
    )
    .context("window list unavailable")?;
    let id = list
        .iter()
        .map(|item| unsafe { CFDictionary::<CFString, CFType>::wrap_under_get_rule(*item as _) })
        .find(|dict| {
            read(dict, "kCGWindowOwnerPID") == Some(pid)
                && read(dict, "kCGWindowLayer").is_some_and(|layer| layer > 0)
        })
        .and_then(|dict| read(&dict, "kCGWindowNumber"))
        .context("no popup window of this process is on screen")?;
    u32::try_from(id).context("window number out of range")
}

#[cfg(not(target_os = "macos"))]
pub fn popup_number() -> Result<u32> {
    bail!("capture needs macOS window APIs")
}

#[cfg(not(target_os = "macos"))]
pub fn number(_: &Window) -> Result<u32> {
    bail!("capture needs macOS window APIs")
}

#[cfg(target_os = "macos")]
pub fn snapshot(number: u32, path: &Path) -> Result<()> {
    use core_graphics::{
        geometry::{CGPoint, CGRect, CGSize},
        window,
    };

    let image = window::create_image(
        CGRect::new(&CGPoint::new(0.0, 0.0), &CGSize::new(0.0, 0.0)),
        window::kCGWindowListOptionIncludingWindow,
        number,
        window::kCGWindowImageBoundsIgnoreFraming | window::kCGWindowImageBestResolution,
    )
    .context("window capture returned nothing")?;
    if image.bits_per_pixel() != 32 {
        bail!(
            "unexpected capture format: {} bits per pixel",
            image.bits_per_pixel()
        );
    }
    let (width, height, stride) = (image.width(), image.height(), image.bytes_per_row());
    let data = image.data();
    let bytes = data.bytes();
    let mut rgba = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for x in 0..width {
            let i = y * stride + x * 4;
            rgba.extend_from_slice(&[bytes[i + 2], bytes[i + 1], bytes[i], bytes[i + 3]]);
        }
    }
    image::save_buffer(
        path,
        &rgba,
        width as u32,
        height as u32,
        image::ColorType::Rgba8,
    )
    .with_context(|| format!("write {}", path.display()))
}

#[cfg(not(target_os = "macos"))]
pub fn snapshot(_: u32, _: &Path) -> Result<()> {
    bail!("capture needs macOS window APIs")
}
