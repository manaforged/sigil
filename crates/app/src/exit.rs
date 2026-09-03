//! Process-wide exit latch for [`crate::launch`] and `run_tui`.

use std::sync::atomic::{AtomicBool, Ordering};

static EXIT: AtomicBool = AtomicBool::new(false);

/// Ask `launch` and `run_tui` to return after the current frame.
pub fn request_exit() {
    EXIT.store(true, Ordering::SeqCst);
}

/// True once if [`request_exit`] ran since the last take.
#[must_use]
pub fn take_exit() -> bool {
    EXIT.swap(false, Ordering::SeqCst)
}

/// Drop a leftover latch before a new host loop starts.
pub fn clear_exit() {
    EXIT.store(false, Ordering::SeqCst);
}

#[cfg(test)]
#[path = "exit_tests.rs"]
mod tests;
