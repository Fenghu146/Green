//! The licence page: GPL, third-party notices and each font's OFL (S6).
//!
//! The page is built from the files that ship beside `green.exe` (or from the
//! repository in a dev build), so what it shows cannot drift from the package.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use ely_office::prelude::*;
use ely_ui::i18n::text;
use gpui::{App, Context};

use crate::desk::Desk;

impl Desk {
    /// Opens the licence page, from the package's files, as a read-only tab.
    pub fn open_licenses(&mut self, cx: &mut Context<Self>) {
        if let Some(id) = self
            .core
            .tabs
            .iter()
            .find(|tab| tab.extra.special)
            .map(|tab| tab.id)
        {
            self.core_mut().set_active(Some(id));
            return cx.notify();
        }
        let page = match page(cx) {
            Ok(page) => page,
            Err(error) => format!("# {}\n\n{error}\n", text(cx, "licenses.title", &[])),
        };
        // The tab lands on the background reader; `tab_opened` names and seals it.
        self.licensing = true;
        self.new_from_text(PAGE, &page, cx);
    }
}

/// The page's file name: the copy in the work folder, never a user file.
pub(crate) const PAGE: &str = "Licences.md";

/// The page, from `root`'s licence files.
fn page_at(root: &Path, cx: &App) -> Result<String, String> {
    let notices = read(&root.join("THIRD-PARTY-NOTICES.txt"))?;
    let gpl = read(&root.join("LICENSES/GPL-3.0.txt"))?;
    let mut out = format!("# {}\n\n", text(cx, "licenses.title", &[]));
    prose(&mut out, summary(&notices));
    fonts(&mut out, &fonts_dir(root), cx)?;
    out.push_str(&format!("## {}\n\n", text(cx, "licenses.crates", &[])));
    for name in licenses(&notices) {
        out.push_str(&format!("- {name}\n"));
    }
    out.push_str("\n`THIRD-PARTY-NOTICES.txt`\n\n");
    out.push_str("## GNU GPL v3\n\n");
    prose(&mut out, &gpl);
    Ok(out)
}

/// The page, from the package beside the program or the source tree.
fn page(cx: &App) -> Result<String, String> {
    page_at(&root()?, cx)
}

/// The folder holding `LICENSES/` and `fonts/`.
fn root() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf)) {
        candidates.push(dir);
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let mut tried = Vec::new();
    for candidate in candidates {
        let candidate = fs::canonicalize(&candidate).unwrap_or(candidate);
        if candidate.join("LICENSES/GPL-3.0.txt").is_file() {
            return Ok(candidate);
        }
        tried.push(candidate.display().to_string());
    }
    Err(format!(
        "no LICENSES/GPL-3.0.txt beside the program or in the source: {}",
        tried.join(", ")
    ))
}

/// The fonts: beside the program in a package, under `assets/` in the source.
fn fonts_dir(root: &Path) -> PathBuf {
    let beside = root.join("fonts");
    if beside.is_dir() {
        beside
    } else {
        root.join("assets/fonts")
    }
}

/// Each font file, then each OFL file with its own copyright line and text.
fn fonts(out: &mut String, dir: &Path, cx: &App) -> Result<(), String> {
    out.push_str(&format!("## {}\n\n", text(cx, "licenses.fonts", &[])));
    let entries = fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
    files.sort();
    for file in &files {
        if matches!(extension(file).as_deref(), Some("otf" | "ttf")) {
            out.push_str(&format!("- `{}`\n", name(file)));
        }
    }
    out.push('\n');
    for file in &files {
        if extension(file).as_deref() != Some("txt") {
            continue;
        }
        let text = read(file)?;
        out.push_str(&format!("### {}\n\n", name(file)));
        if let Some(line) = copyright(&text) {
            out.push_str(&format!("{line}\n\n"));
        }
        prose(out, &text);
    }
    Ok(())
}

