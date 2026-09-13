use std::sync::atomic::{AtomicBool, Ordering};

static EXIT: AtomicBool = AtomicBool::new(false);

pub fn request_exit() {
    EXIT.store(true, Ordering::SeqCst);
}

#[must_use]
pub fn take_exit() -> bool {
    EXIT.swap(false, Ordering::SeqCst)
}

pub fn clear_exit() {
    EXIT.store(false, Ordering::SeqCst);
}

#[cfg(test)]
#[path = "exit_tests.rs"]
mod tests;
