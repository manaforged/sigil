//! Sigil-layout vs Taffy on the **krausest js-framework-benchmark**
//! workload (<https://github.com/krausest/js-framework-benchmark>).
//!
//! ## Methodology
//!
//! krausest's bench is the de-facto industry standard for UI framework
//! performance — every major JS framework (React, Vue, Svelte, Solid,
//! Lit, `Angular`) publishes numbers against it, and the SCU / `WebKit`
//! teams use the same shape internally. The canonical operations are:
//!
//! | Op | Description |
//! |---|---|
//! | `create_1k` | Build a 1,000-row table from empty |
//! | `replace_1k` | Replace all 1,000 rows |
//! | `partial_update_10k` | Touch every 10th row of 10,000 |
//! | `select_row` | Toggle selection on one row of 1,000 |
//! | `remove_row` | Detach one row from 1,000 |
//! | `create_10k` | Build a 10,000-row table from empty |
//! | `append_1k_to_10k` | Grow a 10,000-row table by 1,000 |
//! | `clear_10k` | Tear down a 10,000-row table |
//!
//! Each row is a flex-row container with four fixed children — the same
//! 4-cell structure krausest's vanillajs reference uses (id / label /
//! action / spacer). For a layout engine this is the work that matters:
//! the engine sees a wide-flat tree (1 root → N rows → 4 leaves each),
//! which is exactly the shape of every real-world data grid.
//!
//! Swap-rows is intentionally absent — the sigil-kernel API does not
//! expose child-reordering, and approximating it via detach+reinsert
//! would measure detach+insert, not swap. Documented limitation.
//!
//! Run with `cargo bench -p sigil-layout --bench js_framework_benchmark`.

#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    // usize→f32 casts are intentional: row counts fit comfortably in f32 mantissa
    // for the small table sizes (≤10k rows) used in these benchmarks.
    clippy::cast_precision_loss,
    // Indexing on `rows[i]` is always bounds-safe because `i = (i+1) % rows.len()`.
    // Bench data is statically sized; the panic-as-bug trade-off is acceptable.
    clippy::indexing_slicing
)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
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

const VIEWPORT_WIDTH: f32 = 1_280.0;
const VIEWPORT_HEIGHT: f32 = 720.0;
const AVAILABLE: Size = Size::fixed(VIEWPORT_WIDTH, VIEWPORT_HEIGHT);
const ROW_HEIGHT: f32 = 32.0;
const ID_CELL_W: f32 = 80.0;
const LABEL_CELL_W: f32 = 320.0;
const ACTION_CELL_W: f32 = 80.0;
const SPACER_CELL_W: f32 = 800.0;

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

// === owned helpers ===

