#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    reason = "Tests use bounded small integer-to-float casts for sampling and \
              compare floats with explicit epsilon thresholds elsewhere."
)]

use super::*;

/// Every named curve hits the two endpoints exactly. Any drift means
/// either the math is wrong or a clamp got introduced where one
/// shouldn't be.
#[test]
fn endpoints_are_exact() {
    for curve in [
        Easing::Linear,
        Easing::QuadIn,
        Easing::QuadOut,
        Easing::QuadInOut,
        Easing::CubicIn,
        Easing::CubicOut,
        Easing::CubicInOut,
        Easing::SineIn,
        Easing::SineOut,
        Easing::SineInOut,
        Easing::ExpoIn,
        Easing::ExpoOut,
        Easing::BackOut,
        Easing::BackInOut,
        Easing::BounceOut,
    ] {
        let zero = curve.apply(0.0);
        let one = curve.apply(1.0);
        assert!(
            zero.abs() < 1e-4,
            "{curve:?}: apply(0.0) = {zero}, expected ~0.0"
        );
        assert!(
            (one - 1.0).abs() < 1e-4,
            "{curve:?}: apply(1.0) = {one}, expected ~1.0"
        );
    }
}

/// Out-of-range inputs are clamped, not extrapolated.
#[test]
fn out_of_range_inputs_clamp() {
    for curve in [Easing::Linear, Easing::CubicInOut, Easing::BounceOut] {
        assert!((curve.apply(-1.0) - curve.apply(0.0)).abs() < 1e-6);
        assert!((curve.apply(2.0) - curve.apply(1.0)).abs() < 1e-6);
    }
}

/// Linear is the identity.
#[test]
fn linear_is_identity() {
    for i in 0..=10_u8 {
        let t = f32::from(i) / 10.0;
        assert!((Easing::Linear.apply(t) - t).abs() < 1e-6);
    }
}

/// Strictly-monotonic curves should not move backwards.
#[test]
fn monotonic_curves_increase() {
    for curve in [
        Easing::Linear,
        Easing::QuadIn,
        Easing::QuadOut,
        Easing::QuadInOut,
        Easing::CubicIn,
        Easing::CubicOut,
        Easing::CubicInOut,
        Easing::SineIn,
        Easing::SineOut,
        Easing::SineInOut,
    ] {
        let mut prev = curve.apply(0.0);
        for i in 1..=100_u8 {
            let t = f32::from(i) / 100.0;
            let next = curve.apply(t);
            assert!(
                next + 1e-4 >= prev,
                "{curve:?} not monotonic at t={t}: prev={prev} next={next}"
            );
            prev = next;
        }
    }
}
