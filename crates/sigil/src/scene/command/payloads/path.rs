use std::sync::Arc;

use smallvec::SmallVec;

use crate::geometry::{Point, Rect};
use crate::kernel::NodeId;
use crate::scene::command::Transform;
use crate::scene::command::effects::transform_rect;
use crate::scene::command::fills::{FillRule, PathPaint, StrokeCap, StrokeJoin};
use crate::scene::geometry::KAPPA;

/// One verb of a path outline with its points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PathCmd {
    /// Starts a subpath.
    MoveTo(f32, f32),
    /// A straight line.
    LineTo(f32, f32),
    /// A quadratic curve.
    QuadTo(f32, f32, f32, f32),
    /// A cubic curve.
    CubicTo(f32, f32, f32, f32, f32, f32),
    /// Closes the subpath.
    Close,
}

impl PathCmd {
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

/// The outline drawn around a path.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PathStroke {
    /// The paint.
    pub paint: PathPaint,
    /// The width in path units.
    pub width: f32,
    /// The cap.
    pub cap: StrokeCap,
    /// The join.
    pub join: StrokeJoin,
    /// The dash lengths, on then off; empty for a solid line.
    pub dash: SmallVec<[f32; 4]>,
}

impl PathStroke {
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

/// A hash of `outline` mixed into `seed`.
pub(crate) fn outline_key(seed: u64, outline: &[PathCmd]) -> u64 {
    outline.iter().fold(seed, |hash, cmd| {
        cmd.points()
            .iter()
            .fold(crate::fnv::mix(hash, 1), |hash, (x, y)| {
                crate::fnv::mix_f32(crate::fnv::mix_f32(hash, *x), *y)
            })
    })
}

/// The closed outline of `rect` with corner radii top-left, top-right, bottom-right, bottom-left.
pub(crate) fn rounded_outline(rect: Rect, radii: [f32; 4]) -> Vec<PathCmd> {
    let (l, t) = (rect.origin.x, rect.origin.y);
    let (r, b) = (l + rect.width, t + rect.height);
    let limit = rect.width.min(rect.height).max(0.0) * 0.5;
    let [tl, tr, br, bl] = radii.map(|radius| radius.clamp(0.0, limit));
    let arc = |x0: f32, y0: f32, x1: f32, y1: f32, cx: f32, cy: f32| {
        PathCmd::CubicTo(
            (cx - x0).mul_add(KAPPA, x0),
            (cy - y0).mul_add(KAPPA, y0),
            (cx - x1).mul_add(KAPPA, x1),
            (cy - y1).mul_add(KAPPA, y1),
            x1,
            y1,
        )
    };
    vec![
        PathCmd::MoveTo(l + tl, t),
        PathCmd::LineTo(r - tr, t),
        arc(r - tr, t, r, t + tr, r, t),
        PathCmd::LineTo(r, b - br),
        arc(r, b - br, r - br, b, r, b),
        PathCmd::LineTo(l + bl, b),
        arc(l + bl, b, l, b - bl, l, b),
        PathCmd::LineTo(l, t + tl),
        arc(l, t + tl, l + tl, t, l, t),
        PathCmd::Close,
    ]
}

kittest_pub! {
    /// A command that fills and strokes a vector path.
    #[doc(hidden)]
    #[derive(Clone, Debug, PartialEq)]
    pub struct DrawPath {
        pub(crate) node: Option<NodeId>,
        pub(crate) outline: Arc<[PathCmd]>,
        pub(crate) transform: Transform,
        pub(crate) fill: Option<(FillRule, PathPaint)>,
        pub(crate) stroke: Option<PathStroke>,
        pub(crate) opacity: u16,
        pub(crate) bounds: Rect,
        pub(crate) key: u64,
    }
}

impl DrawPath {
    pub(crate) fn new(outline: Arc<[PathCmd]>, transform: Transform, key: u64) -> Self {
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

    pub(crate) fn with_stroke(mut self, stroke: PathStroke) -> Self {
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
