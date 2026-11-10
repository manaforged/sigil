use crate::events::{ModifiersState, ScrollPhase};
use crate::geometry::Point;

/// The kind of a trackpad or touch gesture.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum GestureKind {
    /// The pinch gesture kind.
    Pinch(f64),
    /// The rotate gesture kind.
    Rotate(f32),
    /// The pan gesture kind.
    Pan(Point),
    /// The double tap gesture kind.
    DoubleTap,
}

/// A trackpad or touch gesture on a node.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GestureEvent {
    /// The kind.
    pub kind: GestureKind,
    /// The phase.
    pub phase: ScrollPhase,
    /// The position.
    pub position: Point,
    /// The modifiers.
    pub modifiers: ModifiersState,
    propagation_stopped: bool,
}

impl GestureEvent {
    pub(crate) const fn new(kind: GestureKind, phase: ScrollPhase, position: Point) -> Self {
        Self {
            kind,
            phase,
            position,
            modifiers: ModifiersState::empty(),
            propagation_stopped: false,
        }
    }

    /// Stops the event from reaching ancestor nodes.
    pub const fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    /// Returns whether propagation stopped is set.
    #[must_use]
    pub const fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }
}
