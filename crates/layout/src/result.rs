//! Computed layout result attached to `sigil-kernel` nodes.

use sigil_hal::{Point, Rect};

use crate::ResolvedBox;

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

    pub(crate) const fn from_resolved_box(layout: &ResolvedBox) -> Self {
        Self {
            rect: Rect::new(Point::new(layout.x, layout.y), layout.w, layout.h),
            content_size: Point::new(layout.w, layout.h),
        }
    }
}
