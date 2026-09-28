#![allow(
    clippy::cast_precision_loss,
    reason = "target sizes are far below the f32 mantissa limit"
)]

use crate::hal::{Point, Rect};
use tiny_skia::{Path, PathBuilder, Pixmap, Point as TsPoint, Transform as TsTransform};

use crate::renderer::scene_command::walk::intersect_rect;
use crate::scene::corner_radii::CornerRadii;

#[path = "outline.rs"]
mod outline;

const KAPPA: f32 = 0.552_284_8;
const ANTIALIAS_REACH: f32 = 1.0;
const MAX_DEVICE_EXTENT: f32 = 16_000.0;

const ARC_DIRS: [[(f32, f32); 2]; 4] = [
    [(1.0, 0.0), (0.0, 1.0)],
    [(0.0, 1.0), (-1.0, 0.0)],
    [(-1.0, 0.0), (0.0, -1.0)],
    [(0.0, -1.0), (1.0, 0.0)],
];

#[derive(Clone, Copy)]
pub(super) struct Bound {
    transform: TsTransform,
    width: u32,
    height: u32,
    pad: f32,
}

impl Bound {
    pub(super) const fn new(transform: TsTransform, width: u32, height: u32) -> Self {
        Self {
            transform,
            width,
            height,
            pad: 0.0,
        }
    }

    pub(super) fn of(target: &Pixmap, transform: TsTransform) -> Self {
        Self::new(transform, target.width(), target.height())
    }

    pub(super) const fn padded(self, pad: f32) -> Self {
        Self { pad, ..self }
    }

    pub(super) fn clip(self, local: Rect) -> Option<Rect> {
        let fields = [
            local.origin.x,
            local.origin.y,
            local.width,
            local.height,
            self.pad,
        ];
        if !fields.iter().all(|v| v.is_finite()) || !self.transform.is_finite() {
            return None;
        }
        let r = ANTIALIAS_REACH;
        let (w, h) = (self.width as f32 + r, self.height as f32 + r);
        let view = hull(
            self.transform.invert()?,
            [(-r, -r), (w, -r), (w, h), (-r, h)],
        )?;
        let clipped = intersect_rect(local, expand(view, self.pad))?;
        let device = hull(self.transform, corners(clipped))?;
        (device.width <= MAX_DEVICE_EXTENT && device.height <= MAX_DEVICE_EXTENT).then_some(clipped)
    }

    pub(super) fn device_scale(self) -> f32 {
        let t = self.transform;
        t.sx.mul_add(t.sy, -(t.kx * t.ky)).abs().sqrt()
    }

    pub(super) fn clip_rect(self, local: Rect) -> Option<tiny_skia::Rect> {
        let r = self.clip(local)?;
        tiny_skia::Rect::from_xywh(r.origin.x, r.origin.y, r.width, r.height)
    }
}

fn corners(rect: Rect) -> [(f32, f32); 4] {
    let (x0, y0) = (rect.origin.x, rect.origin.y);
    let (x1, y1) = (x0 + rect.width, y0 + rect.height);
    [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn hull(transform: TsTransform, corners: [(f32, f32); 4]) -> Option<Rect> {
    let mut points = corners.map(|(x, y)| TsPoint::from_xy(x, y));
    transform.map_points(&mut points);
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in points {
        (x0, y0, x1, y1) = (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y));
    }
    let rect = Rect::new(Point::new(x0, y0), x1 - x0, y1 - y0);
    [x0, y0, x1, y1, rect.width, rect.height]
        .iter()
        .all(|v| v.is_finite())
        .then_some(rect)
}

pub(super) fn expand(rect: Rect, by: f32) -> Rect {
    Rect::new(
        Point::new(rect.origin.x - by, rect.origin.y - by),
        by.mul_add(2.0, rect.width).max(0.0),
        by.mul_add(2.0, rect.height).max(0.0),
    )
}

pub(super) fn rounded_rect(rect: Rect, radii: CornerRadii, bound: Bound) -> Option<Path> {
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return None;
    }
    let clipped = bound.clip(rect)?;
    let radii = radii.normalized_for(rect.width, rect.height);
    if corners(clipped) == corners(rect) {
        return path_of(clipped, radii);
    }
    if radii.iter().all(|r| *r <= 0.0) {
        return path_of(clipped, [0.0; 4]);
    }
    outline::clipped(rect, radii, clipped, bound.device_scale())
}

fn path_of(rect: Rect, radii: [f32; 4]) -> Option<Path> {
    let (left, top, width, height) = (rect.origin.x, rect.origin.y, rect.width, rect.height);
    let [tl, tr, br, bl] = CornerRadii(radii).normalized_for(width, height);
    if tl + tr + br + bl <= 0.0 {
        return PathBuilder::from_rect(tiny_skia::Rect::from_xywh(left, top, width, height)?)
            .into();
    }
    let (right, bottom) = (left + width, top + height);
    let mut b = PathBuilder::new();
    b.move_to(left + tl, top);
    b.line_to(right - tr, top);
    arc(&mut b, tr, (right, top + tr), 0);
    b.line_to(right, bottom - br);
    arc(&mut b, br, (right - br, bottom), 1);
    b.line_to(left + bl, bottom);
    arc(&mut b, bl, (left, bottom - bl), 2);
    b.line_to(left, top + tl);
    arc(&mut b, tl, (left + tl, top), 3);
    b.close();
    b.finish()
}

fn arc(b: &mut PathBuilder, r: f32, to: (f32, f32), corner: usize) {
    if r <= 0.0 {
        return;
    }
    let Some([d0, d1]) = ARC_DIRS.get(corner).copied() else {
        return;
    };
    let from = (
        (d0.0 + d1.0).mul_add(-r, to.0),
        (d0.1 + d1.1).mul_add(-r, to.1),
    );
    let k = KAPPA * r;
    b.cubic_to(
        d0.0.mul_add(k, from.0),
        d0.1.mul_add(k, from.1),
        d1.0.mul_add(-k, to.0),
        d1.1.mul_add(-k, to.1),
        to.0,
        to.1,
    );
}
