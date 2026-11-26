//! `--capture`: one PNG of a document, rendered and read back without ever showing a window.

use std::{cell::Cell, path::PathBuf, rc::Rc, time::Duration};

use ely_gpui_component::Assets;
use ely_platform::{AppId, graphics};
use gpui::{
    App, AppContext as _, Bounds, Entity, Pixels, Render, Size, Window, WindowBounds, point, px,
};

/// Exit codes the command line promises.
pub const BACKEND: i32 = 3;
pub const TIMEOUT: i32 = 4;
pub const OUTPUT: i32 = 5;

/// What the command line asked for.
pub struct Shot {
    pub output: PathBuf,
    /// Logical pixels; the frame comes back at the platform's scale.
    pub size: Size<Pixels>,
    /// How long to wait for the picture to settle; past it the run fails rather than
    /// writing a frame that may still be moving.
    pub timeout: Duration,
}

/// One frame of `files`, written to `shot.output`; the exit code says how it went.
///
/// `ready` says whether the document has drawn itself yet: frames before that are the
/// startup scene or an empty shell, and two of those can be identical, so a picture is only
/// taken once the document is in.
///
/// Returns 0 when a settled, non-blank frame was written; 3 when this machine cannot render;
/// 4 when no frame settled in time; 5 when the picture was blank or not written.
pub fn run<V: Render>(
    app: AppId,
    shot: Shot,
    files: Vec<PathBuf>,
    root: impl FnOnce(AppId, &mut Window, &mut App) -> Entity<V> + 'static,
    ready: impl Fn(&App) -> bool + 'static,
) -> i32 {
    if let Err(error) = graphics::check() {
        eprintln!("capture: this machine cannot render: {error:#}");
        return BACKEND;
    }
    let ready: Rc<dyn Fn(&App) -> bool> = Rc::new(ready);
    let code = Rc::new(Cell::new(TIMEOUT));
    let application = gpui_platform::application().with_assets(Assets);
    {
        let code = code.clone();
        application.run(move |cx: &mut App| {
            if let Err(error) = install(app, cx) {
                eprintln!("capture: {error:#}");
                code.set(BACKEND);
                cx.quit();
                return;
            }
            // The window root reads these as the files given at launch.
            crate::opening::install_launch_files(cx, files);
            let mut options = crate::window_options(app, shot.size, cx);
            // Never shown: no taskbar entry, no activation, nothing for the user to see.
            options.show = false;
            options.focus = false;
            options.window_bounds = Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: shot.size,
            }));
            let opened = cx.open_window(options, move |window, cx| {
                log::info!("capture: the window is built");
                let root = root(app, window, cx);
                park(window);
                // As in a window the platform draws: the first frame has shown, so the
                // startup scene may leave for the document.
                ely_platform::startup::started();
                window.refresh();
                root
            });
            let handle = match opened {
                Ok(handle) => gpui::AnyWindowHandle::from(handle),
                Err(error) => {
                    eprintln!("capture: the window did not open: {error:#}");
                    code.set(BACKEND);
                    cx.quit();
                    return;
                }
            };
            let timeout = shot.timeout;
            let output = shot.output.clone();
            let ready = ready.clone();
            cx.spawn(async move |cx| {
                let said = settled_frame(cx, handle, &output, timeout, ready.as_ref()).await;
                match said {
                    Ok(()) => code.set(0),
                    Err((exit, said)) => {
                        eprintln!("capture: {said}");
                        code.set(exit);
                    }
                }
                cx.update(|cx| cx.quit());
            })
            .detach();
        });
    }
    code.get()
}

