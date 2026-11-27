//! One Markdown file, its text kept for the shell.

use std::path::PathBuf;

use std::ops::Range;

use ely_gpui_component::{
    documents::{
        DocumentEvent, EditRefusal, LARGEST_READ, MarkdownDocument, MarkdownMode, OutlineHeading,
    },
    forms::TextInput,
    overlays::PromptDialog,
};
use gpui::{
    AnyElement, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable as _,
    InteractiveElement as _, IntoElement, MouseButton, MouseUpEvent, ParentElement as _, Render,
    Styled as _, Window, div,
};

use crate::engine::{Doc, mark};

/// The text changed, or a read is in.
pub struct Reported;

/// What went wrong, for the shell to say.
pub struct Failed(pub String);

/// An edit asked did not happen, and why.
pub struct Refused(pub EditRefusal);

/// How a document shows (MARK-02).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Visual,
    Source,
    Preview,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Visual, Mode::Source, Mode::Preview];

    pub fn name(self) -> &'static str {
        match self {
            Mode::Visual => "visual",
            Mode::Source => "source",
            Mode::Preview => "preview",
        }
    }

    pub fn parse(name: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|mode| mode.name() == name)
    }

    fn shown(self) -> MarkdownMode {
        match self {
            Mode::Visual => MarkdownMode::Visual,
            Mode::Source => MarkdownMode::Source,
            Mode::Preview => MarkdownMode::Preview,
        }
    }
}

/// One heading of the outline, and its line from zero.
pub type Heading = OutlineHeading;

/// The caret and the first line in view.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Place {
    pub line: u64,
    #[serde(default)]
    pub column: u64,
    #[serde(default)]
    pub top: u64,
}

pub struct MarkView {
    document: Entity<MarkdownDocument>,
    doc: Doc,
    /// Text changes so far; autosave counts them.
    changed: u64,
    /// The mark at the last load or save.
    saved: String,
    pub mode: Mode,
    /// This file's kept folds are in.
    loaded: bool,
    /// A saved file's folder: pictures resolve and go here.
    pub(crate) folder: Option<PathBuf>,
    /// The mode first took input (PERF-07).
    pub(crate) shown: bool,
    /// Trackpad pixels toward the next size step.
    wheel: f32,
    /// The last right-click menu asked.
    offer: Option<ely_ui::menu::Offer<menu::Asked>>,
    /// A formula's source span, held from the press to the next frame.
    formula_pending: Option<Range<usize>>,
    /// A formula's source span and its small editor, when open.
    formula: Option<(Range<usize>, Entity<TextInput>)>,
    pub focus: FocusHandle,
}

/// Single-dollar spans, read from the source as the document's guard reads them.
fn formula_spans(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let (mut found, mut open, mut ix) = (Vec::new(), None, 0);
    while ix < bytes.len() {
        match bytes[ix] {
            b'\\' => ix += 2,
            b'`' => {
                let start = ix;
                while ix < bytes.len() && bytes[ix] == b'`' {
                    ix += 1;
                }
                let run = ix - start;
                while ix < bytes.len() {
                    if bytes[ix] == b'`' {
                        let at = ix;
                        while ix < bytes.len() && bytes[ix] == b'`' {
                            ix += 1;
                        }
                        if ix - at == run {
                            break;
                        }
                    } else {
                        ix += 1;
                    }
                }
            }
            b'$' if ix + 1 < bytes.len() && bytes[ix + 1] == b'$' => ix += 2,
            b'$' => {
                match open.take() {
                    None => open = Some(ix),
                    Some(start) if ix > start + 1 => found.push(start..ix + 1),
                    Some(_) => {}
                }
                ix += 1;
            }
            _ => ix += 1,
        }
    }
    found
}

impl EventEmitter<Reported> for MarkView {}
impl EventEmitter<Failed> for MarkView {}
impl EventEmitter<Refused> for MarkView {}

