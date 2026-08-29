use sigil_hal::Rect;
use sigil_kernel::{Cx, NodeId};
use sigil_layout::LayoutEngine;

use super::FocusManager;

impl FocusManager {
    pub(crate) fn is_focusable_target(
        &mut self,
        cx: &Cx,
        layout: &LayoutEngine,
        id: NodeId,
    ) -> bool {
        self.ensure_ring(cx, Some(layout));
        self.ring.contains(&id)
    }

    pub(crate) fn is_hittable_target(&self, cx: &Cx, layout: &LayoutEngine, id: NodeId) -> bool {
        let Some(node) = cx.get(id) else { return false };
        if !node.a11y.hittable {
            return false;
        }
        let Some(rect) = layout.result(id).map(|result| result.rect()) else {
            return false;
        };
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return false;
        }
        let root = document_root(cx, id);
        if let Some(viewport) = layout.result(root).map(|result| result.rect())
            && !rects_intersect(rect, viewport)
        {
            return false;
        }
        self.is_within_active_scope(cx, id)
    }

    pub(crate) fn is_within_active_scope(&self, cx: &Cx, id: NodeId) -> bool {
        let Some(top) = self.scope_stack.last() else {
            return true;
        };
        is_self_or_ancestor(cx, top.scope_root, id)
    }
}

fn is_self_or_ancestor(cx: &Cx, ancestor: NodeId, id: NodeId) -> bool {
    let mut current = Some(id);
    while let Some(node_id) = current {
        if node_id == ancestor {
            return true;
        }
        current = cx.parent(node_id);
    }
    false
}

fn document_root(cx: &Cx, id: NodeId) -> NodeId {
    let mut current = id;
    while let Some(parent) = cx.parent(current) {
        current = parent;
    }
    current
}

const fn rects_intersect(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.width
        && a.origin.x + a.width > b.origin.x
        && a.origin.y < b.origin.y + b.height
        && a.origin.y + a.height > b.origin.y
}

#[cfg(test)]
#[path = "targets_tests.rs"]
mod tests;
