use std::hash::{Hash, Hasher};

use crate::geometry::{Point, Rect};

use super::{AnimatorOverlay, NumericAtomKind, StyleAtom, StyleBinding};

const SHIFT_UNITS: f32 = 2_048.0;

#[expect(
    clippy::cast_possible_truncation,
    reason = "the fraction is clamped to the i16 range of 1/2048 steps before the cast"
)]
fn shift_units(fraction: f32) -> i16 {
    (fraction * SHIFT_UNITS)
        .round()
        .clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16
}

/// Holds a 2D translation, scale, and rotation for a node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform2D {
    translation: [f32; 2],
    shift: [i16; 2],
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
        self.shift.hash(state);
        self.origin.hash(state);
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform2D {
    /// The transform that changes nothing.
    pub const IDENTITY: Self = Self {
        translation: [0.0, 0.0],
        shift: [0, 0],
        scale: [1.0, 1.0],
        rotation: 0.0,
        origin: [5_000, 5_000],
    };

    /// Adds a translation.
    #[must_use]
    pub fn translate(mut self, x: f32, y: f32) -> Self {
        assert!(x.is_finite() && y.is_finite());
        self.translation = [x, y];
        self
    }

    /// Adds a scale.
    #[must_use]
    pub fn scale(mut self, x: f32, y: f32) -> Self {
        assert!(x.is_finite() && y.is_finite());
        self.scale = [x, y];
        self
    }

    /// Adds a rotation.
    #[must_use]
    pub fn rotate(mut self, radians: f32) -> Self {
        assert!(radians.is_finite());
        self.rotation = radians;
        self
    }

    /// Sets the point the transform turns around.
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

    pub(crate) fn value(self, kind: NumericAtomKind) -> Option<f32> {
        match kind {
            NumericAtomKind::TranslateX => Some(self.translation[0]),
            NumericAtomKind::TranslateY => Some(self.translation[1]),
            NumericAtomKind::ScaleX => Some(self.scale[0]),
            NumericAtomKind::ScaleY => Some(self.scale[1]),
            NumericAtomKind::Rotation => Some(self.rotation),
            NumericAtomKind::TranslateXFraction => Some(f32::from(self.shift[0]) / SHIFT_UNITS),
            NumericAtomKind::TranslateYFraction => Some(f32::from(self.shift[1]) / SHIFT_UNITS),
            _ => None,
        }
    }

    pub(crate) fn overlay(self, overlay: &dyn AnimatorOverlay) -> Self {
        let value = |kind| overlay.value_for(kind).or_else(|| self.value(kind));
        let get = |kind| value(kind).unwrap_or_default();
        Self {
            shift: [
                shift_units(get(NumericAtomKind::TranslateXFraction)),
                shift_units(get(NumericAtomKind::TranslateYFraction)),
            ],
            ..self
        }
        .translate(
            get(NumericAtomKind::TranslateX),
            get(NumericAtomKind::TranslateY),
        )
        .scale(get(NumericAtomKind::ScaleX), get(NumericAtomKind::ScaleY))
        .rotate(get(NumericAtomKind::Rotation))
    }

    pub(crate) fn is_identity(self) -> bool {
        self.translation == [0.0, 0.0]
            && self.shift == [0, 0]
            && self.scale.map(f32::to_bits) == [1.0_f32.to_bits(); 2]
            && self.rotation == 0.0
    }

    pub(crate) fn matrix(self, rect: Rect) -> [[f32; 3]; 2] {
        let [sx_shift, sy_shift] = self.shift.map(|units| f32::from(units) / SHIFT_UNITS);
        let tx = sx_shift.mul_add(rect.width, self.translation[0]);
        let ty = sy_shift.mul_add(rect.height, self.translation[1]);
        let [sx, sy] = self.scale;
        let Point {
            x: pivot_x,
            y: pivot_y,
        } = self.pivot(rect);
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

    pub(crate) fn pivot(self, rect: Rect) -> Point {
        let [ox, oy] = self.origin.map(|value| f32::from(value) / 10_000.0);
        Point::new(
            ox.mul_add(rect.width, rect.origin.x),
            oy.mul_add(rect.height, rect.origin.y),
        )
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
