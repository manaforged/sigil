use core::cell::Cell;

thread_local! {
    static REDUCED_MOTION: Cell<bool> = const { Cell::new(false) };
}

/// Returns true when the app or the OS asks for reduced motion.
#[must_use]
pub fn reduced_motion() -> bool {
    MotionPrefs::reduced()
}

kittest_pub! {
    /// The reduced-motion setting of this thread.
    #[derive(Debug)]
    pub struct MotionPrefs;
}

impl MotionPrefs {
    #[must_use]
    pub(crate) fn reduced() -> bool {
        REDUCED_MOTION.with(Cell::get)
    }

    kittest_pub! {
        /// Sets whether motion is reduced; true also finishes every running animation.
        #[cfg(host)]
        pub fn set(preferred: bool) {
            REDUCED_MOTION.with(|cell| cell.set(preferred));
            if preferred {
                crate::animation::runtime::state::finish_all();
            }
        }
    }
}
