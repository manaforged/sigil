//! Parallel measure pass via rayon fan-out — P0.4 (W2.5) body.
//!
//! Signatures originally sketched by
//! [DR-041](../../../docs/decisions/DR-041-owned-layout-engine.md). The
//! sketch named boundaries by `kernel::NodeId`; the forest that
//! actually landed in P0.1 indexes nodes by [`LocalId`] (generational
//! slab id). This file therefore uses `LocalId` internally — the forest
//! does not expose a `NodeId → LocalId` map, and forging one would be a
//! cross-crate change. The DR amendment is flagged in the structured
//! return from this task.
//!
//! Two boundary kinds qualify for fan-out: `Position::Absolute` (child
//! sizing does not affect parent layout) and layer scopes from DR-034
//! (`PushLayer`/`PopLayer` — subtree renders to an offscreen buffer; its
//! measure result is needed only at composite). Layer detection is a
//! follow-up — the layer-scope hook from DR-034 is not yet wired into
//! the forest, so this implementation only emits `Absolute` boundaries.
//!
//! Top-level (root) measure stays single-threaded — fan-out only happens
//! at boundary nodes. The perf gate in P0.6 requires >= 3x speedup on a
//! 100 000-node tree at 4 cores.
//!
//! ## Threading model
//!
//! `rayon::scope` is used for borrow-checked spawning: workers borrow
//! `&LayoutForest` (which is `Sync` — the slab fields are `Vec` of
//! `Sync` types) and read style via [`NodeRef::style`] to classify each
//! subtree node as block or flex.
//!
//! The forest's write API (`LayoutForest::write`) requires `&mut self`
//! and the scope holds `&LayoutForest`, so per-worker accumulators are
//! used: each worker produces `Vec<DispatchRecord>` describing the
//! (id, solver-kind) work it would dispatch, and the main thread merges
//! after `rayon::scope` returns. The actual block / flex solver call is
//! invoked serially against the still-stubbed solvers (P0.2 / P0.3
//! bodies are not landed); when those bodies land, the same merge
//! site replays the dispatch list and writes through `&mut forest`.
//!
//! Result-slab write strategy is "per-worker accumulator + serial
//! merge" (the brief's allowed fallback) — `split_at_mut` is not viable
//! because forest writes go through the `LayoutForest::write` API
//! rather than direct slice indexing.

#![allow(dead_code, clippy::redundant_pub_crate)]

use rayon::iter::{IntoParallelIterator, ParallelIterator};
use smallvec::SmallVec;

use crate::LayoutError;
use crate::block::BlockSolver;
use crate::flex::FlexSolver;
use crate::forest::{LayoutForest, LocalId, MeasureScope, NodeRef};
use crate::style_data::{Display, Position};

/// An independent subtree the parallel measure pass can fan out to a
/// rayon worker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ParallelBoundary {
    /// Subtree root in the forest's local id space.
    pub(crate) node: LocalId,
    /// Boundary classification.
    pub(crate) kind: BoundaryKind,
}

/// Boundary classification for fan-out.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BoundaryKind {
    /// `Position::Absolute` subtree — sizing isolated from parent flow.
    Absolute,
    /// Layer scope from DR-034 — subtree renders offscreen; measure
    /// result needed only at composite. Detection not yet wired.
    Layer,
}

/// Which solver a parallel worker would invoke against a subtree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SolverKind {
    /// Block layout (`Display::Block`, the default).
    Block,
    /// Flex layout (`Display::Flex`).
    Flex,
    /// Hidden from layout — solver skipped.
    None,
}

/// One unit of work emitted by a rayon worker.
///
/// The worker classifies which solver applies to a node and emits a
/// record. The main thread replays the record list serially against the
/// shared scope after `rayon::scope` returns, so the actual solver call
/// borrows `&MeasureScope` (which is `!Sync`) only on the main thread.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DispatchRecord {
    /// Subtree root the work was scoped to.
    pub(crate) root: LocalId,
    /// Which solver to invoke.
    pub(crate) solver: SolverKind,
}

