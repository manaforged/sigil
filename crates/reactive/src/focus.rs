//! Focus handles + focus stack (G3 layer B / DR-046 §4).
//!
//! A [`FocusHandle`] is a stable identity for "the focusable region of
//! entity X." Entities mint one via [`Context::focus_handle`] and use
//! it to (a) register action handlers ([`Context::on_action`]) and (b)
//! push themselves onto the focus stack ([`Context::focus`]) so
//! `dispatch_action` resolves handlers along the chain.
//!
//! See DR-046 §4. The focus stack itself lives on [`RuntimeState`] —
//! this module owns the handle type and the allocator counter.
//!
//! [`RuntimeState`]: crate::runtime::RuntimeState

// Focus handle plumbing (DR-046 G3 layer B) is consumed by
// `runtime.rs` (focus_stack field) and `cx.rs` (focus / blur /
// on_action / dispatch_action). During incremental landing the
// dead-code lint fires before those consumers exist — scoped allow
// keeps the focus module reviewable as a single unit.
#![allow(
    dead_code,
    reason = "consumed by runtime focus_stack + Context::focus/blur/on_action in this G3 layer B landing"
)]

use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};

use sigil_actions::HandlerKey;

use crate::runtime::EntityKey;

/// Process-wide monotonic counter for focus handle ids. `1`-based so
/// `0` stays reserved as a sentinel.
static FOCUS_HANDLE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Opaque, `Copy + Eq + Hash` identity for a focus handle.
///
/// Bridged to [`HandlerKey`] via the impls below — the action
/// registry stores handlers under `HandlerKey::from_raw(id.0)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusHandleId(pub(crate) u64);

impl FocusHandleId {
    /// Extract the raw u64 — used for logging / ordering.
    #[must_use]
    pub const fn as_raw(self) -> u64 {
        self.0
    }
}

impl From<FocusHandleId> for HandlerKey {
    fn from(id: FocusHandleId) -> Self {
        Self::from_raw(id.0)
    }
}

/// A focus handle.
///
/// Minted by [`Context::focus_handle`]; carries the [`EntityKey`] of
/// the owning entity so the owner-cascade cleanup can evict the
/// handle from the focus stack on entity disposal.
///
/// `!Send + !Sync` — the runtime state is thread-local and a handle's
/// validity is tied to its entity owner on this thread.
#[derive(Debug)]
pub struct FocusHandle {
    pub(crate) id: FocusHandleId,
    pub(crate) owner: EntityKey,
    // !Send + !Sync — the focus stack and the owning entity arena
    // both live in thread-local storage.
    _not_send: PhantomData<*const ()>,
}

impl FocusHandle {
    /// Internal constructor — wrapped key + minted id.
    pub(crate) fn new(owner: EntityKey) -> Self {
        let raw = FOCUS_HANDLE_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            id: FocusHandleId(raw),
            owner,
            _not_send: PhantomData,
        }
    }

    /// This handle's stable identity.
    #[must_use]
    pub const fn id(&self) -> FocusHandleId {
        self.id
    }

    /// The entity that owns this handle. Exposed for the cleanup
    /// integration test.
    #[must_use]
    pub(crate) const fn owner(&self) -> EntityKey {
        self.owner
    }
}
