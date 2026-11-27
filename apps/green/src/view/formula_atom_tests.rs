//! 0006 验收：视觉模式下行内公式是不可分的原子，源码模式让位。

use std::time::{Duration, Instant};

use ely_gpui_component::{
    documents::{MarkdownDocument, MarkdownMode},
    forms::TextInput,
};
use gpui::{
    Entity, Focusable as _, InputEvent, KeyUpEvent, Modifiers, MouseButton, MouseUpEvent,
    TestAppContext, VisualTestContext, point, px,
};

const TEXT: &str = "通项 $a_n$ 后\n";

type Doc = Entity<MarkdownDocument>;

fn open<'a>(
    text: &str,
    mode: MarkdownMode,
    cx: &'a mut TestAppContext,
) -> (Doc, &'a mut VisualTestContext) {
    crate::output::tests::start_math_once();
    cx.update(ely_gpui_component::init_for_tests);
    let (doc, cx) =
        cx.add_window_view(move |window, cx| MarkdownDocument::new(text, mode, window, cx));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    (doc, cx)
}

fn shown(doc: &Doc, range: std::ops::Range<usize>, cx: &mut VisualTestContext) {
    doc.update_in(cx, |document, window, cx| {
        document.show_match(range, window, cx);
    });
    cx.run_until_parked();
}

fn caret(doc: &Doc, cx: &mut VisualTestContext) -> usize {
    let (line, column) = doc.read_with(cx, |document, cx| document.caret(cx));
    assert_eq!(line, 0, "one-line fixture");
    TEXT[..TEXT.find('\n').unwrap()]
        .char_indices()
        .nth(column)
        .map_or(TEXT.len() - 1, |(at, _)| at)
}

fn body(doc: &Doc, cx: &mut VisualTestContext) -> String {
    doc.read_with(cx, |document, cx| document.text(cx))
}

#[gpui::test]
fn left_and_right_cross_the_whole_formula(cx: &mut TestAppContext) {
    let (doc, cx) = open(TEXT, MarkdownMode::Visual, cx);
    shown(&doc, 8..8, cx);
    assert_eq!(caret(&doc, cx), 8, "the probe starts inside the atom");
    cx.simulate_keystrokes("right");
    cx.run_until_parked();
    assert_eq!(caret(&doc, cx), 12, "Right steps over it");
    cx.simulate_keystrokes("left");
    cx.run_until_parked();
    assert_eq!(caret(&doc, cx), 7, "Left steps back over it");
}

#[gpui::test]
fn backspace_and_delete_take_the_formula_in_one_undo(cx: &mut TestAppContext) {
    for (at, key) in [(7, "delete"), (12, "backspace")] {
        let (doc, cx) = open(TEXT, MarkdownMode::Visual, cx);
        shown(&doc, at..at, cx);
        cx.simulate_keystrokes(key);
        cx.run_until_parked();
        assert_eq!(body(&doc, cx), "通项  后\n", "{key} at {at}");
        cx.simulate_keystrokes("secondary-z");
        cx.run_until_parked();
        assert_eq!(body(&doc, cx), TEXT, "one undo restores it");
    }
}

#[gpui::test]
fn a_selection_half_a_formula_deletes_it_all(cx: &mut TestAppContext) {
    let (doc, cx) = open(TEXT, MarkdownMode::Visual, cx);
    shown(&doc, 8..9, cx);
    assert_eq!(
        doc.read_with(cx, |document, cx| document.selected(cx)),
        "a",
        "half the formula is selected"
    );
    cx.simulate_keystrokes("backspace");
    cx.run_until_parked();
    assert_eq!(body(&doc, cx), "通项  后\n");
}

#[gpui::test]
fn a_second_press_selects_the_whole_formula(cx: &mut TestAppContext) {
    let (doc, cx) = open(TEXT, MarkdownMode::Visual, cx);
    shown(&doc, 8..8, cx);
    cx.update(|window, cx| {
        window.dispatch_event(
            MouseUpEvent {
                position: point(px(20.0), px(20.0)),
                modifiers: Modifiers::none(),
                button: MouseButton::Left,
                click_count: 2,
            }
            .to_platform_input(),
            cx,
        );
    });
    cx.run_until_parked();
    assert_eq!(
        doc.read_with(cx, |document, cx| document.selected(cx)),
        "$a_n$",
        "the word press takes the whole atom"
    );
}

