//! One-shot perf breakdown for the 100k-node flex tree compute path.
//!
//! Phase-level timing for `LayoutEngine::compute` on the same shape the
//! `bench_compute_100k` criterion bench uses. Designed for B (investigate)
//! in the foundation sprint perf gap finding — surfaces where the ~150ms
//! goes so the fix can be aimed.
//!
//! Run with: `cargo run --release -p sigil-layout --example perf_100k_breakdown`.

use std::time::Instant;

use sigil_hal::{LogicalSize, Size};
use sigil_kernel::{Cx, NodeBuilder, NodeId, TextContent, container};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, source: &TextContent, _space: Size) -> LogicalSize {
        let chars = match source {
            TextContent::Static(v) => v.chars().count() as f32,
            TextContent::Reactive(_) => 0.0,
        };
        LogicalSize::new(chars * 8.0, 16.0)
    }
}

const ROOT_WIDTH: f32 = 1280.0;
const ROOT_HEIGHT: f32 = 1000.0;
const ROWS: usize = 1000;
const LEAVES_PER_ROW: usize = 100;
const ROW_HEIGHT: f32 = 1.0;
const LEAF_WIDTH: f32 = 12.8;
const AVAILABLE: Size = Size::fixed(ROOT_WIDTH, ROOT_HEIGHT);

fn leaf() -> NodeBuilder {
    container().layout(LayoutStyle::block().fixed_size(LEAF_WIDTH, ROW_HEIGHT))
}

fn row() -> NodeBuilder {
    let mut row_builder =
        container().layout(LayoutStyle::flex_row().fixed_size(ROOT_WIDTH, ROW_HEIGHT));
    for _ in 0..LEAVES_PER_ROW {
        row_builder = row_builder.child(leaf());
    }
    row_builder
}

fn build_100k_tree() -> (Cx, NodeId) {
    let mut cx = Cx::new_root();
    let mut root_builder =
        container().layout(LayoutStyle::flex_column().fixed_size(ROOT_WIDTH, ROOT_HEIGHT));
    for _ in 0..ROWS {
        root_builder = root_builder.child(row());
    }
    let root = cx.add(root_builder);
    (cx, root)
}

fn main() {
    eprintln!("=== build 100k-node flex tree ===");
    let t0 = Instant::now();
    let (mut cx, root) = build_100k_tree();
    eprintln!(
        "  kernel tree built: {} ms ({} nodes)\n",
        t0.elapsed().as_secs_f64() * 1000.0,
        cx.len()
    );

    eprintln!("=== first compute (cold engine) ===");
    let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    let t0 = Instant::now();
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("first compute");
    eprintln!(
        "  total first compute:                       {:>10.3} ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );
    eprintln!("  mirrored node count: {}\n", engine.mirrored_node_count());

    eprintln!("=== second compute (cached, no mutation) ===");
    let t0 = Instant::now();
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("cached compute");
    eprintln!(
        "  total cached compute:                      {:>10.3} ms\n",
        t0.elapsed().as_secs_f64() * 1000.0
    );

    eprintln!("=== third compute (mark_mutated → full re-sync) ===");
    cx.mark_mutated();
    let t0 = Instant::now();
    engine
        .compute(&mut cx, root, AVAILABLE)
        .expect("mutated compute");
    eprintln!(
        "  total mutated compute:                     {:>10.3} ms\n",
        t0.elapsed().as_secs_f64() * 1000.0
    );

    eprintln!("=== fourth compute (steady-state mutated loop, 5 iterations) ===");
    let mut total = 0.0;
    for i in 0..5 {
        cx.mark_mutated();
        let t0 = Instant::now();
        engine
            .compute(&mut cx, root, AVAILABLE)
            .expect("steady mutated compute");
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        eprintln!("  iter {i}: {ms:>10.3} ms");
        total += ms;
    }
    eprintln!(
        "  mean over 5 iters:                         {:>10.3} ms\n",
        total / 5.0
    );

    eprintln!("=== isolated sync only (mark_mutated → sync only, no compute_mirror) ===");
    // Hot-loop sync alone to see its cost in isolation. compute_mirror is the
    // other half; subtracting the two from total mutated compute tells us the
    // split.
    let mut total_sync = 0.0;
    for i in 0..5 {
        cx.mark_mutated();
        let t0 = Instant::now();
        engine.sync(&cx, root).expect("isolated sync");
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        eprintln!("  sync iter {i}: {ms:>10.3} ms");
        total_sync += ms;
    }
    eprintln!(
        "  mean sync alone:                           {:>10.3} ms\n",
        total_sync / 5.0
    );

    eprintln!("=== gc dry-run (after sync churn) ===");
    let t0 = Instant::now();
    engine.gc(&cx).expect("gc");
    eprintln!(
        "  gc:                                        {:>10.3} ms",
        t0.elapsed().as_secs_f64() * 1000.0
    );
}
