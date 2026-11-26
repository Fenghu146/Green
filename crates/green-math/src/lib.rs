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
    ("prod", "product"),
    ("int", "integral"),
    ("infty", "infinity"),
    ("dots", "dots.h"),
    ("cdots", "dots.h.c"),
    ("quad", "quad"),
    ("qquad", "quad quad"),
    // Sets and relations: the notes' "x ∈ ℝ" and its neighbours.
    ("in", "in"),
    ("notin", "in.not"),
    ("ni", "in.rev"),
    ("subset", "subset"),
    ("subseteq", "subset.eq"),
    ("supset", "supset"),
    ("supseteq", "supset.eq"),
    ("cup", "union"),
    ("cap", "inter"),
    ("emptyset", "emptyset"),
    ("varnothing", "emptyset"),
    ("forall", "forall"),
    ("exists", "exists"),
    ("neg", "not"),
    ("land", "and"),
    ("lor", "or"),
    ("Rightarrow", "arrow.r.double"),
    ("Leftrightarrow", "arrow.l.r.double"),
    ("implies", "arrow.r.double.long"),
    ("iff", "arrow.l.r.double.long"),
    ("partial", "partial"),
    ("nabla", "nabla"),
    ("ldots", "dots.h"),
    ("vdots", "dots.v"),
    ("ddots", "dots.down"),
    ("circ", "compose"),
    ("angle", "angle"),
    // Operators that name themselves: their arguments follow, not their size.
    ("lim", "lim"),
    ("ln", "ln"),
    ("log", "log"),
    ("exp", "exp"),
    ("sin", "sin"),
    ("cos", "cos"),
    ("tan", "tan"),
    ("arcsin", "arcsin"),
    ("arccos", "arccos"),
    ("arctan", "arctan"),
    ("max", "max"),
    ("min", "min"),
    ("sup", "sup"),
    ("inf", "inf"),
    ("det", "det"),
    ("alpha", "alpha"),
    ("beta", "beta"),
    ("gamma", "gamma"),
    ("delta", "delta"),
    ("epsilon", "epsilon"),
    ("theta", "theta"),
    ("lambda", "lambda"),
    ("mu", "mu"),
    ("pi", "pi"),
    ("rho", "rho"),
    ("varepsilon", "epsilon.alt"),
    ("sigma", "sigma"),
    ("phi", "phi"),
    ("omega", "omega"),
    ("nabla", "nabla"),
    ("Gamma", "Gamma"),
    ("Delta", "Delta"),
    ("Theta", "Theta"),
    ("Lambda", "Lambda"),
    ("Pi", "Pi"),
    ("Sigma", "Sigma"),
    ("Phi", "Phi"),
    ("Omega", "Omega"),
];

/// Commands that only change a glyph's size or style: the glyphs stay, the
/// command goes. Typst sizes its own delimiters.
const DROPPED: &[&str] = &[
    "big",
    "Big",
    "bigg",
    "Bigg",
    "bigl",
    "bigr",
    "Bigl",
    "Bigr",
    "biggl",
    "biggr",
    "Biggl",
    "Biggr",
    "displaystyle",
    "textstyle",
    "scriptstyle",
    "scriptscriptstyle",
    "limits",
    "nolimits",
];

/// Characters that follow a backslash as themselves.
const ESCAPED: &[char] = &['{', '}', '%', '_', '&', '#', '$'];

/// Symbols that sit between two operands: a space goes on both sides, so no
/// neighbouring token can fuse with them.
const OPERATORS: &[&str] = &[
    "times",
    "dot.op",
    "plus.minus",
    "minus.plus",
    "div",
    "mod",
    "tilde.op",
    "approx",
    "equiv",
    "lt.eq",
    "gt.eq",
    "eq.not",
    "arrow.r",
    "arrow.l",
    "arrow.r.double",
    // Written by the comparison glyphs, not read from a command.
    "lt",
    "gt",
];

/// The families formulas set their glyphs in: the math face, then the CJK one
/// `\text{…}` and bare Chinese fall back to.
pub const MATH_FONTS: &str = "(\"Noto Sans Math\", \"Noto Sans CJK SC\")";

