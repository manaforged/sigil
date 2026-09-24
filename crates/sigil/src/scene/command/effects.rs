use crate::hal::{Point, Rect};

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
    let p0 = transform_point(transform, rect.origin);
    let p1 = transform_point(
        transform,
        Point::new(rect.origin.x + rect.width, rect.origin.y),
    );
    let p2 = transform_point(
        transform,
        Point::new(rect.origin.x, rect.origin.y + rect.height),
    );
    let p3 = transform_point(
        transform,
        Point::new(rect.origin.x + rect.width, rect.origin.y + rect.height),
    );
    bounding_rect([p0, p1, p2, p3])
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

fn bounding_rect(points: [Point; 4]) -> Rect {
    let min_x = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let max_x = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let min_y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_y = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    Rect::new(Point::new(min_x, min_y), max_x - min_x, max_y - min_y)
}
