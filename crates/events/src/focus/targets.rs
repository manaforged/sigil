//! AT-initiated action-target validation (RC-focus adversarial findings).
//!
//! Split out of `focus.rs` — that file was already over the 400-line
//! hygiene cap from a4972ca's layout-aware scope/ring work before this
//! module existed; this is a focus-ring-adjacent but genuinely distinct
//! concern (validating an `accesskit::ActionRequest`'s target, not
//! managing the ring or scope stack itself) split into its own child
//! module rather than compounding the existing file further.

use sigil_hal::Rect;
use sigil_kernel::{Cx, NodeId};
use sigil_layout::LayoutEngine;

use super::FocusManager;

impl FocusManager {
    /// Whether `id` is a legitimate focus target this frame — present,
    /// focusable, laid out (not `Display::None`), and inside the
    /// topmost active focus scope when one is open.
    ///
    /// Reuses the exact tab-ring membership test (`ensure_ring` /
    /// `build_scoped_ring`) so AT-initiated `accesskit::Action::Focus`
    /// requests are held to the same interactivity contract Tab/Shift+Tab
    /// already enforce — a platform request naming a node behind a modal,
    /// or an un-laid-out (`Display::None`) node, must not move focus
    /// there any more than a keyboard Tab press would.
    pub(crate) fn is_focusable_target(
        &mut self,
        cx: &Cx,
        layout: &LayoutEngine,
        id: NodeId,
    ) -> bool {
        self.ensure_ring(cx, Some(layout));
        self.ring.contains(&id)
    }

    /// Whether `id` is a legitimate AT click target this frame — present,
    /// laid out, hit-test-enabled (`a11y.hittable`), on-screen (see the
    /// viewport-intersection note below), and inside the topmost active
    /// focus scope when one is open.
    ///
    /// Mirrors what `route_pointer`'s hit-test path would already
    /// exclude (a hidden or `hit_test_enabled = false` node never
    /// receives a real click) plus the scope containment `sync_scopes`
    /// enforces for keyboard focus — an AT `Click` request that bypasses
    /// hit-testing entirely (it dispatches directly by node id, see
    /// `pointer_dispatch::invoke_node`) must not reach further than a
    /// real click could.
    ///
    /// RC-focus adversarial finding (viewport check): `hit_test.rs`
    /// itself does not implement ancestor-clip intersection (it checks
    /// only `has_rect && hittable && contains(rect, point)` — no
    /// scroll-clip walk), so there is no existing minimal check to reuse
    /// without building that machinery from scratch. This is the
    /// documented ponytail floor instead: reject when `id`'s rect does
    /// not intersect its document root's rect (the whole-window
    /// viewport). A node scrolled out of an `overflow: hidden` panel
    /// that is itself smaller than the window, but still within window
    /// bounds, is NOT caught by this check — that needs real
    /// ancestor-clip intersection (`LayoutResult::clips()`, already
    /// threaded through the renderer's paint walk but not hit-testing).
    /// Upgrade path: thread clip scopes through `hit_test.rs` first,
    /// then reuse the same walk here instead of the root-only check.
    pub(crate) fn is_hittable_target(&self, cx: &Cx, layout: &LayoutEngine, id: NodeId) -> bool {
        let Some(node) = cx.get(id) else { return false };
        if !node.a11y.hittable {
            return false;
        }
        let Some(rect) = layout.result(id).map(|result| result.rect()) else {
            return false;
        };
        // A zero-area rect passes the open-interval intersection below but
        // no real pointer can hit it — collapsed nodes must reject synthetic
        // clicks too (the reviewer final-gate finding).
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

    /// Whether `id` is `self` or a descendant of the topmost active
    /// scope's root. `true` when no scope is active — focus/click are
    /// document-wide until a modal/popup opens.
    /// Shared by AT-action validation AND IME dispatch (`route_ime`'s
    /// owner/eligibility gate) — the single scope-containment check, per
    /// the convergence both W5 slices flagged.
    pub(crate) fn is_within_active_scope(&self, cx: &Cx, id: NodeId) -> bool {
        let Some(top) = self.scope_stack.last() else {
            return true;
        };
        is_self_or_ancestor(cx, top.scope_root, id)
    }
}

/// Whether `id` is `ancestor` itself or a descendant of it, walking up
/// via `Cx::parent` (bounded by tree depth, no subtree allocation).
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

/// Walk `Cx::parent` up from `id` to the topmost ancestor (the document
/// root). `id` itself if it has no parent.
fn document_root(cx: &Cx, id: NodeId) -> NodeId {
    let mut current = id;
    while let Some(parent) = cx.parent(current) {
        current = parent;
    }
    current
}

/// Whether two axis-aligned rects overlap at all (touching edges do not
/// count as overlap).
const fn rects_intersect(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.width
        && a.origin.x + a.width > b.origin.x
        && a.origin.y < b.origin.y + b.height
        && a.origin.y + a.height > b.origin.y
}

#[cfg(test)]
#[path = "targets_tests.rs"]
mod tests;
