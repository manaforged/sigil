use core::f32::consts::TAU;
use std::time::Duration;

/// Spring constants for damped spring motion.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub struct Spring {
    stiffness: f32,
    damping: f32,
    mass: f32,
}

impl Default for Spring {
    fn default() -> Self {
        Self::SMOOTH
    }
}

impl Spring {
    /// Stiffness 80, damping 14, mass 1: the slowest preset, with a small overshoot.
    pub const GENTLE: Self = Self::new(80.0, 14.0);
    /// Stiffness 180, damping 20, mass 1: a medium speed with a small overshoot.
    pub const SMOOTH: Self = Self::new(180.0, 20.0);
    /// Stiffness 260, damping 26, mass 1: a quick move with a small overshoot.
    pub const SNAPPY: Self = Self::new(260.0, 26.0);
    /// Stiffness 300, damping 12, mass 1: a quick move that swings past the target several times.
    pub const BOUNCY: Self = Self::new(300.0, 12.0);
    /// Stiffness 500, damping 40, mass 1: the fastest preset, with almost no overshoot.
    pub const STIFF: Self = Self::new(500.0, 40.0);

    /// Creates a spring with mass `1.0`; motion bounds its frequency and damping ratio so it stays finite and settles.
    pub const fn new(stiffness: f32, damping: f32) -> Self {
        Self {
            stiffness: bounded(stiffness, f32::MIN_POSITIVE),
            damping: bounded(damping, 0.0),
            mass: 1.0,
        }
    }

    /// Creates a spring without bounce that visually reaches its target in about `duration`, like Motion `visualDuration`.
    pub fn duration(duration: Duration) -> Self {
        let root = TAU / (VISUAL_PERIODS * duration.as_secs_f32());
        Self::new(root * root, 0.0).bounce(0.0)
    }

    /// Sets the bounce from 0.0 (none) to 1.0 (most) and keeps the stiffness and mass, like Motion `bounce`.
    pub fn bounce(mut self, bounce: f32) -> Self {
        let ratio = (1.0 - bounded(bounce, 0.0).min(1.0)).max(SpringState::MIN_DAMPING_RATIO);
        self.damping = bounded(2.0 * ratio * (self.stiffness * self.mass).sqrt(), 0.0);
        self
    }

    /// Sets the spring mass; a NaN or non-positive mass reads as the smallest positive mass.
    pub const fn mass(mut self, mass: f32) -> Self {
        self.mass = bounded(mass, f32::MIN_POSITIVE);
        self
    }

    pub(crate) const fn parameters(self) -> (f32, f32, f32) {
        (self.stiffness, self.damping, self.mass)
    }
}

const VISUAL_PERIODS: f32 = 1.2;

const fn bounded(value: f32, min: f32) -> f32 {
    if value.is_nan() {
        min
    } else {
        value.clamp(min, f32::MAX)
    }
}

/// A damped spring that steps a value toward its target.
#[derive(Clone, Copy, Debug)]
pub(crate) struct SpringState {
    stiffness: f32,
    damping: f32,
    mass: f32,
    substep: f32,
    value_epsilon: f32,
    velocity_epsilon: f32,
    value: f32,
    velocity: f32,
    target: f32,
}

impl SpringState {
    /// The default distance from the target at which the spring settles.
    pub(crate) const DEFAULT_VALUE_EPSILON: f32 = 0.001;

    /// The default speed below which the spring settles.
    pub(crate) const DEFAULT_VELOCITY_EPSILON: f32 = 0.001;

    /// Creates a spring state.
    #[must_use]
    pub(crate) fn new(initial: f32, spring: Spring) -> Self {
        let (stiffness, damping, mass, frequency) = Self::stable(spring);
        Self {
            stiffness,
            damping,
            mass,
            substep: Self::MAX_SUBSTEP_DT.min(Self::MAX_PHASE_PER_SUBSTEP / frequency),
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

    /// Returns the velocity.
    #[must_use]
    pub(crate) const fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Sets the velocity; a non-finite velocity reads as zero.
    pub(crate) const fn set_velocity(&mut self, velocity: f32) {
        self.velocity = if velocity.is_finite() { velocity } else { 0.0 };
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
            && self.velocity.abs() < self.velocity_epsilon.max(precision / self.substep)
    }

    const MAX_SUBSTEP_DT: f32 = 1.0 / 120.0;

    const MAX_SUBSTEPS: u16 = 256;

    const MAX_PHASE_PER_SUBSTEP: f32 = 0.5;

    const MIN_FREQUENCY: f32 = 6.0;

    const MAX_FREQUENCY: f32 = 1000.0;

    const MIN_DAMPING_RATIO: f32 = 0.3;

    const MAX_DAMPING_RATIO: f32 = 1.0;

    fn stable(spring: Spring) -> (f32, f32, f32, f32) {
        let (stiffness, damping, mass) = spring.parameters();
        let frequency = (stiffness / mass).sqrt();
        let clamped = frequency.clamp(Self::MIN_FREQUENCY, Self::MAX_FREQUENCY);
        let ratio = damping / mass / (2.0 * clamped);
        let ratios = Self::MIN_DAMPING_RATIO..=Self::MAX_DAMPING_RATIO;
        if (Self::MIN_FREQUENCY..=Self::MAX_FREQUENCY).contains(&frequency)
            && ratios.contains(&ratio)
        {
            return (stiffness, damping, mass, frequency);
        }
        let ratio = ratio.clamp(Self::MIN_DAMPING_RATIO, Self::MAX_DAMPING_RATIO);
        (clamped * clamped, 2.0 * ratio * clamped, 1.0, clamped)
    }

    /// Advances the spring by `dt` seconds.
    pub(crate) fn step(&mut self, dt: f32) {
        if self.snap_if_settled() || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let clamped = (dt / self.substep)
            .floor()
            .clamp(0.0, f32::from(Self::MAX_SUBSTEPS));
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Bounded by clamp() to [0, MAX_SUBSTEPS] above; never NaN/negative."
        )]
        let whole_steps = clamped as u16;
        for _ in 0..whole_steps {
            self.integrate(self.substep);
            if self.snap_if_settled() {
                return;
            }
        }
        let consumed = f32::from(whole_steps) * self.substep;
        let tail = (dt - consumed).min(self.substep);
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
pub(crate) const FRAME_SECS: f32 = 1.0 / 60.0;

#[cfg(test)]
pub(crate) const SETTLE_FRAMES: u32 = 600;

#[cfg(test)]
#[path = "spring_tests.rs"]
mod tests;
