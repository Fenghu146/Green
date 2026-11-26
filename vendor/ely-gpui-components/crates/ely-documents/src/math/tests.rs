//! The formula source, its key and its failures: the geometry lives in
//! Green's own suite, which has the bundled faces.

use super::*;

fn formula(source: &str, display: bool) -> Formula {
    Formula {
        source: source.to_string().into(),
        display,
        rgba: 0x1122_33ff,
        border: 0x8a8a_8aff,
        font: 1200,
        scale: 200,
    }
}

#[test]
fn an_inline_formula_sets_in_a_box_and_a_block_formula_does_not() {
    let inline = document_source(&formula("x^2", false), "x^2");
    assert!(inline.contains("#box[$x^2$]"), "{inline}");
    let block = document_source(&formula("x^2", true), "x^2");
    assert!(block.contains("$ x^2 $"), "{block}");
    // `\boxed` arrives as markup: it is pasted, never wrapped in `$`.
    let boxed = document_source(&formula(r"\boxed{x}", true), "#gbox[$x$]");
    assert!(boxed.ends_with("#gbox[$x$]\n"), "{boxed}");
}

#[test]
fn the_preamble_carries_the_faces_the_colours_and_the_box_style() {
    let source = document_source(&formula("x", false), "x");
    assert!(source.contains("size: 12pt"), "{source}");
    assert!(source.contains("fill: rgb(\"#112233\")"), "{source}");
    assert!(
        source.contains(green_math::gbox_rule("#8a8a8a").as_str()),
        "{source}"
    );
    assert!(source.contains("top-edge: \"bounds\""), "{source}");
    assert!(source.contains("fill: none"), "no white behind it: {source}");
}

#[test]
fn every_colour_and_size_is_part_of_the_key() {
    let base = formula("x", false);
    let mut other = base.clone();
    other.rgba = 0xff_ffff_ff;
    assert_ne!(base, other, "the theme's colour");
    let mut other = base.clone();
    other.border = 0x0000_00ff;
    assert_ne!(base, other, "the box's stroke");
    let mut other = base.clone();
    other.font += 1;
    assert_ne!(base, other, "the size");
    let mut other = base.clone();
    other.scale += 1;
    assert_ne!(base, other, "the window's scale");
    assert_eq!(base, formula("x", false), "the same formula");
}

#[test]
fn a_command_without_a_conversion_says_which() {
    let Err(error) = Latex::parse("x", r"\notacommand x") else {
        panic!("an unsupported command parses")
    };
    assert!(error.to_string().contains("notacommand"), "{error}");
    assert!(Latex::parse("x", r"A = \pi r^2").is_ok(), "a covered one");
}

#[test]
fn a_failed_formula_shows_as_its_tex() {
    assert_eq!(literal(r"a_n=(2/5)^{n-1}"), r"$a_n=(2/5)^{n-1}$");
}

#[test]
fn the_cache_is_bounded() {
    // The bound is what keeps a long session's pictures from growing.
    assert!((1..=1024).contains(&KEPT), "{KEPT}");
}
