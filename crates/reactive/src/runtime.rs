//! Thread-local reactive runtime.
//!
//! Holds three arenas (owners, signals, effects) plus the current
//! owner/effect TLS stacks and the batched pending-effect queue.
//! Everything here is `!Send`: a reactive runtime belongs to one
//! thread for its entire lifetime.
//!
//! The `Runtime` public surface is split across sibling modules to
//! keep this file under the 400-line cap:
//!
//! - [`runtime_slots`](crate::runtime_slots) — arena slot type
//!   definitions (`OwnerSlot`, `SignalSlot`, …).
//! - [`runtime_phase`](crate::runtime_phase) — frame-phase transitions
//!   (`begin_effect`, `begin_layout`, `begin_paint`, `end_frame`).
//! - [`runtime_executor`](crate::runtime_executor) — executor install
//!   plus the completion-queue surface.
//! - [`runtime_drain`](crate::runtime_drain) — module-scope drain
//!   helpers (`drain_pending_completions`, `drain_observers_to_fixpoint`,
//!   `push_pending_completion`).

// Entity / observer plumbing (DR-045 G2 keystone) lives in
// `entity.rs`, `entity_observer.rs`, `entity_global.rs`, `cx.rs`;
// each of these struct/enum/field below is consumed by exactly one
// of those modules. During incremental landing the dead-code lint
// fires before those consumers exist — scoped allow keeps the
// runtime additions reviewable as a single unit.
#![allow(
    dead_code,
    reason = "entity / observer plumbing consumed by sibling modules in this G2 landing"
)]

use core::any::TypeId;
use core::cell::RefCell;
use core::sync::atomic::{AtomicU64, Ordering};
use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;

use actions::ActionRegistry;

use crate::arena::Arena;
use crate::async_cx::{MainThreadDispatcher, TaskExecutor};
use crate::focus::FocusHandleId;
use crate::local_executor::LocalTaskExecutor;
use crate::phase::{DeferredWrite, DrawPhase};
pub(crate) use crate::runtime_slots::{
    EffectKey, EffectSlot, EntityKey, EntitySlot, ObserveBareFn, ObserveFn, ObserverKey,
    ObserverKind, ObserverSlot, OwnerKey, OwnerSlot, PendingCompletion, PendingObserver, SignalKey,
    SignalSlot, SubscribeFn,
};

/// Maximum quiescence passes for [`Runtime::run_until_idle`].
///
/// Per DR-048 §8 — catches the pathological livelock where each
/// completion enqueues another. Matches DR-047 §8 conservatism.
pub(crate) const MAX_QUIESCENCE_PASSES: u32 = 64;

