#![allow(clippy::indexing_slicing)]

use super::*;

const MONOTONIC: [Easing; 10] = [
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
];

const OVERSHOOTING: [Easing; 5] = [
    Easing::ExpoIn,
    Easing::ExpoOut,
    Easing::BackOut,
    Easing::BackInOut,
    Easing::BounceOut,
];

#[test]
fn endpoints_are_exact() {
    for curve in MONOTONIC.into_iter().chain(OVERSHOOTING) {
        let (zero, one) = (curve.apply(0.0), curve.apply(1.0));
        assert!(zero.abs() < 1e-4, "{curve:?}: apply(0.0) = {zero}");
        assert!((one - 1.0).abs() < 1e-4, "{curve:?}: apply(1.0) = {one}");
    }
}

#[test]
fn out_of_range_inputs_clamp() {
    for curve in [Easing::Linear, Easing::CubicInOut, Easing::BounceOut] {
        assert!((curve.apply(-1.0) - curve.apply(0.0)).abs() < 1e-6);
        assert!((curve.apply(2.0) - curve.apply(1.0)).abs() < 1e-6);
    }
}

#[test]
fn linear_is_identity() {
    for i in 0..=10_u8 {
        let t = f32::from(i) / 10.0;
        assert!((Easing::Linear.apply(t) - t).abs() < 1e-6);
    }
}

#[test]
fn monotonic_curves_increase() {
    for curve in MONOTONIC {
        let samples: Vec<f32> = (0..=100_u8)
            .map(|i| curve.apply(f32::from(i) / 100.0))
            .collect();
        for pair in samples.windows(2) {
            assert!(
                pair[1] + 1e-4 >= pair[0],
                "{curve:?} not monotonic: {pair:?}"
            );
        }
    }
}
