//! P0.5.2 engine driver — end-to-end `measure` over a [`LayoutForest`].
//!
//! Public surface for downstream crates to drive layout against an owned
//! forest without touching `taffy`. The driver dispatches the root's
//! display to the matching solver ([`BlockSolver`] or [`FlexSolver`]),
//! drains the solver's per-node results into the forest's `boxes` slab,
//! and exposes per-node geometry through [`LayoutDrive::computed_box`].
//!
//! Per-node dispatch *across* mixed-display subtrees lives in a later
//! step (P0.5.3); the current driver routes only at the root and lets
//! the solver recurse the subtree under one display kind. This matches
//! the parallel-measure fan-out shape in [`crate::parallel`], which
//! dispatches one solver per boundary subtree.

#![allow(clippy::redundant_pub_crate)]

use crate::block::BlockSolver;
use crate::error::LayoutError;
use crate::flex::FlexSolver;
use crate::forest::{AvailableSpace, LayoutForest, LocalId, MeasureScope};
use crate::measure::TextMeasure;
use crate::style_data::{Display, Size, StyleData};

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

/// Driver owning a [`LayoutForest`] + text measurer.
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

    /// Append `child` to `parent`'s child list.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::TaffyInvalid`] when either handle is
    /// stale.
    pub fn attach(&mut self, parent: NodeHandle, child: NodeHandle) -> Result<(), LayoutError> {
        self.forest.attach(parent.0, child.0)
    }

    /// Live node count.
    #[must_use]
    pub fn count(&self) -> usize {
        self.forest.count()
    }

    /// Walk the tree from `root` and resolve every reachable node's
    /// box.
    ///
    /// Dispatches to [`BlockSolver`] or [`FlexSolver`] based on the
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
    /// Returns [`LayoutError::TaffyInvalid`] when `root` is stale or a
    /// descendant references a stale id mid-walk.
    pub fn measure(&mut self, root: NodeHandle, viewport: Size<f32>) -> Result<(), LayoutError> {
        let avail = AvailableSpace::definite(viewport.width, viewport.height);
        let root_id = root.0;
        let handle = self
            .forest
            .refer(root_id)
            .ok_or_else(|| LayoutError::TaffyInvalid(format!("stale measure root {root_id:?}")))?;
        let display = handle.style().display;
        let scope = MeasureScope::new(&self.forest, self.text.as_ref(), avail);
        let drained = match display {
            Display::Flex => {
                let solver = FlexSolver::new(scope);
                solver.solve(handle)?;
                solver.take()
            }
            Display::Block | Display::None => {
                let solver = BlockSolver::new(scope);
                solver.solve(handle)?;
                solver.take()
            }
        };
        for (id, cb) in drained {
            // Stale ids fall away silently inside `write` — the only
            // path producing one is concurrent forest mutation, which
            // does not happen under `&mut self`.
            let _ = self.forest.write(id, cb);
        }
        Ok(())
    }

    /// Read the computed box for `node` after the most recent
    /// [`Self::measure`] call. Returns `None` when the handle is stale
    /// or the node was not reached.
    #[must_use]
    pub fn computed_box(&self, node: NodeHandle) -> Option<ResolvedBox> {
        let cb = self.forest.boxed(node.0)?;
        // Unreached nodes left as the slab default — distinguish from
        // a real (0, 0, 0, 0) result by returning `None`.
        if cb == crate::forest::ComputedBox::default() {
            return None;
        }
        Some(ResolvedBox {
            x: cb.x,
            y: cb.y,
            w: cb.w,
            h: cb.h,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;
    use crate::style_data::{Dimension, Display, Size as StyleSize, StyleData};
    use sigil_kernel::TextContent;
    use taffy::prelude::{AvailableSpace as TaffyAvailable, Size as TaffySize};

    struct NullText;
    impl TextMeasure for NullText {
        fn measure(&self, _: &TextContent, _: TaffySize<TaffyAvailable>) -> TaffySize<f32> {
            TaffySize::ZERO
        }
    }

    fn drive() -> LayoutDrive {
        LayoutDrive::new(Box::new(NullText))
    }

    fn fixed(w: f32, h: f32) -> StyleData {
        StyleData {
            size: StyleSize::new(Dimension::Px(w), Dimension::Px(h)),
            ..StyleData::default()
        }
    }

    fn flex_fixed(w: f32, h: f32) -> StyleData {
        let mut s = fixed(w, h);
        s.display = Display::Flex;
        s
    }

    #[test]
    fn single_block_root_resolves_to_viewport() {
        let mut d = drive();
        let root = d.insert(StyleData::default());
        d.measure(root, StyleSize::new(640.0, 480.0)).unwrap();
        let box_root = d.computed_box(root).expect("root reached");
        assert_eq!(box_root.w, 640.0);
        assert_eq!(box_root.h, 0.0_f32.max(0.0)); // auto height with no children
        assert_eq!(box_root.x, 0.0);
        assert_eq!(box_root.y, 0.0);
    }

    #[test]
    fn single_block_root_with_fixed_size() {
        let mut d = drive();
        let root = d.insert(fixed(120.0, 80.0));
        d.measure(root, StyleSize::new(640.0, 480.0)).unwrap();
        let b = d.computed_box(root).unwrap();
        assert_eq!(b.w, 120.0);
        assert_eq!(b.h, 80.0);
    }

    #[test]
    fn block_root_with_block_child_writes_both_boxes() {
        let mut d = drive();
        let root = d.insert(fixed(200.0, 200.0));
        let child = d.insert(fixed(50.0, 40.0));
        d.attach(root, child).unwrap();
        d.measure(root, StyleSize::new(400.0, 400.0)).unwrap();
        let rb = d.computed_box(root).unwrap();
        let cb = d.computed_box(child).unwrap();
        assert_eq!(rb.w, 200.0);
        assert_eq!(rb.h, 200.0);
        assert_eq!(cb.w, 50.0);
        assert_eq!(cb.h, 40.0);
        // Child placed at parent border-box origin (no padding).
        assert_eq!(cb.x, 0.0);
        assert_eq!(cb.y, 0.0);
    }

    #[test]
    fn flex_root_dispatches_through_flex_solver() {
        let mut d = drive();
        let root = d.insert(flex_fixed(300.0, 100.0));
        let a = d.insert(fixed(50.0, 50.0));
        let b = d.insert(fixed(80.0, 50.0));
        d.attach(root, a).unwrap();
        d.attach(root, b).unwrap();
        d.measure(root, StyleSize::new(640.0, 480.0)).unwrap();
        let rb = d.computed_box(root).unwrap();
        assert_eq!(rb.w, 300.0);
        assert_eq!(rb.h, 100.0);
        let ab = d.computed_box(a).unwrap();
        let bb = d.computed_box(b).unwrap();
        // Flex Row direction: items lay out horizontally. First item
        // at x=0, second item at x = 50.
        assert_eq!(ab.x, 0.0);
        assert_eq!(bb.x, 50.0);
        assert_eq!(ab.w, 50.0);
        assert_eq!(bb.w, 80.0);
    }

    #[test]
    fn computed_box_for_unreached_handle_returns_none() {
        let mut d = drive();
        let root = d.insert(fixed(40.0, 40.0));
        let orphan = d.insert(fixed(10.0, 10.0));
        d.measure(root, StyleSize::new(100.0, 100.0)).unwrap();
        assert!(d.computed_box(root).is_some());
        // Orphan not attached anywhere — never reached by the walk.
        assert!(d.computed_box(orphan).is_none());
    }

    #[test]
    fn measure_is_idempotent_under_same_inputs() {
        let mut d = drive();
        let root = d.insert(fixed(64.0, 48.0));
        let child = d.insert(fixed(16.0, 16.0));
        d.attach(root, child).unwrap();
        let viewport = StyleSize::new(200.0, 200.0);
        d.measure(root, viewport).unwrap();
        let first_root = d.computed_box(root).unwrap();
        let first_child = d.computed_box(child).unwrap();
        d.measure(root, viewport).unwrap();
        assert_eq!(d.computed_box(root).unwrap(), first_root);
        assert_eq!(d.computed_box(child).unwrap(), first_child);
    }

    #[test]
    fn stale_root_handle_errors() {
        let drive_a = drive();
        // Synthesize a handle the driver did not mint by reusing one
        // from an empty drive — its inner LocalId resolves to no node.
        let mut drive_b = LayoutDrive::new(Box::new(NullText));
        let id = drive_b.insert(StyleData::default());
        drop(drive_a);
        // Different LayoutDrive than the one that minted the id.
        let mut other = LayoutDrive::new(Box::new(NullText));
        let err = other.measure(id, StyleSize::new(10.0, 10.0));
        assert!(err.is_err());
    }
}