#[gpui::test]
fn typing_one_character_keeps_the_formula_legal(cx: &mut TestAppContext) {
    let (doc, cx) = open(TEXT, MarkdownMode::Visual, cx);
    shown(&doc, 8..9, cx);
    cx.simulate_input("x");
    cx.run_until_parked();
    assert_eq!(body(&doc, cx), "通项 $x_n$ 后\n", "one key accumulates inside");
}

#[gpui::test]
fn pasting_over_a_formula_replaces_the_whole_atom(cx: &mut TestAppContext) {
    let (doc, cx) = open(TEXT, MarkdownMode::Visual, cx);
    shown(&doc, 8..9, cx);
    let applied = doc.update(cx, |document, cx| document.insert("b^2", cx));
    cx.run_until_parked();
    assert!(applied);
    assert_eq!(body(&doc, cx), "通项 $b^2$ 后\n");
}

#[gpui::test]
fn an_unsupported_command_is_refused_without_broken_tex(cx: &mut TestAppContext) {
    let (doc, cx) = open(TEXT, MarkdownMode::Visual, cx);
    shown(&doc, 8..9, cx);
    let applied = doc.update(cx, |document, cx| document.insert("\\nope", cx));
    cx.run_until_parked();
    assert!(applied, "the insert itself is routed, its formula is refused");
    assert_eq!(body(&doc, cx), TEXT, "no illegal formula is left behind");
}

#[gpui::test]
fn source_mode_keeps_literal_formula_edits(cx: &mut TestAppContext) {
    let (doc, cx) = open(TEXT, MarkdownMode::Source, cx);
    let applied = doc.update(cx, |document, cx| {
        document.replace(vec![(8..9, "z".into())], cx)
    });
    cx.run_until_parked();
    assert!(applied);
    assert_eq!(body(&doc, cx), "通项 $z_n$ 后\n", "the caret may edit TeX literally");
}

/// Nearest rank, as the fold benchmark reads it.
fn percentile(mut samples: Vec<Duration>, p: f64) -> Duration {
    samples.sort();
    let at = (samples.len() as f64 * p).ceil() as usize;
    samples[at.clamp(1, samples.len()) - 1]
}

#[gpui::test]
#[ignore = "benchmark: run by hand on a quiet machine, e.g. cargo test --release -p green formula_k2p -- --ignored --nocapture --test-threads=1"]
fn formula_k2p_of_thirty_typed_characters(cx: &mut TestAppContext) {
    let (doc, cx) = open(TEXT, MarkdownMode::Visual, cx);
    shown(&doc, 8..8, cx);
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    let mut samples = Vec::new();
    let began = Instant::now();
    for _ in 0..30 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        let start = Instant::now();
        cx.simulate_input("x");
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        samples.push(start.elapsed());
    }
    let total = began.elapsed();
    let (p50, p95, p99) = (
        percentile(samples.clone(), 0.50),
        percentile(samples.clone(), 0.95),
        percentile(samples, 0.99),
    );
    println!(
        "formula k2p: 30 typed characters, total {total:?}, p50 {p50:?}, p95 {p95:?}, p99 {p99:?}, {}",
        crate::bench::machine()
    );
    assert!(p95 <= Duration::from_millis(16), "K2P p95 {p95:?}");
}

const VIEW_TEXT: &str = "通项 $a_n$ 后\n\n下一段落\n";

fn view_opened(cx: &mut TestAppContext) -> (Entity<super::MarkView>, &mut VisualTestContext) {
    crate::output::tests::start_math_once();
    super::tests::open(VIEW_TEXT, cx)
}

fn place_caret(view: &Entity<super::MarkView>, cx: &mut VisualTestContext) {
    view.update_in(cx, |view, window, cx| {
        view.document()
            .update(cx, |document, cx| document.show_match(7..7, window, cx));
    });
    cx.run_until_parked();
}

fn press_formula(view: &Entity<super::MarkView>, cx: &mut VisualTestContext) {
    place_caret(view, cx);
    cx.update(|window, cx| {
        window.dispatch_event(
            MouseUpEvent {
                position: point(px(20.0), px(20.0)),
                modifiers: Modifiers::none(),
                button: MouseButton::Left,
                click_count: 1,
            }
            .to_platform_input(),
            cx,
        );
    });
    cx.run_until_parked();
}

fn view_body(view: &Entity<super::MarkView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |view, cx| view.document().read(cx).text(cx))
}

