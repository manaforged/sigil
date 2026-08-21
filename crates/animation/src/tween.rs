//! Fixed-duration interpolation between two `f32` endpoints under a
//! named [`Easing`] curve.
//!
//! Use [`Tween`] when the animation needs to land at a specific time
//! (transitions, splash screens, choreographed reveals). Use
//! [`crate::Spring`] when the animation should react to a moving
//! target with physical inertia (drags, value tracking, scroll).

use crate::easing::Easing;

/// Time-based interpolation between two endpoints under an [`Easing`].
///
/// Advance with [`Tween::step`], read the current value via
/// [`Tween::value`], and stop ticking when [`Tween::is_finished`]
/// returns `true`.
#[derive(Clone, Copy, Debug)]
pub struct Tween {
    from: f32,
    to: f32,
    duration: f32,
    elapsed: f32,
    easing: Easing,
}

impl Tween {
    /// Construct a tween between `from` and `to` over `duration`
    /// seconds, using [`Easing::Linear`].
    ///
    /// # Panics
    ///
    /// Panics if `duration` is non-positive. Zero-duration tweens are
    /// represented by setting the destination value directly and not
    /// animating.
    #[must_use]
    #[track_caller]
    pub fn new(from: f32, to: f32, duration: f32) -> Self {
        assert!(
            duration > 0.0,
            "Tween duration must be positive (got {duration}). \
             A zero-duration animation is just an assignment."
        );
        Self {
            from,
            to,
            duration,
            elapsed: 0.0,
            easing: Easing::Linear,
        }
    }

    /// Replace the easing curve. Builder-style; consumes `self`.
    #[must_use]
    pub const fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Restart the tween from the beginning. Does not change
    /// endpoints, duration, or easing.
    pub const fn restart(&mut self) {
        self.elapsed = 0.0;
    }

    /// Advance the tween by `dt` seconds. Reading [`Self::value`]
    /// afterwards yields the new interpolated value. Calling `step`
    /// after the tween has finished is a no-op.
    pub fn step(&mut self, dt: f32) {
        if self.is_finished() {
            return;
        }
        self.elapsed = (self.elapsed + dt).min(self.duration);
    }

    /// Current interpolated value. Reads off the easing curve at the
    /// current elapsed time and lerps between the endpoints.
    #[must_use]
    pub fn value(&self) -> f32 {
        let t = self.elapsed / self.duration;
        let eased = self.easing.apply(t);
        (self.to - self.from).mul_add(eased, self.from)
    }

    /// Normalised progress `t ∈ [0, 1]` — useful when the caller wants
    /// to drive several parallel quantities off one tween's clock.
    #[must_use]
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    /// Returns `true` once `elapsed >= duration`. Stop ticking the
    /// tween at that point.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.duration
    }

    /// Start value.
    #[must_use]
    pub const fn from(&self) -> f32 {
        self.from
    }

    /// End value.
    #[must_use]
    pub const fn to(&self) -> f32 {
        self.to
    }

    /// Total duration in seconds.
    #[must_use]
    pub const fn duration(&self) -> f32 {
        self.duration
    }
}

#[cfg(test)]
#[path = "tween_tests.rs"]
mod tests;
