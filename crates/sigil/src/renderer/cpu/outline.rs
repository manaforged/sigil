#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "segment counts are clamped to a small range before the cast"
)]

use std::f32::consts::{FRAC_PI_2, PI};

use crate::geometry::Rect;
use tiny_skia::{Path, PathBuilder};

const TOLERANCE: f32 = 0.05;
const MIN_SEGMENTS: f32 = 2.0;
const MAX_SEGMENTS: f32 = 8192.0;

type Pt = (f32, f32);

pub(super) fn clipped(rect: Rect, radii: [f32; 4], view: Rect, scale: f32) -> Option<Path> {
    let mut points = outline(rect, radii, scale);
    let (x0, y0) = (view.origin.x, view.origin.y);
    let (x1, y1) = (x0 + view.width, y0 + view.height);
    points = clip_edge(&points, |p| p.0 - x0);
    points = clip_edge(&points, |p| x1 - p.0);
    points = clip_edge(&points, |p| p.1 - y0);
    points = clip_edge(&points, |p| y1 - p.1);
    let mut remaining = points.iter();
    let first = remaining.next()?;
    let mut builder = PathBuilder::new();
    builder.move_to(first.0, first.1);
    for point in remaining {
        builder.line_to(point.0, point.1);
    }
    builder.close();
    builder.finish()
}

fn outline(rect: Rect, radii: [f32; 4], scale: f32) -> Vec<Pt> {
    let (l, t) = (rect.origin.x, rect.origin.y);
    let (r, b) = (l + rect.width, t + rect.height);
    let [tl, tr, br, bl] = radii;
    let corners = [
        ((l + tl, t + tl), tl, PI),
        ((r - tr, t + tr), tr, PI + FRAC_PI_2),
        ((r - br, b - br), br, 0.0),
        ((l + bl, b - bl), bl, FRAC_PI_2),
    ];
    let mut points = Vec::new();
    for (center, radius, start) in corners {
        arc(&mut points, center, radius, start, scale);
    }
    points
}

fn arc(points: &mut Vec<Pt>, center: Pt, radius: f32, start: f32, scale: f32) {
    if radius <= 0.0 {
        points.push(center);
        return;
    }
    let device = (radius * scale).max(TOLERANCE);
    let step = 2.0 * (2.0 * TOLERANCE / device).sqrt();
    let segments = (FRAC_PI_2 / step).ceil().clamp(MIN_SEGMENTS, MAX_SEGMENTS) as u32;
    for i in 0..=segments {
        let angle = (FRAC_PI_2 * i as f32).mul_add(1.0 / segments as f32, start);
        points.push((
            radius.mul_add(angle.cos(), center.0),
            radius.mul_add(angle.sin(), center.1),
        ));
    }
}

fn clip_edge(points: &[Pt], distance: impl Fn(Pt) -> f32) -> Vec<Pt> {
    let mut out = Vec::with_capacity(points.len() + 4);
    let Some(&last) = points.last() else {
        return out;
    };
    let mut prev = last;
    for &cur in points {
        let (dp, dc) = (distance(prev), distance(cur));
        if (dp >= 0.0) != (dc >= 0.0) {
            let t = dp / (dp - dc);
            out.push((
                t.mul_add(cur.0 - prev.0, prev.0),
                t.mul_add(cur.1 - prev.1, prev.1),
            ));
        }
        if dc >= 0.0 {
            out.push(cur);
        }
        prev = cur;
    }
    out
}
