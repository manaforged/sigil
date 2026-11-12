#![allow(clippy::redundant_pub_crate)]

use crate::kernel::NodeId;
use smallvec::SmallVec;

use crate::Error;
use crate::layout::block::BlockSolver;
use crate::layout::flex::FlexSolver;
use crate::layout::forest::{
    AvailableSpace, ComputedBox, LayoutForest, LayoutText, LocalId, MeasureScope, NodeRef,
};
use crate::layout::measure::TextMeasure;
use crate::layout::style_data::{Axes, Display, Overflow, StyleData};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResolvedBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeHandle(LocalId);

pub struct LayoutDrive {
    forest: LayoutForest,
    text: Box<dyn TextMeasure>,
}

impl core::fmt::Debug for LayoutDrive {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LayoutDrive")
            .field("forest", &self.forest)
            .finish_non_exhaustive()
    }
}

impl LayoutDrive {
    #[must_use]
    pub fn new(text: Box<dyn TextMeasure>) -> Self {
        Self {
            forest: LayoutForest::new(),
            text,
        }
    }

    #[cfg(feature = "kittest")]
    pub fn insert(&mut self, style: StyleData) -> NodeHandle {
        NodeHandle(self.forest.insert(None, style))
    }

    pub(crate) fn insert_with_kernel(&mut self, kernel: NodeId, style: StyleData) -> NodeHandle {
        NodeHandle(self.forest.insert(Some(kernel), style))
    }

    pub(crate) fn reserve(&mut self, total: usize) {
        self.forest.reserve(total);
    }

    pub(crate) fn kernel_for(&self, handle: NodeHandle) -> Option<NodeId> {
        self.forest.refer(handle.0)?.kernel()
    }

    pub fn set_style(&mut self, node: NodeHandle, style: StyleData) -> Result<(), Error> {
        self.forest.set_style(node.0, style)
    }

    pub(crate) fn style_for(&self, node: NodeHandle) -> Option<&StyleData> {
        self.forest.style_for(node.0)
    }

    pub(crate) fn set_text(
        &mut self,
        node: NodeHandle,
        text: Option<LayoutText>,
    ) -> Result<(), Error> {
        self.forest.set_text(node.0, text)
    }

    pub(crate) fn text_for(&self, node: NodeHandle) -> Option<&LayoutText> {
        self.forest.text_for(node.0)
    }

    #[cfg(feature = "kittest")]
    pub fn attach(&mut self, parent: NodeHandle, child: NodeHandle) -> Result<(), Error> {
        self.forest.attach(parent.0, child.0)
    }

    pub(crate) fn set_children(
        &mut self,
        parent: NodeHandle,
        children: &[NodeHandle],
    ) -> Result<(), Error> {
        let local: SmallVec<[_; 8]> = children.iter().map(|child| child.0).collect();
        self.forest.set_children(parent.0, &local)
    }

    pub(crate) fn subtree_handles(&self, root: NodeHandle) -> Result<Vec<NodeHandle>, Error> {
        let mut out = Vec::new();
        self.collect_subtree(root, &mut out)?;
        Ok(out)
    }

    pub fn remove(&mut self, root: NodeHandle) -> Result<(), Error> {
        let subtree = self.subtree_handles(root)?;
        for handle in subtree.into_iter().rev() {
            self.forest.free(handle.0)?;
        }
        Ok(())
    }

    #[cfg(feature = "kittest")]
    #[must_use]
    pub fn count(&self) -> usize {
        self.forest.count()
    }

    pub(crate) fn invalidate(&self, node: NodeHandle) {
        self.forest.invalidate(node.0);
    }

    pub(crate) fn clear_intrinsic(&self) {
        self.forest.clear_intrinsic();
    }

    pub(crate) fn font_generation(&self) -> u64 {
        self.text.font_generation()
    }

    pub fn begin_frame(&self) {
        self.text.begin_frame();
    }

    #[cfg(feature = "kittest")]
    pub fn measure(&mut self, root: NodeHandle, viewport: Axes<f32>) -> Result<(), Error> {
        self.measure_with_constraints(
            root,
            AvailableSpace::definite(viewport.width, viewport.height),
        )
    }

    pub(crate) fn measure_with_constraints(
        &mut self,
        root: NodeHandle,
        avail: AvailableSpace,
    ) -> Result<(), Error> {
        self.begin_measure_frame();
        self.measure_keep_frame(root, avail)
    }

    pub(crate) fn begin_measure_frame(&mut self) {
        let _ = self.forest.next_frame();
    }

