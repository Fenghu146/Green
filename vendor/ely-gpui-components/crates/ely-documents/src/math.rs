//! Formulas typeset by Typst in process: the editor, the outline and the
//! export share one engine, one font set and one baseline (design §6).

use std::{
    collections::VecDeque,
    fmt,
    sync::{Arc, Mutex, OnceLock},
    time::Instant,
};

use gpui::{
    AnyElement, App, Asset, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderImage, RenderOnce, SharedString, Size, Styled, Window, WrappedLineLayout, div, img, size,
};
use smallvec::SmallVec;
use typst::{
    LibraryExt as _,
    diag::{FileError, FileResult},
    foundations::{Bytes, Datetime, Duration},
    layout::{Frame, FrameItem},
    syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot},
    text::{Font, FontBook},
    utils::{LazyHash, Numeric as _},
};
use typst_layout::{Page, PagedDocument};

use crate::{
    primitives::hold,
    theme::{ActiveTheme, TextSize},
};

/// Why a formula did not typeset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathError {
    message: SharedString,
}

impl fmt::Display for MathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for MathError {}

impl MathError {
    fn said(message: impl Into<SharedString>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// The standard library and the bundled faces, once a process.
struct Engine {
    library: LazyHash<typst::Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
}

static ENGINE: OnceLock<Result<Engine, String>> = OnceLock::new();

/// Starts the formula engine with the faces the app ships; before it, no
/// formula typesets, and each one says why.
pub fn start(fonts: Vec<Font>) -> Result<(), String> {
    if ENGINE.get().is_some() {
        return Ok(());
    }
    let main = FileId::new(RootedPath::new(
        VirtualRoot::Project,
        VirtualPath::new("main.typ").map_err(|_| "main.typ is not a virtual path")?,
    ));
    let book = FontBook::from_fonts(fonts.iter());
    // A family the maths falls back to must be there, or every letter is tofu.
    if book.select("noto sans math", Default::default()).is_none() {
        return Err("Noto Sans Math is not among the faces".into());
    }
    let engine = Engine {
        library: LazyHash::new(typst::Library::default()),
        book: LazyHash::new(book),
        fonts,
        main,
    };
    let _ = ENGINE.set(Ok(engine));
    Ok(())
}

fn engine() -> Result<&'static Engine, MathError> {
    match ENGINE.get() {
        Some(Ok(engine)) => Ok(engine),
        Some(Err(error)) => Err(MathError::said(error.as_str())),
        None => Err(MathError::said("the formula engine has not started")),
    }
}

/// One compile's world: the engine's fonts, and one source in memory.
struct World<'a> {
    engine: &'a Engine,
    source: Source,
}

impl typst::World for World<'_> {
    fn library(&self) -> &LazyHash<typst::Library> {
        &self.engine.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.engine.book
    }

    fn main(&self) -> FileId {
        self.engine.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        match id == self.engine.main {
            true => Ok(self.source.clone()),
            false => Err(FileError::NotFound(id.vpath().get_without_slash().into())),
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.source(id)
            .map(|source| Bytes::new(source.text().as_bytes().to_vec()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.engine.fonts.get(index).cloned()
    }

    fn today(&self, _: Option<Duration>) -> Option<Datetime> {
        None
    }
}

/// A typeset formula: its picture, its logical size, and its baseline.
#[derive(Clone)]
pub(crate) struct Glyph {
    pub picture: Arc<RenderImage>,
    /// Width and height in px; one point is one pixel here.
    pub size: Size<Pixels>,
    /// From the picture's top down to the formula's baseline.
    pub baseline: Pixels,
}

/// What a typeset formula depends on: the TeX as written, its size, the
/// colours the theme gives it, and the window's scale.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Formula {
    source: SharedString,
    display: bool,
    /// The text colour, and the box's stroke, as RGBA.
    rgba: u32,
    border: u32,
    /// Font size and window scale, in hundredths.
    font: u32,
    scale: u32,
}

/// A typeset formula: its picture, or why it has none.
pub(crate) type Typeset = Result<Glyph, MathError>;

/// How the editor's formulas have done: reused, typeset, and failed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MathCounts {
    /// Pictures taken from the cache.
    pub hits: u64,
    /// Pictures typeset afresh.
    pub misses: u64,
    /// Formulas that would not convert or compile.
    pub failed: u64,
}

