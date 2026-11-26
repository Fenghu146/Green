//! Just enough JSON for one event per line.

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    /// An object or array, which no event field uses.
    Compound,
}

pub fn object(text: &str) -> Result<Vec<(String, Value)>, String> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        at: 0,
        fields: Vec::new(),
    };
    parser.skip_ws();
    parser.object()?;
    parser.skip_ws();
    if parser.at != parser.bytes.len() {
        return Err(format!("trailing text at byte {}", parser.at));
    }
    Ok(parser.fields)
}

pub fn string_field<'a>(fields: &'a [(String, Value)], key: &str) -> Option<&'a str> {
    match field(fields, key)? {
        Value::Str(text) => Some(text),
        _ => None,
    }
}

pub fn number_field(fields: &[(String, Value)], key: &str) -> Option<f64> {
    match field(fields, key)? {
        Value::Num(number) => Some(*number),
        _ => None,
    }
}

fn field<'a>(fields: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    fields
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
    fields: Vec<(String, Value)>,
}

impl Parser<'_> {
    fn object(&mut self) -> Result<(), String> {
        self.expect(b'{')?;
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(());
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            let value = self.value()?;
            self.fields.push((key, value));
            self.skip_ws();
            match self.take() {
                Some(b',') => continue,
                Some(b'}') => return Ok(()),
                _ => return Err(format!("expected ',' or '}}' at byte {}", self.at)),
            }
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b'{') => {
                let mut nested = Parser {
                    bytes: self.bytes,
                    at: self.at,
                    fields: Vec::new(),
                };
                nested.object()?;
                self.at = nested.at;
                Ok(Value::Compound)
            }
            Some(b'[') => {
                self.array()?;
                Ok(Value::Compound)
            }
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            _ => self.number(),
        }
    }

    fn array(&mut self) -> Result<(), String> {
        self.expect(b'[')?;
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(());
        }
        loop {
            self.skip_ws();
            let mut nested = Parser {
                bytes: self.bytes,
                at: self.at,
                fields: Vec::new(),
            };
            nested.value()?;
            self.at = nested.at;
            self.skip_ws();
            match self.take() {
                Some(b',') => continue,
                Some(b']') => return Ok(()),
                _ => return Err(format!("expected ',' or ']' at byte {}", self.at)),
            }
        }
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, String> {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(format!("unknown literal at byte {}", self.at))
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.at;
        while matches!(
            self.peek(),
            Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        ) {
            self.at += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.at]).expect("ascii digits");
        match text.parse::<f64>() {
            Ok(number) if number.is_finite() => Ok(Value::Num(number)),
            _ => Err(format!("not a number at byte {start}: {text:?}")),
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let byte = self
                .take()
                .ok_or_else(|| String::from("unterminated string"))?;
            match byte {
                b'"' => return Ok(out),
                b'\\' => out.push(self.escape()?),
                0x00..=0x1f => {
                    return Err(format!("raw control byte in string at {}", self.at - 1));
                }
                _ => {
                    // Copy the whole UTF-8 sequence the first byte starts.
                    let len = utf8_len(byte);
                    let from = self.at - 1;
                    let end = (from + len).min(self.bytes.len());
                    let text = std::str::from_utf8(&self.bytes[from..end])
                        .map_err(|_| format!("bad UTF-8 at byte {from}"))?;
                    out.push_str(text);
                    self.at = end;
                }
            }
        }
    }

    fn escape(&mut self) -> Result<char, String> {
        match self.take() {
            Some(b'"') => Ok('"'),
            Some(b'\\') => Ok('\\'),
            Some(b'/') => Ok('/'),
            Some(b'b') => Ok('\u{8}'),
            Some(b'f') => Ok('\u{c}'),
            Some(b'n') => Ok('\n'),
            Some(b'r') => Ok('\r'),
            Some(b't') => Ok('\t'),
            Some(b'u') => {
                let mut code = self.hex4()? as u32;
                // A high surrogate is only whole with the low one after it.
                if (0xd800..0xdc00).contains(&code) && self.bytes[self.at..].starts_with(b"\\u") {
                    self.at += 2;
                    let low = self.hex4()? as u32;
                    code = 0x10000 + ((code - 0xd800) << 10) + (low.wrapping_sub(0xdc00));
                }
                char::from_u32(code).ok_or_else(|| format!("no character for U+{code:04X}"))
            }
            other => Err(format!(
                "bad escape \\{} at byte {}",
                other.map_or('?', char::from),
                self.at
            )),
        }
    }

    fn hex4(&mut self) -> Result<u16, String> {
        let end = self.at + 4;
        let text = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| String::from("short \\u escape"))?;
        let text = std::str::from_utf8(text).map_err(|_| String::from("bad \\u escape"))?;
        let value =
            u16::from_str_radix(text, 16).map_err(|_| format!("bad \\u escape {text:?}"))?;
        self.at = end;
        Ok(value)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.at += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn take(&mut self) -> Option<u8> {
        let byte = self.peek();
        if byte.is_some() {
            self.at += 1;
        }
        byte
    }

    fn expect(&mut self, want: u8) -> Result<(), String> {
        match self.take() {
            Some(byte) if byte == want => Ok(()),
            _ => Err(format!(
                "expected '{}' at byte {}",
                char::from(want),
                self.at.saturating_sub(1)
            )),
        }
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_event() {
        let fields = object(r#"{"metric":"k2p","ms":7.4}"#).expect("valid");
        assert_eq!(string_field(&fields, "metric"), Some("k2p"));
        assert_eq!(number_field(&fields, "ms"), Some(7.4));
        assert_eq!(number_field(&fields, "absent"), None);
    }

    #[test]
    fn escapes_do_not_end_the_string_early() {
        let fields = object(r#"{"doc":"a\"b\u00e9.md","ms":1}"#).expect("valid");
        assert_eq!(string_field(&fields, "doc"), Some("a\"bé.md"));
    }

    #[test]
    fn keeps_non_ascii_text() {
        let fields = object(r#"{"doc":"作业.md","ms":2}"#).expect("valid");
        assert_eq!(string_field(&fields, "doc"), Some("作业.md"));
    }

    #[test]
    fn skips_nested_values_without_losing_the_next_key() {
        let fields = object(r#"{"meta":{"a":[1,{"b":2}]},"ms":3}"#).expect("valid");
        assert_eq!(number_field(&fields, "ms"), Some(3.0));
        assert_eq!(fields[0].1, Value::Compound);
    }

    #[test]
    fn rejects_what_it_cannot_read() {
        assert!(object(r#"{"ms":1} trailing"#).is_err());
        assert!(object(r#"{"ms":}"#).is_err());
        assert!(object(r#"{"ms":1"#).is_err());
        assert!(object("[]").is_err());
    }
}
