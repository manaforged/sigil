use crate::events::hit_test::visible;
use crate::events::pointer_dispatch::path_to_node;
use crate::kernel::Cx;
use crate::layout::LayoutEngine;

use super::EventRouter;

type PasteHandler = Box<dyn Fn(&str) -> bool + 'static>;

#[derive(Default)]
pub(crate) struct PasteHandlers {
    pub(crate) handlers: Vec<PasteHandler>,
}

impl PasteHandlers {
    fn deliver(&self, text: &str) -> bool {
        self.handlers.iter().any(|handler| handler(text))
    }
}

impl core::fmt::Debug for PasteHandlers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PasteHandlers").finish_non_exhaustive()
    }
}

impl EventRouter {
    pub(super) fn route_paste(&mut self, cx: &mut Cx, layout: &LayoutEngine, text: &str) {
        crate::events::note_focus_source(true);
        self.focus.focus_active_scope(cx, layout);
        let Some(focused) = self.focus.focus().filter(|id| visible(cx, *id)) else {
            return;
        };
        for id in path_to_node(cx, focused).iter().rev().copied() {
            let handled = cx
                .get(id)
                .and_then(|node| node.get_extension::<PasteHandlers>())
                .is_some_and(|paste| paste.deliver(text));
            if handled {
                return;
            }
        }
    }
}
