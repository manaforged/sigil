use std::sync::Arc;

use smallvec::SmallVec;

use crate::geometry::{Point, Rect};
use crate::kernel::NodeId;
use crate::scene::command::Transform;
use crate::scene::command::effects::transform_rect;
use crate::scene::command::fills::{FillRule, PathPaint, StrokeCap, StrokeJoin};
use crate::scene::geometry::KAPPA;

/// One segment of a path outline, with its points in path units.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum PathCommand {
    /// Starts a subpath at `(x, y)`.
    MoveTo(f32, f32),
    /// A straight line to `(x, y)`.
    LineTo(f32, f32),
    /// A quadratic Bézier curve with control point `(cx, cy)` to `(x, y)`.
    QuadTo(f32, f32, f32, f32),
    /// A cubic Bézier curve with control points `(c1x, c1y)` and `(c2x, c2y)` to `(x, y)`.
    CubicTo(f32, f32, f32, f32, f32, f32),
    /// A straight line back to the start of the subpath, which closes it.
    Close,
}

impl PathCommand {
    fn points(self) -> SmallVec<[(f32, f32); 3]> {
        match self {
            Self::MoveTo(x, y) | Self::LineTo(x, y) => smallvec::smallvec![(x, y)],
            Self::QuadTo(cx, cy, x, y) => smallvec::smallvec![(cx, cy), (x, y)],
            Self::CubicTo(c1x, c1y, c2x, c2y, x, y) => {
                smallvec::smallvec![(c1x, c1y), (c2x, c2y), (x, y)]
            }
            Self::Close => SmallVec::new(),
        }
    }
}

/// The line drawn along a path.
#[derive(Clone, Debug, PartialEq)]
pub struct PathStroke {
    pub(crate) paint: PathPaint,
    pub(crate) width: f32,
    pub(crate) cap: StrokeCap,
    pub(crate) join: StrokeJoin,
    pub(crate) dash: SmallVec<[f32; 4]>,
}

impl PathStroke {
    /// Returns the color or gradient the line is painted with.
    #[must_use]
    pub const fn paint(&self) -> &PathPaint {
        &self.paint
    }

    /// Returns the width of the line in path units.
    #[must_use]
    pub const fn width(&self) -> f32 {
        self.width
    }

    /// Returns the shape of the line's open ends.
    #[must_use]
    pub const fn cap(&self) -> StrokeCap {
        self.cap
    }

    /// Returns the shape of the corners where two segments meet.
    #[must_use]
    pub const fn join(&self) -> StrokeJoin {
        self.join
    }

    /// Returns the miter limit: the ratio of a miter join's length to the line width past which the join is beveled, as in SVG `stroke-miterlimit`.
    #[must_use]
    #[allow(
        clippy::unused_self,
        reason = "every stroke shares one limit today; the getter keeps room for a per-stroke limit"
    )]
    pub const fn miter_limit(&self) -> f32 {
        MITER_LIMIT
    }

    /// Returns the dash lengths in path units, alternating on and off; empty for a solid line.
    #[must_use]
    pub fn dash(&self) -> &[f32] {
        &self.dash
    }

    fn reach(&self) -> f32 {
        let half = self.width.max(0.0) * 0.5;
        match (self.join, self.cap) {
            (StrokeJoin::Miter, _) => half * MITER_LIMIT,
            (_, StrokeCap::Square) => half * std::f32::consts::SQRT_2,
            _ => half,
        }
    }
}

pub(crate) const MITER_LIMIT: f32 = 4.0;

/// Mixes one `PathCommand` verb tag and coordinates into `hash`.
pub(crate) fn hash_path_cmd(hash: u64, cmd: PathCommand) -> u64 {
    let (verb, points, len) = match cmd {
        PathCommand::MoveTo(x, y) => (1, [x, y, 0.0, 0.0, 0.0, 0.0], 2),
        PathCommand::LineTo(x, y) => (2, [x, y, 0.0, 0.0, 0.0, 0.0], 2),
        PathCommand::QuadTo(cx, cy, x, y) => (3, [cx, cy, x, y, 0.0, 0.0], 4),
        PathCommand::CubicTo(c1x, c1y, c2x, c2y, x, y) => (4, [c1x, c1y, c2x, c2y, x, y], 6),
        PathCommand::Close => (5, [0.0; 6], 0),
    };
    points
        .iter()
        .take(len)
        .copied()
        .fold(crate::fnv::mix(hash, verb), crate::fnv::mix_f32)
}

/// A hash of `outline` mixed into `seed`.
pub(crate) fn outline_key(seed: u64, outline: &[PathCommand]) -> u64 {
    outline.iter().copied().fold(seed, hash_path_cmd)
}

