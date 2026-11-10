use crate::events::handlers::{EventArg, EventKind, Phase, fire};
use crate::hal::events::GestureEvent;
use crate::kernel::Cx;
use crate::layout::LayoutEngine;

use super::EventRouter;

impl EventRouter {
    pub(super) fn route_gesture(
        &mut self,
        cx: &Cx,
        layout: &LayoutEngine,
        mut event: GestureEvent,
    ) {
        self.bubble(cx, layout, event.position, |id, local| {
            event.position = local;
            let _ = fire(
                cx,
                id,
                EventKind::Gesture,
                Phase::Bubble,
                EventArg::Gesture(&mut event),
            );
            event.propagation_stopped()
        });
    }
}
