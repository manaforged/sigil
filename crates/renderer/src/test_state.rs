use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::slug::{TextTuning, set_text_tuning, text_tuning};

fn lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub struct GlobalTextState {
    _guard: MutexGuard<'static, ()>,
    tuning: TextTuning,
}

impl Drop for GlobalTextState {
    fn drop(&mut self) {
        set_text_tuning(self.tuning);
    }
}

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
