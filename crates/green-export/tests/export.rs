//! End to end: the 28 formulas of the user's assignment note compiled by the
//! Typst CLI, under the limits of design §7. Skipped when `GREEN_TYPST` is
//! unset, since the executable is not part of the repository.

use std::path::{Path, PathBuf};
use std::time::Duration;

use green_export::{Block, Cli, Inline, Style, Warning, export};

fn typst() -> Option<PathBuf> {
    std::env::var_os("GREEN_TYPST").map(PathBuf::from)
}

#[test]
fn the_assignment_note_stays_under_two_megabytes() {
    let Some(exe) = typst() else {
        eprintln!("GREEN_TYPST is unset; the end-to-end export did not run");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root");
    let fonts = root.join("assets/fonts");
    let formulas = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/job-formulas.txt"),
    )
    .expect("the formula fixture");

    let mut blocks = vec![Block::Heading(2, Inline::plain("作业：指令系统"))];
    // The note's own code spans and a fenced block, so the code face is in the PDF.
    blocks.push(Block::Paragraph(Inline::styled(
        "操作由 op1 区分",
        vec![(9..12, Style::Code)],
    )));
    blocks.push(Block::Code {
        language: Some("rust".into()),
        text: "let x = 1;".into(),
    });
    let mut count = 0;
    for formula in formulas
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
    {
        blocks.push(Block::Math(formula.to_string()));
        count += 1;
    }
    assert_eq!(count, 28, "the note's 28 formulas");

    let out = std::env::temp_dir().join(format!("green-export-note-{}.pdf", std::process::id()));
    let export = export(&blocks, &out, &Cli::new(exe, &fonts, &root), &fonts).expect("the export");
    let _ = std::fs::remove_file(&out);

    assert!(
        export.audit.bytes <= 2 * 1024 * 1024,
        "{} bytes",
        export.audit.bytes
    );
    assert_eq!(export.audit.images, 0);
    assert!(export.audit.compressed);
    assert!(export.missing.is_empty(), "{:?}", export.missing);
    assert!(export.warnings.is_empty(), "{:?}", export.warnings);
    assert!(
        export.elapsed <= Duration::from_secs(10),
        "{:?}",
        export.elapsed
    );
    // No font drift: every face in the PDF comes from assets/fonts, and math is
    // ours (Noto Sans Math), not whatever Typst ships inside the executable.
    const BUNDLED: &[&str] = &[
        "NotoSansCJKsc-Regular",
        "NotoSansCJKsc-Bold",
        "NotoSansMath-Regular",
        "NotoSansMono-Regular",
    ];
    for font in &export.audit.fonts {
        assert!(
            BUNDLED.contains(&font.name.as_str()),
            "{} is not a bundled face: {:?}",
            font.name,
            export.audit.fonts
        );
    }
    for family in ["NotoSansMath-Regular", "NotoSansMono-Regular"] {
        assert!(
            export.audit.fonts.iter().any(|font| font.name == family),
            "{family} is missing; something fell back: {:?}",
            export.audit.fonts
        );
    }
}

/// The counterexample: a character none of the four faces has is named with
/// where it is, and the rest of the document still exports.
#[test]
fn a_character_no_bundled_font_has_is_reported_and_the_rest_still_exports() {
    let Some(exe) = typst() else {
        eprintln!("GREEN_TYPST is unset; the counterexample did not run");
        return;
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root");
    let fonts = root.join("assets/fonts");
    let blocks = vec![
        Block::Paragraph(Inline::plain("这一段正常")),
        Block::Paragraph(Inline::plain("表情 😀 在第二段")),
    ];
    let out = std::env::temp_dir().join(format!("green-export-tofu-{}.pdf", std::process::id()));
    let export = export(&blocks, &out, &Cli::new(exe, &fonts, &root), &fonts).expect("the export");
    let _ = std::fs::remove_file(&out);

    assert_eq!(export.missing.len(), 1, "{:?}", export.missing);
    assert_eq!(export.missing[0].ch, '😀');
    assert_eq!(export.missing[0].describe(), "'😀' (U+1F600) at block 2");
    // The rest went out: pages, bytes, and one warning naming the character.
    assert!(export.audit.pages >= 1);
    assert!(export.audit.bytes > 0);
    match export.warnings.as_slice() {
        [Warning::MissingGlyphs { found }, rest @ ..] => {
            assert_eq!(found, &["'😀' (U+1F600) at block 2".to_string()]);
            assert!(rest.is_empty(), "{rest:?}");
        }
        other => panic!("expected the missing-glyph warning first: {other:?}"),
    }
}