/// The file's own copyright notice: a year follows the word, as a notice has.
fn copyright(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| {
        let lower = line.to_lowercase();
        let rest = lower.strip_prefix("copyright").or_else(|| lower.strip_prefix('©'));
        rest.is_some_and(|rest| rest.chars().take(20).any(|c| c.is_ascii_digit()))
    })
}

/// The notices' own words before the per-license crates list.
fn summary(notices: &str) -> &str {
    notices.split("\nCrates\n------").next().unwrap_or(notices)
}

/// The licence names the notices carry, in order.
fn licenses(notices: &str) -> Vec<String> {
    let lines: Vec<&str> = notices.lines().collect();
    let mut found = BTreeSet::new();
    for window in lines.windows(3) {
        if window[0].starts_with("====") && window[2].starts_with("Used by:") {
            found.insert(window[1].trim().to_string());
        }
    }
    found.into_iter().collect()
}

/// `text` as paragraphs that wrap; a rule line stays a rule, not a heading.
fn prose(out: &mut String, text: &str) {
    for line in text.trim_end().lines() {
        let line = line.trim_end();
        let rule = line.len() >= 3 && line.chars().all(|c| c == '=' || c == '-' || c == ' ');
        out.push_str(if rule { "***" } else { line });
        out.push('\n');
    }
    out.push('\n');
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn name(path: &Path) -> String {
    path.file_name()
        .map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned())
}

