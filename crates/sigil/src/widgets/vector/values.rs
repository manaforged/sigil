use crate::style::{Color, ColorAtom};

/// Path fill paint.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PathFill {
    /// One color fills the whole path.
    Solid(ColorAtom),
    /// A gradient whose colors change along the line from `start` to `end`.
    Linear {
        /// The point where offset 0 lies, as (x, y).
        start: (f32, f32),
        /// The point where offset 1 lies, as (x, y).
        end: (f32, f32),
        /// The color stops, each an offset from 0 to 1 and its color.
        stops: Vec<(f32, Color)>,
    },
    /// A gradient whose colors change outward from `center` to a circle of `radius`.
    Radial {
        /// The point where offset 0 lies, as (x, y).
        center: (f32, f32),
        /// The distance from `center` at which offset 1 lies.
        radius: f32,
        /// The color stops, each an offset from 0 to 1 and its color.
        stops: Vec<(f32, Color)>,
    },
}
