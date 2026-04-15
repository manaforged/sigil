//! Owner tree — creation, disposal, and `on_cleanup`.
//!
//! Every signal, effect, and derived value is owned by the current
//! reactive owner. When an owner is disposed, its children are
//! disposed first (depth-first), then its cleanups run in LIFO
//! order, then its own signals and effects are invalidated. This
//! matches the Leptos 0.5+ and Solid.js contract. See DR-005.

use core::panic::Location;

use crate::runtime::{OwnerKey, OwnerSlot, with_rt};

/// RAII guard that restores the previous `current_owner` on drop,
/// whether the protected scope returned normally or panicked.
struct OwnerWithGuard {
    prev: Option<OwnerKey>,
}

impl Drop for OwnerWithGuard {
    fn drop(&mut self) {
        with_rt(|rt| rt.current_owner = self.prev);
    }
}

/// An opaque, generational identifier for a reactive owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OwnerId(pub(crate) OwnerKey);

/// A live owner handle, scoped to the main thread.
///
/// Holding an `Owner` lets the bearer create signals, effects, and
/// cleanups that belong to it. `Owner` is `!Send` and `!Sync` —
/// reactive state lives on one thread. Background threads should
/// hold `OwnerHandle` instead (see DR-005, Phase 1.1c).
#[derive(Debug, Clone, Copy)]
pub struct Owner {
    id: OwnerId,
    // !Send + !Sync
    _not_send: core::marker::PhantomData<*const ()>,
}

impl Owner {
    /// Create a new owner parented to `parent`.
    ///
    /// If `parent` is `None`, the new owner is a root. Roots do not
    /// have a parent and must be disposed explicitly; [`Runtime::scope`]
    /// is the usual way to create and dispose a root.
    ///
    /// # Panics
    ///
    /// Panics if `parent` is `Some(id)` for an already-disposed
    /// owner. Silently creating a detached owner would leak every
    /// signal, effect, and cleanup registered against the new owner
    /// because its parent is gone and cannot cascade disposal.
    ///
    /// [`Runtime::scope`]: crate::Runtime::scope
    #[must_use]
    #[track_caller]
    pub fn new(name: &'static str, parent: Option<OwnerId>) -> Self {
        let location = Location::caller();
        let parent_key = parent.map(|p| p.0);
        let key = with_rt(|rt| {
            // Validate parent is still alive BEFORE insertion.
            if let Some(pk) = parent_key {
                assert!(
                    rt.owners.contains(pk),
                    "Owner::new with a disposed parent (id={pk:?}, creating {name:?} at {location})",
                );
            }
            let key = rt.owners.insert(OwnerSlot {
                parent: parent_key,
                children: Vec::new(),
                signals: Vec::new(),
                effects: Vec::new(),
                cleanups: Vec::new(),
                name,
                location,
            });
            if let Some(pk) = parent_key
                && let Some(p) = rt.owners.get_mut(pk)
            {
                p.children.push(key);
            }
            key
        });
        Self {
            id: OwnerId(key),
            _not_send: core::marker::PhantomData,
        }
    }

    /// This owner's stable identity.
    #[must_use]
    pub const fn id(self) -> OwnerId {
        self.id
    }

    /// The currently active owner, if any.
    ///
    /// Returns `None` when called outside any reactive scope.
    #[must_use]
    pub fn current() -> Option<OwnerId> {
        with_rt(|rt| rt.current_owner.map(OwnerId))
    }

    /// Run `f` with this owner as the current reactive owner.
    ///
    /// Any signal, effect, or cleanup created during `f` is parented
    /// to this owner. Nested `with` calls stack — when `f` returns,
    /// the previous current owner is restored, even if `f` panics.
    ///
    /// # Panics
    ///
    /// Panics if this owner has already been disposed.
    #[track_caller]
    pub fn with<R>(self, f: impl FnOnce() -> R) -> R {
        let location = Location::caller();
        let prev = with_rt(|rt| {
            assert!(
                rt.owners.contains(self.id.0),
                "Owner::with on a disposed owner (id={:?}, at {location})",
                self.id.0,
            );
            let previous = rt.current_owner;
            rt.current_owner = Some(self.id.0);
            previous
        });
        // RAII guard restores `current_owner` on normal return and
        // on unwind.
        let _guard = OwnerWithGuard { prev };
        f()
    }

    /// Dispose this owner and all descendants.
    ///
    /// Children are disposed first (depth-first), then this owner's
    /// cleanups run in LIFO order, then its signals and effects are
    /// invalidated. After this call the owner's `OwnerId` is stale
    /// and any cached handle resolves to `None` on next access.
    pub fn dispose(self) {
        dispose_owner(self.id.0);
    }
}

