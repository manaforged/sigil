use crate::animation::SpringPreset;
use crate::reactive::{Runtime, signal};

use super::{active_count, tick};
use crate::animation_runtime::animator::Animator;
use crate::animation_runtime::config::AnimationConfig;
use crate::animation_runtime::motion::MotionPrefs;
use crate::animation_runtime::{clock, running_clocks, tick_clocks};

fn spring(preset: SpringPreset) -> AnimationConfig {
    AnimationConfig::Spring { preset }
}

#[test]
fn dropping_animator_deregisters_from_runtime() {
    Runtime::scope("dropping_animator_deregisters_from_runtime", || {
        MotionPrefs::set(false);
        let baseline = active_count();
        let mut animator = Animator::new(signal(0.0_f32), spring(SpringPreset::Smooth));
        animator.target(1.0);
        assert_eq!(active_count(), baseline + 1);
        drop(animator);
        assert_eq!(active_count(), baseline);
    });
}

#[test]
fn reduced_motion_skips_animator_registration() {
    Runtime::scope("reduced_motion_skips_animator_registration", || {
        MotionPrefs::set(true);
        assert!(MotionPrefs::reduced());
        let value = signal(0.0_f32);
        let baseline = active_count();
        let mut animator = Animator::new(value, spring(SpringPreset::Smooth));
        assert_eq!(active_count(), baseline);
        assert!(animator.settled());
        animator.target(1.0);
        assert!((value.peek() - 1.0).abs() < f32::EPSILON);
        assert_eq!(active_count(), baseline);
        MotionPrefs::set(false);
    });
}

#[test]
fn settled_animator_becomes_idle_and_can_retarget() {
    Runtime::scope("settled_animator_becomes_idle_and_can_retarget", || {
        MotionPrefs::set(false);
        let baseline = active_count();
        let mut animator = Animator::new(signal(0.0_f32), spring(SpringPreset::Stiff));
        animator.target(1.0);
        assert_eq!(active_count(), baseline + 1);
        for _ in 0..60 {
            tick(1.0 / 60.0);
            if animator.settled() {
                break;
            }
        }
        assert!(animator.settled());
        assert_eq!(active_count(), baseline, "settled animator stopped ticking");
        animator.target(0.25);
        assert_eq!(
            active_count(),
            baseline + 1,
            "target re-arms a settled animator"
        );
        tick(1.0 / 60.0);
        assert!(animator.value() < 1.0);
        drop(animator);
        assert_eq!(active_count(), baseline);
    });
}

#[test]
fn a_clock_advances_by_each_frame_and_stops_with_its_owner() {
    Runtime::scope(
        "a_clock_advances_by_each_frame_and_stops_with_its_owner",
        || {
            let owner = crate::reactive::Owner::new("clock", None);
            let seconds = owner.with(clock);
            assert_eq!(running_clocks(), 1);
            tick_clocks(0.25);
            tick_clocks(0.5);
            assert!((seconds.get() - 0.75).abs() < 1e-9, "{}", seconds.get());
            owner.dispose();
            assert_eq!(running_clocks(), 0);
        },
    );
}

#[test]
fn a_clock_keeps_frame_time_after_long_uptime() {
    Runtime::scope("a_clock_keeps_frame_time_after_long_uptime", || {
        let owner = crate::reactive::Owner::new("clock", None);
        let seconds = owner.with(clock);
        tick_clocks(600_000.0);
        let before = seconds.get();
        tick_clocks(1.0 / 60.0);
        let step = seconds.get() - before;
        assert!(
            (step - 1.0 / 60.0).abs() < 1e-6,
            "a frame after a week advanced {step}"
        );
        owner.dispose();
    });
}

#[test]
fn a_running_clock_lets_animations_settle() {
    Runtime::scope("a_running_clock_lets_animations_settle", || {
        MotionPrefs::set(false);
        let baseline = active_count();
        let owner = crate::reactive::Owner::new("clock", None);
        let _seconds = owner.with(clock);
        assert_eq!(
            active_count(),
            baseline,
            "a clock counts as an unsettled animation"
        );
        owner.dispose();
    });
}
