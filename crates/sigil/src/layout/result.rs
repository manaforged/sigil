use crate::hal::{Point, Rect};

use crate::layout::ResolvedBox;

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

    pub(crate) const fn translated(mut self, dx: f32, dy: f32) -> Self {
        self.rect.origin = Point::new(self.rect.origin.x + dx, self.rect.origin.y + dy);
        self
    }

    pub(crate) fn from_resolved_box(layout: &ResolvedBox, clips: bool) -> Self {
        let (x, y, w, h) = crate::layout::snap::rect(layout.x, layout.y, layout.w, layout.h);
        Self {
            rect: Rect::new(Point::new(x, y), w, h),
            content_size: Point::new(w, h),
            clips,
        }
    }
}
