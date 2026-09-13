use crate::animation::easing::Easing;

#[derive(Clone, Copy, Debug)]
pub struct Tween {
    from: f32,
    to: f32,
    duration: f32,
    elapsed: f32,
    easing: Easing,
}

impl Tween {
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

    #[must_use]
    pub const fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    pub const fn restart(&mut self) {
        self.elapsed = 0.0;
    }

    pub fn step(&mut self, dt: f32) {
        if self.is_finished() {
            return;
        }
        self.elapsed = (self.elapsed + dt).min(self.duration);
    }

    #[must_use]
    pub fn value(&self) -> f32 {
        let t = self.elapsed / self.duration;
        let eased = self.easing.apply(t);
        (self.to - self.from).mul_add(eased, self.from)
    }

    #[must_use]
    pub fn progress(&self) -> f32 {
        (self.elapsed / self.duration).clamp(0.0, 1.0)
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.duration
    }

    #[must_use]
    pub const fn from(&self) -> f32 {
        self.from
    }

    #[must_use]
    pub const fn to(&self) -> f32 {
        self.to
    }

    #[must_use]
    pub const fn duration(&self) -> f32 {
        self.duration
    }
}

#[cfg(test)]
#[path = "tween_tests.rs"]
mod tests;
