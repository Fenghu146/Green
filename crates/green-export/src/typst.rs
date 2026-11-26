//! Blocks as Typst markup: the source a PDF is compiled from.

use anyhow::{Result, anyhow};
use green_math::latex_to_typst;

use crate::doc::{Align, Block, Inline, Item, Style};

/// What every export page shares: the A4 sheet, the bundled CJK font, and
/// justified Chinese paragraphs.
pub(crate) const PREAMBLE: &str = "\
#set page(paper: \"a4\", margin: (x: 2.2cm, y: 2.4cm))
#set text(font: (\"Noto Sans CJK SC\",), size: 10.5pt, lang: \"zh\")
#set par(justify: true, leading: 0.95em, spacing: 1.15em)
#show raw: set text(font: (\"Noto Sans Mono\", \"Noto Sans CJK SC\"))
// The marks the source spells as emoji: brand deep green passes, red fails.
#let mark-ok = rgb(\"#2e7d46\")
#let mark-no = rgb(\"#b42318\")
// The same faces the editor draws with: math as Noto Sans Math, CJK beside it.
#show math.equation: set text(font: (\"Noto Sans Math\", \"Noto Sans CJK SC\"))
#set heading(numbering: none)
#show heading: set text(weight: \"bold\")
#show heading.where(level: 1): set text(size: 1.6em)
#show heading.where(level: 2): set text(size: 1.35em)
#show heading.where(level: 3): set text(size: 1.15em)

";

/// The box's frame is a style, not a converter decision: its one definition
/// comes from `green-math`, the same function the editor's preamble calls.
pub(crate) const GBOX_STROKE: &str = "#8a8a8a";

pub(crate) fn preamble() -> String {
    format!("{PREAMBLE}{}\n\n", green_math::gbox_rule(GBOX_STROKE))
}

/// What a formula must pass before it enters the document: conversion and,
/// when an export supplies one, a real Typst compile.
type Check<'a> = dyn Fn(&str, &str) -> Result<()> + 'a;

fn checked(tex: &str, check: Option<&Check<'_>>) -> Result<()> {
    let markup = formula(tex)?;
    if let Some(check) = check {
        check(tex, &markup)?;
    }
    Ok(())
}

/// `blocks` as one Typst document, converting formulas without compiling them.
/// Export uses [`source_checked`], which also keeps the ones Typst refuses.
pub fn source(blocks: &[Block]) -> Result<(String, Vec<(String, String)>)> {
    source_checked(blocks, None)
}

/// `blocks` as one Typst document, and every formula conversion or compile
/// failure kept as its TeX (§7.8: one formula must not take the document down).
pub fn source_checked(blocks: &[Block], check: Option<&Check<'_>>) -> Result<(String, Vec<(String, String)>)> {
    let mut kept = Vec::new();
    let blocks = settled(blocks, &mut kept, check);
    let mut out = preamble();
    blocks_into(&mut out, &blocks, "")?;
    let mut notes = Vec::new();
    collect_notes(&blocks, &mut notes);
    if !notes.is_empty() {
        out.push_str("#v(1.5em)\n#line(length: 100%, stroke: 0.4pt)\n");
        out.push_str("#text(size: 0.85em)[\n");
        for (label, body) in notes {
            out.push_str(&format!("[{label}] "));
            let mut inner = String::new();
            blocks_into(&mut inner, body, "")?;
            out.push_str(inner.trim_end_matches('\n'));
            out.push_str("\n\n");
        }
        out.push_str("]\n");
    }
    Ok((out, kept))
}

/// `blocks` with every formula the converter refuses kept as text: a block
/// formula as its literal `$…$`, an inline one as its TeX with the math mark
/// taken off. Each is recorded with why.
fn settled(
    blocks: &[Block],
    kept: &mut Vec<(String, String)>,
    check: Option<&Check<'_>>,
) -> Vec<Block> {
    blocks
        .iter()
        .map(|block| match block {
            Block::Math(tex) => match checked(tex, check) {
                Ok(_) => block.clone(),
                Err(error) => {
                    kept.push((tex.clone(), format!("{error:#}")));
                    Block::Paragraph(Inline::plain(format!("${tex}$")))
                }
            },
            Block::Paragraph(text) => Block::Paragraph(settled_inline(text, kept, check)),
            Block::Heading(level, text) => {
                Block::Heading(*level, settled_inline(text, kept, check))
            }
            Block::Quote(body) => Block::Quote(settled(body, kept, check)),
            Block::Alert { title, body } => Block::Alert {
                title: title.clone(),
                body: settled(body, kept, check),
            },
            Block::Footnote { label, body } => Block::Footnote {
                label: label.clone(),
                body: settled(body, kept, check),
            },
            Block::List { start, items } => Block::List {
                start: *start,
                items: items
                    .iter()
                    .map(|item| Item {
                        task: item.task,
                        blocks: settled(&item.blocks, kept, check),
                    })
                    .collect(),
            },
            Block::Table { aligns, head, rows } => Block::Table {
                aligns: aligns.clone(),
                head: head
                    .iter()
                    .map(|cell| settled(cell, kept, check))
                    .collect(),
                rows: rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|cell| settled(cell, kept, check))
                            .collect()
                    })
                    .collect(),
            },
            other => other.clone(),
        })
        .collect()
}

