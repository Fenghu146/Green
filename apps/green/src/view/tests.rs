use std::sync::{Arc, Mutex};

use gpui::{AppContext as _, Entity, TestAppContext, VisualTestContext};

use super::{MarkView, Mode, Reported};
use crate::engine::{Text, mark};
use crate::text::Source;

pub(super) fn open<'a>(
    text: &str,
    cx: &'a mut TestAppContext,
) -> (Entity<MarkView>, &'a mut VisualTestContext) {
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|cx| {
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install_as("en-US", cx);
    });
    let source = Source::decode(text.as_bytes().to_vec()).expect("reads");
    let saved = mark(&source.text);
    let doc = Arc::new(Mutex::new(Text::new(source)));
    let (view, cx) = cx.add_window_view(move |window, cx| MarkView::new(doc, saved, window, cx));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    (view, cx)
}

#[gpui::test]
fn typing_reaches_the_text_the_shell_saves(cx: &mut TestAppContext) {
    let (view, cx) = open("# A\n\nB\n", cx);
    let outline: Vec<(usize, String, usize)> = view.read_with(cx, |view, cx| {
        view.outline(cx)
            .into_iter()
            .map(|heading| (usize::from(heading.depth), heading.title, heading.line))
            .collect()
    });
    assert_eq!(outline, [(1, "A".to_string(), 0)]);
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_input("x");
    let (bytes, modified, changed) = view.read_with(cx, |view, _| {
        (view.bytes(), view.modified(), view.changed())
    });
    assert_eq!(bytes, "x# A\n\nB\n".len());
    assert!(modified && changed > 0);
}

fn current(view: &Entity<MarkView>, cx: &mut VisualTestContext) -> String {
    view.read_with(cx, |view, _| {
        view.doc.lock().expect("the text lock").current.clone()
    })
}

fn focused(view: &Entity<MarkView>, cx: &mut VisualTestContext) {
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
}

#[gpui::test]
fn switching_modes_writes_no_byte(cx: &mut TestAppContext) {
    let (view, cx) = open("# A\r\n\r\n* b\r\n", cx);
    for mode in [Mode::Source, Mode::Preview, Mode::Visual, Mode::Source] {
        view.update_in(cx, |view, window, cx| view.switch(mode, window, cx));
        cx.run_until_parked();
    }
    let written = view.read_with(cx, |view, _| {
        let doc = view.doc.lock().expect("the text lock");
        doc.source.encode(&doc.current)
    });
    assert_eq!(written, b"# A\r\n\r\n* b\r\n");
    let (modified, changed) = view.read_with(cx, |view, _| (view.modified(), view.changed()));
    assert!(!modified && changed == 0);
}

#[gpui::test]
fn undo_reaches_across_modes(cx: &mut TestAppContext) {
    let (view, cx) = open("# A\n", cx);
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    focused(&view, cx);
    cx.simulate_input("x");
    assert_eq!(current(&view, cx), "x# A\n");
    view.update_in(cx, |view, window, cx| view.switch(Mode::Visual, window, cx));
    focused(&view, cx);
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(
        current(&view, cx),
        "# A\n",
        "the source edit undone in visual"
    );
}

#[gpui::test]
fn undo_brings_back_a_text_emptied_in_source(cx: &mut TestAppContext) {
    let (view, cx) = open("Seed\n", cx);
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    view.update(cx, |view, cx| view.adopt(String::new(), cx));
    view.update_in(cx, |view, window, cx| view.switch(Mode::Visual, window, cx));
    focused(&view, cx);
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(current(&view, cx), "Seed\n");
}

#[gpui::test]
fn a_new_empty_file_takes_typing_in_visual(cx: &mut TestAppContext) {
    let (view, cx) = open("", cx);
    assert_eq!(view.read_with(cx, |view, _| view.mode), Mode::Visual);
    focused(&view, cx);
    cx.simulate_input("Hi");
    assert_eq!(current(&view, cx), "Hi");
    assert!(view.read_with(cx, |view, _| view.modified()));
}

