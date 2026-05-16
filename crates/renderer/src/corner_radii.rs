//! Per-corner rectangle radii.

/// Rectangle corner radii in logical pixels: top-left, top-right,
/// bottom-right, bottom-left.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadii(pub [f32; 4]);

impl CornerRadii {
    /// No rounded corners.
    pub const ZERO: Self = Self([0.0, 0.0, 0.0, 0.0]);

    /// Create radii in top-left, top-right, bottom-right, bottom-left order.
    #[must_use]
    pub const fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self([top_left, top_right, bottom_right, bottom_left])
    }

    /// Create a uniform radius for all four corners.
    #[must_use]
    pub const fn uniform(radius: f32) -> Self {
        Self([radius, radius, radius, radius])
    }

    /// Round only the top edge corners.
    #[must_use]
    pub const fn top(radius: f32) -> Self {
        Self([radius, radius, 0.0, 0.0])
    }

    /// Round only the bottom edge corners.
    #[must_use]
    pub const fn bottom(radius: f32) -> Self {
        Self([0.0, 0.0, radius, radius])
    }

    /// Round only the left edge corners.
    #[must_use]
    pub const fn left(radius: f32) -> Self {
        Self([radius, 0.0, 0.0, radius])
    }

    /// Round only the right edge corners.
    #[must_use]
    pub const fn right(radius: f32) -> Self {
        Self([0.0, radius, radius, 0.0])
    }

    #[must_use]
    pub(crate) fn nonnegative_array(self) -> [f32; 4] {
        self.0.map(|radius| radius.max(0.0))
    }

    /// Pick the corner radius for a local point relative to rectangle centre.
    #[must_use]
    pub fn corner_for(self, local: [f32; 2]) -> f32 {
        let [x, y] = local;
        let [top_left, top_right, bottom_right, bottom_left] = self.nonnegative_array();
        match (x > 0.0, y < 0.0) {
            (false, true) => top_left,
            (true, true) => top_right,
            (true, false) => bottom_right,
            (false, false) => bottom_left,
        }
    }
}
