use super::*;

#[test]
fn endpoints_are_exact() {
    let tween = Tween::new(2.0, 7.0, 0.5);
    assert!((tween.value() - 2.0).abs() < f32::EPSILON);

    let mut done = tween;
    done.step(0.5);
    assert!(done.is_finished());
    assert!((done.value() - 7.0).abs() < f32::EPSILON);
}

#[test]
fn midpoint_under_linear_is_exact_midpoint() {
    let mut tween = Tween::new(0.0, 10.0, 1.0);
    tween.step(0.5);
    assert!((tween.value() - 5.0).abs() < 1e-5);
}

#[test]
fn cubic_in_out_at_midpoint_is_half() {
    let mut tween = Tween::new(0.0, 1.0, 1.0).with_easing(Easing::CubicInOut);
    tween.step(0.5);
    assert!((tween.value() - 0.5).abs() < 1e-5);
}

#[test]
fn step_past_duration_clamps() {
    let mut tween = Tween::new(0.0, 1.0, 0.5);
    tween.step(10.0);
    assert!(tween.is_finished());
    assert!((tween.value() - 1.0).abs() < f32::EPSILON);
    assert!((tween.progress() - 1.0).abs() < f32::EPSILON);

    // Subsequent steps are no-ops, not negative-time slides.
    tween.step(10.0);
    assert!((tween.value() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn restart_resets_elapsed() {
    let mut tween = Tween::new(0.0, 1.0, 1.0);
    tween.step(0.8);
    assert!(tween.value() > 0.5);
    tween.restart();
    assert!((tween.value() - 0.0).abs() < f32::EPSILON);
    assert!(!tween.is_finished());
}

#[test]
fn step_is_deterministic() {
    let run = || {
        let mut tween = Tween::new(0.0, 1.0, 0.5).with_easing(Easing::CubicInOut);
        let mut samples = Vec::new();
        for _ in 0..30 {
            tween.step(1.0 / 60.0);
            samples.push(tween.value());
        }
        samples
    };
    let a = run();
    let b = run();
    for (sa, sb) in a.iter().zip(&b) {
        assert!((sa - sb).abs() < f32::EPSILON);
    }
}

#[test]
#[should_panic(expected = "Tween duration must be positive")]
fn zero_duration_panics() {
    let _ = Tween::new(0.0, 1.0, 0.0);
}

#[test]
#[should_panic(expected = "Tween duration must be positive")]
fn negative_duration_panics() {
    let _ = Tween::new(0.0, 1.0, -0.5);
}
