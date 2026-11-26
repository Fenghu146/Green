//! The nine blocks a plugin may return, and how they read as Markdown.
//!
//! A kind this build does not know is dropped whole; an unknown field inside
//! a known block is ignored, so either side may grow first (design §3).

use serde::Deserialize;
use serde_json::Value;

/// One block of a plugin's view.
#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    /// A paragraph; its text is inline Markdown.
    Paragraph(String),
    /// A heading, level 1 to 6.
    Heading { level: u8, text: String },
    /// A list; each item is inline Markdown.
    List {
        ordered: bool,
        start: u64,
        items: Vec<String>,
    },
    /// A code block.
    Code {
        language: Option<String>,
        text: String,
    },
    /// A quote and the blocks inside it.
    Quote(Vec<Block>),
    /// A table.
    Table {
        aligns: Vec<Align>,
        head: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    /// A picture, and the link around it.
    Image {
        url: String,
        alt: String,
        link: Option<String>,
    },
    /// A horizontal rule.
    Rule,
    /// A GitHub alert.
    Alert { level: AlertLevel, text: String },
}

/// A table column's alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    None,
    Left,
    Center,
    Right,
}

impl Align {
    fn of(name: &str) -> Align {
        match name.trim().to_ascii_lowercase().as_str() {
            "left" => Align::Left,
            "center" | "centre" => Align::Center,
            "right" => Align::Right,
            _ => Align::None,
        }
    }

    fn marker(self) -> &'static str {
        match self {
            Align::None => "---",
            Align::Left => ":---",
            Align::Center => ":---:",
            Align::Right => "---:",
        }
    }
}

/// An alert's kind, as GitHub names them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlertLevel {
    Note,
    Tip,
    Important,
    Warning,
    Caution,
}

impl AlertLevel {
    fn of(name: &str) -> Result<AlertLevel, String> {
        match name.trim().to_ascii_lowercase().as_str() {
            "note" => Ok(AlertLevel::Note),
            "tip" => Ok(AlertLevel::Tip),
            "important" => Ok(AlertLevel::Important),
            "warning" => Ok(AlertLevel::Warning),
            "caution" => Ok(AlertLevel::Caution),
            other => Err(format!("alert: {other:?} is not a level")),
        }
    }

    fn title(self) -> &'static str {
        match self {
            AlertLevel::Note => "NOTE",
            AlertLevel::Tip => "TIP",
            AlertLevel::Important => "IMPORTANT",
            AlertLevel::Warning => "WARNING",
            AlertLevel::Caution => "CAUTION",
        }
    }
}

