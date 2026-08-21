#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Owned-engine vs Taffy comparison - `wide_tree` scene from Taffy's
//! published bench corpus.
//!
//! Run with `cargo bench -p sigil-layout --bench taffy_corpus_wide_tree`.

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

// allow unused dev-dep extern crates per repo convention

const ROOT_WIDTH: f32 = 12_800.0;
const ROOT_HEIGHT: f32 = 100.0;
const CHILD_WIDTH: f32 = 12.0;
const CHILD_HEIGHT: f32 = 100.0;
const GRANDCHILD_WIDTH: f32 = 12.0;
const GRANDCHILD_HEIGHT: f32 = 1.0;
const CHILDREN: usize = 1_000;
const GRANDCHILDREN_PER_CHILD: usize = 100;
const AVAILABLE: Size = Size::fixed(ROOT_WIDTH, ROOT_HEIGHT);

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _: &TextContent, _: Size) -> LogicalSize {
        LogicalSize::new(0.0, 0.0)
    }
}

const fn fixed_available(width: f32, height: f32) -> taffy::Size<taffy::AvailableSpace> {
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

fn build_owned_wide_tree(cx: &mut Cx) -> NodeId {
    let mut root = container().layout(LayoutStyle::flex_row().fixed_size(ROOT_WIDTH, ROOT_HEIGHT));
    for _ in 0..CHILDREN {
        root = root.child(owned_wide_tree_child());
    }
    cx.add(root)
}

fn owned_wide_tree_child() -> NodeBuilder {
    let mut child =
        container().layout(LayoutStyle::flex_column().fixed_size(CHILD_WIDTH, CHILD_HEIGHT));
    for _ in 0..GRANDCHILDREN_PER_CHILD {
        child = child.child(
            container()
                .layout(LayoutStyle::block().fixed_size(GRANDCHILD_WIDTH, GRANDCHILD_HEIGHT)),
        );
    }
    child
}

fn bench_owned_wide_tree(c: &mut Criterion) {
    c.bench_function("wide_tree/owned", |b| {
        let mut cx = Cx::new_root();
        let root = build_owned_wide_tree(&mut cx);
        let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        engine.compute(&mut cx, root, AVAILABLE).unwrap();
        cx.clear_dirty();

        b.iter(|| {
            cx.mark_mutated();
            engine.compute(&mut cx, root, AVAILABLE).unwrap();
            black_box(engine.result(root));
        });
    });
}

// === taffy directly ===

fn build_taffy_wide_tree(tree: &mut taffy::TaffyTree<()>) -> taffy::NodeId {
    let mut children = Vec::with_capacity(CHILDREN);
    for _ in 0..CHILDREN {
        children.push(build_taffy_wide_tree_child(tree));
    }

    tree.new_with_children(
        taffy_fixed_style(
            taffy::Display::Flex,
            taffy::FlexDirection::Row,
            ROOT_WIDTH,
            ROOT_HEIGHT,
        ),
        &children,
    )
    .unwrap()
}

fn build_taffy_wide_tree_child(tree: &mut taffy::TaffyTree<()>) -> taffy::NodeId {
    let leaf_style = taffy_fixed_style(
        taffy::Display::Block,
        taffy::FlexDirection::Row,
        GRANDCHILD_WIDTH,
        GRANDCHILD_HEIGHT,
    );
    let mut grandchildren = Vec::with_capacity(GRANDCHILDREN_PER_CHILD);
    for _ in 0..GRANDCHILDREN_PER_CHILD {
        grandchildren.push(tree.new_leaf(leaf_style.clone()).unwrap());
    }

    tree.new_with_children(
        taffy_fixed_style(
            taffy::Display::Flex,
            taffy::FlexDirection::Column,
            CHILD_WIDTH,
            CHILD_HEIGHT,
        ),
        &grandchildren,
    )
    .unwrap()
}

fn bench_taffy_wide_tree(c: &mut Criterion) {
    c.bench_function("wide_tree/taffy", |b| {
        let mut tree = taffy::TaffyTree::<()>::new();
        let root = build_taffy_wide_tree(&mut tree);
        tree.compute_layout(root, fixed_available(ROOT_WIDTH, ROOT_HEIGHT))
            .unwrap();

        b.iter(|| {
            tree.mark_dirty(root).unwrap();
            tree.compute_layout(root, fixed_available(ROOT_WIDTH, ROOT_HEIGHT))
                .unwrap();
            black_box(tree.layout(root).unwrap());
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(30)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5));
    targets = bench_owned_wide_tree, bench_taffy_wide_tree,
);
criterion_main!(benches);
