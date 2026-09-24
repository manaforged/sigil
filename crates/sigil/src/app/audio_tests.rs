#![allow(clippy::unwrap_used, clippy::float_cmp)]

use std::time::Duration;

use crate::reactive::Runtime;

use super::{Player, progress_ratio};

#[test]
fn progress_ratio_is_zero_on_empty_total() {
    assert_eq!(progress_ratio(Duration::from_secs(1), Duration::ZERO), 0.0);
}

#[test]
fn progress_ratio_clamps_at_one() {
    assert_eq!(
        progress_ratio(Duration::from_secs(3), Duration::from_secs(2)),
        1.0
    );
}

#[test]
fn progress_ratio_half() {
    assert!((progress_ratio(Duration::from_secs(1), Duration::from_secs(2)) - 0.5).abs() < 0.001);
}

#[test]
fn tick_without_a_sink_leaves_progress() {
    Runtime::scope("audio-tick", || {
        let player = Player::new();
        player.tick();
        assert_eq!(player.progress.peek(), 0.0);
        assert!(!player.playing.peek());
    });
}

#[test]
fn toggle_without_a_sink_leaves_playing() {
    Runtime::scope("audio-toggle", || {
        let player = Player::new();
        player.toggle();
        assert!(!player.playing.peek());
    });
}

#[test]
fn seek_without_sink_errors() {
    Runtime::scope("audio-seek-idle", || {
        let player = Player::new();
        let err = player.seek(0.5).unwrap_err();
        assert!(
            err.to_string().contains("play_path"),
            "expected play_path error, got {err}"
        );
    });
}
