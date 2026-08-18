//! G5 — `AsyncContext<T>` re-entry token + executor install hooks
//! (DR-048 §5).
//!
//! `AsyncContext<T>` is the `Send + Sync + Clone + 'static` companion
//! to [`crate::EntityCx`] handed to the future body inside
//! [`crate::EntityCx::spawn`]. It cannot touch reactive state directly —
//! the reactive runtime is `!Send`. Instead, [`AsyncContext::update`]
//! posts a closure through the installed [`MainThreadDispatcher`] that
//! re-enters the reactive context on the main thread and signals
//! completion via a oneshot back-channel (DR-048 §5).
//!
//! ## Executor install
//!
//! Sigil's reactive crate cannot depend on `sigil-async` (would be
//! circular). Instead, two small object-safe traits — [`TaskExecutor`]
//! and [`MainThreadDispatcher`] — abstract the executor's spawn and
//! main-thread enqueue paths. The application installs concrete
//! impls (in production: an `AsyncRuntime`-backed pair) via
//! [`crate::Runtime::install_executor`] at startup, and tests install
//! their own deterministic pair.
//!
//! See DR-048 §5 + the implementation seams section.

use core::any::Any;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use std::sync::Arc;

use tokio::sync::oneshot;
use tokio::task::AbortHandle;

use crate::entity::Entity;
use crate::owner::OwnerId;
use crate::runtime::{EntityKey, with_rt};
use crate::task::EntityGone;

/// A type-erased, `Send` future ready to be spawned by a
/// [`TaskExecutor`].
///
/// The executor receives a boxed future and is free to run it on
/// whatever worker pool it owns. The future's output is `()`; the
/// real output `T` is shuttled back through a oneshot the caller
/// constructed and embedded in the future body.
pub type BoxSendFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// A type-erased, `Send` closure scheduled by a
/// [`MainThreadDispatcher`] for execution on the main thread.
///
/// The dispatcher's job is to ferry the closure across the thread
/// boundary; running the closure happens later, during the main
/// thread's drain cycle. Implementations forward to the same MPSC
/// channel that `sigil-async::Dispatcher::send` uses.
pub type BoxMainCallback = Box<dyn FnOnce() + Send + 'static>;

/// Object-safe executor abstraction installed via
/// [`crate::Runtime::install_executor`].
///
/// Production install: `sigil-async::AsyncRuntime` (Tokio-backed).
/// Tests install a runtime with a single multi-thread Tokio Handle
/// captured at fixture init.
///
/// The trait deliberately stays narrow — `spawn` is the
/// only spawn primitive the reactive crate needs.
pub trait TaskExecutor: Send + Sync + 'static {
    /// Spawn a `Send + 'static` future on the executor.
    ///
    /// Returns an [`AbortHandle`] the caller stores in [`crate::Task`]
    /// for RAII cancel-on-drop. Implementations must also register
    /// the abort handle against `owner` so that owner-cascade disposal
    /// fires the abort (DR-048 §3 layer 3).
    fn spawn(&self, owner: OwnerId, fut: BoxSendFuture) -> AbortHandle;
}

/// Reason a [`MainThreadDispatcher::post`] call failed.
///
/// Today the only variant fires when the dispatcher's MPSC has been
/// torn down (runtime shutting down). Listed as an enum so adapters
/// that grow bounded-queue support (DR-007 invariant 5) can add a
/// `Full` variant without a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchError {
    /// The dispatcher's main-thread MPSC channel is closed.
    Closed,
}

impl core::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Closed => f.write_str("main-thread dispatcher is closed"),
        }
    }
}

impl std::error::Error for DispatchError {}

/// Object-safe main-thread dispatcher abstraction.
///
/// Implementations push the closure onto a MPSC channel drained by the
/// main thread; the closure runs under `owner`'s reactive scope at
/// drain time. If `owner` has been disposed by then, the closure is
/// dropped silently (DR-007 invariant 4).
pub trait MainThreadDispatcher: Send + Sync + 'static {
    /// Post `callback` for execution on the main thread under `owner`.
    ///
    /// # Errors
    ///
    /// Returns [`DispatchError::Closed`] if the dispatcher's queue
    /// has been torn down (runtime shutting down). The callback is
    /// dropped without running in that case.
    fn post(&self, owner: OwnerId, callback: BoxMainCallback) -> Result<(), DispatchError>;
}

