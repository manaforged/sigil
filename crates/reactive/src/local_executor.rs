//! `LocalTaskExecutor` — sibling of [`crate::TaskExecutor`] for `!Send`
//! futures (DR-048 §5 amendment, W4.99.B3).
//!
//! `EntityCx::spawn` requires `F: Send + 'static` because the producer
//! executor (Tokio multi-thread) moves the boxed future across worker
//! threads. Some authoring patterns — holding a `Rc<RefCell<_>>` across
//! `.await`, or returning a `!Send` value from the future body —
//! cannot satisfy that bound.
//!
//! The fix is a parallel install seam: a `LocalTaskExecutor` accepts a
//! `Pin<Box<dyn Future<Output = ()> + 'static>>` (no `Send` bound) and
//! drives it on the main thread. Production install is a
//! `tokio::task::LocalSet`-backed executor in `sigil-async`. Tests
//! install a deterministic fixture in the same shape as
//! `TaskExecutor`.
//!
//! See the DR-048 amendment B3 (2026-05-18) — the veteran fresh-eyes
//! audit flagged the missing `!Send` path as the "obvious miss" zone.

use core::future::Future;
use core::pin::Pin;

use crate::owner::OwnerId;
use tokio::task::AbortHandle;

/// A type-erased `!Send` future ready to be spawned by a
/// [`LocalTaskExecutor`].
///
/// Mirrors [`crate::BoxSendFuture`] but drops the `Send` bound — the
/// executor is expected to drive the future on a single thread (the
/// main thread, where the reactive runtime lives).
pub type BoxLocalFuture = Pin<Box<dyn Future<Output = ()> + 'static>>;

/// Object-safe `!Send` executor abstraction installed via
/// [`crate::Runtime::install_local_executor`].
///
/// Production install: a `tokio::task::LocalSet`-backed executor from
/// `sigil-async`. Tests install a deterministic fixture (an `Rc`-backed
/// queue drained inline).
///
/// Note the bounds: the trait itself is `'static` (so it can be erased
/// into `Rc<dyn LocalTaskExecutor>`), but it is NOT `Send + Sync`. A
/// local executor lives on the main thread alongside the reactive
/// runtime and never crosses threads.
pub trait LocalTaskExecutor: 'static {
    /// Spawn a `!Send` future on the local executor.
    ///
    /// Returns an [`AbortHandle`] the caller stores in
    /// [`crate::LocalTask`] for RAII cancel-on-drop. Owner-cascade
    /// disposal of the spawning entity also fires an `on_cleanup`
    /// registered against the spawning entity's owner (DR-048 §3
    /// layer 3) — the executor itself does not duplicate that
    /// tracking.
    fn spawn_local(&self, owner: OwnerId, fut: BoxLocalFuture) -> AbortHandle;
}
