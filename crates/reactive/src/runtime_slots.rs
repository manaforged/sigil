//! Arena slot definitions for the reactive runtime.
//!
//! Extracted from `runtime.rs` to keep that file under the 400-line
//! cap. Every slot type is `pub(crate)` and consumed via the shared
//! [`with_rt`](crate::runtime::with_rt) seam — sibling modules import
//! these by name when they need to read or mutate arena slots.

// Mirrors the allow on `runtime.rs` (G2 keystone landing) — slot
// fields are consumed by sibling modules in this and follow-up
// commits; flagging them as dead-code during the staged landing
// fights the staged landing.
#![allow(
    dead_code,
    reason = "entity / observer plumbing consumed by sibling modules in this G2 landing"
)]

use core::any::{Any, TypeId};
use core::panic::Location;

use smallvec::SmallVec;

use crate::arena::Key;

pub(crate) type OwnerKey = Key<OwnerSlot>;
pub(crate) type SignalKey = Key<SignalSlot>;
pub(crate) type EffectKey = Key<EffectSlot>;
pub(crate) type EntityKey = Key<EntitySlot>;
pub(crate) type ObserverKey = Key<ObserverSlot>;

/// A queued completion callback waiting to run on the main thread.
///
/// `MainThreadDispatcher::post_to_main` adapters push these onto
/// `RuntimeState::pending_completions`. The queue is drained at the
/// top of [`crate::Runtime::begin_effect`] (DR-048 §4) so completion
/// handlers always run in the Effect phase, never Paint.
pub(crate) struct PendingCompletion {
    pub(crate) owner: OwnerKey,
    pub(crate) callback: Box<dyn FnOnce() + Send + 'static>,
}

/// Payload of an entity in the runtime arena.
///
/// `value` is `Box<dyn Any>` so the arena is mono-typed; downcasts
/// happen at the access seam (`Entity::update`, `Entity::read`).
pub(crate) struct EntitySlot {
    pub(crate) owner: OwnerKey,
    pub(crate) value: Box<dyn Any>,
    pub(crate) subscribers: SmallVec<[ObserverKey; 4]>,
    /// Typed event subscribers, keyed by `TypeId::of::<Ev>()`.
    pub(crate) event_subs: SmallVec<[(TypeId, ObserverKey); 4]>,
    /// Set while an `Entity::update` closure is in flight on this
    /// entity. Re-entrant update on the same entity panics.
    pub(crate) locked: bool,
    pub(crate) name: &'static str,
    pub(crate) location: &'static Location<'static>,
}

/// Function fired by an observer callback. Borrows the runtime
/// mutably (callbacks may chain `update` / `observe` calls) plus the
/// observer key (so the dispatcher can resolve the observer's `T`
/// payload) and, for `Subscribe`, the typed event payload erased
/// through `Box<dyn Any>`.
pub(crate) type ObserveFn = Box<dyn FnMut(ObserverKey)>;
pub(crate) type ObserveBareFn = Box<dyn FnMut()>;
pub(crate) type SubscribeFn = Box<dyn FnMut(ObserverKey, &dyn Any)>;

/// Kind of observer callback.
pub(crate) enum ObserverKind {
    /// Plain `cx.observe` — fires on any `cx.notify()` of observed.
    Observe(ObserveFn),
    /// Top-level `observe_global` — bare `FnMut()`, no observer ctx.
    ObserveBare(ObserveBareFn),
    /// `cx.subscribe::<U, Ev>` — fires on matching `cx.emit::<Ev>`.
    Subscribe { ev_type: TypeId, f: SubscribeFn },
}

/// Slot for an observer registered against an entity (or a typed
/// event channel).
pub(crate) struct ObserverSlot {
    /// The observer's own entity, if any. `None` for top-level
    /// `observe_global` registered outside an entity.
    pub(crate) observer_entity: Option<EntityKey>,
    /// The entity being observed.
    pub(crate) observed: EntityKey,
    /// Owner this subscription was created under — used by the
    /// owner-cascade unsubscribe safety net.
    pub(crate) owner: OwnerKey,
    pub(crate) kind: ObserverKind,
}

