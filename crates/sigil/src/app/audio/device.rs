use std::collections::VecDeque;
use std::io::{Read, Seek};
use std::time::Duration;

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player as RodioPlayer, Source};

use crate::reactive::{IntervalId, cancel_interval, interval};

use super::Player;

pub(super) struct Device {
    stream: MixerDeviceSink,
    sink: RodioPlayer,
    durations: VecDeque<Option<Duration>>,
    interval: Option<IntervalId>,
}

impl Drop for Device {
    fn drop(&mut self) {
        if let Some(id) = self.interval.take() {
            cancel_interval(id);
        }
    }
}

pub(super) fn decode_path(
    path: &std::path::Path,
) -> crate::AppResult<Decoder<std::io::BufReader<std::fs::File>>> {
    let file = std::fs::File::open(path).map_err(crate::Error::io)?;
    Decoder::new(std::io::BufReader::new(file)).map_err(crate::Error::io)
}

pub(super) fn decode_bytes(bytes: Vec<u8>) -> crate::AppResult<Decoder<std::io::Cursor<Vec<u8>>>> {
    Decoder::new(std::io::Cursor::new(bytes)).map_err(crate::Error::io)
}

impl Player {
    pub(super) fn set_volume(&self, volume: f32) {
        if let Some(device) = self.device.borrow().as_ref() {
            device.sink.set_volume(rodio::Float::from(volume));
        }
    }

    pub fn toggle(&self) {
        let next = {
            let device = self.device.borrow();
            let Some(device) = device.as_ref() else {
                return;
            };
            if device.sink.is_paused() {
                device.sink.play();
                true
            } else {
                device.sink.pause();
                false
            }
        };
        self.playing.set(next);
    }

    fn open(&self) -> crate::AppResult<()> {
        let mut slot = self.device.borrow_mut();
        if slot.is_some() {
            return Ok(());
        }
        let stream = DeviceSinkBuilder::open_default_sink().map_err(crate::Error::io)?;
        let sink = RodioPlayer::connect_new(stream.mixer());
        sink.set_volume(rodio::Float::from(self.volume.peek()));
        *slot = Some(Device {
            stream,
            sink,
            durations: VecDeque::new(),
            interval: None,
        });
        Ok(())
    }

    pub(super) fn reset(&self) -> crate::AppResult<()> {
        self.open()?;
        if let Some(device) = self.device.borrow_mut().as_mut() {
            let sink = RodioPlayer::connect_new(device.stream.mixer());
            sink.set_volume(rodio::Float::from(self.volume.peek()));
            device.sink = sink;
            device.durations.clear();
        }
        self.progress.set(0.0);
        self.duration.set(None);
        Ok(())
    }

    pub fn stop(&self) {
        if let Some(device) = self.device.borrow_mut().as_mut() {
            device.sink.stop();
            device.durations.clear();
            if let Some(id) = device.interval.take() {
                cancel_interval(id);
            }
        }
        self.playing.set(false);
        self.progress.set(0.0);
        self.duration.set(None);
    }

    pub(super) fn append_source<R>(&self, decoder: Decoder<R>) -> crate::AppResult<()>
    where
        R: Read + Seek + Send + Sync + 'static,
    {
        self.open()?;
        let total = decoder.total_duration();
        let (first, needs_interval) = {
            let mut slot = self.device.borrow_mut();
            let Some(device) = slot.as_mut() else {
                return Ok(());
            };
            let first = device.sink.empty();
            if first {
                device.durations.clear();
            }
            device.durations.push_back(total);
            device.sink.append(decoder);
            device.sink.play();
            (first, device.interval.is_none())
        };
        if first {
            self.progress.set(0.0);
            self.duration.set(total);
        }
        self.playing.set(true);
        if needs_interval {
            let ticking = self.clone();
            let id = interval(Duration::from_millis(50), move || ticking.tick());
            if let Some(device) = self.device.borrow_mut().as_mut() {
                device.interval = Some(id);
            }
        }
        Ok(())
    }

    pub fn tick(&self) {
        let mut slot = self.device.borrow_mut();
        let Some(device) = slot.as_mut() else {
            return;
        };
        let mut advanced = false;
        while device.durations.len() > device.sink.len().max(1) {
            device.durations.pop_front();
            advanced = true;
        }
        let current = device.durations.front().copied().flatten();
        if advanced {
            self.duration.try_set(current);
            self.progress.try_set(0.0);
        }
        if device.sink.empty() {
            self.playing.try_set(false);
            if current.is_some_and(|total| !total.is_zero()) {
                self.progress.try_set(1.0);
            }
            if let Some(id) = device.interval.take() {
                cancel_interval(id);
            }
            return;
        }
        let Some(total) = current.filter(|total| !total.is_zero()) else {
            return;
        };
        let next = super::progress_ratio(device.sink.get_pos(), total);
        if self
            .progress
            .try_peek()
            .is_some_and(|value| (value - next).abs() > 0.001)
        {
            self.progress.try_set(next);
        }
    }

    pub fn seek(&self, ratio: f32) -> crate::AppResult<()> {
        let ratio = ratio.clamp(0.0, 1.0);
        {
            let slot = self.device.borrow();
            let device = slot.as_ref().ok_or_else(|| crate::Error::io(NoSink))?;
            let total = device
                .durations
                .front()
                .copied()
                .flatten()
                .filter(|total| !total.is_zero())
                .ok_or_else(|| crate::Error::io(NoDuration))?;
            device
                .sink
                .try_seek(total.mul_f32(ratio))
                .map_err(|err| crate::Error::io(std::io::Error::other(err.to_string())))?;
        }
        self.progress.set(ratio);
        Ok(())
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
