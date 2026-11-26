//! `--capture`: one PNG of a document, without showing a window.

#[cfg(feature = "capture")]
use std::path::PathBuf;
#[cfg(feature = "capture")]
use std::time::Duration;

#[cfg(feature = "capture")]
const USAGE: &str = "green --capture out.png [--theme light|dark] [--width N] [--height N] [--wait-ms N] file.md";

/// Runs a capture when the command line asks for one; `None` for an ordinary start.
pub fn command_line() -> Option<i32> {
    if !std::env::args().skip(1).any(|arg| arg == "--capture") {
        return None;
    }
    #[cfg(not(feature = "capture"))]
    {
        // A build without the feature still says why it cannot capture.
        eprintln!("green --capture: this build has no capture; build it with --features capture");
        Some(2)
    }
    #[cfg(feature = "capture")]
    {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let request = match parse(&args) {
            Ok(request) => request,
            Err(said) => {
                eprintln!("green --capture: {said}\n{USAGE}");
                return Some(2);
            }
        };
        Some(execute(request))
    }
}

#[cfg(feature = "capture")]
/// What a capture run needs.
struct Request {
    output: PathBuf,
    file: PathBuf,
    theme: String,
    width: f32,
    height: f32,
    settle: Duration,
}

#[cfg(feature = "capture")]
fn parse(args: &[String]) -> Result<Request, String> {
    let mut output = None;
    let mut file = None;
    let mut theme = None;
    let mut width = 1400.0;
    let mut height = 900.0;
    let mut settle = Duration::from_millis(15000);
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| -> Result<String, String> {
            args.next()
                .cloned()
                .ok_or_else(|| format!("{name} has no value"))
        };
        match arg.as_str() {
            "--capture" => output = Some(PathBuf::from(value("--capture")?)),
            "--theme" => {
                let asked = value("--theme")?;
                match asked.as_str() {
                    "light" | "dark" => theme = Some(asked),
                    // There is no desktop to ask for the system's.
                    other => return Err(format!("--theme {other} is not light or dark")),
                }
            }
            "--width" => width = number(&value("--width")?, "--width")?,
            "--height" => height = number(&value("--height")?, "--height")?,
            "--wait-ms" => {
                settle = Duration::from_millis(number(&value("--wait-ms")?, "--wait-ms")? as u64)
            }
            other if other.starts_with("--") => return Err(format!("{other} is not a capture argument")),
            other => {
                if file.replace(PathBuf::from(other)).is_some() {
                    return Err("a capture takes one file".into());
                }
            }
        }
    }
    Ok(Request {
        output: output.ok_or("--capture needs a path")?,
        file: file.ok_or("a capture takes one file")?,
        theme: theme.unwrap_or_else(|| "light".into()),
        width,
        height,
        settle,
    })
}

#[cfg(feature = "capture")]
fn number(value: &str, name: &str) -> Result<f32, String> {
    let number: f32 = value
        .parse()
        .map_err(|_| format!("{name} {value} is not a number"))?;
    if !(number >= 1.0) {
        return Err(format!("{name} {value} is too small"));
    }
    Ok(number)
}

#[cfg(feature = "capture")]
fn execute(request: Request) -> i32 {
    let file = match std::fs::canonicalize(&request.file) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("green --capture: {} does not read: {error}", request.file.display());
            return 2;
        }
    };
    let data = std::env::temp_dir().join(format!("green-capture-{}", std::process::id()));
    // Before anything reads a setting: its own folder, its own theme.
    unsafe {
        std::env::set_var("ELY_DATA_ROOT", &data);
        std::env::set_var("ELY_APPEARANCE", &request.theme);
    }
    if let Err(error) = std::fs::create_dir_all(&data) {
        eprintln!("green --capture: a folder of its own is not there: {error}");
        return 5;
    }
    let shot = ely_ui::capture::Shot {
        output: request.output,
        size: gpui::size(gpui::px(request.width), gpui::px(request.height)),
        timeout: request.settle,
    };
    // The root hands back its desk, so the capture can tell when the document has drawn.
    let held: std::rc::Rc<std::cell::RefCell<Option<gpui::Entity<crate::desk::Desk>>>> =
        std::rc::Rc::new(std::cell::RefCell::new(None));
    let put = held.clone();
    let root = move |app: ely_platform::AppId, window: &mut gpui::Window, cx: &mut gpui::App| {
        let data = match ely_platform::data_dir(app) {
            Ok(data) => data,
            Err(error) => {
                eprintln!("green --capture: a folder of its own is not there: {error:#}");
                std::process::exit(5);
            }
        };
        let (root, desk) = crate::desk::open_in(app, data, window, cx);
        *put.borrow_mut() = Some(desk);
        root
    };
    let ready = move |cx: &gpui::App| {
        held.borrow()
            .as_ref()
            .is_some_and(|desk| desk.read(cx).shown(cx))
    };
    let code = ely_ui::capture::run(ely_platform::AppId::Mark, shot, vec![file], root, ready);
    let _ = std::fs::remove_dir_all(&data);
    code
}
