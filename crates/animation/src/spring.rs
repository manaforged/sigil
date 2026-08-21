//! Hooke's-law spring with damping and mass.
//!
//! [`Spring`] holds three state values — current position, velocity,
//! and target — and exposes [`Spring::step`] to advance them one frame
//! at delta-time `dt`. Integration uses **semi-implicit Euler** because
//! it is stable for stiff systems where explicit Euler explodes; the
//! cost is one extra add per step over the explicit form.
//!
//! Force on the mass each tick:
//!
//! ```text
//! F_spring = -stiffness * (position - target)
//! F_damper = -damping  * velocity
//! a        = (F_spring + F_damper) / mass
//! ```
//!
//! Then:
//!
//! ```text
//! velocity_next = velocity + a * dt
//! position_next = position + velocity_next * dt
//! ```
//!
//! [`Spring::is_settled`] checks both error and velocity against an
//! epsilon — a spring that's *near* its target but still moving fast
//! is not settled and should keep ticking.

/// A handful of named spring shapes calibrated for UI motion. These are
/// not magical — they are the values UI designers reach for first.
/// Custom springs use [`Spring::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpringPreset {
    /// Loose and slow — exploratory feel, ~0.8 s settle.
    Gentle,
    /// Default desktop UI feel — ~0.4 s settle, no perceptible overshoot.
    Smooth,
    /// Snappy click feedback — ~0.25 s settle, minimal overshoot.
    Snappy,
    /// Playful overshoot — visible bounce before settling.
    Bouncy,
    /// Stiff and instant — sub-frame settle on small deltas.
    Stiff,
}

impl SpringPreset {
    /// `(stiffness, damping, mass)` for this preset.
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

/// Hooke's-law spring with damping and mass.
///
/// Use [`Spring::new`] for custom physics, [`Spring::from_preset`] for
/// the calibrated UI presets. Advance with [`Spring::step`] each frame;
/// read the new value via [`Spring::value`]; stop ticking when
/// [`Spring::is_settled`] returns `true`.
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
    /// Default value-error epsilon — appropriate for 0..1 progress
    /// signals. Pixel-space animations want a larger threshold; use
    /// [`Spring::with_epsilons`] to override.
    pub const DEFAULT_VALUE_EPSILON: f32 = 0.001;

    /// Default velocity epsilon — speed below this counts as "stopped"
    /// when paired with [`Self::DEFAULT_VALUE_EPSILON`].
    pub const DEFAULT_VELOCITY_EPSILON: f32 = 0.001;

    /// Construct a spring starting at `initial`, with `stiffness`,
    /// `damping`, and `mass` set explicitly. Settle thresholds default
    /// to [`Self::DEFAULT_VALUE_EPSILON`] /
    /// [`Self::DEFAULT_VELOCITY_EPSILON`]; override with
    /// [`Self::with_epsilons`].
    ///
    /// # Panics
    ///
    /// Panics if `mass` is non-positive — a zero or negative mass makes
    /// the integrator divide by zero.
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

    /// Override the value-error and velocity epsilons used by
    /// [`Self::is_settled`]. Pixel-space animations typically want
    /// `0.5` (sub-pixel) for both; progress signals in `0..1` can keep
    /// the defaults; physics-driven layout wants tighter values.
    ///
    /// Builder-style: consumes `self`.
    ///
    /// # Panics
    ///
    /// Panics if either epsilon is non-positive — a non-positive
    /// threshold would prevent [`Self::is_settled`] from ever
    /// returning `true`.
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

    /// Construct a spring from one of the calibrated UI presets.
    /// Epsilons default to the same values as [`Self::new`]; chain
    /// [`Self::with_epsilons`] to override.
    #[must_use]
    pub fn from_preset(initial: f32, preset: SpringPreset) -> Self {
        let (stiffness, damping, mass) = preset.parameters();
        Self::new(initial, stiffness, damping, mass)
    }

    /// Set the target value the spring chases. The spring may still be
    /// moving when this is called — it just re-aims.
    pub const fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Replace value, velocity, and target all at once. Useful when the
    /// caller wants to teleport the animation without a transition
    /// (e.g. when a scene swap nukes the previous state).
    pub const fn reset(&mut self, value: f32) {
        self.value = value;
        self.velocity = 0.0;
        self.target = value;
    }

