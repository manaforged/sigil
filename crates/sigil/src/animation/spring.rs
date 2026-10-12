#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpringPreset {
    Gentle,
    Smooth,
    Snappy,
    Bouncy,
    Stiff,
}

impl SpringPreset {
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

#[derive(Clone, Copy, Debug)]
pub struct Spring {
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
    pub const DEFAULT_VALUE_EPSILON: f32 = 0.001;

    pub const DEFAULT_VELOCITY_EPSILON: f32 = 0.001;

    #[must_use]
    #[track_caller]
    pub fn new(initial: f32, stiffness: f32, damping: f32, mass: f32) -> Self {
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

    #[must_use]
    #[track_caller]
    pub fn with_epsilons(mut self, value: f32, velocity: f32) -> Self {
        assert!(
            value > 0.0 && velocity > 0.0,
            "Spring epsilons must be positive (got value={value}, velocity={velocity})."
        );
        self.value_epsilon = value;
        self.velocity_epsilon = velocity;
        self
    }

    #[must_use]
    pub fn from_preset(initial: f32, preset: SpringPreset) -> Self {
        let (stiffness, damping, mass) = preset.parameters();
        Self::new(initial, stiffness, damping, mass)
    }

    pub const fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub const fn reset(&mut self, value: f32) {
        self.value = value;
        self.velocity = 0.0;
        self.target = value;
    }

    #[must_use]
    pub const fn value(&self) -> f32 {
        self.value
    }

    #[must_use]
    pub const fn velocity(&self) -> f32 {
        self.velocity
    }

    #[must_use]
    pub const fn target(&self) -> f32 {
        self.target
    }

    #[must_use]
    pub fn is_settled(&self) -> bool {
        let precision = f32::EPSILON * self.value.abs().max(self.target.abs());
        (self.value - self.target).abs() < self.value_epsilon.max(precision)
            && self.velocity.abs() < self.velocity_epsilon.max(precision / Self::MAX_SUBSTEP_DT)
    }

    const MAX_SUBSTEP_DT: f32 = 1.0 / 120.0;

    const MAX_SUBSTEPS: u16 = 256;

    pub fn step(&mut self, dt: f32) {
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
