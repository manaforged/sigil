use std::f32::consts::FRAC_PI_2;

const MIN_LEVEL: f32 = -6.0;
const MAX_LEVEL: f32 = 6.0;
#[cfg(all(feature = "cpu", host))]
const MIN_NORM: f32 = 1.0e-6;

/// The curve of a box corner, as in CSS `corner-shape`.
#[derive(Clone, Copy, Debug, Default)]
#[non_exhaustive]
pub enum CornerShape {
    /// A quarter ellipse, as in CSS `round`.
    #[default]
    Round,
    /// A superellipse between round and square, as in CSS `squircle`.
    Squircle,
    /// A straight cut across the corner, as in CSS `bevel`.
    Bevel,
    /// A concave quarter ellipse, as in CSS `scoop`.
    Scoop,
    /// The CSS `superellipse(k)` curve, with `k` from -6 to 6: 1 is round, 2 a squircle, 0 a bevel, and -1 a scoop.
    Superellipse(f32),
}

impl PartialEq for CornerShape {
    fn eq(&self, other: &Self) -> bool {
        self.code().to_bits() == other.code().to_bits()
    }
}

impl Eq for CornerShape {}

impl core::hash::Hash for CornerShape {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.code().to_bits().hash(state);
    }
}

impl CornerShape {
    const fn level(self) -> f32 {
        match self {
            Self::Round => 1.0,
            Self::Squircle => 2.0,
            Self::Bevel => 0.0,
            Self::Scoop => -1.0,
            Self::Superellipse(k) if k.is_nan() => 1.0,
            Self::Superellipse(k) => k.clamp(MIN_LEVEL, MAX_LEVEL),
        }
    }

    pub(crate) fn code(self) -> f32 {
        let level = self.level();
        if level.to_bits() == 1.0_f32.to_bits() {
            return 0.0;
        }
        let exponent = level.abs().exp2();
        if level < 0.0 { -exponent } else { exponent }
    }

    pub(crate) fn is_round(self) -> bool {
        self.code() == 0.0
    }
}

#[cfg(all(feature = "cpu", host))]
pub(crate) fn corner_distance(a: [f32; 2], code: f32) -> f32 {
    let [x, y] = a;
    if code < 0.0 {
        if x.max(y) >= 1.0 {
            return (x - 1.0).hypot(y).min(x.hypot(y - 1.0));
        }
        let (norm, slope) = norm_slope([1.0 - x, 1.0 - y], -code);
        return (1.0 - norm) / slope;
    }
    let (norm, slope) = norm_slope(a, code);
    (norm - 1.0) / slope
}

#[cfg(all(feature = "cpu", host))]
fn norm_slope([x, y]: [f32; 2], exponent: f32) -> (f32, f32) {
    let norm = (x.powf(exponent) + y.powf(exponent)).powf(exponent.recip());
    let scale = norm.max(MIN_NORM);
    let pull = |v: f32| (v / scale).max(MIN_NORM).powf(exponent - 1.0);
    (norm, pull(x).hypot(pull(y)).max(MIN_NORM))
}

#[cfg(all(feature = "cpu", host))]
pub(crate) fn corner_reach(t: f32, code: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if code < 0.0 {
        let m = -code;
        return 1.0 - (1.0 - (1.0 - t).powf(m)).max(0.0).powf(m.recip());
    }
    (1.0 - t.powf(code)).max(0.0).powf(code.recip())
}

pub(crate) fn corner_point(theta: f32, code: f32) -> [f32; 2] {
    let theta = theta.clamp(0.0, FRAC_PI_2);
    let (sin, cos) = theta.sin_cos();
    if code < 0.0 {
        let power = 2.0 / -code;
        return [
            1.0 - sin.max(0.0).powf(power),
            1.0 - cos.max(0.0).powf(power),
        ];
    }
    let power = 2.0 / code;
    [cos.max(0.0).powf(power), sin.max(0.0).powf(power)]
}
