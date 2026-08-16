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

    /// Resolve oversized radii with the CSS corner-overlap rule.
    ///
    /// Every radius is scaled by the same factor so adjacent corners never
    /// consume more than the corresponding edge. This turns a deliberately
    /// large `rounded-full` value into a capsule instead of an empty SDF.
    #[must_use]
    pub fn normalized_for(self, width: f32, height: f32) -> [f32; 4] {
        let [top_left, top_right, bottom_right, bottom_left] = self.nonnegative_array();
        let factor = edge_factor(width, top_left + top_right)
            .min(edge_factor(width, bottom_left + bottom_right))
            .min(edge_factor(height, top_left + bottom_left))
            .min(edge_factor(height, top_right + bottom_right));
        [top_left, top_right, bottom_right, bottom_left].map(|radius| radius * factor)
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

fn edge_factor(edge: f32, radii: f32) -> f32 {
    if radii > 0.0 {
        (edge.max(0.0) / radii).min(1.0)
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::CornerRadii;

    #[test]
    fn oversized_uniform_radius_becomes_capsule() {
        assert_radii_close(
            CornerRadii::uniform(9_999.0).normalized_for(80.0, 40.0),
            [20.0; 4],
        );
    }

    #[test]
    fn asymmetric_radii_share_one_css_scale_factor() {
        assert_radii_close(
            CornerRadii::new(40.0, 20.0, 10.0, 30.0).normalized_for(30.0, 80.0),
            [20.0, 10.0, 5.0, 15.0],
        );
    }

    fn assert_radii_close(actual: [f32; 4], expected: [f32; 4]) {
        for (observed, wanted) in actual.into_iter().zip(expected) {
            assert!((observed - wanted).abs() < f32::EPSILON);
        }
    }
}
