//! The document tree an export takes: Green's blocks without the editor's UI.
//!
//! The shape follows `ely_documents::tree`, so the app maps one to the other
//! field by field; front matter is metadata, not a block, and is not exported.

use std::ops::Range;

/// How a stretch of inline text prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Style {
    /// Bold.
    Strong,
    /// Italic.
    Emphasis,
    /// Struck through.
    Strike,
    /// Inline code.
    Code,
    /// A link, its address in [`Inline::links`].
    Link,
    /// Inline math: the stretch holds its TeX.
    Math,
    /// A footnote reference, its text the number as drawn.
    Footnote,
    /// Underlined.
    Underline,
    /// A highlighted run, `==like this==`.
    Mark,
}

/// A run of styled text, its styles and links in bytes of `text`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inline {
    /// The text as drawn.
    pub text: String,
    /// Styled stretches.
    pub styles: Vec<(Range<usize>, Style)>,
    /// Link addresses.
    pub links: Vec<(Range<usize>, String)>,
}

impl Inline {
    /// `text`, styled none.
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            styles: Vec::new(),
            links: Vec::new(),
        }
    }

    /// The same, one stretch styled.
    pub fn styled(text: impl Into<String>, styles: Vec<(Range<usize>, Style)>) -> Self {
        Self {
            text: text.into(),
            styles,
            links: Vec::new(),
        }
    }
}

/// Where a table column's text sits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// At the column's start.
    #[default]
    Left,
    /// In the middle.
    Center,
    /// At the column's end.
    Right,
}

/// A list item: its task box, if any, and its blocks.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item {
    /// Done or not, for a task item.
    pub task: Option<bool>,
    /// Its blocks.
    pub blocks: Vec<Block>,
}

/// A block of a markdown document.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    /// A paragraph.
    Paragraph(Inline),
    /// A heading of this level, 1 to 6.
    Heading(u8, Inline),
    /// A quote and its blocks.
    Quote(Vec<Block>),
    /// A code block.
    Code {
        /// Its language, when named.
        language: Option<String>,
        /// Its text.
        text: String,
    },
    /// Items; numbered from `start` when ordered.
    List {
        /// The first item's number, when ordered.
        start: Option<u64>,
        /// Each item, the first numbered `start`.
        items: Vec<Item>,
    },
    /// A table: each cell's blocks.
    Table {
        /// Each column's alignment.
        aligns: Vec<Align>,
        /// The head row's cells.
        head: Vec<Vec<Block>>,
        /// The body rows' cells.
        rows: Vec<Vec<Vec<Block>>>,
    },
    /// A rule.
    Rule,
    /// A display formula's TeX.
    Math(String),
    /// A picture, and the link around it.
    Image {
        /// Its alt text.
        alt: String,
        /// Its address, relative to the document's folder.
        url: String,
        /// The address it links to.
        link: Option<String>,
    },
    /// A footnote definition, listed at the document's end.
    Footnote {
        /// Its label.
        label: String,
        /// Its blocks.
        body: Vec<Block>,
    },
    /// A GitHub alert: a quote under its title.
    Alert {
        /// The title to show, such as `注`.
        title: String,
        /// Its blocks.
        body: Vec<Block>,
    },
    /// `[toc]`: the document's headings.
    Toc,
}
