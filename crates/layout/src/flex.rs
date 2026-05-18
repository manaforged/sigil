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
//! P0.4. Computed boxes land in an internal `RefCell` map; the engine
//! drains them into the forest's `boxes` slab. Tests read the map
//! directly through [`FlexSolver::boxed`].

#![allow(dead_code, clippy::redundant_pub_crate)]

use std::cell::RefCell;
use std::collections::HashMap;

use crate::LayoutError;
use crate::flex_algorithm::{Axis2, FlexCtx, layout_root};
use crate::forest::{ComputedBox, LocalId, MeasureScope, NodeRef};
use crate::style_data::FlexDirection;

/// CSS flexbox layout subset.
///
/// Owns main/cross-axis sizing, flex-basis resolution, grow/shrink
/// distribution, wrapping, gap, justify-content / align-items /
/// align-self / align-content. Reverse directions handled.
pub(crate) struct FlexSolver<'a> {
    scope: MeasureScope<'a>,
    results: RefCell<HashMap<LocalId, ComputedBox>>,
}

impl<'a> FlexSolver<'a> {
    /// Wrap a measure scope for a flex-layout pass.
    pub(crate) fn new(scope: MeasureScope<'a>) -> Self {
        Self {
            scope,
            results: RefCell::new(HashMap::new()),
        }
    }

    /// Run the flex layout algorithm rooted at `subtree`.
    ///
    /// Visits the root as a flex container; its children become flex
    /// items. Writes one [`ComputedBox`] per visited node into the
    /// internal results map. Callers query with [`Self::boxed`] or
    /// drain via [`Self::take`].
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
        };
        layout_root(&ctx, subtree, Position2::origin())
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
    pub(crate) fn write(&self, id: LocalId, cb: ComputedBox) {
        self.results.borrow_mut().insert(id, cb);
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

/// Lookup helper: convert direction to a row/column flag pair plus
/// the reversed flag.
#[derive(Clone, Copy, Debug)]
pub(crate) struct DirFlags {
    pub(crate) row: bool,
    pub(crate) reversed: bool,
}

impl DirFlags {
    pub(crate) const fn from(direction: FlexDirection) -> Self {
        match direction {
            FlexDirection::Row => Self {
                row: true,
                reversed: false,
            },
            FlexDirection::RowReverse => Self {
                row: true,
                reversed: true,
            },
            FlexDirection::Column => Self {
                row: false,
                reversed: false,
            },
            FlexDirection::ColumnReverse => Self {
                row: false,
                reversed: true,
            },
        }
    }
}

#[cfg(test)]
#[path = "flex_tests.rs"]
mod tests;
