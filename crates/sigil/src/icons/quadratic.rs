#![allow(clippy::redundant_pub_crate)]
#![cfg_attr(not(any(test, feature = "kittest")), allow(unreachable_pub))]

use lyon_geom::{CubicBezierSegment, Point, QuadraticBezierSegment};
use usvg::tiny_skia_path::PathSegment;

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

pub(crate) fn to_quads(paths: &[Vec<PathSegment>]) -> Result<Vec<QuadCurve>, BakeError> {
    const TOLERANCE: f32 = 0.5;
    let mut quads: Vec<QuadCurve> = Vec::new();
    let mut cursor = (0.0_f32, 0.0_f32);
    let mut start = cursor;

    for path in paths {
        for seg in path {
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
                    push_cubic(
                        &mut quads,
                        [cursor, (c1.x, c1.y), (c2.x, c2.y), (end.x, end.y)],
                        TOLERANCE,
                    );
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

fn push_cubic(quads: &mut Vec<QuadCurve>, points: [(f32, f32); 4], tolerance: f32) {
    let [from, ctrl1, ctrl2, to] = points.map(|(x, y)| Point::new(x, y));
    CubicBezierSegment {
        from,
        ctrl1,
        ctrl2,
        to,
    }
    .for_each_quadratic_bezier(tolerance, &mut |q| quads.push(QuadCurve::from_lyon(q)));
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
    let mut quads = Vec::new();
    push_cubic(&mut quads, [p0, p1, p2, p3], tolerance);
    quads
}
