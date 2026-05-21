//! Cross-thread wake handle for the winit adapter.
//!
//! `WinitWaker` wraps the underlying `EventLoopProxy<WakeEvent>` in an
//! `Arc<Mutex<..>>` so the handle is `Send + Sync` on every platform
//! without `unsafe`. The mutex is uncontended — `EventLoopProxy` is
//! internally thread-safe in winit 0.30, but only declares `Sync` on
//! macOS/iOS, so wrapping it is how we lift `Sync` uniformly.
//!
//! # Coalescing contract
//!
//! The HAL contract (see [`hal::Shell::Waker`]) requires that
//! multiple `wake()` calls between two event-loop iterations collapse
//! into a single delivered wake. An `Arc<AtomicBool>` "pending wake"
//! flag is shared between the waker and the application handler:
//!
//! - `WinitWaker::wake` attempts a
//!   `compare_exchange(false, true, Acquire, Relaxed)`. Only the
//!   false→true transition actually forwards a `WakeEvent` through the
//!   proxy; subsequent calls observe the flag set and return `Ok(())`
//!   without touching the proxy.
//! - The shell's `user_event` handler stores `false` with `Release`
//!   ordering after the wake hook has drained. This pairs with the
//!   `Acquire` on the next wake so that any enqueue-before-wake is
//!   visible to the next drain.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use winit::event_loop::EventLoopProxy;

use crate::ShellError;

/// Test-only counter that increments every time [`WinitWaker::wake`]
/// transitions the coalescing flag from `false` to `true` and
/// therefore actually forwards a `WakeEvent` through the proxy.
///
/// Exposed as `#[doc(hidden)] pub` (instead of gated on `cfg(test)`)
/// so that integration tests in `tests/shell_contract.rs` — which
/// compile against the library as an external crate — can observe it.
/// The static is public API only in the strictly technical sense; it
/// is not part of the supported surface and must not be relied on by
/// consumers.
#[doc(hidden)]
pub static WAKE_DELIVERED_COUNT: AtomicU64 = AtomicU64::new(0);

/// The single user-event payload the adapter sends through
/// `EventLoopProxy::send_event`.
///
/// Kept as a unit struct on purpose: the proxy coalesces *intent*
/// (one or more `WakeEvent`s collapse into one delivered `user_event`
/// callback), and the payload — `SignalUpdate`s enqueued by
/// `sigil-async` — travels through an MPSC channel rather than through
/// the event loop. See DR-002 and DR-007.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub struct WakeEvent;

/// Cloneable, thread-safe wake handle for [`crate::WinitShell`].
///
/// Produced by `WinitShell::waker` and consumed by `sigil-async` (or
/// anything else that needs to signal the main thread).
#[derive(Clone)]
pub struct WinitWaker {
    pub(crate) proxy: Arc<Mutex<EventLoopProxy<WakeEvent>>>,
    pub(crate) pending: Arc<AtomicBool>,
    pub(crate) alive: Arc<AtomicBool>,
}

impl WinitWaker {
    /// Signal the event loop.
    ///
    /// Coalesces: only the `false`→`true` transition of the shared
    /// pending flag actually forwards a `WakeEvent` through the
    /// proxy. Subsequent calls before the next `user_event` tick
    /// return `Ok(())` without touching the proxy. See the module
    /// docs for the ordering rationale.
    ///
    /// # Errors
    ///
    /// Returns [`ShellError::EventLoopClosed`] if the shell has
    /// already been dropped or the event loop is no longer alive.
    pub fn wake(&self) -> Result<(), ShellError> {
        // Liveness gate first: the shell flips `alive` to false in
        // its `Drop` impl. Without this check, a stale pending bit
        // could mask a dead event loop indefinitely.
        if !self.alive.load(Ordering::Acquire) {
            return Err(ShellError::EventLoopClosed);
        }

        // Acquire on success pairs with the Release store in
        // `user_event` after the wake hook has drained: any MPSC
        // enqueue that happens-before our CAS is visible to the hook
        // that will process the wake we're about to deliver.
        if self
            .pending
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Another call already scheduled a wake that has not yet
            // been delivered; collapse into it.
            return Ok(());
        }

        let proxy = self.proxy.lock().map_err(|_| ShellError::WakerPoisoned)?;
        if proxy.send_event(WakeEvent).is_ok() {
            WAKE_DELIVERED_COUNT.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            // The event loop died between the liveness check and the
            // send (or never saw the alive flag flip). Reset the
            // pending flag so future calls don't silently coalesce,
            // and flip `alive` so subsequent wakes short-circuit.
            self.pending.store(false, Ordering::Release);
            self.alive.store(false, Ordering::Release);
            Err(ShellError::EventLoopClosed)
        }
    }
}

impl core::fmt::Debug for WinitWaker {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WinitWaker").finish_non_exhaustive()
    }
}
