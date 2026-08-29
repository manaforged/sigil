use sigil_animation::{Easing, Spring, SpringPreset, Tween};
use sigil_reactive::Signal;

use crate::config::AnimationConfig;
use crate::motion::MotionPrefs;
use crate::state::{self, AnimatorId};

pub(crate) trait Motion: 'static {
    fn tick(&mut self, dt: f32) -> AnimatorStatus;
    fn retarget(&mut self, value: f32);
    fn value(&self) -> f32;
    fn is_settled(&self) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimatorStatus {
    Running,
    Settled,
}

pub struct Animator {
    id: Option<AnimatorId>,
    signal: Signal<f32>,
}

impl Animator {
    #[must_use]
    pub fn new(signal: Signal<f32>, config: AnimationConfig) -> Self {
        if MotionPrefs::reduced() {
            return Self { id: None, signal };
        }
        let id = match config {
            AnimationConfig::Spring { preset } => state::register_spring(signal, preset),
            AnimationConfig::Tween { ms, easing } => state::register_tween(signal, ms, easing),
        };
        Self {
            id: Some(id),
            signal,
        }
    }

    pub fn target(&mut self, value: f32) {
        match self.id {
            Some(id) => state::retarget(id, value),
            None => self.signal.set(value),
        }
    }

    #[must_use]
    pub fn value(&self) -> f32 {
        let fallback = self.signal.peek();
        self.id.and_then(state::peek_value).unwrap_or(fallback)
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

impl Motion for SpringMotion {
    fn tick(&mut self, dt: f32) -> AnimatorStatus {
        if self.spring.is_settled() {
            return AnimatorStatus::Settled;
        }
        self.spring.step(dt);
        if self.spring.is_settled() {
            AnimatorStatus::Settled
        } else {
            AnimatorStatus::Running
        }
    }

    fn retarget(&mut self, value: f32) {
        self.spring.set_target(value);
    }

    fn value(&self) -> f32 {
        self.spring.value()
    }

    fn is_settled(&self) -> bool {
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

impl Motion for TweenMotion {
    fn tick(&mut self, dt: f32) -> AnimatorStatus {
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

    fn retarget(&mut self, value: f32) {
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

    fn value(&self) -> f32 {
        self.current
    }

    fn is_settled(&self) -> bool {
        self.tween.is_none()
    }
}
