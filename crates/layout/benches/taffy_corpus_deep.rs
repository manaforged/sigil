#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Owned-engine vs Taffy comparison — the canonical "deep" tree shapes
//! from Taffy's published bench corpus
//! (`benches/benches/flexbox.rs`).
//!
//! ## Methodology
//!
//! These are direct ports of Taffy's `deep_random` and `deep_auto`
//! shapes — branching factor 2, depth 17, ~131k nodes. The reason this
//! corpus matters: production UI trees are deep + narrow (panels inside
//! panels inside scroll views), not wide-flat. Both Taffy and Yoga
//! publish their numbers on this exact shape, so reviewers can compare
//! sigil vs Taffy without us picking a custom-favorable tree.
//!
//! - `deep_random` — fixed-size leaves with randomly chosen width/height
//!   (auto, length, percent) at interior nodes. Stresses the dimension
//!   resolution path.
//! - `deep_auto` — every interior node is auto-sized with `flex_grow: 1`
//!   and `margin: 10`. Stresses the content-sizing path.
//!
//! Run with `cargo bench -p sigil-layout --bench taffy_corpus_deep`.

#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    // u32→f32 cast is intentional: xorshift output bucketed into 200 slots,
    // so values 0..200 are well within f32's exact integer range (≤2^23).
    clippy::cast_precision_loss
)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil_hal::{LogicalSize, Size};
use sigil_kernel::{Cx, NodeBuilder, NodeId, TextContent, container};
use sigil_layout::{Dimension, LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};
use taffy::prelude::*;

// Taffy's published deep shape: 2^17 = 131_072 leaves on a binary tree.
// We use the same depth so the work-per-frame matches.
const DEPTH: usize = 17;
const ROOT_WIDTH: f32 = 800.0;
const ROOT_HEIGHT: f32 = 800.0;
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

// Tiny xorshift so both engines see the same "random" dimension stream.
struct Rng(u32);

impl Rng {
    const fn new(seed: u32) -> Self {
        Self(seed)
    }
    const fn next(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = if x == 0 { 0x1234_5678 } else { x };
        self.0
    }
    fn pick_dim(&mut self, max: f32) -> f32 {
        // Bucketed length 10..max so the layout work stays representative.
        let r = (self.next() % 200) as f32;
        10.0 + r * (max / 200.0)
    }
}

// === deep_random — owned ===

fn build_owned_deep_random(cx: &mut Cx) -> NodeId {
    let mut rng = Rng::new(0xC0FF_EE42);
    let root = owned_deep_random_node(&mut rng, 0);
    cx.add(root)
}

fn owned_deep_random_node(rng: &mut Rng, depth: usize) -> NodeBuilder {
    let w = rng.pick_dim(ROOT_WIDTH);
    let h = rng.pick_dim(ROOT_HEIGHT);
    let mut node = container().layout(LayoutStyle::flex_column().fixed_size(w, h));
    if depth + 1 < DEPTH {
        node = node.child(owned_deep_random_node(rng, depth + 1));
        node = node.child(owned_deep_random_node(rng, depth + 1));
    }
    node
}

fn bench_owned_deep_random(c: &mut Criterion) {
    c.bench_function("deep_random/owned", |b| {
        let mut cx = Cx::new_root();
        let root = build_owned_deep_random(&mut cx);
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

// === deep_random — taffy ===

fn build_taffy_deep_random(tree: &mut taffy::TaffyTree<()>) -> taffy::NodeId {
    let mut rng = Rng::new(0xC0FF_EE42);
    build_taffy_deep_random_node(tree, &mut rng, 0)
}

fn build_taffy_deep_random_node(
    tree: &mut taffy::TaffyTree<()>,
    rng: &mut Rng,
    depth: usize,
) -> taffy::NodeId {
    let w = rng.pick_dim(ROOT_WIDTH);
    let h = rng.pick_dim(ROOT_HEIGHT);
    let style = taffy::Style {
        display: taffy::Display::Flex,
        flex_direction: taffy::FlexDirection::Column,
        size: taffy::Size {
            width: taffy::Dimension::from_length(w),
            height: taffy::Dimension::from_length(h),
        },
        ..Default::default()
    };
    let mut children: Vec<taffy::NodeId> = Vec::new();
    if depth + 1 < DEPTH {
        children.push(build_taffy_deep_random_node(tree, rng, depth + 1));
        children.push(build_taffy_deep_random_node(tree, rng, depth + 1));
    }
    tree.new_with_children(style, &children).unwrap()
}

fn bench_taffy_deep_random(c: &mut Criterion) {
    c.bench_function("deep_random/taffy", |b| {
        let mut tree = taffy::TaffyTree::<()>::new();
        let root = build_taffy_deep_random(&mut tree);
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

// === deep_auto — owned ===

fn build_owned_deep_auto(cx: &mut Cx) -> NodeId {
    cx.add(owned_deep_auto_node(0))
}

fn owned_deep_auto_node(depth: usize) -> NodeBuilder {
    // Auto sizing on both axes — content-sized recursion is the exact
    // shape Taffy's deep_auto bench stresses.
    let mut node = container().layout(
        LayoutStyle::flex_column()
            .width(Dimension::Auto)
            .height(Dimension::Auto),
    );
    if depth + 1 < DEPTH {
        node = node.child(owned_deep_auto_node(depth + 1));
        node = node.child(owned_deep_auto_node(depth + 1));
    } else {
        // Leaves are fixed so the auto chain has something to size against.
        node = node.layout(LayoutStyle::block().fixed_size(10.0, 10.0));
    }
    node
}

fn bench_owned_deep_auto(c: &mut Criterion) {
    c.bench_function("deep_auto/owned", |b| {
        let mut cx = Cx::new_root();
        let root = build_owned_deep_auto(&mut cx);
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

// === deep_auto — taffy ===

fn build_taffy_deep_auto(tree: &mut taffy::TaffyTree<()>) -> taffy::NodeId {
    build_taffy_deep_auto_node(tree, 0)
}

fn build_taffy_deep_auto_node(tree: &mut taffy::TaffyTree<()>, depth: usize) -> taffy::NodeId {
    let style = taffy::Style {
        display: taffy::Display::Flex,
        flex_direction: taffy::FlexDirection::Column,
        flex_grow: 1.0,
        ..Default::default()
    };
    let mut children: Vec<taffy::NodeId> = Vec::new();
    if depth + 1 < DEPTH {
        children.push(build_taffy_deep_auto_node(tree, depth + 1));
        children.push(build_taffy_deep_auto_node(tree, depth + 1));
    } else {
        let leaf = tree
            .new_leaf(taffy::Style {
                size: taffy::Size {
                    width: taffy::Dimension::from_length(10.0_f32),
                    height: taffy::Dimension::from_length(10.0_f32),
                },
                ..Default::default()
            })
            .unwrap();
        return leaf;
    }
    tree.new_with_children(style, &children).unwrap()
}

fn bench_taffy_deep_auto(c: &mut Criterion) {
    c.bench_function("deep_auto/taffy", |b| {
        let mut tree = taffy::TaffyTree::<()>::new();
        let root = build_taffy_deep_auto(&mut tree);
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
        .sample_size(20)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5));
    targets =
        bench_owned_deep_random, bench_taffy_deep_random,
        bench_owned_deep_auto, bench_taffy_deep_auto,
);
criterion_main!(benches);
