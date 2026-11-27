//! Markdown as a PDF: the document's blocks through `green-export`, compiled
//! in process (design §7).
//!
//! The tree the editor draws is mapped block for block onto the export's own
//! `Block`, so the PDF holds what the screen holds; front matter is metadata and
//! is not exported. Pictures are copied into the export's scratch folder, and a
//! picture that cannot be carried is left out and said, as the HTML export says
//! it.

use std::{fs, io, path::Path};

use base64::Engine as _;
use ely_gpui_component::documents::{
    AlertKind, AlignKind, Inline, MarkdownBlock, MarkdownReader, Style, alert, markdown_tree, mdast,
};
use green_export::{Align, Block, Inline as Exported, Item, Style as ExportedStyle};

use super::{
    Page, Pictures,
    pictures::{Placed, decoded},
};

/// `text` as the PDF at `path`; `beside` holds the document, `work` is scratch.
pub fn write(text: &str, path: &Path, beside: Option<&Path>, work: &Path, alerts: &[String; 5]) -> io::Result<Page> {
    let nodes = nodes_of(text);
    let folder = work.join("pdf");
    fs::create_dir_all(&folder)?;
    // Pictures copied in, as the HTML export copies them; Typst reads the folder.
    let shelf = Pictures::Pdf(folder.join("files"));
    let mut placed = Placed::new(beside, &shelf);
    let mut carried = Carried {
        placed: &mut placed,
        folder: &folder,
        inline: 0,
        missing: Vec::new(),
        alerts,
    };
    let mut page = Page::default();
    let blocks = blocks_of(&nodes, &mut carried)?;
    page.missing.append(&mut carried.missing);
    let fonts = ely_ui::fonts::dir();
    let compiler = green_export::Lib::new(&fonts, &folder).map_err(io::Error::other)?;
    let export = green_export::export(&blocks, path, &compiler, &fonts).map_err(io::Error::other)?;
    // Characters no bundled face draws print as tofu: say which, and where.
    page.missing.extend(export.missing.iter().map(|missing| missing.describe()));
    // Formulas the converter refused stay as their TeX (§7.8): say which.
    page.formulas_kept = export
        .formulas
        .iter()
        .map(|(tex, _)| {
            let one = tex.lines().next().unwrap_or_default().trim();
            match one.chars().count() > 60 {
                true => format!("{}…", one.chars().take(60).collect::<String>()),
                false => one.to_string(),
            }
        })
        .collect();
    Ok(page)
}

/// The document's blocks, alerts read at the root as the HTML export reads them.
fn nodes_of(text: &str) -> Vec<MarkdownBlock> {
    let tree = markdown_tree(text);
    let reader = MarkdownReader::joined(text, &tree);
    let mut nodes = Vec::new();
    for node in tree.children().expect("the root holds the blocks") {
        // A GitHub alert is a quote with a marker: the reader sees a quote, so
        // it is recognised here, as the HTML export recognises it.
        match node {
            mdast::Node::Blockquote(quote) => match alert(quote) {
                Some((kind, body)) => {
                    let mut blocks = Vec::new();
                    for node in &body {
                        reader.block(node, &mut blocks);
                    }
                    nodes.push(MarkdownBlock::Alert { kind, body: blocks });
                }
                None => reader.block(node, &mut nodes),
            },
            _ => reader.block(node, &mut nodes),
        }
    }
    nodes
}

/// Where the export's pictures go, and which ones it could not carry.
struct Carried<'a, 'b> {
    placed: &'a mut Placed<'b>,
    folder: &'a Path,
    inline: usize,
    /// Pictures left out, as the document wrote them.
    missing: Vec<String>,
    /// The five alert titles, in the window's language.
    alerts: &'a [String; 5],
}

impl Carried<'_, '_> {
    /// The Typst path for `url`, or none when the export cannot carry it.
    fn file(&mut self, url: &str) -> io::Result<Option<String>> {
        match self.placed.place(url)? {
            None => Ok(None),
            Some(src) if src.starts_with("data:") => self.inline(&src).map(Some),
            Some(src) => Ok(Some(decoded(&src))),
        }
    }

    /// A `data:` picture written into the scratch folder, so Typst can read it.
    fn inline(&mut self, url: &str) -> io::Result<String> {
        let lower = url.to_lowercase();
        let extension = match lower.split_once(";base64,").map(|(mime, _)| mime) {
            Some("data:image/png") => "png",
            Some("data:image/jpeg") => "jpg",
            Some("data:image/gif") => "gif",
            Some("data:image/svg+xml") => "svg",
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "a data picture is not base64 PNG, JPEG, GIF or SVG",
                ));
            }
        };
        // Base64 keeps its case: decode the address as written.
        let (_, data) = url.split_once(";base64,").expect("the mime check found it");
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data.trim())
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let dir = self.folder.join("files");
        fs::create_dir_all(&dir)?;
        let name = format!("inline-{}.{extension}", self.inline);
        self.inline += 1;
        fs::write(dir.join(&name), bytes)?;
        Ok(format!("files/{name}"))
    }
}

