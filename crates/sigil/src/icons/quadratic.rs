#![allow(clippy::redundant_pub_crate)]
#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]

use lyon_geom::{CubicBezierSegment, Point, QuadraticBezierSegment};
use usvg::tiny_skia_path::PathSegment;

use crate::icons::parser::SvgPath;
use crate::icons::{BakeError, MAX_PATH_SEGMENTS};

#[derive(Clone, Copy, Debug)]
pub struct QuadCurve {
    pub start: (f32, f32),
    pub control: (f32, f32),
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

pub(crate) fn to_quads(paths: &[SvgPath]) -> Result<Vec<QuadCurve>, BakeError> {
    const TOLERANCE: f32 = 0.5;
    let mut quads: Vec<QuadCurve> = Vec::new();
    let mut cursor = (0.0_f32, 0.0_f32);
    let mut start = cursor;

    for path in paths {
        for seg in &path.segments {
            match *seg {
                PathSegment::MoveTo(p) => {
                    close(&mut quads, cursor, start);
                    cursor = (p.x, p.y);
                    start = cursor;
                }
                PathSegment::LineTo(p) => {
                    quads.push(line(cursor, (p.x, p.y)));
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
                PathSegment::Close => {
                    close(&mut quads, cursor, start);
                    cursor = start;
                }
            }
        }
        close(&mut quads, cursor, start);
        cursor = start;
    }

    let count = u32::try_from(quads.len()).unwrap_or(u32::MAX);
    if count > MAX_PATH_SEGMENTS {
        return Err(BakeError::TooManySegments(count));
    }
    Ok(quads)
}

const fn line(from: (f32, f32), to: (f32, f32)) -> QuadCurve {
    QuadCurve {
        start: from,
        control: (f32::midpoint(from.0, to.0), f32::midpoint(from.1, to.1)),
        end: to,
    }
}

fn close(quads: &mut Vec<QuadCurve>, cursor: (f32, f32), start: (f32, f32)) {
    if cursor != start {
        quads.push(line(cursor, start));
    }
}

#[cfg(any(test, feature = "kittest"))]
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