#[gpui::test]
fn a_long_file_shows_once_its_blocks_are_read(cx: &mut TestAppContext) {
    let long = "para\n\n".repeat(40_000);
    let (view, cx) = open(&long, cx);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    cx.run_until_parked();
    let (shown, read) = view.read_with(cx, |view, cx| {
        (view.shown, view.document.read(cx).read_now(cx))
    });
    assert!(shown && read, "shown with its blocks read");
}

#[gpui::test]
fn a_refresh_is_saved_text_and_keeps_no_undo(cx: &mut TestAppContext) {
    let (view, cx) = open("one\n", cx);
    view.update(cx, |view, cx| view.adopt("edited\n".into(), cx));
    cx.run_until_parked();
    assert!(view.read_with(cx, |view, _| view.modified()));
    let disk = Source::decode(b"from disk\n".to_vec()).expect("reads");
    view.update(cx, |view, cx| view.refresh(disk, cx));
    cx.run_until_parked();
    assert_eq!(current(&view, cx), "from disk\n");
    assert!(!view.read_with(cx, |view, _| view.modified()));
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-z");
    cx.run_until_parked();
    assert_eq!(current(&view, cx), "from disk\n", "undo stops at the disk");
}

#[gpui::test]
fn the_shell_hears_a_load_it_subscribed_to(cx: &mut TestAppContext) {
    cx.update(ely_gpui_component::init_for_tests);
    cx.update(|cx| {
        ely_ui::settings::install_defaults(cx);
        ely_ui::i18n::install_as("en-US", cx);
    });
    let heard = std::rc::Rc::new(std::cell::Cell::new(0));
    let (count, held) = (
        heard.clone(),
        std::rc::Rc::new(std::cell::RefCell::new(None)),
    );
    let holder = held.clone();
    let (_root, cx) = cx.add_window_view(move |window, cx| {
        let source = Source::decode(b"# Opened\n".to_vec()).expect("reads");
        let saved = mark(&source.text);
        let doc = Arc::new(Mutex::new(Text::new(source)));
        let view = cx.new(|cx| MarkView::new(doc, saved, window, cx));
        // As the shell: subscribed after the view is made.
        let count = count.clone();
        cx.subscribe(&view, move |_, _, _: &Reported, _| {
            count.set(count.get() + 1)
        })
        .detach();
        holder.replace(Some(view));
        gpui::Empty
    });
    cx.run_until_parked();
    assert!(heard.get() > 0);
}

#[gpui::test]
fn a_text_grown_past_the_limit_goes_to_source(cx: &mut TestAppContext) {
    let (view, cx) = open("# Old\n", cx);
    let long = "x".repeat(ely_gpui_component::documents::LARGEST_READ + 1);
    view.update(cx, |view, cx| view.adopt(long, cx));
    cx.run_until_parked();
    assert!(view.read_with(cx, |view, _| view.large()));
    assert_eq!(view.read_with(cx, |view, _| view.mode), Mode::Source);
    view.update_in(cx, |view, window, cx| {
        view.switch(Mode::Preview, window, cx)
    });
    assert_eq!(view.read_with(cx, |view, _| view.mode), Mode::Source);
    let outline = view.read_with(cx, |view, cx| view.outline(cx));
    assert!(outline.is_empty(), "no stale heading: {outline:?}");
}

#[gpui::test]
fn a_large_file_stays_in_source(cx: &mut TestAppContext) {
    let large = ely_gpui_component::documents::LARGEST_READ / 2 + 1;
    let (view, cx) = open(&"a\n".repeat(large), cx);
    assert_eq!(view.read_with(cx, |view, _| view.mode), Mode::Source);
    view.update_in(cx, |view, window, cx| view.switch(Mode::Visual, window, cx));
    assert_eq!(view.read_with(cx, |view, _| view.mode), Mode::Source);
}

