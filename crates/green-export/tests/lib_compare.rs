//! The in-process compiler and the executable, on the same documents.
//!
//! `GREEN_TYPST` selects the CLI for the comparison; without it the test says
//! so and skips, since the executable is not part of the repository. The
//! library path needs no executable and is covered by `lib_export.rs`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use green_export::{Align, Block, Cli, Inline, Item, Lib, Style, export};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root")
}

/// Plain text, with `$…$` spans marked as math: enough of the editor's tree
/// for both compilers to receive exactly the same Typst source.
fn inline(text: &str) -> Inline {
    let mut plain = String::new();
    let mut styles = Vec::new();
    let rest = text.trim();
    let mut open = None;
    for ch in rest.chars() {
        if ch == '$' {
            match open.take() {
                Some(start) => styles.push((start..plain.len(), Style::Math)),
                None => open = Some(plain.len()),
            }
            continue;
        }
        plain.push(ch);
    }
    Inline {
        text: plain,
        styles,
        links: Vec::new(),
    }
}

fn cells(line: &str) -> Vec<Inline> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|cell| inline(cell.trim()))
        .collect()
}

/// A small reader for the fixtures' shapes: headings, fences, display math,
/// tables, lists and paragraphs. Both compilers get the blocks it returns.
fn blocks(file: &Path) -> Vec<Block> {
    let text = fs::read_to_string(file).expect("a readable fixture");
    let mut out = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(fence) = trimmed.strip_prefix("```") {
            let mut code = Vec::new();
            for next in lines.by_ref() {
                if next.trim_start().starts_with("```") {
                    break;
                }
                code.push(next.to_string());
            }
            out.push(Block::Code {
                language: fence.trim().to_owned().into(),
                text: code.join("\n"),
            });
            continue;
        }
        if trimmed.starts_with("$$") {
            let mut math = vec![trimmed.trim_start_matches('$').trim().to_owned()];
            for next in lines.by_ref() {
                if next.trim_end().ends_with("$$") {
                    math.push(next.trim_end().trim_end_matches('$').trim().to_owned());
                    break;
                }
                math.push(next.to_string());
            }
            out.push(Block::Math(math.join(" ").trim().to_owned()));
            continue;
        }
        let hashes = trimmed.chars().take_while(|ch| *ch == '#').count();
        if hashes > 0 && trimmed[hashes..].starts_with(' ') {
            out.push(Block::Heading(
                hashes.min(6) as u8,
                inline(&trimmed[hashes + 1..]),
            ));
            continue;
        }
        if trimmed.starts_with('|') {
            let mut rows = vec![cells(trimmed)];
            while lines
                .peek()
                .is_some_and(|next| next.trim().starts_with('|'))
            {
                let next = lines.next().expect("a peeked row");
                if next.chars().all(|ch| matches!(ch, '|' | '-' | ':' | ' ')) {
                    continue;
                }
                rows.push(cells(next));
            }
            let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
            let head = rows.remove(0);
            out.push(Block::Table {
                aligns: vec![Align::Left; columns],
                head: head
                    .into_iter()
                    .map(|cell| vec![Block::Paragraph(cell)])
                    .collect(),
                rows: rows
                    .into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|cell| vec![Block::Paragraph(cell)])
                            .collect()
                    })
                    .collect(),
            });
            continue;
        }
        let list = trimmed.strip_prefix("- ").map(|body| (None, body));
        let list = list.or_else(|| {
            let digits = trimmed.chars().take_while(char::is_ascii_digit).count();
            match digits > 0 && trimmed[digits..].starts_with(". ") {
                true => trimmed[..digits]
                    .parse::<u64>()
                    .ok()
                    .map(|start| (Some(start), &trimmed[digits + 2..])),
                false => None,
            }
        });
        if let Some((start, body)) = list {
            let mut items = vec![Item {
                task: None,
                blocks: vec![Block::Paragraph(inline(body))],
            }];
            while let Some(next) = lines.peek() {
                let next = next.trim();
                if let Some(body) = next.strip_prefix("- ") {
                    lines.next();
                    items.push(Item {
                        task: None,
                        blocks: vec![Block::Paragraph(inline(body))],
                    });
                } else {
                    break;
                }
            }
            out.push(Block::List { start, items });
            continue;
        }
        out.push(Block::Paragraph(inline(trimmed)));
    }
    out
}

#[test]
fn the_library_and_the_executable_agree() {
    let Some(exe) = std::env::var_os("GREEN_TYPST") else {
        eprintln!("GREEN_TYPST is unset; the CLI comparison did not run");
        return;
    };
    let root = root();
    let fonts = root.join("assets/fonts");
    let cases = [
        ("instruction-set", "fixtures/real/skeletons/指令系统.md"),
        (
            "number-representation",
            "fixtures/real/skeletons/数的表示与运算｜完整复习笔记.md",
        ),
        ("cpu", "fixtures/real/skeletons/中央处理器CPU.md"),
        ("homework", "fixtures/perf/homework-28-formulas.md"),
    ];
    for (index, (name, file)) in cases.iter().enumerate() {
        let blocks = blocks(&root.join(file));
        let lib_out = std::env::temp_dir().join(format!("green-lib-{index}.pdf"));
        let cli_out = std::env::temp_dir().join(format!("green-cli-{index}.pdf"));
        let lib = export(
            &blocks,
            &lib_out,
            &Lib::new(&fonts, &root).expect("a library compiler"),
            &fonts,
        )
        .expect("the in-process export");
        let cli = export(&blocks, &cli_out, &Cli::new(&exe, &fonts, &root), &fonts)
            .expect("the executable export");
        println!(
            "{name}: lib pages={} bytes={} elapsed={:?} | cli pages={} bytes={} elapsed={:?}",
            lib.audit.pages,
            lib.audit.bytes,
            lib.elapsed,
            cli.audit.pages,
            cli.audit.bytes,
            cli.elapsed,
        );
        assert_eq!(lib.audit.pages, cli.audit.pages, "{name}: pages");
        assert_eq!(lib.audit.fonts, cli.audit.fonts, "{name}: embedded fonts");
        let difference = lib.audit.bytes.abs_diff(cli.audit.bytes);
        assert!(difference <= 512, "{name}: {difference} bytes apart");
        if std::env::var_os("GREEN_KEEP_PDFS").is_none() {
            let _ = fs::remove_file(&lib_out);
            let _ = fs::remove_file(&cli_out);
        }
    }
}
