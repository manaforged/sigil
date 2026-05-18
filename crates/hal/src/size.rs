//! DPI scale and logical-size primitives.

use crate::{PhysicalSize, Point};

/// Per-window scale factor. `physical_px = logical_px * scale`.
///
/// Always strictly positive. Typical values are 1.0, 1.25, 1.5, 2.0, 3.0.
/// Constructed via [`Scale::new`] or [`Scale::ONE`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scale(f32);

impl Scale {
    /// Identity scale (1.0, one logical pixel = one physical pixel).
    pub const ONE: Self = Self(1.0);

    /// Construct from a positive `f32`. Non-positive or non-finite
    /// values clamp to `ONE` (defensive, winit can in theory report
    /// a `0.0` scale on a transient surface state we don't want to
    /// crash on).
    #[must_use]
    pub fn new(scale: f32) -> Self {
        if scale.is_finite() && scale > 0.0 {
            Self(scale)
        } else {
            Self::ONE
        }
    }

    /// Borrow the raw factor.
    #[must_use]
    pub const fn factor(self) -> f32 {
        self.0
    }

    /// Convert a logical-pixel size into the matching physical-pixel size.
    /// Rounds half-to-even, then clamps each axis to `>= 1` so wgpu's
    /// swap-chain configuration never receives zero.
    #[must_use]
    pub fn to_physical(self, logical: LogicalSize) -> PhysicalSize {
        PhysicalSize::new(
            logical_axis_to_physical(logical.width, self.0),
            logical_axis_to_physical(logical.height, self.0),
        )
    }

    /// Convert a physical-pixel size into the matching logical-pixel size.
    #[must_use]
    pub fn to_logical(self, physical: PhysicalSize) -> LogicalSize {
        LogicalSize::new(
            physical_axis_to_logical(physical.width, self.0),
            physical_axis_to_logical(physical.height, self.0),
        )
    }

    /// Convert a physical position into a logical [`Point`], used by the
    /// shell adapter when translating winit pointer events.
    #[must_use]
    pub fn point_to_logical(self, physical: Point) -> Point {
        Point::new(physical.x / self.0, physical.y / self.0)
    }
}

/// A size in **logical pixels**. The unit users specify everywhere
/// except at the HAL boundary.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct LogicalSize {
    /// Width in logical pixels.
    pub width: f32,
    /// Height in logical pixels.
    pub height: f32,
}

impl LogicalSize {
    /// Construct a logical size.
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
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

#[allow(clippy::cast_precision_loss)]
fn physical_axis_to_logical(physical: u32, scale: f32) -> f32 {
    physical as f32 / scale
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]
fn round_ties_even(value: f32) -> f32 {
    if !value.is_finite() {
        return value;
    }

    let truncated_i = value as i64;
    let truncated = truncated_i as f32;
    let fraction = value - truncated;
    let abs_fraction = if fraction < 0.0 { -fraction } else { fraction };
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
mod tests {
    use super::{LogicalSize, Scale};
    use crate::{PhysicalSize, Point};

    #[test]
    fn scale_new_clamps_zero_to_one() {
        assert_eq!(Scale::new(0.0), Scale::ONE);
    }

    #[test]
    fn scale_new_clamps_negative_to_one() {
        assert_eq!(Scale::new(-1.0), Scale::ONE);
    }

    #[test]
    fn scale_new_clamps_nan_to_one() {
        assert_eq!(Scale::new(f32::NAN), Scale::ONE);
    }

    #[test]
    fn to_physical_roundtrips_at_scale_2x() {
        assert_eq!(
            Scale::new(2.0).to_physical(LogicalSize::new(720.0, 480.0)),
            PhysicalSize::new(1440, 960)
        );
    }

    #[test]
    fn to_physical_clamps_to_one_minimum() {
        assert_eq!(
            Scale::ONE.to_physical(LogicalSize::new(0.4, 0.4)),
            PhysicalSize::new(1, 1)
        );
    }

    #[test]
    fn to_logical_roundtrips_at_scale_125() {
        assert_eq!(
            Scale::new(1.25).to_logical(PhysicalSize::new(900, 600)),
            LogicalSize::new(720.0, 480.0)
        );
    }

    #[test]
    fn point_to_logical_divides_by_factor() {
        assert_eq!(
            Scale::new(2.0).point_to_logical(Point::new(150.0, 75.0)),
            Point::new(75.0, 37.5)
        );
    }
}
