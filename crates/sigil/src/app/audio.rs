use crate::reactive::{IntervalId, cancel_interval, interval};
use crate::reactive::{Signal, signal};

#[derive(Clone)]
pub struct Player {
    pub playing: Signal<bool>,
    pub progress: Signal<f32>,
    device: std::rc::Rc<std::cell::RefCell<Option<Device>>>,
}

struct Device {
    _stream: rodio::MixerDeviceSink,
    sink: rodio::Player,
    duration: Option<std::time::Duration>,
    interval: Option<IntervalId>,
}

impl Drop for Device {
    fn drop(&mut self) {
        if let Some(id) = self.interval.take() {
            cancel_interval(id);
        }
    }
}

impl Player {
    #[must_use]
    pub fn new() -> Self {
        Self {
            playing: signal(false),
            progress: signal(0.0_f32),
            device: std::rc::Rc::new(std::cell::RefCell::new(None)),
        }
    }

    pub fn toggle(&self) {
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

    pub fn tick(&self) {
        sync(self);
    }

    pub fn seek(&self, ratio: f32) -> crate::AppResult<()> {
        seek_rodio(self, ratio)
    }

    pub fn play_path(&self, path: &std::path::Path) -> crate::AppResult<()> {
        play_rodio(self, path)
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn progress_ratio(pos: std::time::Duration, total: std::time::Duration) -> f32 {
    if total.is_zero() {
        return 0.0;
    }
    (pos.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0)
}

fn sync(player: &Player) {
    let mut slot = player.device.borrow_mut();
    let Some(device) = slot.as_mut() else {
        return;
    };
    if device.sink.empty() {
        player.playing.try_set(false);
        if let Some(total) = device.duration.filter(|total| !total.is_zero()) {
            player.progress.try_set(progress_ratio(total, total));
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
    if player
        .progress
        .try_peek()
        .is_some_and(|current| (current - next).abs() > 0.001)
    {
        player.progress.try_set(next);
    }
}

#[derive(Debug)]
struct NoSink;

impl std::fmt::Display for NoSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Player::seek needs Player::play_path")
    }
}

impl std::error::Error for NoSink {}

#[derive(Debug)]
struct NoDuration;

impl std::fmt::Display for NoDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Player::seek needs a known duration")
    }
}

impl std::error::Error for NoDuration {}

fn seek_rodio(player: &Player, ratio: f32) -> crate::AppResult<()> {
    let ratio = ratio.clamp(0.0, 1.0);
    let mut slot = player.device.borrow_mut();
    let device = slot.as_mut().ok_or_else(|| crate::Error::io(NoSink))?;
    let total = device
        .duration
        .filter(|total| !total.is_zero())
        .ok_or_else(|| crate::Error::io(NoDuration))?;
    let pos = total.mul_f32(ratio);
    device
        .sink
        .try_seek(pos)
        .map_err(|err| crate::Error::io(std::io::Error::other(err.to_string())))?;
    player.progress.set(ratio);
    Ok(())
}

fn play_rodio(player: &Player, path: &std::path::Path) -> crate::AppResult<()> {
    use std::fs::File;
    use std::io::BufReader;
    use std::time::Duration;

    use rodio::{Decoder, DeviceSinkBuilder, Player as RodioPlayer, Source};

    let file = File::open(path).map_err(crate::Error::io)?;
    let decoder = Decoder::new(BufReader::new(file)).map_err(crate::Error::io)?;
    let duration = decoder.total_duration();
    let stream = DeviceSinkBuilder::open_default_sink().map_err(crate::Error::io)?;
    let sink = RodioPlayer::connect_new(stream.mixer());
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
    let id = interval(Duration::from_millis(50), move || ticking.tick());
    if let Some(device) = player.device.borrow_mut().as_mut() {
        device.interval = Some(id);
    }
    Ok(())
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;
