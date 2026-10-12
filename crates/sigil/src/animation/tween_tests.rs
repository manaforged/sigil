use super::*;

fn stepped(tween: Tween, dt: f32) -> Tween {
    let mut tween = tween;
    tween.step(dt);
    tween
}

#[test]
fn endpoints_are_exact() {
    let tween = Tween::new(2.0, 7.0, 0.5);
    assert!((tween.value() - 2.0).abs() < f32::EPSILON);
    let done = stepped(tween, 0.5);
    assert!(done.is_finished());
    assert!((done.value() - 7.0).abs() < f32::EPSILON);
}

#[test]
fn midpoints_match_the_easing() {
    assert!((stepped(Tween::new(0.0, 10.0, 1.0), 0.5).value() - 5.0).abs() < 1e-5);
    let cubic = Tween::new(0.0, 1.0, 1.0).with_easing(Easing::CubicInOut);
    assert!((stepped(cubic, 0.5).value() - 0.5).abs() < 1e-5);
}

#[test]
fn step_past_duration_clamps() {
    let mut tween = stepped(Tween::new(0.0, 1.0, 0.5), 10.0);
    assert!(tween.is_finished());
    assert!((tween.value() - 1.0).abs() < f32::EPSILON);
    assert!((tween.progress() - 1.0).abs() < f32::EPSILON);
    tween.step(10.0);
    assert!((tween.value() - 1.0).abs() < f32::EPSILON);
}

#[test]
fn restart_resets_elapsed() {
    let mut tween = stepped(Tween::new(0.0, 1.0, 1.0), 0.8);
    assert!(tween.value() > 0.5);
    tween.restart();
    assert!((tween.value() - 0.0).abs() < f32::EPSILON);
    assert!(!tween.is_finished());
}

#[test]
fn step_is_deterministic() {
    let run = || {
        let mut tween = Tween::new(0.0, 1.0, 0.5).with_easing(Easing::CubicInOut);
        (0..30)
            .map(|_| {
                tween.step(1.0 / 60.0);
                tween.value().to_bits()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
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
