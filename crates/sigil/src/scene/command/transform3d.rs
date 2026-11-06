use smallvec::SmallVec;

use crate::geometry::{Point, Rect};

use super::effects::Transform;

const MIN_W: f32 = 1.0e-4;

/// A CSS 4×4 transform that maps a layer's plane with rotation and perspective.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform3D([[f32; 4]; 4]);

impl Default for Transform3D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl From<Transform> for Transform3D {
    fn from([[xx, xy, tx], [yx, yy, ty]]: Transform) -> Self {
        Self([
            [xx, xy, 0.0, tx],
            [yx, yy, 0.0, ty],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }
}

impl Transform3D {
    /// The transform that changes nothing.
    pub const IDENTITY: Self = Self([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    /// Applies `next` after this transform's functions, like the next entry of a CSS `transform` list.
    #[must_use]
    pub fn then(self, next: Self) -> Self {
        let columns = [0, 1, 2, 3].map(|col| next.0.map(|row| row.get(col).copied().unwrap_or(0.0)));
        Self(self.0.map(|row| {
            columns.map(|column| row.iter().zip(column).map(|(x, y)| x * y).sum())
        }))
    }

    /// Adds a CSS `translate3d(x, y, z)`.
    #[must_use]
    pub fn translate(self, x: f32, y: f32, z: f32) -> Self {
        let mut m = Self::IDENTITY.0;
        (m[0][3], m[1][3], m[2][3]) = (x, y, z);
        self.then(Self(m))
    }

    /// Adds a CSS `rotateX(angle)` in radians.
    #[must_use]
    pub fn rotate_x(self, radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        let mut m = Self::IDENTITY.0;
        (m[1][1], m[1][2], m[2][1], m[2][2]) = (c, -s, s, c);
        self.then(Self(m))
    }

    /// Adds a CSS `rotateY(angle)` in radians.
    #[must_use]
    pub fn rotate_y(self, radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        let mut m = Self::IDENTITY.0;
        (m[0][0], m[0][2], m[2][0], m[2][2]) = (c, s, -s, c);
        self.then(Self(m))
    }

    /// Adds a CSS `rotateZ(angle)` in radians.
    #[must_use]
    pub fn rotate_z(self, radians: f32) -> Self {
        let (s, c) = radians.sin_cos();
        let mut m = Self::IDENTITY.0;
        (m[0][0], m[0][1], m[1][0], m[1][1]) = (c, -s, s, c);
        self.then(Self(m))
    }

    /// Adds a CSS `perspective(distance)`; distances under 1 px count as 1 px.
    #[must_use]
    pub fn perspective(self, distance: f32) -> Self {
        let mut m = Self::IDENTITY.0;
        m[3][2] = -1.0 / distance.max(1.0);
        self.then(Self(m))
    }

    /// Wraps `inner` so it turns around `origin`, like CSS `transform-origin` and `perspective-origin`.
    #[must_use]
    pub fn around(self, origin: Point, inner: Self) -> Self {
        self.translate(origin.x, origin.y, 0.0)
            .then(inner)
            .translate(-origin.x, -origin.y, 0.0)
    }

    pub(crate) fn back_facing(self) -> bool {
        self.0[2][2] < 0.0
    }

    pub(crate) fn visible(self, hide_backface: bool) -> Option<Self> {
        let finite = self.0.iter().flatten().all(|value| value.is_finite());
        (finite && !(hide_backface && self.back_facing())).then_some(self)
    }

    pub(crate) fn homography(self) -> [[f32; 3]; 3] {
        let m = self.0;
        [
            [m[0][0], m[0][1], m[0][3]],
            [m[1][0], m[1][1], m[1][3]],
            [m[3][0], m[3][1], m[3][3]],
        ]
    }

    pub(crate) fn unproject(self, point: Point) -> Option<Point> {
        let forward = self.homography();
        let [u, v, s] = apply(invert(forward)?, [point.x, point.y, 1.0]);
        if s.abs() < f32::MIN_POSITIVE {
            return None;
        }
        let local = Point::new(u / s, v / s);
        let w = apply(forward, [local.x, local.y, 1.0])[2];
        (w > MIN_W && local.x.is_finite() && local.y.is_finite()).then_some(local)
    }

    pub(crate) fn polygon(self, corners: &[Point]) -> SmallVec<[Point; 8]> {
        let plane = self.homography();
        let mapped: SmallVec<[[f32; 3]; 8]> =
            corners.iter().map(|p| apply(plane, [p.x, p.y, 1.0])).collect();
        let mut out = SmallVec::new();
        for (index, &a) in mapped.iter().enumerate() {
            let b = mapped.get(index + 1).or(mapped.first()).copied().unwrap_or(a);
            if a[2] >= MIN_W {
                out.push(Point::new(a[0] / a[2], a[1] / a[2]));
            }
            if (a[2] >= MIN_W) != (b[2] >= MIN_W) {
                let t = (MIN_W - a[2]) / (b[2] - a[2]);
                let at = |k: usize| (b[k] - a[k]).mul_add(t, a[k]);
                out.push(Point::new(at(0) / MIN_W, at(1) / MIN_W));
            }
        }
        out
    }

    pub(crate) fn bounds(self, rect: Rect) -> Option<Rect> {
        let (right, bottom) = (rect.origin.x + rect.width, rect.origin.y + rect.height);
        let corners = [
            rect.origin,
            Point::new(right, rect.origin.y),
            Point::new(right, bottom),
            Point::new(rect.origin.x, bottom),
        ];
        let visible = self.polygon(&corners);
        let first = visible.first()?;
        let (mut x0, mut y0, mut x1, mut y1) = (first.x, first.y, first.x, first.y);
        for point in &visible {
            (x0, y0) = (x0.min(point.x), y0.min(point.y));
            (x1, y1) = (x1.max(point.x), y1.max(point.y));
        }
        Some(Rect::new(Point::new(x0, y0), x1 - x0, y1 - y0))
    }

    #[cfg(feature = "terminal")]
    pub(crate) fn affine_fit(self, rect: Rect) -> Transform {
        let Some(bounds) = self.bounds(rect).filter(|_| rect.width > 0.0 && rect.height > 0.0)
        else {
            return [[0.0, 0.0, rect.origin.x], [0.0, 0.0, rect.origin.y]];
        };
        let (sx, sy) = (bounds.width / rect.width, bounds.height / rect.height);
        [
            [sx, 0.0, sx.mul_add(-rect.origin.x, bounds.origin.x)],
            [0.0, sy, sy.mul_add(-rect.origin.y, bounds.origin.y)],
        ]
    }
}

pub(crate) fn apply(m: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    m.map(|row| row[0].mul_add(v[0], row[1].mul_add(v[1], row[2] * v[2])))
}

pub(crate) fn invert(m: [[f32; 3]; 3]) -> Option<[[f32; 3]; 3]> {
    let [[a, b, c], [d, e, f], [g, h, i]] = m;
    let adjugate = [
        [e.mul_add(i, -f * h), c.mul_add(h, -b * i), b.mul_add(f, -c * e)],
        [f.mul_add(g, -d * i), a.mul_add(i, -c * g), c.mul_add(d, -a * f)],
        [d.mul_add(h, -e * g), b.mul_add(g, -a * h), a.mul_add(e, -b * d)],
    ];
    let det = a.mul_add(
        adjugate[0][0],
        b.mul_add(adjugate[1][0], c * adjugate[2][0]),
    );
    det.is_normal()
        .then(|| adjugate.map(|row| row.map(|value| value / det)))
}