impl MarkView {
    pub fn new(doc: Doc, saved: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let text = doc.lock().expect("the text lock").current.clone();
        let mode = match text.len() > LARGEST_READ {
            true => Mode::Source,
            false => Mode::Visual,
        };
        let view = cx.weak_entity();
        let document = cx.new(|cx| {
            let mut document = MarkdownDocument::new(text, mode.shown(), window, cx);
            document.on_link(move |url, _, cx| {
                view.update(cx, |view, cx| view.follow(url, cx)).ok();
            });
            document
        });
        cx.subscribe_in(&document, window, Self::heard).detach();
        cx.subscribe(&document, |view, _, attached, cx| view.attach(attached, cx))
            .detach();
        // The shell hears the load.
        cx.defer_in(window, |view, _, cx| {
            view.show_once(cx);
            cx.emit(Reported);
        });
        let focus = cx.focus_handle();
        // The view's focus is the document's.
        cx.on_focus(&focus, window, |view, window, cx| {
            let inner = view.document.focus_handle(cx);
            window.focus(&inner, cx);
        })
        .detach();
        MarkView {
            document,
            doc,
            changed: 0,
            saved,
            mode,
            loaded: false,
            folder: None,
            shown: false,
            wheel: 0.0,
            offer: None,
            formula_pending: None,
            formula: None,
            focus,
        }
    }

    pub(crate) fn document(&self) -> &Entity<MarkdownDocument> {
        &self.document
    }

    /// Every change is in the text at once.
    pub fn settled(&self) -> bool {
        true
    }

    pub fn changed(&self) -> u64 {
        self.changed
    }

    /// The text's size now, in bytes.
    pub fn bytes(&self) -> usize {
        self.doc.lock().expect("the text lock").current.len()
    }

    /// Too long to read: source alone (MARK-14).
    pub fn large(&self) -> bool {
        self.bytes() > LARGEST_READ
    }

    pub fn modified(&self) -> bool {
        mark(&self.doc.lock().expect("the text lock").current) != self.saved
    }

    pub fn saved(&mut self, mark: &str, cx: &mut Context<Self>) {
        self.saved = mark.to_owned();
        cx.emit(Reported);
    }

    /// The same text in `mode` (MARK-02).
    pub fn switch(&mut self, mode: Mode, window: &mut Window, cx: &mut Context<Self>) {
        if self.large() && mode != Mode::Source {
            log::info!("mark: a large file stays in source");
            let said = ely_ui::i18n::text(cx, "mark.large.modes", &[]).to_string();
            return cx.emit(Failed(said));
        }
        log::info!("mark: switches to {}", mode.name());
        self.mode = mode;
        self.document.update(cx, |document, cx| {
            document.set_mode(mode.shown(), window, cx)
        });
        cx.notify();
    }

    /// The headings as a tree: keys, parents, and the cleaned titles.
    pub fn outline(&self, cx: &gpui::App) -> Vec<Heading> {
        self.document.read(cx).outline_tree(cx)
    }

    /// The kind of block at the caret, for the paragraph menu's tick.
    pub fn block_style(&self, cx: &gpui::App) -> Option<ely_gpui_component::documents::TargetBlock> {
        self.document.read(cx).block_kind(cx)
    }

    /// Takes this file's kept folds, once it opens.
    pub fn set_folds(&mut self, folded: Vec<String>, cx: &mut Context<Self>) {
        self.loaded = true;
        self.document
            .update(cx, |document, cx| document.set_folded(folded, cx));
    }

    /// Whether this file's kept folds are in yet.
    pub fn loaded_folds(&self) -> bool {
        self.loaded
    }

    /// The headings folded in this file now, as its view state keeps them.
    pub fn folds(&self, cx: &gpui::App) -> Vec<String> {
        self.document.read(cx).folded_keys()
    }

    /// Shows the heading at byte `at` and line `line`, unfolding its ancestors
    /// in the text first.
    pub fn go(&mut self, at: usize, line: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.document
            .update(cx, |document, cx| document.unfold_at(at, cx));
        self.reveal((line, 0), window, cx);
    }

