//! Quiescence drain helpers for the reactive runtime.
//!
//! Three module-scope helpers extracted from `runtime.rs` to keep
//! that file under the 400-line cap:
//!
//! - [`drain_pending_completions`] — empties the G5 task-completion
//!   queue, bounded by [`MAX_QUIESCENCE_PASSES`](crate::runtime::MAX_QUIESCENCE_PASSES).
//! - [`push_pending_completion`] — the cross-thread enqueue seam used
//!   by [`crate::async_cx::MainThreadDispatcher`] adapters.
//! - [`drain_observers_to_fixpoint`] — repeatedly fans observers out
//!   until the queue is stable, bounded by
//!   [`MAX_DRAIN_PASSES`](crate::phase::MAX_DRAIN_PASSES).
//!
//! All three are `pub(crate)` and called from the phase-transition
//! methods on [`Runtime`](crate::Runtime).

use crate::phase::MAX_DRAIN_PASSES;
use crate::runtime::{MAX_QUIESCENCE_PASSES, OwnerKey, PendingCompletion, with_rt};

/// Drain `pending_completions` once — handler bodies may push new
/// entries, which are picked up in subsequent iterations of the same
/// inner loop. Owner-aliveness is checked per entry; if the owner has
/// been disposed since the completion was enqueued, the callback is
/// dropped silently (DR-007 invariant 4).
///
/// # Panics
///
/// Panics if the loop exceeds `MAX_QUIESCENCE_PASSES` — the same cap
/// the wrapping `run_until_idle` uses, applied locally so a runaway
/// chain of handler-enqueued completions surfaces with a useful
/// message before the outer cap fires.
#[allow(
    clippy::panic,
    reason = "DR-048 §4 mandates a panic on completion-drain divergence — the cap catches runaway handler-enqueue cycles"
)]
pub(crate) fn drain_pending_completions() {
    for _pass in 0..MAX_QUIESCENCE_PASSES {
        let next = with_rt(|rt| {
            if rt.pending_completions.is_empty() {
                None
            } else {
                Some(rt.pending_completions.remove(0))
            }
        });
        let Some(entry) = next else {
            return;
        };
        // Skip silently if the owner died between enqueue and drain.
        let alive = with_rt(|rt| rt.owners.contains(entry.owner));
        if alive {
            (entry.callback)();
        }
    }
    panic!(
        "drain_pending_completions exceeded MAX_QUIESCENCE_PASSES ({MAX_QUIESCENCE_PASSES}) — \
         a completion handler keeps enqueuing more completions in a tight loop"
    );
}

/// Push a completion callback onto the thread-local runtime's
/// `pending_completions` queue.
///
/// `pub(crate)` so [`crate::async_cx::MainThreadDispatcher`] adapters
/// can push from their post-to-main implementation. Must be called on
/// the thread that owns the reactive runtime.
pub(crate) fn push_pending_completion(
    owner: OwnerKey,
    callback: Box<dyn FnOnce() + Send + 'static>,
) {
    with_rt(|rt| {
        rt.pending_completions
            .push(PendingCompletion { owner, callback });
    });
}

/// Drain the a11y-dirty flag — W5.2 / DR-050 §G10-phase-2.
///
/// Called from [`crate::Runtime::end_frame`] after the deferred-write
/// replay and the observer fixpoint pass, when every signal mutation
/// produced this frame has settled. If the runtime has had its dirty
/// flag flipped (by an `entity_a11y_observer` effect body that read a
/// changed signal), invoke the installed publisher closure; the
/// publisher itself is responsible for snapshotting the kernel `Cx`
/// into a `TreeUpdate` and forwarding it to the bridge.
///
/// No-op when `a11y_dirty` is false (the steady-state path after a
/// frame where no observed signal changed). No-op without a panic when
/// no publisher has been installed — the bridge install is optional
/// and the kernel must still drive frames in tests that do not exercise
/// the a11y surface.
///
/// The dirty flag is cleared *before* the publisher runs so a
/// publisher that itself touches a tracked signal (rare but legal)
/// re-flips dirty for the next frame instead of being clobbered by a
/// post-drain reset.
pub(crate) fn drain_a11y_updates() {
    let dirty = with_rt(|rt| rt.a11y_dirty);
    if !dirty {
        return;
    }
    let mut publisher = with_rt(|rt| rt.a11y_publisher.take());
    with_rt(|rt| rt.a11y_dirty = false);
    if let Some(ref mut publish) = publisher {
        publish();
    }
    with_rt(|rt| {
        // Restore the publisher unless `install_a11y_publisher` was
        // called *during* the publish closure (in which case the
        // freshly-installed publisher wins).
        if rt.a11y_publisher.is_none() {
            rt.a11y_publisher = publisher;
        }
    });
}

/// Drain `pending_observers` until the queue is stable, with a fixed
/// `MAX_DRAIN_PASSES` cap on the number of observer-fan-out passes.
///
/// Each pass calls [`crate::entity_observer::drain_pending_observers`]
/// (which loops over the queue until empty); we then check whether the
/// loop produced new pending entries — if so, fire again, bounded.
///
/// # Panics
///
/// Panics if the drain does not converge within `MAX_DRAIN_PASSES` —
/// signals a runaway observer cycle (callback A writes signal X,
/// which fires callback A, ad infinitum).
#[allow(
    clippy::panic,
    reason = "DR-047 §8 mandates a panic on observer-drain divergence — the cap is a safety net for a runaway observer cycle, not a recoverable error"
)]
pub(crate) fn drain_observers_to_fixpoint() {
    for _pass in 0..MAX_DRAIN_PASSES {
        let had_pending = with_rt(|rt| !rt.pending_observers.is_empty());
        if !had_pending {
            return;
        }
        crate::entity_observer::drain_pending_observers();
    }
    let remaining = with_rt(|rt| rt.pending_observers.len());
    assert!(
        remaining == 0,
        "drain_observers_to_fixpoint exceeded MAX_DRAIN_PASSES ({MAX_DRAIN_PASSES}) — \
         {remaining} pending observer(s) still queued; suspect a runaway notify cycle"
    );
}
