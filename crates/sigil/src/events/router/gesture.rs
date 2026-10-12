use crate::events::handlers::until_stopped;
use crate::hal::events::GestureEvent;
use crate::kernel::Cx;
use crate::layout::LayoutEngine;
use crate::reactive::owner_by_id;

use super::EventRouter;

type GestureHandler = crate::events::handlers::Handler<GestureEvent>;

#[derive(Default)]
pub(crate) struct GestureHandlers {
    pub(crate) handlers: Vec<GestureHandler>,
}

impl core::fmt::Debug for GestureHandlers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GestureHandlers").finish_non_exhaustive()
    }
}

impl EventRouter {
    pub(super) fn route_gesture(
        &mut self,
        cx: &Cx,
        layout: &LayoutEngine,
        mut event: GestureEvent,
    ) {
        self.bubble(cx, layout, event.position, |id, local| {
            if let Some(node) = cx.get(id)
                && let Some(handlers) = node.get_extension::<GestureHandlers>()
            {
                event.position = local;
                let _ = owner_by_id(node.owner, || {
                    until_stopped(
                        &handlers.handlers,
                        &mut event,
                        GestureEvent::propagation_stopped,
                    );
                });
            }
            event.propagation_stopped()
        });
    }
}