fn extension(path: &Path) -> Option<String> {
    path.extension().map(|ext| ext.to_string_lossy().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{copyright, licenses, page_at, prose, summary};

    const NOTICES: &str = "Green's third-party notices\n===========================\n\nAssets\n------\n\nNoto\n\nCrates\n------\n\n\
================================================================================\nApache License 2.0 (Apache-2.0)\nUsed by:\n  - a 1.0.0\n\
--------------------------------------------------------------------------------\ntext\n\
================================================================================\nMozilla Public License 2.0 (MPL-2.0)\nUsed by:\n  - option-ext 0.2.0\n\
--------------------------------------------------------------------------------\ntext\n";

    #[test]
    fn the_summary_stops_at_the_crates_list() {
        let said = summary(NOTICES);
        assert!(said.starts_with("Green's third-party notices"), "{said}");
        assert!(said.contains("Assets") && !said.contains("Apache License"), "{said}");
    }

    #[test]
    fn every_licence_in_the_notices_is_named_once() {
        assert_eq!(
            licenses(NOTICES),
            vec!["Apache License 2.0 (Apache-2.0)", "Mozilla Public License 2.0 (MPL-2.0)"]
        );
    }

    #[test]
    fn a_rule_line_stays_a_rule_in_wrapping_prose() {
        let mut out = String::new();
        prose(&mut out, "Title\n=====\n\none\ntwo\n\n");
        assert_eq!(out, "Title\n***\n\none\ntwo\n\n");
    }

    #[test]
    fn the_copyright_line_is_the_notice_not_prose_about_it() {
        let ofl = "\"Reserved Font Name\" refers to any names specified as such after the\ncopyright statement(s).\n\nCopyright 2014-2021 Adobe (http://www.adobe.com/), with Reserved Font Name 'Source'.\n";
        assert_eq!(
            copyright(ofl),
            Some("Copyright 2014-2021 Adobe (http://www.adobe.com/), with Reserved Font Name 'Source'.")
        );
        assert_eq!(copyright("This licence names no holder.\n"), None);
    }

    /// A package on disk: GPL, notices, one font file and its OFL.
    fn package() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let write = |path: &str, text: &str| {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        write("LICENSES/GPL-3.0.txt", "GNU GENERAL PUBLIC LICENSE\nVersion 3, 29 June 2007\n");
        write("THIRD-PARTY-NOTICES.txt", NOTICES);
        write("fonts/NotoSansCJKsc-Regular.otf", "");
        write(
            "fonts/OFL.txt",
            "Copyright 2014-2021 Adobe (http://www.adobe.com/), with Reserved Font Name 'Source'.\n\nSIL OPEN FONT LICENSE Version 1.1\n",
        );
        dir
    }

    #[gpui::test]
    fn the_page_shows_the_packages_own_licence_files(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| ely_ui::i18n::install_as("en-US", cx));
        let package = package();
        let page = cx.update(|cx| page_at(package.path(), cx).unwrap());
        assert!(page.starts_with("# Licences"), "{page}");
        assert!(page.contains("## Fonts"), "{page}");
        assert!(page.contains("- `NotoSansCJKsc-Regular.otf`"), "{page}");
        assert!(page.contains("Copyright 2014-2021 Adobe"), "{page}");
        assert!(page.contains("SIL OPEN FONT LICENSE Version 1.1"), "{page}");
        assert!(page.contains("## Third-party crates"), "{page}");
        assert!(page.contains("- Mozilla Public License 2.0 (MPL-2.0)"), "{page}");
        assert!(page.contains("GNU GENERAL PUBLIC LICENSE"), "{page}");
        // The summary stops before the crates list; that lives in the file.
        assert!(!page.contains("  - option-ext 0.2.0"), "{page}");
    }

    #[gpui::test]
    fn a_package_without_its_licences_says_which_file_is_missing(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| ely_ui::i18n::install_as("en-US", cx));
        let dir = tempfile::tempdir().unwrap();
        let said = cx.update(|cx| page_at(dir.path(), cx).unwrap_err());
        assert!(said.contains("THIRD-PARTY-NOTICES.txt"), "{said}");
    }

    #[gpui::test]
    fn the_help_menu_offers_the_page(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| ely_ui::i18n::install_as("en-US", cx));
        let look = crate::look::Look::default();
        let menus = cx.update(|cx| crate::keys::menus(&look, cx));
        let help = menus.iter().find(|menu| menu.name == "Help").expect("a Help menu");
        let (name, action) = help
            .items
            .iter()
            .find_map(|item| match item {
                gpui::MenuItem::Action { name, action, .. } => Some((name.clone(), action.name())),
                _ => None,
            })
            .expect("an item");
        assert_eq!(name, "Licences…");
        assert!(action.ends_with("OpenLicenses"), "{action}");
    }

    #[gpui::test]
    fn the_action_opens_one_read_only_page(cx: &mut gpui::TestAppContext) {
        let data = tempfile::tempdir().unwrap();
        let (desk, cx) = crate::desk::tests::start(cx, data.path());
        cx.dispatch_action(crate::keys::OpenLicenses);
        cx.run_until_parked();
        let title = cx.update(|_, cx| ely_ui::i18n::text(cx, "licenses.title", &[]).to_string());
        let (names, special, mode) = desk.read_with(cx, |desk, cx| {
            let tab = desk.core.tabs.last().expect("a tab");
            (
                desk.core.tabs.iter().map(|tab| tab.name.to_string()).collect::<Vec<_>>(),
                tab.extra.special,
                tab.view.read(cx).mode,
            )
        });
        assert_eq!(names, vec![title]);
        assert!(special, "the page is not a document");
        assert_eq!(mode, crate::view::Mode::Preview);
        let page = desk.read_with(cx, |desk, cx| {
            desk.active_view().expect("a tab").read(cx).document().read(cx).text(cx)
        });
        assert!(page.starts_with("# Licences"), "{}", &page[..page.len().min(200)]);
        assert!(page.contains("## GNU GPL v3"), "{}", &page[..page.len().min(200)]);
        assert!(page.contains("NotoSansCJKsc-Regular.otf"), "{}", &page[..page.len().min(200)]);
        // Preview is the only mode a page shows in.
        cx.dispatch_action(crate::keys::VisualMode);
        cx.run_until_parked();
        let mode = desk.read_with(cx, |desk, cx| {
            desk.core.tabs.last().expect("a tab").view.read(cx).mode
        });
        assert_eq!(mode, crate::view::Mode::Preview, "a page takes no other mode");
        cx.dispatch_action(crate::keys::OpenLicenses);
        cx.run_until_parked();
        assert_eq!(desk.read_with(cx, |desk, _| desk.core.tabs.len()), 1, "one page tab");
    }
}
