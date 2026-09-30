use std::hash::{Hash, Hasher};

use crate::geometry::{Point, Rect};

use super::{AnimatorOverlay, NumericAtomKind, StyleAtom, StyleBinding};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    translation: [f32; 2],
    scale: [f32; 2],
    rotation: f32,
    origin: [u16; 2],
}

impl Eq for Transform2D {}

impl Hash for Transform2D {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for value in self
            .translation
            .into_iter()
            .chain(self.scale)
            .chain([self.rotation])
        {
            (if value == 0.0 { 0 } else { value.to_bits() }).hash(state);
        }
        self.origin.hash(state);
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform2D {
    pub const IDENTITY: Self = Self {
        translation: [0.0, 0.0],
        scale: [1.0, 1.0],
        rotation: 0.0,
        origin: [5_000, 5_000],
    };

    #[must_use]
    pub fn translate(mut self, x: f32, y: f32) -> Self {
        assert!(x.is_finite() && y.is_finite());
        self.translation = [x, y];
        self
    }

    #[must_use]
    pub fn scale(mut self, x: f32, y: f32) -> Self {
        assert!(x.is_finite() && y.is_finite());
        self.scale = [x, y];
        self
    }

    #[must_use]
    pub fn rotate(mut self, radians: f32) -> Self {
        assert!(radians.is_finite());
        self.rotation = radians;
        self
    }

    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "origin fractions are validated and encoded in ten-thousandths"
    )]
    pub fn origin(mut self, x: f32, y: f32) -> Self {
        assert!((0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y));
        self.origin = [(x * 10_000.0).round() as u16, (y * 10_000.0).round() as u16];
        self
    }

    pub(crate) const fn value(self, kind: NumericAtomKind) -> Option<f32> {
        match kind {
            NumericAtomKind::TranslateX => Some(self.translation[0]),
            NumericAtomKind::TranslateY => Some(self.translation[1]),
            NumericAtomKind::ScaleX => Some(self.scale[0]),
            NumericAtomKind::ScaleY => Some(self.scale[1]),
            NumericAtomKind::Rotation => Some(self.rotation),
            _ => None,
        }
    }

    pub(crate) fn overlay(self, overlay: &dyn AnimatorOverlay) -> Self {
        self.translate(
            overlay
                .value_for(NumericAtomKind::TranslateX)
                .unwrap_or(self.translation[0]),
            overlay
                .value_for(NumericAtomKind::TranslateY)
                .unwrap_or(self.translation[1]),
        )
        .scale(
            overlay
                .value_for(NumericAtomKind::ScaleX)
                .unwrap_or(self.scale[0]),
            overlay
                .value_for(NumericAtomKind::ScaleY)
                .unwrap_or(self.scale[1]),
        )
        .rotate(
            overlay
                .value_for(NumericAtomKind::Rotation)
                .unwrap_or(self.rotation),
        )
    }

    pub(crate) fn is_identity(self) -> bool {
        self.translation == [0.0, 0.0]
            && self.scale.map(f32::to_bits) == [1.0_f32.to_bits(); 2]
            && self.rotation == 0.0
    }

    pub(crate) fn matrix(self, rect: Rect) -> [[f32; 3]; 2] {
        let [tx, ty] = self.translation;
        let [sx, sy] = self.scale;
        let [ox, oy] = self.origin.map(|value| f32::from(value) / 10_000.0);
        let pivot_x = ox.mul_add(rect.width, rect.origin.x);
        let pivot_y = oy.mul_add(rect.height, rect.origin.y);
        let (sin, cos) = self.rotation.sin_cos();
        let (xx, xy, yx, yy) = (cos * sx, -sin * sy, sin * sx, cos * sy);
        [
            [
                xx,
                xy,
                xx.mul_add(-pivot_x, xy.mul_add(-pivot_y, pivot_x + tx)),
            ],
            [
                yx,
                yy,
                yx.mul_add(-pivot_x, yy.mul_add(-pivot_y, pivot_y + ty)),
            ],
        ]
    }

    pub(crate) fn inverse(self, point: Point, rect: Rect) -> Option<Point> {
        if self.is_identity() {
            return Some(point);
        }
        let [[xx, xy, tx], [yx, yy, ty]] = self.matrix(rect);
        let determinant = xx.mul_add(yy, -xy * yx);
        if determinant == 0.0 || !determinant.is_finite() {
            return None;
        }
        let (local_x, local_y) = (point.x - tx, point.y - ty);
        Some(Point::new(
            yy.mul_add(local_x, -xy * local_y) / determinant,
            xx.mul_add(local_y, -yx * local_x) / determinant,
        ))
    }

    pub(crate) fn for_binding(binding: &StyleBinding) -> Self {
        let mut transform = Self::IDENTITY;
        for atom in binding.cascade() {
            if let StyleAtom::Transform(value) = atom {
                transform = value;
            }
        }
        binding
            .animator_overlay()
            .map_or(transform, |overlay| transform.overlay(overlay))
    }
}
