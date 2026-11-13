mod access;
pub(crate) mod backdrop;
mod caps;
#[cfg(feature = "scene")]
mod capture;
mod clipboard;
mod colour;
mod glyphs;
mod graphics;
mod handle;
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
pub(crate) use backdrop::backdrop;
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
pub use handle::Terminal;
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
pub use notify::Activity;
pub use paint::{Cell, Grid, Ink, Lines, Underline};
pub use run::Hooks;
#[cfg(test)]
pub(crate) use run::present;
pub(crate) use run::run;
#[cfg(all(test, any(feature = "dialog", hot_patch)))]
pub(crate) use run::step;
pub use screen::Screen;
pub(crate) use term::{mouse_tracking, print_above};