/// `text` without the math marks the converter refuses; the TeX stays as text.
fn settled_inline(
    text: &Inline,
    kept: &mut Vec<(String, String)>,
    check: Option<&Check<'_>>,
) -> Inline {
    let mut out = text.clone();
    out.styles.retain(|(range, style)| match style {
        Style::Math => {
            let tex = &text.text[range.clone()];
            match checked(tex, check) {
                Ok(_) => true,
                Err(error) => {
                    kept.push((tex.to_string(), format!("{error:#}")));
                    false
                }
            }
        }
        _ => true,
    });
    out
}

/// One block's markup, every line prefixed with `indent`.
fn blocks_into(out: &mut String, blocks: &[Block], indent: &str) -> Result<()> {
    for block in blocks {
        block_into(out, block, indent)?;
    }
    Ok(())
}

fn block_into(out: &mut String, block: &Block, indent: &str) -> Result<()> {
    match block {
        Block::Paragraph(text) => {
            out.push_str(indent);
            out.push_str(&line(text)?);
            out.push_str("\n\n");
        }
        Block::Heading(level, text) => {
            out.push_str(indent);
            out.push_str(&"=".repeat((*level).clamp(1, 6) as usize));
            out.push(' ');
            out.push_str(&line(text)?);
            out.push_str("\n\n");
        }
        Block::Quote(body) | Block::Alert { body, .. } => {
            out.push_str(indent);
            out.push_str("#quote(block: true)[\n");
            if let Block::Alert { title, .. } = block {
                out.push_str(&format!("{indent}  *{}*\n\n", escape(title)));
            }
            blocks_into(out, body, &format!("{indent}  "))?;
            out.push_str(indent);
            out.push_str("]\n\n");
        }
        Block::Code { language, text } => {
            let fence = "`".repeat(longest_run(text, '`') + 3);
            out.push_str(indent);
            out.push_str(&fence);
            out.push_str(language.as_deref().unwrap_or(""));
            out.push('\n');
            for line in text.lines() {
                out.push_str(indent);
                out.push_str(line);
                out.push('\n');
            }
            out.push_str(indent);
            out.push_str(&fence);
            out.push_str("\n\n");
        }
        Block::List { start, items } => list_into(out, *start, items, indent)?,
        Block::Table { aligns, head, rows } => table_into(out, aligns, head, rows, indent)?,
        Block::Rule => {
            out.push_str(indent);
            out.push_str("#line(length: 100%, stroke: 0.5pt)\n\n");
        }
        Block::Math(tex) => {
            out.push_str(indent);
            out.push_str(&format!("$ {} $\n\n", formula(tex)?));
        }
        Block::Image { url, link, .. } => {
            let image = format!("#image(\"{}\", width: 100%)", literal(url));
            out.push_str(indent);
            match link {
                Some(link) => out.push_str(&format!("#link(\"{}\")[{image}]\n\n", literal(link))),
                None => {
                    out.push_str(&image);
                    out.push_str("\n\n");
                }
            }
        }
        // Collected and listed at the document's end.
        Block::Footnote { .. } => {}
        Block::Toc => {
            out.push_str(indent);
            out.push_str("#outline()\n\n");
        }
    }
    Ok(())
}

fn list_into(
    out: &mut String,
    start: Option<u64>,
    items: &[crate::doc::Item],
    indent: &str,
) -> Result<()> {
    let inner = format!("{indent}  ");
    for (index, item) in items.iter().enumerate() {
        let mut body = String::new();
        blocks_into(&mut body, &item.blocks, &inner)?;
        out.push_str(indent);
        match start {
            Some(first) => out.push_str(&format!("{}. ", first + index as u64)),
            None => out.push_str("- "),
        }
        match item.task {
            Some(true) => out.push_str("#sym.checkmark "),
            Some(false) => out.push_str("#sym.square.stroked "),
            None => {}
        }
        let body = body.strip_prefix(&inner).unwrap_or(&body);
        out.push_str(body.trim_end_matches('\n'));
        out.push('\n');
    }
    out.push('\n');
    Ok(())
}

