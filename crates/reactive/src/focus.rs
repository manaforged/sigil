//! Focus handles + focus stack.
//!
//! A [`FocusHandle`] is a stable identity for a focusable region.
//! Entities mint one via [`crate::cx::EntityCx::focus_handle`] and use
//! it to register action handlers and to push onto the focus stack.
//!
//! The focus stack lives on [`RuntimeState`].
//!
//! [`RuntimeState`]: crate::runtime::RuntimeState

use core::marker::PhantomData;
use core::sync::atomic::{AtomicU64, Ordering};

use sigil_actions::HandlerKey;

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
/// Minted by [`crate::cx::EntityCx::focus_handle`]. Drop of the minting
/// owner evicts the id from the focus stack.
///
/// `!Send + !Sync` — the focus stack lives in thread-local storage.
#[derive(Debug)]
pub struct FocusHandle {
    pub(crate) id: FocusHandleId,
    _not_send: PhantomData<*const ()>,
}

impl FocusHandle {
    /// Mint a unique handle id.
    pub(crate) fn new() -> Self {
        let raw = FOCUS_HANDLE_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            id: FocusHandleId(raw),
            _not_send: PhantomData,
        }
    }

    /// This handle's stable identity.
    #[must_use]
    pub const fn id(&self) -> FocusHandleId {
        self.id
    }
}
