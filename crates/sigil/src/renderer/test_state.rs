use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::renderer::raster_paint::{set_test_atlas_size_override, test_atlas_size_override};
use crate::renderer::slug::{TextTuning, set_text_tuning, text_tuning};

fn lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

pub struct GlobalTextState {
    _guard: MutexGuard<'static, ()>,
    tuning: TextTuning,
    atlas: (u32, u32),
}

impl Drop for GlobalTextState {
    fn drop(&mut self) {
        set_text_tuning(self.tuning);
        let (initial, max) = self.atlas;
        set_test_atlas_size_override(initial, max);
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
        atlas: test_atlas_size_override(),
    }
}
