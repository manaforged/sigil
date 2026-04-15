//! Thread-local reactive runtime.
//!
//! Holds three arenas (owners, signals, effects) plus the current
//! owner/effect TLS stacks and the batched pending-effect queue.
//! Everything here is `!Send`: a reactive runtime belongs to one
//! thread for its entire lifetime.

use core::any::Any;
use core::cell::RefCell;
use core::panic::Location;
use core::sync::atomic::{AtomicU64, Ordering};
use std::collections::{BTreeSet, VecDeque};

use smallvec::SmallVec;

use crate::arena::{Arena, Key};

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

pub(crate) type OwnerKey = Key<OwnerSlot>;
pub(crate) type SignalKey = Key<SignalSlot>;
pub(crate) type EffectKey = Key<EffectSlot>;

pub(crate) struct OwnerSlot {
    pub(crate) parent: Option<OwnerKey>,
    pub(crate) children: Vec<OwnerKey>,
    pub(crate) signals: Vec<SignalKey>,
    pub(crate) effects: Vec<EffectKey>,
    pub(crate) cleanups: Vec<Box<dyn FnOnce()>>,
    /// Owner debug name captured at construction. Read by
    /// [`crate::debug::dump_owner_tree`]. See DR-005 addendum item 1.
    pub(crate) name: &'static str,
    /// `Location::caller()` at construction. Read by
    /// [`crate::debug::dump_owner_tree`]. See DR-005 addendum item 2.
    pub(crate) location: &'static Location<'static>,
}

pub(crate) struct SignalSlot {
    /// Owner this signal belongs to. Kept for consistency asserts and
    /// currently unread outside of tests; future commits will use it
    /// for cross-owner signal moves and a debug check in
    /// `dispose_owner`.
    #[expect(
        dead_code,
        reason = "read by future cross-owner signal moves; kept to preserve the construction-site invariant"
    )]
    pub(crate) owner: OwnerKey,
    pub(crate) value: Box<dyn Any>,
    pub(crate) subscribers: SmallVec<[EffectKey; 4]>,
    /// `Location::caller()` at `signal()` call site. Currently only
    /// read in test-targeted stale-handle panic paths; the field
    /// stays so the construction-site invariant holds for every
    /// signal.
    #[expect(
        dead_code,
        reason = "stale-handle panic messages use this in a follow-up commit"
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
    pub(crate) f: Box<dyn FnMut()>,
    pub(crate) deps: SmallVec<[SignalKey; 4]>,
    pub(crate) location: &'static Location<'static>,
    /// Debug name for this effect, captured at registration. Set via
    /// [`crate::effect::effect_named`]; [`crate::effect::effect`]
    /// stores `"<unnamed>"`. Emitted by the `RUST_UI_TRACE_EFFECTS`
    /// trace hook and future debug tooling. See DR-005 addendum item 1.
    pub(crate) name: &'static str,
}

pub(crate) struct RuntimeState {
    pub(crate) owners: Arena<OwnerSlot>,
    pub(crate) signals: Arena<SignalSlot>,
    pub(crate) effects: Arena<EffectSlot>,
    pub(crate) current_owner: Option<OwnerKey>,
    pub(crate) current_effect: Option<EffectKey>,
    pub(crate) batch_depth: u32,
    pub(crate) pending: VecDeque<EffectKey>,
    pub(crate) running: BTreeSet<EffectKey>,
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
            current_owner: None,
            current_effect: None,
            batch_depth: 0,
            pending: VecDeque::new(),
            running: BTreeSet::new(),
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
/// Phase 1.1 exposes one method, [`Runtime::scope`], which creates
/// an unnamed root owner, runs a closure with it active, and
/// disposes the owner on return. Tests use this; apps eventually
/// drive roots through their top-level `App` struct (Phase 1.2+).
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
    /// use ui_reactive::{effect, signal, Runtime};
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
}
