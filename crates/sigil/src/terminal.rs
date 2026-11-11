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
#[cfg(fuzz_replies)]
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
pub use caps::{
    Backdrop, Borders, Capabilities, ClipRoute, ColorDepth, Cursor, Erase, Fills, Keyboard,
    Notices, Notify,
};
pub use glyphs::Glyphs;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use glyphs::glyphs;
pub use graphics::{Mux, Protocol};
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use graphics::{blocks, png, sextants};
kittest_pub! {
    #[doc(hidden)]
    pub use keys::map_key;
}
pub use mouse::MouseTracking;
#[cfg(feature = "kittest")]
#[doc(hidden)]
pub use mouse::cell_point;
kittest_pub! {
    #[doc(hidden)]
    pub use mouse::map_mouse;
}
pub use notify::{Activity, activity, bell, notify, progress};
pub use paint::{Cell, Grid, Ink, Lines, Underline};
pub use pixels::{pixels, set_pixels};
#[cfg(test)]
pub(crate) use run::present;
pub(crate) use run::run;
#[cfg(all(test, any(feature = "dialog", hot_patch)))]
pub(crate) use run::step;
pub use run::{Hooks, calibrate};
pub use screen::Screen;
pub use term::{caps, focused, print_above, set_mouse_tracking, set_title, suspend};
