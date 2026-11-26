//! The bundled fonts: one folder, GPUI and Typst read the same files.

use std::{borrow::Cow, path::PathBuf};

use anyhow::{Context as _, Result};
use gpui::App;

/// The family Green's text uses unless a setting says otherwise.
pub const FAMILY: &str = "Noto Sans CJK SC";

/// The family code, paths and spans use, on screen and in the export.
pub const MONO: &str = "Noto Sans Mono";

/// The family formulas use, in the editor and in the export.
pub const MATH: &str = "Noto Sans Math";

/// The faces shipped: the two CJK weights, and the math and code faces.
const FILES: [&str; 4] = [
    "NotoSansCJKsc-Regular.otf",
    "NotoSansCJKsc-Bold.otf",
    "NotoSansMath-Regular.ttf",
    "NotoSansMono-Regular.ttf",
];

/// A build from source keeps the files in the repository.
const DEV: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/fonts");

/// `fonts/` beside the executable, else the source tree's.
pub fn dir() -> PathBuf {
    match std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join("fonts")))
        .filter(|beside| beside.is_dir())
    {
        Some(beside) => beside,
        None => PathBuf::from(DEV),
    }
}

/// The files' bytes, in `FILES` order.
pub fn bytes() -> Result<Vec<Vec<u8>>> {
    let dir = dir();
    FILES
        .iter()
        .map(|name| {
            std::fs::read(dir.join(name)).with_context(|| format!("{}", dir.join(name).display()))
        })
        .collect()
}

/// Registers them with GPUI; the family must be there after.
pub fn install(cx: &App) -> Result<()> {
    let fonts = bytes()?
        .into_iter()
        .map(Cow::Owned)
        .collect::<Vec<_>>();
    log::info!("fonts: {} files from {}", fonts.len(), dir().display());
    cx.text_system().add_fonts(fonts)?;
    for family in [FAMILY, MONO, MATH] {
        anyhow::ensure!(
            cx.text_system().all_font_names().iter().any(|name| *name == *family),
            "{family} is not in the files at {}",
            dir().display()
        );
    }
    log::info!("fonts: Typst reads {} families from the same files", book()?.families().count());
    Ok(())
}

/// The same files for Typst, which lays out formulas and the export.
pub fn book() -> Result<typst::text::FontBook> {
    let fonts = FILES
        .iter()
        .zip(bytes()?)
        .map(|(name, data)| {
            typst::text::Font::new(typst::foundations::Bytes::new(data), 0)
                .with_context(|| format!("{name} is not a font Typst reads"))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(typst::text::FontBook::from_fonts(&fonts))
}