fn table_into(
    out: &mut String,
    aligns: &[Align],
    head: &[Vec<Block>],
    rows: &[Vec<Vec<Block>>],
    indent: &str,
) -> Result<()> {
    let columns = aligns
        .len()
        .max(head.len())
        .max(rows.iter().map(Vec::len).max().unwrap_or(0));
    if columns == 0 {
        return Ok(());
    }
    let inner = format!("{indent}  ");
    out.push_str(indent);
    out.push_str(&format!("#table(\n{inner}columns: {columns},\n"));
    if !aligns.is_empty() {
        let list: Vec<&str> = aligns
            .iter()
            .map(|align| match align {
                Align::Left => "left",
                Align::Center => "center",
                Align::Right => "right",
            })
            .collect();
        out.push_str(&format!("{inner}align: ({}),\n", list.join(", ")));
    }
    if !head.is_empty() {
        out.push_str(&format!("{inner}table.header("));
        for cell in head {
            out.push_str(&format!("{}, ", cell_markup(cell)?));
        }
        out.push_str("),\n");
    }
    for row in rows {
        for column in 0..columns {
            let cell = row.get(column).map_or(&[][..], Vec::as_slice);
            out.push_str(&format!("{inner}{},\n", cell_markup(cell)?));
        }
    }
    out.push_str(indent);
    out.push_str(")\n\n");
    Ok(())
}

/// One table cell, padded to the row's columns.
fn cell_markup(blocks: &[Block]) -> Result<String> {
    let mut inner = String::new();
    blocks_into(&mut inner, blocks, "")?;
    Ok(format!("[{}]", inner.trim_end_matches('\n')))
}

/// A paragraph's inline markup, one line.
fn line(inline: &Inline) -> Result<String> {
    let mut out = String::new();
    inline_into(&mut out, inline)?;
    Ok(guard_ordered_start(out))
}

/// Wraps that can surround a stretch of text, outermost first.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Wrap {
    Mark,
    Strong,
    Emphasis,
    Strike,
    Underline,
    Link(String),
    Footnote,
    Math,
    Code,
}

/// `inline` as Typst markup, its styles nested by their kind.
fn inline_into(out: &mut String, inline: &Inline) -> Result<()> {
    let text = inline.text.as_str();
    let mut cuts = vec![0, text.len()];
    for (range, _) in &inline.styles {
        cuts.push(range.start.min(text.len()));
        cuts.push(range.end.min(text.len()));
    }
    for (range, _) in &inline.links {
        cuts.push(range.start.min(text.len()));
        cuts.push(range.end.min(text.len()));
    }
    cuts.sort_unstable();
    cuts.dedup();
    let mut open: Vec<Wrap> = Vec::new();
    for pair in cuts.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        if from >= to {
            continue;
        }
        let wanted = wraps_at(inline, from, to);
        let shared = open
            .iter()
            .zip(&wanted)
            .take_while(|(mine, want)| mine == want)
            .count();
        while open.len() > shared {
            push_close(out, open.pop().expect("an open wrap"));
        }
        for wrap in &wanted[shared..] {
            push_open(out, wrap);
            open.push(wrap.clone());
        }
        let piece = &text[from..to];
        if wanted.contains(&Wrap::Math) {
            out.push_str(&format!("${}$", formula(piece)?));
        } else if wanted.contains(&Wrap::Code) {
            let fence = "`".repeat(longest_run(piece, '`') + 1);
            out.push_str(&format!("{fence}{piece}{fence}"));
        } else {
            out.push_str(&escape(piece));
        }
    }
    while let Some(wrap) = open.pop() {
        push_close(out, wrap);
    }
    Ok(())
}