/// The export's blocks for the editor's.
fn blocks_of(nodes: &[MarkdownBlock], carried: &mut Carried) -> io::Result<Vec<Block>> {
    let mut blocks = Vec::new();
    for node in nodes {
        if let Some(block) = block_of(node, carried)? {
            blocks.push(block);
        }
    }
    Ok(blocks)
}

fn block_of(node: &MarkdownBlock, carried: &mut Carried) -> io::Result<Option<Block>> {
    Ok(Some(match node {
        MarkdownBlock::Paragraph(text) => Block::Paragraph(inline_of(text)),
        MarkdownBlock::Heading(level, text) => Block::Heading(*level, inline_of(text)),
        MarkdownBlock::Quote(body) => Block::Quote(blocks_of(body, carried)?),
        MarkdownBlock::Code { language, text, .. } => Block::Code {
            language: language.clone(),
            text: text.clone(),
        },
        MarkdownBlock::List { start, items } => {
            let mut listed = Vec::new();
            for (task, blocks) in items {
                listed.push(Item {
                    task: *task,
                    blocks: blocks_of(blocks, carried)?,
                });
            }
            Block::List {
                start: *start,
                items: listed,
            }
        }
        MarkdownBlock::Table { aligns, head, rows } => {
            let mut cells = Vec::new();
            for cell in head {
                cells.push(blocks_of(cell, carried)?);
            }
            let mut body = Vec::new();
            for row in rows {
                let mut kept = Vec::new();
                for cell in row {
                    kept.push(blocks_of(cell, carried)?);
                }
                body.push(kept);
            }
            Block::Table {
                aligns: aligns.iter().map(|align| align_of(*align)).collect(),
                head: cells,
                rows: body,
            }
        }
        MarkdownBlock::Rule => Block::Rule,
        MarkdownBlock::Math(tex) => Block::Math(tex.clone()),
        MarkdownBlock::Image { alt, url, link, .. } => match carried.file(url)? {
            Some(file) => Block::Image {
                alt: alt.clone(),
                url: file,
                link: link.clone(),
            },
            None => {
                carried.missing.push(url.clone());
                return Ok(None);
            }
        },
        MarkdownBlock::Footnote { label, body } => Block::Footnote {
            label: label.clone(),
            body: blocks_of(body, carried)?,
        },
        MarkdownBlock::Alert { kind, body } => Block::Alert {
            title: alert_title(*kind, carried.alerts).to_string(),
            body: blocks_of(body, carried)?,
        },
        MarkdownBlock::Toc(_) => Block::Toc,
        // Metadata, not a block; the export says so too.
        MarkdownBlock::FrontMatter { .. } => return Ok(None),
    }))
}

fn inline_of(inline: &Inline) -> Exported {
    Exported {
        text: inline.text.clone(),
        styles: inline
            .styles
            .iter()
            .filter_map(|(range, style)| style_of(*style).map(|style| (range.clone(), style)))
            .collect(),
        links: inline.links.clone(),
    }
}

fn style_of(style: Style) -> Option<ExportedStyle> {
    let style = match style {
        Style::Strong => ExportedStyle::Strong,
        Style::Emphasis => ExportedStyle::Emphasis,
        Style::Strike => ExportedStyle::Strike,
        Style::Code => ExportedStyle::Code,
        Style::Link => ExportedStyle::Link,
        Style::Math => ExportedStyle::Math,
        Style::Footnote => ExportedStyle::Footnote,
        Style::Underline => ExportedStyle::Underline,
        Style::Mark => ExportedStyle::Mark,
        // Their glyph is already ✓/×; the export's palette has no such style.
        Style::Pass | Style::Fail => return None,
    };
    Some(style)
}

fn align_of(align: AlignKind) -> Align {
    match align {
        AlignKind::Center => Align::Center,
        AlignKind::Right => Align::Right,
        AlignKind::Left | AlignKind::None => Align::Left,
    }
}

/// An alert's title in the window's language, as the page shows it.
fn alert_title(kind: AlertKind, alerts: &[String; 5]) -> &str {
    &alerts[match kind {
        AlertKind::Note => 0,
        AlertKind::Tip => 1,
        AlertKind::Important => 2,
        AlertKind::Warning => 3,
        AlertKind::Caution => 4,
    }]
}

#[cfg(test)]
mod tests {
    use super::{Carried, blocks_of, nodes_of};
    use green_export::Block;
    use std::path::{Path, PathBuf};

    use crate::output::Pictures;
    use crate::output::pictures::Placed;

