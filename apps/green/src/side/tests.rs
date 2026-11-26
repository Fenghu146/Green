use gpui::{
    Modifiers, MouseButton, Pixels, Point, ScrollDelta, ScrollEvent, TestAppContext,
    VisualTestContext, point, px,
};

use super::Pane;
use crate::desk::tests::{open, start};

/// Opens `path` with the outline pane in front.
fn outline_of(desk: &gpui::Entity<crate::desk::Desk>, path: &std::path::Path, cx: &mut VisualTestContext) {
    open(desk, path, cx);
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    side.update(cx, |side, cx| {
        side.pane = Pane::Outline;
        cx.notify();
    });
    cx.run_until_parked();
}

#[gpui::test]
fn a_long_outline_scrolls_to_its_last_heading(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("long.md");
    // Enough headings that the list draws only the rows in view.
    let text: String = (0..300).map(|n| format!("# Heading {n}\n\n")).collect();
    std::fs::write(&path, text).expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    side.update(cx, |side, cx| {
        side.pane = Pane::Outline;
        cx.notify();
    });
    cx.run_until_parked();
    let panel = cx
        .debug_bounds("mark-outline")
        .expect("the outline is drawn");
    assert!(cx.debug_bounds("outline-row-0").is_some(), "the first row draws");
    assert!(
        cx.debug_bounds("outline-row-299").is_none(),
        "the last row waits off screen"
    );
    let at = point(panel.left() + px(20.0), panel.top() + px(100.0));
    for _ in 0..40 {
        cx.simulate_event(ScrollEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-1000.0))),
            ..Default::default()
        });
        cx.run_until_parked();
    }
    assert!(
        cx.debug_bounds("outline-row-299").is_some(),
        "the last heading is in view"
    );
}

/// Searches the folder for `words`; the hits as found.
fn search(
    side: &gpui::Entity<super::Side>,
    words: &str,
    cx: &mut gpui::VisualTestContext,
) -> Vec<(Option<u64>, crate::folder::Hit)> {
    side.update(cx, |side, cx| {
        side.query.update(cx, |query, cx| query.set_text(words, cx));
        side.wide = true;
        side.search(cx);
    });
    cx.run_until_parked();
    side.read_with(cx, |side, _| side.hits.clone())
}

#[gpui::test]
fn a_hit_opens_its_file_with_the_caret_on_the_match(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    std::fs::write(files.path().join("a.md"), "# A\n\nNo match.\n").expect("a");
    std::fs::write(files.path().join("b.md"), "# B\n\nThe Needle here.\n").expect("b");
    let (desk, cx) = start(cx, data.path());
    open(&desk, files.path(), cx);
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    let hits = search(&side, "needle", cx);
    let found: Vec<_> = hits.iter().map(|(_, hit)| (hit.line, hit.column)).collect();
    assert_eq!(found, [(3, 4)]);
    let (tab, hit) = hits[0].clone();
    assert_eq!(tab, None, "b.md is not open");
    desk.update_in(cx, |desk, window, cx| {
        desk.open_at(tab, hit.file, (hit.line - 1, hit.column), window, cx)
    });
    cx.run_until_parked();
    let place = desk.read_with(cx, |desk, cx| {
        desk.active_view().expect("b.md opened").read(cx).place(cx)
    });
    assert_eq!((place.line, place.column), (2, 4));
}

#[gpui::test]
fn a_hit_in_an_open_file_moves_its_caret(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n\nfirst\n\nsecond Word\n").expect("a");
    let (desk, cx) = start(cx, data.path());
    open(&desk, files.path(), cx);
    open(&desk, &path, cx);
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    let hits = search(&side, "word", cx);
    let (tab, hit) = hits[0].clone();
    assert!(tab.is_some(), "the open tab searches");
    desk.update_in(cx, |desk, window, cx| {
        desk.open_at(tab, hit.file, (hit.line - 1, hit.column), window, cx)
    });
    cx.run_until_parked();
    let place = desk.read_with(cx, |desk, cx| {
        desk.active_view().expect("a").read(cx).place(cx)
    });
    assert_eq!((place.line, place.column), (4, 7));
}

