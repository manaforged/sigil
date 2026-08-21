#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
//! Phase 10 layout invalidation benchmarks.
//!
//! Run with `cargo bench -p sigil-layout --bench invalidation_rig`.

#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used
)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use criterion::{Criterion, criterion_group, criterion_main};
use sigil_hal::{LogicalSize, Size};
use sigil_kernel::{Cx, NodeBuilder, NodeId, TextContent, container};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};

const ROOT_WIDTH: f32 = 1280.0;
const ROOT_HEIGHT: f32 = 720.0;
const ROWS: usize = 100;
const LEAVES_PER_ROW: usize = 100;
const ROW_HEIGHT: f32 = 7.2;
const LEAF_WIDTH: f32 = 12.8;
const AVAILABLE: Size = Size::fixed(ROOT_WIDTH, ROOT_HEIGHT);

struct StubMeasure;

impl TextMeasure for StubMeasure {
    #[allow(clippy::cast_precision_loss)]
    fn measure(&self, source: &TextContent, _space: Size) -> LogicalSize {
        let chars = match source {
            TextContent::Static(value) => value.chars().count() as f32,
            TextContent::Reactive(_) | TextContent::Compute(_) => 0.0,
        };
        LogicalSize::new(chars * 8.0, 16.0)
    }
}

fn bench_static_frame(c: &mut Criterion) {
    c.bench_function("layout_invalidation/S1_static_10k", |b| {
        let (mut cx, root, mut engine) = build_10k_tree();
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("warm layout compute succeeds");

        b.iter(|| {
            engine
                .compute(&mut cx, root, AVAILABLE)
                .expect("cached layout compute succeeds");
            black_box(engine.result(root));
        });
    });
}

fn bench_one_mutation(c: &mut Criterion) {
    c.bench_function("layout_invalidation/S2_one_mutation_10k", |b| {
        let (mut cx, root, mut engine) = build_10k_tree();
        let leaves = collect_leaf_ids(&cx, root);
        let mut idx = 0_usize;
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("warm layout compute succeeds");

        b.iter(|| {
            let leaf = leaves
                .get(idx % leaves.len())
                .copied()
                .expect("leaf exists");
            let _ = cx.get_mut(leaf).expect("leaf exists");
            cx.mark_mutated();
            engine
                .compute(&mut cx, root, AVAILABLE)
                .expect("mutated layout compute succeeds");
            idx = idx.wrapping_add(1);
            black_box(engine.result(root));
        });
    });
}

fn bench_structural_change(c: &mut Criterion) {
    c.bench_function("layout_invalidation/S3_structural_10k", |b| {
        let (mut cx, root, mut engine) = build_10k_tree();
        let rows = collect_row_ids(&cx, root);
        let mut idx = 0_usize;
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("warm layout compute succeeds");

        b.iter_custom(|iters| {
            let mut elapsed = Duration::ZERO;
            for _ in 0..iters {
                let row = rows.get(idx % rows.len()).copied().expect("row exists");
                idx = idx.wrapping_add(1);
                let start = Instant::now();
                let added = cx.add_child(row, leaf());
                engine
                    .compute(&mut cx, root, AVAILABLE)
                    .expect("structural layout compute succeeds");
                elapsed += start.elapsed();
                black_box(engine.result(added));
                assert!(cx.remove(added));
                engine.gc(&cx).expect("cleanup gc succeeds");
            }
            elapsed
        });
    });
}

fn build_10k_tree() -> (Cx, NodeId, LayoutEngine) {
    let mut cx = Cx::new_root();
    let mut root_builder =
        container().layout(LayoutStyle::flex_column().fixed_size(ROOT_WIDTH, ROOT_HEIGHT));
    for _ in 0..ROWS {
        root_builder = root_builder.child(row());
    }
    let root = cx.add(root_builder);
    let engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    (cx, root, engine)
}

fn row() -> NodeBuilder {
    let mut row_builder =
        container().layout(LayoutStyle::flex_row().fixed_size(ROOT_WIDTH, ROW_HEIGHT));
    for _ in 0..LEAVES_PER_ROW {
        row_builder = row_builder.child(leaf());
    }
    row_builder
}

fn leaf() -> NodeBuilder {
    container().layout(LayoutStyle::block().fixed_size(LEAF_WIDTH, ROW_HEIGHT))
}

fn collect_row_ids(cx: &Cx, root: NodeId) -> Vec<NodeId> {
    cx.get(root).expect("root exists").children().to_vec()
}

fn collect_leaf_ids(cx: &Cx, root: NodeId) -> Vec<NodeId> {
    let rows = collect_row_ids(cx, root);
    let mut leaves = Vec::with_capacity(ROWS * LEAVES_PER_ROW);
    for row in rows {
        leaves.extend(cx.get(row).expect("row exists").children().iter().copied());
    }
    leaves
}

criterion_group!(
    benches,
    bench_static_frame,
    bench_one_mutation,
    bench_structural_change,
);
criterion_main!(benches);
