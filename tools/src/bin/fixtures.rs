//! Writes the generated fixtures, or checks the committed ones.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use green_tools::fixtures::{self, Artifact};

const MANIFEST: &str = "fixtures/perf/GENERATED.md";

fn main() -> ExitCode {
    let check = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => false,
        [flag] if flag == "--check" => true,
        [flag] if flag == "--help" || flag == "-h" => {
            println!(
                "Usage: fixtures [--check]\n\nWrites the generated fixtures, or with --check compares the committed copies."
            );
            return ExitCode::SUCCESS;
        }
        other => {
            eprintln!("fixtures: unknown arguments {other:?}; try --help");
            return ExitCode::from(2);
        }
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tools/ has a parent")
        .to_path_buf();
    match if check { verify(&root) } else { write(&root) } {
        Ok(()) => ExitCode::SUCCESS,
        Err(problem) => {
            eprintln!("fixtures: {problem}");
            ExitCode::FAILURE
        }
    }
}

fn write(root: &Path) -> Result<(), String> {
    let artifacts = fixtures::all();
    for artifact in &artifacts {
        let path = root.join(artifact.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        fs::write(&path, &artifact.bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        report(artifact);
    }
    let manifest = root.join(MANIFEST);
    fs::write(&manifest, fixtures::manifest(&artifacts))
        .map_err(|error| format!("{}: {error}", manifest.display()))?;
    println!("fixtures: wrote {} and {MANIFEST}", artifacts.len());
    Ok(())
}

fn verify(root: &Path) -> Result<(), String> {
    let artifacts = fixtures::all();
    let mut stale = Vec::new();
    for artifact in &artifacts {
        let path = root.join(artifact.path);
        match fs::read(&path) {
            Ok(bytes) if bytes == artifact.bytes => report(artifact),
            Ok(_) => stale.push(format!("{} differs", artifact.path)),
            Err(error) => stale.push(format!("{}: {error}", artifact.path)),
        }
    }
    let manifest = root.join(MANIFEST);
    match fs::read_to_string(&manifest) {
        Ok(text) if text == fixtures::manifest(&artifacts) => {}
        Ok(_) => stale.push(format!("{MANIFEST} differs")),
        Err(error) => stale.push(format!("{MANIFEST}: {error}")),
    }
    if stale.is_empty() {
        println!("fixtures: {} files match the generator", artifacts.len());
        return Ok(());
    }
    Err(format!(
        "stale, rerun without --check:\n  {}",
        stale.join("\n  ")
    ))
}

fn report(artifact: &Artifact) {
    let chars = artifact
        .text()
        .map_or(String::from("—"), |_| artifact.chars().to_string());
    println!(
        "  {:<40} {:>9} bytes {:>8} chars  {}",
        artifact.path,
        artifact.bytes.len(),
        chars,
        artifact.note
    );
}
