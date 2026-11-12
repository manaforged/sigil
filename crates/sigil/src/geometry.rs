pub use crate::hal::geometry::{PhysicalSize, Point, Rect};
pub use crate::hal::layout::Constraint;
pub(crate) use crate::hal::layout::Dim;
pub use crate::hal::size::Scale;
pub use crate::hal::size::Size;

#[cfg(feature = "scene")]
/// Returns the Hermite smoothstep of `value` between the `low` and `high` edges, from 0.0 to 1.0.
pub(crate) fn smoothstep(low: f32, high: f32, value: f32) -> f32 {
    let t = ((value - low) / (high - low)).clamp(0.0, 1.0);
    t * t * 2.0f32.mul_add(-t, 3.0)
}
