//! LaTeX math to Typst markup: one converter for the editor, the outline and
//! the export (design §6). Whitelisted only; anything else is named, not guessed.

use std::{
    collections::HashMap,
    fmt,
    sync::{Mutex, OnceLock},
};

/// A command the whitelist does not cover.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsupported {
    /// The command as written, without its backslash.
    pub name: String,
    /// The byte where it starts in the source.
    pub at: usize,
}

impl fmt::Display for Unsupported {
    /// For logs: the UI words this through `mark.math.unsupported`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\\{} at byte {} is not in the formula whitelist",
            self.name,
            self.at + 1
        )
    }
}

/// Commands that mean one Typst symbol.
const SYMBOLS: &[(&str, &str)] = &[
    ("times", "times"),
    ("cdot", "dot.op"),
    ("pm", "plus.minus"),
    ("mp", "minus.plus"),
    ("div", "div"),
    ("bmod", "mod"),
    ("sim", "tilde.op"),
    ("approx", "approx"),
    ("equiv", "equiv"),
    ("le", "lt.eq"),
    ("leq", "lt.eq"),
    ("ge", "gt.eq"),
    ("geq", "gt.eq"),
    ("ne", "eq.not"),
    ("neq", "eq.not"),
    ("to", "arrow.r"),
    ("rightarrow", "arrow.r"),
    ("leftarrow", "arrow.l"),
    ("Rightarrow", "arrow.r.double"),
    ("sum", "sum"),
    ("prod", "prod"),
    ("int", "integral"),
    ("infty", "infinity"),
    ("dots", "dots.h"),
    ("cdots", "dots.h.c"),
    ("quad", "quad"),
    ("qquad", "quad quad"),
    ("alpha", "alpha"),
    ("beta", "beta"),
    ("gamma", "gamma"),
    ("delta", "delta"),
    ("epsilon", "epsilon"),
    ("theta", "theta"),
    ("lambda", "lambda"),
    ("mu", "mu"),
    ("pi", "pi"),
    ("sigma", "sigma"),
    ("phi", "phi"),
    ("omega", "omega"),
    ("Gamma", "Gamma"),
    ("Delta", "Delta"),
    ("Theta", "Theta"),
    ("Lambda", "Lambda"),
    ("Pi", "Pi"),
    ("Sigma", "Sigma"),
    ("Phi", "Phi"),
    ("Omega", "Omega"),
];

/// Characters that follow a backslash as themselves.
const ESCAPED: &[char] = &['{', '}', '%', '_', '&', '#', '$'];

/// The whitelist, for the message an unsupported command gets.
pub fn whitelisted(name: &str) -> bool {
    SYMBOLS.iter().any(|(known, _)| *known == name)
        || matches!(name, "frac" | "sqrt" | "text" | "boxed" | "left" | "right")
}

struct Reader<'a> {
    source: &'a str,
    chars: Vec<(usize, char)>,
    at: usize,
    errors: Vec<Unsupported>,
}

