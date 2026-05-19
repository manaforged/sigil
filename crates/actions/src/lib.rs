//! Typed action dispatch — Layer A.
//!
//! See **DR-046** for the full design. This crate ships:
//!
//! - The [`Action`] trait — type-keyed dispatch identity.
//! - [`ActionRegistry`] — per-`(TypeId, HandlerKey)` handler storage.
//! - [`DispatchCx`] — handler-side context carrying the propagation flag.
//! - [`HandlerKey`] — opaque owner identity (Layer B maps this to
//!   `EntityKey`/`FocusHandle`).
//! - [`DispatchResult`] — `{ handled, fired_owners }`.
//!
//! Re-exports [`sigil_actions_macros::Action`] as the
//! [`DeriveAction`] alias so consumers can write
//! `#[derive(sigil_actions::DeriveAction)]` when full-path clarity matters.
//!
//! # Example
//!
//! ```rust,ignore
//! use sigil_actions::{Action, ActionRegistry, DispatchCx, HandlerKey};
//!
//! #[derive(sigil_actions::DeriveAction)]
//! struct CloseTab;
//!
//! let mut registry = ActionRegistry::new();
//! registry.register::<CloseTab>(HandlerKey::from_raw(1), |_action, _cx| {
//!     // ...
//! });
//! let result = registry.dispatch(&CloseTab, &[HandlerKey::from_raw(1)]);
//! assert!(result.handled());
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;

use smallvec::SmallVec;

// `criterion` is a dev-dep used only by the `dispatch` bench. Cargo's
// lib-test target pulls all dev-deps into the compilation unit, which
// trips `unused_crate_dependencies` here.
#[cfg(test)]
use criterion as _;

pub use sigil_actions_macros::Action as DeriveAction;

/// Type-keyed dispatch identity for an action.
///
/// Action types are `'static + Send` per DR-046 §1; the registry and
/// dispatch loop remain main-thread only.
///
/// `boxed_clone` is the trait-level clone primitive used by the
/// deferred-write replay queue in `sigil-reactive`: when
/// `Context::dispatch_action` is called during `DrawPhase::Paint`, the
/// dispatch is deferred to the next Effect phase and the payload must
/// be owned across that boundary. Moving the clone primitive onto the
/// trait (rather than a `Clone` bound at the call site) means
/// consumers only opt in once at type definition time via
/// `#[derive(Clone, Action)]`. See the DR-046 2026-05-18 amendment.
pub trait Action: Any + Send + 'static {
    /// Human-readable type name for tracing / debug output.
    fn debug_name(&self) -> &'static str;

    /// Clone the action into a heap-allocated trait object.
    ///
    /// Used by the deferred-replay path to capture an owned copy of
    /// the action when `Context::dispatch_action` is invoked during
    /// the Paint phase. `#[derive(Action)]` emits this impl
    /// automatically by delegating to `Clone::clone` on the deriving
    /// type — consumers therefore write `#[derive(Clone, Action)]`
    /// together.
    fn boxed_clone(&self) -> Box<dyn Action>;

    /// Upcast to `&dyn Any` for the dispatch downcast path.
    ///
    /// Stable Rust does not support trait upcasting from
    /// `&dyn Action` to `&dyn Any` directly; this trampoline closes
    /// the gap so [`ActionRegistry::dispatch_dyn`] can drive a
    /// `&dyn Action` through the same `Any::downcast_ref` path as the
    /// monomorphic [`ActionRegistry::dispatch`]. `#[derive(Action)]`
    /// emits the impl automatically; manual `impl Action` blocks
    /// write `fn as_any(&self) -> &dyn Any { self }`.
    fn as_any(&self) -> &dyn Any;
}

/// Opaque owner identity for a handler.
///
/// Layer A uses a raw `u64`; Layer B will map this to the reactive
/// runtime's `EntityKey`/`FocusKey`. The `Eq` impl is the source of
/// truth for "is this handler scoped to this owner?" during dispatch.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct HandlerKey(u64);

impl HandlerKey {
    /// Construct from a raw u64. Layer B replaces this with key-typed
    /// constructors; Layer A's tests use the raw form directly.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Extract the raw u64 — used for ordering / logging.
    #[must_use]
    pub const fn as_raw(self) -> u64 {
        self.0
    }
}

/// Handler-side context. Carries the propagation flag that controls
/// whether dispatch continues up the chain after this handler returns.
pub struct DispatchCx {
    propagate: bool,
}

