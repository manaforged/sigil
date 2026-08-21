//! P0.5.2 engine driver — end-to-end `measure` over a `LayoutForest`.
//!
//! Public surface for downstream crates to drive layout against an owned
//! forest without touching an upstream layout engine. The driver dispatches the root's
//! display to the matching solver (`BlockSolver` or `FlexSolver`),
//! drains the solver's per-node results into the forest's `boxes` slab,
//! and exposes per-node geometry through [`LayoutDrive::computed_box`].
//!
//! Per-node dispatch *across* mixed-display subtrees lives in a later
//! step (P0.5.3); the current driver routes only at the root and lets
//! the solver recurse the subtree under one display kind. This matches
//! the parallel-measure fan-out shape in [`crate::parallel`], which
//! dispatches one solver per boundary subtree.

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

/// Geometry resolved by [`LayoutDrive::measure`] for one node.
///
/// Fields are border-box coordinates in the viewport space passed to
/// `measure`. `Copy` because the type is small and value-passed
/// downstream.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResolvedBox {
    /// Border-box x in viewport coordinates.
    pub x: f32,
    /// Border-box y in viewport coordinates.
    pub y: f32,
    /// Border-box width.
    pub w: f32,
    /// Border-box height.
    pub h: f32,
}

/// Stable, public handle for a node inside a [`LayoutDrive`].
///
/// Wraps the crate-private `LocalId` so downstream crates can address
/// nodes without depending on the slab id type. Handles are `Copy` and
/// stable across measure calls; freeing a node is not yet exposed at
/// this layer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct NodeHandle(LocalId);

/// Driver owning a `LayoutForest` + text measurer.
///
/// `LayoutDrive` is the public entry point for the owned layout engine
/// in P0.5. Construct one, build the tree with [`Self::insert`] +
/// [`Self::attach`], call [`Self::measure`] to resolve geometry, then
/// read back per-node boxes with [`Self::computed_box`].
///
/// The text measurer is supplied at construction so the engine can hand
/// it to the solvers without storing one per-call. Tests and headless
/// callers pass a zero-measuring stub.
pub struct LayoutDrive {
    forest: LayoutForest,
    text: Box<dyn TextMeasure>,
}

impl LayoutDrive {
    /// Build a driver around a text measurer.
    #[must_use]
    pub fn new(text: Box<dyn TextMeasure>) -> Self {
        text_memo_clear();
        Self {
            forest: LayoutForest::new(),
            text,
        }
    }

    /// Insert a node with the given style and return its handle. The
    /// node has no parent until [`Self::attach`] places it.
    pub fn insert(&mut self, style: StyleData) -> NodeHandle {
        NodeHandle(self.forest.insert(None, style))
    }

    /// Insert a node carrying its originating `sigil_kernel::NodeId`
    /// link. `LayoutEngine` uses this path so the inverse `handle →
    /// kernel` lookup reads `LayoutNode.kernel` directly instead of a
    /// parallel hashmap.
    pub(crate) fn insert_with_kernel(
        &mut self,
        kernel: sigil_kernel::NodeId,
        style: StyleData,
    ) -> NodeHandle {
        NodeHandle(self.forest.insert(Some(kernel), style))
    }

    /// Resolve `handle` back to its kernel `NodeId`. Returns `None`
    /// when the handle is stale or the node was inserted without a
    /// kernel link (e.g. test fixtures using `LayoutDrive::insert`).
    pub(crate) fn kernel_for(&self, handle: NodeHandle) -> Option<sigil_kernel::NodeId> {
        self.forest.refer(handle.0)?.kernel()
    }

    /// Replace a node's style.
    pub fn set_style(&mut self, node: NodeHandle, style: StyleData) -> Result<(), LayoutError> {
        self.forest.set_style(node.0, style)
    }

    /// Borrow a node's currently-stored style. Returns `None` when the
    /// handle is stale. Used by `LayoutEngine` to skip the
    /// clone-and-write path when the kernel-side intent already
    /// matches the mirror.
    pub(crate) fn style_for(&self, node: NodeHandle) -> Option<&StyleData> {
        self.forest.style_for(node.0)
    }

    /// Replace the optional text measurement context for a node.
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

    /// Borrow a node's currently-stored text payload. Mirror of
    /// `style_for`. Returns `None` when the handle is stale.
    pub(crate) fn text_for(&self, node: NodeHandle) -> Option<&LayoutText> {
        self.forest.text_for(node.0)
    }

    /// Append `child` to `parent`'s child list.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::LayoutInvalid`] when either handle is
    /// stale.
    pub fn attach(&mut self, parent: NodeHandle, child: NodeHandle) -> Result<(), LayoutError> {
        self.forest.attach(parent.0, child.0)
    }

    /// Replace `parent`'s child list.
    pub fn set_children(
        &mut self,
        parent: NodeHandle,
        children: &[NodeHandle],
    ) -> Result<(), LayoutError> {
        let local: SmallVec<[_; 8]> = children.iter().map(|child| child.0).collect();
        self.forest.set_children(parent.0, &local)
    }

