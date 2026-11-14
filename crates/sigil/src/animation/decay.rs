use super::SpringState;

/// An exponential decay that coasts a value to rest, keeping its start velocity.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Decay {
    target: f32,
    offset: f32,
}

impl Decay {
    /// Seconds for the remaining distance to fall to 1/e, like Motion `timeConstant`.
    pub(crate) const TIME_CONSTANT: f32 = 0.325;

    /// Creates a decay from `value` that starts at `velocity` units per second.
    pub(crate) fn new(value: f32, velocity: f32) -> Self {
        let offset = velocity * Self::TIME_CONSTANT;
        let target = value + offset;
        if offset.is_finite() && target.is_finite() {
            Self { target, offset }
        } else {
            Self {
                target: value,
                offset: 0.0,
            }
        }
    }

    /// Returns the value where the decay comes to rest.
    pub(crate) const fn target(self) -> f32 {
        self.target
    }

    /// Returns the value.
    pub(crate) const fn value(self) -> f32 {
        self.target - self.offset
    }

    /// Returns the velocity in units per second.
    pub(crate) const fn velocity(self) -> f32 {
        self.offset / Self::TIME_CONSTANT
    }

    /// Returns whether the decay is at rest.
    pub(crate) const fn is_settled(self) -> bool {
        self.offset.abs() < SpringState::DEFAULT_VALUE_EPSILON
    }

    /// Advances the decay by `dt` seconds.
    pub(crate) fn step(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.offset *= (-dt / Self::TIME_CONSTANT).exp();
        if self.is_settled() {
            self.finish();
        }
    }

    /// Moves the decay to its rest value.
    pub(crate) const fn finish(&mut self) {
        self.offset = 0.0;
    }
}
