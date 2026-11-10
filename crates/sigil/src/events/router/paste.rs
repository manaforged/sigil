use crate::events::handlers::{EventArg, EventKind, Phase, fire};
use crate::events::hit_test::visible;
use crate::events::pointer_dispatch::path_to_node;
use crate::kernel::Cx;
use crate::layout::LayoutEngine;

use super::EventRouter;

impl EventRouter {
    pub(super) fn route_paste(&mut self, cx: &mut Cx, layout: &LayoutEngine, text: &str) {
        crate::events::note_focus_source(true);
        self.focus.focus_active_scope(cx, layout);
        let Some(focused) = self.focus.focus().filter(|id| visible(cx, *id)) else {
            return;
        };
        for id in path_to_node(cx, focused).iter().rev().copied() {
            let mut handled = false;
            let _ = fire(
                cx,
                id,
                EventKind::Paste,
                Phase::Bubble,
                EventArg::Paste(text, &mut handled),
            );
            if handled {
                return;
            }
        }
    }
}
