use std::cell::RefCell;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Duration;

use crate::animation::{AnimationDirection, AnimationFill, Keyframe, Keyframes};
use crate::animation_runtime::{MotionPrefs, Playback, active_count, tick};
use crate::prelude::*;
use crate::reactive::Runtime;

fn frames() -> Keyframes {
    Keyframes::new(
        [Keyframe::new(0.0, 0.0), Keyframe::new(1.0, 100.0)],
        Duration::from_secs(1),
    )
}

fn twice(direction: AnimationDirection) -> Keyframes {
    frames()
        .repeat(NonZeroU32::new(2).expect("two iterations"))
        .direction(direction)
}

#[track_caller]
fn assert_at(value: Signal<f32>, expected: f32) {
    assert_eq!(value.peek().to_bits(), expected.to_bits());
}

fn seek_ms(playback: &mut Playback, ms: u64) {
    playback.seek(Duration::from_millis(ms));
}

#[test]
fn paused_playback_holds_its_value_and_resumes_from_that_position() {
    Runtime::scope("playback-pause", || {
        let value = signal(0.0);
        let mut playback = Playback::new(value, frames());
        tick(0.25);
        assert_at(value, 25.0);
        playback.pause();
        tick(0.5);
        assert_at(value, 25.0);
        assert_eq!(active_count(), 0);
        playback.resume();
        tick(0.25);
        assert_at(value, 50.0);
    });
}

#[test]
fn seeking_across_delay_and_alternate_reverse_cycles_preserves_the_endpoint() {
    Runtime::scope("playback-direction", || {
        let value = signal(7.0);
        let frames = twice(AnimationDirection::AlternateReverse)
            .delay(Duration::from_millis(500))
            .fill(AnimationFill::Both);
        let mut playback = Playback::new(value, frames);
        assert_at(value, 100.0);
        for (ms, expected) in [(750, 75.0), (1750, 25.0), (2500, 100.0)] {
            seek_ms(&mut playback, ms);
            assert_at(value, expected);
        }
        assert!(playback.finished());
        assert_eq!(active_count(), 0);
    });
}

#[test]
fn no_fill_restores_the_original_value_after_playback_finishes() {
    Runtime::scope("playback-no-fill", || {
        let value = signal(7.0);
        let frames = frames()
            .delay(Duration::from_millis(500))
            .fill(AnimationFill::None);
        let playback = Playback::new(value, frames);
        assert_at(value, 7.0);
        tick(0.75);
        assert_at(value, 25.0);
        tick(0.75);
        assert_at(value, 7.0);
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
        assert_at(value, 7.0);
        assert!(playback.finished());
        assert_eq!(active_count(), 0);
        playback.restart();
        tick(0.25);
        assert_at(value, 25.0);
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
        assert_at(value, 25.0);
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
        assert_at(value, 100.0);
        assert!(playback.finished());
        assert_eq!(active_count(), 0);
        MotionPrefs::set(false);
    });
}

#[test]
fn effects_observe_a_complete_animation_frame_without_borrowing_the_registry() {
    Runtime::scope("playback-batch", || {
        let (x, y) = (signal(0.0), signal(0.0));
        let _x = Playback::new(x, frames());
        let _y = Playback::new(y, frames());
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&seen);
        effect(move || sink.borrow_mut().push((x.get(), y.get(), active_count())));
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
        seek_ms(&mut playback, 125);
        assert_at(value, 40.0);
        seek_ms(&mut playback, 625);
        assert_at(value, 90.0);
    });
}

#[test]
fn reverse_playback_runs_backwards_on_every_iteration() {
    Runtime::scope("playback-reverse", || {
        let value = signal(7.0);
        let mut playback = Playback::new(value, twice(AnimationDirection::Reverse));
        for ms in [250, 1250] {
            seek_ms(&mut playback, ms);
            assert_at(value, 75.0);
        }
        playback.finish();
        assert_at(value, 0.0);
        assert!(playback.finished());
    });
}

#[test]
fn alternate_playback_returns_to_its_start_after_two_iterations() {
    Runtime::scope("playback-alternate", || {
        let value = signal(7.0);
        let mut playback = Playback::new(value, twice(AnimationDirection::Alternate));
        for (ms, expected) in [(250, 25.0), (1250, 75.0), (2000, 0.0)] {
            seek_ms(&mut playback, ms);
            assert_at(value, expected);
        }
        assert!(playback.finished());
    });
}

#[test]
fn fill_mode_decides_the_value_during_delay_and_after_finish() {
    for (fill, during_delay, after) in [
        (AnimationFill::Backwards, 0.0, 7.0),
        (AnimationFill::Forwards, 7.0, 100.0),
    ] {
        Runtime::scope("playback-fill", || {
            let value = signal(7.0);
            let frames = frames().delay(Duration::from_millis(500)).fill(fill);
            let mut playback = Playback::new(value, frames);
            assert_at(value, during_delay);
            seek_ms(&mut playback, 750);
            assert_at(value, 25.0);
            playback.finish();
            assert_at(value, after);
        });
    }
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
