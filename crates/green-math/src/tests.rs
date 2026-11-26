use std::path::PathBuf;
use std::process::Command;

use super::{SYMBOLS, Unsupported, cached, latex_to_typst};

fn typst(source: &str) -> String {
    latex_to_typst(source).unwrap_or_else(|errors| panic!("{source} did not convert: {errors:?}"))
}

#[test]
fn every_formula_in_the_job_converts() {
    let formulas = include_str!("../tests/fixtures/job-formulas.txt");
    let lines: Vec<&str> = formulas
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert!(
        lines.len() >= 26,
        "the job's formulas are fewer than before: {}",
        lines.len()
    );
    for line in &lines {
        match latex_to_typst(line) {
            Ok(done) => assert!(!done.is_empty(), "{line} converted to nothing"),
            Err(errors) => panic!("{line} failed: {errors:?}"),
        }
    }
}

#[test]
fn a_name_is_its_letters_not_a_word() {
    // B052H is five italic characters in LaTeX, never a quoted word.
    assert_eq!(
        typst("B052H-0008H=B04AH"),
        "B 0 5 2 H - 0 0 0 8 H = B 0 4 A H"
    );
    // Numbers stay numbers.
    assert_eq!(typst("2^4=16"), "2^4 = 16");
    assert_eq!(typst("0000\\ 0001"), "0000 space 0001");
}

#[test]
fn the_whitelist_maps_as_the_design_table_says() {
    assert_eq!(typst("\\frac{2}{5}"), "frac(2, 5)");
    assert_eq!(typst("\\sqrt[3]{x}"), "root(3, x)");
    assert_eq!(typst("\\text{不溢出}"), "\"不溢出\"");
    assert_eq!(typst("\\leftarrow"), "arrow.l");
    assert_eq!(typst("\\times"), "times");
    assert_eq!(typst("\\cdot"), "dot.op");
    assert_eq!(typst("\\bmod"), "mod");
    assert_eq!(typst("\\le"), "lt.eq");
    assert_eq!(typst("x^{2}"), "x^2");
    assert_eq!(typst("x^{n-1}"), "x^(n - 1)");
    assert_eq!(typst("\\nabla"), "nabla");
    assert_eq!(typst("\\rho"), "rho");
    assert_eq!(typst("\\varepsilon"), "epsilon.alt");
    assert_eq!(typst("\\vec{E}"), "arrow(E)");
    assert!(typst("\\boxed{x}").starts_with("#box("));
}

#[test]
fn left_and_right_keep_their_brackets() {
    assert_eq!(
        typst("\\left(\\frac{2}{5}\\right)^{n-1}"),
        "(frac(2, 5))^(n - 1)"
    );
}

#[test]
fn an_unknown_command_is_named_and_pointed_at() {
    let errors = latex_to_typst("a+\\mathbb{R}").expect_err("not whitelisted");
    assert_eq!(
        errors,
        [Unsupported {
            name: "mathbb".into(),
            at: 2
        }]
    );
    let said = errors[0].to_string();
    assert!(
        said.contains("\\mathbb") && said.contains("whitelist"),
        "{said}"
    );
}

#[test]
fn the_same_source_converts_once() {
    let first = cached("\\frac{1}{2}").expect("converts");
    let second = cached("\\frac{1}{2}").expect("converts");
    assert_eq!(first, second);
    assert_eq!(cached("\\frac{1}{2}"), Ok("frac(1, 2)".to_string()));
}

#[test]
fn bare_cjk_is_a_string_not_a_name() {
    // `R型` is one name to Typst, and an unknown one; the CJK half is text.
    assert_eq!(typst("R型"), "R \"型\"");
    // ASCII names keep splitting one letter at a time beside it.
    assert_eq!(typst("B052H型"), "B 0 5 2 H \"型\"");
    let boxed = typst("\\boxed{R型\\ 16种；I+J\\ 128种；寄存器最多4个}");
    assert!(boxed.contains("\"型\""), "{boxed}");
    assert!(boxed.contains("\"种；寄存器最多\""), "{boxed}");
}

#[test]
fn a_comparison_never_turns_into_an_arrow() {
    // `<-` is Typst's left arrow and `<=>` its double arrow: name the symbol.
    assert_eq!(typst("-65136<-32768"), "-65136 lt -32768");
    assert_eq!(typst("a<=b"), "a lt.eq b");
    assert_eq!(typst("a>=b"), "a gt.eq b");
    assert_eq!(typst("a>b"), "a gt b");
    assert_eq!(typst("x=-y"), "x = -y");
}

/// The invariant this converter owes its callers: whatever the whitelist
/// covers converts to Typst that parses. A `typst` executable comes from
/// `GREEN_TYPST`; without one the real compile is skipped, not faked.
#[test]
fn every_covered_sample_compiles_in_typst() {
    let Some(exe) = std::env::var_os("GREEN_TYPST") else {
        eprintln!("GREEN_TYPST is unset; the real compile did not run");
        return;
    };
    let mut samples: Vec<String> = SYMBOLS
        .iter()
        .map(|(name, _)| format!("\\{name} x"))
        .collect();
    samples.extend(
        [
            "\\frac{2}{5}",
            "\\sqrt[3]{x}",
            "\\text{不溢出}",
            "\\boxed{x}",
            "\\vec{E}",
            "\\nabla \\cdot \\vec{E} = \\frac{\\rho}{\\varepsilon_0}",
            "\\left(\\frac{2}{5}\\right)^{n-1}",
            "\\{ \\} \\% \\_ \\& \\# \\$ \\\\",
            "R型",
            "B052H型",
            "\\boxed{R型\\ 16种；I+J\\ 128种；寄存器最多4个}",
            "-65136<-32768",
            "a<=b",
            "a>=b",
            "a>b",
            "x=-y",
            "x--y",
            "寄存器最多4个",
            "（也可采用I型）",
        ]
        .map(String::from),
    );
    samples.extend(
        include_str!("../tests/fixtures/job-formulas.txt")
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(String::from),
    );

    let mut document = String::from("#set text(font: (\"Noto Sans CJK SC\",), size: 10pt)\n");
    for sample in &samples {
        document.push_str(&format!("$ {} $\n\n", typst(sample)));
    }

    let directory = std::env::temp_dir().join(format!("green-math-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a scratch directory");
    let source = directory.join("samples.typ");
    let pdf = directory.join("samples.pdf");
    std::fs::write(&source, &document).expect("the samples");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root");
    let output = Command::new(exe)
        .arg("compile")
        .arg("--ignore-system-fonts")
        .arg("--font-path")
        .arg(root.join("assets/fonts"))
        .arg("--root")
        .arg(&directory)
        .arg(&source)
        .arg(&pdf)
        .output()
        .expect("the typst executable");
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    if output.status.success() && said.is_empty() {
        let _ = std::fs::remove_dir_all(&directory);
    } else if !output.status.success() {
        panic!(
            "typst could not parse the converted samples (kept at {}):\n{said}",
            source.display()
        );
    }
    for refused in ["unknown variable", "syntax error"] {
        assert!(!said.contains(refused), "typst said {refused}:\n{said}");
    }
    assert!(
        samples.len() > 60,
        "the samples shrank to {}",
        samples.len()
    );
}