/// Return `true` if `id` names a live reactive owner on this thread.
///
/// Exposed for `ui-async`, which holds `OwnerHandle`s on background
/// threads and needs an identity-only liveness check at dispatch-drain
/// time. Returning `false` here is the signal that the enqueued
/// callback should be dropped rather than run.
///
/// Must be called from the thread that owns the reactive runtime.
/// Background threads do not have reactive state; they post work
/// through the `ui-async` dispatcher and this check runs on the main
/// thread once the work is drained.
///
/// See DR-005 addendum §3 and DR-007 invariant (4).
#[must_use]
pub fn is_live(id: OwnerId) -> bool {
    with_rt(|rt| rt.owners.contains(id.0))
}

/// Run `f` with the owner named by `id` as the current reactive owner.
///
/// On success `f` runs with `current_owner = Some(id)` and the prior
/// `current_owner` (if any) is restored when `f` returns — including
/// on panic via an RAII guard. Returns `true` iff the owner was live
/// and `f` ran.
///
/// If `id` names a disposed owner, `f` is dropped without running and
/// this function returns `false`. This is the "no-op target" semantic
/// from DR-005 addendum §3: a `dispatch_to_owner` whose owner died
/// between enqueue and drain is silently skipped, never panics.
///
/// Must be called from the thread that owns the reactive runtime.
///
/// See DR-005 addendum §3 and DR-007 invariant (4).
#[track_caller]
pub fn with_owner_by_id(id: OwnerId, f: impl FnOnce()) -> bool {
    let prev = with_rt(|rt| {
        if !rt.owners.contains(id.0) {
            return None;
        }
        let previous = rt.current_owner;
        rt.current_owner = Some(id.0);
        Some(previous)
    });
    let Some(prev) = prev else {
        return false;
    };
    // RAII guard restores `current_owner` on normal return and on
    // unwind — same pattern as `Owner::with`.
    let _guard = OwnerWithGuard { prev };
    f();
    true
}

/// Register a cleanup callback on the current reactive owner.
///
/// The callback runs when the current owner is disposed, after all
/// descendant owners have been disposed and before this owner's
/// signals and effects are invalidated. Cleanups run in reverse
/// registration order (LIFO).
///
/// # Panics
///
/// Panics if called outside any reactive scope.
#[track_caller]
pub fn on_cleanup(f: impl FnOnce() + 'static) {
    let f: Box<dyn FnOnce()> = Box::new(f);
    with_rt(|rt| {
        let owner = rt
            .current_owner
            .expect("on_cleanup called outside a reactive scope");
        rt.owners
            .get_mut(owner)
            .expect("current owner slot missing")
            .cleanups
            .push(f);
    });
}

/// Recursively dispose `key` and every descendant, then invalidate
/// its owned signals and effects.
pub(crate) fn dispose_owner(key: OwnerKey) {
    // Phase 1: dispose children depth-first.
    let children = with_rt(|rt| {
        rt.owners
            .get(key)
            .map(|o| o.children.clone())
            .unwrap_or_default()
    });
    for child in children {
        dispose_owner(child);
    }

    // Phase 2: run cleanups in LIFO order, outside the runtime borrow.
    let cleanups = with_rt(|rt| {
        rt.owners
            .get_mut(key)
            .map(|o| core::mem::take(&mut o.cleanups))
            .unwrap_or_default()
    });
    for cleanup in cleanups.into_iter().rev() {
        cleanup();
    }

    // Phase 3: invalidate owned signals and effects.
    let (signals, effects) = with_rt(|rt| {
        rt.owners
            .get_mut(key)
            .map(|o| {
                (
                    core::mem::take(&mut o.signals),
                    core::mem::take(&mut o.effects),
                )
            })
            .unwrap_or_default()
    });
    // Dispose each effect's current run_owner so signals/effects/
    // cleanups registered inside the closure are torn down. This
    // must happen outside the runtime borrow because dispose_owner
    // re-borrows.
    let effect_run_owners: Vec<OwnerKey> = with_rt(|rt| {
        effects
            .iter()
            .filter_map(|&eff_key| rt.effects.get_mut(eff_key).and_then(|e| e.run_owner.take()))
            .collect()
    });
    for run_owner in effect_run_owners {
        dispose_owner(run_owner);
    }
    with_rt(|rt| {
        for sig_key in signals {
            rt.signals.remove(sig_key);
        }
        for eff_key in effects {
            let deps = rt
                .effects
                .get(eff_key)
                .map(|e| e.deps.clone())
                .unwrap_or_default();
            for dep in deps {
                if let Some(sig) = rt.signals.get_mut(dep)
                    && let Some(pos) = sig.subscribers.iter().position(|k| *k == eff_key)
                {
                    sig.subscribers.swap_remove(pos);
                }
            }
            rt.effects.remove(eff_key);
            rt.pending.retain(|p| *p != eff_key);
            rt.running.remove(&eff_key);
        }
    });

    // Phase 4: detach from parent, then drop the owner slot.
    with_rt(|rt| {
        let parent = rt.owners.get(key).and_then(|o| o.parent);
        if let Some(parent) = parent
            && let Some(p) = rt.owners.get_mut(parent)
        {
            p.children.retain(|&c| c != key);
        }
        rt.owners.remove(key);
    });
}
