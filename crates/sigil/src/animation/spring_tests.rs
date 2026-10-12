use super::*;

fn toward(preset: SpringPreset, target: f32) -> Spring {
    let mut spring = Spring::from_preset(0.0, preset);
    spring.set_target(target);
    spring
}

fn frames_to_settle(spring: &mut Spring, limit: u32) -> u32 {
    let mut frames = 0;
    while !spring.is_settled() && frames < limit {
        spring.step(1.0 / 60.0);
        frames += 1;
    }
    frames
}

#[test]
fn smooth_spring_settles_within_two_seconds() {
    let mut spring = toward(SpringPreset::Smooth, 1.0);
    let frames = frames_to_settle(&mut spring, 240);
    assert!(
        spring.is_settled(),
        "frames={frames}, value={}",
        spring.value()
    );
    assert!((spring.value() - 1.0).abs() < Spring::DEFAULT_VALUE_EPSILON);
}

#[test]
fn bouncy_spring_overshoots() {
    let mut spring = toward(SpringPreset::Bouncy, 1.0);
    let max = (0..240)
        .map(|_| {
            spring.step(1.0 / 60.0);
            spring.value()
        })
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(max > 1.0, "Bouncy spring did not overshoot 1.0 (max={max})");
}

#[test]
fn stiff_spring_settles_fast() {
    let frames = frames_to_settle(&mut toward(SpringPreset::Stiff, 0.1), 60);
    assert!(frames < 30, "Stiff spring took {frames} frames (>0.5 s)");
}

#[test]
fn stiff_spring_does_not_diverge_on_long_dt() {
    let mut spring = toward(SpringPreset::Stiff, 1.0);
    for _ in 0..10 {
        spring.step(0.2);
        assert!(spring.value().is_finite() && spring.value().abs() < 10.0);
    }
}

#[test]
fn step_rejects_non_finite_or_non_positive_dt() {
    let mut spring = toward(SpringPreset::Smooth, 1.0);
    let before = (spring.value(), spring.velocity());
    for dt in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
        spring.step(dt);
        assert_eq!((spring.value(), spring.velocity()), before);
    }
}

#[test]
fn reset_clears_motion() {
    let mut spring = toward(SpringPreset::Smooth, 1.0);
    for _ in 0..5 {
        spring.step(1.0 / 60.0);
    }
    assert!(spring.velocity().abs() > 0.0);
    spring.reset(5.0);
    assert!((spring.value() - 5.0).abs() < f32::EPSILON);
    assert!(spring.velocity().abs() < f32::EPSILON);
    assert!((spring.target() - 5.0).abs() < f32::EPSILON);
    assert!(spring.is_settled());
}

#[test]
fn step_is_deterministic() {
    let run = || {
        let mut spring = toward(SpringPreset::Smooth, 1.0);
        (0..30)
            .map(|_| {
                spring.step(1.0 / 60.0);
                spring.value().to_bits()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

#[test]
#[should_panic(expected = "Spring mass must be positive")]
fn zero_mass_panics() {
    let _ = Spring::new(0.0, 100.0, 10.0, 0.0);
}

#[test]
fn with_epsilons_changes_settle_time() {
    let tight = frames_to_settle(&mut toward(SpringPreset::Smooth, 1.0), 600);
    let mut loose = toward(SpringPreset::Smooth, 1.0).with_epsilons(0.05, 0.05);
    let loose = frames_to_settle(&mut loose, 600);
    assert!(loose < tight, "tight={tight}, loose={loose}");
}

#[test]
#[should_panic(expected = "Spring epsilons must be positive")]
fn zero_epsilon_panics() {
    let _ = Spring::from_preset(0.0, SpringPreset::Smooth).with_epsilons(0.0, 0.001);
}
