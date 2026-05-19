//! Owned-engine vs Taffy comparison — the real-world shape: a
//! ~100k-node tree where ONE leaf style changes between frames.
//!
//! Why this matters: most production UIs do not re-lay out the whole
//! tree every frame. They mutate one or a few leaves (text field,
//! hover indicator, animation tick). Spineless Traversal (PLDI 2025)
//! and Flutter's relayout-boundary model both target this scenario.
//! Our per-node `Node::version` short-circuit makes the cost
//! proportional to nodes that actually changed, not tree size.
//!
//! Run with `cargo bench -p sigil-layout --bench taffy_corpus_incremental`.

#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used
)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil_hal::{LogicalSize, Size};
use sigil_kernel::{Cx, NodeBuilder, NodeId, TextContent, container};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};
use taffy::prelude::*;

use ahash as _;
use allocation_counter as _;
use rayon as _;
use sigil_reactive as _;
use sigil_style as _;
use slotmap as _;
use smallvec as _;

const ROOT_WIDTH: f32 = 12_800.0;
const ROOT_HEIGHT: f32 = 100.0;
const CHILD_WIDTH: f32 = 12.0;
const CHILD_HEIGHT: f32 = 100.0;
const LEAF_WIDTH: f32 = 12.0;
const LEAF_HEIGHT: f32 = 1.0;
const CHILDREN: usize = 1_000;
const LEAVES_PER_CHILD: usize = 100;
const AVAILABLE: Size = Size::fixed(ROOT_WIDTH, ROOT_HEIGHT);

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _: &TextContent, _: Size) -> LogicalSize {
        LogicalSize::new(0.0, 0.0)
    }
}

fn fixed_available(width: f32, height: f32) -> taffy::Size<taffy::AvailableSpace> {
    taffy::Size {
        width: taffy::AvailableSpace::Definite(width),
        height: taffy::AvailableSpace::Definite(height),
    }
}

fn taffy_fixed_style(
    display: taffy::Display,
    direction: taffy::FlexDirection,
    width: f32,
    height: f32,
) -> taffy::Style {
    taffy::Style {
        display,
        flex_direction: direction,
        size: taffy::Size {
            width: taffy::Dimension::from_length(width),
            height: taffy::Dimension::from_length(height),
        },
        ..Default::default()
    }
}

// === owned engine ===

fn build_owned(cx: &mut Cx) -> (NodeId, NodeId) {
    let mut root = container().layout(LayoutStyle::flex_row().fixed_size(ROOT_WIDTH, ROOT_HEIGHT));
    for _ in 0..CHILDREN {
        root = root.child(column());
    }
    let root_id = cx.add(root);
    // Pick the first leaf under the first column — that's our
    // mutation target. We re-find it via the kernel tree walk to
    // avoid coupling the helper to internal id state.
    let mut leaf = root_id;
    let first_col = cx.nth_child(root_id, 0).expect("root has a first column");
    leaf = cx.nth_child(first_col, 0).expect("column has a first leaf");
    (root_id, leaf)
}

fn column() -> NodeBuilder {
    let mut child =
        container().layout(LayoutStyle::flex_column().fixed_size(CHILD_WIDTH, CHILD_HEIGHT));
    for _ in 0..LEAVES_PER_CHILD {
        child = child
            .child(container().layout(LayoutStyle::block().fixed_size(LEAF_WIDTH, LEAF_HEIGHT)));
    }
    child
}

fn bench_owned_incremental(c: &mut Criterion) {
    c.bench_function("incremental_one_leaf/owned", |b| {
        let mut cx = Cx::new_root();
        let (root, leaf) = build_owned(&mut cx);
        let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        engine.compute(&mut cx, root, AVAILABLE).unwrap();
        cx.clear_dirty();

        let mut width = LEAF_WIDTH;
        b.iter(|| {
            // Mutate ONE leaf's width by attaching a fresh
            // LayoutStyle. This is the realistic shape: a single
            // node changes per frame, the rest of the tree is
            // unchanged. Our per-node version stamp short-circuits
            // sync work for the 100,999 nodes we did not touch.
            width = if width >= LEAF_WIDTH + 10.0 {
                LEAF_WIDTH
            } else {
                width + 1.0
            };
            cx.get_mut(leaf)
                .expect("leaf is live")
                .attach(LayoutStyle::block().fixed_size(width, LEAF_HEIGHT));
            engine.compute(&mut cx, root, AVAILABLE).unwrap();
            black_box(engine.result(root));
        });
    });
}

// === taffy directly ===

fn build_taffy(tree: &mut taffy::TaffyTree<()>) -> (taffy::NodeId, taffy::NodeId) {
    let leaf_style = taffy_fixed_style(
        taffy::Display::Block,
        taffy::FlexDirection::Row,
        LEAF_WIDTH,
        LEAF_HEIGHT,
    );
    let mut columns = Vec::with_capacity(CHILDREN);
    let mut first_leaf = None;
    for col_idx in 0..CHILDREN {
        let mut leaves = Vec::with_capacity(LEAVES_PER_CHILD);
        for leaf_idx in 0..LEAVES_PER_CHILD {
            let leaf = tree.new_leaf(leaf_style.clone()).unwrap();
            if col_idx == 0 && leaf_idx == 0 {
                first_leaf = Some(leaf);
            }
            leaves.push(leaf);
        }
        let column = tree
            .new_with_children(
                taffy_fixed_style(
                    taffy::Display::Flex,
                    taffy::FlexDirection::Column,
                    CHILD_WIDTH,
                    CHILD_HEIGHT,
                ),
                &leaves,
            )
            .unwrap();
        columns.push(column);
    }
    let root = tree
        .new_with_children(
            taffy_fixed_style(
                taffy::Display::Flex,
                taffy::FlexDirection::Row,
                ROOT_WIDTH,
                ROOT_HEIGHT,
            ),
            &columns,
        )
        .unwrap();
    (root, first_leaf.expect("at least one leaf"))
}

fn bench_taffy_incremental(c: &mut Criterion) {
    c.bench_function("incremental_one_leaf/taffy", |b| {
        let mut tree = taffy::TaffyTree::<()>::new();
        let (root, leaf) = build_taffy(&mut tree);
        tree.compute_layout(root, fixed_available(ROOT_WIDTH, ROOT_HEIGHT))
            .unwrap();

        let mut width = LEAF_WIDTH;
        b.iter(|| {
            width = if width >= LEAF_WIDTH + 10.0 {
                LEAF_WIDTH
            } else {
                width + 1.0
            };
            tree.set_style(
                leaf,
                taffy_fixed_style(
                    taffy::Display::Block,
                    taffy::FlexDirection::Row,
                    width,
                    LEAF_HEIGHT,
                ),
            )
            .unwrap();
            tree.compute_layout(root, fixed_available(ROOT_WIDTH, ROOT_HEIGHT))
                .unwrap();
            black_box(tree.layout(root).unwrap());
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(50)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5));
    targets = bench_owned_incremental, bench_taffy_incremental,
);
criterion_main!(benches);
