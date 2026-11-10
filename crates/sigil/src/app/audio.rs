mod device;

use std::time::Duration;

use crate::reactive::{Signal, effect, signal};

/// Audio player (feature `audio`).
#[derive(Clone)]
pub struct Player {
    /// The playing.
    pub playing: Signal<bool>,
    /// The progress.
    pub progress: Signal<f32>,
    /// The volume.
    pub volume: Signal<f32>,
    /// The duration.
    pub duration: Signal<Option<Duration>>,
    device: std::rc::Rc<std::cell::RefCell<Option<device::Device>>>,
}

impl Player {
    /// Creates a player.
    #[must_use]
    pub fn new() -> Self {
        let player = Self {
            playing: signal(false),
            progress: signal(0.0_f32),
            volume: signal(1.0_f32),
            duration: signal(None),
            device: std::rc::Rc::new(std::cell::RefCell::new(None)),
        };
        let watcher = player.clone();
        effect(move || watcher.set_volume(watcher.volume.get()));
        player
    }

    /// Plays the audio file at `path`.
    pub fn play_path(&self, path: &std::path::Path) -> crate::AppResult<()> {
        let decoder = device::decode_path(path)?;
        self.reset()?;
        self.append_source(decoder)
    }

    /// Plays encoded audio from memory.
    pub fn play_bytes(&self, bytes: Vec<u8>) -> crate::AppResult<()> {
        let decoder = device::decode_bytes(bytes)?;
        self.reset()?;
        self.append_source(decoder)
    }

    /// Replaces the queue with the audio files in `paths`.
    pub fn queue<P: AsRef<std::path::Path>>(
        &self,
        paths: impl IntoIterator<Item = P>,
    ) -> crate::AppResult<()> {
        self.reset()?;
        for path in paths {
            self.append(path.as_ref())?;
        }
        Ok(())
    }

    /// Adds the audio file at `path` to the end of the queue.
    pub fn append(&self, path: &std::path::Path) -> crate::AppResult<()> {
        let decoder = device::decode_path(path)?;
        self.append_source(decoder)
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn progress_ratio(pos: Duration, total: Duration) -> f32 {
    if total.is_zero() {
        return 0.0;
    }
    (pos.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0)
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;
