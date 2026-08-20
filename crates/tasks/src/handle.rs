//! `OwnerHandle` — the `Send + Sync` weak reference to a reactive owner.
//!
//! Background tasks carry an `OwnerHandle`, never an `Owner`. The
//! handle is identity-only: it does not grant reactive access on its
//! own. Reactive ops still have to run on the main thread via
//! [`dispatch_to_owner`][crate::dispatch_to_owner] (or
//! [`spawn_to_signal`][crate::spawn_to_signal], which is built on it).
//!
//! See DR-005 addendum §3.

use sigil_reactive::OwnerId;

/// An identity-only, `Send + Sync` weak reference to a reactive
/// [`Owner`][sigil_reactive::Owner].
///
/// Constructed from an `OwnerId` (see [`OwnerHandle::from_id`] or
/// [`OwnerHandle::current`]). Holding one grants **no** reactive
/// access on its own; background code uses the handle as a target
/// for [`dispatch_to_owner`][crate::dispatch_to_owner], and the
/// dispatcher performs the liveness check on the main thread at
/// drain time. An `OwnerHandle` whose owner has been disposed is a
/// no-op target, never a panic.
///
/// # Thread safety
///
/// The underlying [`OwnerId`] is a generational `(index, generation)`
/// key (see [`sigil_reactive`] arena). Copying it across threads is
/// always safe; resolving it touches the thread-local reactive
/// runtime and must therefore happen on the main thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OwnerHandle {
    id: OwnerId,
}

impl OwnerHandle {
    /// Wrap an existing [`OwnerId`] as a handle.
    ///
    /// Called from the main thread once, typically at the point
    /// where a background task is about to be spawned:
    /// `OwnerHandle::from_id(Owner::current().expect(...))`.
    #[must_use]
    pub const fn from_id(id: OwnerId) -> Self {
        Self { id }
    }

    /// Capture the currently-active reactive owner as a handle.
    ///
    /// Returns `None` when called outside any reactive scope. Must
    /// be called on the thread that owns the reactive runtime.
    #[must_use]
    pub fn current() -> Option<Self> {
        sigil_reactive::Owner::current().map(Self::from_id)
    }

    /// The underlying [`OwnerId`].
    #[must_use]
    pub const fn id(self) -> OwnerId {
        self.id
    }

    /// Return `true` if the referenced owner is still live in the
    /// reactive runtime.
    ///
    /// Must be called on the thread that owns the reactive runtime.
    /// Background code should not use this directly — it is the
    /// check the dispatcher performs at drain time on the caller's
    /// behalf. Exposed publicly so tests and higher-level helpers
    /// can reuse the same check without a second dispatch round-trip.
    #[must_use]
    pub fn is_alive(&self) -> bool {
        sigil_reactive::is_live(self.id)
    }
}

// Compile-time guarantee that the handle satisfies the
// Scheduler::OwnerHandle bound. If OwnerId ever loses its Send/Sync
// auto-impl the build breaks here with a clear message instead of at
// every call site.
const _: fn() = || {
    const fn assert_send_sync_clone_static<T: Send + Sync + Clone + 'static>() {}
    assert_send_sync_clone_static::<OwnerHandle>();
};