    /// Line and column `at` in view and focus, caret there.
    pub fn reveal(&mut self, at: (usize, usize), window: &mut Window, cx: &mut Context<Self>) {
        self.document.update(cx, |document, cx| {
            document.reveal(at.0, window, cx);
            document.set_caret(at.0, at.1, cx);
        });
    }

    /// The caret and the view, to keep across launches.
    pub fn place(&self, cx: &gpui::App) -> Place {
        let document = self.document.read(cx);
        let (line, column) = document.caret(cx);
        Place {
            line: line as u64,
            column: column as u64,
            top: document.top_line(cx) as u64,
        }
    }

    /// The disk's text replaces a clean one; undo starts there.
    pub fn refresh(&mut self, source: crate::text::Source, cx: &mut Context<Self>) {
        let text = source.text.clone();
        {
            let mut doc = self.doc.lock().expect("the text lock");
            doc.current = text.clone();
            doc.source = source;
        }
        self.saved = mark(&text);
        self.document
            .update(cx, |document, cx| document.reset(text, cx));
        cx.emit(Reported);
    }

    /// `text` replaces the document's, as one undoable edit.
    pub fn adopt(&mut self, text: String, cx: &mut Context<Self>) {
        self.document
            .update(cx, |document, cx| document.set_text(text, cx));
    }

    /// Shows as `mode` at `place`; recovered text unsaved.
    pub fn reopen(
        &mut self,
        mode: Mode,
        place: Option<Place>,
        text: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(text) = text {
            self.document
                .update(cx, |document, cx| document.reset(text, cx));
        }
        let mode = if self.large() { Mode::Source } else { mode };
        self.switch(mode, window, cx);
        if let Some(place) = place {
            self.document.update(cx, |document, cx| {
                document.set_caret(place.line as usize, place.column as usize, cx);
                document.set_top_line(place.top as usize, window, cx);
            });
        }
    }

    /// The saved file's folder; pictures follow it.
    pub fn moved(&mut self, folder: PathBuf, cx: &mut Context<Self>) {
        if self.folder.as_ref() == Some(&folder) {
            return;
        }
        self.document
            .update(cx, |document, cx| document.set_base(folder.as_path(), cx));
        self.folder = Some(folder);
    }

    fn heard(
        &mut self,
        document: &Entity<MarkdownDocument>,
        event: &DocumentEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let DocumentEvent::EditRefused(why) = event {
            return cx.emit(Refused(*why));
        }
        if let DocumentEvent::Pointed(target) = event {
            let at = window.mouse_position();
            return self.ask_menu(menu::Asked::Text(target.clone()), at, cx);
        }
        if *event == DocumentEvent::Changed {
            let text = document.read(cx).text(cx);
            self.doc.lock().expect("the text lock").current = text;
            self.changed += 1;
            if self.large() && self.mode != Mode::Source {
                log::info!("mark: the text grew past the read limit");
                self.switch(Mode::Source, window, cx);
            }
        }
        self.show_once(cx);
        cx.emit(Reported);
    }

    /// Logs the first moment the mode takes input.
    fn show_once(&mut self, cx: &gpui::App) {
        let ready = self.mode == Mode::Source || self.document.read(cx).read_now(cx);
        if ready && !self.shown {
            self.shown = true;
            log::info!("mark: shown as {:?}", self.mode.name());
        }
    }

    /// A pressed link: web and mail open outside (M-05).
    fn follow(&mut self, url: &str, cx: &mut Context<Self>) {
        match ely_ui::outward(url) {
            Ok(address) => {
                log::info!("mark: opens a link outside");
                cx.open_url(&address);
            }
            Err(notice) => {
                log::info!("mark: a link was refused");
                let said = ely_ui::i18n::text(cx, notice, &[]).to_string();
                cx.emit(Failed(said));
            }
        }
    }
}

