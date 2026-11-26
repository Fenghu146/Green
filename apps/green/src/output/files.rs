//! An export written to disk.

use std::{
    io,
    path::Path,
};

use super::{Page, Pictures, body, document, page_of, pictures::Placed, plain};

/// What an export writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// One HTML file, pictures inside.
    Page,
    /// HTML and a folder of its pictures.
    Linked,
    Pdf,
    Text,
}

/// Where one export goes, and what it leaves alone.
pub struct Target<'a> {
    pub path: &'a Path,
    /// The tab's own file, never written over.
    pub session: Option<&'a Path>,
    pub revision: u64,
    /// A scratch folder of the tab's.
    pub work: &'a Path,
    /// The five alert titles, in the window's language.
    pub alerts: &'a [String; 5],
}

/// `text` written as `format`; what it left out.
pub fn write(
    format: Format,
    text: &str,
    beside: Option<&Path>,
    target: &Target,
) -> io::Result<Page> {
    let title = target
        .path
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let page = match format {
        Format::Page => {
            let page = body(text, beside, &Pictures::Inline)?;
            export(target, document(&title, &page.html).as_bytes())?;
            page
        }
        Format::Linked => linked(text, beside, target, &title)?,
        Format::Pdf => pdf(text, beside, target)?,
        Format::Text => {
            export(target, plain(text)?.as_bytes())?;
            Page::default()
        }
    };
    log::info!("mark: exported {format:?}, revision {}", target.revision);
    Ok(page)
}

fn export(target: &Target, bytes: &[u8]) -> io::Result<()> {
    ely_files::export(target.path, target.session, bytes, target.revision).map(|_| ())
}

/// The page beside a new folder of its pictures.
fn linked(text: &str, beside: Option<&Path>, target: &Target, title: &str) -> io::Result<Page> {
    let folder = target.path.with_file_name(format!("{title}_files"));
    let pictures = Pictures::Folder(folder.clone());
    let mut placed = Placed::new(beside, &pictures);
    let written = page_of(text, &mut placed)
        .and_then(|page| export(target, document(title, &page.html).as_bytes()).map(|()| page));
    // A failure takes only the folder it made.
    if written.is_err()
        && placed.made
        && let Err(left) = std::fs::remove_dir_all(&folder)
    {
        log::error!("mark: the half-made picture folder stays: {left}");
    }
    written
}

/// The page set by Typst, through `green-export`.
fn pdf(text: &str, beside: Option<&Path>, target: &Target) -> io::Result<Page> {
    let dir = target.work.join("pdf");
    std::fs::create_dir_all(&dir)?;
    let set = (|| -> io::Result<Page> {
        let out = dir.join("out.pdf");
        let page = super::pdf::write(text, &out, beside, target.work, target.alerts)?;
        export(target, &std::fs::read(&out)?)?;
        Ok(page)
    })();
    let cleaned = std::fs::remove_dir_all(&dir);
    let page = set?;
    cleaned?;
    Ok(page)
}
