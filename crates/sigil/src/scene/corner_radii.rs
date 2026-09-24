#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadii(pub [f32; 4]);

impl CornerRadii {
    pub const ZERO: Self = Self([0.0, 0.0, 0.0, 0.0]);

    #[must_use]
    pub const fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self([top_left, top_right, bottom_right, bottom_left])
    }

    #[must_use]
    pub const fn uniform(radius: f32) -> Self {
        Self([radius, radius, radius, radius])
    }

    #[must_use]
    pub const fn top(radius: f32) -> Self {
        Self([radius, radius, 0.0, 0.0])
    }

    #[must_use]
    pub const fn bottom(radius: f32) -> Self {
        Self([0.0, 0.0, radius, radius])
    }

    #[must_use]
    pub const fn left(radius: f32) -> Self {
        Self([radius, 0.0, 0.0, radius])
    }

    #[must_use]
    pub const fn right(radius: f32) -> Self {
        Self([0.0, radius, radius, 0.0])
    }

    #[must_use]
    pub(crate) fn nonnegative_array(self) -> [f32; 4] {
        self.0.map(|radius| radius.max(0.0))
    }

    #[must_use]
    pub fn normalized_for(self, width: f32, height: f32) -> [f32; 4] {
        let [top_left, top_right, bottom_right, bottom_left] = self.nonnegative_array();
        let factor = edge_factor(width, top_left + top_right)
            .min(edge_factor(width, bottom_left + bottom_right))
            .min(edge_factor(height, top_left + bottom_left))
            .min(edge_factor(height, top_right + bottom_right));
        [top_left, top_right, bottom_right, bottom_left].map(|radius| radius * factor)
    }

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
#[path = "corner_radii_tests.rs"]
mod tests;