/// The rule the editor's preamble and the export's both set maths with.
///
/// `EquationElem` forces its own font list on math, so a `#set text(font: …)`
/// outside this rule never reaches it. The two edges matter for formulas set
/// among words: without them the page box clips parentheses and descenders.
pub fn math_rule() -> String {
    format!(
        "#show math.equation: set text(font: {MATH_FONTS}, \
         top-edge: \"bounds\", bottom-edge: \"bounds\")"
    )
}

/// `#gbox`'s one definition, the frame in `stroke` (`#rrggbb`).
///
/// The converter writes `#gbox[…]` and nothing else: a box's width, inset,
/// radius and colour are the preamble's, so the editor and the export agree
/// and a theme change moves both.
pub fn gbox_rule(stroke: &str) -> String {
    format!("#let gbox(body) = box(stroke: 0.6pt + rgb(\"{stroke}\"), inset: 3pt, radius: 1pt, body)")
}

/// The whitelist, for the message an unsupported command gets.
pub fn whitelisted(name: &str) -> bool {
    SYMBOLS.iter().any(|(known, _)| *known == name)
        || DROPPED.contains(&name)
        || matches!(
            name,
            "frac" | "dfrac" | "sqrt" | "text" | "boxed" | "left" | "right" | "mathbb" | "begin"
                | "end" | "vec"
        )
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
        while matches!(self.peek(), Some(' ' | '\t' | '\n' | '\r' | '\u{200b}')) {
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
            return inner.trim_end().to_string();
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
                    // Spacing, as Typst names it.
                    ' ' => "space".into(),
                    ',' => "thin".into(),
                    ';' | ':' => "med".into(),
                    '!' => "#h(-0.167em)".into(),
                    // Escaped as Typst escapes it: `#`, `$` and `_` are syntax in math.
                    other if ESCAPED.contains(&other) => format!("\\{other}"),
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
            "begin" => self.environment(at),
            "end" => {
                // The environment's end was consumed with its body.
                self.raw_group();
                String::new()
            }
            "frac" | "dfrac" => {
                let (up, down) = (self.atom(), self.atom());
                format!("frac({up}, {down})")
            }
            "mathbb" => {
                let set = self.atom();
                format!("bb({set})")
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
                // The frame's style is the preamble's: this layer knows no theme.
                format!("#gbox[${inner}$]")
            }
            // An accent over its argument, as Typst draws one.
            "vec" => {
                let inner = self.atom();
                format!("arrow({inner})")
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
                // A size or style switch changes no meaning here.
                None if DROPPED.contains(&name.as_str()) => String::new(),
                None => {
                    self.errors.push(Unsupported { name, at });
                    String::new()
                }
            },
        }
    }

    /// `\begin{cases} … \end{cases}`: Typst's own `cases`, one argument a row.
    /// Any other environment is named, not guessed.
    fn environment(&mut self, at: usize) -> String {
        let Some(name) = self.raw_group() else {
            self.errors.push(Unsupported {
                name: "begin".into(),
                at,
            });
            return String::new();
        };
        match name.trim() {
            "cases" => self.cases(at),
            other => {
                self.errors.push(Unsupported {
                    name: format!("begin{{{other}}}"),
                    at,
                });
                String::new()
            }
        }
    }

    /// The body of `cases` to `\end{cases}`; rows break at `\\`.
    fn cases(&mut self, at: usize) -> String {
        const END: &str = "\\end{cases}";
        let start = self.offset();
        let rest = &self.source[start..];
        let Some(stop) = rest.find(END) else {
            self.errors.push(Unsupported {
                name: "begin{cases}".into(),
                at,
            });
            return String::new();
        };
        let body = rest[..stop].to_string();
        // Step past the body and its end marker.
        let mut left = stop + END.len();
        while left > 0 {
            let Some(ch) = self.bump() else { break };
            left -= ch.len_utf8();
        }
        let rows = rows(&body);
        // Typst's own `cases`, an argument a row; amsmath spaces the columns a
        // quad apart, and Typst's alignment point alone would run them on.
        let mut out = String::from("cases(");
        for (ix, row) in rows.iter().enumerate() {
            if ix > 0 {
                out.push_str(", ");
            }
            let cells: Vec<String> = (row.split('&'))
                .map(|cell| {
                    let mut reader = Reader::new(cell);
                    let converted = reader.sequence(None);
                    self.errors.extend(reader.errors);
                    converted.trim().to_string()
                })
                .filter(|cell| !cell.is_empty())
                .collect();
            out.push_str(&cells.join(" & quad "));
        }
        out.push(')');
        out
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
                    if OPERATORS.contains(&piece.as_str()) {
                        separated(&mut out, &piece);
                        continue;
                    }
                    // A word meets a number with a space, or Typst reads one name.
                    let word = piece
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '"' || ch == '#');
                    if word
                        && out
                            .chars()
                            .last()
                            .is_some_and(|ch| ch.is_ascii_alphanumeric())
                    {
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
                    out.push_str(inner.trim_end());
                    out.push(')');
                }
                '}' => break,
                ' ' | '\t' | '\n' | '\r' | '\u{200b}' => {
                    self.skip_space();
                    if !out.ends_with(' ') && !out.is_empty() {
                        out.push(' ');
                    }
                }
                // Comparisons are symbol names: `<-` is Typst's left arrow.
                '<' | '>' => {
                    self.at += 1;
                    let equal = self.peek() == Some('=');
                    if equal {
                        self.at += 1;
                    }
                    let piece = match (ch, equal) {
                        ('<', true) => "lt.eq",
                        ('<', false) => "lt",
                        (_, true) => "gt.eq",
                        (_, false) => "gt",
                    };
                    separated(&mut out, piece);
                }
                // CJK is text in math: Typst would read it as one long name.
                ch if is_cjk(ch) => {
                    let mut run = String::new();
                    while let Some(ch) = self.peek().filter(|ch| is_cjk(*ch)) {
                        run.push(ch);
                        self.at += 1;
                    }
                    separated(&mut out, &format!("\"{}\"", quote(&run)));
                }
                ch if ch.is_ascii_alphanumeric() => {
                    let mut run = String::new();
                    while let Some(ch) = self.peek().filter(|ch| ch.is_ascii_alphanumeric()) {
                        run.push(ch);
                        self.at += 1;
                    }
                    out.push_str(&identifiers(&run));
                }
                '=' => {
                    self.at += 1;
                    separated(&mut out, "=");
                }
                _ if matches!(ch, '+' | '-') && after_operand(&out) => {
                    self.at += 1;
                    separated(&mut out, &ch.to_string());
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

/// `cases` rows, broken at `\\`; a row's `[4pt]` spacing is the layout's.
fn rows(body: &str) -> Vec<String> {
    let mut rows = Vec::new();
    let mut current = String::new();
    let mut chars = body.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'\\') {
            chars.next();
            rows.push(std::mem::take(&mut current));
            if chars.peek() == Some(&'[') {
                chars.next();
                for ch in chars.by_ref() {
                    if ch == ']' {
                        break;
                    }
                }
            }
            continue;
        }
        current.push(ch);
    }
    rows.push(current);
    rows
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

