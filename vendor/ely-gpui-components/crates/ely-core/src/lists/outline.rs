use std::{collections::HashSet, rc::Rc};

use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, Pixels, RenderOnce, Rems,
    Role, SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::*, uniform_list,
};

use crate::{
    layout::seeded::use_seeded,
    primitives::Triangle,
    theme::{ActiveTheme, ControlSize, IconSize, Palette, Radius, TextSize},
    typography::Ellipsis,
};

type OnKey = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
type OnFold = Rc<dyn Fn(&SharedString, bool, &mut App)>;

/// One heading of an outline.
#[derive(Clone, Debug, PartialEq)]
struct Item {
    key: SharedString,
    title: SharedString,
    level: usize,
}

/// A document's headings as a tree: each level steps in, a heading with
/// headings under it shows ▾ open or ▸ folded, and a leaf keeps the triangle's
/// room so rows never jump. Large files draw only the rows in view.
///
/// ```
/// use ely_core::lists::Outline;
///
/// let headings = Outline::new("headings")
///     .item("intro", "Introduction", 1)
///     .item("setup", "Setup", 2)
///     .open(["intro"])
///     .current("intro")
///     .on_select(|_key, _, _| {});
/// ```
#[derive(IntoElement)]
pub struct Outline {
    id: ElementId,
    items: Vec<Item>,
    open: Option<Vec<SharedString>>,
    current: Option<SharedString>,
    on_select: Option<OnKey>,
    on_fold: Option<OnFold>,
}

impl Outline {
    /// An empty outline under `id`.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            open: None,
            current: None,
            on_select: None,
            on_fold: None,
        }
    }

    /// A heading under `key`, its title, and its level, counted from one.
    pub fn item(
        mut self,
        key: impl Into<SharedString>,
        title: impl Into<SharedString>,
        level: usize,
    ) -> Self {
        assert!(level >= 1, "heading levels start at one");
        self.items.push(Item {
            key: key.into(),
            title: title.into(),
            level,
        });
        self
    }

    /// The keys whose headings show their children; the rest are folded.
    /// Without it every heading shows its children.
    pub fn open(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.open = Some(keys.into_iter().map(Into::into).collect());
        self
    }

    /// The heading in view, by key.
    pub fn current(mut self, key: impl Into<SharedString>) -> Self {
        self.current = Some(key.into());
        self
    }

    /// Runs with the key of the heading pressed.
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// Hears each heading toggled: its key, and whether its children show.
    pub fn on_fold(mut self, handler: impl Fn(&SharedString, bool, &mut App) + 'static) -> Self {
        self.on_fold = Some(Rc::new(handler));
        self
    }
}

/// The keys with headings under them.
fn with_children(items: &[Item]) -> HashSet<SharedString> {
    (0..items.len())
        .filter(|ix| items.get(ix + 1).is_some_and(|next| next.level > items[*ix].level))
        .map(|ix| items[ix].key.clone())
        .collect()
}

/// The items in view, depth first, through the open headings.
fn visible(items: &[Item], open: &HashSet<SharedString>) -> Vec<usize> {
    let mut out = Vec::with_capacity(items.len());
    // The level whose folded heading hides the levels under it.
    let mut folded_at: Option<usize> = None;
    for (ix, item) in items.iter().enumerate() {
        if folded_at.is_some_and(|level| item.level > level) {
            continue;
        }
        folded_at = None;
        out.push(ix);
        let opens = items.get(ix + 1).is_some_and(|next| next.level > item.level);
        if opens && !open.contains(&item.key) {
            folded_at = Some(item.level);
        }
    }
    out
}

/// The triangle's share of its old box: two thirds across.
const MARK: f32 = 2.0 / 3.0;

/// What drawing rows needs.
struct Rows {
    id: ElementId,
    items: Rc<Vec<Item>>,
    shown: Rc<Vec<usize>>,
    opened: HashSet<SharedString>,
    current: Option<SharedString>,
    on_select: Option<OnKey>,
    toggle: Rc<dyn Fn(&SharedString, bool, &mut App)>,
    indent: Rems,
    /// The triangle's box, the slot it sits in, and the press it keeps.
    mark: Pixels,
    slot: Pixels,
    hit: Pixels,
    height: Pixels,
    radius: Rems,
    palette: Palette,
}

