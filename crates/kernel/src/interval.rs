//! `Cx::interval` — periodic callback API on the composition context.
//!
//! This is the kernel-side ergonomic entry point. The actual interval
//! registry lives in
//! [`sigil_reactive::interval`](sigil_reactive::register_interval) — see
//! that module for the lifecycle / driver contract.
//!
//! ```text
//! use sigil::prelude::*;
//! use core::time::Duration;
//!
//! cx.interval(Duration::from_millis(50), || {
//!     elapsed.set(elapsed.get() + 1);
//! });
//! ```

use core::time::Duration;

use sigil_reactive::{IntervalId, register_interval};

use crate::Cx;

impl Cx {
    /// Register a periodic callback owned by the current component scope.
    ///
    /// The callback fires every `period` once the run loop drives
    /// [`sigil_reactive::tick_intervals`]. The first fire occurs at
    /// `now + period`; intervals never fire immediately on registration.
    ///
    /// When the surrounding component is removed from the tree, the
    /// interval is cancelled automatically via the reactive owner's
    /// cleanup chain — apps never need to track ids.
    ///
    /// Returns the [`IntervalId`] for explicit early cancellation via
    /// [`sigil_reactive::cancel_interval`].
    ///
    /// # Panics
    ///
    /// Panics if called outside any reactive scope (i.e., outside an
    /// [`IntoNode::build`](crate::IntoNode::build) implementation or
    /// the [`Runtime::scope`](sigil_reactive::Runtime::scope) entry
    /// point).
    #[track_caller]
    pub fn interval(&self, period: Duration, callback: impl Fn() + 'static) -> IntervalId {
        // `&self` (read-only) is intentional: registration touches the
        // reactive thread-local registry, not the kernel arena. Holding
        // `&self` keeps the call site composable with `Cx` borrows that
        // already exist in component build code.
        let _ = self;
        register_interval(period, callback)
    }
}

#[cfg(test)]
#[path = "interval_tests.rs"]
mod tests;
