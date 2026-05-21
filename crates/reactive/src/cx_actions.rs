//! Action-dispatch methods on [`Context`] (DR-046 §4 + §6).
//!
//! Extracted from `cx.rs` to keep that file under the 400-line cap.
//! Three pieces of API live here:
//! [`Context::action`] (register), [`Context::dispatch`]
//! (fire along the focus stack), and [`dispatch_replay_action`] (the
//! deferred-replay seam used by the Paint-phase defer queue,
//! DR-047 §5).

use core::any::Any;
use core::panic::Location;

use actions::{Action, ActionRegistry, DispatchCx, HandlerKey};
use smallvec::SmallVec;

use crate::cx::Context;
use crate::focus::FocusHandle;
use crate::runtime::with_rt;

impl<T: 'static> Context<'_, T> {
    /// Register an action handler keyed by `handle`.
    ///
    /// The handler fires only when `handle` is on the focus stack at
    /// the time of dispatch. Most-nested handle wins; outer handles
    /// in the stack fire only if the inner handler called
    /// [`DispatchCx::propagate`].
    pub fn action<A: Action>(
        &mut self,
        handle: &FocusHandle,
        handler: impl FnMut(&A, &mut DispatchCx) + 'static,
    ) {
        let key = HandlerKey::from(handle.id());
        with_rt(|rt| {
            let registry = rt.action_registry.get_or_insert_with(ActionRegistry::new);
            registry.register::<A>(key, handler);
        });
    }

    /// Dispatch `action` along the current focus stack.
    ///
    /// Walks leaf-first, firing the most-nested handler registered
    /// for `A`. If that handler calls [`DispatchCx::propagate`],
    /// dispatch continues to the next outer focus chain entry that
    /// has a handler.
    ///
    /// Re-entrant: a handler may itself call `dispatch` to
    /// dispatch a different action through the same registry. The
    /// chain captured at the top-level call is preserved — re-entrant
    /// dispatch uses the *current* focus stack (which may have been
    /// mutated by the handler).
    ///
    /// No `Clone` bound at the call site: the deferred-replay path
    /// uses [`Action::boxed_clone`] to capture an owned copy when
    /// dispatch is invoked during the Paint phase. Consumers only opt
    /// in to `Clone` at type definition (`#[derive(Clone, Action)]`).
    /// See the DR-046 2026-05-18 amendment.
    #[track_caller]
    pub fn dispatch<A: Action>(&mut self, action: &A) {
        // G4 hookup (DR-047 §6): writes during Paint are illegal. In
        // debug build the assert panics on the consumer's call site;
        // in release the action is cloned and queued for replay at
        // end_frame against the then-current focus stack (DR-047 §5).
        let painting = with_rt(|rt| rt.phase == crate::DrawPhase::Paint);
        if painting {
            // Debug build: panic via the standard write-seam assert,
            // so the message + call-site routing match every other
            // illegal-write diagnostic.
            with_rt(|rt| crate::phase::assert_paint_safe(rt, "Context::dispatch"));
            // Release build (debug_assert is compiled out): boxed-clone
            // the action so an owned trait object crosses the
            // defer/replay boundary. The replay closure consumes the
            // Box and routes through `ActionRegistry::dispatch_dyn`.
            let caller = Location::caller();
            let cloned: Box<dyn Action> = action.boxed_clone();
            let replay: crate::phase::ReplayFn = Box::new(move || {
                dispatch_replay_action(cloned.as_ref());
            });
            crate::phase::enqueue_deferred(replay, caller);
            return;
        }
        // Snapshot the chain into a stack-local; the focus_stack itself
        // may be mutated by the handler (e.g. handler calls `focus()`
        // on a different region) but that should not retarget the
        // dispatch already in flight.
        let chain: SmallVec<[HandlerKey; 8]> = with_rt(|rt| {
            rt.focus_stack
                .iter()
                .copied()
                .map(HandlerKey::from)
                .collect()
        });
        // Take the entry list out so handler bodies can mutate the
        // registry (re-entrant dispatch of a different action type,
        // or `on_action` registering a new handler). DR-046 §6.
        let Some(mut taken) = with_rt(|rt| {
            rt.action_registry
                .as_mut()
                .map(ActionRegistry::take_entries::<A>)
        }) else {
            return;
        };
        if taken.is_empty() {
            // Restore the (empty) marker so a later `register` lands
            // back in the same slot — preserves type-id stability.
            with_rt(|rt| {
                if let Some(registry) = rt.action_registry.as_mut() {
                    registry.restore_entries(taken);
                }
            });
            return;
        }
        let _result = ActionRegistry::dispatch_taken(&mut taken, action, &chain);
        with_rt(|rt| {
            if let Some(registry) = rt.action_registry.as_mut() {
                registry.restore_entries(taken);
            }
        });
    }
}

/// Re-dispatch a deferred action against the focus stack as it exists
/// at replay time (DR-047 §5).
///
/// Called from inside the deferred-write drain closure; phase is
/// Effect at replay time (set by [`crate::Runtime::end_frame`] before
/// the drain), so this routes through the normal non-deferred dispatch
/// path without re-entering the defer branch.
///
/// Operates on `&dyn Action` rather than a monomorphic `&A` — the
/// deferred queue holds `Box<dyn Action>` so the call site doesn't
/// have to carry an `A: Clone` bound (DR-046 2026-05-18 amendment).
fn dispatch_replay_action(action: &dyn Action) {
    let chain: SmallVec<[HandlerKey; 8]> = with_rt(|rt| {
        rt.focus_stack
            .iter()
            .copied()
            .map(HandlerKey::from)
            .collect()
    });
    let type_id = Any::type_id(action.as_any());
    let Some(mut taken) = with_rt(|rt| {
        rt.action_registry
            .as_mut()
            .map(|registry| registry.take_entries_by_id(type_id))
    }) else {
        return;
    };
    if taken.is_empty() {
        with_rt(|rt| {
            if let Some(registry) = rt.action_registry.as_mut() {
                registry.restore_entries(taken);
            }
        });
        return;
    }
    let _ = ActionRegistry::dispatch_taken_dyn(&mut taken, action, &chain);
    with_rt(|rt| {
        if let Some(registry) = rt.action_registry.as_mut() {
            registry.restore_entries(taken);
        }
    });
}
