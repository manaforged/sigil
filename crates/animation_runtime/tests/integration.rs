#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil_animation::{Easing, SpringPreset};
use sigil_animation_runtime::{AnimationConfig, Animator, MotionPrefs, tick};
use sigil_reactive::{Runtime, signal};

const DT_60HZ: f32 = 1.0 / 60.0;

fn tick_until_settled(animator: &Animator, max_frames: u32) -> u32 {
    let mut frames = 0;
    while !animator.settled() && frames < max_frames {
        tick(DT_60HZ);
        frames += 1;
    }
    frames
}

#[test]
fn spring_animator_settles_within_known_frame_budget() {
    Runtime::scope("spring_animator_settles_within_known_frame_budget", || {
        MotionPrefs::set(false);
        let value = signal(0.0_f32);
        let mut animator = Animator::new(
            value,
            AnimationConfig::Spring {
                preset: SpringPreset::Smooth,
            },
        );
        animator.target(1.0);

        let frames = tick_until_settled(&animator, 240);
        assert!(
            animator.settled(),
            "spring did not settle within 240 frames (~4s @ 60Hz)"
        );
        assert!(
            (value.peek() - 1.0).abs() < 0.005,
            "settled value {} did not reach target 1.0 within epsilon",
            value.peek()
        );
        assert!(
            frames < 240,
            "spring took the full budget — preset may be miscalibrated"
        );
    });
}

#[test]
fn tween_animator_lands_at_target_when_duration_elapses() {
    Runtime::scope(
        "tween_animator_lands_at_target_when_duration_elapses",
        || {
            MotionPrefs::set(false);
            let value = signal(0.0_f32);
            let mut animator = Animator::new(
                value,
                AnimationConfig::Tween {
                    ms: 200,
                    easing: Easing::CubicInOut,
                },
            );
            animator.target(10.0);

            for _ in 0..15 {
                tick(DT_60HZ);
            }

            assert!(animator.settled());
            assert!(
                (value.peek() - 10.0).abs() < f32::EPSILON * 100.0,
                "tween value {} != target 10.0",
                value.peek()
            );
        },
    );
}

#[test]
fn retarget_during_animation_continues_from_current_value() {
    Runtime::scope(
        "retarget_during_animation_continues_from_current_value",
        || {
            MotionPrefs::set(false);
            let value = signal(0.0_f32);
            let mut animator = Animator::new(
                value,
                AnimationConfig::Spring {
                    preset: SpringPreset::Smooth,
                },
            );
            animator.target(1.0);

            for _ in 0..5 {
                tick(DT_60HZ);
            }
            let mid = value.peek();
            assert!(
                mid > 0.0 && mid < 1.0,
                "expected mid-flight value, got {mid}"
            );

            animator.target(0.5);
            tick(DT_60HZ);
            let after_retarget = value.peek();
            assert!(
                (after_retarget - mid).abs() < 0.4,
                "retarget jumped: was {mid}, after one frame is {after_retarget}"
            );

            let _ = tick_until_settled(&animator, 240);
            assert!((value.peek() - 0.5).abs() < 0.01);
        },
    );
}

#[test]
fn tick_with_invalid_dt_is_a_no_op() {
    Runtime::scope("tick_with_invalid_dt_is_a_no_op", || {
        MotionPrefs::set(false);
        let value = signal(0.0_f32);
        let mut animator = Animator::new(
            value,
            AnimationConfig::Spring {
                preset: SpringPreset::Smooth,
            },
        );
        animator.target(1.0);

        let before = value.peek();
        tick(0.0);
        tick(-1.0);
        tick(f32::NAN);
        tick(f32::INFINITY);
        let after = value.peek();
        assert!(
            (before - after).abs() < f32::EPSILON,
            "invalid dt should be a no-op"
        );
    });
}
