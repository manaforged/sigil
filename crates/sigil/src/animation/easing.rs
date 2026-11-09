#![allow(clippy::suboptimal_flops)]

use core::f32::consts::{FRAC_PI_2, PI};
use std::hash::{Hash, Hasher};
use std::num::NonZeroU16;

/// Tween easing curve.
#[derive(Clone, Copy, Debug, Default)]
pub enum Easing {
    /// The linear easing.
    #[default]
    Linear,
    /// The quad in easing.
    QuadIn,
    /// The quad out easing.
    QuadOut,
    /// The quad in out easing.
    QuadInOut,
    /// The cubic in easing.
    CubicIn,
    /// The cubic out easing.
    CubicOut,
    /// The cubic in out easing.
    CubicInOut,
    /// The sine in easing.
    SineIn,
    /// The sine out easing.
    SineOut,
    /// The sine in out easing.
    SineInOut,
    /// The expo in easing.
    ExpoIn,
    /// The expo out easing.
    ExpoOut,
    /// The back out easing.
    BackOut,
    /// The back in out easing.
    BackInOut,
    /// The bounce out easing.
    BounceOut,
    /// The steps easing.
    Steps {
        /// The count.
        count: NonZeroU16,
        /// Whether start.
        start: bool,
    },
    /// The CSS `cubic-bezier(x1, y1, x2, y2)` easing.
    CubicBezier {
        /// The first control point x, in 0..=1.
        x1: f32,
        /// The first control point y.
        y1: f32,
        /// The second control point x, in 0..=1.
        x2: f32,
        /// The second control point y.
        y2: f32,
    },
}

impl PartialEq for Easing {
    fn eq(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
}

impl Eq for Easing {}

impl Hash for Easing {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key().hash(state);
    }
}

impl Easing {
    /// The Tailwind `ease-in` curve, `cubic-bezier(0.4, 0, 1, 1)`.
    pub const IN: Self = Self::cubic_bezier(0.4, 0.0, 1.0, 1.0);
    /// The Tailwind `ease-out` curve, `cubic-bezier(0, 0, 0.2, 1)`.
    pub const OUT: Self = Self::cubic_bezier(0.0, 0.0, 0.2, 1.0);
    /// The Tailwind `ease-in-out` curve, `cubic-bezier(0.4, 0, 0.2, 1)`.
    pub const IN_OUT: Self = Self::cubic_bezier(0.4, 0.0, 0.2, 1.0);

    /// Creates the CSS `cubic-bezier(x1, y1, x2, y2)` easing; `x1` and `x2` are clamped to 0..=1, and a non-finite point gives `Linear`.
    #[must_use]
    pub const fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        if !(x1.is_finite() && y1.is_finite() && x2.is_finite() && y2.is_finite()) {
            return Self::Linear;
        }
        Self::CubicBezier {
            x1: x1.clamp(0.0, 1.0),
            y1,
            x2: x2.clamp(0.0, 1.0),
            y2,
        }
    }

    fn key(self) -> (u8, [u32; 4]) {
        match self {
            Self::Steps { count, start } => (1, [u32::from(count.get()), u32::from(start), 0, 0]),
            Self::CubicBezier { x1, y1, x2, y2 } => {
                (2, [x1, y1, x2, y2].map(|value| (value + 0.0).to_bits()))
            }
            other => (0, [other.ordinal(), 0, 0, 0]),
        }
    }

    const fn ordinal(self) -> u32 {
        match self {
            Self::Linear => 0,
            Self::QuadIn => 1,
            Self::QuadOut => 2,
            Self::QuadInOut => 3,
            Self::CubicIn => 4,
            Self::CubicOut => 5,
            Self::CubicInOut => 6,
            Self::SineIn => 7,
            Self::SineOut => 8,
            Self::SineInOut => 9,
            Self::ExpoIn => 10,
            Self::ExpoOut => 11,
            Self::BackOut => 12,
            Self::BackInOut => 13,
            Self::BounceOut => 14,
            Self::Steps { .. } | Self::CubicBezier { .. } => 15,
        }
    }

    /// Creates a step easing with `count` steps, jumping at the start of each step when `start` is true.
    #[must_use]
    pub const fn steps(count: NonZeroU16, start: bool) -> Self {
        Self::Steps { count, start }
    }

    /// Maps progress `t` through this easing curve.
    #[must_use]
    pub fn apply(self, t: f32) -> f32 {
        let t = unit(t);
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
            Self::Steps { count, start } => {
                let count = f32::from(count.get());
                ((t * count).floor() + f32::from(u8::from(start))).min(count) / count
            }
            Self::CubicBezier { x1, y1, x2, y2 } => cubic_bezier(t, [x1, y1, x2, y2]),
        }
    }
}

#[allow(
    clippy::manual_clamp,
    reason = "max then min maps NaN to 0.0, where clamp keeps NaN"
)]
fn unit(t: f32) -> f32 {
    t.max(0.0).min(1.0)
}

fn cubic_bezier(x: f32, points: [f32; 4]) -> f32 {
    let [x1, y1, x2, y2] = points;
    if x <= 0.0 || x >= 1.0 || !points.iter().all(|point| point.is_finite()) {
        return x;
    }
    let (mut low, mut high) = (0.0_f32, 1.0_f32);
    for _ in 0..32 {
        let t = f32::midpoint(low, high);
        if bezier_coordinate(t, x1, x2) < x {
            low = t;
        } else {
            high = t;
        }
    }
    bezier_coordinate(f32::midpoint(low, high), y1, y2)
}

fn bezier_coordinate(t: f32, first: f32, second: f32) -> f32 {
    let inverse = 1.0 - t;
    (3.0 * inverse * inverse * t).mul_add(first, (3.0 * inverse * t * t).mul_add(second, t * t * t))
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
