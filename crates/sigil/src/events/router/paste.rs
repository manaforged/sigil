use crate::events::PasteEvent;
use crate::events::hit_test::visible;
use crate::events::pointer_dispatch::path_to_node;
use crate::kernel::{Cx, NodeBuilder};
use crate::layout::LayoutEngine;

use super::EventRouter;

type PasteHandler = Box<dyn Fn(&mut PasteEvent) + 'static>;

#[derive(Default)]
pub(crate) struct PasteHandlers {
    handlers: Vec<PasteHandler>,
}

impl PasteHandlers {
    fn invoke(&self, event: &mut PasteEvent) {
        for handler in &self.handlers {
            handler(event);
            if event.propagation_stopped() {
                return;
            }
        }
    }
}

impl core::fmt::Debug for PasteHandlers {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PasteHandlers").finish_non_exhaustive()
    }
}

pub(crate) fn on_paste(
    builder: NodeBuilder,
    handler: impl Fn(&mut PasteEvent) + 'static,
) -> NodeBuilder {
    builder.mutate(move |node| {
        if node.get_extension::<PasteHandlers>().is_none() {
            let _ = node.attach(PasteHandlers::default());
        }
        if let Some(handlers) = node.get_extension_mut::<PasteHandlers>() {
            handlers.handlers.push(Box::new(handler));
        }
    })
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
                handlers.invoke(&mut event);
            }
            if event.propagation_stopped() {
                return;
            }
        }
    }
}
