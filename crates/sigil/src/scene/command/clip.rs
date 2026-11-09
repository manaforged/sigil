use std::sync::Arc;

use crate::geometry::{Point, Rect};
use crate::scene::command::effects::{LayerEffect, LayerScope};
use crate::scene::command::fills::{FillRule, PathPaint};
use crate::scene::command::payloads::{DrawPath, PathCmd};
use crate::scene::corner_radii::CornerRadii;
use crate::style::Rgba8;

const IDENTITY: crate::scene::command::Transform = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
const KAPPA: f32 = 0.552_284_8;
const OPAQUE: Rgba8 = Rgba8 {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

/// The region a clip keeps.
#[allow(
    clippy::large_enum_variant,
    reason = "SceneCommand already holds DrawPath inline, so boxing it here saves no space and adds an allocation"
)]
#[derive(Clone, Debug, PartialEq)]
pub enum Clip {
    /// A rectangle, applied with the scissor.
    Rect(Rect),
    /// A rectangle with rounded corners.
    Rounded(Rect, CornerRadii),
    /// The pixels a path covers, weighted by its alpha.
    Path(DrawPath),
}

impl Clip {
    /// A rectangle with corners of `radius`.
    #[must_use]
    pub const fn rounded(rect: Rect, radius: f32) -> Self {
        Self::Rounded(rect, CornerRadii::uniform(radius))
    }

    /// A disc of `radius` around `center`.
    #[must_use]
    pub fn circle(center: Point, radius: f32) -> Self {
        let radius = radius.max(0.0);
        let corner = Point::new(center.x - radius, center.y - radius);
        Self::rounded(Rect::new(corner, radius * 2.0, radius * 2.0), radius)
    }

    /// The ellipse inscribed in `rect`.
    #[must_use]
    pub fn ellipse(rect: Rect) -> Self {
        let (rx, ry) = (rect.width * 0.5, rect.height * 0.5);
        let (cx, cy) = (rect.origin.x + rx, rect.origin.y + ry);
        let (kx, ky) = (rx * KAPPA, ry * KAPPA);
        let outline = [
            PathCmd::MoveTo(cx + rx, cy),
            PathCmd::CubicTo(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry),
            PathCmd::CubicTo(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy),
            PathCmd::CubicTo(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry),
            PathCmd::CubicTo(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy),
            PathCmd::Close,
        ];
        Self::outline(outline.into(), FillRule::NonZero)
    }

    /// The polygon through `points`, filled by the nonzero rule.
    #[must_use]
    pub fn polygon(points: &[Point]) -> Self {
        let mut outline = Vec::with_capacity(points.len() + 1);
        for (index, point) in points.iter().enumerate() {
            outline.push(if index == 0 {
                PathCmd::MoveTo(point.x, point.y)
            } else {
                PathCmd::LineTo(point.x, point.y)
            });
        }
        outline.push(PathCmd::Close);
        Self::outline(outline.into(), FillRule::NonZero)
    }

    pub(crate) fn outline(outline: Arc<[PathCmd]>, rule: FillRule) -> Self {
        let key = outline_key(&outline, rule);
        let draw = DrawPath::new(outline, IDENTITY, key).with_fill(rule, PathPaint::Solid(OPAQUE));
        Self::Path(draw)
    }

    pub(crate) const fn bounds(&self) -> Rect {
        match self {
            Self::Rect(rect) | Self::Rounded(rect, _) => *rect,
            Self::Path(draw) => draw.bounds,
        }
    }

    pub(crate) fn composites(&self) -> bool {
        match self {
            Self::Rect(_) => false,
            Self::Rounded(_, radii) => radii.0.iter().any(|radius| *radius > 0.0),
            Self::Path(_) => true,
        }
    }

    pub(crate) const fn mask(&self) -> Option<&DrawPath> {
        match self {
            Self::Path(draw) => Some(draw),
            Self::Rect(_) | Self::Rounded(..) => None,
        }
    }

    pub(crate) const fn scope(&self) -> LayerScope {
        LayerScope {
            clip: self.bounds(),
            clip_radii: match self {
                Self::Rounded(_, radii) => *radii,
                Self::Rect(_) | Self::Path(_) => CornerRadii::ZERO,
            },
            effect: LayerEffect::Identity,
            opacity: 1.0,
            blend: super::BlendMode::Normal,
        }
    }

    pub(crate) fn shift(&mut self, dx: f32, dy: f32) {
        match self {
            Self::Rect(rect) | Self::Rounded(rect, _) => {
                rect.origin.x += dx;
                rect.origin.y += dy;
            }
            Self::Path(draw) => draw.shift(dx, dy),
        }
    }
}

impl From<Rect> for Clip {
    fn from(rect: Rect) -> Self {
        Self::Rect(rect)
    }
}

fn outline_key(outline: &[PathCmd], rule: FillRule) -> u64 {
    let start = crate::fnv::mix(crate::fnv::OFFSET, u64::from(rule == FillRule::EvenOdd));
    outline.iter().fold(start, |hash, cmd| {
        let (verb, points, len) = match *cmd {
            PathCmd::MoveTo(x, y) => (1, [x, y, 0.0, 0.0, 0.0, 0.0], 2),
            PathCmd::LineTo(x, y) => (2, [x, y, 0.0, 0.0, 0.0, 0.0], 2),
            PathCmd::QuadTo(cx, cy, x, y) => (3, [cx, cy, x, y, 0.0, 0.0], 4),
            PathCmd::CubicTo(c1x, c1y, c2x, c2y, x, y) => (4, [c1x, c1y, c2x, c2y, x, y], 6),
            PathCmd::Close => (5, [0.0; 6], 0),
        };
        points
            .iter()
            .take(len)
            .fold(crate::fnv::mix(hash, verb), |hash, value| {
                crate::fnv::mix(hash, u64::from(value.to_bits()))
            })
    })
}
