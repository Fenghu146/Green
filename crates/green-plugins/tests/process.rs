//! Real plugin processes: the shipped CSV plugin, and every way a run fails.

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

use green_plugins::{Block, Host, Reason, View};

/// `python` or `python3`, whichever this machine runs; None skips a test.
fn python() -> Option<String> {
    ["python", "python3"].into_iter().find_map(|name| {
        let runs = Command::new(name)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        runs.then(|| name.to_string())
    })
}

/// A plugins folder holding `id`, with `script` and a manifest built by `json`.
fn plugin(root: &Path, id: &str, script: &str, manifest: &str) -> PathBuf {
    let folder = root.join(id);
    std::fs::create_dir_all(&folder).expect("a plugin folder");
    std::fs::write(folder.join("run.py"), script).expect("a script");
    std::fs::write(folder.join("manifest.json"), manifest).expect("a manifest");
    folder
}

fn manifest(python: &str, id: &str, extension: &str, timeout_ms: u64) -> String {
    format!(
        r#"{{"id": "{id}", "version": "0.1.0", "apiVersion": 1,
            "extensions": ["{extension}"], "command": ["{python}", "run.py"],
            "timeoutMs": {timeout_ms}}}"#
    )
}

#[test]
fn the_shipped_csv_plugin_opens_a_csv_as_a_table() {
    let Some(python) = python() else {
        eprintln!("skipping: no python on PATH");
        return;
    };
    let root = tempfile::tempdir().expect("a temp dir");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/csv");
    let folder = root.path().join("csv");
    std::fs::create_dir_all(&folder).expect("a folder");
    std::fs::copy(source.join("csv_plugin.py"), folder.join("run.py")).expect("the script");
    std::fs::write(
        folder.join("manifest.json"),
        manifest(&python, "csv", ".csv", 5_000),
    )
    .expect("a manifest");
    let host = Host::load(root.path());

    let files = tempfile::tempdir().expect("a temp dir");
    let csv = files.path().join("data.csv");
    std::fs::write(&csv, "name,qty\napple,3\npear,5\n").expect("a csv");

    let view = host.open(&csv);
    assert_eq!(
        view,
        View::Blocks {
            plugin: "csv".into(),
            blocks: vec![Block::Table {
                aligns: vec![],
                head: vec!["name".into(), "qty".into()],
                rows: vec![
                    vec!["apple".into(), "3".into()],
                    vec!["pear".into(), "5".into()],
                ],
            }],
            meta: serde_json::json!({"rows": 2, "columns": 2}),
            cached: false,
        }
    );
    assert_eq!(
        view.markdown().expect("Markdown"),
        "| name | qty |\n| --- | --- |\n| apple | 3 |\n| pear | 5 |\n"
    );
}

#[test]
fn a_second_open_of_the_same_file_does_not_run_the_plugin() {
    let Some(python) = python() else {
        eprintln!("skipping: no python on PATH");
        return;
    };
    let root = tempfile::tempdir().expect("a temp dir");
    let script = r#"
import json, sys
request = json.load(sys.stdin)
with open("runs.txt", "a", encoding="utf-8") as log:
    log.write("ran\n")
print(json.dumps({"apiVersion": 1, "blocks": [{"kind": "paragraph", "text": "once"}]}))
"#;
    let folder = plugin(
        root.path(),
        "count",
        script,
        &manifest(&python, "count", ".count", 5_000),
    );
    let host = Host::load(root.path());
    let file = root.path().join("a.count");
    std::fs::write(&file, "x").expect("a file");

    assert!(!matches!(host.open(&file), View::Plain { .. }));
    let again = host.open(&file);
    assert!(matches!(again, View::Blocks { cached: true, .. }));
    let runs = std::fs::read_to_string(folder.join("runs.txt")).expect("the log");
    assert_eq!(runs.lines().count(), 1, "one run for two opens");

    // A new mtime is a new key.
    std::fs::write(&file, "xx").expect("a file");
    let fresh = host.open(&file);
    assert!(matches!(fresh, View::Blocks { cached: false, .. }));
    let runs = std::fs::read_to_string(folder.join("runs.txt")).expect("the log");
    assert_eq!(runs.lines().count(), 2);
}