/// Pending observer-callback entry queued by `cx.notify()` /
/// `cx.emit()`. Drained at the end of the outermost `update` turn.
pub(crate) struct PendingObserver {
    pub(crate) observer: ObserverKey,
    /// `Some` for typed-event drain (`subscribe`); `None` for plain
    /// `observe` / `observe_global`.
    pub(crate) event: Option<Box<dyn Any>>,
}

pub(crate) struct OwnerSlot {
    pub(crate) parent: Option<OwnerKey>,
    pub(crate) index_in_parent: Option<usize>,
    pub(crate) children: Vec<OwnerKey>,
    pub(crate) signals: Vec<SignalKey>,
    pub(crate) effects: Vec<EffectKey>,
    /// Entities created under this owner. Disposed depth-first when
    /// the owner is disposed (DR-045 decision 3).
    pub(crate) entities: Vec<EntityKey>,
    /// Observer keys registered while this owner was current — fed
    /// into the owner-cascade unsubscribe safety net (DR-045
    /// decision 4). Both `subscribers` (plain) and `event_subs`
    /// (typed) live here, distinguished by their `ObserverKind`.
    pub(crate) observers: Vec<ObserverKey>,
    pub(crate) cleanups: Vec<Box<dyn FnOnce()>>,
    /// Owner debug name captured at construction. Read by
    /// [`crate::debug::dump_owner_tree`]. See DR-005 addendum item 1.
    pub(crate) name: &'static str,
    /// `Location::caller()` at construction. Read by
    /// [`crate::debug::dump_owner_tree`]. See DR-005 addendum item 2.
    pub(crate) location: &'static Location<'static>,
}

pub(crate) struct SignalSlot {
    /// Owner of this signal.
    #[expect(
        dead_code,
        reason = "construction-site owner; write path does not read it"
    )]
    pub(crate) owner: OwnerKey,
    pub(crate) value: Box<dyn Any>,
    pub(crate) subscribers: SmallVec<[EffectKey; 4]>,
    /// Call site of `signal()`.
    #[expect(
        dead_code,
        reason = "construction-site location; write path does not read it"
    )]
    pub(crate) location: &'static Location<'static>,
}

pub(crate) struct EffectSlot {
    /// The owner in whose scope `effect()` was called. Each run of
    /// the effect creates its own child owner (`run_owner`) under
    /// this one so that signals/effects/cleanups registered inside
    /// the closure are torn down before the next re-run. See
    /// `effect::run_effect`.
    pub(crate) owner: OwnerKey,
    /// The owner created for the most recent (or in-progress) run
    /// of this effect. Disposed before each new run and on effect
    /// removal. `None` until the first run starts.
    pub(crate) run_owner: Option<OwnerKey>,
    pub(crate) f: Option<Box<dyn FnMut()>>,
    /// Dependencies from the active run. During an effect run this is
    /// rebuilt from scratch; outside a run it is the current live
    /// dependency set.
    pub(crate) deps: SmallVec<[SignalKey; 4]>,
    /// Dependencies from the previous run while the effect is
    /// currently executing.
    ///
    /// We keep those signal subscriptions installed until the new
    /// run completes, then unsubscribe only deps that disappeared.
    /// That avoids a remove/re-add pass for stable dependencies,
    /// which is the common fan-out path.
    pub(crate) prev_deps: SmallVec<[SignalKey; 4]>,
    /// Fast cursor into `prev_deps` for the common case where an
    /// effect reads the same dependency list in the same order on
    /// each run.
    pub(crate) prev_dep_cursor: usize,
    pub(crate) location: &'static Location<'static>,
    /// Debug name for this effect, captured at registration. Set via
    /// [`crate::effect::effect_named`]; [`crate::effect::effect`]
    /// stores `"<unnamed>"`. Emitted by the `SIGIL_TRACE_EFFECTS`
    /// trace hook and future debug tooling. See DR-005 addendum item 1.
    pub(crate) name: &'static str,
}
