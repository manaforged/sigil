//! Virtual list — GPUI `uniform_list` parity bench.
//!
//! This bench drives the real `virtual_list(items, render)` widget API.
//! The 100,000-row data source is built in `iter_batched` setup or kept
//! warm outside the measured scroll loop; the timed work is per-frame
//! widget tree construction into a fresh `Cx` plus one layout pass.

#![allow(
    missing_docs,
    dead_code,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::trivially_copy_pass_by_ref
)]

use arboard as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use sigil::widgets::virtual_list;
use sigil_hal::{LogicalSize, Size};
use sigil_kernel::{Cx, NodeBuilder, NodeId, TextContent, container};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};
use sigil_reactive::signal;

const TOTAL_ROWS: usize = 100_000;
const ROW_HEIGHT: f32 = 32.0;
const VIEWPORT_HEIGHT: f32 = 1_280.0;
const VIEWPORT_WIDTH: f32 = 1_280.0;
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

#[derive(Clone, Copy)]
struct RowData {
    id: u32,
}

fn make_data_source() -> Vec<RowData> {
    (0..TOTAL_ROWS).map(|i| RowData { id: i as u32 }).collect()
}

fn row_node(_index: usize, data: &RowData) -> NodeBuilder {
    black_box(data.id);
    container()
        .layout(LayoutStyle::flex_row().fixed_size(ROW_WIDTH, ROW_HEIGHT))
        .child(container().layout(LayoutStyle::block().fixed_size(ID_CELL_W, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(LABEL_CELL_W, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(ACTION_CELL_W, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(SPACER_CELL_W, ROW_HEIGHT)))
}

fn build_widget(cx: &mut Cx, data: Vec<RowData>, scroll_y: sigil_reactive::Signal<f32>) -> NodeId {
    cx.add(
        virtual_list(data, row_node)
            .row_height(ROW_HEIGHT)
            .viewport_height(VIEWPORT_HEIGHT)
            .viewport_width(VIEWPORT_WIDTH)
            .scroll_bind(scroll_y),
    )
}

fn bench_first_render(c: &mut Criterion) {
    sigil_reactive::Runtime::scope("sigil-virtual-list-bench-first-render", || {
        c.bench_function("virtual_list/first_render", |b| {
            b.iter_batched(
                || {
                    let data = make_data_source();
                    let scroll_y = signal(0.0_f32);
                    let cx = Cx::new_root();
                    let engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
                    (data, scroll_y, cx, engine)
                },
                |(data, scroll_y, mut cx, mut engine)| {
                    let root = build_widget(&mut cx, data, scroll_y);
                    engine.compute(&mut cx, root, AVAILABLE).unwrap();
                    black_box(engine.result(root));
                },
                BatchSize::SmallInput,
            );
        });
    });
}

fn bench_scroll_step(c: &mut Criterion) {
    sigil_reactive::Runtime::scope("sigil-virtual-list-bench-scroll-step", || {
        c.bench_function("virtual_list/scroll_step", |b| {
            let data = make_data_source();
            let scroll_y = signal(0.0_f32);
            let max_scroll = (TOTAL_ROWS as f32).mul_add(ROW_HEIGHT, -VIEWPORT_HEIGHT);
            b.iter_batched(
                || {
                    let data = data.clone();
                    let cx = Cx::new_root();
                    let engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
                    (data, cx, engine)
                },
                |(data, mut cx, mut engine)| {
                    scroll_y.update(|value| {
                        *value += ROW_HEIGHT;
                        if *value > max_scroll {
                            *value = 0.0;
                        }
                    });
                    let root = build_widget(&mut cx, data, scroll_y);
                    engine.compute(&mut cx, root, AVAILABLE).unwrap();
                    black_box(engine.result(root));
                },
                BatchSize::SmallInput,
            );
        });
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
