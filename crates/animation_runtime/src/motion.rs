//! Reduced-motion gate (DR-053 §6).
//!
//! Phase 1 is a per-thread cell consulted by [`Animator`](crate::Animator)
//! constructors. Phase 2 will surface the AccessKit
//! `prefers_reduced_motion` signal from DR-050 phase 2 — at that point
//! [`MotionPrefs::reduced`] will read from the installed bridge and
//! the override here becomes a test-only escape hatch.

use core::cell::Cell;

thread_local! {
    static REDUCED_MOTION: Cell<bool> = const { Cell::new(false) };
}

/// Reduced-motion preference accessor.
///
/// Namespaced façade over the per-thread reduced-motion cell. Phase 1
/// reads/writes the thread-local; phase 2 will read from the installed
/// AccessKit bridge and treat [`MotionPrefs::set`] as a test-only
/// override.
pub struct MotionPrefs;

impl MotionPrefs {
    /// True when the user prefers reduced motion. Phase 1 returns the
    /// value most recently set via [`MotionPrefs::set`]; the default is
    /// `false`.
    #[must_use]
    pub fn reduced() -> bool {
        REDUCED_MOTION.with(Cell::get)
    }

    /// Override the reduced-motion preference for the current thread.
    /// Tests use this to exercise both branches; production code wires
    /// the AccessKit signal in DR-050 phase 2.
    pub fn set(preferred: bool) {
        REDUCED_MOTION.with(|cell| cell.set(preferred));
    }
}
