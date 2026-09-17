use std::time::Duration;

use crate::animation::Keyframes;
use crate::reactive::Signal;

use super::keyframes::KeyframeMotion;
use super::motion_state::Motion;
use super::state::{self, AnimatorId};

pub struct Playback {
    id: AnimatorId,
}

impl Playback {
    #[must_use]
    pub fn new(value: Signal<f32>, frames: Keyframes) -> Self {
        let motion = KeyframeMotion::new(frames, value.peek());
        let id = state::register(value, Motion::Keyframes(motion));
        state::seek(id, 0.0);
        Self { id }
    }

    pub fn pause(&mut self) {
        state::pause(self.id, true);
    }
    pub fn resume(&mut self) {
        state::pause(self.id, false);
    }
    pub fn finish(&mut self) {
        state::finish(self.id);
    }
    pub fn cancel(&mut self) {
        state::cancel(self.id);
    }
    pub fn seek(&mut self, elapsed: Duration) {
        state::seek(self.id, elapsed.as_secs_f64());
    }

    pub fn restart(&mut self) {
        state::seek(self.id, 0.0);
        state::pause(self.id, false);
    }

    #[must_use]
    pub fn finished(&self) -> bool {
        state::is_settled(self.id)
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        state::deregister(self.id);
    }
}
