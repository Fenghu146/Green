//! Inline `$…$` as one atom in the document's own edit layer (patch 0006).

use std::ops::Range;

/// Single-dollar spans, skipping escaped dollars, doubled dollars and code.
pub(super) fn spans(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let (mut found, mut open, mut ix) = (Vec::new(), None, 0);
    while ix < bytes.len() {
        match bytes[ix] {
            b'\\' => ix += 2,
            b'`' => {
                let tick = ix;
                while ix < bytes.len() && bytes[ix] == b'`' {
                    ix += 1;
                }
                let run = ix - tick;
                while ix < bytes.len() {
                    if bytes[ix] == b'`' {
                        let start = ix;
                        while ix < bytes.len() && bytes[ix] == b'`' {
                            ix += 1;
                        }
                        if ix - start == run {
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

/// Which span holds `at`, when it is inside one.
fn holding(spans: &[Range<usize>], at: usize) -> Option<Range<usize>> {
    spans
        .iter()
        .find(|span| span.start < at && at < span.end)
        .cloned()
}

/// A caret inside a formula sits on the nearer end of it.
pub(super) fn snap_caret(text: &str, at: usize) -> usize {
    let Some(span) = holding(&spans(text), at) else {
        return at;
    };
    match at - span.start <= span.end - at {
        true => span.start,
        false => span.end,
    }
}

/// Whether `range` cuts into `span`, including a caret strictly inside it.
fn touching(span: &Range<usize>, range: &Range<usize>) -> bool {
    (range.start < span.end && range.end > span.start)
        || (range.is_empty() && span.start < range.start && range.start < span.end)
}

/// A typed/pasted `incoming` becomes a whole formula, or is refused.
fn written(incoming: &str) -> Option<String> {
    let trimmed = incoming.trim();
    if trimmed.is_empty() {
        return Some(String::new());
    }
    let inner = trimmed
        .strip_prefix('$')
        .and_then(|text| text.strip_suffix('$'))
        .unwrap_or(trimmed);
    if green_math::cached(inner).is_err() {
        return None;
    }
    Some(match trimmed.starts_with('$') && trimmed.ends_with('$') {
        true => trimmed.to_string(),
        false => format!("${inner}$"),
    })
}

/// One block field edit: widen a cut into a formula to the whole formula.
pub(super) fn edit_in(base: &str, range: Range<usize>, incoming: &str) -> (String, usize) {
    let plain = || {
        let mut text = base.to_string();
        text.replace_range(range.clone(), incoming);
        (text, range.start + incoming.len())
    };
    for span in spans(base) {
        // A whole formula already covered by the edit needs no widening.
        if span.start >= range.start && span.end <= range.end {
            continue;
        }
        if !touching(&span, &range) {
            continue;
        }
        let mut text = base.to_string();
        // One typed character accumulates inside the dollars; a deletion or
        // a paste takes the whole atom. Both refuse anything the converter
        // does not know, so an unbalanced `$` never reaches the document.
        let inside = match incoming.chars().count() {
            1 => {
                let body = &base[span.start + 1..span.end - 1];
                let from = range.start.saturating_sub(span.start + 1).min(body.len());
                let to = range
                    .end
                    .saturating_sub(span.start + 1)
                    .clamp(from, body.len());
                let next = format!("{}{}{}", &body[..from], incoming, &body[to..]);
                match green_math::cached(&next) {
                    Ok(_) => {
                        let caret = span.start + 1 + from + incoming.len();
                        let whole = format!("${next}$");
                        text.replace_range(span, &whole);
                        return (text, caret);
                    }
                    Err(_) => None,
                }
            }
            _ => written(incoming),
        };
        let replacement = match inside {
            Some(text) => text,
            None => {
                log::warn!("documents: an edit inside a formula was refused");
                return (base.to_string(), span.end);
            }
        };
        text.replace_range(span.clone(), &replacement);
        let caret = match replacement.is_empty() {
            true => span.start,
            false => span.start + replacement.len() - 1,
        };
        return (text, caret);
    }
    plain()
}

/// Document-side source edits take the same widening, in visual mode only.
pub(super) fn expand_edits(
    text: &str,
    edits: Vec<(Range<usize>, String)>,
) -> Vec<(Range<usize>, String)> {
    let found = spans(text);
    let mut wide: Vec<(Range<usize>, String)> = Vec::new();
    for (range, with) in edits {
        let hit = found.iter().find(|span| {
            !(span.start >= range.start && span.end <= range.end) && touching(span, &range)
        });
        let (range, with) = match hit {
            None => (range, with),
            Some(span) => match with.is_empty() {
                true => (span.clone(), with),
                false => match written(&with) {
                    Some(text) => (span.clone(), text),
                    None => {
                        log::warn!("documents: a replacement inside a formula was refused");
                        continue;
                    }
                },
            },
        };
        match wide.last_mut() {
            Some(last) if range.start <= last.0.end => {
                last.0.end = last.0.end.max(range.end);
                last.1.push_str(&with);
            }
            _ => wide.push((range, with)),
        }
    }
    wide
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dollars_in_code_and_escapes_are_not_formulas() {
        assert_eq!(spans("a $x$ ` $y$ ` \\$z b $q$"), vec![2..5, 20..23]);
    }

    #[test]
    fn a_cut_inside_widens_and_a_boundary_does_not() {
        let text = "通 $a_n$ 后";
        assert_eq!(edit_in(text, 5..6, "").0, "通  后");
        assert_eq!(edit_in(text, 3..3, "x").0, "通 x$a_n$ 后");
        assert_eq!(edit_in(text, 8..9, "").0, "通 $a_n$ 后".replace("$a_n$", ""));
    }

    #[test]
    fn typing_inside_wraps_and_a_bad_command_is_refused() {
        let text = "通 $a_n$ 后";
        assert_eq!(edit_in(text, 5..5, "x").0, "通 $xa_n$ 后");
        assert_eq!(edit_in(text, 5..5, "\\nope"), ("通 $a_n$ 后".to_string(), 9));
    }

    #[test]
    fn a_caret_snap_lands_on_the_nearer_end() {
        assert_eq!(snap_caret("a $x^2$ b", 5), 7);
        assert_eq!(snap_caret("a $x^2$ b", 4), 2);
    }
}