/// Monotonic counter for per-thread reactive runtime identities.
///
/// Each thread's [`RuntimeState`] claims a unique, non-zero id from
/// this counter the first time it is touched (via [`runtime_id`]).
/// The id is embedded in every [`crate::SendableSignal`] produced by
/// that runtime so re-hydration on a different thread panics instead
/// of silently aliasing into an unrelated signal's arena slot.
///
/// A `u64` counter cannot wrap in any realistic scenario — ten
/// billion runtime constructions per second would still take 58 years
/// to exhaust it.
static RUNTIME_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) struct RuntimeState {
    pub(crate) owners: Arena<OwnerSlot>,
    pub(crate) signals: Arena<SignalSlot>,
    pub(crate) effects: Arena<EffectSlot>,
    pub(crate) entities: Arena<EntitySlot>,
    pub(crate) observers: Arena<ObserverSlot>,
    /// Pending observer queue, drained at the end of the outermost
    /// `Entity::update` turn (DR-045 decision 5). Parallel to
    /// `pending` for effects.
    pub(crate) pending_observers: VecDeque<PendingObserver>,
    /// Depth of nested `Entity::update` calls. Observer drain fires
    /// only when this returns to 0 (the outermost update).
    pub(crate) update_depth: u32,
    /// `TypeId` → `EntityKey` map for global state (DR-045 decision 6).
    /// At ≤100 globals scale a linear-scan `Vec` beats a `HashMap` and
    /// stays `const`-constructible for the thread-local.
    pub(crate) globals: Vec<(TypeId, EntityKey)>,
    pub(crate) current_owner: Option<OwnerKey>,
    pub(crate) current_effect: Option<EffectKey>,
    /// Entity whose `update` closure is currently in flight.
    /// `cx.notify()` reads this to know which entity's subscribers
    /// to schedule.
    pub(crate) current_entity: Option<EntityKey>,
    pub(crate) batch_depth: u32,
    pub(crate) pending: VecDeque<EffectKey>,
    pub(crate) pending_set: BTreeSet<EffectKey>,
    pub(crate) running: BTreeSet<EffectKey>,
    /// Focus stack — innermost-last. `Context::focus` pushes,
    /// `Context::blur` removes, owner-cleanup evicts on entity drop.
    /// `dispatch_action` walks this leaf-first to resolve handlers.
    ///
    /// `Vec` (not `SmallVec`) so the field has a `const fn new()` for
    /// the thread-local-init `const` block. Focus stacks are bounded
    /// by UI nesting depth (rarely >10 levels) and pushes are rare
    /// (focus changes); the heap-alloc penalty is irrelevant.
    pub(crate) focus_stack: Vec<FocusHandleId>,
    /// Action handler registry. Wrapped in `Option` so
    /// `Context::dispatch_action` can move it out, dispatch without
    /// holding a borrow on the runtime, then put it back — the
    /// re-entrant dispatch pattern (DR-046 §6).
    pub(crate) action_registry: Option<ActionRegistry>,
    /// Current frame phase. Reads cheap (`Copy` enum); consulted by
    /// every reactive write seam via [`crate::phase::assert_paint_safe`].
    /// See DR-047.
    pub(crate) phase: DrawPhase,
    /// Queue of writes captured during [`DrawPhase::Paint`] that are
    /// replayed at [`Runtime::end_frame`]. Today only `dispatch_action`
    /// defers; direct state mutations panic in debug and no-op in
    /// release. See DR-047 §5.
    pub(crate) deferred_writes: Vec<DeferredWrite>,
    /// G5 — completion callbacks delivered by background tasks, drained
    /// at the top of [`Runtime::begin_effect`] (DR-048 §4). The queue
    /// is `FnOnce + Send` so adapters can push from any thread, even
    /// though the drain itself happens on the main thread.
    pub(crate) pending_completions: Vec<PendingCompletion>,
    /// G5 — installed task executor (DR-048 §5). `None` until
    /// [`Runtime::install_executor`] runs. Cloned per [`crate::Context::spawn`]
    /// call.
    pub(crate) executor: Option<Arc<dyn TaskExecutor>>,
    /// G5 — installed main-thread dispatcher (DR-048 §5). `None` until
    /// [`Runtime::install_executor`] runs. Cloned per [`crate::Context::spawn`]
    /// call into the produced [`crate::AsyncContext`].
    pub(crate) dispatcher: Option<Arc<dyn MainThreadDispatcher>>,
    /// W4.99.B3 — installed `!Send`-future local executor. `None` until
    /// [`Runtime::install_local_executor`] runs. Cloned per
    /// [`crate::Context::spawn_local`] call.
    ///
    /// Held as `Arc<dyn LocalTaskExecutor>` rather than `Rc<...>` so the
    /// runtime's tear-down path stays uniform with the `executor` /
    /// `dispatcher` siblings; the trait itself is intentionally NOT
    /// `Send + Sync`, but `Arc<dyn !Send>` is still legal (the Arc just
    /// inherits the trait object's negative bounds).
    pub(crate) local_executor: Option<Arc<dyn LocalTaskExecutor>>,
    /// W5.2 / DR-050 §G10-phase-2 — a11y tree is dirty and needs to be
    /// republished at the next [`Runtime::end_frame`] drain.
    ///
    /// Flipped on by the `entity_a11y_observer` reactive effect bodies
    /// whenever any subscribed `A11yProps` signal (`checked`, `value_now`,
    /// `expanded`, `text_value`, `live`, …) changes. Cleared by the
    /// drain in [`crate::runtime_drain::drain_a11y_updates`].
    pub(crate) a11y_dirty: bool,
    /// Whether any visual-affecting reactive mutation occurred during the
    /// current frame. Reset to `false` at [`Runtime::begin_effect`]; flipped on
    /// by every mutation source that schedules observer/effect work (signal
    /// `set`/`update`, `Context::notify`, `set_global`). The application frame
    /// driver reads it via [`Runtime::frame_dirty`] to skip layout + paint when
    /// a frame (e.g. a bare cursor-move) changed nothing. Errs toward dirty: an
    /// extra paint is cheap, a missed one is a visible freeze. Starts `true` so
    /// the first frame always paints.
    pub(crate) frame_dirty: bool,
    /// W5.2 / DR-050 §G10-phase-2 — installed publisher callback that
    /// the end-of-frame drain invokes to snapshot the current `Cx` into a
    /// `TreeUpdate` and forward it to the installed `A11yBridge`.
    ///
    /// `None` until [`Runtime::install_a11y_publisher`] is called. The
    /// publisher is a `FnMut` so it can hold a mutable handle to both the
    /// kernel `Cx` and the bridge — both of which the application owns
    /// outside the reactive runtime.
    pub(crate) a11y_publisher: Option<Box<dyn FnMut()>>,
    /// Unique, non-zero identity for this thread's reactive runtime.
    ///
    /// Lazy-initialized on first call to [`runtime_id`] (zero until
    /// then, since [`RuntimeState::new`] must be `const` for the
    /// thread-local `const` block). The value is stable for the
    /// lifetime of the thread and is embedded in every
    /// [`crate::SendableSignal`] so cross-thread re-hydration can
    /// panic on mismatch instead of silently aliasing into an
    /// unrelated runtime's arena.
    pub(crate) runtime_id: u64,
}

