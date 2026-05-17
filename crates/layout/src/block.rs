//! CSS block layout — owned engine. Lands the P0.2 (W2.3) body.
//!
//! Algorithm conformance (per
//! [DR-041](../../../docs/decisions/DR-041-owned-layout-engine.md)):
//! single-axis stacking, adjacent + parent-first-child margin collapse,
//! padding + border, width/height with `Dimension::{Px, Percent, Auto}`,
//! min/max clamping, `Position::Absolute` pinning isolated from parent
//! flow, `box-sizing: content-box | border-box`.
//!
//! Out of scope: floats, clearance, collapse-through, writing-modes,
//! grid, multi-column.
//!
//! The solver does not mutate the [`LayoutForest`] — `MeasureScope`
//! borrows it immutably so multiple solvers can run concurrently in
//! P0.4. Computed boxes land in an internal `RefCell` map; the engine
//! drains them into the forest's `boxes` slab in P0.5 (W2.6) when the
//! taffy backend is removed. Tests read the map directly through
//! [`BlockSolver::boxed`].

#![allow(dead_code, clippy::redundant_pub_crate)]

use std::cell::RefCell;
use std::collections::HashMap;

use crate::LayoutError;
use crate::block_collapse::OpenMargin;
use crate::block_size::{AxisIntent, NodeEdges, SizeIntent};
use crate::forest::{ComputedBox, LocalId, MeasureScope, NodeRef};
use crate::style_data::{Display, Position, Size, StyleData};

/// CSS block layout subset.
///
/// Owns margin collapse (adjacent sibling + parent-first-child),
/// padding, border, width/height resolution with `auto`, min/max
/// clamping, absolute-position pinning, and content-box vs border-box
/// sizing.
pub(crate) struct BlockSolver<'a> {
    scope: MeasureScope<'a>,
    results: RefCell<HashMap<LocalId, ComputedBox>>,
}

impl<'a> BlockSolver<'a> {
    /// Wrap a measure scope for a block-layout pass.
    pub(crate) fn new(scope: MeasureScope<'a>) -> Self {
        Self {
            scope,
            results: RefCell::new(HashMap::new()),
        }
    }

    /// Run the block layout algorithm rooted at `subtree`.
    ///
    /// The pass writes one [`ComputedBox`] per visited in-flow node
    /// (and one per absolute descendant) into the internal results
    /// map. Callers query with [`Self::boxed`] or drain via
    /// [`Self::take`]. The pass is single-threaded and recurses
    /// top-down; sizes resolve against the parent's content box.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::TaffyInvalid`] (renamed in P0.5) when
    /// the subtree references a stale `NodeId`. Stale forest ids are
    /// the only failure mode at this layer.
    pub(crate) fn solve(&self, subtree: NodeRef<'_>) -> Result<(), LayoutError> {
        let avail = self.scope.available();
        let cb = root_containing_block(avail);
        let style = subtree.style();
        let edges = NodeEdges::build(style, cb.width);
        // Root border-box origin = root's own margin offset against
        // the viewport's (0,0) origin.
        let border_origin = Position2::new(edges.margin.left.unwrap_or(0.0), edges.margin_top());
        let me = solve_box(self, subtree, cb, border_origin)?;
        let _ = me;
        Ok(())
    }

    /// Borrow the computed box for `id`. Returns `None` if the solver
    /// never visited the id during this pass.
    pub(crate) fn boxed(&self, id: LocalId) -> Option<ComputedBox> {
        self.results.borrow().get(&id).copied()
    }

    /// Drain the results map. Used by the engine driver (P0.5) to
    /// scatter results into the forest's `boxes` slab.
    pub(crate) fn take(&self) -> HashMap<LocalId, ComputedBox> {
        std::mem::take(&mut self.results.borrow_mut())
    }

    /// Record a computed box keyed by node id.
    fn write(&self, id: LocalId, cb: ComputedBox) {
        self.results.borrow_mut().insert(id, cb);
    }
}

/// Pinned origin handed to a child's solve pass — the top-left corner
/// of the child's *containing block* (parent's content box) in the
/// root coordinate space.
#[derive(Clone, Copy, Debug)]
struct Position2 {
    x: f32,
    y: f32,
}