#[gpui::test]
fn a_press_on_a_formula_opens_its_source(cx: &mut TestAppContext) {
    let (view, cx) = view_opened(cx);
    press_formula(&view, cx);
    let source = view.read_with(cx, |view, cx| {
        view.formula
            .as_ref()
            .expect("the source editor opened")
            .1
            .read(cx)
            .text()
            .to_string()
    });
    assert_eq!(source, "$a_n$");
}

#[gpui::test]
fn enter_reflows_and_one_undo_takes_the_formula_back(cx: &mut TestAppContext) {
    let (view, cx) = view_opened(cx);
    press_formula(&view, cx);
    view.update(cx, |view, cx| {
        let field = view.formula.as_ref().expect("open").1.clone();
        field.update(cx, |field, cx| field.set_text("$b^2$", cx));
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.simulate_event(KeyUpEvent {
        keystroke: gpui::Keystroke::parse("enter").expect("a key"),
    });
    cx.run_until_parked();
    assert_eq!(view_body(&view, cx), "通项 $b^2$ 后\n\n下一段落\n");
    assert!(view.read_with(cx, |view, _| view.formula.is_none()));
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(view_body(&view, cx), VIEW_TEXT, "one undo restores the formula");
}

#[gpui::test]
fn escape_leaves_the_document_and_focus_where_it_was(cx: &mut TestAppContext) {
    let (view, cx) = view_opened(cx);
    place_caret(&view, cx);
    let before = cx.update(|window, cx| window.focused(cx));
    cx.update(|window, cx| {
        window.dispatch_event(
            MouseUpEvent {
                position: point(px(20.0), px(20.0)),
                modifiers: Modifiers::none(),
                button: MouseButton::Left,
                click_count: 1,
            }
            .to_platform_input(),
            cx,
        );
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(view.read_with(cx, |view, _| view.formula.is_none()));
    assert_eq!(view_body(&view, cx), VIEW_TEXT);
    let home = cx.update(|window, cx| {
        let view = view.read(cx);
        let here = view.focus.clone();
        let there = view.document().read(cx).focus_handle(cx);
        here.is_focused(window) || there.is_focused(window)
    });
    assert!(before.is_some(), "something held focus before the overlay");
    assert!(home, "focus came back to the view, not the dialog");
}

#[gpui::test]
fn the_open_overlay_does_not_relay_out_other_blocks(cx: &mut TestAppContext) {
    let (view, cx) = view_opened(cx);
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    place_caret(&view, cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    for _ in 0..8 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    let name = Box::leak("markdown-block-1".to_string().into_boxed_str());
    view.update(cx, |view, cx| {
        assert!(view.commit_formula(cx), "the press lands on a formula");
    });
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    for _ in 0..8 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    let before = cx.debug_bounds(name);
    assert!(before.is_some(), "the second block is drawn");
    view.update_in(cx, |view, window, cx| view.show_formula(window, cx));
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    let after = cx.debug_bounds(name);
    assert!(before.is_some(), "the second block is drawn");
    assert_eq!(before, after, "the rest of the page keeps its frame");
}

#[gpui::test]
fn an_ordinary_text_input_has_no_formula_guard(cx: &mut TestAppContext) {
    cx.update(ely_gpui_component::init_for_tests);
    let (field, cx) = cx.add_window_view(|window, cx| TextInput::new(window, cx));
    field.update(cx, |input, cx| {
        input.set_text(TEXT.trim_end_matches('\n'), cx);
        input.select(8..9, cx);
    });
    cx.run_until_parked();
    let focus = field.read_with(cx, |input, cx| input.focus_handle(cx));
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_input("x");
    cx.run_until_parked();
    assert_eq!(
        field.read_with(cx, |input, _| input.text().to_string()),
        "通项 $x_n$ 后",
        "a plain field replaces only what the caret selected"
    );
}

#[gpui::test]
fn input_events_still_flow_from_a_plain_field(cx: &mut TestAppContext) {
    cx.update(ely_gpui_component::init_for_tests);
    let (field, cx) = cx.add_window_view(|window, cx| TextInput::new(window, cx));
    cx.run_until_parked();
    let focus = field.read_with(cx, |input, cx| input.focus_handle(cx));
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_input("plain");
    cx.run_until_parked();
    assert_eq!(field.read_with(cx, |input, _| input.text().to_string()), "plain");
}

