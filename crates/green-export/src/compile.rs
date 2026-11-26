//! Compilation: the `typst` executable today, `typst::compile` in process later.

use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};

/// A way to turn Typst source into a PDF. The CLI implements it now; the
/// library API another line is proving implements the same trait later, so the
/// export above does not change.
pub trait Compiler {
    /// `source` as a PDF at `out`.
    fn compile(&self, source: &str, out: &Path) -> Result<()>;
}

/// The `typst` executable: source arrives on stdin, so no `.typ` lands beside
/// the PDF. `root` bounds what the document may read, and is where relative
/// picture addresses resolve.
pub struct Cli {
    exe: PathBuf,
    fonts: PathBuf,
    root: PathBuf,
}

impl Cli {
    /// The executable at `exe`, reading fonts from `fonts` and pictures from `root`.
    pub fn new(
        exe: impl Into<PathBuf>,
        fonts: impl Into<PathBuf>,
        root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            exe: exe.into(),
            fonts: fonts.into(),
            root: root.into(),
        }
    }
}

impl Compiler for Cli {
    fn compile(&self, source: &str, out: &Path) -> Result<()> {
        let mut child = Command::new(&self.exe)
            .arg("compile")
            .arg("--font-path")
            .arg(&self.fonts)
            .arg("--root")
            .arg(&self.root)
            // Only the bundled faces: a system font or Typst's own assets would
            // be embedded whole, and the PDF would not match the editor.
            .arg("--ignore-system-fonts")
            .arg("--ignore-embedded-fonts")
            .arg("-")
            .arg(out)
            .current_dir(&self.root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("starting {}", self.exe.display()))?;
        child
            .stdin
            .take()
            .expect("a piped stdin")
            .write_all(source.as_bytes())
            .with_context(|| format!("writing the source for {}", out.display()))?;
        let output = child
            .wait_with_output()
            .with_context(|| format!("waiting for {}", self.exe.display()))?;
        let said = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            bail!("typst {}\n{}", output.status, said.trim());
        }
        if !said.trim().is_empty() {
            log::warn!("export: typst {}", said.trim());
        }
        Ok(())
    }
}
