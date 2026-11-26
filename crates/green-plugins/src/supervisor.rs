//! One plugin run: spawn it, hand it JSON on stdin, read JSON from stdout,
//! and kill it when its time is up (design §3).

use std::{
    fmt,
    io::{Read as _, Write as _},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

use serde::Serialize;

use crate::{
    blocks::Response,
    manifest::{API_VERSION, Plugin},
};

/// How much of a failed run's stderr the reason keeps.
const STDERR_KEPT: usize = 400;
/// How long the pipes get to close after the process ends.
const CLOSING: Duration = Duration::from_secs(2);
/// How often the process is asked whether it has ended.
const STEP: Duration = Duration::from_millis(5);

/// What the host writes to a plugin's stdin.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Request<'a> {
    pub api_version: u32,
    pub path: &'a str,
    pub ext: &'a str,
    /// Base64 of the file's bytes; `None` when `content_omitted`.
    pub content: Option<String>,
    pub content_omitted: bool,
    pub capabilities: &'a [&'a str],
}

/// Why one run produced no view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The process did not start.
    Spawn(String),
    /// The request did not serialize.
    Request(String),
    /// The host could not wait for the process.
    Wait(String),
    /// The time ran out; the process was killed.
    Timeout(u64),
    /// It ended with a failure code.
    Exit { code: Option<i32>, stderr: String },
    /// It wrote no JSON, or JSON this build cannot read.
    InvalidJson(String),
    /// Its `apiVersion` is not this build's.
    Api { found: u32 },
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::Spawn(what) => write!(f, "it did not start: {what}"),
            Failure::Request(why) => write!(f, "the request did not serialize: {why}"),
            Failure::Wait(why) => write!(f, "it could not be waited for: {why}"),
            Failure::Timeout(ms) => write!(f, "it did not finish within {ms} ms and was killed"),
            Failure::Exit { code, stderr } if stderr.is_empty() => {
                write!(f, "it ended with {}", code_word(*code))
            }
            Failure::Exit { code, stderr } => {
                write!(f, "it ended with {}: {stderr}", code_word(*code))
            }
            Failure::InvalidJson(why) => write!(f, "its answer is not JSON: {why}"),
            Failure::Api { found } => write!(f, "its apiVersion {found} is not {API_VERSION}"),
        }
    }
}

fn code_word(code: Option<i32>) -> String {
    match code {
        Some(code) => format!("exit code {code}"),
        None => "no exit code".to_string(),
    }
}

/// Runs `plugin` for one request, killing it at its `timeoutMs`.
pub(crate) fn run(plugin: &Plugin, request: &Request<'_>) -> Result<Response, Failure> {
    let manifest = &plugin.manifest;
    let mut command = Command::new(&manifest.command[0]);
    command
        .args(&manifest.command[1..])
        .current_dir(&plugin.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A GUI app spawns no console window for its plugins.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .spawn()
        .map_err(|error| Failure::Spawn(format!("{}: {error}", manifest.command[0])))?;
    let input = serde_json::to_vec(request).map_err(|error| Failure::Request(error.to_string()))?;
    // Each pipe has its own thread: a plugin may read late or not at all.
    let writer = child.stdin.take().map(|mut stdin| {
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        })
    });
    let (out_tx, out_rx) = mpsc::channel();
    let reader = child.stdout.take().map(|mut stdout| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stdout.read_to_end(&mut bytes);
            let _ = out_tx.send(bytes);
        })
    });
    let (err_tx, err_rx) = mpsc::channel();
    let errors = child.stderr.take().map(|mut stderr| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stderr.read_to_end(&mut bytes);
            let _ = err_tx.send(bytes);
        })
    });
    let deadline = Instant::now() + Duration::from_millis(manifest.timeout_ms);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => {
                log::warn!(
                    "plugins: {} ran past {} ms; killed",
                    manifest.id,
                    manifest.timeout_ms
                );
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(STEP),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Failure::Wait(error.to_string()));
            }
        }
    };
    // A pipe a grandchild still holds would hold these; the view matters more.
    drop(writer);
    drop(reader);
    drop(errors);
    let stdout = out_rx.recv_timeout(CLOSING).unwrap_or_default();
    let stderr = err_rx.recv_timeout(CLOSING).unwrap_or_default();
    let Some(status) = status else {
        return Err(Failure::Timeout(manifest.timeout_ms));
    };
    if !status.success() {
        return Err(Failure::Exit {
            code: status.code(),
            stderr: tail(&stderr),
        });
    }
    let response = Response::parse(&stdout).map_err(Failure::InvalidJson)?;
    if response.api_version != API_VERSION {
        return Err(Failure::Api {
            found: response.api_version,
        });
    }
    Ok(response)
}

/// The last `STDERR_KEPT` bytes of `bytes`, as one line.
fn tail(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(STDERR_KEPT);
    let tail = String::from_utf8_lossy(&bytes[start..]);
    tail.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tail_keeps_the_end_on_one_line() {
        let long = format!("{}\nTraceback: boom\n", "x".repeat(1_000));
        let tail = tail(long.as_bytes());
        assert!(tail.ends_with("Traceback: boom"));
        assert!(tail.len() <= STDERR_KEPT);
        assert!(!tail.contains('\n'));
    }

    #[test]
    fn a_failure_says_what_happened() {
        assert_eq!(
            Failure::Timeout(300).to_string(),
            "it did not finish within 300 ms and was killed"
        );
        assert_eq!(
            Failure::Exit {
                code: Some(3),
                stderr: "boom".into()
            }
            .to_string(),
            "it ended with exit code 3: boom"
        );
    }
}
