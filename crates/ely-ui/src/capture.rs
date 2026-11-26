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
    /// How long the document draws before the picture is taken: Typst sets formulas in the
    /// background, so the first frame is often not the finished one.
    pub settle: Duration,
}

/// Opens `files` in a window that is never shown, renders it, writes `shot.output`.
///
/// Returns the exit code: 0 when a non-blank frame was written, 3 when this machine cannot
/// render, 4 when no frame came back in time, 5 when the picture was blank or not written.
pub fn run<V: Render>(
    app: AppId,
    shot: Shot,
    files: Vec<PathBuf>,
    root: impl FnOnce(AppId, &mut Window, &mut App) -> Entity<V> + 'static,
) -> i32 {
    if let Err(error) = graphics::check() {
        eprintln!("capture: this machine cannot render: {error:#}");
        return BACKEND;
    }
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
            let settle = shot.settle;
            let output = shot.output.clone();
            cx.spawn(async move |cx| {
                cx.background_executor().timer(settle).await;
                // The platform never draws a window that is not shown, so the frames are
                // drawn here: layout and paint, then the pixels read back.
                let mut frame = None;
                let mut said = Ok(());
                for _ in 0..6 {
                    let drew = cx.update(|cx| {
                        cx.update_window(handle, |_, window, cx| {
                            window.draw(cx).clear(cx);
                            let size = window.viewport_size();
                            (size, window.render_to_image())
                        })
                    });
                    match drew {
                        Ok((size, Ok(image))) => {
                            log::info!("capture: a frame at {size:?}");
                            frame = Some(image);
                        }
                        Ok((_, Err(error))) => {
                            said = Err((BACKEND, format!("no frame came back: {error:#}")));
                            break;
                        }
                        Err(error) => {
                            said = Err((TIMEOUT, format!("the window went before its frame: {error:#}")));
                            break;
                        }
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(250))
                        .await;
                }
                let said = match (said, frame) {
                    (Err(said), _) => Err(said),
                    (Ok(()), Some(image)) => save(&image, &output),
                    (Ok(()), None) => Err((TIMEOUT, "no frame was drawn".to_string())),
                };
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