    /// Return the reachable subtree handles below `root`, including `root`.
    pub(crate) fn subtree_handles(&self, root: NodeHandle) -> Result<Vec<NodeHandle>, LayoutError> {
        let mut out = Vec::new();
        self.collect_subtree(root, &mut out)?;
        Ok(out)
    }

    /// Remove a node and all of its descendants from the owned forest.
    pub fn remove(&mut self, root: NodeHandle) -> Result<(), LayoutError> {
        let subtree = self.subtree_handles(root)?;
        for handle in subtree.into_iter().rev() {
            self.forest.free(handle.0)?;
        }
        Ok(())
    }

    /// Live node count.
    #[must_use]
    pub fn count(&self) -> usize {
        self.forest.count()
    }

    /// Notify the text measurer that a new frame is beginning.
    pub fn begin_frame(&self) {
        self.text.begin_frame();
    }

    /// Walk the tree from `root` and resolve every reachable node's
    /// box.
    ///
    /// Dispatches to `BlockSolver` or `FlexSolver` based on the
    /// root's `display`. The chosen solver recurses through the
    /// subtree and writes per-node boxes into an internal map; the
    /// driver then scatters those into the forest's `boxes` slab so
    /// [`Self::computed_box`] can read them.
    ///
    /// Idempotent: re-calling with the same root and same styles is
    /// observably a no-op — the solver re-derives the same boxes from
    /// the same style inputs and the scatter clobbers identical values.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::LayoutInvalid`] when `root` is stale or a
    /// descendant references a stale id mid-walk.
    pub fn measure(&mut self, root: NodeHandle, viewport: Size<f32>) -> Result<(), LayoutError> {
        let avail = AvailableSpace::definite(viewport.width, viewport.height);
        self.measure_with_constraints(root, avail)
    }

    /// Measure with full owned space constraints.
    ///
    /// `LayoutEngine` uses this path so `sigil_hal::Dim::{Fixed,
    /// MinContent, MaxContent}` survive the cutover. The older
    /// [`Self::measure`] convenience remains for tests and callers with
    /// definite viewport sizes.
    pub(crate) fn measure_with_constraints(
        &mut self,
        root: NodeHandle,
        avail: AvailableSpace,
    ) -> Result<(), LayoutError> {
        // Bump the frame tag before dispatching so every box the
        // solver scatter-writes through `MeasureScope::write` lands
        // stamped with the current frame. `computed_box` compares
        // against this tag to detect "reached this measure" vs
        // "stale slab entry" — replaces the previous per-frame
        // `AHashSet<LocalId>`.
        self.begin_measure_frame();
        self.measure_keep_frame(root, avail)
    }

    /// Bump the frame tag once. Callers measuring several roots that
    /// belong to the *same* logical layout pass (the incremental path's
    /// containment roots) must bump once and then use
    /// [`Self::measure_keep_frame`] per root — bumping per root would
    /// stamp each root's boxes with a different tag, so `computed_box`
    /// would treat every root but the last as stale.
    pub(crate) const fn begin_measure_frame(&mut self) {
        let _ = self.forest.next_frame();
    }

    /// Measure a root under the *current* frame tag without bumping it.
    /// See [`Self::begin_measure_frame`]. Takes `&self`: the forest's
    /// box slab is written through interior mutability by the solver.
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

    /// Read the computed box for `node` after the most recent
    /// [`Self::measure`] call. Returns `None` when the handle is stale
    /// or the node was not reached.
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

    /// Whether `node` clips its overflowing content (`overflow: hidden` or
    /// `scroll` on either axis). Drives the renderer's clip scope so a node
    /// like a scroll viewport or the editor pane scissors descendants that
    /// fall outside its box instead of letting them paint over siblings.
    #[must_use]
    pub fn clips_content(&self, node: NodeHandle) -> bool {
        self.forest.style_for(node.0).is_some_and(|style| {
            style.overflow.width != Overflow::Visible || style.overflow.height != Overflow::Visible
        })
    }

    /// Offset every computed box in `root`'s subtree by `(dx, dy)` logical px.
    ///
    /// The incremental remeasure path re-solves a containment root as an
    /// isolated layout root, which anchors it at `(0, 0)`. Shifting the whole
    /// subtree back to the root's true in-tree origin keeps the drive boxes —
    /// the single source of truth read by paint (via the attached
    /// `LayoutResult`), hit-testing, and pointer dispatch (both via
    /// `LayoutEngine::result`) — in absolute root coordinates and mutually
    /// consistent. A `(0, 0)` shift is a no-op.
    ///
    /// Reads through the frame-checked [`Self::computed_box`], not the raw
    /// slab: a node the solver skipped this measure (e.g. `display: none`)
    /// keeps a stale box + old frame tag and must stay unreached. Translating
    /// it via the raw box would re-stamp the current frame and resurrect it as
    /// a live (mispositioned) node.
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
#[allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]
#[path = "engine_drive_tests.rs"]
mod tests;