/// Draws until the picture has stopped changing, then writes it.
///
/// The platform never draws a window it does not show, so every frame here is drawn by hand.
/// The picture is taken once the document is in and one frame has stood unchanged for
/// `STILL`: two frames alone can agree while the app is still working on work no frame
/// shows yet — Typst sets formulas in the background, and the frames before they land look
/// settled.
async fn settled_frame(
    cx: &mut gpui::AsyncApp,
    handle: gpui::AnyWindowHandle,
    output: &PathBuf,
    timeout: Duration,
    ready: &dyn Fn(&App) -> bool,
) -> Result<(), (i32, String)> {
    // Longer than the gap measured between "the document is drawn" and the formulas landing.
    const STILL: Duration = Duration::from_millis(2000);
    let started = std::time::Instant::now();
    let mut stable_since: Option<std::time::Instant> = None;
    let mut last: Option<image::RgbaImage> = None;
    let mut drawn = 0u32;
    loop {
        if started.elapsed() >= timeout {
            return Err((
                TIMEOUT,
                format!(
                    "no frame settled within {} ms ({drawn} frames drawn)",
                    timeout.as_millis()
                ),
            ));
        }
        let drew = cx.update(|cx| {
            cx.update_window(handle, |_, window, cx| {
                // The move lands a moment after it is asked for, so ask, then read the
                // pointer again: until it is read from the parked window it stays where it
                // was when the window was still under it, and a block wears a hover.
                if drawn < 3 {
                    park(window);
                    window.bounds_changed(cx);
                }
                window.draw(cx).clear(cx);
                let size = window.viewport_size();
                let pointer = window.mouse_position();

                (size, ready(cx), pointer, window.render_to_image())
            })
        });
        let frame = match drew {
            Ok((size, ready, pointer, Ok(image))) => {
                drawn += 1;
                log::info!(
                    "capture: frame {drawn} at {size:?}, the document shown {ready}, the pointer at {pointer:?}"
                );
                Some((ready, image))
            }
            Ok((_, _, _, Err(error))) => return Err((BACKEND, format!("no frame came back: {error:#}"))),
            Err(error) => return Err((TIMEOUT, format!("the window went before its frame: {error:#}"))),
        };
        if let Some((true, image)) = frame {
            let unchanged = last.as_ref().is_some_and(|before| before.as_raw() == image.as_raw());
            let since = *stable_since.get_or_insert_with(std::time::Instant::now);
            if unchanged && since.elapsed() >= STILL {
                return save(&image, output);
            }
            if !unchanged {
                // A new picture: the count starts again.
                stable_since = Some(std::time::Instant::now());
            }
            last = Some(image);
        } else {
            // Before the document shows, nothing is a picture of the document.
            last = None;
            stable_since = None;
        }
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;
    }
}

/// Puts the never-shown window off every display, so the pointer is never over it.
///
/// A block under the pointer draws a hover highlight, and that would make the same command
/// write different pixels depending on where the user left their mouse. The window is never
/// shown, so moving it costs nothing.
#[cfg(windows)]
fn park(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
    };
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return log::error!("capture: the window has no handle to park");
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return log::error!("capture: the window is not a Win32 window");
    };
    // SAFETY: a live window; the size and z-order stay as they are, and it stays hidden.
    let moved = unsafe {
        SetWindowPos(
            handle.hwnd.get() as _,
            std::ptr::null_mut(),
            -32000,
            -32000,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
    };
    if moved == 0 {
        log::error!("capture: the window was not parked off screen");
    }
}

#[cfg(not(windows))]
fn park(window: &Window) {
    let _ = window;
}

/// The settings, fonts and theme a window needs; a capture leaves out storage and updates.
fn install(app: AppId, cx: &mut App) -> anyhow::Result<()> {
    ely_platform::logging::init(app)?;
    cx.set_app_identity(&format!("Green.{}", app.name()), app.name());
    crate::fonts::install(cx)?;
    ely_gpui_component::init(cx)?;
    crate::settings::install(app, cx);
    crate::theme::install(cx);
    crate::i18n::install(cx);
    crate::commands::bind_keys(cx);
    crate::commands::bind_quit(cx);
    crate::shell::want_scene();
    Ok(())
}

/// Writes the frame, refusing to pass a blank one off as a document.
fn save(image: &image::RgbaImage, output: &PathBuf) -> Result<(), (i32, String)> {
    if blank(image) {
        let aside = output.with_extension("blank.png");
        let _ = image::save_buffer(&aside, image.as_raw(), image.width(), image.height(), image::ColorType::Rgba8);
        return Err((
            OUTPUT,
            format!("the frame is blank; it is at {} for a look", aside.display()),
        ));
    }
    image::save_buffer(output, image.as_raw(), image.width(), image.height(), image::ColorType::Rgba8)
        .map_err(|error| (OUTPUT, format!("{} was not written: {error}", output.display())))
}

/// A grid of pixels: a document holds many colours, a blank frame one or two.
fn blank(image: &image::RgbaImage) -> bool {
    use std::collections::HashSet;
    let step_x = (image.width() / 40).max(1);
    let step_y = (image.height() / 40).max(1);
    let mut seen = HashSet::new();
    let mut total = 0usize;
    let mut counts = std::collections::HashMap::new();
    for y in (0..image.height()).step_by(step_y as usize) {
        for x in (0..image.width()).step_by(step_x as usize) {
            let pixel = image.get_pixel(x, y);
            seen.insert(pixel.0);
            *counts.entry(pixel.0).or_insert(0usize) += 1;
            total += 1;
        }
    }
    let dominant = counts.values().copied().max().unwrap_or(0) as f64 / total.max(1) as f64;
    seen.len() <= 4 || dominant >= 0.995
}
