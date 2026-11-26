//! Folding's numbers: a 5万字 file's folds stay inside 16 ms, and lay no
//! text out again (design §5.4, §9). A benchmark, not a gate: it is `#[ignore]`d
//! and run by hand when the machine is quiet.

use std::time::{Duration, Instant};

use gpui::{Entity, TestAppContext};

use super::MarkView;
use super::tests::open;

/// 5 万字 of Chinese and English under headings, section by section.
fn long_text() -> String {
    let mut text = String::new();
    let body = "这是一段用于测量折叠与展开性能的中文正文，包含标点、English words 与数字 12345。";
    for part in 0..40 {
        text.push_str(&format!("# Part {part}\n\n"));
        for step in 0..5 {
            text.push_str(&format!("## Step {part}.{step}\n\n"));
            text.push_str(&body.repeat(4));
            text.push_str("\n\n");
            for note in 0..3 {
                text.push_str(&format!("### Note {part}.{step}.{note}\n\n"));
                text.push_str("短小的一节，只有这一行正文。\n\n");
            }
        }
    }
    text
}

/// Nearest rank: the p95 of `samples`.
fn p95(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    let at = (samples.len() as f64 * 0.95).ceil() as usize;
    samples[at.clamp(1, samples.len()) - 1]
}

/// The heading to fold: a small section in the middle of the file.
fn target(view: &Entity<MarkView>, cx: &mut TestAppContext) -> (usize, usize) {
    let headings = view.read_with(cx, |view, cx| view.outline(cx));
    let found = headings
        .iter()
        .find(|heading| heading.title == "Note 39.4.2")
        .expect("the last note");
    (found.at, found.line)
}

/// One fold or unfold, as a press runs it.
fn toggle(view: &Entity<MarkView>, at: usize, cx: &mut gpui::VisualTestContext) {
    view.update_in(cx, |view, window, cx| {
        let document = view.document().clone();
        document.update(cx, |document, cx| {
            document.toggle_fold_at(at, window, cx);
        });
    });
}

#[gpui::test]
#[ignore = "benchmark: run by hand on a quiet machine, e.g. cargo test --release -p green --features shape-count -- --ignored --nocapture --test-threads=1"]
fn folding_fifty_thousand_characters_keeps_the_goal(cx: &mut TestAppContext) {
    let (view, cx) = open(&long_text(), cx);
    let (at, line) = target(&view, cx);
    view.update_in(cx, |view, window, cx| view.go(at, line, window, cx));
    // Warm: every block in view is drawn once before the first fold.
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    // One fold and unfold draws whatever the first toggle brings into view;
    // the 30 below are the measurement.
    for _ in 0..2 {
        toggle(&view, at, cx);
        cx.run_until_parked();
    }
    let mut times = Vec::new();
    #[cfg(feature = "shape-count")]
    let mut shapes = Vec::new();
    for _ in 0..30 {
        // A press lands in a drawn frame, so the fold it makes starts from the
        // frame before it; without one here the test measures a state no press
        // can reach.
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        #[cfg(feature = "shape-count")]
        gpui::WindowTextSystem::reset_shaped_lines();
        let start = Instant::now();
        toggle(&view, at, cx);
        cx.run_until_parked();
        times.push(start.elapsed());
        // The frame the fold causes: where a line it hid would lay out again.
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        #[cfg(feature = "shape-count")]
        shapes.push(gpui::WindowTextSystem::shaped_lines());
    }
    let p95 = p95(times.clone());
    println!(
        "folding: 30 toggles, p95 {:?}, max {:?}, {}",
        p95,
        times.iter().max().copied().unwrap_or_default(),
        crate::bench::machine(),
    );
    #[cfg(feature = "shape-count")]
    {
        println!("folding: shapes per toggle {shapes:?}");
        assert_eq!(
            shapes.iter().sum::<usize>(),
            0,
            "folding and unfolding must lay no text out again: {shapes:?}"
        );
    }
    // Only a release build holds design §9; a debug build's number is printed,
    // not gated, since it runs an order of magnitude slower.
    if !cfg!(debug_assertions) {
        let goal = std::time::Duration::from_millis(16);
        assert!(p95 <= goal, "folding's p95 is {p95:?}, over {goal:?}");
    }
}