#[gpui::test]
fn an_edit_keeps_the_rest_of_the_file_byte_for_byte(cx: &mut TestAppContext) {
    let bytes = b"\xEF\xBB\xBF# A\r\n\r\nB\r\n".to_vec();
    let (view, cx) = open(std::str::from_utf8(&bytes).expect("UTF-8"), cx);
    view.update_in(cx, |view, window, cx| view.switch(Mode::Source, window, cx));
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.run_until_parked();
    cx.simulate_input("x");
    let written = view.read_with(cx, |view, _| {
        let doc = view.doc.lock().expect("the text lock");
        doc.source.encode(&doc.current)
    });
    assert_eq!(written, b"\xEF\xBB\xBFx# A\r\n\r\nB\r\n");
}

#[gpui::test]
fn a_web_link_opens_outside(cx: &mut TestAppContext) {
    let (view, cx) = open("x\n", cx);
    view.update(cx, |view, cx| view.follow("https://ely.dev", cx));
    assert_eq!(cx.opened_url().as_deref(), Some("https://ely.dev/"), "in the URL Standard's form");
}

#[gpui::test]
fn a_file_link_is_refused_and_said(cx: &mut TestAppContext) {
    let (view, cx) = open("x\n", cx);
    let said = std::rc::Rc::new(std::cell::Cell::new(false));
    let heard = said.clone();
    cx.update(|_, cx| {
        cx.subscribe(&view, move |_, _: &super::Failed, _| heard.set(true))
            .detach()
    });
    view.update(cx, |view, cx| view.follow("file:///etc/hosts", cx));
    cx.run_until_parked();
    assert!(said.get() && cx.opened_url().is_none());
}

/// A GIF whose frame never decodes.
fn broken_gif() -> Vec<u8> {
    let gif =
        "47494638396101000100800000000000ffffff21f90401000000002c0000000001000100000202ffff003b";
    (0..gif.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&gif[at..at + 2], 16).expect("hex"))
        .collect()
}