/// `Send + Sync + Clone` re-entry token handed to spawned futures
/// (DR-048 §5).
///
/// Holds the spawning entity's identity (as a type-erased `EntityKey`),
/// the owner the spawn was parented to, and a clone of the installed
/// main-thread dispatcher. Cloning is cheap: one `Arc::clone` on the
/// dispatcher.
///
/// `AsyncContext<T>` is `Send + Sync + Clone + 'static` — these bounds
/// are verified at compile time in the const block at the bottom of
/// the module.
pub struct AsyncContext<T: 'static> {
    pub(crate) entity_key: EntityKey,
    pub(crate) owner: OwnerId,
    pub(crate) dispatcher: Arc<dyn MainThreadDispatcher>,
    _t: PhantomData<fn() -> T>,
}

impl<T: 'static> Clone for AsyncContext<T> {
    fn clone(&self) -> Self {
        Self {
            entity_key: self.entity_key,
            owner: self.owner,
            dispatcher: Arc::clone(&self.dispatcher),
            _t: PhantomData,
        }
    }
}

impl<T: 'static> core::fmt::Debug for AsyncContext<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AsyncContext")
            .field("entity", &self.entity_key)
            .field("owner", &self.owner)
            .finish_non_exhaustive()
    }
}

impl<T: 'static> AsyncContext<T> {
    /// `pub(crate)` constructor — only [`crate::EntityCx::spawn`]
    /// produces an `AsyncContext`.
    #[allow(
        dead_code,
        reason = "consumed by EntityCx::spawn in the same DR-048 landing — module landing order"
    )]
    pub(crate) fn new(
        entity_key: EntityKey,
        owner: OwnerId,
        dispatcher: Arc<dyn MainThreadDispatcher>,
    ) -> Self {
        Self {
            entity_key,
            owner,
            dispatcher,
            _t: PhantomData,
        }
    }

    /// Mutate the spawning entity from a background future.
    ///
    /// Posts `f` to the main thread, where it runs inside an
    /// `Entity::update` against the spawning entity. The future
    /// `await`s the result through a oneshot back-channel.
    ///
    /// # Errors
    ///
    /// Returns `Err(EntityGone)` if the entity (or its owner) was
    /// disposed between the spawn and the update attempt, or if the
    /// dispatcher's MPSC was closed (runtime tearing down).
    pub async fn update<R, F>(&self, f: F) -> Result<R, EntityGone>
    where
        F: FnOnce(&mut T, &mut crate::EntityCx<'_, T>) -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel::<Result<R, EntityGone>>();
        let entity_key = self.entity_key;
        let callback: BoxMainCallback = Box::new(move || {
            let outcome = run_update_on_main::<T, R>(entity_key, f);
            // Receiver was dropped if the future was cancelled mid-flight;
            // ignoring the send error is the correct path.
            let _ = tx.send(outcome);
        });
        if self.dispatcher.post(self.owner, callback).is_err() {
            return Err(EntityGone);
        }
        rx.await.unwrap_or(Err(EntityGone))
    }

    /// Mutate a peer entity from a background future.
    ///
    /// Same trampoline shape as [`AsyncContext::update`] but for an
    /// entity other than the spawning one. The peer's identity is
    /// captured by `Entity<V>`; the peer's owner is **not** consulted
    /// for dispatch — the closure runs under the spawning entity's
    /// owner (so observers fired by the closure parent against the
    /// spawning scope). `EntityGone` is returned if `entity` itself
    /// was disposed.
    ///
    /// # Errors
    ///
    /// Returns `Err(EntityGone)` if `entity` (or the spawning owner)
    /// was disposed, or if the dispatcher's MPSC was closed.
    pub fn update_peer<V, R, F>(
        &self,
        entity: &Entity<V>,
        f: F,
    ) -> impl core::future::Future<Output = Result<R, EntityGone>> + Send + 'static
    where
        V: 'static,
        F: FnOnce(&mut V, &mut crate::EntityCx<'_, V>) -> R + Send + 'static,
        R: Send + 'static,
    {
        // Snapshot the peer entity's identity BEFORE the `async move`
        // so we never capture `&Entity<V>` across the await — `Entity<V>`
        // is !Sync and the captured borrow would poison the future's
        // Send-ness. The EntityKey itself is Copy + Send + Sync.
        let peer_key = entity.key;
        let owner = self.owner;
        let dispatcher = Arc::clone(&self.dispatcher);
        async move {
            let (tx, rx) = oneshot::channel::<Result<R, EntityGone>>();
            let callback: BoxMainCallback = Box::new(move || {
                let outcome = run_update_on_main::<V, R>(peer_key, f);
                let _ = tx.send(outcome);
            });
            if dispatcher.post(owner, callback).is_err() {
                return Err(EntityGone);
            }
            rx.await.unwrap_or(Err(EntityGone))
        }
    }

    /// Read-only access to the spawning entity from a background
    /// future.
    ///
    /// Same trampoline shape as [`AsyncContext::update`] without a
    /// `EntityCx` (no mutation). The closure receives `&T`.
    ///
    /// # Errors
    ///
    /// Returns `Err(EntityGone)` if the entity (or its owner) was
    /// disposed, or if the dispatcher's MPSC was closed.
    pub async fn read<R, F>(&self, f: F) -> Result<R, EntityGone>
    where
        F: FnOnce(&T) -> R + Send + 'static,
        R: Send + 'static,
    {
        let (tx, rx) = oneshot::channel::<Result<R, EntityGone>>();
        let entity_key = self.entity_key;
        let callback: BoxMainCallback = Box::new(move || {
            let outcome = run_read_on_main::<T, R>(entity_key, f);
            let _ = tx.send(outcome);
        });
        if self.dispatcher.post(self.owner, callback).is_err() {
            return Err(EntityGone);
        }
        rx.await.unwrap_or(Err(EntityGone))
    }
}

