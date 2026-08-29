#[cfg(feature = "audio")]
use sigil_reactive::{IntervalId, cancel_interval, register_interval};
use sigil_reactive::{Signal, signal};

#[derive(Clone)]
pub struct Player {
    pub playing: Signal<bool>,
    pub progress: Signal<f32>,
    #[cfg(feature = "audio")]
    device: std::rc::Rc<std::cell::RefCell<Option<Device>>>,
}

#[cfg(feature = "audio")]
struct Device {
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
    #[must_use]
    pub fn new() -> Self {
        Self {
            playing: signal(false),
            progress: signal(0.0_f32),
            #[cfg(feature = "audio")]
            device: std::rc::Rc::new(std::cell::RefCell::new(None)),
        }
    }

    #[allow(
        clippy::missing_const_for_fn,
        reason = "no-op body without feature `audio` is trivially const-able; the real body is not"
    )]
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

    #[allow(
        clippy::missing_const_for_fn,
        reason = "const-eligible only under one cfg configuration"
    )]
    pub fn tick(&self) {
        #[cfg(feature = "audio")]
        {
            sync(self);
        }
    }

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
