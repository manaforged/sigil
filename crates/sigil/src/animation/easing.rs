#![allow(clippy::suboptimal_flops)]

use core::f32::consts::{FRAC_PI_2, PI};
use std::num::NonZeroU16;

use super::bezier::CubicBezier;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Easing {
    #[default]
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    SineIn,
    SineOut,
    SineInOut,
    ExpoIn,
    ExpoOut,
    BackOut,
    BackInOut,
    BounceOut,
    Bezier(CubicBezier),
    Steps {
        count: NonZeroU16,
        start: bool,
    },
}

impl Easing {
    #[must_use]
    pub fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self::Bezier(CubicBezier::new(x1, y1, x2, y2))
    }

    #[must_use]
    pub const fn steps(count: NonZeroU16, start: bool) -> Self {
        Self::Steps { count, start }
    }

    #[must_use]
    pub fn apply(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::QuadIn => t * t,
            Self::QuadOut => 1.0 - (1.0 - t).powi(2),
            Self::QuadInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0f32).mul_add(t, 2.0).powi(2) / 2.0
                }
            }
            Self::CubicIn => t * t * t,
            Self::CubicOut => 1.0 - (1.0 - t).powi(3),
            Self::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0f32).mul_add(t, 2.0).powi(3) / 2.0
                }
            }
            Self::SineIn => 1.0 - (t * FRAC_PI_2).cos(),
            Self::SineOut => (t * FRAC_PI_2).sin(),
            Self::SineInOut => -((PI * t).cos() - 1.0) / 2.0,
            Self::ExpoIn => {
                if t == 0.0 {
                    0.0
                } else {
                    10.0f32.mul_add(t, -10.0).exp2()
                }
            }
            Self::ExpoOut => {
                if (t - 1.0).abs() < f32::EPSILON {
                    1.0
                } else {
                    1.0 - (-10.0 * t).exp2()
                }
            }
            Self::BackOut => back_out(t),
            Self::BackInOut => back_in_out(t),
            Self::BounceOut => bounce_out(t),
            Self::Bezier(curve) => curve.apply(t),
            Self::Steps { count, start } => {
                let count = f32::from(count.get());
                ((t * count).floor() + f32::from(u8::from(start))).min(count) / count
            }
        }
    }
}

const BACK_C1: f32 = 1.701_58;
const BACK_C3: f32 = BACK_C1 + 1.0;

fn back_out(t: f32) -> f32 {
    let t_minus = t - 1.0;
    BACK_C1.mul_add(t_minus.powi(2), BACK_C3.mul_add(t_minus.powi(3), 1.0))
}

fn back_in_out(t: f32) -> f32 {
    const C2: f32 = BACK_C1 * 1.525;
    if t < 0.5 {
        let two_t = 2.0 * t;
        two_t.powi(2) * (C2 + 1.0).mul_add(two_t, -C2) / 2.0
    } else {
        let inner = 2.0f32.mul_add(t, -2.0);
        f32::midpoint(inner.powi(2) * (C2 + 1.0).mul_add(inner, C2), 2.0)
    }
}

fn bounce_out(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;
    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        (N1 * t).mul_add(t, 0.75)
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        (N1 * t).mul_add(t, 0.9375)
    } else {
        let t = t - 2.625 / D1;
        (N1 * t).mul_add(t, 0.984_375)
    }
}

#[cfg(test)]
#[path = "easing_tests.rs"]
mod tests;
