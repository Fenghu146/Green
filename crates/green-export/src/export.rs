//! One export: blocks to Typst, Typst to PDF, the PDF audited and judged.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::audit::{self, Audit, Warning};
use crate::compile::Compiler;
use crate::doc::Block;
use crate::typst;

/// A finished export.
#[derive(Clone, Debug)]
pub struct Export {
    /// The PDF written.
    pub pdf: PathBuf,
    /// The Typst source it was compiled from.
    pub source: String,
    /// What the PDF turns out to hold.
    pub audit: Audit,
    /// Every size threshold the audit crosses; empty is a good export.
    pub warnings: Vec<Warning>,
    /// Compiling and auditing.
    pub elapsed: Duration,
}

impl Export {
    /// The design §7 audit line.
    pub fn line(&self) -> String {
        self.audit.line()
    }
}

/// `blocks` as the PDF at `pdf`; `compiler` builds it.
pub fn export(blocks: &[Block], pdf: &Path, compiler: &dyn Compiler) -> Result<Export> {
    let started = Instant::now();
    let source = typst::source(blocks)?;
    compiler.compile(&source, pdf)?;
    let audit = audit::read(pdf)?;
    let export = Export {
        pdf: pdf.to_path_buf(),
        source,
        warnings: audit.warnings(),
        audit,
        elapsed: started.elapsed(),
    };
    log::info!("{}", export.line());
    for warning in &export.warnings {
        log::warn!("{}", warning.message());
    }
    Ok(export)
}
