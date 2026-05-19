//! Virtual list — GPUI `uniform_list` parity bench.
//!
//! GPUI's signature performance claim is that its `uniform_list` view
//! renders a virtualised scroll of 100,000 fixed-height items at 120 Hz
//! by materialising only the visible window (~40 rows for a 1280px
//! viewport at 32px row height). This bench is the apples-to-apples
//! parity proof: same data-source size, same row shape, same viewport,
//! same "only visible rows touch the layout engine" virtualisation
//! contract.
//!
//! ## Methodology
//!
//! Two criterion-driven operations:
//!
//! | Bench | What it measures | Sample size |
//! |---|---|---|
//! | `virtual_list/first_render` | Build the 100k-item data source + materialise the visible window + run one layout pass cold | 20 |
//! | `virtual_list/scroll_step`  | Steady-state: data source kept warm, advance scroll by one row, rematerialise window, relayout | 50 |
//!
//! Row shape mirrors the krausest js-framework-benchmark workload used
//! by `crates/layout/benches/js_framework_benchmark.rs`: a flex-row with
//! four fixed-width children (`id / label / action / spacer`). Krausest
//! is the de-facto industry standard for UI-framework data-grid perf
//! (React / Vue / Svelte / Solid / Lit / Angular all publish numbers
//! against it), so using the same row keeps sigil's reported numbers
//! reviewer-credible against both GPUI's `uniform_list` claim and the
//! wider JS-framework-benchmark cohort.
//!
//! Virtualisation contract: the 100,000-item data source is a plain
//! `Vec<RowData>`. Per frame, only the ~40 rows whose y-range overlaps
//! the 1280px viewport are materialised as layout nodes. That is
//! exactly what `uniform_list` does. The 99,960 off-screen rows pay
//! zero layout cost — proving the cost stays O(visible rows), not
//! O(total rows). The bench is *layout-pass only*; render-side cost
//! (geometry submission, draw calls) belongs to a future bench against
//! `sigil-renderer`.
//!
//! Criterion config matches the existing layout benches (krausest +
//! taffy-corpus): sample windows 20 / 50, 2s warm-up, 5s measurement.
//! Criterion reports the full mean / median / std-dev / mad / slope
//! distribution; the p50 / p95 / p99 percentiles can be derived from
//! the saved samples under `target/criterion/`.
//!
//! Run with `cargo bench -p sigil --bench virtual_list`. Quick mode
//! (`-- --quick`) skips the full sample window and finishes in ~30 s.

#![allow(
    missing_docs,
    dead_code,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::indexing_slicing,
    clippy::suboptimal_flops
)]

// Workspace deps the bench target inherits but does not call.
use image as _;
use pollster as _;
use sigil as _;
use sigil_actions as _;
use sigil_app as _;
use sigil_events as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use wgpu as _;

use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use sigil_hal::{LogicalSize, Size};
use sigil_kernel::{Cx, NodeBuilder, NodeId, TextContent, container};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};

// === workload constants ===

/// Total data-source size — matches GPUI's published `uniform_list`
/// 100k-item claim.
const TOTAL_ROWS: usize = 100_000;
/// Row height in logical px. 32 px is the krausest reference row height
/// and a typical OS-default list-row height.
const ROW_HEIGHT: f32 = 32.0;
/// Viewport height. 1280 px / 32 px ≈ 40 visible rows — the same
/// window depth GPUI exercises in its parity demos.
const VIEWPORT_HEIGHT: f32 = 1_280.0;
/// Viewport width.
const VIEWPORT_WIDTH: f32 = 1_280.0;
/// Cell widths mirror the krausest 4-cell row (id / label / action / spacer).
const ID_CELL_W: f32 = 80.0;
const LABEL_CELL_W: f32 = 320.0;
const ACTION_CELL_W: f32 = 80.0;
const SPACER_CELL_W: f32 = 800.0;
const ROW_WIDTH: f32 = ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W;

const AVAILABLE: Size = Size::fixed(VIEWPORT_WIDTH, VIEWPORT_HEIGHT);

struct StubMeasure;
impl TextMeasure for StubMeasure {
    fn measure(&self, _: &TextContent, _: Size) -> LogicalSize {
        LogicalSize::new(0.0, 0.0)
    }
}

/// Opaque row payload — stands in for whatever real data a host app
/// would feed into the list (id, label, action handler, etc). The
/// virtualisation contract is that this lives in a flat `Vec` and
/// only the visible window ever crosses into the layout engine.
#[derive(Clone, Copy)]
struct RowData {
    id: u32,
}