/// A block read from `value`, or why it is not one.
pub fn parse(value: &Value) -> Result<Block, String> {
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .ok_or("the block has no kind")?;
    match kind {
        "paragraph" => {
            #[derive(Deserialize)]
            struct P {
                text: String,
            }
            Ok(Block::Paragraph(read::<P>(value, kind)?.text))
        }
        "heading" => {
            #[derive(Deserialize)]
            struct H {
                level: u8,
                text: String,
            }
            let heading = read::<H>(value, kind)?;
            if !(1..=6).contains(&heading.level) {
                return Err(format!("heading: level {} is outside 1..=6", heading.level));
            }
            Ok(Block::Heading {
                level: heading.level,
                text: heading.text,
            })
        }
        "list" => {
            #[derive(Deserialize)]
            struct L {
                #[serde(default)]
                ordered: bool,
                #[serde(default = "one")]
                start: u64,
                items: Vec<String>,
            }
            let list = read::<L>(value, kind)?;
            Ok(Block::List {
                ordered: list.ordered,
                start: list.start.max(1),
                items: list.items,
            })
        }
        "code" => {
            #[derive(Deserialize)]
            struct C {
                #[serde(default)]
                language: Option<String>,
                text: String,
            }
            let code = read::<C>(value, kind)?;
            Ok(Block::Code {
                language: code.language.filter(|name| !name.trim().is_empty()),
                text: code.text,
            })
        }
        "quote" => {
            #[derive(Deserialize)]
            struct Q {
                blocks: Vec<Value>,
            }
            let quote = read::<Q>(value, kind)?;
            Ok(Block::Quote(nested(quote.blocks)))
        }
        "table" => {
            #[derive(Deserialize)]
            struct T {
                #[serde(default)]
                aligns: Vec<String>,
                head: Vec<String>,
                #[serde(default)]
                rows: Vec<Vec<String>>,
            }
            let table = read::<T>(value, kind)?;
            if table.head.is_empty() {
                return Err("table: head is empty".into());
            }
            Ok(Block::Table {
                aligns: table.aligns.iter().map(|name| Align::of(name)).collect(),
                head: table.head,
                rows: table.rows,
            })
        }
        "image" => {
            #[derive(Deserialize)]
            struct I {
                url: String,
                #[serde(default)]
                alt: String,
                #[serde(default)]
                link: Option<String>,
            }
            let image = read::<I>(value, kind)?;
            if image.url.trim().is_empty() {
                return Err("image: url is empty".into());
            }
            Ok(Block::Image {
                url: image.url,
                alt: image.alt,
                link: image.link,
            })
        }
        "rule" => Ok(Block::Rule),
        "alert" => {
            #[derive(Deserialize)]
            struct A {
                level: String,
                text: String,
            }
            let alert = read::<A>(value, kind)?;
            Ok(Block::Alert {
                level: AlertLevel::of(&alert.level)?,
                text: alert.text,
            })
        }
        other => Err(format!("{other:?} is not a block kind")),
    }
}

fn one() -> u64 {
    1
}

/// What a plugin writes on stdout.
#[derive(Clone, Debug, PartialEq)]
pub struct Response {
    /// The contract the plugin answered in.
    pub api_version: u32,
    /// Every block this build knows.
    pub blocks: Vec<Block>,
    /// Whatever the plugin put beside its blocks.
    pub meta: Value,
    /// How many blocks this build dropped as unknown or malformed.
    pub skipped: usize,
}

impl Response {
    /// `bytes` as a response, or why it is not one.
    pub fn parse(bytes: &[u8]) -> Result<Response, String> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Raw {
            api_version: u32,
            #[serde(default)]
            blocks: Vec<Value>,
            #[serde(default)]
            meta: Value,
        }
        let raw: Raw = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        let mut blocks = Vec::new();
        let mut skipped = 0;
        for (at, value) in raw.blocks.iter().enumerate() {
            match parse(value) {
                Ok(block) => blocks.push(block),
                Err(why) => {
                    skipped += 1;
                    log::warn!("plugins: block {at} is dropped: {why}");
                }
            }
        }
        Ok(Response {
            api_version: raw.api_version,
            blocks,
            meta: raw.meta,
            skipped,
        })
    }
}

fn read<T: serde::de::DeserializeOwned>(value: &Value, kind: &str) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| format!("{kind}: {error}"))
}

/// The blocks of a quote, each bad one dropped.
fn nested(blocks: Vec<Value>) -> Vec<Block> {
    blocks
        .iter()
        .enumerate()
        .filter_map(|(at, value)| match parse(value) {
            Ok(block) => Some(block),
            Err(why) => {
                log::warn!("plugins: quote block {at} is dropped: {why}");
                None
            }
        })
        .collect()
}

