#![deny(missing_docs)]
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
//! - [`config`] — lookup from [`sigil_style::TransitionId`] to
//!   [`AnimationConfig`].
//! - [`tick`] — per-animation-frame driver. Apps call during Effect,
//!   before layout. Walks the moving animator set and writes new values;
//!   settled handles stay registered so a later target can re-arm them.
//! - Reduced-motion gate — see [`MotionPrefs`]. Phase 1 is a
//!   thread-local override; phase 2 wires AccessKit's
//!   `prefers_reduced_motion` from DR-050.
//!
//! [DR-053]: ../../../docs/decisions/DR-053-g9-animation.md

// `criterion` is a dev-dep used by the `animator_ticks` bench. Mark it
// referenced under test+bench builds so the workspace's
// `unused_crate_dependencies` warning stays quiet on lib builds.
#[cfg(test)]
use criterion as _;

pub(crate) mod animator;
pub mod binding_animate;
pub(crate) mod binding_animate_color;
pub(crate) mod config;
pub(crate) mod motion;
pub(crate) mod registry;
pub(crate) mod state;

pub use animator::{Animator, AnimatorStatus};
pub use binding_animate::{
    ANIMATION_REGISTRY_MISS, AttachedAnimator, AttachedAnimators, AttachedColorAnimator,
    install_animators, install_animators_themed,
};
pub use binding_animate_color::{lerp_rgba8, linear_channel_to_srgb, srgb_channel_to_linear};
pub use config::AnimationConfig;
pub use motion::MotionPrefs;
pub use registry::{config, presets};
// W5.1b: `NumericAtomKind` moved into `sigil-style` for the
// `AnimatorOverlay` trait surface; re-export keeps the prior import
// path stable for crates depending on this runtime.
pub use sigil_style::NumericAtomKind;
pub use state::{active_count, tick};
