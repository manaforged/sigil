//! Owned-engine vs Taffy comparison - `huge_nested` scene from Taffy's
//! published bench corpus.
//!
//! Run with `cargo bench -p sigil-layout --bench taffy_corpus_huge_nested`.

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
use ahash as _;
use allocation_counter as _;
use rayon as _;
use sigil_profile as _;
use sigil_reactive as _;
use sigil_style as _;
use slotmap as _;
use smallvec as _;

const NODE_COUNT: usize = 10_000;
const NODE_SIZE: f32 = 100.0;
const AVAILABLE: Size = Size::fixed(NODE_SIZE, NODE_SIZE);

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

fn taffy_flex_column_fixed(width: f32, height: f32) -> taffy::Style {
    taffy::Style {
        display: taffy::Display::Flex,
        flex_direction: taffy::FlexDirection::Column,
        size: taffy::Size {
            width: taffy::Dimension::from_length(width),
            height: taffy::Dimension::from_length(height),
        },
        ..Default::default()
    }
}

// === owned engine ===

fn build_owned_huge_nested(cx: &mut Cx) -> NodeId {
    cx.add(owned_huge_nested_node(0))
}

fn owned_huge_nested_node(index: usize) -> NodeBuilder {
    let mut node = container().layout(LayoutStyle::flex_column().fixed_size(NODE_SIZE, NODE_SIZE));

    let left = index.saturating_mul(2).saturating_add(1);
    let right = left.saturating_add(1);
    if left < NODE_COUNT {
        node = node.child(owned_huge_nested_node(left));
    }
    if right < NODE_COUNT {
        node = node.child(owned_huge_nested_node(right));
    }

    node
}

fn bench_owned_huge_nested(c: &mut Criterion) {
    c.bench_function("huge_nested/owned", |b| {
        let mut cx = Cx::new_root();
        let root = build_owned_huge_nested(&mut cx);
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

fn build_taffy_huge_nested(tree: &mut taffy::TaffyTree<()>) -> taffy::NodeId {
    build_taffy_huge_nested_node(tree, 0)
}

fn build_taffy_huge_nested_node(tree: &mut taffy::TaffyTree<()>, index: usize) -> taffy::NodeId {
    let left = index.saturating_mul(2).saturating_add(1);
    let right = left.saturating_add(1);
    let mut children = Vec::with_capacity(2);
    if left < NODE_COUNT {
        children.push(build_taffy_huge_nested_node(tree, left));
    }
    if right < NODE_COUNT {
        children.push(build_taffy_huge_nested_node(tree, right));
    }

    tree.new_with_children(taffy_flex_column_fixed(NODE_SIZE, NODE_SIZE), &children)
        .unwrap()
}

fn bench_taffy_huge_nested(c: &mut Criterion) {
    c.bench_function("huge_nested/taffy", |b| {
        let mut tree = taffy::TaffyTree::<()>::new();
        let root = build_taffy_huge_nested(&mut tree);
        tree.compute_layout(root, fixed_available(NODE_SIZE, NODE_SIZE))
            .unwrap();

        b.iter(|| {
            tree.mark_dirty(root).unwrap();
            tree.compute_layout(root, fixed_available(NODE_SIZE, NODE_SIZE))
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
    targets = bench_owned_huge_nested, bench_taffy_huge_nested,
);
criterion_main!(benches);