impl RenderOnce for Outline {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let seed = match &self.open {
            Some(open) => open.clone(),
            None => with_children(&self.items).into_iter().collect(),
        };
        let opened = use_seeded((id.clone(), "open"), seed, window, cx);
        let theme = cx.theme();
        let shown: HashSet<SharedString> = opened.read(cx).value.iter().cloned().collect();
        let items = Rc::new(self.items);
        let shown_rows = Rc::new(visible(&items, &shown));
        let fold = self.on_fold;
        let toggle: Rc<dyn Fn(&SharedString, bool, &mut App)> = Rc::new({
            let opened = opened.clone();
            move |key: &SharedString, open: bool, cx: &mut App| {
                log::info!("outline: {} {key}", if open { "opened" } else { "folded" });
                opened.update(cx, |seeded, cx| {
                    seeded.value.retain(|known| known != key);
                    if open {
                        seeded.value.push(key.clone());
                    }
                    cx.notify();
                });
                if let Some(fold) = &fold {
                    fold(key, open, cx);
                }
            }
        });
        let rows = Rows {
            id: id.clone(),
            items,
            shown: shown_rows.clone(),
            opened: shown,
            current: self.current,
            on_select: self.on_select,
            toggle: toggle.clone(),
            indent: theme.tree_indent(),
            mark: theme.icon_size(IconSize::Xs).to_pixels(window.rem_size()) * MARK,
            slot: theme.tree_indent().to_pixels(window.rem_size()) * MARK,
            hit: theme.icon_size(IconSize::Xs).to_pixels(window.rem_size()),
            height: theme.control_height(ControlSize::Sm).to_pixels(window.rem_size()),
            radius: theme.radius(Radius::Sm),
            palette: theme.colors.clone(),
        };
        let rows = Rc::new(rows);
        let count = shown_rows.len();
        let list = uniform_list((id.clone(), "rows"), count, {
            let rows = rows.clone();
            move |range, _window, _cx| {
                let rows = rows.as_ref();
                range.map(|at| row(rows, at)).collect()
            }
        })
        .size_full();
        div()
            .id(id)
            .relative()
            .size_full()
            .text_size(theme.text_size(TextSize::Sm))
            .child(list)
    }
}

/// One row: its triangle, its room per level, and its heading.
fn row(rows: &Rows, at: usize) -> AnyElement {
    let item = &rows.items[rows.shown[at]];
    let opens = rows
        .items
        .get(rows.shown[at] + 1)
        .is_some_and(|next| next.level > item.level);
    let open = opens && rows.opened.contains(&item.key);
    let here = rows.current.as_ref() == Some(&item.key);
    let colors = &rows.palette;
    let depth = item.level.saturating_sub(1);
    let slot = div()
        .id((rows.id.clone(), format!("chevron-{at}")))
        .debug_selector(|| format!("outline-chevron-{at}"))
        .flex_none()
        .relative()
        .w(rows.slot)
        .h(rows.height)
        .ml(rows.indent * depth as f32)
        .flex()
        .justify_center()
        .when(opens, |slot| {
            let key = item.key.clone();
            let (toggle, open) = (rows.toggle.clone(), open);
            // The mark is two thirds of its old box; its press keeps the old
            // triangle's width and the whole row's height.
            let press = div()
                .id((rows.id.clone(), format!("fold-{at}")))
                .absolute()
                .left((rows.slot - rows.hit) * 0.5)
                .top_0()
                .w(rows.hit)
                .h(rows.height)
                .cursor_pointer()
                .on_mouse_down(gpui::MouseButton::Left, move |_, _, cx| {
                    cx.stop_propagation();
                    toggle(&key, !open, cx);
                });
            slot.child(Triangle::new(open).size(rows.mark))
                .child(press)
        });
    let select = rows.on_select.clone();
    let key = item.key.clone();
    div()
        .id((rows.id.clone(), format!("row-{at}")))
        .debug_selector(|| format!("outline-row-{at}"))
        .role(Role::TreeItem)
        .aria_label(item.title.clone())
        .aria_level(item.level)
        .when(opens, |row| row.aria_expanded(open))
        .aria_selected(here)
        .relative()
        .w_full()
        .h(rows.height)
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .rounded(rows.radius)
        .text_color(if here { colors.fg } else { colors.fg_muted })
        .when(here, |row| {
            row.bg(colors.active).font_weight(FontWeight::MEDIUM)
        })
        .when(!here, |row| row.hover(|style| style.bg(colors.hover)))
        .child(slot)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(Ellipsis::new(item.title.clone())),
        )
        .on_click(move |_, window, cx| {
            if let Some(select) = &select {
                select(&key, window, cx);
            }
        })
        .into_any_element()
}
