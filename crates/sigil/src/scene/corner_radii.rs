kittest_pub! {
    /// Per-corner radii in the order top-left, top-right, bottom-right, bottom-left.
    #[doc(hidden)]
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct CornerRadii(
        /// The four radii, clockwise from the top left corner.
        pub [f32; 4],
    );
}

impl CornerRadii {
    kittest_pub! {
        /// Square corners.
        pub const ZERO: Self = Self([0.0, 0.0, 0.0, 0.0]);
    }

    kittest_pub! {
        if any(test, feature = "scene");
        /// Creates a corner radii.
        #[must_use]
        pub const fn new(
            top_left: f32,
            top_right: f32,
            bottom_right: f32,
            bottom_left: f32,
        ) -> Self {
            Self([top_left, top_right, bottom_right, bottom_left])
        }
    }

    kittest_pub! {
        /// Uses `radius` on all four corners.
        #[must_use]
        pub const fn uniform(radius: f32) -> Self {
            Self([radius, radius, radius, radius])
        }
    }

    kittest_pub! {
        if test;
        /// Rounds the two top corners.
        #[must_use]
        pub const fn top(radius: f32) -> Self {
            Self([radius, radius, 0.0, 0.0])
        }
    }

    kittest_pub! {
        if test;
        /// Rounds the two bottom corners.
        #[must_use]
        pub const fn bottom(radius: f32) -> Self {
            Self([0.0, 0.0, radius, radius])
        }
    }

    kittest_pub! {
        if test;
        /// Rounds the two left corners.
        #[must_use]
        pub const fn left(radius: f32) -> Self {
            Self([radius, 0.0, 0.0, radius])
        }
    }

    kittest_pub! {
        if test;
        /// Rounds the two right corners.
        #[must_use]
        pub const fn right(radius: f32) -> Self {
            Self([0.0, radius, radius, 0.0])
        }
    }

    #[cfg(any(test, feature = "kittest", feature = "scene"))]
    #[must_use]
    pub(crate) fn nonnegative_array(self) -> [f32; 4] {
        self.0.map(|radius| {
            if radius.is_nan() {
                0.0
            } else {
                radius.max(0.0)
            }
        })
    }

    kittest_pub! {
        if any(test, feature = "scene");
        /// Scales the radii to fit a box of this size.
        #[must_use]
        pub fn normalized_for(self, width: f32, height: f32) -> [f32; 4] {
            let longest = width.max(height).max(0.0);
            let [top_left, top_right, bottom_right, bottom_left] =
                self.nonnegative_array().map(|radius| radius.min(longest));
            let factor = edge_factor(width, top_left + top_right)
                .min(edge_factor(width, bottom_left + bottom_right))
                .min(edge_factor(height, top_left + bottom_left))
                .min(edge_factor(height, top_right + bottom_right));
            [top_left, top_right, bottom_right, bottom_left].map(|radius| radius * factor)
        }
    }
}

#[cfg(any(test, feature = "kittest", feature = "scene"))]
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