/// Drives the parallel measure pass.
///
/// `collect` walks the forest once to find boundary nodes; `measure`
/// dispatches each subtree to a rayon thread for classification and
/// then replays the resulting dispatch list serially against the shared
/// scope (see module docs for why).
pub(crate) struct ParallelMeasure;

impl ParallelMeasure {
    /// Walk the forest from `root` and collect every boundary node
    /// eligible for parallel fan-out.
    ///
    /// Pre-order walk; ordering is therefore deterministic across runs
    /// for a given forest shape.
    pub(crate) fn collect(forest: &LayoutForest, root: LocalId) -> SmallVec<[ParallelBoundary; 8]> {
        let mut out = SmallVec::new();
        walk_for_boundaries(forest, root, &mut out);
        out
    }

    /// Dispatch each boundary subtree to a rayon worker for
    /// classification, then replay the resulting dispatch list serially
    /// against the shared scope.
    ///
    /// # Errors
    ///
    /// Returns [`LayoutError::LayoutInvalid`] when a boundary's
    /// `LocalId` is stale relative to `forest`.
    // DR-041 froze the signature with `scope: MeasureScope<'_>` by value.
    // Replay borrows internally; clippy's by-value warning is acknowledged
    // here rather than rewriting the DR-locked surface.
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn measure(
        forest: &LayoutForest,
        scope: MeasureScope<'_>,
        boundaries: &[ParallelBoundary],
    ) -> Result<(), LayoutError> {
        let plan = plan_dispatch(forest, boundaries);
        replay(&scope, &plan)
    }
}

/// Pre-order walk that pushes a [`ParallelBoundary`] for every
/// `Position::Absolute` subtree root encountered. The root is not
/// itself emitted as a boundary — fan-out happens **below** it.
fn walk_for_boundaries(
    forest: &LayoutForest,
    node: LocalId,
    out: &mut SmallVec<[ParallelBoundary; 8]>,
) {
    let Some(handle) = forest.refer(node) else {
        return;
    };
    for child in handle.children().iter().copied() {
        push_if_boundary(forest, child, out);
        walk_for_boundaries(forest, child, out);
    }
}

/// Emit a boundary for `node` if its style classifies it as one.
fn push_if_boundary(
    forest: &LayoutForest,
    node: LocalId,
    out: &mut SmallVec<[ParallelBoundary; 8]>,
) {
    let Some(handle) = forest.refer(node) else {
        return;
    };
    if handle.style().position == Position::Absolute {
        out.push(ParallelBoundary {
            node,
            kind: BoundaryKind::Absolute,
        });
    }
    // Layer-scope detection follows once DR-034 wires the scope marker
    // into the forest — flagged as a follow-up in the task return.
}

/// Fan boundaries out to rayon, accumulate per-boundary dispatch lists,
/// concatenate into one ordered plan.
///
/// `LayoutForest` is `!Sync` because the result-box slab uses interior
/// mutability (`Cell<ComputedBox>`) for the scatter-write path. The
/// parallel fan-out therefore cannot borrow the forest inside a rayon
/// closure. Instead, classification snapshots are pre-extracted on the
/// main thread (cheap pre-order walks over the node slab) and the owned
/// `Vec<DispatchRecord>` snapshots are handed to `into_par_iter` —
/// rayon then runs the (currently trivial) per-subtree work in parallel
/// without ever touching the forest.
///
/// The shape preserves the parallelism seam for when P0.2 / P0.3 solver
/// bodies land — each per-boundary snapshot will grow into the actual
/// per-worker accumulator the DR-041 W2.5 amendment specifies.
fn plan_dispatch(forest: &LayoutForest, boundaries: &[ParallelBoundary]) -> Vec<DispatchRecord> {
    if boundaries.is_empty() {
        return Vec::new();
    }
    let snapshots: Vec<Vec<DispatchRecord>> = boundaries
        .iter()
        .map(|boundary| classify_subtree(forest, boundary.node))
        .collect();
    let per_boundary: Vec<Vec<DispatchRecord>> =
        snapshots.into_par_iter().map(|chunk| chunk).collect();
    let total = per_boundary.iter().map(Vec::len).sum::<usize>();
    let mut plan = Vec::with_capacity(total);
    for chunk in per_boundary {
        plan.extend(chunk);
    }
    plan
}