#[test]
fn a_plugin_past_its_time_is_killed_and_the_host_lives() {
    let Some(python) = python() else {
        eprintln!("skipping: no python on PATH");
        return;
    };
    let root = tempfile::tempdir().expect("a temp dir");
    let script = r#"
import time
time.sleep(30)
open("finished.txt", "w", encoding="utf-8").write("yes")
"#;
    plugin(
        root.path(),
        "slow",
        script,
        &manifest(&python, "slow", ".slow", 300),
    );
    let host = Host::load(root.path());
    let file = root.path().join("a.slow");
    std::fs::write(&file, "x").expect("a file");

    let began = Instant::now();
    let view = host.open(&file);
    let spent = began.elapsed();
    assert_eq!(
        view,
        View::Plain {
            reason: Reason::Timeout(300)
        }
    );
    assert!(spent < Duration::from_secs(5), "it returned in {spent:?}");
    // The killed process never reached its last line.
    std::thread::sleep(Duration::from_millis(500));
    assert!(!root.path().join("slow/finished.txt").exists());
    // And the host still answers.
    assert!(!matches!(host.open(&file), View::Blocks { .. }));
}

#[test]
fn a_failure_code_is_a_reason_with_its_stderr() {
    let Some(python) = python() else {
        eprintln!("skipping: no python on PATH");
        return;
    };
    let root = tempfile::tempdir().expect("a temp dir");
    let script = "import sys\nsys.stderr.write('boom\\n')\nsys.exit(3)\n";
    plugin(
        root.path(),
        "fail",
        script,
        &manifest(&python, "fail", ".fail", 5_000),
    );
    let host = Host::load(root.path());
    let file = root.path().join("a.fail");
    std::fs::write(&file, "x").expect("a file");
    assert_eq!(
        host.open(&file),
        View::Plain {
            reason: Reason::Exit {
                code: Some(3),
                stderr: "boom".into()
            }
        }
    );
}

#[test]
fn an_answer_that_is_not_json_is_a_reason() {
    let Some(python) = python() else {
        eprintln!("skipping: no python on PATH");
        return;
    };
    let root = tempfile::tempdir().expect("a temp dir");
    plugin(
        root.path(),
        "junk",
        "print('not json')\n",
        &manifest(&python, "junk", ".junk", 5_000),
    );
    let host = Host::load(root.path());
    let file = root.path().join("a.junk");
    std::fs::write(&file, "x").expect("a file");
    assert!(matches!(
        host.open(&file),
        View::Plain {
            reason: Reason::InvalidJson(_)
        }
    ));
}

#[test]
fn a_command_that_is_not_there_is_a_reason() {
    let root = tempfile::tempdir().expect("a temp dir");
    let folder = root.path().join("ghost");
    std::fs::create_dir_all(&folder).expect("a folder");
    std::fs::write(
        folder.join("manifest.json"),
        r#"{"id": "ghost", "version": "1", "apiVersion": 1, "extensions": [".ghost"],
            "command": ["green-no-such-command-2f4a", "x"]}"#,
    )
    .expect("a manifest");
    let host = Host::load(root.path());
    let file = root.path().join("a.ghost");
    std::fs::write(&file, "x").expect("a file");
    assert!(matches!(
        host.open(&file),
        View::Plain {
            reason: Reason::Spawn(_)
        }
    ));
}

#[test]
fn a_bad_answer_version_is_a_reason() {
    let Some(python) = python() else {
        eprintln!("skipping: no python on PATH");
        return;
    };
    let root = tempfile::tempdir().expect("a temp dir");
    plugin(
        root.path(),
        "future",
        "print('{\"apiVersion\": 2, \"blocks\": []}')\n",
        &manifest(&python, "future", ".future", 5_000),
    );
    let host = Host::load(root.path());
    let file = root.path().join("a.future");
    std::fs::write(&file, "x").expect("a file");
    assert_eq!(
        host.open(&file),
        View::Plain {
            reason: Reason::Api { found: 2 }
        }
    );
}

