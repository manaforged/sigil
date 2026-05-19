// The crate is a thin set of cross-module helpers behind a small
// public surface. Many internal fns are crate-internal by design,
// which clippy's `redundant_pub_crate` nursery lint treats as noise.
// Same allow as sigil-reactive uses for the same reason.
#![allow(clippy::redundant_pub_crate)]

//! Animation orchestration over [`sigil_animation`] primitives.
//!
//! See [DR-053] for the locked design. Phase 1 ships:
//!
//! - [`Animator`] — owns a [`Signal<f32>`](sigil_reactive::Signal) and
//!   drives it via a wrapped [`Spring`](sigil_animation::Spring) or
//!   [`Tween`](sigil_animation::Tween).
//! - [`AnimationRegistry`] — `phf`-backed lookup from
//!   [`sigil_style::TransitionId`] to [`AnimationConfig`].
//! - [`tick`] — per-frame driver. Apps call once per frame between
//!   `Runtime::begin_layout` and `Runtime::begin_paint`. Walks the
//!   active animator set, writes new values, removes settled.
//! - Reduced-motion gate — see [`MotionPrefs`]. Phase 1 is a
//!   thread-local override; phase 2 wires AccessKit's
//!   `prefers_reduced_motion` from DR-050.
//!
//! [DR-053]: ../../../docs/decisions/DR-053-g9-animation.md

pub(crate) mod animator;
pub(crate) mod config;
pub(crate) mod motion;
pub(crate) mod registry;
pub(crate) mod state;

pub use animator::{Animator, AnimatorStatus};
pub use config::AnimationConfig;
pub use motion::MotionPrefs;
pub use registry::{AnimationRegistry, presets};
pub use state::tick;