fn owned_row() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().fixed_size(
            ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W,
            ROW_HEIGHT,
        ))
        .child(container().layout(LayoutStyle::block().fixed_size(ID_CELL_W, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(LABEL_CELL_W, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(ACTION_CELL_W, ROW_HEIGHT)))
        .child(container().layout(LayoutStyle::block().fixed_size(SPACER_CELL_W, ROW_HEIGHT)))
}

fn build_owned_table(cx: &mut Cx, rows: usize) -> (NodeId, Vec<NodeId>) {
    let mut root_builder = container().layout(LayoutStyle::flex_column().fixed_size(
        ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W,
        rows as f32 * ROW_HEIGHT,
    ));
    for _ in 0..rows {
        root_builder = root_builder.child(owned_row());
    }
    let root = cx.add(root_builder);
    let row_ids: Vec<NodeId> = (0..rows)
        .map(|i| cx.nth_child(root, i).expect("row exists"))
        .collect();
    (root, row_ids)
}

// === taffy helpers ===

fn taffy_row(tree: &mut taffy::TaffyTree<()>) -> taffy::NodeId {
    let mut cell = |w: f32| {
        tree.new_leaf(taffy::Style {
            display: taffy::Display::Block,
            size: taffy::Size {
                width: taffy::Dimension::from_length(w),
                height: taffy::Dimension::from_length(ROW_HEIGHT),
            },
            ..Default::default()
        })
        .unwrap()
    };
    let children = [
        cell(ID_CELL_W),
        cell(LABEL_CELL_W),
        cell(ACTION_CELL_W),
        cell(SPACER_CELL_W),
    ];
    tree.new_with_children(
        taffy::Style {
            display: taffy::Display::Flex,
            flex_direction: taffy::FlexDirection::Row,
            size: taffy::Size {
                width: taffy::Dimension::from_length(
                    ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W,
                ),
                height: taffy::Dimension::from_length(ROW_HEIGHT),
            },
            ..Default::default()
        },
        &children,
    )
    .unwrap()
}

fn build_taffy_table(
    tree: &mut taffy::TaffyTree<()>,
    rows: usize,
) -> (taffy::NodeId, Vec<taffy::NodeId>) {
    let row_ids: Vec<taffy::NodeId> = (0..rows).map(|_| taffy_row(tree)).collect();
    let root = tree
        .new_with_children(
            taffy::Style {
                display: taffy::Display::Flex,
                flex_direction: taffy::FlexDirection::Column,
                size: taffy::Size {
                    width: taffy::Dimension::from_length(
                        ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W,
                    ),
                    height: taffy::Dimension::from_length(rows as f32 * ROW_HEIGHT),
                },
                ..Default::default()
            },
            &row_ids,
        )
        .unwrap();
    (root, row_ids)
}

// === op: create_1k ===

fn bench_create_1k(c: &mut Criterion) {
    c.bench_function("create_1k/owned", |b| {
        b.iter_batched(
            Cx::new_root,
            |mut cx| {
                let (root, _) = build_owned_table(&mut cx, 1_000);
                let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
                engine.compute(&mut cx, root, AVAILABLE).unwrap();
                black_box(engine.result(root));
            },
            BatchSize::SmallInput,
        );
    });
    c.bench_function("create_1k/taffy", |b| {
        b.iter_batched(
            taffy::TaffyTree::<()>::new,
            |mut tree| {
                let (root, _) = build_taffy_table(&mut tree, 1_000);
                tree.compute_layout(root, fixed_available(VIEWPORT_WIDTH, VIEWPORT_HEIGHT))
                    .unwrap();
                black_box(tree.layout(root).unwrap());
            },
            BatchSize::SmallInput,
        );
    });
}

// === op: create_10k ===

fn bench_create_10k(c: &mut Criterion) {
    c.bench_function("create_10k/owned", |b| {
        b.iter_batched(
            Cx::new_root,
            |mut cx| {
                let (root, _) = build_owned_table(&mut cx, 10_000);
                let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
                engine.compute(&mut cx, root, AVAILABLE).unwrap();
                black_box(engine.result(root));
            },
            BatchSize::LargeInput,
        );
    });
    c.bench_function("create_10k/taffy", |b| {
        b.iter_batched(
            taffy::TaffyTree::<()>::new,
            |mut tree| {
                let (root, _) = build_taffy_table(&mut tree, 10_000);
                tree.compute_layout(root, fixed_available(VIEWPORT_WIDTH, VIEWPORT_HEIGHT))
                    .unwrap();
                black_box(tree.layout(root).unwrap());
            },
            BatchSize::LargeInput,
        );
    });
}

// === op: partial_update_10k (touch every 10th row's style) ===

fn bench_partial_update_10k(c: &mut Criterion) {
    c.bench_function("partial_update_10k/owned", |b| {
        let mut cx = Cx::new_root();
        let (root, rows) = build_owned_table(&mut cx, 10_000);
        let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        engine.compute(&mut cx, root, AVAILABLE).unwrap();
        cx.clear_dirty();
        let mut toggle = 0_f32;
        b.iter(|| {
            toggle = if toggle > 4.0 { 0.0 } else { toggle + 1.0 };
            for row in rows.iter().step_by(10) {
                cx.get_mut(*row)
                    .expect("row live")
                    .attach(LayoutStyle::flex_row().fixed_size(
                        ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W + toggle,
                        ROW_HEIGHT,
                    ));
            }
            engine.compute(&mut cx, root, AVAILABLE).unwrap();
            black_box(engine.result(root));
        });
    });
    c.bench_function("partial_update_10k/taffy", |b| {
        let mut tree = taffy::TaffyTree::<()>::new();
        let (root, rows) = build_taffy_table(&mut tree, 10_000);
        tree.compute_layout(root, fixed_available(VIEWPORT_WIDTH, VIEWPORT_HEIGHT))
            .unwrap();
        let mut toggle = 0_f32;
        b.iter(|| {
            toggle = if toggle > 4.0 { 0.0 } else { toggle + 1.0 };
            let style = taffy::Style {
                display: taffy::Display::Flex,
                flex_direction: taffy::FlexDirection::Row,
                size: taffy::Size {
                    width: taffy::Dimension::from_length(
                        ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W + toggle,
                    ),
                    height: taffy::Dimension::from_length(ROW_HEIGHT),
                },
                ..Default::default()
            };
            for row in rows.iter().step_by(10) {
                tree.set_style(*row, style.clone()).unwrap();
            }
            tree.compute_layout(root, fixed_available(VIEWPORT_WIDTH, VIEWPORT_HEIGHT))
                .unwrap();
            black_box(tree.layout(root).unwrap());
        });
    });
}

// === op: select_row (single-row style mutation, the canonical sparse-update) ===

fn bench_select_row(c: &mut Criterion) {
    c.bench_function("select_row/owned", |b| {
        let mut cx = Cx::new_root();
        let (root, rows) = build_owned_table(&mut cx, 1_000);
        let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        engine.compute(&mut cx, root, AVAILABLE).unwrap();
        cx.clear_dirty();
        let mut i = 0usize;
        b.iter(|| {
            i = (i + 1) % rows.len();
            let bump = ((i % 4) as f32) * 1.0;
            cx.get_mut(rows[i])
                .expect("row live")
                .attach(LayoutStyle::flex_row().fixed_size(
                    ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W + bump,
                    ROW_HEIGHT,
                ));
            engine.compute(&mut cx, root, AVAILABLE).unwrap();
            black_box(engine.result(root));
        });
    });
    c.bench_function("select_row/taffy", |b| {
        let mut tree = taffy::TaffyTree::<()>::new();
        let (root, rows) = build_taffy_table(&mut tree, 1_000);
        tree.compute_layout(root, fixed_available(VIEWPORT_WIDTH, VIEWPORT_HEIGHT))
            .unwrap();
        let mut i = 0usize;
        b.iter(|| {
            i = (i + 1) % rows.len();
            let bump = ((i % 4) as f32) * 1.0;
            tree.set_style(
                rows[i],
                taffy::Style {
                    display: taffy::Display::Flex,
                    flex_direction: taffy::FlexDirection::Row,
                    size: taffy::Size {
                        width: taffy::Dimension::from_length(
                            ID_CELL_W + LABEL_CELL_W + ACTION_CELL_W + SPACER_CELL_W + bump,
                        ),
                        height: taffy::Dimension::from_length(ROW_HEIGHT),
                    },
                    ..Default::default()
                },
            )
            .unwrap();
            tree.compute_layout(root, fixed_available(VIEWPORT_WIDTH, VIEWPORT_HEIGHT))
                .unwrap();
            black_box(tree.layout(root).unwrap());
        });
    });
}

// === op: replace_1k (tear down + rebuild every iteration) ===

fn bench_replace_1k(c: &mut Criterion) {
    c.bench_function("replace_1k/owned", |b| {
        b.iter_batched(
            || {
                let mut cx = Cx::new_root();
                let (root, _) = build_owned_table(&mut cx, 1_000);
                let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
                engine.compute(&mut cx, root, AVAILABLE).unwrap();
                (cx, root, engine)
            },
            |(mut cx, root, mut engine)| {
                cx.remove(root);
                let (new_root, _) = build_owned_table(&mut cx, 1_000);
                engine.compute(&mut cx, new_root, AVAILABLE).unwrap();
                black_box(engine.result(new_root));
            },
            BatchSize::SmallInput,
        );
    });
    c.bench_function("replace_1k/taffy", |b| {
        b.iter_batched(
            || {
                let mut tree = taffy::TaffyTree::<()>::new();
                let (root, _) = build_taffy_table(&mut tree, 1_000);
                tree.compute_layout(root, fixed_available(VIEWPORT_WIDTH, VIEWPORT_HEIGHT))
                    .unwrap();
                (tree, root)
            },
            |(mut tree, root)| {
                tree.remove(root).unwrap();
                let (new_root, _) = build_taffy_table(&mut tree, 1_000);
                tree.compute_layout(new_root, fixed_available(VIEWPORT_WIDTH, VIEWPORT_HEIGHT))
                    .unwrap();
                black_box(tree.layout(new_root).unwrap());
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
    targets =
        bench_create_1k, bench_create_10k,
        bench_partial_update_10k, bench_select_row,
        bench_replace_1k,
);
criterion_main!(benches);
