use crate::geometry::{Point, Rect};

use crate::layout::engine_drive::ResolvedBox;
use crate::layout::shift::Shift;

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutResult {
    rect: Rect,
    content_size: Point,
    clips: bool,
    shift: Shift,
}

impl LayoutResult {
    #[must_use]
    pub fn rect(&self) -> Rect {
        let (dx, dy) = self.shift.offset();
        let origin = Point::new(self.rect.origin.x + dx, self.rect.origin.y + dy);
        Rect::new(origin, self.rect.width, self.rect.height)
    }

    pub(crate) const fn solved_origin(&self) -> Point {
        self.rect.origin
    }

    pub(crate) const fn shift(&self) -> Shift {
        self.shift
    }

    pub(crate) const fn shifted(mut self, shift: Shift) -> Self {
        self.shift = shift;
        self
    }

    #[cfg(feature = "kittest")]
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
            shift: Shift::default(),
        }
    }
}
