use crate::geometry::{Point, Rect};

use crate::scene::corner_radii::CornerRadii;

pub type Transform = [[f32; 3]; 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteKind {
    Mono,
    Subpixel,
    Poly,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteSource {
    pub atlas: u32,
    pub region: Rect,
    pub phase: f32,
}

impl SpriteSource {
    #[cfg(feature = "scene")]
    pub(crate) fn fitted(self, atlas: (f32, f32), dest: Rect) -> Option<(Rect, Rect)> {
        let region = self.region;
        if region.width <= 0.0 || region.height <= 0.0 {
            return None;
        }
        let x0 = region.origin.x.max(0.0);
        let y0 = region.origin.y.max(0.0);
        let x1 = (region.origin.x + region.width).min(atlas.0);
        let y1 = (region.origin.y + region.height).min(atlas.1);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let sx = dest.width / region.width;
        let sy = dest.height / region.height;
        let visible = Rect::new(
            Point::new(
                (x0 - region.origin.x).mul_add(sx, dest.origin.x),
                (y0 - region.origin.y).mul_add(sy, dest.origin.y),
            ),
            (x1 - x0) * sx,
            (y1 - y0) * sy,
        );
        Some((Rect::new(Point::new(x0, y0), x1 - x0, y1 - y0), visible))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorationKind {
    Underline,
    Strikethrough,
    WavyUnderline,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerScope {
    pub clip: Rect,
    pub clip_radii: CornerRadii,
    pub effect: LayerEffect,
    pub opacity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayerEffect {
    Identity,
    Transform(Transform),
    LayerBlur { sigma: f32 },
}

pub(crate) fn transform_rect(transform: Transform, rect: Rect) -> Rect {
    let (left, top) = (rect.origin.x, rect.origin.y);
    let (right, bottom) = (left + rect.width, top + rect.height);
    let points = [(left, top), (right, top), (left, bottom), (right, bottom)]
        .map(|(x, y)| transform_point(transform, Point::new(x, y)));
    let fold = |axis: fn(&Point) -> f32, start: f32, pick: fn(f32, f32) -> f32| {
        points.iter().map(axis).fold(start, pick)
    };
    let (min_x, max_x) = (
        fold(|p| p.x, f32::INFINITY, f32::min),
        fold(|p| p.x, f32::NEG_INFINITY, f32::max),
    );
    let (min_y, max_y) = (
        fold(|p| p.y, f32::INFINITY, f32::min),
        fold(|p| p.y, f32::NEG_INFINITY, f32::max),
    );
    Rect::new(Point::new(min_x, min_y), max_x - min_x, max_y - min_y)
}

#[allow(
    clippy::missing_const_for_fn,
    reason = "const f32::mul_add requires Rust 1.94; Sigil supports 1.90"
)]
pub(crate) fn transform_point(transform: Transform, point: Point) -> Point {
    Point::new(
        transform[0][0].mul_add(point.x, transform[0][1].mul_add(point.y, transform[0][2])),
        transform[1][0].mul_add(point.x, transform[1][1].mul_add(point.y, transform[1][2])),
    )
}
