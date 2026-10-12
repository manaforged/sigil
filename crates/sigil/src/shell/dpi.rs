#![allow(clippy::redundant_pub_crate)]

use crate::geometry::{Point, Scale};
use crate::shell::event_map::point_from_f64;

pub(crate) fn logical_point_from_f64(x: f64, y: f64, scale: Scale) -> Point {
    scale.point_to_logical(point_from_f64(x, y))
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "values are bounded to the target range before the cast"
)]
pub(crate) fn scale_from_f64(scale: f64) -> Scale {
    Scale::new(scale as f32)
}

#[cfg(test)]
#[path = "dpi_tests.rs"]
mod tests;