impl Block {
    /// This block as Markdown.
    pub fn markdown(&self) -> String {
        match self {
            Block::Paragraph(text) => text.trim_end().to_string(),
            Block::Heading { level, text } => {
                format!("{} {}", "#".repeat(usize::from(*level)), text.trim())
            }
            Block::List {
                ordered,
                start,
                items,
            } => items
                .iter()
                .enumerate()
                .map(|(at, item)| {
                    let marker = match ordered {
                        true => format!("{}. ", start + at as u64),
                        false => "- ".to_string(),
                    };
                    let indent = " ".repeat(marker.len());
                    let item = item.trim_end().replace('\n', &format!("\n{indent}"));
                    format!("{marker}{item}")
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Block::Code { language, text } => {
                let fence = "`".repeat(fence_for(text));
                let language = language.as_deref().unwrap_or("").trim();
                format!(
                    "{fence}{language}\n{}\n{fence}",
                    text.trim_end_matches('\n')
                )
            }
            Block::Quote(blocks) => quoted(&to_markdown(blocks)),
            Block::Table { aligns, head, rows } => table(aligns, head, rows),
            Block::Image { url, alt, link } => {
                let alt = alt.replace('\\', "\\\\").replace(']', "\\]");
                let picture = format!("![{alt}]({url})");
                match link {
                    Some(link) => format!("[{picture}]({link})"),
                    None => picture,
                }
            }
            Block::Rule => "---".to_string(),
            Block::Alert { level, text } => {
                let mut lines = vec![format!("> [!{}]", level.title())];
                lines.extend(text.trim_end().lines().map(|line| format!("> {line}")));
                lines.join("\n")
            }
        }
    }
}

/// `blocks` as one Markdown text, a blank line between blocks.
pub fn to_markdown(blocks: &[Block]) -> String {
    let parts: Vec<String> = blocks
        .iter()
        .map(Block::markdown)
        .filter(|part| !part.trim().is_empty())
        .collect();
    if parts.is_empty() {
        return String::new();
    }
    format!("{}\n", parts.join("\n\n"))
}

fn quoted(markdown: &str) -> String {
    markdown
        .trim_end()
        .lines()
        .map(|line| match line.is_empty() {
            true => ">".to_string(),
            false => format!("> {line}"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn table(aligns: &[Align], head: &[String], rows: &[Vec<String>]) -> String {
    let columns = head.len();
    let cell = |text: &str| text.replace('|', "\\|").replace(['\n', '\r'], "<br>");
    let row = |cells: &[String]| {
        let mut cells: Vec<String> = cells.iter().take(columns).map(|text| cell(text)).collect();
        cells.resize(columns, String::new());
        format!("| {} |", cells.join(" | "))
    };
    let mut lines = vec![row(head)];
    let mut markers: Vec<&str> = (0..columns)
        .map(|at| aligns.get(at).copied().unwrap_or_default().marker())
        .collect();
    markers.resize(columns, "---");
    lines.push(format!("| {} |", markers.join(" | ")));
    lines.extend(rows.iter().map(|cells| row(cells)));
    lines.join("\n")
}

/// A fence longer than any run of backticks in `text`.
fn fence_for(text: &str) -> usize {
    let mut longest = 0;
    let mut run = 0;
    for byte in text.bytes() {
        run = match byte {
            b'`' => run + 1,
            _ => 0,
        };
        longest = longest.max(run);
    }
    longest.max(2) + 1
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn parsed(value: serde_json::Value) -> Result<Block, String> {
        parse(&value)
    }

    #[test]
    fn every_kind_reads() {
        assert_eq!(
            parsed(json!({"kind": "paragraph", "text": "Hi"})).expect("a paragraph"),
            Block::Paragraph("Hi".into())
        );
        assert_eq!(
            parsed(json!({"kind": "heading", "level": 2, "text": "T"})).expect("a heading"),
            Block::Heading {
                level: 2,
                text: "T".into()
            }
        );
        assert_eq!(
            parsed(json!({"kind": "list", "ordered": true, "start": 3, "items": ["a", "b"]}))
                .expect("a list"),
            Block::List {
                ordered: true,
                start: 3,
                items: vec!["a".into(), "b".into()]
            }
        );
        assert_eq!(
            parsed(json!({"kind": "code", "language": "rs", "text": "fn f() {}"})).expect("code"),
            Block::Code {
                language: Some("rs".into()),
                text: "fn f() {}".into()
            }
        );
        assert_eq!(
            parsed(json!({"kind": "quote", "blocks": [
                {"kind": "paragraph", "text": "in"},
                {"kind": "future", "text": "gone"}
            ]}))
            .expect("a quote"),
            Block::Quote(vec![Block::Paragraph("in".into())])
        );
        assert_eq!(
            parsed(json!({"kind": "table", "head": ["a"], "rows": [["1"]]})).expect("a table"),
            Block::Table {
                aligns: vec![],
                head: vec!["a".into()],
                rows: vec![vec!["1".into()]]
            }
        );
        assert_eq!(
            parsed(json!({"kind": "image", "url": "a.png"})).expect("an image"),
            Block::Image {
                url: "a.png".into(),
                alt: String::new(),
                link: None
            }
        );
        assert_eq!(
            parsed(json!({"kind": "rule"})).expect("a rule"),
            Block::Rule
        );
        assert_eq!(
            parsed(json!({"kind": "alert", "level": "warning", "text": "Careful"})).expect("alert"),
            Block::Alert {
                level: AlertLevel::Warning,
                text: "Careful".into()
            }
        );
    }

    #[test]
    fn what_is_not_a_block_is_dropped_whole() {
        assert!(parsed(json!({"kind": "future"})).is_err());
        assert!(parsed(json!({"text": "no kind"})).is_err());
        assert!(parsed(json!({"kind": "heading", "level": 7, "text": "T"})).is_err());
        assert!(parsed(json!({"kind": "heading", "text": "T"})).is_err());
        assert!(parsed(json!({"kind": "table", "head": []})).is_err());
        assert!(parsed(json!({"kind": "alert", "level": "shout", "text": "T"})).is_err());
    }

    #[test]
    fn a_field_from_a_later_contract_is_ignored() {
        let block = parsed(json!({
            "kind": "paragraph", "text": "Hi", "footnote": "later"
        }))
        .expect("a paragraph");
        assert_eq!(block, Block::Paragraph("Hi".into()));
    }

    #[test]
    fn a_table_reads_as_gfm() {
        let blocks = [
            Block::Table {
                aligns: vec![Align::Left, Align::Right],
                head: vec!["name".into(), "qty".into()],
                rows: vec![vec!["a|b".into(), "3".into()], vec!["line\nbreak".into()]],
            },
            Block::Paragraph("After.".into()),
        ];
        assert_eq!(
            to_markdown(&blocks),
            "| name | qty |\n| :--- | ---: |\n| a\\|b | 3 |\n| line<br>break |  |\n\nAfter.\n"
        );
    }

    #[test]
    fn every_kind_reads_back_as_markdown() {
        let blocks = vec![
            Block::Heading {
                level: 1,
                text: "Title".into(),
            },
            Block::Paragraph("Text.".into()),
            Block::List {
                ordered: false,
                start: 1,
                items: vec!["one".into(), "two".into()],
            },
            Block::List {
                ordered: true,
                start: 1,
                items: vec!["first".into()],
            },
            Block::Code {
                language: Some("py".into()),
                text: "print(`x`)".into(),
            },
            Block::Quote(vec![Block::Paragraph("Quoted.".into())]),
            Block::Image {
                url: "a.png".into(),
                alt: "A".into(),
                link: Some("https://example.com".into()),
            },
            Block::Rule,
            Block::Alert {
                level: AlertLevel::Note,
                text: "Read me.".into(),
            },
        ];
        assert_eq!(
            to_markdown(&blocks),
            "# Title\n\nText.\n\n- one\n- two\n\n1. first\n\n```py\nprint(`x`)\n```\n\n> Quoted.\n\n\
             [![A](a.png)](https://example.com)\n\n---\n\n> [!NOTE]\n> Read me.\n"
        );
    }

    #[test]
    fn a_fence_outgrows_its_text() {
        assert_eq!(fence_for("plain"), 3);
        assert_eq!(fence_for("```"), 4);
        assert_eq!(fence_for("`````"), 6);
    }
}
