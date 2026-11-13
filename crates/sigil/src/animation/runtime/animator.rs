use crate::animation::{Spring, Transition};
use crate::reactive::{Runtime, Signal, batch, cleanup, signal};

use crate::animation::runtime::config::AnimationConfig;
use crate::animation::runtime::motion_prefs::MotionPrefs;
use crate::animation::runtime::state::{self, AnimatorId};

/// An animated `f32` value driven by a tween or spring on the frame clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct Animator {
    value: Signal<f32>,
    active: Signal<bool>,
    inner: Signal<SignalAnimator>,
}

impl Animator {
    /// Creates a tween animator from `initial` using `transition`'s timing (spring presets use 150 ms ease-in-out).
    #[track_caller]
    pub fn tween(initial: f32, transition: Transition) -> Self {
        let (ms, easing, delay_ms) = transition.timing();
        Self::from_config(
            initial,
            AnimationConfig::Tween {
                ms,
                easing,
                delay_ms,
            },
        )
    }

    /// Creates an animator from `initial` using `spring` physics.
    #[track_caller]
    pub fn spring(initial: f32, spring: Spring) -> Self {
        Self::from_config(initial, AnimationConfig::Spring(spring))
    }

    #[track_caller]
    fn from_config(initial: f32, config: AnimationConfig) -> Self {
        assert!(
            initial.is_finite(),
            "animation initial value must be finite"
        );
        let value = signal(initial);
        let active = signal(false);
        let inner = signal(SignalAnimator::with_active(value, active, config));
        cleanup(move || {
            inner.try_update(SignalAnimator::cancel);
        });
        Self {
            value,
            active,
            inner,
        }
    }

    /// Animates toward `target`.
    pub fn target(&self, target: f32) {
        self.with_inner(|inner| inner.target(target));
    }

    /// Jumps immediately to `value` and stops any active motion.
    pub fn snap(&self, value: f32) {
        self.with_inner(|inner| inner.snap(value));
    }

    /// Stops any active motion, leaving the current value where it is.
    pub fn cancel(&self) {
        self.with_inner(SignalAnimator::cancel);
    }

    fn with_inner(&self, f: impl FnOnce(&mut SignalAnimator)) {
        batch(|| {
            self.inner.try_update(f);
        });
    }

    /// Reads the current animated value, subscribing the enclosing reactive scope.
    #[must_use]
    #[track_caller]
    pub fn get(&self) -> f32 {
        self.value.get()
    }

    /// Returns the underlying value signal.
    #[must_use]
    pub const fn value(&self) -> Signal<f32> {
        self.value
    }

    /// Returns a signal that is `true` while motion toward a target is running.
    #[must_use]
    pub const fn active(&self) -> Signal<bool> {
        self.active
    }
}

#[derive(Debug)]
pub(crate) struct SignalAnimator {
    id: Option<AnimatorId>,
    signal: Signal<f32>,
    active: Option<Signal<bool>>,
    config: AnimationConfig,
}

impl SignalAnimator {
    pub(crate) fn new(signal: Signal<f32>, config: AnimationConfig) -> Self {
        let id = (matches!(config, AnimationConfig::Playback { .. }) && !MotionPrefs::reduced())
            .then(|| state::register_config(signal, None, config));
        Self {
            id,
            signal,
            active: None,
            config,
        }
    }

    const fn with_active(
        signal: Signal<f32>,
        active: Signal<bool>,
        config: AnimationConfig,
    ) -> Self {
        Self {
            id: None,
            signal,
            active: Some(active),
            config,
        }
    }

    pub(crate) fn set_config(&mut self, config: AnimationConfig) {
        if self.config != config {
            self.cancel();
            self.config = config;
        }
    }

    pub(crate) fn target(&mut self, value: f32) {
        if !value.is_finite() {
            tracing::warn!(target: "sigil::animation", value, "ignored a non-finite animation target");
            return;
        }
        if MotionPrefs::reduced() {
            self.apply_immediate(value);
            return;
        }
        let current = self.signal.try_peek().unwrap_or_default();
        #[allow(
            clippy::float_cmp,
            reason = "exact match with a settled value avoids starting a no-op animation"
        )]
        if current == value && self.settled() {
            self.cancel();
            return;
        }
        let (signal, active, config) = (self.signal, self.active, self.config);
        self.id
            .get_or_insert_with(|| state::register_config(signal, active, config))
            .retarget(value);
    }

    pub(crate) fn snap(&mut self, value: f32) {
        if !value.is_finite() {
            tracing::warn!(target: "sigil::animation", value, "ignored a non-finite animation snap value");
            return;
        }
        self.apply_immediate(value);
    }

    fn apply_immediate(&mut self, value: f32) {
        batch(|| {
            self.cancel();
            self.signal.try_set(value);
            Runtime::mark_frame_dirty();
        });
    }

    pub(crate) fn finish(&self) {
        if let Some(id) = self.id {
            id.finish();
        }
    }

    pub(crate) fn cancel(&mut self) {
        if let Some(id) = self.id.take() {
            id.deregister();
        }
        if let Some(active) = self.active {
            active.try_set(false);
        }
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn value(&self) -> f32 {
        self.id
            .and_then(AnimatorId::peek_value)
            .or_else(|| self.signal.try_peek())
            .unwrap_or_default()
    }

    #[must_use]
    pub(crate) fn settled(&self) -> bool {
        self.id.is_none_or(AnimatorId::is_settled)
    }
}

impl Drop for SignalAnimator {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            id.deregister();
        }
    }
}