impl DispatchCx {
    const fn new() -> Self {
        Self { propagate: false }
    }

    /// Signal that the dispatcher should continue walking the chain
    /// after this handler returns. Default is "stop after first match"
    /// per DR-046 §6.
    pub const fn propagate(&mut self) {
        self.propagate = true;
    }

    pub(crate) const fn should_propagate(&self) -> bool {
        self.propagate
    }
}

/// Result of [`ActionRegistry::dispatch`].
#[derive(Debug, Clone)]
pub struct DispatchResult {
    handled: bool,
    fired_owners: SmallVec<[HandlerKey; 4]>,
}

impl DispatchResult {
    /// True if at least one handler in the chain fired.
    #[must_use]
    pub const fn handled(&self) -> bool {
        self.handled
    }

    /// Owners whose handlers fired, in the order they fired
    /// (most-nested first; further entries only present when a handler
    /// called [`DispatchCx::propagate`]).
    #[must_use]
    pub fn fired_owners(&self) -> &[HandlerKey] {
        &self.fired_owners
    }
}

type Handler = Box<dyn FnMut(&dyn Any, &mut DispatchCx)>;

struct HandlerEntry {
    owner: HandlerKey,
    handler: Handler,
}

/// Type-keyed handler storage. Handlers register against a
/// `(TypeId::of::<A>(), HandlerKey)` pair; dispatch walks a chain of
/// owners and fires the most-nested matching handler.
#[derive(Default)]
pub struct ActionRegistry {
    handlers: HashMap<TypeId, SmallVec<[HandlerEntry; 4]>>,
}

impl ActionRegistry {
    /// Construct an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler for action type `A` scoped to `owner`.
    ///
    /// Multiple handlers may share the same `(TypeId, owner)` pair —
    /// dispatch fires every handler registered under the most-nested
    /// matching owner. Across different owners, dispatch fires only
    /// the most-nested owner's handlers unless propagation is
    /// requested.
    pub fn register<A: Action>(
        &mut self,
        owner: HandlerKey,
        mut handler: impl FnMut(&A, &mut DispatchCx) + 'static,
    ) {
        let entry = HandlerEntry {
            owner,
            handler: Box::new(move |any, cx| {
                if let Some(action) = any.downcast_ref::<A>() {
                    handler(action, cx);
                }
            }),
        };
        self.handlers
            .entry(TypeId::of::<A>())
            .or_default()
            .push(entry);
    }

    /// Dispatch `action` along `chain`, walking from end (most-nested)
    /// to front (outermost).
    ///
    /// The first chain entry whose owner has a handler for `A` fires
    /// that handler. If the handler called [`DispatchCx::propagate`],
    /// dispatch continues to the next chain entry that has a handler,
    /// and so on. Returns the set of owners whose handlers fired, in
    /// fire-order (most-nested first).
    pub fn dispatch<A: Action>(&mut self, action: &A, chain: &[HandlerKey]) -> DispatchResult {
        let Some(entries) = self.handlers.get_mut(&TypeId::of::<A>()) else {
            return DispatchResult {
                handled: false,
                fired_owners: SmallVec::new(),
            };
        };
        Self::dispatch_entries(entries, action as &dyn Any, chain)
    }

    /// Dispatch a `&dyn Action` along `chain`.
    ///
    /// The `dyn`-typed sibling of [`Self::dispatch`] — used by the
    /// deferred-write replay path in `sigil-reactive`, which holds the
    /// action as `Box<dyn Action>` because the original
    /// `dispatch_action<A>` monomorphic call site was erased to
    /// support generic deferral without an `A: Clone` bound at the
    /// call site (see DR-046 2026-05-18 amendment).
    pub fn dispatch_dyn(&mut self, action: &dyn Action, chain: &[HandlerKey]) -> DispatchResult {
        let type_id = Any::type_id(action.as_any());
        let Some(entries) = self.handlers.get_mut(&type_id) else {
            return DispatchResult {
                handled: false,
                fired_owners: SmallVec::new(),
            };
        };
        Self::dispatch_entries(entries, action.as_any(), chain)
    }

