//! Folding a heading's blocks, and the keys a file remembers (design §5).

use std::collections::{HashMap, HashSet};

use gpui::{App, Context, Window};

use super::{DocumentEvent, MarkdownDocument, TargetBlock, parse::Block};
use crate::tree::DocumentHeading;

/// A top-level heading that can fold.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Head {
    /// Its key across edits: its text and its ordinal among headings its level.
    pub key: String,
    pub level: u8,
    /// One past the last block it folds.
    pub end: usize,
    /// Whether any block sits under it.
    pub opens: bool,
}

/// What is hidden, and which heading hides it.
#[derive(Clone, Debug, Default)]
pub(crate) struct Folds {
    heads: HashMap<usize, Head>,
    /// Per block, the outermost folded heading hiding it.
    hidden: Vec<Option<usize>>,
}

impl Folds {
    /// The folds `folded` names among `blocks`; headings give the keys.
    pub(crate) fn new(
        blocks: &[Block],
        headings: &[DocumentHeading],
        keys: &[String],
        folded: &HashSet<String>,
    ) -> Folds {
        let keys_at: HashMap<usize, &String> = headings
            .iter()
            .zip(keys)
            .map(|(heading, key)| (heading.at, key))
            .collect();
        let mut heads = HashMap::new();
        for (ix, block) in blocks.iter().enumerate() {
            let TargetBlock::Heading(level) = block.kind else {
                continue;
            };
            let Some(key) = keys_at.get(&block.range.start) else {
                // A heading inside a quote or list folds with its outer block only.
                continue;
            };
            let end = (ix + 1..blocks.len())
                .find(|at| matches!(blocks[*at].kind, TargetBlock::Heading(other) if other <= level))
                .unwrap_or(blocks.len());
            heads.insert(
                ix,
                Head {
                    key: (*key).clone(),
                    level,
                    end,
                    opens: end > ix + 1,
                },
            );
        }
        let mut hidden = vec![None; blocks.len()];
        for (ix, head) in &heads {
            if !folded.contains(&head.key) || !head.opens {
                continue;
            }
            for inside in ix + 1..head.end {
                hidden[inside].get_or_insert(*ix);
            }
        }
        Folds { heads, hidden }
    }

    /// The heading block `block` is, when it is one.
    pub(crate) fn head(&self, block: usize) -> Option<&Head> {
        self.heads.get(&block)
    }

    /// The outermost folded heading hiding `block`.
    pub(crate) fn hides(&self, block: usize) -> Option<usize> {
        self.hidden.get(block).copied().flatten()
    }

    /// Whether a folded heading hides `block`.
    pub(crate) fn hidden(&self, block: usize) -> bool {
        self.hides(block).is_some()
    }
}

/// A heading's key: its level, its ordinal among headings of that level, and
/// its text. Two headings with one text still differ by their ordinal.
pub(crate) fn key(level: u8, ordinal: usize, text: &str) -> String {
    format!("{level}|{ordinal}|{text}")
}

/// Keys and parents for `headings`, in document order.
pub(crate) fn keys(headings: &[DocumentHeading]) -> (Vec<String>, Vec<Option<usize>>) {
    let mut counts = [0usize; 7];
    let mut keys = Vec::with_capacity(headings.len());
    let mut parents = Vec::with_capacity(headings.len());
    let mut open: Vec<(u8, usize)> = Vec::new();
    for (ix, heading) in headings.iter().enumerate() {
        let level = heading.depth.clamp(1, 6);
        let ordinal = counts[usize::from(level)];
        counts[usize::from(level)] += 1;
        keys.push(key(level, ordinal, &heading.title));
        while open.last().is_some_and(|(depth, _)| *depth >= level) {
            open.pop();
        }
        parents.push(open.last().map(|(_, at)| *at));
        open.push((level, ix));
    }
    (keys, parents)
}

/// A heading as the outline tree shows it: no source marks, and its place in
/// the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct OutlineHeading {
    /// Its level, 1 to 6.
    pub depth: u8,
    /// Its text, without `#`, `**`, backticks or `$`.
    pub title: String,
    /// Its line, from zero.
    pub line: usize,
    /// Where its source starts, in bytes.
    pub at: usize,
    /// Its key in the file's view state.
    pub key: String,
    /// The heading it sits under, by index.
    pub parent: Option<usize>,
    /// Whether headings sit under it.
    pub opens: bool,
}

/// `title` as the outline shows it: the marks leave, the words stay.
pub(crate) fn plain_title(title: &str) -> String {
    let title = title.trim_start_matches(['#', ' ', '\t']);
    let mut out = String::with_capacity(title.len());
    let mut rest = title.chars().peekable();
    while let Some(ch) = rest.next() {
        match ch {
            // A formula keeps its text and loses its dollars.
            '$' => {}
            '`' => {}
            '*' if rest.peek() == Some(&'*') => {
                rest.next();
            }
            _ => out.push(ch),
        }
    }
    out.trim().to_string()
}

impl MarkdownDocument {
    /// Recomputes what the folded keys hide after a read or a toggle.
    pub(super) fn refold(&mut self) {
        self.folds = Folds::new(
            &self.parsed.blocks,
            &self.parsed.headings,
            &self.parsed.keys,
            &self.folded,
        );
    }

