//! Compilation: Typst in process for the product; the executable only in tests.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, anyhow, bail};
use typst_layout::PagedDocument;

use crate::preflight::Preflight;

/// A way to turn Typst source into a PDF. The product uses [`Lib`]; [`Cli`]
/// exists only for the tests that compare it with the official executable.
pub trait Compiler {
    /// `source` as a PDF at `out`.
    fn compile(&self, source: &str, out: &Path) -> Result<()>;
}

/// Typst linked into the program: no executable, no system fonts, no scratch
/// `.typ`. The faces come from the same folder the editor reads.
pub struct Lib {
    preflight: Preflight,
    root: PathBuf,
}

impl Lib {
    /// The bundled faces in `fonts`, with relative files read from `root`.
    pub fn new(fonts: &Path, root: &Path) -> Result<Self> {
        Ok(Self {
            preflight: Preflight::new(fonts)?,
            root: root.to_path_buf(),
        })
    }
}

impl Compiler for Lib {
    fn compile(&self, source: &str, out: &Path) -> Result<()> {
        let world = self.preflight.world(source.to_owned(), Some(&self.root));
        let warned = ::typst::compile::<PagedDocument>(&world);
        for warning in &warned.warnings {
            log::warn!("export: typst said: {}", warning.message);
        }
        let document = warned.output.map_err(first_error)?;
        // `timestamp: None` is deliberate: the same source gives the same
        // bytes. The CLI's 185 B of date metadata is not a target (design §7).
        let pdf =
            typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(first_error)?;
        fs::write(out, pdf).with_context(|| format!("writing {}", out.display()))
    }
}

fn first_error(errors: impl IntoIterator<Item = ::typst::diag::SourceDiagnostic>) -> anyhow::Error {
    errors
        .into_iter()
        .next()
        .map(|error| anyhow!("{}", error.message))
        .unwrap_or_else(|| anyhow!("typst reported an unnamed error"))
}

/// The `typst` executable, only for the tests that compare the library with
/// it: source arrives on stdin, so no `.typ` lands beside the PDF. `root`
/// bounds what the document may read, and is where relative pictures resolve.
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
