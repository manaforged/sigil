use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Duration;

use crate::animation_runtime::{active_count, tick};
use crate::prelude::*;

fn frames() -> Keyframes {
    Keyframes::new(
        [Keyframe::new(0.0, 0.0), Keyframe::new(1.0, 100.0)],
        Duration::from_secs(1),
    )
}

#[test]
fn paused_playback_holds_its_value_and_resumes_from_that_position() {
    Runtime::scope("playback-pause", || {
        let value = signal(0.0);
        let mut playback = Playback::new(value, frames());
        tick(0.25);
        assert_eq!(value.peek().to_bits(), 25.0_f32.to_bits());
        playback.pause();
        tick(0.5);
        assert_eq!(value.peek().to_bits(), 25.0_f32.to_bits());
        assert_eq!(active_count(), 0);
        playback.resume();
        tick(0.25);
        assert_eq!(value.peek().to_bits(), 50.0_f32.to_bits());
    });
}

#[test]
fn seeking_across_delay_and_alternate_reverse_cycles_preserves_the_endpoint() {
    Runtime::scope("playback-direction", || {
        let value = signal(7.0);
        let mut playback = Playback::new(
            value,
            frames()
                .delay(Duration::from_millis(500))
                .repeat(NonZeroU32::new(2).expect("two iterations"))
                .direction(AnimationDirection::AlternateReverse)
                .fill(AnimationFill::Both),
        );
        assert_eq!(value.peek().to_bits(), 100.0_f32.to_bits());
        playback.seek(Duration::from_millis(750));
        assert_eq!(value.peek().to_bits(), 75.0_f32.to_bits());
        playback.seek(Duration::from_millis(1750));
        assert_eq!(value.peek().to_bits(), 25.0_f32.to_bits());
        playback.seek(Duration::from_millis(2500));
        assert_eq!(value.peek().to_bits(), 100.0_f32.to_bits());
        assert!(playback.finished());
        assert_eq!(active_count(), 0);
    });
}

#[test]
fn no_fill_restores_the_original_value_after_playback_finishes() {
    Runtime::scope("playback-no-fill", || {
        let value = signal(7.0);
        let playback = Playback::new(
            value,
            frames()
                .delay(Duration::from_millis(500))
                .fill(AnimationFill::None),
        );
        assert_eq!(value.peek().to_bits(), 7.0_f32.to_bits());
        tick(0.75);
        assert_eq!(value.peek().to_bits(), 25.0_f32.to_bits());
        tick(0.75);
        assert_eq!(value.peek().to_bits(), 7.0_f32.to_bits());
        assert!(playback.finished());
        assert_eq!(active_count(), 0);
    });
}

#[test]
fn cancellation_restores_the_original_value_until_explicitly_restarted() {
    Runtime::scope("playback-cancel", || {
        let value = signal(7.0);
        let mut playback = Playback::new(value, frames().repeat_forever());
        tick(0.25);
        playback.cancel();
        tick(1.0);
        assert_eq!(value.peek().to_bits(), 7.0_f32.to_bits());
        assert!(playback.finished());
        assert_eq!(active_count(), 0);
        playback.restart();
        tick(0.25);
        assert_eq!(value.peek().to_bits(), 25.0_f32.to_bits());
        assert!(!playback.finished());
    });
}

#[test]
fn dropping_playback_stops_updates_to_the_bound_value() {
    Runtime::scope("playback-drop", || {
        let value = signal(0.0);
        let playback = Playback::new(value, frames().repeat_forever());
        tick(0.25);
        drop(playback);
        tick(0.25);
        assert_eq!(value.peek().to_bits(), 25.0_f32.to_bits());
        assert_eq!(active_count(), 0);
    });
}

#[test]
fn reduced_motion_finishes_even_a_paused_animation() {
    Runtime::scope("playback-reduced-motion", || {
        let value = signal(0.0);
        let mut playback = Playback::new(value, frames());
        playback.pause();
        MotionPrefs::set(true);
        assert_eq!(value.peek().to_bits(), 100.0_f32.to_bits());
        assert!(playback.finished());
        assert_eq!(active_count(), 0);
        MotionPrefs::set(false);
    });
}

#[test]
fn effects_observe_a_complete_animation_frame_without_borrowing_the_registry() {
    Runtime::scope("playback-batch", || {
        let x = signal(0.0);
        let y = signal(0.0);
        let _x = Playback::new(x, frames());
        let _y = Playback::new(y, frames());
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&seen);
        effect(move || {
            sink.borrow_mut().push((x.get(), y.get(), active_count()));
        });
        seen.borrow_mut().clear();
        tick(0.25);
        assert_eq!(&*seen.borrow(), &[(25.0, 25.0, 2)]);
    });
}

