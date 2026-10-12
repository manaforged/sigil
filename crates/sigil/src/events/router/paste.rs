use crate::events::PasteEvent;
use crate::events::handlers::until_stopped;
use crate::events::hit_test::visible;
use crate::events::pointer_dispatch::path_to_node;
use crate::kernel::Cx;
use crate::layout::LayoutEngine;

use super::EventRouter;

type PasteHandler = crate::events::handlers::Handler<PasteEvent>;

#[derive(Default)]
pub(crate) struct PasteHandlers {
    pub(crate) handlers: Vec<PasteHandler>,
}

impl core::fmt::Debug for PasteHandlers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PasteHandlers").finish_non_exhaustive()
    }
}

impl EventRouter {
    pub(super) fn route_paste(&mut self, cx: &mut Cx, layout: &LayoutEngine, text: String) {
        crate::events::note_focus_source(true);
        self.focus.focus_active_scope(cx, layout);
        let Some(focused) = self.focus.focus().filter(|id| visible(cx, *id)) else {
            return;
        };
        let mut event = PasteEvent::new(text);
        for id in path_to_node(cx, focused).iter().rev().copied() {
            if let Some(handlers) = cx
                .get(id)
                .and_then(|node| node.get_extension::<PasteHandlers>())
            {
                until_stopped(
                    &handlers.handlers,
                    &mut event,
                    PasteEvent::propagation_stopped,
                );
            }
            if event.propagation_stopped() {
                return;
            }
        }
    }
}
