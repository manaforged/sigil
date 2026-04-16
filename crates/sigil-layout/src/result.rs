//! Computed layout result attached to `sigil-core` nodes.

use sigil_hal::{Point, Rect};

/// Computed layout attached back to a node after layout runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutResult {
    rect: Rect,
    content_size: Point,
}

impl LayoutResult {
    /// Return the parent-relative border-box rectangle in logical pixels.
    #[must_use]
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    /// Return the scrollable content extent in logical pixels.
    #[must_use]
    pub const fn content_size(&self) -> Point {
        self.content_size
    }

    pub(crate) const fn from_taffy(layout: &taffy::tree::Layout) -> Self {
        Self {
            rect: Rect::new(
                Point::new(layout.location.x, layout.location.y),
                layout.size.width,
                layout.size.height,
            ),
            content_size: Point::new(layout.content_size.width, layout.content_size.height),
        }
    }
}