/// A formula kept, least recently drawn first.
struct Kept {
    formula: Formula,
    glyph: Glyph,
}

struct Cache {
    kept: VecDeque<Kept>,
    counts: MathCounts,
    /// The last failure's words, for the status bar.
    last: Option<SharedString>,
}

/// Formulas held at once; each picture is a few kilobytes.
const KEPT: usize = 256;

static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();

fn cache() -> &'static Mutex<Cache> {
    CACHE.get_or_init(|| {
        Mutex::new(Cache {
            kept: VecDeque::new(),
            counts: MathCounts::default(),
            last: None,
        })
    })
}

/// Hits, misses and failures so far; the status bar reads them.
pub fn counts() -> MathCounts {
    cache().lock().expect("the formula cache").counts
}

/// Why the last formula failed, if one did.
pub fn last_failure() -> Option<SharedString> {
    cache().lock().expect("the formula cache").last.clone()
}

/// Reuses the formula when it was drawn before, the newest first out.
fn kept(formula: &Formula) -> Option<Glyph> {
    let mut cache = cache().lock().expect("the formula cache");
    let at = cache
        .kept
        .iter()
        .position(|kept| &kept.formula == formula)?;
    let kept = cache.kept.remove(at).expect("the formula found");
    let glyph = kept.glyph.clone();
    cache.kept.push_back(kept);
    cache.counts.hits += 1;
    Some(glyph)
}

/// `formula` typeset, from the cache when it was drawn before.
fn typeset_now(formula: &Formula) -> Typeset {
    let started = Instant::now();
    if let Some(glyph) = kept(formula) {
        log::debug!(
            "math: a formula came from the cache in {:.3} ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
        return Ok(glyph);
    }
    match build(formula) {
        Ok(glyph) => {
            log::debug!(
                "math: a formula typeset in {:.2} ms, {:.2}x{:.2} pt, baseline {:.2}",
                started.elapsed().as_secs_f64() * 1000.0,
                glyph.size.width.as_f32(),
                glyph.size.height.as_f32(),
                glyph.baseline.as_f32()
            );
            let mut cache = cache().lock().expect("the formula cache");
            cache.counts.misses += 1;
            cache.kept.push_back(Kept {
                formula: formula.clone(),
                glyph: glyph.clone(),
            });
            while cache.kept.len() > KEPT {
                cache.kept.pop_front();
            }
            Ok(glyph)
        }
        Err(error) => Err(failed_counted(error)),
    }
}

/// Counts a formula that will not draw, and says so once per formula: the
/// note is the number the status bar reads, not the words.
fn failed_counted(error: MathError) -> MathError {
    let mut cache = cache().lock().expect("the formula cache");
    cache.counts.failed += 1;
    cache.last = Some(error.message.clone());
    // The formula is the user's text: the log names no words.
    log::warn!("math: a formula did not typeset");
    error
}

/// The document one formula is compiled from.
fn document_source(formula: &Formula, markup: &str) -> String {
    let markup = markup.trim();
    let body = match markup.starts_with('#') {
        true => markup.to_string(),
        false => match formula.display {
            true => format!("$ {markup} $"),
            false => format!("#box[${markup}$]"),
        },
    };
    format!("{}{body}\n", preamble(formula))
}

/// The page, the text, the maths and the box style every formula shares; the
/// export spells the same rules from `green_math` (design §7).
fn preamble(formula: &Formula) -> String {
    let size = f64::from(formula.font) / 100.0;
    let fill = hex(formula.rgba);
    let stroke = hex(formula.border);
    format!(
        "#set page(width: auto, height: auto, margin: 0pt, fill: none)\n\
         #set text(font: (\"Noto Sans CJK SC\",), size: {size}pt, fill: rgb(\"{fill}\"))\n\
         #set par(leading: 0pt)\n\
         {}\n\
         {}\n",
        green_math::math_rule(),
        green_math::gbox_rule(&stroke)
    )
}

/// `rgba` as `#rrggbb`.
fn hex(rgba: u32) -> String {
    format!("#{:06x}", rgba >> 8)
}

/// Width, height and baseline of one formula's page, in points.
struct Shape {
    width: f32,
    height: f32,
    /// The content frame's baseline; the page frame's own is its bottom.
    baseline: f32,
}

/// Read the page, following single groups down to the frame carrying the
/// baseline: `Frame::push_frame` inlines a soft frame of five items or fewer
/// and loses its baseline with it.
fn shape(page: &Page) -> Shape {
    let mut content = &page.frame;
    while let Some(next) = single_group(content) {
        content = next;
    }
    let size = page.frame.size();
    Shape {
        width: size.x.to_pt() as f32,
        height: size.y.to_pt() as f32,
        baseline: content.baseline().to_pt() as f32,
    }
}

/// The only item of `frame`, when it is an untransformed group.
fn single_group(frame: &Frame) -> Option<&Frame> {
    let mut found: Option<&Frame> = None;
    for (position, item) in frame.items() {
        match item {
            // Tags are metadata, not content.
            FrameItem::Tag(..) => continue,
            FrameItem::Group(group) if group.clip.is_none() && position.is_zero() => {
                if found.is_some() {
                    return None;
                }
                found = Some(&group.frame);
            }
            _ => return None,
        }
    }
    found
}

/// `svg` at `scale` device pixels a point: usvg reads points at 96 dpi by
/// default, which would make every formula a third larger than it says.
fn raster(svg: &str, scale: f32) -> Result<Arc<RenderImage>, MathError> {
    let options = usvg::Options {
        dpi: 72.0,
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg, &options)
        .map_err(|error| MathError::said(format!("a formula's svg did not read: {error}")))?;
    let bounds = tree.size();
    let width = (bounds.width() * scale).ceil() as u32;
    let height = (bounds.height() * scale).ceil() as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| MathError::said("a formula has no pixels"))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let mut buffer = image::ImageBuffer::from_raw(width, height, pixmap.take())
        .ok_or_else(|| MathError::said("a formula's pixels did not fit its size"))?;
    // gpui takes its pixels blue first.
    for pixel in buffer.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let picture = RenderImage::new(SmallVec::from_const([image::Frame::new(buffer)]));
    Ok(Arc::new(picture))
}

