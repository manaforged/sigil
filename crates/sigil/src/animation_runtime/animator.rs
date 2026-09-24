use crate::animation::{Easing, Spring, SpringPreset, Tween};
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
        if MotionPrefs::reduced() {
            return Self {
                id: None,
                signal,
                config,
            };
        }
        let id = match config {
            AnimationConfig::Spring { preset } => state::register_spring(signal, preset),
            AnimationConfig::Tween { ms, easing } => state::register_tween(signal, ms, easing),
        };
        Self {
            id: Some(id),
            signal,
            config,
        }
    }

    pub(crate) fn set_config(&mut self, config: AnimationConfig) {
        if self.config == config {
            return;
        }
        if let Some(id) = self.id.take() {
            state::deregister(id);
        }
        self.config = config;
    }

    pub fn target(&mut self, value: f32) {
        if !value.is_finite() {
            tracing::warn!(target: "sigil::animation", value, "ignored a non-finite animation target");
            return;
        }
        if MotionPrefs::reduced() {
            if let Some(id) = self.id.take() {
                state::deregister(id);
            }
            self.signal.try_set(value);
            Runtime::mark_frame_dirty();
            return;
        }
        let id = *self.id.get_or_insert_with(|| match self.config {
            AnimationConfig::Spring { preset } => state::register_spring(self.signal, preset),
            AnimationConfig::Tween { ms, easing } => state::register_tween(self.signal, ms, easing),
        });
        state::retarget(id, value);
    }

    pub fn pause(&mut self) {
        if let Some(id) = self.id {
            state::pause(id, true);
        }
    }

    pub fn resume(&mut self) {
        if let Some(id) = self.id {
            state::pause(id, false);
        }
    }

    pub fn finish(&mut self) {
        if let Some(id) = self.id {
            state::finish(id);
        }
    }

    pub fn cancel(&mut self) {
        if let Some(id) = self.id.take() {
            state::deregister(id);
        }
    }

    #[must_use]
    pub fn value(&self) -> f32 {
        self.id
            .and_then(state::peek_value)
            .or_else(|| self.signal.try_peek())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn settled(&self) -> bool {
        self.id.is_none_or(state::is_settled)
    }
}

impl Drop for Animator {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            state::deregister(id);
        }
    }
}

pub(crate) struct SpringMotion {
    spring: Spring,
}

impl SpringMotion {
    pub(crate) fn from_preset(initial: f32, preset: SpringPreset) -> Self {
        Self {
            spring: Spring::from_preset(initial, preset),
        }
    }
}

impl SpringMotion {
    pub(crate) const fn finish(&mut self) {
        self.spring.reset(self.spring.target());
    }

    pub(crate) fn tick(&mut self, dt: f32) -> AnimatorStatus {
        if self.spring.is_settled() {
            return AnimatorStatus::Settled;
        }
        let before = self.spring.value();
        self.spring.step(dt);
        if self.spring.value().to_bits() == before.to_bits() {
            self.finish();
        }
        if self.spring.is_settled() {
            AnimatorStatus::Settled
        } else {
            AnimatorStatus::Running
        }
    }

    pub(crate) const fn retarget(&mut self, value: f32) {
        self.spring.set_target(value);
    }

    pub(crate) const fn value(&self) -> f32 {
        self.spring.value()
    }

    pub(crate) fn is_settled(&self) -> bool {
        self.spring.is_settled()
    }
}

pub(crate) struct TweenMotion {
    current: f32,
    secs: f32,
    easing: Easing,
    tween: Option<Tween>,
}

impl TweenMotion {
    pub(crate) const fn new(initial: f32, ms: u16, easing: Easing) -> Self {
        Self {
            current: initial,
            secs: ms as f32 / 1000.0,
            easing,
            tween: None,
        }
    }
}

impl TweenMotion {
    pub(crate) const fn finish(&mut self) {
        if let Some(tween) = self.tween.take() {
            self.current = tween.to();
        }
    }

    pub(crate) fn tick(&mut self, dt: f32) -> AnimatorStatus {
        let Some(tween) = self.tween.as_mut() else {
            return AnimatorStatus::Settled;
        };
        tween.step(dt);
        self.current = tween.value();
        if tween.is_finished() {
            self.tween = None;
            AnimatorStatus::Settled
        } else {
            AnimatorStatus::Running
        }
    }

    pub(crate) fn retarget(&mut self, value: f32) {
        if self.secs <= 0.0 {
            self.current = value;
            self.tween = None;
            return;
        }
        if (self.current - value).abs() < f32::EPSILON {
            self.tween = None;
            return;
        }
        self.tween = Some(Tween::new(self.current, value, self.secs).with_easing(self.easing));
    }

    pub(crate) const fn value(&self) -> f32 {
        self.current
    }

    pub(crate) const fn is_settled(&self) -> bool {
        self.tween.is_none()
    }
}