    /// Walk `chain` leaf-first against an already-borrowed entry list.
    ///
    /// Shared between [`Self::dispatch`], [`Self::dispatch_dyn`], and
    /// the re-entrant pattern in
    /// `sigil-reactive::Context::dispatch_action`, where the caller
    /// takes the entry list out of the registry (via [`Self::take_entries`])
    /// for the duration of the call so handler bodies can recursively
    /// dispatch *different* actions through the registry without
    /// tripping a borrow conflict.
    fn dispatch_entries(
        entries: &mut SmallVec<[HandlerEntry; 4]>,
        action: &dyn Any,
        chain: &[HandlerKey],
    ) -> DispatchResult {
        let mut fired_owners: SmallVec<[HandlerKey; 4]> = SmallVec::new();
        for owner in chain.iter().rev() {
            let mut any_at_owner = false;
            let mut should_continue = false;
            for entry in entries.iter_mut().filter(|e| e.owner == *owner) {
                let mut cx = DispatchCx::new();
                (entry.handler)(action, &mut cx);
                any_at_owner = true;
                if cx.should_propagate() {
                    should_continue = true;
                }
            }
            if any_at_owner {
                fired_owners.push(*owner);
                if !should_continue {
                    return DispatchResult {
                        handled: true,
                        fired_owners,
                    };
                }
            }
        }
        DispatchResult {
            handled: !fired_owners.is_empty(),
            fired_owners,
        }
    }

    /// Take the entry list for action type `A` out of the registry.
    ///
    /// Pair with [`Self::restore_entries`] to dispatch without holding
    /// a borrow on `self` for the duration of handler execution — the
    /// load-bearing primitive for re-entrant dispatch.
    pub fn take_entries<A: Action>(&mut self) -> TakenEntries {
        self.take_entries_by_id(TypeId::of::<A>())
    }

    /// Take the entry list keyed by `type_id` out of the registry.
    ///
    /// `dyn`-typed sibling of [`Self::take_entries`] — used by the
    /// deferred-replay path which holds the action as
    /// `Box<dyn Action>` and reads the `TypeId` off the trait object
    /// rather than via a monomorphic `A`.
    pub fn take_entries_by_id(&mut self, type_id: TypeId) -> TakenEntries {
        let entries = self.handlers.remove(&type_id).unwrap_or_default();
        TakenEntries { type_id, entries }
    }

    /// Restore an entry list previously removed by [`Self::take_entries`].
    ///
    /// If a handler registered while the list was taken (i.e. a
    /// re-entrant `register` for the same action type), the freshly
    /// registered entries are preserved by appending the restored list
    /// after them.
    pub fn restore_entries(&mut self, taken: TakenEntries) {
        let slot = self.handlers.entry(taken.type_id).or_default();
        // Re-registrations during the dispatch window remain at the
        // front; restored entries return in their original order.
        slot.extend(taken.entries);
    }

    /// Run [`Self::dispatch_entries`] against an entry list previously
    /// removed by [`Self::take_entries`].
    pub fn dispatch_taken<A: Action>(
        taken: &mut TakenEntries,
        action: &A,
        chain: &[HandlerKey],
    ) -> DispatchResult {
        debug_assert_eq!(
            taken.type_id,
            TypeId::of::<A>(),
            "dispatch_taken type mismatch: entries were taken for a different action type",
        );
        Self::dispatch_entries(&mut taken.entries, action as &dyn Any, chain)
    }

    /// Run [`Self::dispatch_entries`] against an entry list previously
    /// removed by [`Self::take_entries_by_id`], driving the dispatch
    /// from a `&dyn Action` rather than a monomorphic `&A`.
    pub fn dispatch_taken_dyn(
        taken: &mut TakenEntries,
        action: &dyn Action,
        chain: &[HandlerKey],
    ) -> DispatchResult {
        debug_assert_eq!(
            taken.type_id,
            Any::type_id(action.as_any()),
            "dispatch_taken_dyn type mismatch: entries were taken for a different action type",
        );
        Self::dispatch_entries(&mut taken.entries, action.as_any(), chain)
    }
}

/// Owned handler list returned by [`ActionRegistry::take_entries`].
///
/// Carries the `TypeId` the list was indexed under plus the entry
/// `SmallVec` itself. Opaque on purpose — callers thread it back into
/// the registry via [`ActionRegistry::restore_entries`] without
/// poking at the entry shape.
#[must_use = "TakenEntries must be restored via ActionRegistry::restore_entries"]
pub struct TakenEntries {
    type_id: TypeId,
    entries: SmallVec<[HandlerEntry; 4]>,
}

impl TakenEntries {
    /// True if no handlers were taken (no registrations for this
    /// action type). The caller can short-circuit dispatch.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