    /// The folded keys, sorted, as a file's view state keeps them.
    pub fn folded_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.folded.iter().cloned().collect();
        keys.sort();
        keys
    }

    /// Folds exactly `keys`; the list lays out again around them.
    pub fn set_folded(
        &mut self,
        keys: impl IntoIterator<Item = String>,
        cx: &mut Context<Self>,
    ) {
        let folded: HashSet<String> = keys.into_iter().collect();
        if folded == self.folded {
            return;
        }
        self.folded = folded;
        self.refold();
        let count = self.list.item_count();
        self.list.remeasure_items(0..count);
        cx.emit(DocumentEvent::Folded);
        cx.notify();
    }

    /// Whether a folded heading hides block `block`.
    pub fn hidden(&self, block: usize) -> bool {
        self.folds.hidden(block)
    }

    /// The heading block `block` is, with whether it has blocks under it.
    pub(crate) fn fold_head(&self, block: usize) -> Option<(String, u8, bool, bool)> {
        let head = self.folds.head(block)?;
        Some((
            head.key.clone(),
            head.level,
            head.opens,
            self.folded.contains(&head.key),
        ))
    }

    /// Folds or unfolds the heading at `block`; the answer is whether it now
    /// folds, and None when the block folds nothing.
    pub fn toggle_fold(
        &mut self,
        block: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        let head = self.folds.head(block)?.clone();
        if !head.opens {
            return None;
        }
        let folding = !self.folded.contains(&head.key);
        if folding {
            // A block being hidden writes back first, so no field hides.
            let inside = self
                .open
                .as_ref()
                .is_some_and(|open| (block + 1..head.end).contains(&self.block_at(open.range.start)));
            if inside && self.commit(cx).is_some() {
                self.reparse(true, cx);
            }
            // What is on screen now is about to hide: keep its layout, so
            // unfolding reuses it instead of shaping it again.
            window.text_system().keep_frame();
            self.folded.insert(head.key.clone());
        } else {
            self.folded.remove(&head.key);
        }
        let head = self.folds.head(block).cloned().unwrap_or(head);
        self.refold();
        self.list.remeasure_items(block + 1..head.end);
        log::info!(
            "markdown document: {} the heading at block {block} (level {})",
            if folding { "folds" } else { "unfolds" },
            head.level
        );
        cx.emit(DocumentEvent::Folded);
        cx.notify();
        Some(folding)
    }

    /// Folds or unfolds the heading whose source starts at `at`.
    pub fn toggle_fold_at(
        &mut self,
        at: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        let block = self.block_at(at);
        self.toggle_fold(block, window, cx)
    }

    /// Unfolds every heading hiding `block`; true when one did.
    pub fn unfold_block(&mut self, block: usize, cx: &mut Context<Self>) -> bool {
        let (mut from, mut to, mut changed) = (usize::MAX, 0, false);
        while let Some(head) = self.folds.hides(block) {
            let Some(found) = self.folds.head(head).cloned() else {
                break;
            };
            self.folded.remove(&found.key);
            from = from.min(head + 1);
            to = to.max(found.end);
            changed = true;
            self.refold();
        }
        if changed {
            self.list.remeasure_items(from..to);
            cx.emit(DocumentEvent::Folded);
            cx.notify();
        }
        changed
    }

    /// Unfolds every heading hiding the block at byte `at`; true when one did.
    pub fn unfold_at(&mut self, at: usize, cx: &mut Context<Self>) -> bool {
        let block = self.block_at(at);
        self.unfold_block(block, cx)
    }

    /// The first block a fold does not hide, from `next` and walking `down`.
    pub(super) fn shown_from(&self, next: Option<usize>, down: bool) -> Option<usize> {
        let len = self.parsed.blocks.len();
        let mut at = next;
        while let Some(ix) = at {
            if !self.folds.hidden(ix) {
                return Some(ix);
            }
            at = match down {
                true => Some(ix + 1).filter(|ix| *ix < len),
                false => ix.checked_sub(1),
            };
        }
        None
    }

    /// The headings as a tree, with their keys and parents, for the outline.
    pub fn outline_tree(&self, cx: &App) -> Vec<OutlineHeading> {
        let editor = self.editor.read(cx);
        let len = editor.text().len();
        self.parsed
            .headings
            .iter()
            .enumerate()
            .filter(|(_, heading)| heading.at <= len)
            .map(|(ix, heading)| OutlineHeading {
                depth: heading.depth,
                title: plain_title(&heading.title),
                line: editor.buffer().line_of(heading.at),
                at: heading.at,
                key: self.parsed.keys[ix].clone(),
                parent: self.parsed.parents[ix],
                opens: self
                    .parsed
                    .headings
                    .get(ix + 1)
                    .is_some_and(|next| next.depth > heading.depth),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading(depth: u8, title: &str, at: usize) -> DocumentHeading {
        DocumentHeading {
            depth,
            title: title.to_string(),
            runs: Default::default(),
            at,
        }
    }

    #[test]
    fn keys_count_headings_of_the_same_level() {
        let found = [
            heading(1, "Lift", 0),
            heading(2, "Lift", 4),
            heading(1, "Lift", 9),
            heading(3, "Light", 14),
        ];
        let (keys, parents) = keys(&found);
        assert_eq!(
            keys,
            ["1|0|Lift", "2|0|Lift", "1|1|Lift", "3|0|Light"]
        );
        assert_eq!(parents, [None, Some(0), None, Some(2)]);
    }

    #[test]
    fn titles_lose_their_marks_and_keep_their_words() {
        assert_eq!(plain_title("# Lift **it**"), "Lift it");
        assert_eq!(plain_title("通项 $a_n$ 是等比"), "通项 a_n 是等比");
        assert_eq!(plain_title("`code` and more"), "code and more");
        assert_eq!(plain_title("a *star*"), "a *star*");
    }
}
