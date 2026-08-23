//! Audio playback host service.
//!
//! Bind [`Player::playing`] and [`Player::progress`] to `transport()`.
//! [`Player::play_path`] holds the output stream until this player
//! drops. [`Player::tick`] writes progress when the decoder reports a
//! duration. [`Player::seek`] jumps when duration is known.
//! `launch` drives that tick through `tick_intervals`.

#[cfg(feature = "audio")]
use sigil_reactive::{IntervalId, cancel_interval, register_interval};
use sigil_reactive::{Signal, signal};

/// Playback handle. Signals are `Copy`; the device lives in the sink.
#[derive(Clone)]
pub struct Player {
    /// True while a sink is playing.
    pub playing: Signal<bool>,
    /// 0–1 progress. Written by [`Player::tick`] when duration is known.
    pub progress: Signal<f32>,
    #[cfg(feature = "audio")]
    device: std::rc::Rc<std::cell::RefCell<Option<Device>>>,
}

#[cfg(feature = "audio")]
struct Device {
    /// Must outlive [`Self::sink`].
    _stream: rodio::OutputStream,
    sink: rodio::Sink,
    duration: Option<std::time::Duration>,
    interval: Option<IntervalId>,
}

#[cfg(feature = "audio")]
impl Drop for Device {
    fn drop(&mut self) {
        if let Some(id) = self.interval.take() {
            cancel_interval(id);
        }
    }
}

impl Player {
    /// Idle player. Call [`play_path`] to open a file.
    #[must_use]
    pub fn new() -> Self {
        Self {
            playing: signal(false),
            progress: signal(0.0_f32),
            #[cfg(feature = "audio")]
            device: std::rc::Rc::new(std::cell::RefCell::new(None)),
        }
    }

    /// Toggle pause/resume when a sink exists.
    ///
    /// No-op until [`Player::play_path`] opens a device. Tests write
    /// [`Player::playing`] directly.
    pub fn toggle(&self) {
        #[cfg(feature = "audio")]
        {
            let device = self.device.borrow();
            if let Some(device) = device.as_ref() {
                if device.sink.is_paused() {
                    device.sink.play();
                    self.playing.set(true);
                } else {
                    device.sink.pause();
                    self.playing.set(false);
                }
            }
        }
    }

    /// Write [`Player::progress`] and [`Player::playing`] from the sink.
    ///
    /// `launch` registers an interval that calls this. An embed host
    /// that does not tick intervals should call it once per frame.
    pub fn tick(&self) {
        #[cfg(feature = "audio")]
        {
            sync(self);
        }
    }

    /// Seek to a 0–1 ratio. Requires feature `audio` and a playing
    /// file whose decoder reports a duration.
    ///
    /// # Errors
    ///
    /// Returns when no sink is open, duration is unknown, or the
    /// decoder cannot seek. Without feature `audio`, always errors.
    pub fn seek(&self, ratio: f32) -> crate::AppResult<()> {
        #[cfg(feature = "audio")]
        {
            seek_rodio(self, ratio)
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = ratio;
            Err(crate::Error::new(Disabled { feature: "audio" }))
        }
    }

    /// Decode and play `path`. Requires feature `audio`.
    ///
    /// Holds the output stream on this player. [`Player::tick`] writes
    /// [`Player::progress`] when the decoder reports a duration.
    ///
    /// # Errors
    ///
    /// Returns when the file cannot be opened, decoded, or the device
    /// cannot start. Without feature `audio`, always errors.
    pub fn play_path(&self, path: &std::path::Path) -> crate::AppResult<()> {
        #[cfg(feature = "audio")]
        {
            play_rodio(self, path)
        }
        #[cfg(not(feature = "audio"))]
        {
            let _ = path;
            Err(crate::Error::new(Disabled { feature: "audio" }))
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(feature = "audio"))]
#[derive(Debug)]
struct Disabled {
    feature: &'static str,
}

#[cfg(not(feature = "audio"))]
impl std::fmt::Display for Disabled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "enable feature `{}`", self.feature)
    }
}

#[cfg(not(feature = "audio"))]
impl std::error::Error for Disabled {}

#[cfg(any(test, feature = "audio"))]
pub(crate) fn progress_ratio(pos: std::time::Duration, total: std::time::Duration) -> f32 {
    if total.is_zero() {
        return 0.0;
    }
    (pos.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0)
}

#[cfg(feature = "audio")]
fn sync(player: &Player) {
    let mut slot = player.device.borrow_mut();
    let Some(device) = slot.as_mut() else {
        return;
    };
    if device.sink.empty() {
        player.playing.set(false);
        if let Some(total) = device.duration.filter(|total| !total.is_zero()) {
            player.progress.set(progress_ratio(total, total));
        }
        if let Some(id) = device.interval.take() {
            cancel_interval(id);
        }
        return;
    }
    let Some(total) = device.duration.filter(|total| !total.is_zero()) else {
        return;
    };
    let next = progress_ratio(device.sink.get_pos(), total);
    if (player.progress.peek() - next).abs() > 0.001 {
        player.progress.set(next);
    }
}

#[cfg(feature = "audio")]
#[derive(Debug)]
struct NoSink;

#[cfg(feature = "audio")]
impl std::fmt::Display for NoSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Player::seek needs Player::play_path")
    }
}

#[cfg(feature = "audio")]
impl std::error::Error for NoSink {}

#[cfg(feature = "audio")]
#[derive(Debug)]
struct NoDuration;

#[cfg(feature = "audio")]
impl std::fmt::Display for NoDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Player::seek needs a known duration")
    }
}

#[cfg(feature = "audio")]
impl std::error::Error for NoDuration {}

#[cfg(feature = "audio")]
fn seek_rodio(player: &Player, ratio: f32) -> crate::AppResult<()> {
    let ratio = ratio.clamp(0.0, 1.0);
    let mut slot = player.device.borrow_mut();
    let device = slot.as_mut().ok_or_else(|| crate::Error::new(NoSink))?;
    let total = device
        .duration
        .filter(|total| !total.is_zero())
        .ok_or_else(|| crate::Error::new(NoDuration))?;
    let pos = total.mul_f32(ratio);
    device
        .sink
        .try_seek(pos)
        .map_err(|err| crate::Error::new(std::io::Error::other(err.to_string())))?;
    player.progress.set(ratio);
    Ok(())
}

#[cfg(feature = "audio")]
fn play_rodio(player: &Player, path: &std::path::Path) -> crate::AppResult<()> {
    use std::fs::File;
    use std::io::BufReader;
    use std::time::Duration;

    use rodio::{Decoder, OutputStream, Sink, Source};

    let file = File::open(path).map_err(crate::Error::new)?;
    let decoder = Decoder::new(BufReader::new(file)).map_err(crate::Error::new)?;
    let duration = decoder.total_duration();
    let (stream, handle) = OutputStream::try_default().map_err(crate::Error::new)?;
    let sink = Sink::try_new(&handle).map_err(crate::Error::new)?;
    sink.append(decoder);
    sink.play();
    player.playing.set(true);
    player.progress.set(0.0);
    *player.device.borrow_mut() = Some(Device {
        _stream: stream,
        sink,
        duration,
        interval: None,
    });
    let ticking = player.clone();
    let id = register_interval(Duration::from_millis(50), move || ticking.tick());
    if let Some(device) = player.device.borrow_mut().as_mut() {
        device.interval = Some(id);
    }
    Ok(())
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;
