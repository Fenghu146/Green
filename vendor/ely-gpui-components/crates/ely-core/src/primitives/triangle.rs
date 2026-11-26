use gpui::{App, Hsla, IntoElement, Pixels, RenderOnce, Window, prelude::*, svg};

use crate::theme::{ActiveTheme, IconSize};

/// A filled triangle pointing right while shut and down while open, drawn from
/// bytes so it needs no font. Unlike `Disclosure`'s chevron it is a solid
/// triangle, the outline's own mark.
///
/// ```
/// use ely_core::primitives::Triangle;
/// use gpui::px;
///
/// let mark = Triangle::new(true).size(px(8.0));
/// ```
#[derive(IntoElement)]
pub struct Triangle {
    open: bool,
    size: Option<Pixels>,
    color: Option<Hsla>,
}

impl Triangle {
    /// A triangle pointing down when `open`, right otherwise.
    pub fn new(open: bool) -> Self {
        Self {
            open,
            size: None,
            color: None,
        }
    }

    /// Its size; the theme's smallest icon unless set.
    pub fn size(mut self, size: Pixels) -> Self {
        self.size = Some(size);
        self
    }

    /// Its color; the subtle foreground unless set.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }
}

/// Pointing right: folded.
const RIGHT: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><path d="M1.6 0.6 L6.6 4 L1.6 7.4 Z" fill="currentColor"/></svg>"#;

/// Pointing down: open.
const DOWN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><path d="M0.6 1.6 L7.4 1.6 L4 6.6 Z" fill="currentColor"/></svg>"#;

impl RenderOnce for Triangle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let color = self.color.unwrap_or(theme.colors.fg_subtle);
        let size = self
            .size
            .unwrap_or_else(|| theme.icon_size(IconSize::Xs).to_pixels(window.rem_size()));
        svg()
            .data(if self.open { DOWN } else { RIGHT })
            .size(size)
            .flex_none()
            .text_color(color)
    }
}