    /// `text` mapped; the shelf keeps the copied pictures alive.
    fn mapped<'a>(
        text: &str,
        beside: Option<&'a Path>,
        folder: &'a Path,
        shelf: &'a Pictures,
    ) -> (Vec<Block>, Vec<String>, Placed<'a>) {
        let mut placed = Placed::new(beside, shelf);
        let alerts = ["Note", "Tip", "Important", "Warning", "Caution"].map(str::to_string);
        let mut carried = Carried {
            placed: &mut placed,
            folder,
            inline: 0,
            missing: Vec::new(),
            alerts: &alerts,
        };
        let blocks = blocks_of(&nodes_of(text), &mut carried).expect("the blocks");
        (blocks, carried.missing, placed)
    }

    #[test]
    fn the_document_maps_block_for_block() {
        let text = "# Title\n\nA **bold** word.\n\n- [x] done\n- two\n\n| a | b |\n| :- | -: |\n| 1 | 2 |\n\n```rust\nfn x() {}\n```\n\n$$\na+b\n$$\n\n> [!NOTE]\n> kept\n";
        let scratch = tempfile::tempdir().unwrap();
        let shelf = Pictures::Pdf(scratch.path().join("files"));
        let (blocks, missing, _) = mapped(text, None, scratch.path(), &shelf);
        assert!(missing.is_empty(), "{missing:?}");
        assert!(matches!(blocks[0], Block::Heading(1, _)));
        let Block::Paragraph(word) = &blocks[1] else {
            panic!("a paragraph: {blocks:?}");
        };
        assert_eq!(word.text, "A bold word.");
        assert_eq!(word.styles.len(), 1, "the bold run: {word:?}");
        let Block::List { start, items } = &blocks[2] else {
            panic!("a list: {blocks:?}");
        };
        assert_eq!(*start, None);
        assert_eq!(items[0].task, Some(true));
        let Block::Table { aligns, head, rows } = &blocks[3] else {
            panic!("a table: {blocks:?}");
        };
        assert_eq!(aligns, &[green_export::Align::Left, green_export::Align::Right]);
        assert_eq!(head.len(), 2);
        assert_eq!(rows.len(), 1);
        assert!(matches!(blocks[4], Block::Code { .. }));
        assert!(matches!(blocks[5], Block::Math(_)));
        let Block::Alert { title, body } = &blocks[6] else {
            panic!("an alert: {blocks:?}");
        };
        assert_eq!(title, "Note");
        assert_eq!(body.len(), 1);
    }

    #[test]
    fn front_matter_stays_out_and_a_missing_picture_is_said() {
        let scratch = tempfile::tempdir().unwrap();
        let shelf = Pictures::Pdf(scratch.path().join("files"));
        let (blocks, missing, _) = mapped("---\ntitle: x\n---\n\nbody\n\n![gone](nowhere.png)\n", None, scratch.path(), &shelf);
        assert_eq!(blocks.len(), 1, "{blocks:?}");
        assert_eq!(missing, vec!["nowhere.png".to_string()]);
    }

    #[test]
    fn a_picture_beside_the_document_is_copied_under_the_root() {
        let doc = tempfile::tempdir().unwrap();
        let pics = doc.path().join("pics");
        std::fs::create_dir_all(&pics).unwrap();
        std::fs::write(pics.join("icon.png"), b"not really a png").unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let shelf = Pictures::Pdf(scratch.path().join("files"));
        let (blocks, missing, placed) = mapped("![icon](pics/icon.png)\n", Some(doc.path()), scratch.path(), &shelf);
        assert!(missing.is_empty(), "{missing:?}");
        assert_eq!(placed.copied.len(), 1);
        let Block::Image { url, .. } = &blocks[0] else {
            panic!("a picture: {blocks:?}");
        };
        assert_eq!(url, "files/icon.png");
        assert!(scratch.path().join("files/icon.png").is_file());
    }

    #[test]
    fn a_picture_in_a_sibling_folder_is_copied_too() {
        let base = tempfile::tempdir().unwrap();
        let doc = base.path().join("notes");
        let pics = base.path().join("pics");
        std::fs::create_dir_all(&doc).unwrap();
        std::fs::create_dir_all(&pics).unwrap();
        std::fs::write(pics.join("a.png"), b"x").unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let shelf = Pictures::Pdf(scratch.path().join("files"));
        let (blocks, missing, _) = mapped("![a](../pics/a.png)\n", Some(&doc), scratch.path(), &shelf);
        assert!(missing.is_empty(), "{missing:?}");
        let Block::Image { url, .. } = &blocks[0] else {
            panic!("a picture: {blocks:?}");
        };
        assert_eq!(url, "files/a.png");
    }

    #[test]
    fn a_data_picture_is_written_where_typst_can_read_it() {
        let scratch = tempfile::tempdir().unwrap();
        let shelf = Pictures::Pdf(scratch.path().join("files"));
        let (blocks, missing, _) = mapped(
            "![dot](data:image/png;base64,iVBORw0KGgo=)\n",
            None,
            scratch.path(),
            &shelf,
        );
        assert!(missing.is_empty(), "{missing:?}");
        let Block::Image { url, .. } = &blocks[0] else {
            panic!("a picture: {blocks:?}");
        };
        assert_eq!(url, "files/inline-0.png");
        assert!(Path::new(&scratch.path().join(url)).is_file());
        assert_eq!(std::fs::read(scratch.path().join(url)).unwrap(), vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);
        assert_eq!(PathBuf::from(url).components().count(), 2);
    }
}
