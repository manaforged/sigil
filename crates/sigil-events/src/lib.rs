//! Event routing, hit-testing, focus, and builder ergonomics for Sigil.
//!
//! `sigil-events` sits above shell delivery and below widgets. It
//! receives translated [`sigil_hal::ShellEvent`] values, finds the
//! target node from layout geometry, runs capture/target/bubble
//! handlers, and maintains keyboard focus.

#![allow(clippy::redundant_pub_crate)]

// `criterion` is a dev-dep used only by the `events_ops` bench. Cargo
// pulls dev-deps into the lib-test target, so keep the dependency
// visible to the unused-crate-dependencies lint.
#[cfg(test)]
use allocation_counter as _;
#[cfg(test)]
use criterion as _;
#[cfg(test)]
use sigil_reactive as _;
#[cfg(test)]
use sigil_style as _;
#[cfg(test)]
use taffy as _;

mod builder_ext;
mod focus;
mod handlers;
mod hit_test;
pub mod prelude;
mod router;

pub use builder_ext::NodeEventsExt;
pub use handlers::EventHandlers;
pub use router::EventRouter;
