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

fn silent_wav(millis: u32) -> Vec<u8> {
    let rate: u32 = 8_000;
    let samples = rate * millis / 1_000;
    let data = samples * 2;
    let mut wav = Vec::with_capacity(44 + data as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&(rate * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data.to_le_bytes());
    wav.resize(44 + data as usize, 0);
    wav
}

#[test]
#[ignore = "manual: needs an audio output device"]
fn tracks_play_back_to_back_on_one_device() {
    Runtime::scope("audio-queue", || {
        let player = Player::new();
        player.play_bytes(silent_wav(300)).unwrap();
        assert!(player.playing.peek());
        assert_eq!(player.duration.peek(), Some(Duration::from_millis(300)));
        let started = std::time::Instant::now();
        player.play_bytes(silent_wav(200)).unwrap();
        assert!(
            started.elapsed() < Duration::from_millis(5),
            "switching tracks took {:?}",
            started.elapsed()
        );
        assert_eq!(player.duration.peek(), Some(Duration::from_millis(200)));
        player.stop();
        assert!(!player.playing.peek());
        assert_eq!(player.duration.peek(), None);
    });
}
