//! Cubic→quadratic Bézier approximation via `lyon_geom`.
#![allow(clippy::redundant_pub_crate)]

use lyon_geom::{CubicBezierSegment, Point, QuadraticBezierSegment};
use usvg::tiny_skia_path::PathSegment;

use crate::parser::SvgPath;
use crate::{BakeError, MAX_PATH_SEGMENTS};

/// A quadratic Bézier segment in viewBox-space coordinates.
#[derive(Clone, Copy, Debug)]
pub struct QuadCurve {
    /// Start point (anchor).
    pub start: (f32, f32),
    /// Control point.
    pub control: (f32, f32),
    /// End point (anchor).
    pub end: (f32, f32),
}

impl QuadCurve {
    const fn from_lyon(q: &QuadraticBezierSegment<f32>) -> Self {
        Self {
            start: (q.from.x, q.from.y),
            control: (q.ctrl.x, q.ctrl.y),
            end: (q.to.x, q.to.y),
        }
    }
}

/// Convert a slice of [`SvgPath`]s into a flat list of [`QuadCurve`]s.
///
/// Cubics are split into quadratic approximations at tolerance 0.5 path-units.
/// Lines are represented as degenerate quads (control = midpoint).
/// Move and Close segments are consumed as contour boundaries with no curve output.
///
/// Returns [`BakeError::TooManySegments`] if the total exceeds
/// [`MAX_PATH_SEGMENTS`].
pub(crate) fn to_quads(paths: &[SvgPath]) -> Result<Vec<QuadCurve>, BakeError> {
    const TOLERANCE: f32 = 0.5;
    let mut quads: Vec<QuadCurve> = Vec::new();
    let mut cursor = (0.0_f32, 0.0_f32);

    for path in paths {
        for seg in &path.segments {
            match *seg {
                PathSegment::MoveTo(p) => {
                    cursor = (p.x, p.y);
                }
                PathSegment::LineTo(p) => {
                    let mid = (f32::midpoint(cursor.0, p.x), f32::midpoint(cursor.1, p.y));
                    quads.push(QuadCurve {
                        start: cursor,
                        control: mid,
                        end: (p.x, p.y),
                    });
                    cursor = (p.x, p.y);
                }
                PathSegment::QuadTo(ctrl, end) => {
                    quads.push(QuadCurve {
                        start: cursor,
                        control: (ctrl.x, ctrl.y),
                        end: (end.x, end.y),
                    });
                    cursor = (end.x, end.y);
                }
                PathSegment::CubicTo(c1, c2, end) => {
                    let cubic = CubicBezierSegment {
                        from: Point::new(cursor.0, cursor.1),
                        ctrl1: Point::new(c1.x, c1.y),
                        ctrl2: Point::new(c2.x, c2.y),
                        to: Point::new(end.x, end.y),
                    };
                    cubic.for_each_quadratic_bezier(TOLERANCE, &mut |q| {
                        quads.push(QuadCurve::from_lyon(q));
                    });
                    cursor = (end.x, end.y);
                }
                PathSegment::Close => {}
            }
        }
    }

    let count = u32::try_from(quads.len()).unwrap_or(u32::MAX);
    if count > MAX_PATH_SEGMENTS {
        return Err(BakeError::TooManySegments(count));
    }
    Ok(quads)
}

/// Converts a cubic Bézier into quadratic approximations at the given tolerance.
///
/// Lyon's max-deviation tolerance is not the same as the sampled Hausdorff
/// distance used by property tests, especially on high-curvature cubics.
#[must_use]
pub fn cubic_to_quadratics(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    tolerance: f32,
) -> Vec<QuadCurve> {
    let cubic = CubicBezierSegment {
        from: Point::new(p0.0, p0.1),
        ctrl1: Point::new(p1.0, p1.1),
        ctrl2: Point::new(p2.0, p2.1),
        to: Point::new(p3.0, p3.1),
    };
    let mut quads = Vec::new();
    cubic.for_each_quadratic_bezier(tolerance, &mut |q| {
        quads.push(QuadCurve::from_lyon(q));
    });
    quads
}
