mod device;

use std::time::Duration;

use crate::reactive::{Signal, effect, signal};

/// Audio player (feature `audio`).
#[derive(Clone)]
pub struct Player {
    /// True while audio plays; false when it is paused, stopped, or the queue has run out.
    pub playing: Signal<bool>,
    /// The position in the current track, from 0 to 1; it stays 0 when the length is unknown.
    pub progress: Signal<f32>,
    /// The volume, where 1 is full; a write changes the volume of the device.
    pub volume: Signal<f32>,
    /// The length of the current track, or `None` when the decoder cannot tell.
    pub duration: Signal<Option<Duration>>,
    device: std::rc::Rc<std::cell::RefCell<Option<device::Device>>>,
}

impl core::fmt::Debug for Player {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Player")
            .field("playing", &self.playing)
            .field("progress", &self.progress)
            .field("volume", &self.volume)
            .field("duration", &self.duration)
            .finish_non_exhaustive()
    }
}

impl Player {
    /// Creates a player that is not playing, at full volume, with no device open.
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
