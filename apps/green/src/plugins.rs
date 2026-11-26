//! External-process plugins: the folder read at startup, and the files they
//! read (design §3). A plugin's blocks open as a derived document; any failure
//! opens the file as text and says why, and never stops the app.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use ely_gpui_component::primitives::Severity;
use ely_office::{Opening as _, Tabs as _, files};
use gpui::{App, Context, Global};
use green_plugins::{Host, View};

use crate::desk::Desk;

/// The plugins this window loaded; the view cache lives with them.
struct Plugins(Arc<Host>);

impl Global for Plugins {}

/// Reads `data/plugins`; a bad manifest is a log line, never a startup failure.
pub(crate) fn start(data: &Path, cx: &mut App) {
    let host = Host::load(data.join("plugins"));
    for problem in host.problems() {
        log::warn!(
            "mark: plugin {} is skipped: {}",
            problem.dir.display(),
            problem.why
        );
    }
    log::info!(
        "mark: {} plugin(s) from {}",
        host.plugins().len(),
        host.dir().display()
    );
    cx.set_global(Plugins(Arc::new(host)));
}

/// Takes `path` when a plugin reads it; `false` leaves it to the app.
pub(crate) fn intercept(path: &Path, cx: &mut Context<Desk>) -> bool {
    // Markdown is the app's own; a plugin never takes it.
    let ours = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            <Desk as ely_office::Host>::KIND
                .kept
                .iter()
                .any(|kept| kept.eq_ignore_ascii_case(extension))
        });
    if ours {
        return false;
    }
    let Some(host) = cx
        .try_global::<Plugins>()
        .map(|plugins| Arc::clone(&plugins.0))
    else {
        return false;
    };
    if !host.serves(path) {
        return false;
    }
    let asked = path.to_path_buf();
    log::info!("mark: {} is a plugin's", asked.display());
    cx.spawn(async move |desk, cx| {
        let reading = asked.clone();
        let opened = cx
            .background_executor()
            .spawn(async move { host.open(&reading) })
            .await;
        desk.update(cx, |desk, cx| desk.plugin_opened(asked, opened, cx))
            .ok();
    })
    .detach();
    true
}

impl Desk {
    /// A plugin's answer: its blocks as a document, or the file as text.
    fn plugin_opened(&mut self, path: PathBuf, view: View, cx: &mut Context<Self>) {
        let name = files::file_name(&path);
        match view {
            View::Blocks {
                plugin,
                blocks,
                cached,
                ..
            } => {
                let markdown = green_plugins::to_markdown(&blocks);
                let cached = if cached { ", cached" } else { "" };
                log::info!(
                    "mark: {name} opens through plugin {plugin}: {} block(s){cached}",
                    blocks.len()
                );
                self.import_text(&name, Arc::new(markdown.into_bytes()), (), cx);
            }
            View::Plain { reason } => {
                log::warn!("mark: {name} shows as text: {reason}");
                let said = ely_ui::i18n::text(
                    cx,
                    "office.error.open",
                    &[("name", &name), ("error", &reason.to_string())],
                );
                self.toast(said, Severity::Warning, cx);
                self.open_file(path, cx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path, process::Command, time::Duration};

    use gpui::{Entity, TestAppContext, VisualTestContext};

    use super::Desk;
    use crate::desk::tests::{open, start};

    /// `python` or `python3`, whichever runs here; None skips a test.
    fn python() -> Option<String> {
        ["python", "python3"].into_iter().find_map(|name| {
            let runs = Command::new(name)
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|status| status.success())
                .unwrap_or(false);
            runs.then(|| name.to_string())
        })
    }

    /// A plugin folder `id` under `data/plugins`.
    fn install(data: &Path, id: &str, manifest: &str, script: Option<&Path>) -> std::path::PathBuf {
        let folder = data.join("plugins").join(id);
        fs::create_dir_all(&folder).expect("a plugin folder");
        fs::write(folder.join("manifest.json"), manifest).expect("a manifest");
        if let Some(script) = script {
            fs::copy(script, folder.join("run.py")).expect("a script");
        }
        folder
    }

    fn manifest(python: &str, id: &str, extension: &str, timeout_ms: u64) -> String {
        format!(
            r#"{{"id": "{id}", "version": "0.1.0", "apiVersion": 1,
                "extensions": ["{extension}"], "command": ["{python}", "run.py"],
                "timeoutMs": {timeout_ms}}}"#
        )
    }

