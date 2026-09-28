use std::f32::consts::{FRAC_1_SQRT_2, TAU};

use crate::renderer::shadow_paint::ShadowKind;

const SHARP_SIGMA: f32 = 0.01;
const SAMPLES: u8 = 4;

type Vec2 = (f32, f32);

#[derive(Clone, Copy)]
pub(super) struct ShadowField {
    pub(super) center: Vec2,
    pub(super) half: Vec2,
    pub(super) radii: [f32; 4],
    pub(super) offset: Vec2,
    pub(super) sigma: f32,
    pub(super) spread: f32,
    pub(super) kind: ShadowKind,
    pub(super) unit: f32,
}

impl ShadowField {
    pub(super) fn coverage(&self, px: f32, py: f32) -> f32 {
        let local = (px - self.center.0, py - self.center.1);
        let shifted = (local.0 - self.offset.0, local.1 - self.offset.1);
        if self.kind != ShadowKind::Inner {
            return self.body(shifted, self.spread);
        }
        let distance = sdf(local, self.half, self.radii, 0.0);
        let edge = 1.0 - smoothstep(-0.5, 0.5, distance);
        if edge <= 0.0 {
            return 0.0;
        }
        edge * (1.0 - self.body(shifted, -self.spread))
    }

    fn body(&self, p: Vec2, spread: f32) -> f32 {
        let half = (
            (self.half.0 + spread).max(0.0),
            (self.half.1 + spread).max(0.0),
        );
        if self.sigma <= SHARP_SIGMA * self.unit {
            let d = sdf(p, half, self.radii, spread);
            let aa = 0.5 * self.unit;
            return 1.0 - smoothstep(-aa, aa, d);
        }
        blurred(p, self.sigma, self.radii, half, spread)
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * 2.0f32.mul_add(-t, 3.0)
}

fn erf(x: f32) -> f32 {
    let a = x.abs();
    let mut v = 0.078_108f32
        .mul_add(a * a, 0.230_389)
        .mul_add(a, 0.278_393)
        .mul_add(a, 1.0);
    v *= v;
    x.signum() - x.signum() / (v * v)
}

fn corner_radius(p: Vec2, radii: [f32; 4]) -> f32 {
    let [tl, tr, br, bl] = radii;
    match (p.0 > 0.0, p.1 < 0.0) {
        (true, true) => tr,
        (false, true) => tl,
        (true, false) => br,
        (false, false) => bl,
    }
}

fn sdf(p: Vec2, half: Vec2, radii: [f32; 4], spread: f32) -> f32 {
    let radius = (corner_radius(p, radii) + spread).max(0.0);
    let q = (
        p.0.abs() - half.0.max(0.0) + radius,
        p.1.abs() - half.1.max(0.0) + radius,
    );
    let outside = q.0.max(0.0).hypot(q.1.max(0.0));
    outside + q.0.max(q.1).min(0.0) - radius
}

fn gaussian(x: f32, sigma: f32) -> f32 {
    (-(x * x) / (2.0 * sigma * sigma)).exp() / (TAU.sqrt() * sigma)
}

fn row(x: f32, y: f32, sigma: f32, radius: f32, half: Vec2) -> f32 {
    let dy = (half.1 - radius - y.abs()).min(0.0);
    let curved = half.0 - radius + dy.mul_add(-dy, radius * radius).max(0.0).sqrt();
    let k = FRAC_1_SQRT_2 / sigma;
    let low = 0.5f32.mul_add(erf((x - curved) * k), 0.5);
    let high = 0.5f32.mul_add(erf((x + curved) * k), 0.5);
    high - low
}

fn blurred(p: Vec2, sigma: f32, radii: [f32; 4], half: Vec2, spread: f32) -> f32 {
    let (low, high) = (p.1 - half.1, p.1 + half.1);
    let start = (-3.0 * sigma).max(low).min(high);
    let end = (3.0 * sigma).max(low).min(high);
    let step = (end - start) / f32::from(SAMPLES);
    let mut y = step.mul_add(0.5, start);
    let mut sum = 0.0;
    for _ in 0..SAMPLES {
        let sample = (p.0, p.1 - y);
        let radius = (corner_radius(sample, radii) + spread).max(0.0);
        sum = (row(p.0, sample.1, sigma, radius, half) * gaussian(y, sigma)).mul_add(step, sum);
        y += step;
    }
    sum.clamp(0.0, 1.0)
}
