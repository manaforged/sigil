use std::sync::Arc;

use crate::geometry::{Point, Rect};
use crate::scene::command::fills::{FillRule, PathPaint};
use crate::scene::command::payloads::{DrawPath, PathCommand};
#[cfg(feature = "scene")]
use crate::scene::corner_radii::CornerRadii;
use crate::scene::geometry::{IDENTITY, KAPPA};
use crate::style::Color;

const OPAQUE: Color = Color {
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
#[non_exhaustive]
pub enum Clip {
    /// A rectangle, applied with the scissor.
    Rect(Rect),
    /// A rectangle with rounded corners.
    Rounded(Rect, [f32; 4]),
    /// The pixels a path covers, weighted by its alpha.
    Path(DrawPath),
}

impl Clip {
    /// A rectangle with corners of `radius`.
    #[must_use]
    pub const fn rounded(rect: Rect, radius: f32) -> Self {
        Self::Rounded(rect, [radius; 4])
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
            PathCommand::MoveTo(cx + rx, cy),
            PathCommand::CubicTo(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry),
            PathCommand::CubicTo(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy),
            PathCommand::CubicTo(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry),
            PathCommand::CubicTo(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy),
            PathCommand::Close,
        ];
        Self::outline(outline.into(), FillRule::NonZero)
    }

    /// The polygon through `points`, filled by the nonzero rule.
    #[must_use]
    pub fn polygon(points: &[Point]) -> Self {
        let mut outline = Vec::with_capacity(points.len() + 1);
        for (index, point) in points.iter().enumerate() {
            outline.push(if index == 0 {
                PathCommand::MoveTo(point.x, point.y)
            } else {
                PathCommand::LineTo(point.x, point.y)
            });
        }
        outline.push(PathCommand::Close);
        Self::outline(outline.into(), FillRule::NonZero)
    }

    pub(crate) fn outline(outline: Arc<[PathCommand]>, rule: FillRule) -> Self {
        let key = outline_key(&outline, rule);
        let draw = DrawPath::new(outline, IDENTITY, key).with_fill(rule, PathPaint::Solid(OPAQUE));
        Self::Path(draw)
    }

    #[cfg(vector_paint)]
    pub(crate) const fn bounds(&self) -> Rect {
        match self {
            Self::Rect(rect) | Self::Rounded(rect, _) => *rect,
            Self::Path(draw) => draw.bounds,
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

#[cfg(feature = "scene")]
impl Clip {
    pub(crate) fn composites(&self) -> bool {
        match self {
            Self::Rect(_) => false,
            Self::Rounded(_, radii) => radii.iter().any(|radius| *radius > 0.0),
            Self::Path(_) => true,
        }
    }

    #[cfg(scene_host)]
    pub(crate) const fn mask(&self) -> Option<&DrawPath> {
        match self {
            Self::Path(draw) => Some(draw),
            Self::Rect(_) | Self::Rounded(..) => None,
        }
    }

    pub(crate) const fn scope(&self) -> super::LayerScope {
        super::LayerScope {
            clip: self.bounds(),
            clip_radii: match self {
                Self::Rounded(_, radii) => CornerRadii(*radii),
                Self::Rect(_) | Self::Path(_) => CornerRadii::ZERO,
            },
            effect: super::LayerEffect::Identity,
            opacity: 1.0,
            blend: super::BlendMode::Normal,
        }
    }
}

impl From<Rect> for Clip {
    fn from(rect: Rect) -> Self {
        Self::Rect(rect)
    }
}

fn outline_key(outline: &[PathCommand], rule: FillRule) -> u64 {
    let start = crate::fnv::mix(crate::fnv::OFFSET, u64::from(rule == FillRule::EvenOdd));
    super::payloads::outline_key(start, outline)
}
