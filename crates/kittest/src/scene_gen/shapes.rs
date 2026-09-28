use sigil::hal::{Point, Rect};
use sigil::scene::{
    CornerRadii, LayerEffect, LayerScope, Shadow, ShadowKind, Stroke, StrokeAlign, StrokeStyle,
    Transform,
};
use sigil::style::Rgba8;

use super::Gen;

const EXTREMES: [f32; 6] = [1.0e7, -1.0e7, 65_536.0, -65_536.0, 0.0, 0.25];
const SIZES: [f32; 8] = [0.0, 0.01, 0.3, 0.5, 1.0, 1.5, 1.0e5, 1.0e7];
const RADII: [f32; 7] = [0.0, 0.5, 4.0, 16.0, 1.0e6, -3.0, 1000.0];
const WIDTHS: [f32; 7] = [0.0, 0.1, 0.5, 1.0, 3.0, 40.0, 1.0e6];

impl Gen<'_, '_, '_> {
    pub(crate) fn coord(&mut self, extent: f32) -> f32 {
        match self.entropy.below(10) {
            0 => self.entropy.pick(&EXTREMES),
            1 => self.entropy.range(-1.0, 1.0),
            _ => self.entropy.range(-0.25 * extent, 1.1 * extent),
        }
    }

    pub(crate) fn extent(&mut self, extent: f32) -> f32 {
        if self.entropy.chance(15) {
            return self.entropy.pick(&SIZES);
        }
        self.entropy.range(0.0, extent)
    }

    pub(crate) fn rect(&mut self) -> Rect {
        let (w, h) = self.viewport;
        let origin = Point::new(self.coord(w), self.coord(h));
        Rect::new(origin, self.extent(w), self.extent(h))
    }

    pub(crate) fn color(&mut self) -> Rgba8 {
        let e = &mut *self.entropy;
        let a = if e.chance(30) { 255 } else { e.byte() };
        Rgba8 {
            r: e.byte(),
            g: e.byte(),
            b: e.byte(),
            a,
        }
    }

    pub(crate) fn opacity_u16(&mut self) -> u16 {
        match self.entropy.below(4) {
            0 => 0,
            1 => 10_000,
            _ => u16::try_from(self.entropy.below(10_001)).unwrap_or(10_000),
        }
    }

    pub(crate) fn opacity_f32(&mut self) -> f32 {
        f32::from(self.opacity_u16()) / 10_000.0
    }

    pub(crate) fn radii(&mut self) -> CornerRadii {
        if self.entropy.chance(40) {
            return CornerRadii::ZERO;
        }
        if self.entropy.chance(50) {
            return CornerRadii::uniform(self.entropy.pick(&RADII));
        }
        let e = &mut *self.entropy;
        CornerRadii::new(
            e.pick(&RADII),
            e.pick(&RADII),
            e.pick(&RADII),
            e.pick(&RADII),
        )
    }

    pub(crate) fn stroke(&mut self) -> Option<Stroke> {
        if self.entropy.chance(60) {
            return None;
        }
        let color = self.color();
        let e = &mut *self.entropy;
        let style = if e.chance(30) {
            StrokeStyle::Dashed(e.pick(&[0.0, 0.01, 1.0, 4.0, 1.0e6]))
        } else {
            StrokeStyle::Solid
        };
        Some(Stroke {
            color,
            width: e.pick(&WIDTHS),
            style,
            align: e.pick(&[
                StrokeAlign::Centered,
                StrokeAlign::Inside,
                StrokeAlign::Outside,
            ]),
        })
    }

    pub(crate) fn transform(&mut self) -> Transform {
        let (w, h) = self.viewport;
        let (tx, ty) = (self.coord(w), self.coord(h));
        let rng = &mut *self.entropy;
        let [xx, xy, yx, yy] = match rng.below(7) {
            0 => [1.0, 0.0, 0.0, 1.0],
            1 => {
                let factor = rng.pick(&[0.0, -1.0, 0.5, 2.0, 1.0e-4, 1.0e4, -3.0]);
                [factor, 0.0, 0.0, rng.pick(&[factor, 1.0, -1.0, 0.0])]
            }
            2 | 3 => {
                let angle = rng.range(-std::f32::consts::TAU, std::f32::consts::TAU);
                let (sin, cos) = angle.sin_cos();
                [cos, sin, -sin, cos]
            }
            4 => [1.0, rng.range(-2.0, 2.0), rng.range(-2.0, 2.0), 1.0],
            5 => [0.0, 0.0, 0.0, 0.0],
            _ => [
                rng.range(-3.0, 3.0),
                rng.range(-3.0, 3.0),
                rng.range(-3.0, 3.0),
                rng.range(-3.0, 3.0),
            ],
        };
        let (tx, ty) = if self.entropy.chance(50) {
            (tx, ty)
        } else {
            (0.0, 0.0)
        };
        [[xx, yx, tx], [xy, yy, ty]]
    }

    pub(crate) fn layer_scope(&mut self) -> LayerScope {
        let clip = self.rect();
        let clip_radii = self.radii();
        let effect = match self.entropy.below(3) {
            0 => LayerEffect::Identity,
            1 => LayerEffect::Transform(self.transform()),
            _ => LayerEffect::LayerBlur {
                sigma: self.entropy.pick(&[0.0, 0.3, 2.0, 8.0, 64.0, 1.0e4]),
            },
        };
        LayerScope {
            clip,
            clip_radii,
            effect,
            opacity: self.opacity_f32(),
        }
    }

    pub(crate) fn shadow(&mut self) -> Shadow {
        let e = &mut *self.entropy;
        let color = [e.unit(), e.unit(), e.unit(), e.unit()];
        let offset = [e.range(-40.0, 40.0), e.range(-40.0, 40.0)];
        let blur = e.pick(&[0.0, 0.2, 2.0, 12.0, 80.0, 1.0e5]);
        let spread = e.pick(&[0.0, -8.0, 4.0, -1.0e5, 1.0e5]);
        let kind = e.pick(&[ShadowKind::Outer, ShadowKind::Inner, ShadowKind::Glow]);
        Shadow::new(offset, blur, spread, color).with_kind(kind)
    }
}