#[gpui::test]
fn a_broken_gif_ends_and_its_error_names_no_path(cx: &mut TestAppContext) {
    let (_view, cx) = open("x\n", cx);
    let folder = std::env::temp_dir().join(format!("ely-marker-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("a folder");
    let path = folder.join("broken.gif");
    std::fs::write(&path, broken_gif()).expect("the gif");
    let resource = gpui::Resource::Path(path.into());
    let loaded = loop {
        let asked = cx.update(|_, cx| cx.fetch_asset::<gpui::ImageAssetLoader>(&resource));
        if let Some(loaded) = asked {
            break loaded;
        }
        cx.run_until_parked();
    };
    std::fs::remove_dir_all(&folder).expect("cleaned");
    let error = loaded.expect_err("a broken frame fails the load");
    assert!(!format!("{error:?}").contains("ely-marker"), "{error:?}");
}

#[gpui::test]
fn a_broken_inline_gif_ends(cx: &mut TestAppContext) {
    let (_view, cx) = open("x\n", cx);
    let renderer = cx.update(|_, cx| cx.svg_renderer());
    let image = gpui::Image::from_bytes(gpui::ImageFormat::Gif, broken_gif());
    assert!(image.to_image_data(renderer).is_err());
}

#[gpui::test]
fn a_wheel_beside_the_text_scrolls_it(cx: &mut TestAppContext) {
    use gpui::{ScrollDelta, ScrollEvent, point, px};
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    let path = files.path().join("long.md");
    std::fs::write(&path, (0..300).map(|n| format!("Paragraph {n}.

")).collect::<String>()).expect("a file");
    let (desk, cx) = crate::desk::tests::start(cx, data.path());
    crate::desk::tests::open(&desk, &path, cx);
    let view = desk.read_with(cx, |desk, _| desk.active_view().expect("a tab"));
    cx.run_until_parked();
    // Left of the first block is margin, not text.
    let block = cx.debug_bounds("markdown-block-0").expect("the first block");
    let top = |cx: &mut VisualTestContext| cx.update(|_, app| view.read(app).place(app).top);
    let before = top(cx);
    cx.simulate_event(ScrollEvent {
        position: point(block.left() - px(20.), block.center().y),
        delta: ScrollDelta::Pixels(point(px(0.), px(-600.))),
        ..Default::default()
    });
    cx.run_until_parked();
    assert!(top(cx) > before, "the margin's wheel moved the text");
}

#[gpui::test]
fn a_folded_heading_hides_its_section_and_leaves_the_text(cx: &mut TestAppContext) {
    let text = "# A\n\nalpha\n\n## B\n\nbeta\n\n# C\n\ngamma\n";
    let (view, cx) = open(text, cx);
    view.update_in(cx, |view, window, cx| {
        let document = view.document().clone();
        document.update(cx, |document, cx| {
            assert_eq!(document.toggle_fold_at(0, window, cx), Some(true));
            assert!(document.hidden(1) && document.hidden(2) && document.hidden(3));
            assert!(!document.hidden(4), "the next level-one heading stays");
            assert_eq!(document.folded_keys(), ["1|0|A"]);
            // A second press unfolds it.
            assert_eq!(document.toggle_fold_at(0, window, cx), Some(false));
            assert!(!document.hidden(1) && !document.hidden(3));
            assert!(document.folded_keys().is_empty());
        });
    });
    let kept = view.read_with(cx, |view, _| {
        view.doc.lock().expect("the text lock").current.clone()
    });
    assert_eq!(kept, text, "folding never touches the text");
}

#[gpui::test]
fn a_fold_hides_every_level_below_it_down_to_a_same_level_heading(cx: &mut TestAppContext) {
    // One block a heading, one its text: 0 # A, 1 ## B, 2 alpha, 3 #### D,
    // 4 deep, 5 ### E, 6 less deep, 7 ## F, 8 next, 9 # G, 10 gamma.
    let text = "# A\n\n## B\n\nalpha\n\n#### D\n\ndeep\n\n### E\n\nless deep\n\n## F\n\nnext\n\n# G\n\ngamma\n";
    let (view, cx) = open(text, cx);
    view.update_in(cx, |view, window, cx| {
        let document = view.document().clone();
        document.update(cx, |document, cx| {
            assert_eq!(document.toggle_fold(3, window, cx), Some(true), "#### D");
            assert!(document.hidden(4), "its text folds");
            assert!(!document.hidden(5), "a higher level stays");
            assert_eq!(document.toggle_fold(3, window, cx), Some(false));
            // ## B folds the #### D under it and every level between.
            assert_eq!(document.toggle_fold(1, window, cx), Some(true), "## B");
            assert!(document.hidden(2) && document.hidden(3) && document.hidden(5));
            assert!(!document.hidden(7), "## F stays");
            assert!(!document.hidden(9), "# G stays");
        });
    });
}

#[gpui::test]
fn a_fold_survives_a_mode_switch(cx: &mut TestAppContext) {
    let (view, cx) = open("# A\n\nalpha\n", cx);
    view.update_in(cx, |view, window, cx| {
        let document = view.document().clone();
        document.update(cx, |document, cx| {
            assert_eq!(document.toggle_fold_at(0, window, cx), Some(true));
        });
        view.switch(Mode::Source, window, cx);
        view.switch(Mode::Visual, window, cx);
    });
    cx.run_until_parked();
    let hidden = view.read_with(cx, |view, cx| {
        let document = view.document().read(cx);
        (document.hidden(1), document.folded_keys())
    });
    assert_eq!(hidden, (true, vec!["1|0|A".to_string()]));
}

#[gpui::test]
fn the_outline_is_a_tree_of_headings_with_plain_titles(cx: &mut TestAppContext) {
    let text = "# A $x^2$ and **bold**\n\na paragraph\n\n- a list item\n\n## B\n\n### C\n\nmore\n";
    let (view, cx) = open(text, cx);
    let tree = view.read_with(cx, |view, cx| view.outline(cx));
    let shape: Vec<(u8, &str, Option<usize>, bool)> = tree
        .iter()
        .map(|heading| (heading.depth, heading.title.as_str(), heading.parent, heading.opens))
        .collect();
    assert_eq!(
        shape,
        [
            (1, "A x^2 and bold", None, true),
            (2, "B", Some(0), true),
            (3, "C", Some(1), false),
        ],
        "headings only, marks gone, levels a tree"
    );
}