/// Compile, measure and draw one formula.
fn build(formula: &Formula) -> Typeset {
    let engine = engine()?;
    let markup = convert(&formula.source)?;
    let world = World {
        engine,
        source: Source::new(engine.main, document_source(formula, &markup)),
    };
    let warned = typst::compile::<PagedDocument>(&world);
    for warning in &warned.warnings {
        log::warn!("math: typst said: {}", warning.message);
    }
    let document = warned.output.map_err(first_error)?;
    let page = document
        .pages()
        .first()
        .ok_or_else(|| MathError::said("the formula made no page"))?;
    let shape = shape(page);
    let svg = typst_svg::svg(page, &typst_svg::SvgOptions::default());
    // Two device pixels a point, whatever the window's scale, so a sharper
    // window re-rasterizes instead of stretching.
    let scale = formula.scale as f32 / 100.0 * 2.0;
    Ok(Glyph {
        picture: raster(&svg, scale)?,
        size: size(Pixels::from(shape.width), Pixels::from(shape.height)),
        baseline: Pixels::from(shape.baseline),
    })
}

/// `source` as Typst math, or the first command without a conversion.
fn convert(source: &str) -> Result<String, MathError> {
    green_math::cached(source).map_err(|unsupported| {
        MathError::said(unsupported.first().map_or_else(
            || "the formula does not convert".to_string(),
            ToString::to_string,
        ))
    })
}

fn first_error(errors: impl IntoIterator<Item = typst::diag::SourceDiagnostic>) -> MathError {
    MathError::said(errors.into_iter().next().map_or_else(
        || "the formula does not compile".to_string(),
        |error| error.message.to_string(),
    ))
}

enum Typesetter {}

impl Asset for Typesetter {
    type Source = Formula;
    type Output = Typeset;

    fn load(formula: Formula, _: &mut App) -> impl Future<Output = Typeset> + Send + 'static {
        async move { typeset_now(&formula) }
    }
}