impl MarkView {
    /// The formula the caret sits on, in source bytes.
    fn formula_at_caret(&self, cx: &mut Context<Self>) -> Option<Range<usize>> {
        if self.mode != Mode::Visual || self.formula.is_some() {
            return None;
        }
        let (line, column) = self.document.read(cx).caret(cx);
        let text = self.document.read(cx).text(cx);
        let mut line_at = 0;
        for _ in 0..line {
            line_at += text[line_at..].find('\n')? + 1;
        }
        let rest = &text[line_at..];
        let end = rest.find('\n').unwrap_or(rest.len());
        let byte = line_at
            + rest[..end]
                .char_indices()
                .nth(column)
                .map_or(end, |(at, _)| at);
        formula_spans(&text)
            .into_iter()
            .find(|span| byte == span.start || byte == span.end)
    }

    /// Writes back the block a press opened and holds the formula for the
    /// next frame; this is where the page settles before the dialog appears.
    fn commit_formula(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(span) = self.formula_at_caret(cx) else {
            return false;
        };
        let (line, column) = self.document.read(cx).caret(cx);
        self.document
            .update(cx, |document, cx| document.set_caret(line, column, cx));
        self.formula_pending = Some(span);
        true
    }

    /// A press first writes back the block it opened, then shows the editor
    /// a frame later: the page keeps its new frame while the dialog takes focus.
    fn press_formula(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.commit_formula(cx) {
            return false;
        }
        let view = cx.entity();
        window.defer(cx, move |window, cx| {
            view.update(cx, |view, cx| view.show_formula(window, cx));
        });
        true
    }

    /// Opens the held formula's `$…$` source in a small editor.
    fn show_formula(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(span) = self.formula_pending.take() else {
            return;
        };
        let text = self.document.read(cx).text(cx);
        let source = text[span.clone()].to_string();
        let field = cx.new(|cx| {
            let mut field = TextInput::new(window, cx);
            field.set_text(source, cx);
            field
        });
        self.formula = Some((span, field));
        cx.notify();
    }

    /// The small source editor: one range edit on Enter, nothing on Escape.
    fn formula_element(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let (span, field) = self.formula.as_ref()?;
        let (span, field) = (span.clone(), field.clone());
        let closed = cx.entity();
        let sent = cx.entity();
        let prompt = PromptDialog::new(
            "mark-formula",
            ely_ui::i18n::text(cx, "menu.formula", &[]),
            &field,
            move |_, cx| {
                closed.update(cx, |view, cx| {
                    view.formula = None;
                    cx.notify();
                })
            },
        )
        .label("$…$")
        .submit(ely_ui::i18n::text(cx, "doc.formula.apply", &[]))
        .on_submit(move |source, _, cx| {
            sent.update(cx, |view, cx| {
                view.formula = None;
                view.document.update(cx, |document, cx| {
                    document.replace(vec![(span.clone(), source.to_string())], cx);
                });
                cx.notify();
            })
        });
        Some(prompt.into_any_element())
    }
}

impl Render for MarkView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        // Ctrl and the wheel size text, as Ctrl+= does.
        let zoom = ely_ui::wheel::zoom_layer(move |event, window, cx| {
            let steps = view.update(cx, |view, _| ely_ui::wheel::steps(event, &mut view.wheel));
            for _ in 0..steps.unsigned_abs() {
                match steps > 0 {
                    true => window.dispatch_action(Box::new(crate::look::Larger), cx),
                    false => window.dispatch_action(Box::new(crate::look::Smaller), cx),
                }
            }
        });
        div()
            .relative()
            .size_full()
            .track_focus(&self.focus)
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|view, event, _, cx| view.pressed(event, cx)),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|view, _: &MouseUpEvent, window, cx| {
                    if view.press_formula(window, cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .child(self.document.clone())
            .child(zoom)
            .children(self.context_menu(window, cx))
            .children(self.formula_element(cx))
    }
}

mod menu;
#[cfg(test)]
mod fold_tests;
#[cfg(test)]
mod formula_atom_tests;
#[cfg(test)]
mod tests;