/// `piece` with a space on both sides, so no neighbour can fuse with it.
fn separated(out: &mut String, piece: &str) {
    if !out.is_empty() && !out.ends_with(' ') {
        out.push(' ');
    }
    out.push_str(piece);
    out.push(' ');
}

/// Whether the markup so far ends in an operand, making `+` or `-` binary.
fn after_operand(out: &str) -> bool {
    let tail = out.trim_end();
    // A relation or operation name (`lt`, `times`) is not an operand: the sign
    // after it is unary.
    if OPERATORS.iter().any(|name| tail.ends_with(name)) {
        return false;
    }
    tail.chars()
        .last()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ')' | ']' | '"'))
}

/// A character Typst reads as a name: CJK ideographs, kana, hangul and their
/// punctuation, which only print inside a string.
fn is_cjk(ch: char) -> bool {
    matches!(ch,
        '\u{2E80}'..='\u{2FFF}'
            | '\u{3000}'..='\u{303F}'
            | '\u{3040}'..='\u{30FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{AC00}'..='\u{D7AF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{FF00}'..='\u{FFEF}'
            | '\u{20000}'..='\u{2FA1F}'
    )
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
    static SEEN: OnceLock<Mutex<HashMap<String, Result<String, Vec<Unsupported>>>>> =
        OnceLock::new();
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
