//! Table-core interaction budgets.
//!
//! Interaction-triggered operations (a sort click, a filter keystroke)
//! answer to the ~50 ms perceptible-lag threshold, not the 16.7 ms
//! frame budget — the derived recompute happens once per interaction,
//! off the paint path. Record medians when they move.

#![allow(missing_docs)]

use arboard as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_bidi as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_profile as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
#[cfg(feature = "tui")]
use sigil_tui as _;
use sigil_widgets as _;
use wgpu as _;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::widgets::{TableColumn, TableState};
use sigil_reactive::Runtime;

#[derive(Clone)]
struct Row {
    label: String,
    value: i64,
}

fn rows(count: u32) -> Vec<Row> {
    (0..count)
        .map(|i| Row {
            label: format!("row-{}", i % 4096),
            // Knuth-hash the index so the sort never hits a
            // pattern-defeating fast path — the budget must reflect a
            // genuinely unsorted click.
            value: i64::from(i.wrapping_mul(2_654_435_761)),
        })
        .collect()
}

fn columns() -> Vec<TableColumn<Row>> {
    vec![
        TableColumn::new("label", "Label")
            .sort_by(|row: &Row| row.label.clone())
            .filterable(|row: &Row, query: &str| row.label.contains(query)),
        TableColumn::new("value", "Value").sort_by(|row: &Row| row.value),
    ]
}

fn bench_table_ops(c: &mut Criterion) {
    c.bench_function("table_sort_toggle_100k", |b| {
        Runtime::scope("bench-table-sort", || {
            let state = TableState::new(rows(100_000), columns());
            b.iter(|| {
                state.toggle_sort("value");
                std::hint::black_box(state.visible().get().len());
            });
        });
    });
    c.bench_function("table_quick_filter_100k", |b| {
        Runtime::scope("bench-table-filter", || {
            let state = TableState::new(rows(100_000), columns());
            let mut flip = false;
            b.iter(|| {
                flip = !flip;
                state
                    .quick_filter()
                    .set(if flip { "row-40" } else { "row-1" }.into());
                std::hint::black_box(state.visible().get().len());
            });
        });
    });
}

criterion_group!(benches, bench_table_ops);
criterion_main!(benches);
