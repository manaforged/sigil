//! Floating combat text — particle-style numbers that bypass layout.
//!
//! Each spawn pushes a particle into a fixed-capacity pool
//! ([`state`]). A reactive tick advances ages and evicts expired
//! particles. Positions are stored directly on the pool (no layout
//! round-trip), so a downstream renderer can read
//! [`FctState::snapshot`] each frame and emit scene commands without
//! invoking the layout engine.
//!
//! Design intent: the "500 damage numbers per second" use case for
//! the host engine. Per-frame cost is dominated by the simulation step (linear
//! over active particles); the widget itself never recomputes layout.
//!
//! [`FloatingTextBuilder::at`] anchors a spawned particle to a
//! caller-provided screen-position signal (e.g. a unit's projected
//! world position) so the floater follows the unit while it lives.
//! [`FloatingTextBuilder::style`] picks a semantic [`FloaterStyle`]
//! (Damage / Heal / Miss) instead of an ad-hoc RGBA per spawn — sigil
//! stays projection- and color-agnostic: the caller supplies the
//! screen coordinate, sigil supplies the theme-consistent color.
//!
//! See the strategic plan §F6.
//!
//! # Hygiene-exception - `Signal<f32>` time input
//!
//! A particle system needs a clock; this implementation accepts one via
//! [`FloatingTextBuilder::time`] (caller-driven seconds-since-start).
//! This keeps the widget agnostic to whatever frame loop the caller runs
//! and makes the bench measurable. If no clock is supplied, the builder
//! installs an inert `0.0` signal.
//!
//! # Hygiene-exception — `Signal<FctSpawn>` edge semantics
//!
//! The spawn signal is treated as edge-triggered: each `set()` triggers
//! the spawn effect, which reads the current value and pushes one
//! particle. Holding the same value is a no-op — callers spawn N
//! particles by calling `spawn.set(...)` N times.

mod builder;
mod spawn;
mod state;
mod style;

pub use builder::{FctNode, FloatingTextBuilder, floating_text};
pub use spawn::{FctSpawn, Particle};
pub use state::{FCT_POOL_CAPACITY, FctState};
pub use style::FloaterStyle;

#[cfg(test)]
#[path = "floating_text_tests.rs"]
mod tests;