impl RuntimeState {
    const fn new() -> Self {
        Self {
            owners: Arena::new(),
            signals: Arena::new(),
            effects: Arena::new(),
            entities: Arena::new(),
            observers: Arena::new(),
            pending_observers: VecDeque::new(),
            update_depth: 0,
            globals: Vec::new(),
            current_owner: None,
            current_effect: None,
            current_entity: None,
            batch_depth: 0,
            pending: VecDeque::new(),
            pending_set: BTreeSet::new(),
            running: BTreeSet::new(),
            focus_stack: Vec::new(),
            action_registry: None,
            phase: DrawPhase::Idle,
            deferred_writes: Vec::new(),
            pending_completions: Vec::new(),
            executor: None,
            dispatcher: None,
            local_executor: None,
            a11y_dirty: false,
            frame_dirty: true,
            a11y_publisher: None,
            runtime_id: 0,
        }
    }
}

thread_local! {
    static RUNTIME: RefCell<RuntimeState> = const { RefCell::new(RuntimeState::new()) };
}

/// Run `f` with exclusive access to the runtime state.
///
/// `f` must not call user code — only take short, synchronous
/// snapshots and apply focused mutations. Any callback that might
/// itself touch the runtime (effect functions, cleanups, signal
/// subscribers) runs *outside* the borrow.
///
/// Marked `#[inline]` so the closure + `RefCell` borrow fuse with
/// the caller's frame. Every reactive op goes through this — the
/// extra compiled size is trivial against the measurable save on
/// the hot path.
#[inline]
pub(crate) fn with_rt<R>(f: impl FnOnce(&mut RuntimeState) -> R) -> R {
    RUNTIME.with(|rt| f(&mut rt.borrow_mut()))
}

/// Return this thread's runtime id, lazily claiming one on first call.
///
/// The value is non-zero and stable for the lifetime of the thread.
/// Two different threads' runtimes are guaranteed to have different
/// ids; that guarantee is what makes the `SendableSignal` runtime
/// check useful.
///
/// Only called by `SendableSignal::into_sendable` and
/// `SendableSignal::into_signal` — both are cold paths relative to
/// normal reactive ops, so the `Ordering::Relaxed` `fetch_add` and
/// the extra `with_rt` are free.
pub(crate) fn runtime_id() -> u64 {
    with_rt(|rt| {
        if rt.runtime_id == 0 {
            // `fetch_add` returns the previous value; add 1 so the
            // first id handed out is 1 and 0 remains the sentinel
            // for "uninitialized".
            rt.runtime_id = RUNTIME_ID_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        }
        rt.runtime_id
    })
}

/// The reactive runtime — entry points that create and dispose
/// root owners.
///
/// The public surface is split across this module plus three siblings:
///
/// - [`runtime_phase`](crate::runtime_phase) — frame-phase transitions.
/// - [`runtime_executor`](crate::runtime_executor) — executor /
///   completion-queue API.
/// - here — root-scope construction and phase introspection.
#[derive(Debug)]
pub struct Runtime;

impl Runtime {
    /// Create a root owner named `name`, run `f` with it active,
    /// then dispose the owner.
    ///
    /// `f` receives no arguments; everything inside its body reads
    /// and writes the thread-local runtime directly via the public
    /// `signal`, `effect`, and `on_cleanup` functions.
    ///
    /// Re-entering `scope` from inside another `scope` is allowed —
    /// the outer scope's current owner is saved and restored.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use reactive::{effect, signal, Runtime};
    ///
    /// let result = Runtime::scope("example", || {
    ///     let n = signal(1_i32);
    ///     n.set(42);
    ///     n.get()
    /// });
    /// assert_eq!(result, 42);
    /// ```
    ///
    /// (See the crate-level note for why doctests are `ignore`.)
    #[track_caller]
    pub fn scope<R>(name: &'static str, f: impl FnOnce() -> R) -> R {
        let owner = crate::owner::Owner::new(name, None);
        let result = owner.with(f);
        owner.dispose();
        result
    }

    /// Read the current frame phase.
    ///
    /// Cheap (`Copy` enum + one thread-local borrow). Public so
    /// consumers — typically widgets choosing between an immediate-write
    /// path and a deferred path — can branch on phase.
    #[must_use]
    pub fn current_phase() -> DrawPhase {
        with_rt(|rt| rt.phase)
    }
}
