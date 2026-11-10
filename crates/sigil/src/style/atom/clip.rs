use std::hash::{Hash, Hasher};

use crate::geometry::{Point, Rect};
use crate::scene::Clip;

/// The region a node and its children keep, like CSS `clip-path`, measured against the border box.
#[derive(Debug, Clone, Copy)]
pub enum ClipShape {
    /// `inset(top right bottom left round radius)` in px.
    Inset {
        /// The top, right, bottom, and left insets.
        inset: [i16; 4],
        /// The corner radius.
        round: i16,
    },
    /// `circle(radius)` centred in the box; `None` reaches the closest side.
    Circle(Option<i16>),
    /// `ellipse()` filling the box.
    Ellipse,
    /// `polygon(..)` with points as fractions of the box width and height.
    Polygon(&'static [(f32, f32)]),
}

impl ClipShape {
    #[must_use]
    pub(crate) fn resolve(self, rect: Rect) -> Clip {
        let (x, y, w, h) = (rect.origin.x, rect.origin.y, rect.width, rect.height);
        match self {
            Self::Inset { inset, round } => {
                let [top, right, bottom, left] = inset.map(f32::from);
                let inner = Rect::new(
                    Point::new(x + left, y + top),
                    (w - left - right).max(0.0),
                    (h - top - bottom).max(0.0),
                );
                if round > 0 {
                    Clip::rounded(inner, f32::from(round))
                } else {
                    Clip::Rect(inner)
                }
            }
            Self::Circle(radius) => {
                let reach = radius.map_or_else(|| w.min(h) * 0.5, f32::from);
                Clip::circle(Point::new(w.mul_add(0.5, x), h.mul_add(0.5, y)), reach)
            }
            Self::Ellipse => Clip::ellipse(rect),
            Self::Polygon(points) => {
                let mapped: Vec<Point> = points
                    .iter()
                    .map(|(px, py)| Point::new(w.mul_add(*px, x), h.mul_add(*py, y)))
                    .collect();
                Clip::polygon(&mapped)
            }
        }
    }

    #[must_use]
    pub(crate) fn contains(self, rect: Rect, point: Point) -> bool {
        let (x, y, w, h) = (rect.origin.x, rect.origin.y, rect.width, rect.height);
        match self {
            Self::Inset { inset, round } => {
                let [top, right, bottom, left] = inset.map(f32::from);
                let inner = [x + left, y + top, x + w - right, y + h - bottom];
                in_rounded(inner, f32::from(round.max(0)), point)
            }
            Self::Circle(radius) => {
                let reach = radius.map_or_else(|| w.min(h) * 0.5, f32::from);
                w.mul_add(-0.5, point.x - x)
                    .hypot(h.mul_add(-0.5, point.y - y))
                    <= reach
            }
            Self::Ellipse => {
                let (rx, ry) = ((w * 0.5).max(f32::EPSILON), (h * 0.5).max(f32::EPSILON));
                ((point.x - x - rx) / rx).hypot((point.y - y - ry) / ry) <= 1.0
            }
            Self::Polygon(points) => in_polygon(points, rect, point),
        }
    }
}

fn in_rounded([l, t, r, b]: [f32; 4], radius: f32, point: Point) -> bool {
    if point.x < l || point.x > r || point.y < t || point.y > b {
        return false;
    }
    let radius = radius.min((r - l) * 0.5).min((b - t) * 0.5);
    let cx = point.x.clamp(l + radius, r - radius);
    let cy = point.y.clamp(t + radius, b - radius);
    (point.x - cx).hypot(point.y - cy) <= radius
}

fn in_polygon(points: &[(f32, f32)], rect: Rect, point: Point) -> bool {
    let at = |(px, py): (f32, f32)| {
        (
            rect.width.mul_add(px, rect.origin.x),
            rect.height.mul_add(py, rect.origin.y),
        )
    };
    let mut inside = false;
    let mut previous = points.last().copied().map(at);
    for corner in points.iter().copied().map(at) {
        if let Some((px, py)) = previous {
            let (cx, cy) = corner;
            if (cy > point.y) != (py > point.y)
                && point.x < (px - cx) * (point.y - cy) / (py - cy) + cx
            {
                inside = !inside;
            }
        }
        previous = Some(corner);
    }
    inside
}

impl PartialEq for ClipShape {
    fn eq(&self, other: &Self) -> bool {
        let bits = |points: &[(f32, f32)]| -> Vec<(u32, u32)> {
            points
                .iter()
                .map(|(x, y)| (x.to_bits(), y.to_bits()))
                .collect()
        };
        match (self, other) {
            (Self::Inset { inset: a, round: r }, Self::Inset { inset: b, round: q }) => {
                a == b && r == q
            }
            (Self::Circle(a), Self::Circle(b)) => a == b,
            (Self::Ellipse, Self::Ellipse) => true,
            (Self::Polygon(a), Self::Polygon(b)) => bits(a) == bits(b),
            _ => false,
        }
    }
}

impl Eq for ClipShape {}

impl Hash for ClipShape {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Inset { inset, round } => (inset, round).hash(state),
            Self::Circle(radius) => radius.hash(state),
            Self::Ellipse => {}
            Self::Polygon(points) => {
                for (x, y) in *points {
                    (x.to_bits(), y.to_bits()).hash(state);
                }
            }
        }
    }
}