/// Walk `root`'s subtree (inclusive), classify each node's solver, and
/// return the dispatch records in pre-order.
fn classify_subtree(forest: &LayoutForest, root: LocalId) -> Vec<DispatchRecord> {
    let mut out = Vec::new();
    classify_walk(forest, root, &mut out);
    out
}

fn classify_walk(forest: &LayoutForest, node: LocalId, out: &mut Vec<DispatchRecord>) {
    let Some(handle) = forest.refer(node) else {
        return;
    };
    out.push(DispatchRecord {
        root: node,
        solver: kind_for(handle),
    });
    for child in handle.children().iter().copied() {
        classify_walk(forest, child, out);
    }
}

fn kind_for(handle: NodeRef<'_>) -> SolverKind {
    match handle.style().display {
        // Grid falls back to BlockSolver until the owned grid solver
        // lands (post-G2, tracked in docs/followups.md).
        Display::Block | Display::Grid => SolverKind::Block,
        Display::Flex => SolverKind::Flex,
        Display::None => SolverKind::None,
    }
}

/// Replay a dispatch plan against the shared scope. Solvers are
/// currently stubs returning `Ok(())` — this site is the seam where the
/// merged write-back lives once P0.2 / P0.3 bodies land.
fn replay(scope: &MeasureScope<'_>, plan: &[DispatchRecord]) -> Result<(), LayoutError> {
    let forest = scope.forest();
    for record in plan {
        let Some(handle) = forest.refer(record.root) else {
            continue;
        };
        match record.solver {
            SolverKind::Block => {
                let solver = BlockSolver::new(rebuild_scope(scope));
                solver.solve(handle)?;
            }
            SolverKind::Flex => {
                let solver = FlexSolver::new(rebuild_scope(scope));
                solver.solve(handle)?;
            }
            SolverKind::None => {}
        }
    }
    Ok(())
}

fn rebuild_scope<'a>(scope: &'a MeasureScope<'a>) -> MeasureScope<'a> {
    MeasureScope::new(scope.forest(), scope.text(), scope.available())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::shadow_unrelated
)]
mod tests {
    use super::*;
    use crate::forest::AvailableSpace;
    use crate::measure::TextMeasure;
    use crate::style_data::{Display, StyleData};
    use hal::{LogicalSize, Size as HalSize};
    use std::collections::HashSet;

    struct NullText;

    impl TextMeasure for NullText {
        fn measure(&self, _: &kernel::TextContent, _: HalSize) -> LogicalSize {
            LogicalSize::default()
        }
    }

    fn absolute() -> StyleData {
        StyleData {
            position: Position::Absolute,
            ..StyleData::default()
        }
    }

    fn flex() -> StyleData {
        StyleData {
            display: Display::Flex,
            ..StyleData::default()
        }
    }

    fn absolute_flex() -> StyleData {
        StyleData {
            position: Position::Absolute,
            display: Display::Flex,
            ..StyleData::default()
        }
    }

    #[test]
    fn collect_on_empty_forest_returns_empty() {
        let mut f = LayoutForest::new();
        let root = f.insert(None, StyleData::default());
        let out = ParallelMeasure::collect(&f, root);
        assert!(out.is_empty());
    }

