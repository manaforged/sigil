use crate::hal::{PhysicalSize, Point};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale(f32);

impl Scale {
    pub const ONE: Self = Self(1.0);

    #[must_use]
    pub fn new(scale: f32) -> Self {
        if scale.is_finite() && scale > 0.0 {
            Self(scale)
        } else {
            Self::ONE
        }
    }

    #[must_use]
    pub const fn factor(self) -> f32 {
        self.0
    }

    #[must_use]
    pub fn to_physical(self, logical: LogicalSize) -> PhysicalSize {
        PhysicalSize::new(
            logical_axis_to_physical(logical.width, self.0),
            logical_axis_to_physical(logical.height, self.0),
        )
    }

    #[must_use]
    pub fn to_logical(self, physical: PhysicalSize) -> LogicalSize {
        LogicalSize::new(
            physical_axis_to_logical(physical.width, self.0),
            physical_axis_to_logical(physical.height, self.0),
        )
    }

    #[must_use]
    pub fn point_to_logical(self, physical: Point) -> Point {
        Point::new(physical.x / self.0, physical.y / self.0)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct LogicalSize {
    pub width: f32,
    pub height: f32,
}

impl LogicalSize {
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
fn logical_axis_to_physical(logical: f32, scale: f32) -> u32 {
    let physical = round_ties_even(logical * scale);
    if !physical.is_finite() || physical < 1.0 {
        1
    } else if physical >= u32::MAX as f32 {
        u32::MAX
    } else {
        physical as u32
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
)]
fn physical_axis_to_logical(physical: u32, scale: f32) -> f32 {
    physical as f32 / scale
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "values are bounded to the target range before the cast"
)]
fn round_ties_even(value: f32) -> f32 {
    if !value.is_finite() {
        return value;
    }

    let truncated_i = value as i64;
    let truncated = truncated_i as f32;
    let fraction = value - truncated;
    let abs_fraction = fraction.abs();
    let step = if value < 0.0 { -1.0 } else { 1.0 };
    if abs_fraction < 0.5 {
        truncated
    } else if abs_fraction > 0.5 {
        truncated + step
    } else if truncated_i % 2 == 0 {
        truncated
    } else {
        truncated + step
    }
}

#[cfg(test)]
#[path = "size_tests.rs"]
mod tests;
