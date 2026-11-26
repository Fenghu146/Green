use super::{Unsupported, cached, latex_to_typst};

fn typst(source: &str) -> String {
    latex_to_typst(source).unwrap_or_else(|errors| panic!("{source} did not convert: {errors:?}"))
}

#[test]
fn every_formula_in_the_job_converts() {
    let formulas = include_str!("../tests/fixtures/job-formulas.txt");
    let lines: Vec<&str> = formulas.lines().filter(|line| !line.trim().is_empty()).collect();
    assert!(lines.len() >= 26, "the job's formulas are fewer than before: {}", lines.len());
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
    assert_eq!(typst("B052H-0008H=B04AH"), "B 0 5 2 H-0 0 0 8 H=B 0 4 A H");
    // Numbers stay numbers.
    assert_eq!(typst("2^4=16"), "2^4=16");
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
    assert_eq!(typst("x^{n-1}"), "x^(n-1)");
    assert!(typst("\\boxed{x}").starts_with("#box("));
}

#[test]
fn left_and_right_keep_their_brackets() {
    assert_eq!(
        typst("\\left(\\frac{2}{5}\\right)^{n-1}"),
        "(frac(2, 5))^(n-1)"
    );
}

#[test]
fn an_unknown_command_is_named_and_pointed_at() {
    let errors = latex_to_typst("a+\\mathbb{R}").expect_err("not whitelisted");
    assert_eq!(errors, [Unsupported { name: "mathbb".into(), at: 2 }]);
    let said = errors[0].to_string();
    assert!(said.contains("\\mathbb") && said.contains("白名单"), "{said}");
}

#[test]
fn the_same_source_converts_once() {
    let first = cached("\\frac{1}{2}").expect("converts");
    let second = cached("\\frac{1}{2}").expect("converts");
    assert_eq!(first, second);
    assert_eq!(cached("\\frac{1}{2}"), Ok("frac(1, 2)".to_string()));
}
