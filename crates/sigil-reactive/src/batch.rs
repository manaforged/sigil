//! Write batching.
//!
//! `batch(|| { ... })` coalesces signal writes so that dependent
//! effects run at most once per subscribed signal set per batch,
//! regardless of how many times those signals were written inside
//! the closure.

use crate::effect::drain_pending;
use crate::runtime::with_rt;

/// RAII guard that decrements `batch_depth` on drop. Keeps the
/// runtime consistent across panics inside `f`.
struct BatchGuard;

impl Drop for BatchGuard {
    fn drop(&mut self) {
        with_rt(|rt| rt.batch_depth -= 1);
    }
}

/// Run `f` with signal writes coalesced into a single effect drain.
///
/// While `f` is running, every `Signal::set` / `Signal::update` call
/// enqueues dependent effects instead of running them immediately.
/// When `f` returns, the queue drains in FIFO order and the
/// deduplication ensures each unique effect runs at most once per
/// batch exit.
///
/// Batches nest. Effects only drain when the outermost batch exits.
///
/// # Drain ordering
///
/// The drain runs WHILE `batch_depth > 0` (specifically, while this
/// call's increment is still in effect). That way any write triggered
/// by an effect that runs during the drain enqueues further effects
/// rather than running them synchronously — which would let an
/// already-pending effect run twice, once synchronously from the
/// inner write and once from its still-pending queue slot.
/// Decrementing happens after the drain, via `BatchGuard::drop`.
///
/// If `f` panics, the drain is skipped (the pending queue may be
/// inconsistent after a panic mid-write) but `batch_depth` still
/// decrements via the guard so subsequent `batch` calls behave
/// correctly.
pub fn batch<R>(f: impl FnOnce() -> R) -> R {
    with_rt(|rt| rt.batch_depth += 1);
    let guard = BatchGuard;
    let result = f();
    // Drain only if this is the outermost batch. We check via the
    // depth counter because `batch_depth == 1` here means ours is
    // the only active batch (the guard has not dropped yet).
    let is_outermost = with_rt(|rt| rt.batch_depth == 1);
    if is_outermost {
        drain_pending();
    }
    drop(guard);
    result
}
