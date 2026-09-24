use crate::hal::Rect;
use crate::kernel::{Cx, NodeId};
use crate::layout::LayoutEngine;

use crate::events::hit_test::visible;

use super::FocusManager;

impl FocusManager {
    pub(crate) fn focus_active_scope(&mut self, cx: &mut Cx, layout: &LayoutEngine) {
        if !self.scope_stack.is_empty()
            && self
                .focused
                .is_none_or(|id| !self.is_within_active_scope(cx, id))
        {
            self.focus_next(cx, Some(layout));
        }
    }

    pub(crate) fn is_focusable_target(
        &mut self,
        cx: &Cx,
        layout: &LayoutEngine,
        id: NodeId,
    ) -> bool {
        self.ensure_ring(cx, Some(layout));
        self.ring.contains(&id)
            && visible(cx, id)
            && cx.get(id).is_some_and(|node| node.a11y.is_focusable())
    }

    pub(crate) fn holds_live_focus(&self, cx: &Cx) -> bool {
        self.focused.is_some_and(|id| {
            visible(cx, id)
                && cx.get(id).is_some_and(|node| node.a11y.is_focusable())
                && self.is_within_active_scope(cx, id)
        })
    }

    pub(crate) fn focus_first(&mut self, cx: &mut Cx, layout: &LayoutEngine) {
        self.ensure_ring(cx, Some(layout));
        let ring = self.enabled_ring(cx);
        let first = super::scope::first_in_document_order(cx, |id| ring.contains(&id));
        self.set_focus(cx, first);
    }

    pub(crate) fn is_hittable_target(&self, cx: &Cx, layout: &LayoutEngine, id: NodeId) -> bool {
        let Some(node) = cx.get(id) else { return false };
        if !node.a11y.hittable || node.a11y.is_disabled() || !visible(cx, id) {
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

pub(crate) fn is_self_or_ancestor(cx: &Cx, ancestor: NodeId, id: NodeId) -> bool {
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
