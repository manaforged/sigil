use crate::geometry::{Point, Rect};

/// Payload and geometry delivered when a drag settles on a drop target.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct DropEvent<T> {
    value: T,
    position: Point,
    local_position: Point,
    bounds: Rect,
}

impl<T> DropEvent<T> {
    pub(crate) const fn new(
        value: T,
        position: Point,
        local_position: Point,
        bounds: Rect,
    ) -> Self {
        Self {
            value,
            position,
            local_position,
            bounds,
        }
    }

    /// Consumes the drop event and returns the dropped value.
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }

    /// Returns a reference to the dropped value.
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }

    /// Returns the drop position in window coordinates.
    #[must_use]
    pub const fn position(&self) -> Point {
        self.position
    }

    /// Returns the drop position relative to the target node's top-left corner.
    #[must_use]
    pub const fn local_position(&self) -> Point {
        self.local_position
    }

    /// Returns the target node's laid-out bounds at drop time.
    #[must_use]
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
}
