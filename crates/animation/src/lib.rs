//! Animation primitives — springs, easing curves, time-based tweens.
//!
//! `sigil-animation` is pure math: it doesn't own a frame loop, doesn't
//! touch wgpu, and doesn't drive widgets by itself. Consumers (sigil-app,
//! widgets, downstream apps) feed `dt` into a [`Spring`] or [`Tween`] each
//! frame and read the next value out.
//!
//! Three primitives ship today:
//!
//! - [`Spring`] — Hooke's law spring. Stiffness, damping, mass; advance
//!   with [`Spring::step`]. The current value chases a target and settles
//!   when both position error and velocity fall below an epsilon.
//! - [`Easing`] — named curves (linear, `ease_in/out/in_out` variants for
//!   sine, quad, cubic, expo, plus a back-overshoot and a bounce). Apply
//!   to a normalized `t ∈ [0, 1]` to bend it.
//! - [`Tween`] — fixed-duration interpolation between two `f32` endpoints
//!   under an [`Easing`]. Advance with [`Tween::step`] and read the
//!   current value.
//!
//! Future work: vector-valued springs (Vec2/Vec3), `Sequence` for
//! choreographed timelines, and a `Signal`-driving animator that ticks
//! with the frame loop. Logged as follow-ups in the host engine roadmap.

#![deny(missing_docs)]
mod easing;
mod spring;
mod tween;

pub use easing::Easing;
pub use spring::{Spring, SpringPreset};
pub use tween::Tween;
