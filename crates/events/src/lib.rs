//! Event routing, hit-testing, focus, and builder ergonomics for Sigil.
//!
//! `sigil-events` sits above shell delivery and below widgets. It
//! receives translated [`hal::ShellEvent`] values, finds the
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
use reactive as _;
#[cfg(test)]
use style as _;
mod builder_ext;
mod focus;
mod handlers;
mod hit_test;
mod pointer_dispatch;
pub mod prelude;
mod router;

pub use builder_ext::NodeEventsExt;
pub use focus::{FocusIntent, focus_from_keyboard, note_focus_source};
pub use handlers::EventHandlers;
pub use router::EventRouter;
