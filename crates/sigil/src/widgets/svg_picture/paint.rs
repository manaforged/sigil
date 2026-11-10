use std::sync::Arc;

use resvg::usvg::{self, Transform};

use crate::scene::command::PathPaint;
use crate::style::Color;

use super::fallback::Fallback;
use super::gradient;

/// A usvg paint resolved for the scene.
pub(super) enum Fill {
    /// A paint in the path's own units.
    Plain(PathPaint),
    /// A paint in a gradient space that the path's points must be mapped into.
    Space(PathPaint, Transform),
    /// A pattern tile drawn over the path at this opacity.
    Pattern(Arc<usvg::Pattern>, f32),
}

pub(super) fn fill(paint: &usvg::Paint, opacity: f32) -> Result<Fill, Fallback> {
    match paint {
        usvg::Paint::Color(color) => Ok(Fill::Plain(PathPaint::Solid(solid(*color, opacity)))),
        usvg::Paint::LinearGradient(linear) => Ok(gradient::linear(linear, opacity)),
        usvg::Paint::RadialGradient(radial) => gradient::radial(radial, opacity),
        usvg::Paint::Pattern(pattern) => Ok(Fill::Pattern(Arc::clone(pattern), opacity)),
    }
}

pub(super) fn solid(color: usvg::Color, opacity: f32) -> Color {
    Color {
        r: color.red,
        g: color.green,
        b: color.blue,
        a: alpha(opacity),
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "opacity is clamped to 0..=1 before the cast"
)]
pub(super) fn alpha(opacity: f32) -> u8 {
    (opacity.clamp(0.0, 1.0) * 255.0).round() as u8
}

pub(super) const OPAQUE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
