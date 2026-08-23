//! Event routing, hit-testing, focus, and builder ergonomics for Sigil.
//!
//! `sigil-events` sits above shell delivery and below widgets. It
//! receives translated [`sigil_hal::ShellEvent`] values, finds the
//! target node from last-frame layout boxes, runs capture/target/bubble
//! handlers, and maintains keyboard focus.

#![allow(clippy::redundant_pub_crate)]
#![deny(missing_docs)]
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
mod builder_ext;
mod focus;
mod handlers;
mod hit_test;
mod ime_handlers;
mod pointer_dispatch;
pub mod prelude;
mod router;
mod window_command;

pub use builder_ext::NodeEventsExt;
pub use focus::{FocusIntent, focus_from_keyboard, note_focus_source, request_focus};
pub use handlers::EventHandlers;
pub use router::EventRouter;
pub use window_command::{request_window_command, take_window_commands};

/// Synthesize a click on `target` by node id.
///
/// The AT-initiated `accesskit::Action::Click` dispatch seam (DR-050
/// §4). Free function rather than an `EventRouter` method: the
/// dispatch is stateless (reads `cx`/`layout`, invokes handlers) and
/// doesn't touch router state, so callers (e.g. `sigil-app`'s a11y
/// action drain) don't need a router borrow for it.
pub fn invoke_node(
    cx: &sigil_kernel::Cx,
    layout: &sigil_layout::LayoutEngine,
    target: sigil_kernel::NodeId,
) {
    pointer_dispatch::invoke_node(cx, layout, target);
}
