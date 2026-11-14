use crate::animation::{Motion, Spring, Transition};
use crate::reactive::{Runtime, Signal, batch, cleanup, signal, untrack};

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

    /// Coasts from the current value at `velocity` and slows to rest by exponential decay, like Motion `inertia`.
    pub fn fling(&self, velocity: f32) {
        let _ = self.fling_to(velocity, &[]);
    }

    /// Springs to the point in `points` nearest to where a `fling` would rest, starting at `velocity`; returns that point, or `None` when it picked none.
    #[must_use]
    pub fn fling_to(&self, velocity: f32, points: &[f32]) -> Option<f32> {
        self.with_inner(|inner| inner.fling(velocity, points))
            .flatten()
    }

    /// Returns the current velocity in value units per second, without subscribing.
    #[must_use]
    pub fn velocity(&self) -> f32 {
        if !self.inner.is_alive() {
            return 0.0;
        }
        untrack(|| self.inner.with(|inner| inner.id)).map_or(0.0, AnimatorId::velocity)
    }

    fn with_inner<R>(&self, f: impl FnOnce(&mut SignalAnimator) -> R) -> Option<R> {
        let mut out = None;
        batch(|| {
            self.inner.try_update(|inner| out = Some(f(inner)));
        });
        out
    }

    /// Reads the current animated value, subscribing the enclosing reactive scope.
    #[must_use]
    #[track_caller]
    pub fn get(&self) -> f32 {
        self.value.get()
    }

    /// Reads the current animated value without subscribing, for event handlers.
    #[must_use]
    #[track_caller]
    pub fn peek(&self) -> f32 {
        self.value.peek()
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
            self.config = config;
            if let Some(id) = self.id {
                id.update(|motion| motion.reconfigure(config));
            }
        }
    }

    pub(crate) fn target(&mut self, value: f32) {
        if !finite(value, "target") {
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
        let config = self.config;
        self.source().retarget(value, config);
    }

    pub(crate) fn fling(&mut self, velocity: f32, points: &[f32]) -> Option<f32> {
        if !finite(velocity, "fling velocity") {
            return None;
        }
        let config = self.config;
        let landed = self
            .source()
            .update(|motion| motion.fling(velocity, points, config))
            .flatten();
        if MotionPrefs::reduced() {
            self.cancel();
        }
        landed
    }

    fn source(&mut self) -> AnimatorId {
        let (signal, active, config) = (self.signal, self.active, self.config);
        *self
            .id
            .get_or_insert_with(|| state::register_config(signal, active, config))
    }

    pub(crate) fn rebase(&mut self, by: f32) {
        if !finite(by, "rebase offset") {
            return;
        }
        let config = self.config;
        let moved = self.id.and_then(|id| {
            id.update(|motion| {
                *motion = Motion::configured(motion.value() + by, motion.velocity(), config);
            })
        });
        if moved.is_none() {
            self.apply_immediate(self.signal.try_peek().unwrap_or_default() + by);
        }
    }

    pub(crate) fn snap(&mut self, value: f32) {
        if !finite(value, "snap value") {
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

fn finite(value: f32, input: &'static str) -> bool {
    let finite = value.is_finite();
    if !finite {
        tracing::warn!(target: "sigil::animation", value, input, "ignored a non-finite animation input");
    }
    finite
}
