//! [`Animator`] — public handle that drives a [`Signal<f32>`]. The
//! heavy state (the wrapped spring or tween) lives in [`crate::state`];
//! the handle holds only an opaque [`AnimatorId`] and the signal it
//! drives.

use sigil_animation::{Easing, Spring, SpringPreset, Tween};
use sigil_reactive::Signal;

use crate::config::AnimationConfig;
use crate::motion::MotionPrefs;
use crate::state::{self, AnimatorId};

/// Internal per-frame contract. Implementations exist for `Spring`
/// (`SpringMotion`) and `Tween` (`TweenMotion`).
pub(crate) trait Motion: 'static {
    /// Advance the motion by `dt` seconds and report its status.
    fn tick(&mut self, dt: f32) -> AnimatorStatus;
    /// Re-aim the motion at a new target.
    fn retarget(&mut self, value: f32);
    /// Snapshot the current driven value (untracked).
    fn value(&self) -> f32;
    /// True when the wrapped primitive considers itself done.
    fn is_settled(&self) -> bool;
}

/// Result of one tick. The runtime uses [`AnimatorStatus::Settled`] to
/// evict animators from the active set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimatorStatus {
    /// Still moving; keep ticking next frame.
    Running,
    /// Motion settled this frame; runtime removes from active set.
    Settled,
}

/// Animator driving a [`Signal<f32>`] under a spring or tween config.
///
/// Construct via [`Animator::new`] with an [`AnimationConfig`]. The
/// constructor self-registers with the per-thread active set; the
/// runtime's [`crate::tick`] walks that set each frame and writes new
/// values into the bound signal. Drop the animator to de-register.
///
/// Re-target a still-running or settled animator with
/// [`Animator::target`]; the spring/tween resumes ticking from its
/// current value.
pub struct Animator {
    id: Option<AnimatorId>,
    signal: Signal<f32>,
}

impl Animator {
    /// Construct an animator driving `signal` under `config`. Initial
    /// position is the signal's current value.
    ///
    /// When [`MotionPrefs::reduced`](crate::MotionPrefs::reduced)
    /// returns `true` at construction the animator does **not**
    /// register — the signal jumps to whatever target the caller sets.
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

    /// Re-aim at `value`. Settled animators re-arm; running animators
    /// retarget without resetting velocity. Reduced-motion animators
    /// jump instantly.
    pub fn target(&mut self, value: f32) {
        match self.id {
            Some(id) => state::retarget(id, value),
            None => self.signal.set(value),
        }
    }

    /// Current driven value (untracked). Reduced-motion animators
    /// return the bound signal's current value.
    #[must_use]
    pub fn value(&self) -> f32 {
        let fallback = self.signal.peek();
        self.id.and_then(state::peek_value).unwrap_or(fallback)
    }

    /// True when the underlying motion is settled (or the animator is
    /// reduced-motion).
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

// -------------------------------------------------------------------
// Spring motion (impl `Motion` for the registry's box-dyn slot)
// -------------------------------------------------------------------

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

// -------------------------------------------------------------------
// Tween motion
// -------------------------------------------------------------------

pub(crate) struct TweenMotion {
    current: f32,
    secs: f32,
    easing: Easing,
    /// Active interpolation. `None` until a `retarget` arms it — a
    /// tween with no target is a no-op steady state.
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
