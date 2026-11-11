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
    #[derive(Debug)]
    pub struct MotionPrefs;
}

impl MotionPrefs {
    #[must_use]
    pub(crate) fn reduced() -> bool {
        REDUCED_MOTION.with(Cell::get)
    }

    kittest_pub! {
        #[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
        pub fn set(preferred: bool) {
            REDUCED_MOTION.with(|cell| cell.set(preferred));
            if preferred {
                crate::animation::runtime::state::finish_all();
            }
        }
    }
}
