use crate::geometry::{PhysicalSize, Point};

/// Device scale factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale(f32);

impl Scale {
    /// The scale of one physical pixel per logical unit.
    pub const ONE: Self = Self(1.0);

    /// Creates a scale.
    #[must_use]
    pub fn new(scale: f32) -> Self {
        if scale.is_finite() && scale > 0.0 {
            Self(scale)
        } else {
            Self::ONE
        }
    }

    /// Returns the factor.
    #[must_use]
    pub const fn factor(self) -> f32 {
        self.0
    }

    /// Converts a logical size to physical pixels.
    #[must_use]
    pub fn to_physical(self, logical: Size) -> PhysicalSize {
        PhysicalSize::new(
            logical_axis_to_physical(logical.width, self.0),
            logical_axis_to_physical(logical.height, self.0),
        )
    }

    /// Converts a physical size to logical units.
    #[must_use]
    #[allow(
        clippy::cast_precision_loss,
        reason = "pixel magnitudes are bounded; sub-unit precision is irrelevant at render scale"
    )]
    pub fn to_logical(self, physical: PhysicalSize) -> Size {
        Size::new(
            physical.width as f32 / self.0,
            physical.height as f32 / self.0,
        )
    }

    /// Converts a physical point to logical units.
    #[must_use]
    pub fn point_to_logical(self, physical: Point) -> Point {
        Point::new(physical.x / self.0, physical.y / self.0)
    }
}

/// A size in logical px or cells.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Size {
    /// The width.
    pub width: f32,
    /// The height.
    pub height: f32,
}

impl Size {
    /// Creates a logical size.
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the cast saturates and the value is at least one"
)]
fn logical_axis_to_physical(logical: f32, scale: f32) -> u32 {
    let physical = (logical * scale).round_ties_even();
    if physical.is_finite() && physical >= 1.0 {
        physical as u32
    } else {
        1
    }
}

#[cfg(test)]
#[path = "size_tests.rs"]
mod tests;
