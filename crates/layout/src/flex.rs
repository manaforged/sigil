#![allow(clippy::redundant_pub_crate)]

use std::cell::{Cell, RefCell};

use ahash::AHashMap;

use crate::LayoutError;
use crate::flex_algorithm::{Axis2, FlexCtx, layout_root};
use crate::forest::{ComputedBox, LocalId, MeasureScope, NodeRef};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct IntrinsicKey {
    id: LocalId,
    main: u32,
    cross: u32,
    along_main: bool,
}

pub(crate) struct FlexSolver<'a> {
    scope: MeasureScope<'a>,
    intrinsic: RefCell<AHashMap<IntrinsicKey, f32>>,
    intrinsic_stores: Cell<u32>,
}

impl<'a> FlexSolver<'a> {
    pub(crate) fn new(scope: MeasureScope<'a>) -> Self {
        Self {
            scope,
            intrinsic: RefCell::new(AHashMap::new()),
            intrinsic_stores: Cell::new(0),
        }
    }

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
            assigned_w: None,
            assigned_h: None,
        };
        layout_root(&ctx, subtree, Position2::origin())
    }

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
            assigned_w: None,
            assigned_h: None,
        };
        layout_root(&ctx, subtree, origin)
    }

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

    pub(crate) fn write(&self, id: LocalId, cb: ComputedBox) {
        self.scope.write(id, cb);
    }

    pub(crate) const fn scope(&self) -> &MeasureScope<'_> {
        &self.scope
    }

    pub(crate) fn intrinsic_get(
        &self,
        id: LocalId,
        cb_main: f32,
        cb_cross: f32,
        along_main: bool,
    ) -> Option<f32> {
        let key = IntrinsicKey {
            id,
            main: cb_main.to_bits(),
            cross: cb_cross.to_bits(),
            along_main,
        };
        self.intrinsic.borrow().get(&key).copied()
    }

    #[allow(clippy::too_many_arguments, reason = "memo key is five fields")]
    pub(crate) fn intrinsic_put(
        &self,
        id: LocalId,
        cb_main: f32,
        cb_cross: f32,
        along_main: bool,
        extent: f32,
    ) {
        let key = IntrinsicKey {
            id,
            main: cb_main.to_bits(),
            cross: cb_cross.to_bits(),
            along_main,
        };
        self.intrinsic.borrow_mut().insert(key, extent);
        self.intrinsic_stores
            .set(self.intrinsic_stores.get().saturating_add(1));
    }

    #[cfg(test)]
    pub(crate) const fn intrinsic_stores(&self) -> u32 {
        self.intrinsic_stores.get()
    }
}

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

#[derive(Clone, Copy, Debug)]
pub(crate) struct CbSize {
    pub(crate) w: f32,
    pub(crate) h: f32,
}

impl CbSize {
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
