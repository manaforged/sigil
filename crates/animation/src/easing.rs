//! Easing curves — bend a normalised `t ∈ [0, 1]` into an output
//! progress used by [`crate::Tween`] and any time-based animator.
//!
//! Curve catalogue follows Robert Penner's classic naming. Every curve
//! satisfies `apply(0.0) == 0.0` and `apply(1.0) == 1.0`. Inputs outside
//! `[0, 1]` are clamped to the endpoints — animations that overshoot do
//! so via [`Easing::BackOut`] / [`Easing::BackInOut`], not by feeding
//! out-of-range `t`.

// The Penner curves read more clearly in their textbook algebraic form
// than rewritten through `mul_add`/`exp2`. The numerical-precision win
// nursery cites is negligible at f32 with `t ∈ [0, 1]`, and FMA-style
// rewrites would obscure which constants come from which paper. Allow
// only here; fmas are still required in spring/tween hot paths.
#![allow(clippy::suboptimal_flops)]

use core::f32::consts::{FRAC_PI_2, PI};

/// Named easing curve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Easing {
    /// `t` — straight line.
    #[default]
    Linear,
    /// Slow start, fast finish — quadratic.
    QuadIn,
    /// Fast start, slow finish — quadratic.
    QuadOut,
    /// Slow start, fast middle, slow finish — quadratic.
    QuadInOut,
    /// Cubic accelerate.
    CubicIn,
    /// Cubic decelerate.
    CubicOut,
    /// Cubic ease-in then -out.
    CubicInOut,
    /// Sinusoidal accelerate.
    SineIn,
    /// Sinusoidal decelerate.
    SineOut,
    /// Sinusoidal ease-in then -out.
    SineInOut,
    /// Exponential accelerate.
    ExpoIn,
    /// Exponential decelerate.
    ExpoOut,
    /// Slight pull-back before springing forward — overshoots above 1.0
    /// mid-curve, settles at 1.0.
    BackOut,
    /// Back-style in and out — overshoots both endpoints mid-curve.
    BackInOut,
    /// Bouncing-ball decelerate.
    BounceOut,
}

impl Easing {
    /// Apply this curve to a normalised progress value. Inputs outside
    /// `[0, 1]` are clamped first so animations never read garbage past
    /// their endpoints.
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
