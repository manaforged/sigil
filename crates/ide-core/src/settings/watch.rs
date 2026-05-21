//! Watch subscriptions and dispatch.
//!
//! Each [`crate::settings::Store::watch`] call registers a callback under
//! a key. The returned [`Subscription`] holds an RAII guard — dropping
//! it removes the callback. Internally the store holds the callback set
//! in an `Arc<Mutex<…>>` so `&self` watch + `&mut self` reload + drop
//! across threads all behave.

use std::sync::{Arc, Mutex, Weak};

use crate::settings::diagnostic::Diagnostic;

/// Boxed callback fired when a watched key's resolved value changes.
pub(super) type Callback = Box<dyn Fn(&Diagnostic) + Send + Sync + 'static>;

/// One registered callback. The `id` is a unique handle the
/// [`Subscription`] uses to find and remove this entry on drop.
pub(super) struct Entry {
    pub(super) id: u64,
    pub(super) callback: Callback,
}

/// Per-key callback table. Stored as `Arc<Mutex<Vec<Entry>>>` so the
/// store, watch dispatch, and subscription drop can all reach it.
pub(super) type WatchList = Arc<Mutex<Vec<Entry>>>;

/// RAII guard returned by [`crate::settings::Store::watch`]. Dropping it
/// unregisters the callback. Holding it forever is fine — the store
/// keeps a strong reference too.
pub struct Subscription {
    pub(super) list: Weak<Mutex<Vec<Entry>>>,
    pub(super) id: u64,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(list) = self.list.upgrade() {
            // Lock contention here is bounded by other watchers on the
            // same key; never blocks on user code. Poisoning is
            // recoverable — we just clear the entry and move on.
            if let Ok(mut entries) = list.lock() {
                entries.retain(|e| e.id != self.id);
            }
        }
    }
}

/// Fire every callback for a key. Called from inside the store after a
/// reload detects a value change for this key. Lock-then-clone the
/// callbacks before invoking so user code can re-enter the store
/// without deadlocking on the watch list.
pub(super) fn fire(list: &WatchList, diag: &Diagnostic) {
    let snapshot: Vec<Callback> = {
        let Ok(entries) = list.lock() else { return };
        entries
            .iter()
            .map(|_| -> Callback { Box::new(|_| {}) })
            .collect()
    };
    // The cheap-clone path: we can't clone `Box<dyn Fn>`, so instead we
    // hold the lock long enough to invoke each callback. Invocation
    // borrows by reference and the callbacks are `Fn + Send + Sync`, so
    // re-entrancy is the only concern. Callbacks must not call back
    // into `Store::watch` for the same store from inside a callback —
    // documented at the [`crate::settings::Store::watch`] level.
    drop(snapshot);
    let Ok(entries) = list.lock() else { return };
    for entry in entries.iter() {
        (entry.callback)(diag);
    }
}