impl<'a> Reader<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().collect(),
            at: 0,
            errors: Vec::new(),
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).map(|(_, ch)| *ch)
    }

    fn offset(&self) -> usize {
        self.chars.get(self.at).map_or(self.source.len(), |(at, _)| *at)
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.at += 1;
        Some(ch)
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r')) {
            self.at += 1;
        }
    }

    /// Letters after a backslash.
    fn name(&mut self) -> String {
        let mut name = String::new();
        while let Some(ch) = self.peek().filter(|ch| ch.is_ascii_alphabetic()) {
            name.push(ch);
            self.at += 1;
        }
        name
    }

    /// Raw text to the matching brace, braces counted.
    fn raw_group(&mut self) -> Option<String> {
        self.skip_space();
        if self.peek() != Some('{') {
            return None;
        }
        self.at += 1;
        let mut text = String::new();
        let mut depth = 1usize;
        while let Some(ch) = self.bump() {
            match ch {
                '{' => {
                    depth += 1;
                    text.push(ch);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(text);
                    }
                    text.push(ch);
                }
                other => text.push(other),
            }
        }
        None
    }

    /// A group's inside, converted; a bare token when no braces.
    fn atom(&mut self) -> String {
        self.skip_space();
        if self.peek() == Some('{') {
            self.at += 1;
            let inner = self.sequence(Some('}'));
            if self.peek() == Some('}') {
                self.at += 1;
            }
            return inner;
        }
        match self.peek() {
            Some('\\') => {
                self.at += 1;
                self.command()
            }
            Some(_) => self.glyph(),
            None => String::new(),
        }
    }

    fn glyph(&mut self) -> String {
        let Some(ch) = self.bump() else {
            return String::new();
        };
        ch.to_string()
    }

    /// One command, its backslash read.
    fn command(&mut self) -> String {
        let at = self.offset().saturating_sub(1);
        match self.peek() {
            Some(ch) if !ch.is_ascii_alphabetic() => {
                let ch = self.bump().expect("a character");
                return match ch {
                    '\\' => "\\".into(),
                    ' ' | ',' | ';' | ':' | '!' => "space".into(),
                    other if ESCAPED.contains(&other) => other.to_string(),
                    other => {
                        self.errors.push(Unsupported {
                            name: other.to_string(),
                            at,
                        });
                        String::new()
                    }
                };
            }
            None => return String::new(),
            _ => {}
        }
        let name = self.name();
        match name.as_str() {
            "frac" => {
                let (up, down) = (self.atom(), self.atom());
                format!("frac({up}, {down})")
            }
            "sqrt" => {
                self.skip_space();
                let index = match self.peek() {
                    Some('[') => {
                        self.at += 1;
                        let inner = self.sequence(Some(']'));
                        if self.peek() == Some(']') {
                            self.at += 1;
                        }
                        Some(inner)
                    }
                    _ => None,
                };
                let body = self.atom();
                match index {
                    Some(index) => format!("root({index}, {body})"),
                    None => format!("sqrt({body})"),
                }
            }
            "text" => match self.raw_group() {
                Some(text) => format!("\"{}\"", quote(&text)),
                None => String::new(),
            },
            "boxed" => {
                let inner = self.atom();
                format!("#box(stroke: 0.6pt, inset: 3pt, radius: 1pt)[${inner}$]")
            }
            // Typst sizes delimiters by itself; the parens stay.
            "left" => {
                if self.peek() == Some('.') {
                    self.at += 1;
                }
                String::new()
            }
            "right" => {
                if self.peek() == Some('.') {
                    self.at += 1;
                }
                String::new()
            }
            _ => match SYMBOLS.iter().find(|(known, _)| *known == name) {
                Some((_, typst)) => (*typst).into(),
                None => {
                    self.errors.push(Unsupported { name, at });
                    String::new()
                }
            },
        }
    }

    /// A run of glyphs to `end`, or the end.
    fn sequence(&mut self, end: Option<char>) -> String {
        let mut out = String::new();
        while let Some(ch) = self.peek() {
            if Some(ch) == end {
                break;
            }
            match ch {
                '^' | '_' => {
                    self.at += 1;
                    let script = self.atom();
                    let needs = script.chars().count() > 1 && !script.starts_with('#');
                    out.push(ch);
                    match needs {
                        true => {
                            out.push('(');
                            out.push_str(&script);
                            out.push(')');
                        }
                        false => out.push_str(&script),
                    }
                }
                '\\' => {
                    self.at += 1;
                    let piece = self.command();
                    // A word meets a number with a space, or Typst reads one name.
                    let word = piece.chars().next().is_some_and(|ch| ch.is_ascii_alphabetic());
                    if word && out.chars().last().is_some_and(|ch| ch.is_ascii_alphanumeric()) {
                        out.push(' ');
                    }
                    out.push_str(&piece);
                    if word && self.peek().is_some_and(|ch| ch.is_ascii_alphanumeric()) {
                        out.push(' ');
                    }
                }
                '{' => {
                    self.at += 1;
                    let inner = self.sequence(Some('}'));
                    if self.peek() == Some('}') {
                        self.at += 1;
                    }
                    out.push('(');
                    out.push_str(&inner);
                    out.push(')');
                }
                '}' => break,
                ' ' | '\t' | '\n' | '\r' => {
                    self.skip_space();
                    if !out.ends_with(' ') && !out.is_empty() {
                        out.push(' ');
                    }
                }
                ch if ch.is_ascii_alphanumeric() => {
                    let mut run = String::new();
                    while let Some(ch) = self.peek().filter(|ch| ch.is_ascii_alphanumeric()) {
                        run.push(ch);
                        self.at += 1;
                    }
                    out.push_str(&identifiers(&run));
                }
                other => {
                    self.at += 1;
                    out.push(other);
                }
            }
        }
        out
    }
}

/// A run of letters and digits: numbers stay whole, a name means its letters
/// one by one (`B052H` is B, 0, 5, 2, H in italic, not a word in quotes).
fn identifiers(run: &str) -> String {
    match run.chars().all(|ch| ch.is_ascii_digit()) {
        true => run.to_string(),
        false => run
            .chars()
            .map(|ch| ch.to_string())
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// `text` as a Typst string's insides.
fn quote(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// `source` as Typst math markup; every unsupported command is named.
pub fn latex_to_typst(source: &str) -> Result<String, Vec<Unsupported>> {
    let mut reader = Reader::new(source);
    let out = reader.sequence(None);
    match reader.errors.is_empty() {
        true => Ok(out.trim().to_string()),
        false => Err(reader.errors),
    }
}

/// The same, remembered by `source`: one conversion however many draws.
pub fn cached(source: &str) -> Result<String, Vec<Unsupported>> {
    static SEEN: OnceLock<Mutex<HashMap<String, Result<String, Vec<Unsupported>>>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(found) = seen.lock().expect("the conversion cache").get(source) {
        return found.clone();
    }
    let converted = latex_to_typst(source);
    let mut seen = seen.lock().expect("the conversion cache");
    // Bounded: a long session keeps the latest, not everything ever typed.
    if seen.len() >= 512 {
        seen.clear();
    }
    seen.insert(source.to_string(), converted.clone());
    converted
}

#[cfg(test)]
mod tests;
