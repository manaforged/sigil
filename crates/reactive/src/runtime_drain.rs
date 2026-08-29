use crate::phase::MAX_DRAIN_PASSES;
use crate::runtime::{MAX_QUIESCENCE_PASSES, OwnerKey, PendingCompletion, with_rt};

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

pub(crate) fn push_pending_completion(
    owner: OwnerKey,
    callback: Box<dyn FnOnce() + Send + 'static>,
) {
    with_rt(|rt| {
        rt.pending_completions
            .push(PendingCompletion { owner, callback });
    });
}

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