#[gpui::test]
fn the_side_folds_from_the_shared_key(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    side.update(cx, |side, cx| {
        side.pane = Pane::Outline;
        cx.notify();
    });
    cx.run_until_parked();
    assert!(cx.debug_bounds("mark-outline").is_some(), "the side shows");
    let focus = desk.read_with(cx, |desk, _| desk.core.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    let kept = || crate::look::load(&data.path().join("look.json")).expect("the look reads");
    // The panel slides; wait out its motion.
    let drawn = |cx: &mut gpui::VisualTestContext, want: bool| {
        for _ in 0..3000 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
            if cx.debug_bounds("mark-outline").is_some() == want {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        false
    };
    // As `ely_ui::run` does at start.
    cx.update(|_, cx| ely_ui::commands::bind_keys(cx));
    let shown = |cx: &mut gpui::VisualTestContext| cx.update(|_, cx| ely_ui::settings::panels::shown(true, cx));
    cx.simulate_keystrokes("secondary-alt-s");
    assert!(drawn(cx, false), "the side folds");
    assert!(!shown(cx) && !kept().focus, "only the side folds");
    cx.simulate_keystrokes("secondary-alt-s");
    assert!(drawn(cx, true), "the side is back");
    assert!(shown(cx));
}

#[gpui::test]
fn a_shut_recent_menu_stays_shut_when_files_return(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let list = cx.debug_bounds("mark-recent-list").expect("the recent list");
    let row = point(list.center().x, list.top() + px(12.0));
    cx.simulate_mouse_down(row, gpui::MouseButton::Right, gpui::Modifiers::none());
    cx.simulate_mouse_up(row, gpui::MouseButton::Right, gpui::Modifiers::none());
    cx.run_until_parked();
    let menu = "menu-row-Remove from Recent";
    assert!(cx.debug_bounds(menu).is_some(), "opened");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(cx.debug_bounds(menu).is_none(), "shut");
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    for pane in [Pane::Outline, Pane::Files] {
        side.update(cx, |side, cx| {
            side.pane = pane;
            cx.notify();
        });
        cx.run_until_parked();
    }
    assert!(cx.debug_bounds("mark-recent-list").is_some(), "files are back");
    assert!(cx.debug_bounds(menu).is_none(), "not replayed");
}

/// A right press at `at`.
fn right(at: Point<Pixels>, cx: &mut VisualTestContext) {
    cx.simulate_mouse_move(at, None, Modifiers::none());
    cx.simulate_mouse_down(at, MouseButton::Right, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Right, Modifiers::none());
    cx.run_until_parked();
}

/// The clipboard after pressing the open menu's `row`.
fn copied(row: &'static str, cx: &mut VisualTestContext) -> Option<String> {
    let at = cx.debug_bounds(row).expect("the menu row");
    cx.simulate_click(at.center(), Modifiers::none());
    cx.run_until_parked();
    cx.update(|_, app| app.read_from_clipboard()).and_then(|item| item.text())
}

#[gpui::test]
fn a_folder_file_copies_its_name_and_path(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    std::fs::write(files.path().join("a.md"), "# A\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, files.path(), cx);
    let slot = cx.debug_bounds("tree-chevron-a.md").expect("a.md in the tree");
    let row = slot.center() + point(px(40.0), px(0.0));
    right(row, cx);
    assert_eq!(copied("menu-row-Copy File Name", cx).as_deref(), Some("a.md"));
    right(row, cx);
    let root = desk.read_with(cx, |desk, _| desk.folder.as_ref().expect("a folder").root.clone());
    assert_eq!(copied("menu-row-Copy Path", cx), Some(root.join("a.md").display().to_string()));
}

#[gpui::test]
fn a_tab_copies_its_file_path(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    // The tab strip sits above the body, past the 40 px toolbar.
    let body = cx.debug_bounds("shell-body").expect("the body");
    right(point(px(30.0), body.top() - px(56.0)), cx);
    let kept = desk.read_with(cx, |desk, _| {
        let tab = desk.core.tabs.last().expect("a tab");
        tab.session.path().map(std::path::Path::to_path_buf).expect("a saved tab")
    });
    assert_eq!(copied("menu-row-Copy Path", cx), Some(kept.display().to_string()));
}

#[gpui::test]
fn a_recent_row_copies_its_name_beside_remove(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let list = cx.debug_bounds("mark-recent-list").expect("the recent list");
    right(point(list.center().x, list.top() + px(12.0)), cx);
    assert!(cx.debug_bounds("menu-row-Remove from Recent").is_some());
    assert_eq!(copied("menu-row-Copy File Name", cx).as_deref(), Some("a.md"));
}

#[gpui::test]
fn a_recent_root_copies_only_its_path(cx: &mut TestAppContext) {
    let data = tempfile::tempdir().expect("data");
    let root = data.path().ancestors().last().expect("a root").to_path_buf();
    let (desk, cx) = start(cx, data.path());
    desk.update(cx, |desk, cx| {
        desk.remember(&root, cx);
        cx.notify();
    });
    cx.run_until_parked();
    let list = cx.debug_bounds("mark-recent-list").expect("the recent list");
    right(point(list.center().x, list.top() + px(12.0)), cx);
    assert!(cx.debug_bounds("menu-row-Remove from Recent").is_some());
    assert!(cx.debug_bounds("menu-row-Copy File Name").is_none());
    assert_eq!(copied("menu-row-Copy Path", cx), Some(root.display().to_string()));
}

#[gpui::test]
fn a_folder_the_filter_finds_opens_to_its_files(cx: &mut TestAppContext) {
    let (data, files) = (tempfile::tempdir().expect("data"), tempfile::tempdir().expect("files"));
    std::fs::create_dir_all(files.path().join("notes/sub")).expect("folders");
    for (path, text) in [("notes/a.md", "a\n"), ("notes/sub/b.md", "b\n"), ("other.md", "o\n")] {
        std::fs::write(files.path().join(path), text).expect("a file");
    }
    let (desk, cx) = start(cx, data.path());
    open(&desk, files.path(), cx);
    let field = cx.debug_bounds("input-root").expect("the file filter");
    cx.simulate_click(field.center(), Modifiers::none());
    cx.simulate_input("notes");
    cx.run_until_parked();
    assert!(cx.debug_bounds("tree-chevron-notes").is_some(), "the folder is found");
    assert!(cx.debug_bounds("tree-chevron-other.md").is_none(), "the rest is filtered");
    assert!(cx.debug_bounds("tree-chevron-notes/a.md").is_none(), "found folded");
    let chevron = cx.debug_bounds("tree-chevron-notes").expect("the folder");
    cx.simulate_click(chevron.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(cx.debug_bounds("tree-chevron-notes/a.md").is_some(), "its file");
    assert!(cx.debug_bounds("tree-chevron-notes/sub").is_some(), "its folder");
}

#[gpui::test]
fn the_outline_folds_itself_and_not_the_text(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n\nalpha\n\n## B\n\nbeta\n\n## C\n\ngamma\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    outline_of(&desk, &path, cx);
    let document = desk.read_with(cx, |desk, cx| {
        desk.active_view().expect("a tab").read(cx).document().clone()
    });
    let chevron = cx.debug_bounds("outline-chevron-0").expect("A's triangle");
    cx.simulate_click(chevron.center(), Modifiers::none());
    cx.run_until_parked();
    assert!(
        cx.debug_bounds("outline-row-1").is_none(),
        "A's headings fold in the panel"
    );
    assert!(
        cx.debug_bounds("outline-chevron-0").is_some(),
        "A keeps its triangle"
    );
    assert!(
        !document.read_with(cx, |document, _| document.hidden(1)),
        "the text keeps its own fold"
    );
    // The panel's folds are kept beside the text's, in the same file entry.
    cx.executor().advance_clock(std::time::Duration::from_secs(1));
    cx.run_until_parked();
    let kept = std::fs::read_to_string(data.path().join("view-state.json")).expect("the state");
    assert!(
        kept.contains("\"panel\"") && kept.contains("1|0|A"),
        "{kept}"
    );
    // The triangle opens it again.
    let chevron = cx.debug_bounds("outline-chevron-0").expect("A's triangle");
    cx.simulate_mouse_move(chevron.center(), None, Modifiers::none());
    cx.simulate_click(chevron.center(), Modifiers::none());
    cx.run_until_parked();
    let folded = desk.read_with(cx, |desk, cx| desk.side.read(cx).collapsed());
    assert!(
        cx.debug_bounds("outline-row-1").is_some(),
        "B is back; the panel folds {folded:?}"
    );
}

#[gpui::test]
fn a_kept_panel_fold_comes_back_with_the_file(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n\nalpha\n\n## B\n\nbeta\n").expect("a file");
    let mut entries = serde_json::Map::new();
    entries.insert(
        crate::viewstate::key(&path),
        serde_json::json!({ "panel": ["1|0|A"] }),
    );
    let state = serde_json::json!({ "files": entries });
    std::fs::write(
        data.path().join("view-state.json"),
        serde_json::to_vec_pretty(&state).expect("json"),
    )
    .expect("the state");
    let (desk, cx) = start(cx, data.path());
    outline_of(&desk, &path, cx);
    assert!(
        cx.debug_bounds("outline-row-1").is_none(),
        "the outline opens as it was folded"
    );
    let document = desk.read_with(cx, |desk, cx| {
        desk.active_view().expect("a tab").read(cx).document().clone()
    });
    assert!(
        !document.read_with(cx, |document, _| document.hidden(1)),
        "the text's fold was its own"
    );
}

#[gpui::test]
fn the_outline_opens_from_ctrl_shift_o(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n\ntext\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    // As a reader starts: the side folded away.
    cx.update(|_, app| ely_ui::settings::panels::show(true, false, app));
    let focus = desk.read_with(cx, |desk, _| desk.core.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.simulate_keystrokes("secondary-shift-o");
    assert!(cx.update(|_, app| ely_ui::settings::panels::shown(true, app)));
    assert!(side.read_with(cx, |side, _| side.is_outline()));
    cx.simulate_keystrokes("secondary-shift-o");
    assert!(!cx.update(|_, app| ely_ui::settings::panels::shown(true, app)));
}

#[gpui::test]
fn a_heading_in_the_outline_jumps_and_unfolds_the_text(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("a.md");
    std::fs::write(&path, "# A\n\nalpha\n\n## B\n\nbeta\n").expect("a file");
    let (desk, cx) = start(cx, data.path());
    outline_of(&desk, &path, cx);
    let view = desk.read_with(cx, |desk, _| desk.active_view().expect("a tab"));
    let document = view.read_with(cx, |view, _| view.document().clone());
    desk.update_in(cx, |_, window, cx| {
        document.update(cx, |document, cx| {
            assert_eq!(document.toggle_fold_at(0, window, cx), Some(true));
        });
    });
    cx.run_until_parked();
    assert!(
        document.read_with(cx, |document, _| document.hidden(1)),
        "the text folds A"
    );
    // A's title is in the panel; B's row shows because the panel is open.
    let row = cx.debug_bounds("outline-row-1").expect("B's row");
    cx.simulate_click(point(row.left() + px(80.0), row.center().y), Modifiers::none());
    cx.run_until_parked();
    assert!(
        !document.read_with(cx, |document, _| document.hidden(1)),
        "the jump unfolds the text's ancestors"
    );
    assert_eq!(
        view.read_with(cx, |view, cx| view.place(cx).line),
        4,
        "the caret lands on B"
    );
}

#[gpui::test]
fn a_long_outline_opens_quickly(cx: &mut TestAppContext) {
    let (data, files) = (
        tempfile::tempdir().expect("data"),
        tempfile::tempdir().expect("files"),
    );
    let path = files.path().join("long.md");
    let text: String = (0..3000).map(|n| format!("# H{n}\n\ntext\n\n")).collect();
    std::fs::write(&path, text).expect("a file");
    let (desk, cx) = start(cx, data.path());
    open(&desk, &path, cx);
    let side = desk.read_with(cx, |desk, _| desk.side.clone());
    let start = std::time::Instant::now();
    side.update(cx, |side, cx| {
        side.pane = Pane::Outline;
        cx.notify();
    });
    cx.run_until_parked();
    let took = start.elapsed();
    println!("outline: 3000 headings opened in {took:?}");
    // Debug bounds exist only in debug builds.
    if cfg!(debug_assertions) {
        assert!(cx.debug_bounds("outline-row-0").is_some(), "the tree draws");
        assert!(
            cx.debug_bounds("outline-row-2999").is_none(),
            "only the rows in view draw"
        );
    }
    // A debug build is the slow bench; the release build holds design §9.
    let goal = if cfg!(debug_assertions) {
        std::time::Duration::from_millis(300)
    } else {
        std::time::Duration::from_millis(100)
    };
    assert!(took < goal, "3000 headings took {took:?}, over {goal:?}");
}
