use core::cell::Cell;
use std::thread::LocalKey;

use crate::reactive::{Owner, Signal, signal};

type Slot = Cell<Option<Signal<bool>>>;

thread_local! {
    static REDUCED_MOTION: Slot = const { Cell::new(None) };
    static REDUCED_TRANSPARENCY: Slot = const { Cell::new(None) };
}

#[derive(Clone, Copy)]
enum Pref {
    Motion,
    Transparency,
}

impl Pref {
    const fn slot(self) -> &'static LocalKey<Slot> {
        match self {
            Self::Motion => &REDUCED_MOTION,
            Self::Transparency => &REDUCED_TRANSPARENCY,
        }
    }

    fn store(self) -> Signal<bool> {
        self.slot().with(|slot| {
            slot.get()
                .filter(|value| value.is_alive())
                .unwrap_or_else(|| {
                    let value = Owner::new("sigil::accessibility", None).with(|| signal(false));
                    slot.set(Some(value));
                    value
                })
        })
    }

    fn peek(self) -> bool {
        self.slot()
            .with(Cell::get)
            .and_then(Signal::try_peek)
            .unwrap_or(false)
    }

    fn get(self) -> bool {
        self.store().get()
    }

    #[cfg(host)]
    fn write(self, value: bool) {
        let _ = self.store().try_set(value);
        crate::reactive::Runtime::mark_frame_dirty();
    }
}

/// Returns true when the app or the OS asks for reduced motion; an effect that reads it reruns when it changes.
#[must_use]
pub fn reduced_motion() -> bool {
    Pref::Motion.get()
}

/// Returns true when the app or the OS asks for reduced transparency; an effect that reads it reruns when it changes.
#[must_use]
pub fn reduced_transparency() -> bool {
    Pref::Transparency.get()
}

kittest_pub! {
    /// The reduced-motion and reduced-transparency settings of this thread.
    #[derive(Debug)]
    pub struct MotionPrefs;
}

impl MotionPrefs {
    #[must_use]
    pub(crate) fn reduced() -> bool {
        Pref::Motion.peek()
    }

    #[must_use]
    pub(crate) fn reduced_transparency() -> bool {
        Pref::Transparency.peek()
    }

    kittest_pub! {
        /// Sets whether motion is reduced; true also finishes every running animation.
        #[cfg(host)]
        pub fn set(preferred: bool) {
            Pref::Motion.write(preferred);
            if preferred {
                crate::animation::runtime::state::finish_all();
            }
        }
    }

    kittest_pub! {
        /// Sets whether transparency is reduced.
        #[cfg(host)]
        pub fn transparency(reduced: bool) {
            Pref::Transparency.write(reduced);
        }
    }
}
