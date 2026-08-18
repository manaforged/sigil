//! CSS flex layout — owned engine. Lands the P0.3 (W2.4) body.
//!
//! Algorithm conformance (per
//! [DR-041](../../../docs/decisions/DR-041-owned-layout-engine.md)):
//! main / cross axes via [`FlexDirection`], `flex-basis` with grow +
//! shrink resolution, multi-line wrap, `gap`, `padding`, `margin`,
//! `border`, `min-size` / `max-size` clamps, `justify-content`,
//! `align-items`, `align-self`, `align-content`, `order`. Reverse
//! directions handled.
//!
//! Out of scope: subgrid, container queries, intrinsic min-content
//! resolution beyond the simple `basis = 0` case, baseline alignment
//! (falls back to `start`).
//!
//! The solver does not mutate the [`LayoutForest`] — `MeasureScope`
//! borrows it immutably so multiple solvers can run concurrently in
//! P0.4. Computed boxes scatter-write directly into the forest's
//! `boxes` slab via [`MeasureScope::write`], with the forest's
//! `current_frame` tag stamped into each slot so callers can
//! distinguish "reached this measure" from "stale slab entry." Tests
//! read results back through [`FlexSolver::boxed`], which delegates to
//! `forest.boxed`.

#![allow(clippy::redundant_pub_crate)]

use crate::LayoutError;
use crate::flex_algorithm::{Axis2, FlexCtx, layout_root};
use crate::forest::{ComputedBox, LocalId, MeasureScope, NodeRef};

/// CSS flexbox layout subset.
///
/// Owns main/cross-axis sizing, flex-basis resolution, grow/shrink
/// distribution, wrapping, gap, justify-content / align-items /
/// align-self / align-content. Reverse directions handled.
pub(crate) struct FlexSolver<'a> {
    scope: MeasureScope<'a>,
}

impl<'a> FlexSolver<'a> {
    /// Wrap a measure scope for a flex-layout pass.
    pub(crate) const fn new(scope: MeasureScope<'a>) -> Self {
        Self { scope }
    }

    /// Run the flex layout algorithm rooted at `subtree`.
    ///
    /// Visits the root as a flex container; its children become flex
    /// items. Writes one [`ComputedBox`] per visited node directly into
    /// the forest's `boxes` slab through [`MeasureScope::write`].
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::LayoutInvalid`] when the subtree
    /// references a stale `NodeId`.
    pub(crate) fn solve(&self, subtree: NodeRef<'_>) -> Result<(), LayoutError> {
        let avail = self.scope.available();
        let cb = root_cb(avail);
        let style = subtree.style();
        let axes = Axis2::for_direction(style.flex_direction);
        let ctx = FlexCtx {
            solver: self,
            axes,
            cb,
            suppress_stretch: false,
            // Layout root: no parent assignment, shrink-to-fit applies.
            assigned_w: None,
            assigned_h: None,
        };
        layout_root(&ctx, subtree, Position2::origin())
    }

    /// Run the flex layout algorithm against `subtree` as a child of
    /// some containing block of size `cb`, anchored at `origin` (the
    /// subtree root's border-box top-left BEFORE this node's own margin
    /// is applied). Used by per-subtree solver dispatch when a flex-
    /// display node lands inside a block parent.
    ///
    /// Mirrors [`Self::solve`]'s shape but takes an explicit containing
    /// block + origin instead of deriving them from `scope.available()`
    /// at the viewport origin.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::LayoutInvalid`] when `subtree` references a
    /// stale id.
    pub(crate) fn solve_at(
        &self,
        subtree: NodeRef<'_>,
        cb: CbSize,
        origin: Position2,
    ) -> Result<(), LayoutError> {
        let style = subtree.style();
        let axes = Axis2::for_direction(style.flex_direction);
        let ctx = FlexCtx {
            solver: self,
            axes,
            cb,
            suppress_stretch: false,
            // Layout root: no parent assignment, shrink-to-fit applies.
            assigned_w: None,
            assigned_h: None,
        };
        layout_root(&ctx, subtree, origin)
    }

    /// Lay out `subtree` into a definite area. Grid stretch uses this
    /// so a flex item fills its track instead of hugging content.
    pub(crate) fn solve_at_area(
        &self,
        subtree: NodeRef<'_>,
        cb: CbSize,
        origin: Position2,
    ) -> Result<(), LayoutError> {
        let style = subtree.style();
        let axes = Axis2::for_direction(style.flex_direction);
        let ctx = FlexCtx {
            solver: self,
            axes,
            cb,
            suppress_stretch: false,
            assigned_w: Some(cb.w),
            assigned_h: (cb.h > 0.0).then_some(cb.h),
        };
        layout_root(&ctx, subtree, origin)
    }

    /// Borrow the computed box for `id`. Returns `None` when the id is
    /// stale or the solver never visited it during this pass (frame
    /// tag does not match the forest's current frame).
    #[cfg(test)]
    pub(crate) fn boxed(&self, id: LocalId) -> Option<ComputedBox> {
        let forest = self.scope.forest();
        if forest.frame_for(id)? != forest.current_frame() {
            return None;
        }
        forest.boxed(id)
    }

    /// Record a computed box keyed by node id. Writes scatter-directly
    /// into the forest's box slab via the shared `MeasureScope` borrow.
    pub(crate) fn write(&self, id: LocalId, cb: ComputedBox) {
        self.scope.write(id, cb);
    }

    /// Borrow the measure scope (used by helpers in sibling modules).
    pub(crate) const fn scope(&self) -> &MeasureScope<'_> {
        &self.scope
    }
}

/// Pinned origin (top-left in root coordinates).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Position2 {
    pub(crate) x: f32,
    pub(crate) y: f32,
}

impl Position2 {
    pub(crate) const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub(crate) const fn origin() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

/// Root containing block — derived from the measure scope's
/// available-space pair. Unbounded axes collapse to zero at this
/// layer; the engine driver in P0.5 supplies definite outer bounds.
const fn root_cb(avail: crate::forest::AvailableSpace) -> CbSize {
    CbSize {
        w: definite(avail.w),
        h: definite(avail.h),
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

/// Pixel containing-block size on both axes.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CbSize {
    pub(crate) w: f32,
    pub(crate) h: f32,
}

impl CbSize {
    /// Axis-major view: (main, cross) under the container's direction.
    pub(crate) const fn along(self, axes: Axis2) -> (f32, f32) {
        if axes.row {
            (self.w, self.h)
        } else {
            (self.h, self.w)
        }
    }
}

#[cfg(test)]
#[path = "flex_tests.rs"]
mod tests;
