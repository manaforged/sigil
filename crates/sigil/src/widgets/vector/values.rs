use crate::style::{Color, ColorAtom};

/// Path fill paint.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PathFill {
    /// The solid path fill.
    Solid(ColorAtom),
    /// The linear path fill.
    Linear {
        /// The start.
        start: (f32, f32),
        /// The end.
        end: (f32, f32),
        /// The stops.
        stops: Vec<(f32, Color)>,
    },
    /// The radial path fill.
    Radial {
        /// The center.
        center: (f32, f32),
        /// The radius.
        radius: f32,
        /// The stops.
        stops: Vec<(f32, Color)>,
    },
}
