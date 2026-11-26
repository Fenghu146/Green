//! Which characters the bundled faces cannot draw: Typst draws a tofu and says
//! nothing, so the characters are checked against the fonts' cmaps instead.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result};
use ttf_parser::Face;

use crate::doc::{Block, Inline, Style};

/// A character none of the bundled fonts has a glyph for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Missing {
    /// The character, as written.
    pub ch: char,
    /// Where it first appears, such as `block 3 > list item 2`.
    pub at: String,
}

impl Missing {
    /// For the warning line: the code point and where it is.
    pub fn describe(&self) -> String {
        format!("{:?} (U+{:04X}) at {}", self.ch, self.ch as u32, self.at)
    }
}

/// The characters `blocks` draw that none of the fonts in `dir` covers.
pub fn missing(blocks: &[Block], dir: &Path) -> Result<Vec<Missing>> {
    let files = fonts(dir)?;
    let faces = files
        .iter()
        .map(|(name, bytes)| {
            Face::parse(bytes, 0).with_context(|| format!("{name} is not a font this check reads"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut drawn = Vec::new();
    walk(blocks, "", &mut drawn);

    let mut seen = HashSet::new();
    let mut missing = Vec::new();
    for (ch, at) in drawn {
        // Whitespace and control characters are layout, not glyphs; a mark the
        // converter replaces is drawn as a symbol the fonts do have.
        let mark = crate::typst::MARKS.iter().any(|(mark, _)| *mark == ch);
        if ch.is_whitespace() || ch.is_control() || mark || !seen.insert(ch) {
            continue;
        }
        if !faces.iter().any(|face| face.glyph_index(ch).is_some()) {
            missing.push(Missing { ch, at });
        }
    }
    Ok(missing)
}

/// The font files in `dir`, by name.
fn fonts(dir: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("listing {}", dir.display()))?
        .flatten()
    {
        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "otf" | "ttf" | "ttc" | "otc") {
            continue;
        }
        let bytes = std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        files.push((path.display().to_string(), bytes));
    }
    files.sort();
    anyhow::ensure!(!files.is_empty(), "no fonts in {}", dir.display());
    Ok(files)
}

/// `path`, then `step`.
fn join(path: &str, step: &str) -> String {
    match path.is_empty() {
        true => step.to_string(),
        false => format!("{path} > {step}"),
    }
}

/// Each character drawn under `blocks`, in reading order, with where it is.
fn walk(blocks: &[Block], path: &str, out: &mut Vec<(char, String)>) {
    for (index, block) in blocks.iter().enumerate() {
        let here = join(path, &format!("block {}", index + 1));
        match block {
            Block::Paragraph(text) | Block::Heading(_, text) => inline_text(text, &here, out),
            Block::Quote(body) => walk(body, &join(&here, "quote"), out),
            Block::Alert { title, body } => {
                drawn(title, &join(&here, "alert title"), out);
                walk(body, &join(&here, "alert"), out);
            }
            Block::Code { text, .. } => drawn(text, &join(&here, "code"), out),
            Block::List { items, .. } => {
                for (item, entry) in items.iter().enumerate() {
                    walk(
                        &entry.blocks,
                        &join(&here, &format!("item {}", item + 1)),
                        out,
                    );
                }
            }
            Block::Table { head, rows, .. } => {
                for (cell, blocks) in head.iter().enumerate() {
                    walk(
                        blocks,
                        &join(&here, &format!("head cell {}", cell + 1)),
                        out,
                    );
                }
                for (row, cells) in rows.iter().enumerate() {
                    for (cell, blocks) in cells.iter().enumerate() {
                        walk(
                            blocks,
                            &join(&here, &format!("row {} cell {}", row + 1, cell + 1)),
                            out,
                        );
                    }
                }
            }
            Block::Rule | Block::Toc | Block::Image { .. } => {}
            // TeX is structure; only what it prints as text can be missing.
            Block::Math(tex) => math(tex, &here, out),
            Block::Footnote { label, body } => {
                drawn(label, &join(&here, "footnote label"), out);
                walk(body, &join(&here, "footnote"), out);
            }
        }
    }
}

/// A paragraph's drawn characters: its text, minus the TeX inside math spans.
fn inline_text(inline: &Inline, at: &str, out: &mut Vec<(char, String)>) {
    for (offset, ch) in inline.text.char_indices() {
        let in_math = inline
            .styles
            .iter()
            .any(|(range, style)| *style == Style::Math && range.contains(&offset));
        if in_math && ch.is_ascii() {
            continue;
        }
        out.push((ch, at.to_string()));
    }
}

/// A formula's printed text: ASCII is markup, the rest is text Typst quotes.
fn math(tex: &str, at: &str, out: &mut Vec<(char, String)>) {
    for ch in tex.chars().filter(|ch| !ch.is_ascii()) {
        out.push((ch, format!("{at} > formula")));
    }
}

fn drawn(text: &str, at: &str, out: &mut Vec<(char, String)>) {
    for ch in text.chars() {
        out.push((ch, at.to_string()));
    }
}