impl Position2 {
    const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Resolved geometry of a single layout pass on one node.
#[derive(Clone, Copy, Debug)]
struct Resolved {
    box_w: f32,
    box_h: f32,
    edges: NodeEdges,
}

/// Root-level containing block — derived from the measure scope's
/// available-space pair. `MinContent` / `MaxContent` collapse to zero
/// at this layer (block roots size against a definite outer constraint
/// in the engine driver; the unbounded paths are flex-only).
const fn root_containing_block(avail: crate::forest::AvailableSpace) -> Size<f32> {
    Size {
        width: definite(avail.w),
        height: definite(avail.h),
    }
}

const fn definite(c: crate::forest::SpaceConstraint) -> f32 {
    match c {
        crate::forest::SpaceConstraint::Definite(v) => v,
        crate::forest::SpaceConstraint::MinContent | crate::forest::SpaceConstraint::MaxContent => {
            0.0
        }
    }
}

/// Recursive single-node block-layout step.
///
/// `cb` is the containing-block size on each axis in pixels.
/// `border_origin` is the node's own *border-box* top-left in root
/// coordinates — the caller is responsible for having advanced past
/// any in-flow margin (margin collapse happens at the parent layer,
/// not here). The function writes a [`ComputedBox`] into the solver's
/// results map and returns the resolved geometry so the caller can
/// stack siblings.
fn solve_box(
    solver: &BlockSolver<'_>,
    node: NodeRef<'_>,
    cb: Size<f32>,
    border_origin: Position2,
) -> Result<Resolved, LayoutError> {
    let style = node.style();
    let edges = NodeEdges::build(style, cb.width);
    let intent = SizeIntent::build(style, cb, &edges);
    let box_w = intent.w.clamp(width_for(style, intent.w, cb.width, &edges));
    let inner_w = (box_w - edges.inline_chrome()).max(0.0);
    let inner_cb = Size {
        width: inner_w,
        height: intent.h.preferred.unwrap_or(cb.height),
    };
    let content_origin = Position2::new(
        border_origin.x + edges.border.left + edges.padding.left,
        border_origin.y + edges.border.top + edges.padding.top,
    );
    let content_height = layout_children(solver, node, inner_cb, content_origin)?;
    let auto_h = content_height + edges.block_chrome();
    let box_h = intent
        .h
        .clamp(intent.h.preferred.unwrap_or(auto_h).max(0.0));
    solver.write(
        node.id(),
        ComputedBox {
            x: border_origin.x,
            y: border_origin.y,
            w: box_w,
            h: box_h,
        },
    );
    Ok(Resolved {
        box_w,
        box_h,
        edges,
    })
}

/// Inline-axis sizing for one node. `Auto` widths fill the containing
/// block on this axis (the CSS rule for in-flow block elements).
fn width_for(style: &StyleData, intent: AxisIntent, cb_w: f32, edges: &NodeEdges) -> f32 {
    if let Some(preferred) = intent.preferred {
        return preferred;
    }
    // Auto width — fill containing block, less inline margin.
    let lateral = edges.margin.left.unwrap_or(0.0) + edges.margin.right.unwrap_or(0.0);
    let fill = cb_w - lateral;
    // box-sizing has already folded chrome into intent.preferred via
    // resolve; auto path here is content-box style: width *is* the
    // border-box.
    let _ = (style.box_sizing, edges);
    fill.max(0.0)
}

/// Walk the in-flow children, stack vertically with margin collapse,
/// and recurse absolute children against the parent's padding box.
/// Returns the content-area height consumed by in-flow children
/// (excludes absolute descendants — they don't contribute to flow).
fn layout_children(
    solver: &BlockSolver<'_>,
    parent: NodeRef<'_>,
    inner_cb: Size<f32>,
    content_origin: Position2,
) -> Result<f32, LayoutError> {
    let forest = solver.scope.forest();
    let mut cursor_y = 0.0_f32;
    let mut open = OpenMargin::sealed();
    for child_id in parent.children() {
        let child = forest
            .refer(*child_id)
            .ok_or_else(|| LayoutError::TaffyInvalid(format!("stale child id {child_id:?}")))?;
        let style = child.style();
        if matches!(style.display, Display::None) {
            continue;
        }
        if matches!(style.position, Position::Absolute) {
            place_absolute(solver, child, inner_cb, content_origin)?;
            continue;
        }
        let edges_preview = NodeEdges::build(style, inner_cb.width);
        let gap = open.absorb_top(edges_preview.margin_top());
        cursor_y += gap;
        let child_border_origin = Position2::new(
            content_origin.x + edges_preview.margin.left.unwrap_or(0.0),
            content_origin.y + cursor_y,
        );
        let resolved = solve_box(solver, child, inner_cb, child_border_origin)?;
        cursor_y += resolved.box_h;
        open.emit_bottom(resolved.edges.margin_bottom());
    }
    Ok(cursor_y)
}

/// Place a `Position::Absolute` child against the parent's padding box.
/// Absolute children are isolated from in-flow stacking: they do not
/// advance the flow cursor or participate in margin collapse, and
/// they size against the parent's inner content box.
fn place_absolute(
    solver: &BlockSolver<'_>,
    child: NodeRef<'_>,
    inner_cb: Size<f32>,
    content_origin: Position2,
) -> Result<(), LayoutError> {
    let style = child.style();
    let edges = NodeEdges::build(style, inner_cb.width);
    let intent = SizeIntent::build(style, inner_cb, &edges);
    let auto_w =
        (inner_cb.width - edges.margin.left.unwrap_or(0.0) - edges.margin.right.unwrap_or(0.0))
            .max(0.0);
    let box_w = intent.w.clamp(intent.w.preferred.unwrap_or(auto_w));
    let inner_w = (box_w - edges.inline_chrome()).max(0.0);
    let inner_cb_child = Size {
        width: inner_w,
        height: intent.h.preferred.unwrap_or(inner_cb.height),
    };
    let content_origin_child = Position2::new(
        content_origin.x
            + edges.margin.left.unwrap_or(0.0)
            + edges.border.left
            + edges.padding.left,
        content_origin.y + edges.margin_top() + edges.border.top + edges.padding.top,
    );
    let content_h = layout_children(solver, child, inner_cb_child, content_origin_child)?;
    let auto_h = content_h + edges.block_chrome();
    let box_h = intent.h.clamp(intent.h.preferred.unwrap_or(auto_h));
    solver.write(
        child.id(),
        ComputedBox {
            x: content_origin.x + edges.margin.left.unwrap_or(0.0),
            y: content_origin.y + edges.margin_top(),
            w: box_w,
            h: box_h,
        },
    );
    Ok(())
}

#[cfg(test)]
#[path = "block_tests.rs"]
mod tests;
