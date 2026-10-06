use crate::events::{ModifiersState, ScrollPhase};
use crate::geometry::Point;

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum GestureKind {
    Pinch(f64),
    Rotate(f32),
    Pan(Point),
    DoubleTap,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GestureEvent {
    pub kind: GestureKind,
    pub phase: ScrollPhase,
    pub position: Point,
    pub modifiers: ModifiersState,
    propagation_stopped: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureEventInit {
    pub kind: GestureKind,
    pub phase: ScrollPhase,
    pub position: Point,
    pub modifiers: ModifiersState,
}

impl GestureEvent {
    #[must_use]
    pub const fn new(init: GestureEventInit) -> Self {
        Self {
            kind: init.kind,
            phase: init.phase,
            position: init.position,
            modifiers: init.modifiers,
            propagation_stopped: false,
        }
    }

    pub const fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    #[must_use]
    pub const fn propagation_stopped(&self) -> bool {
        self.propagation_stopped
    }
}
