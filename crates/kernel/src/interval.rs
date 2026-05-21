//! `Cx::interval` — periodic callback API on the composition context.
//!
//! This is the kernel-side ergonomic entry point. The actual interval
//! registry lives in
//! [`reactive::interval`](reactive::register_interval) — see
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

use reactive::{IntervalId, register_interval};

use crate::Cx;

impl Cx {
    /// Register a periodic callback owned by the current component scope.
    ///
    /// The callback fires every `period` once the run loop drives
    /// [`reactive::tick_intervals`]. The first fire occurs at
    /// `now + period`; intervals never fire immediately on registration.
    ///
    /// When the surrounding component is removed from the tree, the
    /// interval is cancelled automatically via the reactive owner's
    /// cleanup chain — apps never need to track ids.
    ///
    /// Returns the [`IntervalId`] for explicit early cancellation via
    /// [`reactive::cancel_interval`].
    ///
    /// # Panics
    ///
    /// Panics if called outside any reactive scope (i.e., outside an
    /// [`IntoNode::build`](crate::IntoNode::build) implementation or
    /// the [`Runtime::scope`](reactive::Runtime::scope) entry
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use core::cell::Cell;
    use std::rc::Rc;
    use std::time::Instant;

    use reactive::{
        __test_advance_clock, __test_clear_clock, __test_set_clock, Runtime, tick_intervals,
    };

    use super::*;

    #[test]
    fn cx_interval_fires_via_tick() {
        Runtime::scope("cx_interval::fires", || {
            __test_set_clock(Instant::now());
            let cx = Cx::new_root();
            let count = Rc::new(Cell::new(0_u32));
            let writer = Rc::clone(&count);
            cx.interval(Duration::from_millis(100), move || {
                writer.set(writer.get() + 1);
            });

            for _ in 0..3 {
                __test_advance_clock(Duration::from_millis(100));
                let _ = tick_intervals();
            }
            assert_eq!(count.get(), 3);
            __test_clear_clock();
        });
    }
}
