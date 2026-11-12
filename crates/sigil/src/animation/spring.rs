kittest_pub! {
    /// Spring constants.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub enum SpringPreset {
        /// The gentle spring preset.
        Gentle,
        /// The smooth spring preset.
        Smooth,
        /// The snappy spring preset.
        Snappy,
        /// The bouncy spring preset.
        Bouncy,
        /// The stiff spring preset.
        Stiff,
    }
}

impl SpringPreset {
    kittest_pub! {
        /// The stiffness, damping, and mass of this preset, in that order.
        #[must_use]
        pub const fn parameters(self) -> (f32, f32, f32) {
            match self {
                Self::Gentle => (80.0, 14.0, 1.0),
                Self::Smooth => (180.0, 20.0, 1.0),
                Self::Snappy => (260.0, 26.0, 1.0),
                Self::Bouncy => (300.0, 12.0, 1.0),
                Self::Stiff => (500.0, 40.0, 1.0),
            }
        }
    }
}

/// A damped spring that steps a value toward its target.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Spring {
    stiffness: f32,
    damping: f32,
    mass: f32,
    value_epsilon: f32,
    velocity_epsilon: f32,
    value: f32,
    velocity: f32,
    target: f32,
}

impl Spring {
    /// The default distance from the target at which the spring settles.
    pub(crate) const DEFAULT_VALUE_EPSILON: f32 = 0.001;

    /// The default speed below which the spring settles.
    pub(crate) const DEFAULT_VELOCITY_EPSILON: f32 = 0.001;

    /// Creates a spring.
    #[must_use]
    #[track_caller]
    pub(crate) fn new(initial: f32, stiffness: f32, damping: f32, mass: f32) -> Self {
        assert!(
            mass > 0.0,
            "Spring mass must be positive (got {mass}). Use 1.0 for the default."
        );
        Self {
            stiffness,
            damping,
            mass,
            value_epsilon: Self::DEFAULT_VALUE_EPSILON,
            velocity_epsilon: Self::DEFAULT_VELOCITY_EPSILON,
            value: initial,
            velocity: 0.0,
            target: initial,
        }
    }

    #[cfg(test)]
    /// Rest thresholds of a spring.
    #[must_use]
    #[track_caller]
    pub(crate) fn with_epsilons(mut self, value: f32, velocity: f32) -> Self {
        assert!(
            value > 0.0 && velocity > 0.0,
            "Spring epsilons must be positive (got value={value}, velocity={velocity})."
        );
        self.value_epsilon = value;
        self.velocity_epsilon = velocity;
        self
    }

    /// Creates a spring from preset.
    #[must_use]
    pub(crate) fn from_preset(initial: f32, preset: SpringPreset) -> Self {
        let (stiffness, damping, mass) = preset.parameters();
        Self::new(initial, stiffness, damping, mass)
    }

    /// Sets the target.
    pub(crate) const fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Moves the spring to `value` and clears its velocity.
    pub(crate) const fn reset(&mut self, value: f32) {
        self.value = value;
        self.velocity = 0.0;
        self.target = value;
    }

    /// Returns the value.
    #[must_use]
    pub(crate) const fn value(&self) -> f32 {
        self.value
    }

    #[cfg(test)]
    /// Returns the velocity.
    #[must_use]
    pub(crate) const fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Returns the target.
    #[must_use]
    pub(crate) const fn target(&self) -> f32 {
        self.target
    }

    /// Returns whether this spring is settled.
    #[must_use]
    pub(crate) fn is_settled(&self) -> bool {
        let precision = f32::EPSILON * self.value.abs().max(self.target.abs());
        (self.value - self.target).abs() < self.value_epsilon.max(precision)
            && self.velocity.abs() < self.velocity_epsilon.max(precision / Self::MAX_SUBSTEP_DT)
    }

    const MAX_SUBSTEP_DT: f32 = 1.0 / 120.0;

    const MAX_SUBSTEPS: u16 = 256;

    /// Advances the spring by `dt` seconds.
    pub(crate) fn step(&mut self, dt: f32) {
        if self.snap_if_settled() || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let clamped = (dt / Self::MAX_SUBSTEP_DT)
            .floor()
            .clamp(0.0, f32::from(Self::MAX_SUBSTEPS));
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Bounded by clamp() to [0, MAX_SUBSTEPS] above; never NaN/negative."
        )]
        let whole_steps = clamped as u16;
        for _ in 0..whole_steps {
            self.integrate(Self::MAX_SUBSTEP_DT);
            if self.snap_if_settled() {
                return;
            }
        }
        let consumed = f32::from(whole_steps) * Self::MAX_SUBSTEP_DT;
        let tail = (dt - consumed).min(Self::MAX_SUBSTEP_DT);
        if tail.is_sign_positive() && tail > f32::EPSILON {
            self.integrate(tail);
            self.snap_if_settled();
        }
    }

    pub(crate) fn advance(&mut self, dt: f32) {
        if self.is_settled() {
            return;
        }
        let before = self.value;
        self.step(dt);
        if self.value.to_bits() == before.to_bits() {
            self.reset(self.target);
        }
    }

    fn snap_if_settled(&mut self) -> bool {
        let settled = self.is_settled();
        if settled {
            self.value = self.target;
            self.velocity = 0.0;
        }
        settled
    }

    fn integrate(&mut self, dt: f32) {
        let spring_force = -self.stiffness * (self.value - self.target);
        let damper_force = -self.damping * self.velocity;
        let acceleration = (spring_force + damper_force) / self.mass;
        self.velocity = acceleration.mul_add(dt, self.velocity);
        self.value = self.velocity.mul_add(dt, self.value);
    }
}

#[cfg(test)]
#[path = "spring_tests.rs"]
mod tests;
