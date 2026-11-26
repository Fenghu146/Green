//! Compile one formula before the whole document goes to Typst.
//!
//! `typst::compile` is all-or-nothing: one bad formula would otherwise take
//! every other one out of the PDF. The editor has the same probe in
//! `ely-documents::math`; export keeps its own so the PDF path stays vector
//! only and does not pull in the component library.

use std::{fs, path::Path};

use anyhow::{Context as _, Result, anyhow, bail};
use ::typst::{
    diag::FileError,
    foundations::{Bytes, Datetime, Duration},
    syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot},
    text::{Font, FontBook},
    utils::LazyHash,
    LibraryExt as _,
};
use typst_layout::PagedDocument;


/// The bundled faces and one fixed in-memory source, shared by every probe.
pub struct Preflight {
    library: LazyHash<::typst::Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
}

impl Preflight {
    /// Reads the same folder the CLI gets through `--font-path`.
    pub fn new(dir: &Path) -> Result<Self> {
        let mut fonts = Vec::new();
        for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
            let path = entry.context("a font entry")?.path();
            if !path.is_file() {
                continue;
            }
            let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
            if let Some(font) = Font::new(Bytes::new(bytes), 0) {
                fonts.push(font);
            }
        }
        let book = FontBook::from_fonts(fonts.iter());
        if book.select("noto sans math", Default::default()).is_none() {
            bail!("Noto Sans Math is not among the bundled faces");
        }
        let main = FileId::new(
            RootedPath::new(
                VirtualRoot::Project,
                VirtualPath::new("preflight.typ").map_err(|_| anyhow!("preflight.typ is not virtual"))?,
            ),
        );
        Ok(Self {
            library: LazyHash::new(::typst::Library::default()),
            book: LazyHash::new(book),
            fonts,
            main,
        })
    }

    /// `markup` as a one-formula document: does Typst actually typeset it?
    pub fn check(&self, markup: &str) -> Result<()> {
        let source = format!("{} $ {markup} $\n", crate::typst::preamble());
        let world = World {
            preflight: self,
            source: Source::new(self.main, source),
        };
        let result = ::typst::compile::<PagedDocument>(&world);
        for warning in &result.warnings {
            log::warn!("export: typst said: {}", warning.message);
        }
        match result.output {
            Ok(_) => Ok(()),
            Err(errors) => {
                let first = errors
                    .into_iter()
                    .next()
                    .map(|error| error.message.to_string())
                    .unwrap_or_else(|| "the formula does not compile".into());
                bail!("{first}")
            }
        }
    }
}

struct World<'a> {
    preflight: &'a Preflight,
    source: Source,
}

impl ::typst::World for World<'_> {
    fn library(&self) -> &LazyHash<::typst::Library> {
        &self.preflight.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.preflight.book
    }

    fn main(&self) -> FileId {
        self.preflight.main
    }

    fn source(&self, id: FileId) -> ::typst::diag::FileResult<Source> {
        match id == self.preflight.main {
            true => Ok(self.source.clone()),
            false => Err(FileError::NotFound(id.vpath().get_without_slash().into())),
        }
    }

    fn file(&self, id: FileId) -> ::typst::diag::FileResult<Bytes> {
        self.source(id)
            .map(|source| Bytes::new(source.text().as_bytes().to_vec()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.preflight.fonts.get(index).cloned()
    }

    fn today(&self, _: Option<Duration>) -> Option<Datetime> {
        None
    }
}
