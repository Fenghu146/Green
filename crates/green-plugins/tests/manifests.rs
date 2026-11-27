//! The shipped examples must declare a timeout that covers interpreter startup,
//! and their READMEs must not quote a different number from the manifest.
//! A missing field silently takes the host default (8 s), which is too close
//! to the 7.47 s cold start observed on windows-latest.

use std::{fs, path::Path};

const MIN_TIMEOUT_MS: u64 = 15_000;

fn interpreter(command: &str) -> bool {
    let name = Path::new(command)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(command)
        .to_ascii_lowercase();
    matches!(
        name.as_str(),
        "python" | "python3" | "python.exe" | "python3.exe" | "node" | "node.exe"
    )
}

/// Only `timeoutMs: N` and `"timeoutMs": N` count; prose such as
/// "defaults to 8000" is deliberately ignored.
fn quoted_timeouts(readme: &str) -> Vec<u64> {
    let mut found = Vec::new();
    for needle in ["timeoutMs:", "\"timeoutMs\":"] {
        for (at, _) in readme.match_indices(needle) {
            let digits: String = readme[at + needle.len()..]
                .trim_start()
                .chars()
                .take_while(|ch| ch.is_ascii_digit())
                .collect();
            if let Ok(value) = digits.parse() {
                found.push(value);
            }
        }
    }
    found
}

#[test]
fn shipped_interpreter_plugins_declare_a_generous_timeout() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins");
    let mut checked = 0;
    for entry in fs::read_dir(&root).expect("examples/plugins") {
        let folder = entry.expect("an example folder").path();
        let path = folder.join("manifest.json");
        if !path.is_file() {
            continue;
        }
        let bytes = fs::read(&path).expect("a manifest");
        let manifest: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON manifest");
        let command = manifest
            .get("command")
            .and_then(|command| command.get(0))
            .and_then(|command| command.as_str())
            .unwrap_or_default();
        if !interpreter(command) {
            continue;
        }
        checked += 1;
        let timeout = manifest.get("timeoutMs").and_then(|value| value.as_u64());
        assert!(
            timeout.is_some_and(|ms| ms >= MIN_TIMEOUT_MS),
            "{}: interpreter plugin {command:?} needs an explicit timeoutMs >= {MIN_TIMEOUT_MS} ms, found {timeout:?}; windows-latest cold starts reach 7.47 s",
            path.display()
        );

        let readme = folder.join("README.md");
        if readme.is_file() {
            let text = fs::read_to_string(&readme).expect("README.md");
            for quoted in quoted_timeouts(&text) {
                assert_eq!(
                    quoted,
                    timeout.expect("a valid timeout above"),
                    "{}: README says timeoutMs {quoted}, but {} says {timeout:?}",
                    readme.display(),
                    path.display()
                );
            }
        }
    }
    assert!(
        checked > 0,
        "no interpreter plugins found under {}",
        root.display()
    );
}