#[test]
fn playback_interpolates_each_keyframe_segment_in_its_own_time_range() {
    Runtime::scope("playback-segments", || {
        let value = signal(0.0);
        let frames = Keyframes::new(
            [
                Keyframe::new(0.0, 0.0),
                Keyframe::new(0.25, 80.0),
                Keyframe::new(1.0, 100.0),
            ],
            Duration::from_secs(1),
        );
        let mut playback = Playback::new(value, frames);
        playback.seek(Duration::from_millis(125));
        assert_eq!(value.peek().to_bits(), 40.0_f32.to_bits());
        playback.seek(Duration::from_millis(625));
        assert_eq!(value.peek().to_bits(), 90.0_f32.to_bits());
    });
}

#[test]
fn reverse_playback_runs_backwards_on_every_iteration() {
    Runtime::scope("playback-reverse", || {
        let value = signal(7.0);
        let mut playback = Playback::new(
            value,
            frames()
                .repeat(NonZeroU32::new(2).expect("two iterations"))
                .direction(AnimationDirection::Reverse),
        );
        for elapsed in [250, 1250] {
            playback.seek(Duration::from_millis(elapsed));
            assert_eq!(value.peek().to_bits(), 75.0_f32.to_bits());
        }
        playback.finish();
        assert_eq!(value.peek().to_bits(), 0.0_f32.to_bits());
        assert!(playback.finished());
    });
}

#[test]
fn alternate_playback_returns_to_its_start_after_two_iterations() {
    Runtime::scope("playback-alternate", || {
        let value = signal(7.0);
        let mut playback = Playback::new(
            value,
            frames()
                .repeat(NonZeroU32::new(2).expect("two iterations"))
                .direction(AnimationDirection::Alternate),
        );
        playback.seek(Duration::from_millis(250));
        assert_eq!(value.peek().to_bits(), 25.0_f32.to_bits());
        playback.seek(Duration::from_millis(1250));
        assert_eq!(value.peek().to_bits(), 75.0_f32.to_bits());
        playback.seek(Duration::from_secs(2));
        assert_eq!(value.peek().to_bits(), 0.0_f32.to_bits());
        assert!(playback.finished());
    });
}

#[test]
fn backwards_fill_applies_during_delay_but_restores_the_value_at_completion() {
    Runtime::scope("playback-backwards-fill", || {
        let value = signal(7.0);
        let mut playback = Playback::new(
            value,
            frames()
                .delay(Duration::from_millis(500))
                .fill(AnimationFill::Backwards),
        );
        assert_eq!(value.peek().to_bits(), 0.0_f32.to_bits());
        playback.seek(Duration::from_millis(750));
        assert_eq!(value.peek().to_bits(), 25.0_f32.to_bits());
        playback.finish();
        assert_eq!(value.peek().to_bits(), 7.0_f32.to_bits());
    });
}

#[test]
fn forwards_fill_preserves_the_initial_value_during_delay_and_holds_the_end() {
    Runtime::scope("playback-forwards-fill", || {
        let value = signal(7.0);
        let mut playback = Playback::new(value, frames().delay(Duration::from_millis(500)));
        assert_eq!(value.peek().to_bits(), 7.0_f32.to_bits());
        playback.seek(Duration::from_millis(750));
        assert_eq!(value.peek().to_bits(), 25.0_f32.to_bits());
        playback.finish();
        assert_eq!(value.peek().to_bits(), 100.0_f32.to_bits());
    });
}

#[test]
fn a_completed_frame_does_not_schedule_more_paints_or_publish_more_values() {
    Runtime::scope("playback-idle", || {
        let value = signal(0.0);
        let playback = Playback::new(value, frames());
        let paints = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&paints);
        effect(move || sink.borrow_mut().push(value.get()));
        paints.borrow_mut().clear();
        tick(1.0);
        assert!(playback.finished());
        assert_eq!(&*paints.borrow(), &[100.0]);
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        Runtime::end_frame();
        for _ in 0..120 {
            tick(1.0 / 60.0);
        }
        assert!(!Runtime::frame_dirty());
        assert_eq!(&*paints.borrow(), &[100.0]);
        assert_eq!(active_count(), 0);
    });
}