/// The closed outline of `rect` with corner radii top-left, top-right, bottom-right, bottom-left.
pub(crate) fn rounded_outline(rect: Rect, radii: [f32; 4]) -> Vec<PathCommand> {
    let (l, t) = (rect.origin.x, rect.origin.y);
    let (r, b) = (l + rect.width, t + rect.height);
    let limit = rect.width.min(rect.height).max(0.0) * 0.5;
    let [tl, tr, br, bl] = radii.map(|radius| radius.clamp(0.0, limit));
    let arc = |x0: f32, y0: f32, x1: f32, y1: f32, cx: f32, cy: f32| {
        PathCommand::CubicTo(
            (cx - x0).mul_add(KAPPA, x0),
            (cy - y0).mul_add(KAPPA, y0),
            (cx - x1).mul_add(KAPPA, x1),
            (cy - y1).mul_add(KAPPA, y1),
            x1,
            y1,
        )
    };
    vec![
        PathCommand::MoveTo(l + tl, t),
        PathCommand::LineTo(r - tr, t),
        arc(r - tr, t, r, t + tr, r, t),
        PathCommand::LineTo(r, b - br),
        arc(r, b - br, r - br, b, r, b),
        PathCommand::LineTo(l + bl, b),
        arc(l + bl, b, l, b - bl, l, b),
        PathCommand::LineTo(l, t + tl),
        arc(l, t + tl, l + tl, t, l, t),
        PathCommand::Close,
    ]
}

/// A command that fills and strokes a vector path.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct DrawPath {
    pub(crate) node: Option<NodeId>,
    pub(crate) outline: Arc<[PathCommand]>,
    pub(crate) transform: Transform,
    pub(crate) fill: Option<(FillRule, PathPaint)>,
    pub(crate) stroke: Option<PathStroke>,
    pub(crate) opacity: u16,
    pub(crate) bounds: Rect,
    pub(crate) key: u64,
}

impl DrawPath {
    /// Sets the opacity in basis points; 10000 is opaque.
    pub const fn opacity(mut self, opacity: u16) -> Self {
        self.opacity = opacity;
        self
    }

    /// Returns the owning node, if any.
    #[must_use]
    pub const fn node(&self) -> Option<NodeId> {
        self.node
    }

    /// Returns the transformed path bounding rectangle.
    #[must_use]
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Returns the 2D affine transform applied to the path.
    #[must_use]
    pub const fn transform(&self) -> Transform {
        self.transform
    }

    /// Returns the opacity in basis points; 10000 is opaque.
    #[must_use]
    pub const fn opacity_bp(&self) -> u16 {
        self.opacity
    }

    /// Returns the segments of the outline, in path units before `transform`.
    #[must_use]
    pub fn outline(&self) -> &[PathCommand] {
        &self.outline
    }

    /// Returns the paint the inside of the path is filled with, if it is filled.
    #[must_use]
    pub fn fill(&self) -> Option<&PathPaint> {
        self.fill.as_ref().map(|(_, paint)| paint)
    }

    /// Returns the rule that decides which areas of a self-crossing outline are filled.
    #[must_use]
    pub const fn fill_rule(&self) -> FillRule {
        match &self.fill {
            Some((rule, _)) => *rule,
            None => FillRule::NonZero,
        }
    }

    /// Returns the line drawn along the outline, if it has one.
    #[must_use]
    pub const fn stroke(&self) -> Option<&PathStroke> {
        self.stroke.as_ref()
    }

    pub(crate) fn new(outline: Arc<[PathCommand]>, transform: Transform, key: u64) -> Self {
        Self {
            node: None,
            outline,
            transform,
            fill: None,
            stroke: None,
            opacity: 10_000,
            bounds: Rect::new(Point::new(0.0, 0.0), 0.0, 0.0),
            key,
        }
        .measured()
    }

    pub(crate) fn with_fill(mut self, rule: FillRule, paint: PathPaint) -> Self {
        self.fill = Some((rule, paint));
        self
    }

    pub(crate) fn stroked(mut self, stroke: PathStroke) -> Self {
        self.stroke = Some(stroke);
        self.measured()
    }

    pub(crate) fn shift(&mut self, dx: f32, dy: f32) {
        self.transform[0][2] += dx;
        self.transform[1][2] += dy;
        self.bounds.origin.x += dx;
        self.bounds.origin.y += dy;
    }

    #[cfg(feature = "terminal")]
    pub(crate) fn key_at(&self, scale: f32) -> u64 {
        [scale, self.bounds.width, self.bounds.height]
            .into_iter()
            .fold(self.key, crate::fnv::mix_f32)
    }

    fn measured(mut self) -> Self {
        let reach = self.stroke.as_ref().map_or(0.0, PathStroke::reach);
        let hull = self.outline.iter().flat_map(|cmd| cmd.points()).fold(
            None,
            |hull: Option<[f32; 4]>, (x, y)| {
                Some(hull.map_or([x, y, x, y], |[l, t, r, b]| {
                    [l.min(x), t.min(y), r.max(x), b.max(y)]
                }))
            },
        );
        self.bounds = hull.map_or(self.bounds, |[l, t, r, b]| {
            let local = Rect::new(Point::new(l, t), r - l, b - t).expand(reach);
            transform_rect(self.transform, local)
        });
        self
    }
}