/// Helper executed on the main thread inside an enqueued callback.
///
/// Performs the entity-alive check + downcast inside the runtime
/// borrow, then drops the borrow before invoking `f` (which itself may
/// call `with_rt` re-entrantly via `EntityCx` methods).
fn run_update_on_main<U: 'static, R>(
    key: EntityKey,
    f: impl FnOnce(&mut U, &mut crate::EntityCx<'_, U>) -> R + Send + 'static,
) -> Result<R, EntityGone> {
    // Liveness check. If the slot is gone, the entity (or its owner)
    // was disposed between spawn and dispatch.
    let alive = with_rt(|rt| rt.entities.contains(key));
    if !alive {
        return Err(EntityGone);
    }
    // Re-enter the entity via the public update path. `run_update` is
    // pub(crate); we duplicate the type-erased dispatch by reading the
    // payload's TypeId-checked downcast inline.
    let result = crate::entity::run_update::<U, R>(key, core::panic::Location::caller(), f);
    Ok(result)
}

/// Helper executed on the main thread for `AsyncContext::read`.
///
/// Performs the entity-alive check + downcast, calls `f`, restores the
/// slot. Read does not lock the slot beyond the closure body.
fn run_read_on_main<U: 'static, R>(
    key: EntityKey,
    f: impl FnOnce(&U) -> R + Send + 'static,
) -> Result<R, EntityGone> {
    // Take the value out so user code cannot trigger a re-entrant
    // borrow through with_rt — same pattern as Entity::read.
    let value: Box<dyn Any> = match with_rt(|rt| {
        rt.entities
            .get_mut(key)
            .map(|slot| core::mem::replace(&mut slot.value, Box::new(())) as Box<dyn Any>)
    }) {
        Some(v) => v,
        None => return Err(EntityGone),
    };
    // RAII restore so a panicking `f` does not leak the placeholder.
    let mut guard = ReadRestoreGuard {
        key,
        value: Some(value),
    };
    let result = {
        let inner = guard
            .value
            .as_ref()
            .expect("populated by guard ctor")
            .downcast_ref::<U>()
            .expect("entity type mismatch in AsyncContext::read");
        f(inner)
    };
    let restored = guard.value.take().expect("value present");
    with_rt(|rt| {
        if let Some(slot) = rt.entities.get_mut(key) {
            slot.value = restored;
        }
    });
    Ok(result)
}

/// RAII restore guard for [`run_read_on_main`] — puts the value back
/// on unwind so a panicking `f` does not leak the placeholder.
struct ReadRestoreGuard {
    key: EntityKey,
    value: Option<Box<dyn Any>>,
}

impl Drop for ReadRestoreGuard {
    fn drop(&mut self) {
        if let Some(v) = self.value.take() {
            with_rt(|rt| {
                if let Some(slot) = rt.entities.get_mut(self.key) {
                    slot.value = v;
                }
            });
        }
    }
}

// Compile-time guarantees for the public surface.
const _: fn() = || {
    const fn assert_send<T: Send>() {}
    const fn assert_sync<T: Sync>() {}
    const fn assert_clone<T: Clone>() {}
    const fn assert_static<T: 'static>() {}
    assert_send::<AsyncContext<()>>();
    assert_sync::<AsyncContext<()>>();
    assert_clone::<AsyncContext<()>>();
    assert_static::<AsyncContext<()>>();
};
