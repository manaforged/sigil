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
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_exact() {
        let tween = Tween::new(2.0, 7.0, 0.5);
        assert!((tween.value() - 2.0).abs() < f32::EPSILON);

        let mut done = tween;
        done.step(0.5);
        assert!(done.is_finished());
        assert!((done.value() - 7.0).abs() < f32::EPSILON);
    }

    #[test]
    fn midpoint_under_linear_is_exact_midpoint() {
        let mut tween = Tween::new(0.0, 10.0, 1.0);
        tween.step(0.5);
        assert!((tween.value() - 5.0).abs() < 1e-5);
    }

    #[test]
    fn cubic_in_out_at_midpoint_is_half() {
        let mut tween = Tween::new(0.0, 1.0, 1.0).with_easing(Easing::CubicInOut);
        tween.step(0.5);
        assert!((tween.value() - 0.5).abs() < 1e-5);
    }

    #[test]
    fn step_past_duration_clamps() {
        let mut tween = Tween::new(0.0, 1.0, 0.5);
        tween.step(10.0);
        assert!(tween.is_finished());
        assert!((tween.value() - 1.0).abs() < f32::EPSILON);
        assert!((tween.progress() - 1.0).abs() < f32::EPSILON);

        // Subsequent steps are no-ops, not negative-time slides.
        tween.step(10.0);
        assert!((tween.value() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn restart_resets_elapsed() {
        let mut tween = Tween::new(0.0, 1.0, 1.0);
        tween.step(0.8);
        assert!(tween.value() > 0.5);
        tween.restart();
        assert!((tween.value() - 0.0).abs() < f32::EPSILON);
        assert!(!tween.is_finished());
    }

    #[test]
    fn step_is_deterministic() {
        let run = || {
            let mut tween = Tween::new(0.0, 1.0, 0.5).with_easing(Easing::CubicInOut);
            let mut samples = Vec::new();
            for _ in 0..30 {
                tween.step(1.0 / 60.0);
                samples.push(tween.value());
            }
            samples
        };
        let a = run();
        let b = run();
        for (sa, sb) in a.iter().zip(&b) {
            assert!((sa - sb).abs() < f32::EPSILON);
        }
    }

    #[test]
    #[should_panic(expected = "Tween duration must be positive")]
    fn zero_duration_panics() {
        let _ = Tween::new(0.0, 1.0, 0.0);
    }

    #[test]
    #[should_panic(expected = "Tween duration must be positive")]
    fn negative_duration_panics() {
        let _ = Tween::new(0.0, 1.0, -0.5);
    }
}
