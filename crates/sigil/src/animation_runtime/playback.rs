use std::time::Duration;

use crate::animation::{Keyframes, Motion};
use crate::reactive::Signal;

use super::state::{self, AnimatorId};

pub struct Playback {
    id: AnimatorId,
}

impl Playback {
    #[must_use]
    pub fn new(value: Signal<f32>, frames: Keyframes) -> Self {
        let motion = Motion::keyframes(frames, value.try_peek().unwrap_or_default());
        let id = state::register(value, motion);
        id.seek(0.0);
        Self { id }
    }

    pub fn pause(&mut self) {
        self.id.pause(true);
    }
    pub fn resume(&mut self) {
        self.id.pause(false);
    }
    pub fn finish(&mut self) {
        self.id.finish();
    }
    pub fn cancel(&mut self) {
        self.id.cancel();
    }
    pub fn seek(&mut self, elapsed: Duration) {
        self.id.seek(elapsed.as_secs_f64());
    }

    pub fn restart(&mut self) {
        self.id.seek(0.0);
        self.id.pause(false);
    }

    #[must_use]
    pub fn finished(&self) -> bool {
        self.id.is_settled()
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        self.id.deregister();
    }
}

#[cfg(test)]
#[path = "playback_tests.rs"]
mod tests;
