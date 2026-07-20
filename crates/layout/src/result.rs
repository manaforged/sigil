//! Computed layout result attached to `sigil-kernel` nodes.

use sigil_hal::{Point, Rect};

use crate::ResolvedBox;

/// Computed layout attached back to a node after layout runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutResult {
    rect: Rect,
    content_size: Point,
    clips: bool,
}

impl LayoutResult {
    /// Return the border-box rectangle in root (layout-tree) logical
    /// pixels.
    ///
    /// Both the `BlockSolver` and the `FlexSolver` write `ComputedBox`
    /// values whose `(x, y)` are accumulated through every ancestor's
    /// border + padding offsets — i.e. they are **already in root
    /// coordinates**. Consumers walking the kernel tree (scene
    /// emission, hit testing) must therefore use the rect directly
    /// instead of re-accumulating a `parent_origin`; doing so would
    /// double-count the ancestor offsets and mislocate the node by
    /// every padding / margin / gap above it in the tree.
    #[must_use]
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    /// Return the scrollable content extent in logical pixels.
    #[must_use]
    pub const fn content_size(&self) -> Point {
        self.content_size
    }

    /// Whether this node clips overflowing descendants (`overflow: hidden`
    /// or `scroll` on either axis). The scene walker wraps the node's
    /// children in a clip scope so glyphs and fills outside the box are
    /// scissored instead of painting over siblings. Without this the
    /// overflow setting affects layout only, never the rendered pixels.
    #[must_use]
    pub const fn clips(&self) -> bool {
        self.clips
    }

    pub(crate) const fn from_resolved_box(layout: &ResolvedBox, clips: bool) -> Self {
        Self {
            rect: Rect::new(Point::new(layout.x, layout.y), layout.w, layout.h),
            content_size: Point::new(layout.w, layout.h),
            clips,
        }
    }
}
