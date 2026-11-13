use super::*;

fn toward(spring: Spring, target: f32) -> SpringState {
    let mut state = SpringState::new(0.0, spring);
    state.set_target(target);
    state
}

fn frames_to_settle(spring: &mut SpringState, limit: u32) -> u32 {
    let mut frames = 0;
    while !spring.is_settled() && frames < limit {
        spring.step(FRAME_SECS);
        frames += 1;
    }
    frames
}

#[test]
fn smooth_spring_settles_within_two_seconds() {
    let mut spring = toward(Spring::SMOOTH, 1.0);
    let frames = frames_to_settle(&mut spring, 240);
    assert!(
        spring.is_settled(),
        "frames={frames}, value={}",
        spring.value()
    );
    assert!((spring.value() - 1.0).abs() < SpringState::DEFAULT_VALUE_EPSILON);
}

#[test]
fn bouncy_spring_overshoots() {
    let mut spring = toward(Spring::BOUNCY, 1.0);
    let max = (0..240)
        .map(|_| {
            spring.step(FRAME_SECS);
            spring.value()
        })
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(max > 1.0, "Bouncy spring did not overshoot 1.0 (max={max})");
}

#[test]
fn stiff_spring_settles_fast() {
    let frames = frames_to_settle(&mut toward(Spring::STIFF, 0.1), 60);
    assert!(frames < 30, "Stiff spring took {frames} frames (>0.5 s)");
}

#[test]
fn stiff_spring_does_not_diverge_on_long_dt() {
    let mut spring = toward(Spring::STIFF, 1.0);
    for _ in 0..10 {
        spring.step(0.2);
        assert!(spring.value().is_finite() && spring.value().abs() < 10.0);
    }
}

#[test]
fn step_rejects_non_finite_or_non_positive_dt() {
    let mut spring = toward(Spring::SMOOTH, 1.0);
    let before = (spring.value(), spring.velocity());
    for dt in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
        spring.step(dt);
        assert_eq!((spring.value(), spring.velocity()), before);
    }
}

#[test]
fn reset_clears_motion() {
    let mut spring = toward(Spring::SMOOTH, 1.0);
    for _ in 0..5 {
        spring.step(FRAME_SECS);
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
        let mut spring = toward(Spring::SMOOTH, 1.0);
        (0..30)
            .map(|_| {
                spring.step(FRAME_SECS);
                spring.value().to_bits()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

#[test]
fn with_epsilons_changes_settle_time() {
    let tight = frames_to_settle(&mut toward(Spring::SMOOTH, 1.0), SETTLE_FRAMES);
    let mut loose = toward(Spring::SMOOTH, 1.0).with_epsilons(0.05, 0.05);
    let loose = frames_to_settle(&mut loose, SETTLE_FRAMES);
    assert!(loose < tight, "tight={tight}, loose={loose}");
}

#[test]
#[should_panic(expected = "Spring epsilons must be positive")]
fn zero_epsilon_panics() {
    let _ = SpringState::new(0.0, Spring::SMOOTH).with_epsilons(0.0, 0.001);
}

const MAX_SWING: f32 = 2.0;

const PARAMETERS: [f32; 8] = [
    f32::NAN,
    f32::INFINITY,
    f32::NEG_INFINITY,
    -1.0,
    0.0,
    1e-6,
    1e6,
    f32::MAX,
];

fn assert_settles_finitely(spring: Spring) {
    let mut state = toward(spring, 1.0);
    for frame in 1..=SETTLE_FRAMES {
        state.advance(FRAME_SECS);
        assert!(
            state.value().is_finite() && state.velocity().is_finite(),
            "{spring:?} went non-finite at frame {frame}"
        );
        assert!(
            state.value().abs() < MAX_SWING,
            "{spring:?} swung to {} at frame {frame}",
            state.value()
        );
        if state.is_settled() {
            assert!(
                (state.value() - 1.0).abs() < f32::EPSILON,
                "{spring:?} settled at {}",
                state.value()
            );
            return;
        }
    }
    panic!("{spring:?} did not settle within {SETTLE_FRAMES} frames");
}

#[test]
fn light_overdamped_and_undamped_springs_stay_finite_and_settle() {
    for spring in [
        Spring::SMOOTH.mass(0.05),
        Spring::new(100.0, 300.0),
        Spring::new(180.0, 0.0),
    ] {
        assert_settles_finitely(spring);
    }
}

#[test]
fn any_parameters_stay_finite_and_settle() {
    for stiffness in PARAMETERS {
        for damping in PARAMETERS {
            for mass in PARAMETERS {
                let spring = Spring::new(stiffness, damping).mass(mass);
                assert_eq!(spring, Spring::new(stiffness, damping).mass(mass));
                assert_settles_finitely(spring);
            }
        }
    }
}

#[test]
fn presets_keep_their_parameters_and_step() {
    for spring in [
        Spring::BOUNCY,
        Spring::GENTLE,
        Spring::SMOOTH,
        Spring::SNAPPY,
        Spring::STIFF,
    ] {
        let state = SpringState::new(0.0, spring);
        let (stiffness, damping, mass) = spring.parameters();
        assert_eq!(
            [state.stiffness, state.damping, state.mass].map(f32::to_bits),
            [stiffness, damping, mass].map(f32::to_bits),
            "{spring:?}"
        );
        assert_eq!(
            state.substep.to_bits(),
            SpringState::MAX_SUBSTEP_DT.to_bits(),
            "{spring:?}"
        );
        assert_settles_finitely(spring);
    }
}
