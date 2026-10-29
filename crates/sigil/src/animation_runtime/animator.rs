use crate::reactive::{Runtime, Signal};

use crate::animation_runtime::config::AnimationConfig;
use crate::animation_runtime::motion::MotionPrefs;
use crate::animation_runtime::state::{self, AnimatorId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimatorStatus {
    Running,
    Settled,
}

pub struct Animator {
    id: Option<AnimatorId>,
    signal: Signal<f32>,
    config: AnimationConfig,
}

impl Animator {
    #[must_use]
    pub fn new(signal: Signal<f32>, config: AnimationConfig) -> Self {
        Self {
            id: (!MotionPrefs::reduced()).then(|| state::register_config(signal, config)),
            signal,
            config,
        }
    }

    pub(crate) fn set_config(&mut self, config: AnimationConfig) {
        if self.config != config {
            self.cancel();
            self.config = config;
        }
    }

    pub fn target(&mut self, value: f32) {
        if !value.is_finite() {
            tracing::warn!(target: "sigil::animation", value, "ignored a non-finite animation target");
            return;
        }
        if MotionPrefs::reduced() {
            self.cancel();
            self.signal.try_set(value);
            Runtime::mark_frame_dirty();
            return;
        }
        let (signal, config) = (self.signal, self.config);
        self.id
            .get_or_insert_with(|| state::register_config(signal, config))
            .retarget(value);
    }

    pub fn pause(&mut self) {
        if let Some(id) = self.id {
            id.pause(true);
        }
    }

    pub fn resume(&mut self) {
        if let Some(id) = self.id {
            id.pause(false);
        }
    }

    pub fn finish(&mut self) {
        if let Some(id) = self.id {
            id.finish();
        }
    }

    pub fn cancel(&mut self) {
        if let Some(id) = self.id.take() {
            id.deregister();
        }
    }

    #[must_use]
    pub fn value(&self) -> f32 {
        self.id
            .and_then(AnimatorId::peek_value)
            .or_else(|| self.signal.try_peek())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn settled(&self) -> bool {
        self.id.is_none_or(AnimatorId::is_settled)
    }
}

impl Drop for Animator {
    fn drop(&mut self) {
        self.cancel();
    }
}
