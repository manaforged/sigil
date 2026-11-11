use std::sync::Arc;

use resvg::usvg::tiny_skia_path::{Path, PathSegment};
use resvg::usvg::{self, LineCap, LineJoin, Transform};
use smallvec::SmallVec;

use crate::fnv::mix_f32;
use crate::scene::command::{PathCmd, StrokeCap, StrokeJoin, Transform as SceneTransform};

const DEFAULT_MITER: f32 = 4.0;
const RESOLUTION: f32 = 4.0;

/// The native stroke settings Sigil draws without outlining the stroke first.
pub(super) struct Native {
    pub(super) cap: StrokeCap,
    pub(super) join: StrokeJoin,
    pub(super) dash: SmallVec<[f32; 4]>,
}

pub(super) fn outline(path: &Path) -> Arc<[PathCmd]> {
    path.segments()
        .map(|segment| match segment {
            PathSegment::MoveTo(p) => PathCmd::MoveTo(p.x, p.y),
            PathSegment::LineTo(p) => PathCmd::LineTo(p.x, p.y),
            PathSegment::QuadTo(c, p) => PathCmd::QuadTo(c.x, c.y, p.x, p.y),
            PathSegment::CubicTo(a, b, p) => PathCmd::CubicTo(a.x, a.y, b.x, b.y, p.x, p.y),
            PathSegment::Close => PathCmd::Close,
        })
        .collect()
}

pub(super) const fn scene_transform(t: Transform) -> SceneTransform {
    [[t.sx, t.kx, t.tx], [t.ky, t.sy, t.ty]]
}

pub(super) fn transform_key(seed: u64, t: Transform) -> u64 {
    [t.sx, t.kx, t.tx, t.ky, t.sy, t.ty]
        .into_iter()
        .fold(seed, mix_f32)
}

pub(super) fn native(path: &Path, stroke: &usvg::Stroke) -> Option<Native> {
    let dotted = stroke.dasharray().is_some_and(|dash| dash.contains(&0.0));
    if stroke.linecap() != LineCap::Butt && (dotted || has_dot(path)) {
        return None;
    }
    let join = match stroke.linejoin() {
        LineJoin::Miter if (stroke.miterlimit().get() - DEFAULT_MITER).abs() < 1e-3 => {
            StrokeJoin::Miter
        }
        LineJoin::Round => StrokeJoin::Round,
        LineJoin::Bevel => StrokeJoin::Bevel,
        LineJoin::Miter | LineJoin::MiterClip => return None,
    };
    if stroke.dashoffset().abs() > f32::EPSILON {
        return None;
    }
    let cap = match stroke.linecap() {
        LineCap::Butt => StrokeCap::Butt,
        LineCap::Round => StrokeCap::Round,
        LineCap::Square => StrokeCap::Square,
    };
    let dash = stroke
        .dasharray()
        .unwrap_or_default()
        .iter()
        .copied()
        .collect();
    Some(Native { cap, join, dash })
}

fn has_dot(path: &Path) -> bool {
    let mut start = None;
    let mut moved = false;
    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(p) => {
                if start.is_some() && !moved {
                    return true;
                }
                (start, moved) = (Some(p), false);
            }
            PathSegment::LineTo(p) | PathSegment::QuadTo(_, p) | PathSegment::CubicTo(_, _, p) => {
                moved |= start.is_some_and(|s| s != p);
            }
            PathSegment::Close => {}
        }
    }
    start.is_some() && !moved
}

pub(super) fn stroked(path: &Path, stroke: &usvg::Stroke, t: Transform) -> Option<Path> {
    let line = stroke.to_tiny_skia();
    let (sx, sy) = t.get_scale();
    let resolution = sx.max(sy).max(1.0) * RESOLUTION;
    match &line.dash {
        Some(dash) => path.dash(dash, resolution)?.stroke(&line, resolution),
        None => path.stroke(&line, resolution),
    }
}
