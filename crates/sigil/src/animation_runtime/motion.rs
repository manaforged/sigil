use core::cell::Cell;

thread_local! {
    static REDUCED_MOTION: Cell<bool> = const { Cell::new(false) };
}

pub struct MotionPrefs;

impl MotionPrefs {
    #[must_use]
    pub fn reduced() -> bool {
        REDUCED_MOTION.with(Cell::get)
    }

    pub fn set(preferred: bool) {
        REDUCED_MOTION.with(|cell| cell.set(preferred));
        if preferred {
            crate::animation_runtime::state::finish_all();
        }
    }
}
