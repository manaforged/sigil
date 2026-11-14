use crate::events::{Modifiers, ScrollPhase};
use crate::geometry::Point;

/// The kind of a trackpad or touch gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Gesture {
    /// A two-finger pinch; a positive delta zooms in and a negative delta zooms out.
    Pinch(f64),
    /// A two-finger rotation by a delta in degrees; a positive delta turns counterclockwise.
    Rotate(f32),
    /// A pan by a distance in logical pixels.
    Pan(Point),
    /// A double tap, sent once with the `Ended` phase.
    DoubleTap,
}

/// A trackpad or touch gesture on a node.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GestureEvent {
    pub(crate) kind: Gesture,
    pub(crate) phase: ScrollPhase,
    pub(crate) position: Point,
    pub(crate) modifiers: Modifiers,
    pub(super) propagation_stopped: bool,
}

impl GestureEvent {
    pub(crate) const fn new(kind: Gesture, phase: ScrollPhase, position: Point) -> Self {
        Self {
            kind,
            phase,
            position,
            modifiers: Modifiers::empty(),
            propagation_stopped: false,
        }
    }

    /// Which gesture it is, with its delta.
    #[must_use]
    pub const fn kind(&self) -> Gesture {
        self.kind
    }

    /// The stage of the gesture: started, moved, ended, or cancelled.
    #[must_use]
    pub const fn phase(&self) -> ScrollPhase {
        self.phase
    }

    /// The pointer position at the time of the gesture, in logical pixels.
    #[must_use]
    pub const fn position(&self) -> Point {
        self.position
    }

    /// The modifiers held during the gesture.
    #[must_use]
    pub const fn modifiers(&self) -> Modifiers {
        self.modifiers
    }
}
