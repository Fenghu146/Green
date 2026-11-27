//! The library path on its own: no executable, no PATH, no scratch `.typ`.

use std::{fs, path::Path};

use green_export::{Block, Inline, Lib, Style, export};

fn fonts() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/fonts")
        .canonicalize()
        .expect("the bundled fonts")
}

#[test]
fn the_assignment_note_exports_in_process() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root");
    let fonts = fonts();
    let formulas = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/job-formulas.txt"),
    )
    .expect("the formula fixture");
    let mut blocks = vec![Block::Heading(2, Inline::plain("作业：指令系统"))];
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
    assert_eq!(count, 28);

    let out = std::env::temp_dir().join(format!("green-lib-note-{}.pdf", std::process::id()));
    let compiler = Lib::new(&fonts, &root).expect("the library compiler");
    let export = export(&blocks, &out, &compiler, &fonts).expect("the export");
    let _ = fs::remove_file(&out);

    assert!(
        export.audit.bytes <= 2 * 1024 * 1024,
        "{}",
        export.audit.bytes
    );
    assert!(export.audit.pages >= 1);
    assert_eq!(export.audit.images, 0);
    assert!(export.audit.compressed);
    assert!(export.missing.is_empty(), "{:?}", export.missing);
    assert!(export.warnings.is_empty(), "{:?}", export.warnings);
    const BUNDLED: &[&str] = &[
        "NotoSansCJKsc-Regular",
        "NotoSansCJKsc-Bold",
        "NotoSansMath-Regular",
        "NotoSansMono-Regular",
    ];
    for font in &export.audit.fonts {
        assert!(BUNDLED.contains(&font.name.as_str()), "{font:?}");
    }
}

#[test]
fn the_same_source_exported_twice_is_the_same_file() {
    let fonts = fonts();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root");
    let blocks = vec![
        Block::Heading(1, Inline::plain("可复现")),
        Block::Math(r"\frac{1}{2}".into()),
    ];
    let compiler = Lib::new(&fonts, &root).expect("the library compiler");
    let first = std::env::temp_dir().join(format!("green-lib-repeat-a-{}.pdf", std::process::id()));
    let second =
        std::env::temp_dir().join(format!("green-lib-repeat-b-{}.pdf", std::process::id()));
    export(&blocks, &first, &compiler, &fonts).expect("the first export");
    export(&blocks, &second, &compiler, &fonts).expect("the second export");
    let (a, b) = (
        fs::read(&first).expect("the first file"),
        fs::read(&second).expect("the second file"),
    );
    let _ = fs::remove_file(&first);
    let _ = fs::remove_file(&second);
    assert_eq!(a, b, "deterministic PDF output (design §7)");
}

#[test]
fn one_refused_formula_still_degrades_to_tex() {
    let fonts = fonts();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root");
    let blocks = vec![
        Block::Math(r"x\in(-1,1]".into()),
        Block::Paragraph(Inline::plain("其余正文照常")),
        Block::Math(r"\frac{1}{2}".into()),
    ];
    let out = std::env::temp_dir().join(format!("green-lib-fallback-{}.pdf", std::process::id()));
    let compiler = Lib::new(&fonts, &root).expect("the library compiler");
    let export = export(&blocks, &out, &compiler, &fonts).expect("the export");
    let _ = fs::remove_file(&out);

    assert_eq!(export.formulas.len(), 1, "{:?}", export.formulas);
    assert!(export.source.contains("\\$x\\\\in"), "{}", export.source);
    assert!(export.source.contains("frac(1, 2)"), "{}", export.source);
    assert!(export.audit.pages >= 1);
}
