#![allow(clippy::redundant_pub_crate)]

use sigil_kernel::TextContent;
use sigil_style::TextStyle;
use smallvec::SmallVec;

use crate::block::BlockSolver;
use crate::error::LayoutError;
use crate::flex::FlexSolver;
use crate::forest::{
    AvailableSpace, ComputedBox, LayoutForest, LayoutText, LocalId, MeasureScope, text_memo_clear,
};
use crate::measure::TextMeasure;
use crate::style_data::{Display, Overflow, Size, StyleData};

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

impl LayoutDrive {
    #[must_use]
    pub fn new(text: Box<dyn TextMeasure>) -> Self {
        text_memo_clear();
        Self {
            forest: LayoutForest::new(),
            text,
        }
    }

    pub fn insert(&mut self, style: StyleData) -> NodeHandle {
        NodeHandle(self.forest.insert(None, style))
    }

    pub(crate) fn insert_with_kernel(
        &mut self,
        kernel: sigil_kernel::NodeId,
        style: StyleData,
    ) -> NodeHandle {
        NodeHandle(self.forest.insert(Some(kernel), style))
    }

    pub(crate) fn kernel_for(&self, handle: NodeHandle) -> Option<sigil_kernel::NodeId> {
        self.forest.refer(handle.0)?.kernel()
    }

    pub fn set_style(&mut self, node: NodeHandle, style: StyleData) -> Result<(), LayoutError> {
        self.forest.set_style(node.0, style)
    }

    pub(crate) fn style_for(&self, node: NodeHandle) -> Option<&StyleData> {
        self.forest.style_for(node.0)
    }

    pub(crate) fn set_text(
        &mut self,
        node: NodeHandle,
        content: Option<TextContent>,
        style: Option<TextStyle>,
        flags: crate::forest::TextFlags,
    ) -> Result<(), LayoutError> {
        let text = content.map(|content| LayoutText {
            content,
            style,
            wrap: flags.wrap,
            ellipsis: flags.ellipsis,
        });
        self.forest.set_text(node.0, text)
    }

    pub(crate) fn text_for(&self, node: NodeHandle) -> Option<&LayoutText> {
        self.forest.text_for(node.0)
    }

    pub fn attach(&mut self, parent: NodeHandle, child: NodeHandle) -> Result<(), LayoutError> {
        self.forest.attach(parent.0, child.0)
    }

    pub fn set_children(
        &mut self,
        parent: NodeHandle,
        children: &[NodeHandle],
    ) -> Result<(), LayoutError> {
        let local: SmallVec<[_; 8]> = children.iter().map(|child| child.0).collect();
        self.forest.set_children(parent.0, &local)
    }

    pub(crate) fn subtree_handles(&self, root: NodeHandle) -> Result<Vec<NodeHandle>, LayoutError> {
        let mut out = Vec::new();
        self.collect_subtree(root, &mut out)?;
        Ok(out)
    }

    pub fn remove(&mut self, root: NodeHandle) -> Result<(), LayoutError> {
        let subtree = self.subtree_handles(root)?;
        for handle in subtree.into_iter().rev() {
            self.forest.free(handle.0)?;
        }
        Ok(())
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.forest.count()
    }

    pub fn begin_frame(&self) {
        self.text.begin_frame();
    }

    pub fn measure(&mut self, root: NodeHandle, viewport: Size<f32>) -> Result<(), LayoutError> {
        let avail = AvailableSpace::definite(viewport.width, viewport.height);
        self.measure_with_constraints(root, avail)
    }

    pub(crate) fn measure_with_constraints(
        &mut self,
        root: NodeHandle,
        avail: AvailableSpace,
    ) -> Result<(), LayoutError> {
        self.begin_measure_frame();
        self.measure_keep_frame(root, avail)
    }

    pub(crate) const fn begin_measure_frame(&mut self) {
        let _ = self.forest.next_frame();
    }

    pub(crate) fn measure_keep_frame(
        &self,
        root: NodeHandle,
        avail: AvailableSpace,
    ) -> Result<(), LayoutError> {
        let root_id = root.0;
        let handle = self
            .forest
            .refer(root_id)
            .ok_or_else(|| LayoutError::LayoutInvalid(format!("stale measure root {root_id:?}")))?;
        let display = handle.style().display;
        if matches!(display, Display::None) {
            MeasureScope::new(&self.forest, self.text.as_ref(), avail).write(
                root_id,
                ComputedBox {
                    x: 0.0,
                    y: 0.0,
                    w: 0.0,
                    h: 0.0,
                },
            );
            return Ok(());
        }
        let scope = MeasureScope::new(&self.forest, self.text.as_ref(), avail);
        match display {
            Display::Flex => {
                let solver = FlexSolver::new(scope);
                solver.solve(handle)?;
            }
            Display::Grid => {
                let solver = crate::grid::GridSolver::new(scope);
                solver.solve(handle)?;
            }
            Display::Block | Display::None => {
                let solver = BlockSolver::new(scope);
                solver.solve(handle)?;
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn computed_box(&self, node: NodeHandle) -> Option<ResolvedBox> {
        if self.forest.frame_for(node.0)? != self.forest.current_frame() {
            return None;
        }
        let cb = self.forest.boxed(node.0)?;
        Some(ResolvedBox {
            x: cb.x,
            y: cb.y,
            w: cb.w,
            h: cb.h,
        })
    }

    #[must_use]
    pub fn clips_content(&self, node: NodeHandle) -> bool {
        self.forest.style_for(node.0).is_some_and(|style| {
            style.overflow.width != Overflow::Visible || style.overflow.height != Overflow::Visible
        })
    }

    pub(crate) fn translate_subtree(
        &mut self,
        root: NodeHandle,
        dx: f32,
        dy: f32,
    ) -> Result<(), LayoutError> {
        if dx == 0.0 && dy == 0.0 {
            return Ok(());
        }
        let mut handles = Vec::new();
        self.collect_subtree(root, &mut handles)?;
        for handle in handles {
            let Some(box_) = self.computed_box(handle) else {
                continue;
            };
            self.forest.write(
                handle.0,
                ComputedBox {
                    x: box_.x + dx,
                    y: box_.y + dy,
                    w: box_.w,
                    h: box_.h,
                },
            )?;
        }
        Ok(())
    }

    fn collect_subtree(
        &self,
        root: NodeHandle,
        out: &mut Vec<NodeHandle>,
    ) -> Result<(), LayoutError> {
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    reason = "test/instrumentation context; the invariant is proven by the caller"
)]
#[path = "engine_drive_tests.rs"]
mod tests;
