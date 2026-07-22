//! Tier 1/2/4 layout operation benchmarks for `sigil-layout`.
//!
//! Run with `cargo bench -p sigil-layout`.

#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used
)]

use allocation_counter as _;

use std::hint::black_box;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use ahash as _;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use rayon::{ThreadPool, ThreadPoolBuilder};
use sigil_hal::{LogicalSize, Scale, Size};
use sigil_kernel::TextContent;
use sigil_kernel::{Cx, NodeBuilder, NodeId, container, text};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};
use sigil_reactive as _;
use sigil_style as _;
use slotmap as _;
use smallvec as _;
use taffy as _;

const ROOT_WIDTH: f32 = 1280.0;
const ROOT_HEIGHT: f32 = 1000.0;
const ROWS_100K: usize = 1000;
const LEAVES_PER_ROW_100K: usize = 100;
const ROW_HEIGHT_100K: f32 = 1.0;
const LEAF_WIDTH_100K: f32 = 12.8;
const AVAILABLE_100K: Size = Size::fixed(ROOT_WIDTH, ROOT_HEIGHT);

enum BenchCommand {
    Run,
    Stop,
}

struct StubMeasure;
impl TextMeasure for StubMeasure {
    #[allow(clippy::cast_precision_loss)]
    fn measure(&self, source: &TextContent, _space: Size) -> LogicalSize {
        let chars = match source {
            TextContent::Static(v) => v.chars().count() as f32,
            TextContent::Reactive(_) => 0.0,
        };
        LogicalSize::new(chars * 8.0, 16.0)
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

fn build_100k_flex_tree() -> (Cx, NodeId, LayoutEngine) {
    let mut cx = Cx::new_root();
    let root = build_100k_root(&mut cx);
    let engine = stub_engine();
    (cx, root, engine)
}

fn build_100k_root(cx: &mut Cx) -> NodeId {
    let mut root_builder =
        container().layout(LayoutStyle::flex_column().fixed_size(ROOT_WIDTH, ROOT_HEIGHT));
    for _ in 0..ROWS_100K {
        root_builder = root_builder.child(row_100k());
    }
    cx.add(root_builder)
}

fn first_leaf_under(cx: &Cx, root: NodeId) -> NodeId {
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let node = cx.get(id).expect("node exists");
        if node.children().is_empty() {
            return id;
        }
        stack.extend(node.children().iter().rev().copied());
    }
    unreachable!("root subtree contains at least one node")
}

fn row_100k() -> NodeBuilder {
    let mut row_builder =
        container().layout(LayoutStyle::flex_row().fixed_size(ROOT_WIDTH, ROW_HEIGHT_100K));
    for _ in 0..LEAVES_PER_ROW_100K {
        row_builder = row_builder.child(leaf_100k());
    }
    row_builder
}

fn leaf_100k() -> NodeBuilder {
    container().layout(LayoutStyle::block().fixed_size(LEAF_WIDTH_100K, ROW_HEIGHT_100K))
}

fn spawn_compute_worker(pool: &ThreadPool) -> (Sender<BenchCommand>, Receiver<()>) {
    let (command_tx, command_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::channel();

    pool.spawn(move || {
        let (mut cx, root, mut engine) = build_100k_flex_tree();
        engine
            .compute(&mut cx, root, AVAILABLE_100K)
            .expect("warm layout compute succeeds");
        ready_tx.send(()).expect("ready signal sends");

        while let Ok(command) = command_rx.recv() {
            match command {
                BenchCommand::Run => {
                    cx.mark_mutated();
                    engine
                        .compute(&mut cx, root, AVAILABLE_100K)
                        .expect("layout compute succeeds");
                    black_box(engine.result(root));
                    done_tx.send(()).expect("done signal sends");
                }
                BenchCommand::Stop => break,
            }
        }
    });

    ready_rx.recv().expect("worker starts");
    (command_tx, done_rx)
}

fn attach_root_intent(cx: &mut Cx, root: NodeId, intent: LayoutStyle) {
    let _ = cx.get_mut(root).expect("root exists").attach(intent);
}

fn bench_compute_single_node(c: &mut Criterion) {
    c.bench_function("compute_single_node", |b| {
        let mut cx = Cx::new_root();
        let root = cx.add(container());
        attach_root_intent(
            &mut cx,
            root,
            LayoutStyle::flex_row().fixed_size(100.0, 40.0),
        );
        let mut engine = stub_engine();

        b.iter(|| {
            cx.mark_mutated();
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
            LayoutStyle::flex_row().fixed_size(1200.0, 40.0),
        );
        let mut engine = stub_engine();

        b.iter(|| {
            cx.mark_mutated();
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
            LayoutStyle::flex_column().fixed_size(800.0, 1200.0),
        );
        let mut engine = stub_engine();

        b.iter(|| {
            cx.mark_mutated();
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
            LayoutStyle::flex_column().fixed_size(800.0, 12_000.0),
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
                    LayoutStyle::flex_column().fixed_size(800.0, 1200.0),
                );
                let remove_root = cx
                    .get(root)
                    .and_then(|node| node.children().first().copied())
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

fn bench_scale_change_reflow_1000_nodes(c: &mut Criterion) {
    c.bench_function("scale_change_reflow_1000_nodes", |b| {
        b.iter_batched(
            || {
                let mut cx = Cx::new_root();
                let root = cx.add(tree_with_text_children(1000));
                attach_root_intent(
                    &mut cx,
                    root,
                    LayoutStyle::flex_column().fixed_size(1280.0, 720.0),
                );
                let mut engine = stub_engine();
                engine
                    .compute(&mut cx, root, Size::fixed(1280.0, 720.0))
                    .expect("initial layout compute succeeds");
                (cx, root, engine)
            },
            |(mut cx, root, mut engine)| {
                cx.set_scale(Scale::new(2.0));
                engine
                    .compute(&mut cx, root, Size::fixed(1280.0, 720.0))
                    .expect("scale-change layout compute succeeds");
                black_box(engine.result(root));
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_incremental_single_leaf_mutation(c: &mut Criterion) {
    c.bench_function("bench_incremental_single_leaf_mutation_100k", |b| {
        b.iter_batched(
            || {
                let mut cx = Cx::new_root();
                let root = build_100k_root(&mut cx);
                let leaf = first_leaf_under(&cx, root);
                let mut engine = stub_engine();
                engine
                    .compute(&mut cx, root, AVAILABLE_100K)
                    .expect("warm layout compute succeeds");
                cx.clear_dirty();
                (cx, root, leaf, engine)
            },
            |(mut cx, root, leaf, mut engine)| {
                cx.get_mut(leaf).expect("leaf exists").attach(
                    LayoutStyle::block().fixed_size(LEAF_WIDTH_100K + 1.0, ROW_HEIGHT_100K),
                );
                engine
                    .compute(&mut cx, root, AVAILABLE_100K)
                    .expect("incremental compute succeeds");
                black_box(engine.result(leaf));
            },
            BatchSize::PerIteration,
        );
    });
}

// NOTE: `compute` is SERIAL. This runs one serial compute on a rayon
// worker pool of `threads` threads, so the `{threads}_threads` variants
// measure worker-dispatch overhead of running layout OFF the main thread
// — NOT parallel-layout scaling (the engine does not parallelize a single
// compute). Do not read the multi-thread numbers as a speedup.
fn bench_compute_100k_flex_tree(c: &mut Criterion, threads: usize) {
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("rayon thread pool builds");
    let bench_name = format!("bench_compute_100k/{threads}_threads");
    c.bench_function(&bench_name, |b| {
        let (command_tx, done_rx) = spawn_compute_worker(&pool);

        b.iter(|| {
            command_tx
                .send(BenchCommand::Run)
                .expect("run signal sends");
            done_rx.recv().expect("layout compute completes");
        });
        command_tx
            .send(BenchCommand::Stop)
            .expect("stop signal sends");
    });
}

fn bench_compute_100k_1_thread(c: &mut Criterion) {
    bench_compute_100k_flex_tree(c, 1);
}

fn bench_compute_100k_2_threads(c: &mut Criterion) {
    bench_compute_100k_flex_tree(c, 2);
}

fn bench_compute_100k_4_threads(c: &mut Criterion) {
    bench_compute_100k_flex_tree(c, 4);
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(2000)
        .warm_up_time(Duration::from_secs(2));
    targets =
        bench_compute_single_node,
        bench_compute_flex_row_100_children,
        bench_compute_tree_100,
        bench_sync_tree_1000_nodes,
        bench_gc_after_remove_100,
        bench_scale_change_reflow_1000_nodes,
        bench_incremental_single_leaf_mutation,
        bench_compute_100k_1_thread,
        bench_compute_100k_2_threads,
        bench_compute_100k_4_threads,
);
criterion_main!(benches);
