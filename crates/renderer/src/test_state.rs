//! Serialised access to the process-global text-tuning test hook.
//!
//! [`set_text_tuning`](crate::set_text_tuning) writes process-global
//! atomics. Two tests in the same binary that both use it race whenever
//! the harness runs tests as THREADS: measured on 2026-07-20, such a
//! suite failed 1 run in 4 under the default harness, while the same
//! tests under `cargo nextest` passed 6 of 6 because nextest gives each
//! test its own process.
//!
//! A suite whose result depends on the harness is not a suite. Take
//! [`lock_global_text_state`] for the duration of any test that touches
//! the hook: it serialises those tests within a process and restores
//! the previous values on drop, so a test cannot leak state into whatever
//! runs next.

use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::slug::{TextTuning, set_text_tuning, text_tuning};

fn lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Exclusive hold on the global text-tuning test state.
///
/// Restores the tuning that was in effect when it was taken, so a
/// failing test cannot poison its neighbours.
pub struct GlobalTextState {
    _guard: MutexGuard<'static, ()>,
    tuning: TextTuning,
}

impl Drop for GlobalTextState {
    fn drop(&mut self) {
        set_text_tuning(self.tuning);
    }
}

/// Serialise access to the global text-tuning hook for the caller's scope.
///
/// Poisoning is ignored on purpose: a panicking test leaves the globals in
/// some arbitrary state, and the point of this guard is that the NEXT test
/// still gets a clean, restored one rather than inheriting the mess.
#[must_use]
pub fn lock_global_text_state() -> GlobalTextState {
    let guard = lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    GlobalTextState {
        _guard: guard,
        tuning: text_tuning(),
    }
}