    pub(crate) fn measure_assigned(
        &self,
        root: NodeHandle,
        avail: AvailableSpace,
        width: f32,
        height: f32,
    ) -> Result<(), Error> {
        let handle = self.measure_root(root)?;
        let scope = self.pinned_scope(avail);
        let origin = crate::layout::block::Position2 { x: 0.0, y: 0.0 };
        let solved = match handle.style().display {
            Display::Flex => FlexSolver::new(scope).solve_at_area(
                handle,
                crate::layout::flex::CbSize {
                    w: width,
                    h: height,
                },
                crate::layout::flex::Position2::new(0.0, 0.0),
            ),
            Display::Grid => {
                crate::layout::grid::GridSolver::new(scope).solve_at(handle, width, origin)
            }
            Display::Block | Display::None => BlockSolver::new(scope).solve_at_assigned(
                handle,
                Axes { width, height },
                origin,
                Axes {
                    width: Some(width),
                    height: None,
                },
            ),
        };
        self.forest.pin_fonts(None);
        solved
    }

    pub(crate) fn measure_keep_frame(
        &self,
        root: NodeHandle,
        avail: AvailableSpace,
    ) -> Result<(), Error> {
        let handle = self.measure_root(root)?;
        let scope = self.pinned_scope(avail);
        let solved = match handle.style().display {
            Display::None => {
                scope.write(root.0, ComputedBox::default());
                Ok(())
            }
            Display::Flex => FlexSolver::new(scope).solve(handle),
            Display::Grid => crate::layout::grid::GridSolver::new(scope).solve(handle),
            Display::Block => BlockSolver::new(scope).solve(handle),
        };
        self.forest.pin_fonts(None);
        solved
    }

    fn pinned_scope(&self, avail: AvailableSpace) -> MeasureScope<'_> {
        self.forest.pin_fonts(Some(self.text.font_generation()));
        MeasureScope::new(&self.forest, self.text.as_ref(), avail)
    }

    fn measure_root(&self, root: NodeHandle) -> Result<NodeRef<'_>, Error> {
        self.forest
            .refer(root.0)
            .ok_or_else(|| Error::layout(format!("stale measure root {:?}", root.0)))
    }

    #[must_use]
    pub(crate) fn stored_box(&self, node: NodeHandle) -> Option<ResolvedBox> {
        self.forest.boxed(node.0).map(resolved)
    }

    #[must_use]
    pub(crate) fn frame_for(&self, node: NodeHandle) -> Option<u32> {
        self.forest.frame_for(node.0)
    }

    pub(crate) fn restore_box(
        &mut self,
        node: NodeHandle,
        cb: ResolvedBox,
        frame: u32,
    ) -> Result<(), Error> {
        self.forest.write_at_frame(node.0, computed(cb), frame)
    }

    #[must_use]
    pub fn computed_box(&self, node: NodeHandle) -> Option<ResolvedBox> {
        if self.forest.frame_for(node.0)? != self.forest.current_frame() {
            return None;
        }
        self.stored_box(node)
    }

    #[must_use]
    pub(crate) fn clips_content(&self, node: NodeHandle) -> bool {
        self.forest.style_for(node.0).is_some_and(|style| {
            style.overflow.width != Overflow::Visible || style.overflow.height != Overflow::Visible
        })
    }

    pub(crate) fn translate_subtree(
        &mut self,
        root: NodeHandle,
        dx: f32,
        dy: f32,
    ) -> Result<(), Error> {
        if dx == 0.0 && dy == 0.0 {
            return Ok(());
        }
        let handles = self.subtree_handles(root)?;
        let root_frame = self.forest.frame_for(root.0).unwrap_or(0);
        self.forest.invalidate_solved_ancestors(root.0);
        for handle in handles {
            self.forest
                .shift_solved_from(handle.0, root_frame, (dx, dy));
            if self.forest.frame_for(handle.0).unwrap_or(0) < root_frame {
                continue;
            }
            if let Some(cb) = self.forest.boxed(handle.0) {
                self.forest.write(
                    handle.0,
                    ComputedBox {
                        x: cb.x + dx,
                        y: cb.y + dy,
                        ..cb
                    },
                )?;
            }
        }
        Ok(())
    }

    fn collect_subtree(&self, root: NodeHandle, out: &mut Vec<NodeHandle>) -> Result<(), Error> {
        let Some(node) = self.forest.refer(root.0) else {
            return Ok(());
        };
        out.push(root);
        for child in node.children() {
            self.collect_subtree(NodeHandle(*child), out)?;
        }
        Ok(())
    }
}

const fn resolved(cb: ComputedBox) -> ResolvedBox {
    ResolvedBox {
        x: cb.x,
        y: cb.y,
        w: cb.w,
        h: cb.h,
    }
}

const fn computed(cb: ResolvedBox) -> ComputedBox {
    ComputedBox {
        x: cb.x,
        y: cb.y,
        w: cb.w,
        h: cb.h,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "engine_drive_tests.rs"]
mod tests;
