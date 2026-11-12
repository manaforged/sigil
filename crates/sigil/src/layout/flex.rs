#![allow(clippy::redundant_pub_crate)]

use std::cell::Cell;

use crate::Error;
use crate::layout::flex_algorithm::{Axis2, FlexCtx, layout_root};
use crate::layout::forest::{
    ComputedBox, IntrinsicKey, LocalId, MeasureScope, NodeRef, SpaceConstraint,
};
use crate::layout::style_data::{Display, FlexDirection, FlexWrap, Length, Position};

pub(crate) struct FlexSolver<'a> {
    scope: MeasureScope<'a>,
    intrinsic_stores: Cell<u32>,
}

impl<'a> FlexSolver<'a> {
    pub(crate) const fn new(scope: MeasureScope<'a>) -> Self {
        Self {
            scope,
            intrinsic_stores: Cell::new(0),
        }
    }

    pub(crate) fn fills_width(&self, node: NodeRef<'_>) -> bool {
        let style = node.style();
        if style.size.width != Length::Auto
            || matches!(style.display, Display::Grid | Display::None)
        {
            return false;
        }
        let forest = self.scope.forest();
        if let Some(known) = forest.fills_get(node.id()) {
            return known;
        }
        let row = matches!(style.display, Display::Flex)
            && matches!(
                style.flex_direction,
                FlexDirection::Row | FlexDirection::RowReverse
            );
        let fills = node
            .children()
            .iter()
            .filter_map(|id| forest.refer(*id))
            .any(|child| {
                let child_style = child.style();
                child_style.display != Display::None
                    && child_style.position != Position::Absolute
                    && (child_style.size.width == Length::Fill
                        || (row && child_style.flex_grow > 0.0)
                        || self.fills_width(child))
            });
        forest.fills_put(node.id(), fills);
        fills
    }

    pub(crate) fn height_free(&self, node: NodeRef<'_>) -> bool {
        let forest = self.scope.forest();
        if let Some(known) = forest.height_free_get(node.id()) {
            return known;
        }
        let style = node.style();
        let tall = |dim: Length| matches!(dim, Length::Percent(_) | Length::Fill);
        let free = !matches!(style.display, Display::Grid)
            && !matches!(style.position, Position::Absolute)
            && matches!(style.flex_wrap, FlexWrap::NoWrap)
            && !tall(style.size.height)
            && !tall(style.min_size.height)
            && !tall(style.max_size.height)
            && !tall(style.flex_basis)
            && !tall(style.gap.height)
            && node
                .children()
                .iter()
                .filter_map(|id| forest.refer(*id))
                .all(|child| self.height_free(child));
        forest.height_free_put(node.id(), free);
        free
    }

    pub(crate) fn solve(&self, subtree: NodeRef<'_>) -> Result<(), Error> {
        let avail = self.scope.available();
        let cb = CbSize {
            w: definite(avail.w),
            h: definite(avail.h),
        };
        let style = subtree.style();
        let stretched = style.stretch_width && style.size.width == Length::Auto;
        let assigned_w =
            (stretched && matches!(avail.w, SpaceConstraint::Definite(_))).then_some(cb.w);
        self.solve_with(subtree, cb, Position2::new(0.0, 0.0), (assigned_w, None))
    }

    pub(crate) fn solve_at(
        &self,
        subtree: NodeRef<'_>,
        cb: CbSize,
        origin: Position2,
    ) -> Result<(), Error> {
        self.solve_with(subtree, cb, origin, (None, None))
    }

    pub(crate) fn solve_at_area(
        &self,
        subtree: NodeRef<'_>,
        cb: CbSize,
        origin: Position2,
    ) -> Result<(), Error> {
        let assigned = (Some(cb.w), (cb.h > 0.0).then_some(cb.h));
        self.solve_with(subtree, cb, origin, assigned)
    }

    fn solve_with(
        &self,
        subtree: NodeRef<'_>,
        cb: CbSize,
        origin: Position2,
        (assigned_w, assigned_h): (Option<f32>, Option<f32>),
    ) -> Result<(), Error> {
        let ctx = FlexCtx {
            solver: self,
            axes: Axis2::for_direction(subtree.style().flex_direction),
            cb,
            suppress_stretch: false,
            measures_height: false,
            assigned_w,
            assigned_h,
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
        self.scope
            .forest()
            .intrinsic_get(id, self.intrinsic_key(cb_main, cb_cross, along_main))
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
        let key = self.intrinsic_key(cb_main, cb_cross, along_main);
        self.scope.forest().intrinsic_put(id, key, extent);
        crate::profile::count(crate::profile::Counter::LayoutIntrinsicMiss);
        self.intrinsic_stores
            .set(self.intrinsic_stores.get().saturating_add(1));
    }

    fn intrinsic_key(&self, cb_main: f32, cb_cross: f32, along_main: bool) -> IntrinsicKey {
        IntrinsicKey::new(
            (cb_main, cb_cross),
            along_main,
            self.scope.available(),
            self.scope.text().font_generation(),
        )
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
}

const fn definite(c: SpaceConstraint) -> f32 {
    match c {
        SpaceConstraint::Definite(v) => v,
        SpaceConstraint::MinContent | SpaceConstraint::MaxContent => 0.0,
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

    pub(crate) const fn across(axes: Axis2, main: f32, cross: f32) -> Self {
        if axes.row {
            Self { w: main, h: cross }
        } else {
            Self { w: cross, h: main }
        }
    }
}

#[cfg(test)]
#[path = "flex_tests.rs"]
mod tests;