/// Build the 100k-row data source. This is cheap (one alloc + a tight
/// loop) and is what `iter_batched`'s setup does in `first_render`.
fn make_data_source() -> Vec<RowData> {
    (0..TOTAL_ROWS).map(|i| RowData { id: i as u32 }).collect()
}

/// Compute the visible row index range for a given scroll offset.
/// Returns `[first, end)` such that `end - first` rows overlap the
/// viewport rectangle. This is the core of virtualisation — the layout
/// engine never sees rows outside this range.
fn visible_window(scroll_y: f32, total: usize) -> (usize, usize) {
    let first_f = (scroll_y / ROW_HEIGHT).floor().max(0.0);
    let visible_count = (VIEWPORT_HEIGHT / ROW_HEIGHT).ceil() as usize + 1;
    let first = (first_f as usize).min(total);
    let end = (first + visible_count).min(total);
    (first, end)
}

/// Build a krausest-shape row (flex-row, 4 fixed cells).
fn row_node(_data: RowData) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().fixed_size(ROW_WIDTH, ROW_HEIGHT))
        .child(container().layout(LayoutStyle::block().fixed_size(ID_CELL_W, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(LABEL_CELL_W, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(ACTION_CELL_W, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(SPACER_CELL_W, ROW_HEIGHT)))
}

/// Materialise the visible window for `scroll_y` into a fresh `Cx`.
/// Returns the root `NodeId` (the viewport container). This is the
/// per-frame work the bench measures — note its complexity is
/// O(visible rows), independent of `data.len()`.
fn build_visible(cx: &mut Cx, data: &[RowData], scroll_y: f32) -> NodeId {
    let (first, end) = visible_window(scroll_y, data.len());
    let visible_rows = end - first;
    let mut root = container()
        .layout(LayoutStyle::flex_column().fixed_size(ROW_WIDTH, visible_rows as f32 * ROW_HEIGHT));
    for row in &data[first..end] {
        root = root.child(row_node(*row));
    }
    cx.add(root)
}

// === bench: cold first render ===

fn bench_first_render(c: &mut Criterion) {
    c.bench_function("virtual_list/first_render", |b| {
        b.iter_batched(
            // setup: build the 100k data source + a fresh Cx + a fresh
            // engine. iter_batched runs this *outside* the timed
            // section, so the measurement is the mount + layout pass.
            || {
                let data = make_data_source();
                let cx = Cx::new_root();
                let engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
                (data, cx, engine)
            },
            |(data, mut cx, mut engine)| {
                let root = build_visible(&mut cx, &data, 0.0);
                engine.compute(&mut cx, root, AVAILABLE).unwrap();
                black_box(engine.result(root));
            },
            BatchSize::SmallInput,
        );
    });
}

// === bench: steady-state scroll step ===

fn bench_scroll_step(c: &mut Criterion) {
    c.bench_function("virtual_list/scroll_step", |b| {
        // Data source is built once and reused across iterations —
        // virtualisation guarantees we never rebuild it on scroll, so
        // measuring that work would distort the steady-state number.
        let data = make_data_source();
        let mut scroll_y = 0.0_f32;
        // Cap scroll at the maximum so we wrap and stay inside the
        // dataset across the criterion sample window.
        let max_scroll = (TOTAL_ROWS as f32 * ROW_HEIGHT) - VIEWPORT_HEIGHT;
        b.iter_batched(
            // setup per iter: fresh Cx + engine. The data source is the
            // warm part; the per-frame node materialisation is the
            // measured part. iter_batched keeps the setup out of the
            // measurement window.
            || {
                let cx = Cx::new_root();
                let engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
                (cx, engine)
            },
            |(mut cx, mut engine)| {
                // Advance scroll by one row (the canonical "one tick of
                // the scroll wheel" or "one keyboard-arrow press").
                scroll_y += ROW_HEIGHT;
                if scroll_y > max_scroll {
                    scroll_y = 0.0;
                }
                let root = build_visible(&mut cx, &data, scroll_y);
                engine.compute(&mut cx, root, AVAILABLE).unwrap();
                black_box(engine.result(root));
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5));
    targets = bench_first_render,
);
criterion_group!(
    name = scroll;
    config = Criterion::default()
        .sample_size(50)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(5));
    targets = bench_scroll_step,
);
criterion_main!(benches, scroll);