    /// Current position of the spring's mass.
    #[must_use]
    pub const fn value(&self) -> f32 {
        self.value
    }

    /// Current velocity. Useful when chaining a second animation off
    /// the first's tail momentum.
    #[must_use]
    pub const fn velocity(&self) -> f32 {
        self.velocity
    }

    /// Target the spring is heading toward.
    #[must_use]
    pub const fn target(&self) -> f32 {
        self.target
    }

    /// Returns `true` when both position error and velocity are below
    /// the spring's configured epsilons. Stop ticking the spring at
    /// that point.
    #[must_use]
    pub fn is_settled(&self) -> bool {
        (self.value - self.target).abs() < self.value_epsilon
            && self.velocity.abs() < self.velocity_epsilon
    }

    /// Maximum integration sub-step. A `step(dt)` larger than this is
    /// internally split into multiple sub-steps so a long frame hitch
    /// (alt-tab, GC pause, debugger break) does not let a stiff spring
    /// diverge. 1/120 s keeps even `Stiff` (`k=500`) inside the stable
    /// region of semi-implicit Euler.
    const MAX_SUBSTEP_DT: f32 = 1.0 / 120.0;

    /// Hard upper bound on substeps per [`Self::step`] call. Past this
    /// many missed frames the animation has visibly skipped anyway; we
    /// snap to the integrated state and stop. ~2 s of work at the cap.
    const MAX_SUBSTEPS: u16 = 256;

    /// Advance the spring one frame at delta-time `dt`. Reading the
    /// state afterwards yields the new value/velocity. Calling `step`
    /// after [`Self::is_settled`] returns `true` is a no-op (the spring
    /// snaps to the target and freezes).
    ///
    /// Sub-steps internally at `Self::MAX_SUBSTEP_DT` so a long frame
    /// (hitch, debugger break) cannot push the integrator outside its
    /// stable region.
    pub fn step(&mut self, dt: f32) {
        if self.is_settled() {
            self.value = self.target;
            self.velocity = 0.0;
            return;
        }
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        // Bound the work even if `dt` is astronomically large (debugger
        // pause, suspended tab) — past `MAX_SUBSTEPS` substeps the
        // animation has visibly skipped anyway, snap to the target.
        let raw_steps = (dt / Self::MAX_SUBSTEP_DT).floor();
        let clamped = raw_steps.clamp(0.0, f32::from(Self::MAX_SUBSTEPS));
        // `clamped` is already in `[0, MAX_SUBSTEPS]` (a small finite
        // u16-representable integer); cast is bounded by construction.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Bounded by clamp() to [0, MAX_SUBSTEPS] above; never NaN/negative."
        )]
        let whole_steps = clamped as u16;
        for _ in 0..whole_steps {
            self.integrate(Self::MAX_SUBSTEP_DT);
            if self.is_settled() {
                self.value = self.target;
                self.velocity = 0.0;
                return;
            }
        }
        let consumed = f32::from(whole_steps) * Self::MAX_SUBSTEP_DT;
        let tail = (dt - consumed).min(Self::MAX_SUBSTEP_DT);
        if tail.is_sign_positive() && tail > f32::EPSILON {
            self.integrate(tail);
            if self.is_settled() {
                self.value = self.target;
                self.velocity = 0.0;
            }
        }
    }

    fn integrate(&mut self, dt: f32) {
        let spring_force = -self.stiffness * (self.value - self.target);
        let damper_force = -self.damping * self.velocity;
        let acceleration = (spring_force + damper_force) / self.mass;
        // Semi-implicit Euler — update velocity first, then use the new
        // velocity to update position. Stable for stiff systems where
        // the explicit form would diverge, provided dt is bounded.
        self.velocity = acceleration.mul_add(dt, self.velocity);
        self.value = self.velocity.mul_add(dt, self.value);
    }
}

#[cfg(test)]
#[path = "spring_tests.rs"]
mod tests;