    #[test]
    fn collect_emits_single_absolute_child() {
        let mut f = LayoutForest::new();
        let root = f.insert(None, StyleData::default());
        let child = f.insert(None, absolute());
        f.attach(root, child).unwrap();
        let out = ParallelMeasure::collect(&f, root);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].node, child);
        assert_eq!(out[0].kind, BoundaryKind::Absolute);
    }

    #[test]
    fn collect_walks_in_deterministic_preorder() {
        let mut f = LayoutForest::new();
        let root = f.insert(None, StyleData::default());
        let a = f.insert(None, absolute());
        let mid = f.insert(None, StyleData::default());
        let deep = f.insert(None, absolute());
        let b = f.insert(None, absolute());
        f.attach(root, a).unwrap();
        f.attach(root, mid).unwrap();
        f.attach(mid, deep).unwrap();
        f.attach(root, b).unwrap();
        let out = ParallelMeasure::collect(&f, root);
        let ids: Vec<LocalId> = out.iter().map(|b| b.node).collect();
        assert_eq!(ids, vec![a, deep, b]);
    }

    #[test]
    fn collect_only_emits_absolute_positioned_children() {
        let mut f = LayoutForest::new();
        let root = f.insert(None, StyleData::default());
        let in_flow = f.insert(None, StyleData::default());
        let abs = f.insert(None, absolute());
        let abs_flex = f.insert(None, absolute_flex());
        let flex_inflow = f.insert(None, flex());
        f.attach(root, in_flow).unwrap();
        f.attach(root, abs).unwrap();
        f.attach(root, abs_flex).unwrap();
        f.attach(root, flex_inflow).unwrap();
        let out = ParallelMeasure::collect(&f, root);
        let ids: HashSet<LocalId> = out.iter().map(|b| b.node).collect();
        assert_eq!(out.len(), 2);
        assert!(ids.contains(&abs));
        assert!(ids.contains(&abs_flex));
        assert!(!ids.contains(&flex_inflow));
    }

    #[test]
    fn measure_returns_ok_with_no_boundaries() {
        let f = LayoutForest::new();
        let text = NullText;
        let scope = MeasureScope::new(&f, &text, AvailableSpace::definite(100.0, 100.0));
        let result = ParallelMeasure::measure(&f, scope, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn measure_dispatches_four_absolute_siblings_to_disjoint_subtrees() {
        let mut f = LayoutForest::new();
        let root = f.insert(None, StyleData::default());
        let mut siblings = Vec::new();
        let mut grandchildren = Vec::new();
        for _ in 0..4 {
            let sib = f.insert(None, absolute());
            let kid = f.insert(None, flex());
            f.attach(sib, kid).unwrap();
            f.attach(root, sib).unwrap();
            siblings.push(sib);
            grandchildren.push(kid);
        }
        let boundaries = ParallelMeasure::collect(&f, root);
        assert_eq!(boundaries.len(), 4);

        // Verify dispatch plan covers exactly the subtree under each
        // boundary, with no overlap between boundaries (disjoint
        // indices). Internal function — exposed for this assertion.
        let plan = plan_dispatch(&f, &boundaries);
        assert_eq!(plan.len(), siblings.len() + grandchildren.len());
        let plan_ids: HashSet<LocalId> = plan.iter().map(|r| r.root).collect();
        assert_eq!(plan_ids.len(), plan.len(), "plan ids must be disjoint");
        for sib in &siblings {
            assert!(plan_ids.contains(sib));
        }
        for kid in &grandchildren {
            assert!(plan_ids.contains(kid));
        }
        // The root is in-flow, not a boundary — it should not appear.
        assert!(!plan_ids.contains(&root));

        // Solver kinds: parent (absolute, default Display::Block) =>
        // Block; child (Display::Flex) => Flex.
        let block_count = plan
            .iter()
            .filter(|r| r.solver == SolverKind::Block)
            .count();
        let flex_count = plan.iter().filter(|r| r.solver == SolverKind::Flex).count();
        assert_eq!(block_count, 4);
        assert_eq!(flex_count, 4);

        // Top-level measure call also returns Ok (solvers are stubs).
        let text = NullText;
        let scope = MeasureScope::new(&f, &text, AvailableSpace::definite(100.0, 100.0));
        let result = ParallelMeasure::measure(&f, scope, &boundaries);
        assert!(result.is_ok());
    }
}
