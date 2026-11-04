mod access;
pub(crate) mod backdrop;
mod caps;
#[cfg(feature = "scene")]
mod capture;
mod clipboard;
mod colour;
mod glyphs;
mod graphics;
pub(crate) mod hud;
mod input;
mod keys;
#[cfg(all(fuzzing, unix))]
pub(crate) use caps::fuzz_reply;
#[cfg(fuzzing)]
pub(crate) use keys::fuzz_strip;
mod mouse;
mod notify;
mod paint;
mod pixels;
mod placeholder;
mod run;
mod screen;
mod scroll;
mod signals;
mod term;
mod transcript;
mod write;

pub use access::{Access, FocusCursor};
pub use backdrop::backdrop;
pub use caps::{Backdrop, Borders, Caps, ColorDepth, Cursor, Fills, Keyboard, Notify};
kittest_pub! {
    #[doc(hidden)]
    pub use glyphs::Glyphs;
}
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use glyphs::glyphs;
kittest_pub! {
    #[doc(hidden)]
    pub use graphics::{base64, detect};
}
pub use graphics::{Mux, Protocol};
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use graphics::{blocks, png, sextants};
kittest_pub! {
    #[doc(hidden)]
    pub use keys::map_key;
}
pub use mouse::MouseTracking;
#[cfg(any(test, feature = "kittest"))]
#[doc(hidden)]
pub use mouse::cell_point;
kittest_pub! {
    #[doc(hidden)]
    pub use mouse::map_mouse;
}
pub use notify::{Activity, activity, bell, notify, progress};
pub use paint::{Cell, Grid, Ink, Lines, Underline};
pub(crate) use run::run;
pub use run::{Hooks, calibrate};
#[cfg(test)]
pub(crate) use run::{present, step};
pub use screen::Screen;
pub use pixels::{pixels, set_pixels};
pub use term::{caps, focused, print_above, set_mouse_tracking, set_title, suspend};
