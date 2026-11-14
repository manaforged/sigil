use std::cell::Cell;

use crate::reactive::{Owner, Signal, signal};
use crate::style::Backdrop;

thread_local! {
    static SYSTEM_BACKDROP: Cell<Option<Signal<Option<Backdrop>>>> = const { Cell::new(None) };
}

/// The operating system's light or dark appearance, updated when it changes.
#[must_use]
pub fn system_backdrop() -> Signal<Option<Backdrop>> {
    SYSTEM_BACKDROP.with(|slot| {
        slot.get()
            .filter(|value| value.is_alive())
            .unwrap_or_else(|| {
                let value = Owner::new("sigil::system_backdrop", None)
                    .with(|| signal(detect_system_backdrop()));
                slot.set(Some(value));
                value
            })
    })
}

pub(crate) fn set_system_backdrop(backdrop: Backdrop) {
    let _ = system_backdrop().try_set(Some(backdrop));
}

#[cfg(any(feature = "desktop", feature = "terminal"))]
fn detect_system_backdrop() -> Option<Backdrop> {
    dark_light::detect().ok().and_then(from_dark_light)
}

#[cfg(not(any(feature = "desktop", feature = "terminal")))]
const fn detect_system_backdrop() -> Option<Backdrop> {
    None
}

#[cfg(any(feature = "desktop", feature = "terminal"))]
pub(crate) const fn from_dark_light(mode: dark_light::Mode) -> Option<Backdrop> {
    match mode {
        dark_light::Mode::Dark => Some(Backdrop::Dark),
        dark_light::Mode::Light => Some(Backdrop::Light),
        dark_light::Mode::Unspecified => None,
    }
}
