use crate::geometry::{Point, Rect};

use crate::scene::corner_radii::CornerRadii;

/// A 2D affine transform as two rows of three values.
pub type Transform = [[f32; 3]; 2];

/// How a renderer colors a sprite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteKind {
    /// The mono sprite kind.
    Mono,
    /// The subpixel sprite kind.
    Subpixel,
    /// The poly sprite kind.
    Poly,
}

/// Where a sprite's pixels come from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteSource {
    /// The atlas.
    pub atlas: u32,
    /// The region.
    pub region: Rect,
    /// The phase.
    pub phase: f32,
}

impl SpriteSource {
    #[cfg(all(
        feature = "scene",
        any(test, feature = "kittest", feature = "desktop", feature = "terminal")
    ))]
    pub(crate) fn fitted(self, atlas: (f32, f32), dest: Rect) -> Option<(Rect, Rect)> {
        let region = self.region;
        if region.width <= 0.0 || region.height <= 0.0 {
            return None;
        }
        let clipped = region.intersection(Rect::new(Point::new(0.0, 0.0), atlas.0, atlas.1))?;
        let sx = dest.width / region.width;
        let sy = dest.height / region.height;
        let visible = Rect::new(
            Point::new(
                (clipped.origin.x - region.origin.x).mul_add(sx, dest.origin.x),
                (clipped.origin.y - region.origin.y).mul_add(sy, dest.origin.y),
            ),
            clipped.width * sx,
            clipped.height * sy,
        );
        Some((clipped, visible))
    }
}

/// The line style of a text decoration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorationKind {
    /// The underline decoration kind.
    Underline,
    /// The strikethrough decoration kind.
    Strikethrough,
    /// The wavy underline decoration kind.
    WavyUnderline,
}

/// The bounds and effects of a composite layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerScope {
    /// The clip.
    pub clip: Rect,
    /// The clip radii.
    pub clip_radii: CornerRadii,
    /// The effect.
    pub effect: LayerEffect,
    /// The opacity.
    pub opacity: f32,
}

/// An effect applied to a composite layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayerEffect {
    /// The identity layer effect.
    Identity,
    /// The transform layer effect.
    Transform(Transform),
    /// The layer blur layer effect.
    LayerBlur {
        /// The blur radius.
        sigma: f32,
    },
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
