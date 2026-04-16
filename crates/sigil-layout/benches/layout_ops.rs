//! Tier 1/2/4 layout operation benchmarks for `sigil-layout`.
//!
//! Run with `cargo bench -p sigil-layout`.

#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used
)]

use std::hint::black_box;

use ahash as _;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use sigil_core::TextSource;
use sigil_core::{Cx, NodeBuilder, NodeId, container, text};
use sigil_hal::Size;
use sigil_layout::{LayoutEngine, LayoutIntent, TextMeasure};
use smallvec as _;
use taffy as _;
use taffy::prelude::{AvailableSpace, Size as TaffySize};

struct StubMeasure;
impl TextMeasure for StubMeasure {
    #[allow(clippy::cast_precision_loss)]
    fn measure(&self, source: &TextSource, _space: TaffySize<AvailableSpace>) -> TaffySize<f32> {
        let chars = match source {
            TextSource::Static(v) => v.chars().count() as f32,
            TextSource::Reactive(_) => 0.0,
        };
        TaffySize {
            width: chars * 8.0,
            height: 16.0,
        }
    }
}

fn stub_engine() -> LayoutEngine {
    LayoutEngine::with_text_measure(Box::new(StubMeasure))
}

fn tree_with_text_children(count: usize) -> NodeBuilder {
    let mut builder = container();
    for i in 0..count {
        builder = builder.child(text(format!("node {i}")));
    }
    builder
}

fn nested_subtree_with_text_children(count: usize) -> NodeBuilder {
    container().child(tree_with_text_children(count))
}

fn attach_root_intent(cx: &mut Cx, root: NodeId, intent: LayoutIntent) {
    let _ = cx.get_mut(root).expect("root exists").attach(intent);
}

fn bench_compute_single_node(c: &mut Criterion) {
    c.bench_function("compute_single_node", |b| {
        let mut cx = Cx::new_root();
        let root = cx.add(container());
        attach_root_intent(
            &mut cx,
            root,
            LayoutIntent::flex_row().fixed_size(100.0, 40.0),
        );
        let mut engine = stub_engine();

        b.iter(|| {
            engine
                .compute(&mut cx, root, Size::fixed(100.0, 40.0))
                .expect("layout compute succeeds");
            black_box(engine.result(root));
        });
    });
}

fn bench_compute_flex_row_100_children(c: &mut Criterion) {
    c.bench_function("compute_flex_row_100_children", |b| {
        let mut cx = Cx::new_root();
        let root = cx.add(tree_with_text_children(100));
        attach_root_intent(
            &mut cx,
            root,
            LayoutIntent::flex_row().fixed_size(1200.0, 40.0),
        );
        let mut engine = stub_engine();

        b.iter(|| {
            engine
                .compute(&mut cx, root, Size::fixed(1200.0, 40.0))
                .expect("layout compute succeeds");
            black_box(engine.result(root));
        });
    });
}

fn bench_compute_tree_100(c: &mut Criterion) {
    c.bench_function("compute_tree_100", |b| {
        let mut cx = Cx::new_root();
        let root = cx.add(tree_with_text_children(100));
        attach_root_intent(
            &mut cx,
            root,
            LayoutIntent::flex_column().fixed_size(800.0, 1200.0),
        );
        let mut engine = stub_engine();

        b.iter(|| {
            engine
                .compute(&mut cx, root, Size::fixed(800.0, 1200.0))
                .expect("layout compute succeeds");
            black_box(engine.result(root));
        });
    });
}

fn bench_sync_tree_1000_nodes(c: &mut Criterion) {
    c.bench_function("sync_tree_1000_nodes", |b| {
        let mut cx = Cx::new_root();
        let root = cx.add(tree_with_text_children(1000));
        attach_root_intent(
            &mut cx,
            root,
            LayoutIntent::flex_column().fixed_size(800.0, 12_000.0),
        );
        let mut engine = stub_engine();

        b.iter(|| {
            engine.sync(&cx, root).expect("layout sync succeeds");
            black_box(engine.mirrored_node_count());
        });
    });
}

fn bench_gc_after_remove_100(c: &mut Criterion) {
    c.bench_function("gc_after_remove_100", |b| {
        b.iter_batched(
            || {
                let mut cx = Cx::new_root();
                let root = cx.add(nested_subtree_with_text_children(100));
                attach_root_intent(
                    &mut cx,
                    root,
                    LayoutIntent::flex_column().fixed_size(800.0, 1200.0),
                );
                let remove_root = cx
                    .get(root)
                    .and_then(|node| node.children.first())
                    .copied()
                    .expect("nested subtree exists");
                let mut engine = stub_engine();
                engine.sync(&cx, root).expect("initial sync succeeds");
                (cx, engine, remove_root)
            },
            |(mut cx, mut engine, remove_root)| {
                assert!(cx.remove(remove_root));
                engine.gc(&cx).expect("gc succeeds");
                black_box(engine.mirrored_node_count());
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_compute_single_node,
    bench_compute_flex_row_100_children,
    bench_compute_tree_100,
    bench_sync_tree_1000_nodes,
    bench_gc_after_remove_100,
);
criterion_main!(benches);
