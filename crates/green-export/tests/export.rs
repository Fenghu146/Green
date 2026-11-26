//! End to end: the 28 formulas of the user's assignment note compiled by the
//! Typst CLI, under the limits of design §7. Skipped when `GREEN_TYPST` is
//! unset, since the executable is not part of the repository.

use std::path::{Path, PathBuf};
use std::time::Duration;

use green_export::{Block, Cli, Inline, export};

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
    let export = export(&blocks, &out, &Cli::new(exe, fonts, root)).expect("the export");
    let _ = std::fs::remove_file(&out);

    assert!(
        export.audit.bytes <= 2 * 1024 * 1024,
        "{} bytes",
        export.audit.bytes
    );
    assert_eq!(export.audit.images, 0);
    assert!(export.audit.compressed);
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
    ];
    for font in &export.audit.fonts {
        assert!(
            BUNDLED.contains(&font.name.as_str()),
            "{} is not a bundled face: {:?}",
            font.name,
            export.audit.fonts
        );
    }
    assert!(
        export
            .audit
            .fonts
            .iter()
            .any(|font| font.name == "NotoSansMath-Regular"),
        "math fell back to another font: {:?}",
        export.audit.fonts
    );
}
