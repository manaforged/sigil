//! Showcase apps that use only what a dependent crate can reach in `sigil-ui`.

use sigil::layout::{LayoutStyle, Position};
use sigil::prelude::Length;

/// Sun and sky for one place, painted as a watercolour wash.
pub mod almanac;
/// A card with a turning border light, a staggered headline, and a rolling number.
pub mod beam;
/// One painted plate graded live with filters, blend modes, and a mask.
pub mod darkroom;
/// Flashcards that turn over in 3D on a spring.
pub mod deck;
/// A pomodoro timer on frosted glass over stage lights.
pub mod focus;
/// A glass session tray that is dragged, flung, and snaps to its stops.
pub mod tray;

/// The window size every showcase is designed for, in logical pixels.
pub const SIZE: [u32; 2] = [1180, 720];

pub(crate) fn overlay() -> LayoutStyle {
    LayoutStyle::flex_column()
        .position(Position::Absolute)
        .inset(Length::Px(0.0))
}