/// `source` as outlined SVG, for export.
pub fn formula_svg(source: &str, display: bool) -> Result<String, MathError> {
    let formula = Formula {
        source: source.to_owned().into(),
        display,
        rgba: 0x0000_00ff,
        border: 0x8a8a_8aff,
        font: 1600,
        scale: 100,
    };
    let engine = engine()?;
    let world = World {
        engine,
        source: Source::new(engine.main, document_source(&formula, &convert(source)?)),
    };
    let document = typst::compile::<PagedDocument>(&world)
        .output
        .map_err(first_error)?;
    let page = document
        .pages()
        .first()
        .ok_or_else(|| MathError::said("the formula made no page"))?;
    Ok(typst_svg::svg(page, &typst_svg::SvgOptions::default()))
}

/// The asset key for `source` at `font` in `window`.
fn formula(
    (source, display): (&SharedString, bool),
    font: Pixels,
    window: &Window,
    cx: &App,
) -> Formula {
    let colors = &cx.theme().colors;
    // The words' colour, so a formula follows the text it sits in (an
    // outline's muted row, a heading's own).
    let fg = window.text_style().color;
    Formula {
        source: source.clone(),
        display,
        rgba: u32::from(fg.to_rgb()),
        border: u32::from(colors.border.to_rgb()),
        font: (font.as_f32() * 100.0).round() as u32,
        scale: (window.scale_factor() * 100.0).round() as u32,
    }
}

/// `source` typeset at `font` in `window`; None while it loads.
pub(crate) fn typeset(
    id: impl Into<ElementId>,
    (source, display): (&SharedString, bool),
    font: Pixels,
    window: &mut Window,
    cx: &mut App,
) -> Option<Typeset> {
    let formula = formula((source, display), font, window, cx);
    let picture = |typeset: &Typeset| typeset.as_ref().ok().map(|glyph| glyph.picture.clone());
    hold::<Typesetter>(id, &formula, picture, window, cx)
}

/// Where a line's baseline sits, from the top of its box: gpui centres the
/// shaped line, then draws it from its ascent.
pub(crate) fn baseline_in(line: &WrappedLineLayout, line_height: Pixels) -> Pixels {
    let padding = (line_height - line.ascent() - line.descent()) / 2.0;
    padding + line.ascent()
}

impl RenderOnce for Latex {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let font = cx.theme().text_size(self.size).to_pixels(window.rem_size());
        match typeset(self.id, (&self.source, self.display), font, window, cx) {
            Some(Ok(glyph)) => img(glyph.picture)
                .w(glyph.size.width)
                .h(glyph.size.height)
                .into_any_element(),
            Some(Err(error)) => failed(&error, &self.source, cx),
            None => div().h(font).into_any_element(),
        }
    }
}

/// TeX typeset by Typst; a failure shows why and its source.
///
/// ```
/// use ely_documents::Latex;
///
/// let area = Latex::parse("area", r"A = \pi r^2").expect("parses").display();
/// ```
#[derive(IntoElement)]
pub struct Latex {
    id: ElementId,
    source: SharedString,
    size: TextSize,
    display: bool,
}

impl Latex {
    /// A formula under `id` from TeX `source`, or the command that has no
    /// conversion yet.
    pub fn parse(id: impl Into<ElementId>, source: &str) -> Result<Self, MathError> {
        if let Err(error) = convert(source) {
            return Err(failed_counted(error));
        }
        Ok(Self {
            id: id.into(),
            source: source.to_string().into(),
            size: TextSize::Lg,
            display: false,
        })
    }

    /// Its text size, large unless set.
    pub fn size(mut self, size: TextSize) -> Self {
        self.size = size;
        self
    }

    /// Display style: limits above and below, larger operators.
    pub fn display(mut self) -> Self {
        self.display = true;
        self
    }
}

/// A formula that did not typeset: why, then its source.
pub(crate) fn failed(error: &MathError, source: &str, cx: &App) -> AnyElement {
    let theme = cx.theme();
    div()
        .debug_selector(|| "latex-error".into())
        .flex()
        .flex_col()
        .child(
            div()
                .text_color(theme.colors.danger)
                .child(error.to_string()),
        )
        .child(
            div()
                .font_family(theme.mono_family.clone())
                .text_color(theme.colors.fg_muted)
                .child(source.to_string()),
        )
        .into_any_element()
}

/// The words a paragraph draws a failed formula as: its TeX, delimiters and
/// all, beside the words the user wrote.
pub(crate) fn literal(source: &str) -> String {
    format!("${source}$")
}

#[cfg(test)]
mod tests;