    /// The first tab's text, once a load lands.
    fn text(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> String {
        for _ in 0..300 {
            cx.run_until_parked();
            let found = desk.read_with(cx, |desk, _| {
                desk.core
                    .tabs
                    .first()
                    .map(|tab| tab.doc.lock().expect("the text").current.clone())
            });
            if let Some(text) = found {
                return text;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("no tab opened");
    }

    fn toasts(desk: &Entity<Desk>, cx: &mut VisualTestContext) -> Vec<String> {
        desk.read_with(cx, |desk, cx| {
            let root = desk.core.root.upgrade().expect("the window");
            root.read(cx)
                .toasts(cx)
                .into_iter()
                .map(|(said, _)| said.to_string())
                .collect()
        })
    }

    #[gpui::test]
    fn a_csv_opens_as_a_table(cx: &mut TestAppContext) {
        let Some(python) = python() else {
            eprintln!("skipping: no python on PATH");
            return;
        };
        let data = tempfile::tempdir().expect("data");
        let script =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/csv/csv_plugin.py");
        install(
            data.path(),
            "csv",
            &manifest(&python, "csv", ".csv", 5_000),
            Some(&script),
        );
        let files = tempfile::tempdir().expect("files");
        let csv = files.path().join("data.csv");
        fs::write(&csv, "name,qty\napple,3\npear,5\n").expect("a csv");

        let (desk, cx) = start(cx, data.path());
        open(&desk, &csv, cx);
        assert_eq!(
            text(&desk, cx),
            "| name | qty |\n| --- | --- |\n| apple | 3 |\n| pear | 5 |\n"
        );
        assert_eq!(
            desk.read_with(cx, |desk, _| desk.core.tabs[0].name.clone()),
            "data.csv",
            "the tab keeps the file's name"
        );
        assert!(toasts(&desk, cx).is_empty());
    }

    #[gpui::test]
    fn a_plugin_that_times_out_leaves_the_text_and_says_so(cx: &mut TestAppContext) {
        let Some(python) = python() else {
            eprintln!("skipping: no python on PATH");
            return;
        };
        let data = tempfile::tempdir().expect("data");
        let script = data.path().join("slow.py");
        fs::write(&script, "import time\ntime.sleep(30)\n").expect("a script");
        install(
            data.path(),
            "slow",
            &manifest(&python, "slow", ".slow", 200),
            Some(&script),
        );
        let files = tempfile::tempdir().expect("files");
        let slow = files.path().join("a.slow");
        fs::write(&slow, "a,b\n1,2\n").expect("a file");

        let (desk, cx) = start(cx, data.path());
        open(&desk, &slow, cx);
        assert_eq!(text(&desk, cx), "a,b\n1,2\n");
        assert!(
            toasts(&desk, cx)
                .iter()
                .any(|said| said.contains("did not finish within 200 ms")),
            "{:?}",
            toasts(&desk, cx)
        );
    }

    #[gpui::test]
    fn a_switched_off_plugin_leaves_the_text(cx: &mut TestAppContext) {
        let data = tempfile::tempdir().expect("data");
        install(
            data.path(),
            "off",
            r#"{"id": "off", "version": "1", "apiVersion": 1, "extensions": [".off"],
                "command": ["green-no-such-command"], "enabled": false}"#,
            None,
        );
        let files = tempfile::tempdir().expect("files");
        let off = files.path().join("a.off");
        fs::write(&off, "plain words\n").expect("a file");

        let (desk, cx) = start(cx, data.path());
        open(&desk, &off, cx);
        assert_eq!(text(&desk, cx), "plain words\n");
    }

    #[gpui::test]
    fn three_bad_manifests_do_not_stop_a_markdown_file(cx: &mut TestAppContext) {
        let data = tempfile::tempdir().expect("data");
        install(data.path(), "syntax", "{\"id\": ", None);
        install(
            data.path(),
            "version",
            r#"{"id": "v", "version": "1", "apiVersion": 99, "extensions": [".v"],
                "command": ["python", "x.py"]}"#,
            None,
        );
        install(
            data.path(),
            "command",
            r#"{"id": "c", "version": "1", "apiVersion": 1, "extensions": [".c"],
                "command": []}"#,
            None,
        );
        let files = tempfile::tempdir().expect("files");
        let note = files.path().join("note.md");
        fs::write(&note, "# Still here\n").expect("a file");

        let (desk, cx) = start(cx, data.path());
        open(&desk, &note, cx);
        assert_eq!(text(&desk, cx), "# Still here\n");
        assert_eq!(
            desk.read_with(cx, |desk, _| desk.core.tabs.len()),
            1,
            "one tab, none from the bad manifests"
        );
    }
}