fn wraps_at(inline: &Inline, from: usize, to: usize) -> Vec<Wrap> {
    let covers = |range: &std::ops::Range<usize>| range.start <= from && to <= range.end;
    let mut wraps: Vec<Wrap> = inline
        .styles
        .iter()
        .filter(|(range, _)| covers(range))
        .filter_map(|(_, style)| match style {
            Style::Mark => Some(Wrap::Mark),
            Style::Strong => Some(Wrap::Strong),
            Style::Emphasis => Some(Wrap::Emphasis),
            Style::Strike => Some(Wrap::Strike),
            Style::Underline => Some(Wrap::Underline),
            Style::Code => Some(Wrap::Code),
            Style::Math => Some(Wrap::Math),
            Style::Footnote => Some(Wrap::Footnote),
            // The address rides in `links`, below.
            Style::Link => None,
        })
        .collect();
    wraps.extend(
        inline
            .links
            .iter()
            .filter(|(range, _)| covers(range))
            .map(|(_, url)| Wrap::Link(url.clone())),
    );
    wraps.sort();
    wraps.dedup();
    wraps
}

fn push_open(out: &mut String, wrap: &Wrap) {
    match wrap {
        Wrap::Mark => out.push_str("#highlight["),
        Wrap::Strong => out.push('*'),
        Wrap::Emphasis => out.push('_'),
        Wrap::Strike => out.push_str("#strike["),
        Wrap::Underline => out.push_str("#underline["),
        Wrap::Link(url) => out.push_str(&format!("#link(\"{}\")[", literal(url))),
        Wrap::Footnote => out.push_str("#super["),
        Wrap::Math | Wrap::Code => {}
    }
}

fn push_close(out: &mut String, wrap: Wrap) {
    match wrap {
        Wrap::Strong => out.push('*'),
        Wrap::Emphasis => out.push('_'),
        Wrap::Math | Wrap::Code => {}
        _ => out.push(']'),
    }
}

/// `tex` as Typst math, or every command the whitelist does not cover.
pub fn formula(tex: &str) -> Result<String> {
    let tex = tex.replace('\u{200b}', "");
    latex_to_typst(&tex).map_err(|unsupported| {
        let names: Vec<String> = unsupported.iter().map(ToString::to_string).collect();
        anyhow!("formula `{tex}`: {}", names.join("; "))
    })
}

/// The marks the source spells as emoji, and the markup the export draws
/// instead: no bundled face has U+2705, U+274C or even `✗`, so a green `✓` and
/// a red `×` carry the meaning. Colours live in the preamble.
pub(crate) const MARKS: &[(char, &str)] = &[
    ('✅', "#text(fill: mark-ok)[#sym.checkmark]"),
    ('❌', "#text(fill: mark-no)[#sym.times]"),
];

/// `text` as markup, its punctuation literal.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        // The notes glue formulas with U+200B; it must not reach the PDF's
        // text layer, where copying it would carry an invisible character.
        if ch == '\u{200b}' {
            continue;
        }
        // A mark the bundled faces cannot draw, as the symbol they can.
        if let Some((_, markup)) = MARKS.iter().find(|(mark, _)| *mark == ch) {
            out.push_str(markup);
            continue;
        }
        if matches!(
            ch,
            '\\' | '#'
                | '$'
                | '*'
                | '_'
                | '`'
                | '['
                | ']'
                | '<'
                | '>'
                | '@'
                | '~'
                | '-'
                | '+'
                | '='
                | '/'
                | '\''
                | '"'
        ) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

/// `text` as a Typst string's insides.
fn literal(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// A line that begins `2.` or `3)` would read as a numbered list; escape the
/// punctuation so the paragraph keeps its words.
fn guard_ordered_start(line: String) -> String {
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let after = line[digits..].chars().next();
    match digits > 0 && matches!(after, Some('.' | ')')) {
        true => format!("{}\\{}", &line[..digits], &line[digits..]),
        false => line,
    }
}

fn longest_run(text: &str, needle: char) -> usize {
    let (mut longest, mut run) = (0, 0);
    for ch in text.chars() {
        match ch == needle {
            true => {
                run += 1;
                longest = longest.max(run);
            }
            false => run = 0,
        }
    }
    longest
}

/// Every footnote definition, in reading order.
fn collect_notes<'a>(blocks: &'a [Block], into: &mut Vec<(&'a str, &'a [Block])>) {
    for block in blocks {
        match block {
            Block::Footnote { label, body } => into.push((label, body)),
            Block::Quote(body) | Block::Alert { body, .. } => collect_notes(body, into),
            Block::List { items, .. } => {
                for item in items {
                    collect_notes(&item.blocks, into);
                }
            }
            Block::Table { head, rows, .. } => {
                for cell in head {
                    collect_notes(cell, into);
                }
                for row in rows {
                    for cell in row {
                        collect_notes(cell, into);
                    }
                }
            }
            _ => {}
        }
    }
}
