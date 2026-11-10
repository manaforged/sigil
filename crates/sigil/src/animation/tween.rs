use crate::animation::easing::Easing;

/// A timed animation from one value to another along an easing curve.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tween {
    from: f32,
    to: f32,
    duration: f32,
    delay: f32,
    elapsed: f32,
    easing: Easing,
}

impl Tween {
    /// Creates a tween.
    #[must_use]
    #[track_caller]
    pub(crate) fn new(from: f32, to: f32, duration: f32) -> Self {
        assert!(
            duration > 0.0,
            "Tween duration must be positive (got {duration}). \
             A zero-duration animation is just an assignment."
        );
        Self {
            from,
            to,
            duration,
            delay: 0.0,
            elapsed: 0.0,
            easing: Easing::Linear,
        }
    }

    /// Sets the easing.
    #[must_use]
    pub(crate) const fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Holds the start value for `delay` seconds before the tween moves.
    #[must_use]
    pub(crate) const fn with_delay(mut self, delay: f32) -> Self {
        self.delay = delay.max(0.0);
        self
    }

    #[cfg(test)]
    /// Set the elapsed time back to zero.
    pub(crate) const fn restart(&mut self) {
        self.elapsed = 0.0;
    }

    /// Advances the tween by `dt` seconds.
    pub(crate) fn step(&mut self, dt: f32) {
        if self.is_finished() {
            return;
        }
        self.elapsed = (self.elapsed + dt).min(self.delay + self.duration);
    }

    /// Returns the value.
    #[must_use]
    pub(crate) fn value(&self) -> f32 {
        let t = (self.elapsed - self.delay).max(0.0) / self.duration;
        let eased = self.easing.apply(t);
        (self.to - self.from).mul_add(eased, self.from)
    }

    #[cfg(test)]
    /// Returns the progress.
    #[must_use]
    pub(crate) fn progress(&self) -> f32 {
        ((self.elapsed - self.delay) / self.duration).clamp(0.0, 1.0)
    }

    /// Returns whether this tween is finished.
    #[must_use]
    pub(crate) fn is_finished(&self) -> bool {
        self.elapsed >= self.delay + self.duration
    }

    /// Returns the to.
    #[must_use]
    pub(crate) const fn to(&self) -> f32 {
        self.to
    }
}

#[cfg(test)]
#[path = "tween_tests.rs"]
mod tests;
