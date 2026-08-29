use sigil_hal::{Point, Rect};

use crate::ResolvedBox;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutResult {
    rect: Rect,
    content_size: Point,
    clips: bool,
}

impl LayoutResult {
    #[must_use]
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    #[must_use]
    pub const fn content_size(&self) -> Point {
        self.content_size
    }

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
