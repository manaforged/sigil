use crate::events::FileDropEvent;
use crate::events::handlers::dispatch;
use crate::events::pointer_dispatch::path_to_node;
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;

use super::EventRouter;

impl EventRouter {
    pub(super) fn route_file_drop(
        &mut self,
        cx: &Cx,
        layout: &LayoutEngine,
        event: &FileDropEvent,
    ) {
        let at_pointer = self
            .last_pointer
            .as_ref()
            .and_then(|pointer| self.hit_cache.hit_path(cx, layout, pointer.position));
        let path = at_pointer.or_else(|| self.focus.focus().map(|id| path_to_node(cx, id)));
        let Some(path) = path else {
            broadcast_file_drop(cx, event);
            return;
        };
        for id in path.iter().rev().copied() {
            if dispatch(cx, id, |handlers| handlers.invoke_file_drop(event)) == Some(true) {
                return;
            }
        }
    }
}

fn broadcast_file_drop(cx: &Cx, event: &FileDropEvent) {
    let mut stack: Vec<NodeId> = cx.roots().to_vec();
    while let Some(id) = stack.pop() {
        let Some(node) = cx.get(id) else {
            continue;
        };
        let _ = dispatch(cx, id, |handlers| handlers.invoke_file_drop(event));
        stack.extend(node.children().iter().copied());
    }
}