#[test]
fn unknown_fields_and_blocks_are_ignored_on_both_sides() {
    let Some(python) = python() else {
        eprintln!("skipping: no python on PATH");
        return;
    };
    let root = tempfile::tempdir().expect("a temp dir");
    let script = r#"
import json, sys
request = json.load(sys.stdin)
assert request["apiVersion"] == 1 and request["contentOmitted"] is False
print(json.dumps({
    "apiVersion": 1,
    "future": "ignored",
    "blocks": [
        {"kind": "paragraph", "text": "kept", "future": 1},
        {"kind": "future", "text": "dropped"},
        {"kind": "heading", "level": 2, "text": "also kept"}
    ],
    "meta": {"future": True},
}))
"#;
    plugin(
        root.path(),
        "wide",
        script,
        &format!(
            r#"{{"id": "wide", "version": "1", "apiVersion": 1, "future": 1,
                "extensions": [".wide"], "command": ["{python}", "run.py"]}}"#
        ),
    );
    let host = Host::load(root.path());
    let file = root.path().join("a.wide");
    std::fs::write(&file, "x").expect("a file");
    let view = host.open(&file);
    assert_eq!(
        view.blocks().expect("blocks"),
        [
            Block::Paragraph("kept".into()),
            Block::Heading {
                level: 2,
                text: "also kept".into()
            }
        ]
    );
}

#[test]
fn a_file_past_the_limit_sends_its_path_alone() {
    let Some(python) = python() else {
        eprintln!("skipping: no python on PATH");
        return;
    };
    let root = tempfile::tempdir().expect("a temp dir");
    let script = r#"
import json, sys
request = json.load(sys.stdin)
assert request["content"] is None, "the bytes came anyway"
assert request["contentOmitted"] is True
assert request["path"].endswith("big.omitted")
print(json.dumps({"apiVersion": 1, "blocks": [{"kind": "paragraph", "text": "path only"}]}))
"#;
    plugin(
        root.path(),
        "omitted",
        script,
        &manifest(&python, "omitted", ".omitted", 5_000),
    );
    let host = Host::load(root.path());
    let file = root.path().join("big.omitted");
    std::fs::write(&file, vec![b'a'; green_plugins::CONTENT_LIMIT as usize + 1])
        .expect("a big file");
    let view = host.open(&file);
    assert_eq!(
        view.blocks().expect("blocks"),
        [Block::Paragraph("path only".into())]
    );
}

#[test]
fn three_bad_manifests_leave_the_host_serving() {
    let root = tempfile::tempdir().expect("a temp dir");
    let broken = root.path().join("syntax");
    std::fs::create_dir_all(&broken).expect("a folder");
    std::fs::write(broken.join("manifest.json"), "{\"id\": ").expect("a manifest");
    let version = root.path().join("version");
    std::fs::create_dir_all(&version).expect("a folder");
    std::fs::write(
        version.join("manifest.json"),
        r#"{"id": "v", "version": "1", "apiVersion": 99, "extensions": [".v"],
            "command": ["python", "x.py"]}"#,
    )
    .expect("a manifest");
    let command = root.path().join("command");
    std::fs::create_dir_all(&command).expect("a folder");
    std::fs::write(
        command.join("manifest.json"),
        r#"{"id": "c", "version": "1", "apiVersion": 1, "extensions": [".c"],
            "command": []}"#,
    )
    .expect("a manifest");
    let host = Host::load(root.path());
    assert_eq!(host.problems().len(), 3, "{:?}", host.problems());
    assert!(host.plugins().is_empty());
    let file = root.path().join("a.md");
    std::fs::write(&file, "# x\n").expect("a file");
    assert!(!host.serves(&file));
}
