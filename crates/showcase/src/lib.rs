//! Showcase apps that use only what a dependent crate can reach in `sigil-ui`.

use sigil::layout::{LayoutStyle, Position};
use sigil::prelude::{AppConfig, Length};

/// Sun and sky for one place, painted as a watercolour wash.
pub mod almanac;
/// A card with a turning border light, a staggered headline, and a rolling number.
pub mod beam;
/// A desk instrument: four knobs that redraw a harmonograph trace on a scope.
pub mod console;
/// One painted plate graded live with filters, blend modes, and a mask.
pub mod darkroom;
/// Flashcards that turn over in 3D on a spring.
pub mod deck;
/// A pomodoro timer on frosted glass over stage lights.
pub mod focus;
/// A scrolling list of glowing cards that fade in as they come into view.
pub mod gallery;
/// An editorial index of parts set in a serif display face.
pub mod index;
/// A marquee, a shimmering loading card, a toast stack and a bottom sheet.
pub mod kit;
/// A glass session tray that is dragged, flung, and snaps to its stops.
pub mod tray;

/// The window size every showcase is designed for, in logical pixels.
pub const SIZE: [u32; 2] = [1180, 720];

/// The id of the page canvas that a showcase draws into in a browser.
pub const CANVAS: &str = "app";

/// The window of a showcase: its title, the shared size, and the page canvas in a browser.
#[must_use]
pub fn window(title: &str) -> AppConfig {
    AppConfig::new(title).size(SIZE[0], SIZE[1]).canvas(CANVAS)
}

pub(crate) fn overlay() -> LayoutStyle {
    LayoutStyle::flex_column()
        .position(Position::Absolute)
        .inset(Length::Px(0.0))
}
