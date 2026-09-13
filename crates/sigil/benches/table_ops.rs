#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::reactive::Runtime;
use sigil::widgets::{TableColumn, TableState};

#[derive(Clone)]
struct Row {
    label: String,
    value: i64,
}

fn rows(count: u32) -> Vec<Row> {
    (0..count)
        .map(|i| Row {
            label: format!("row-{}", i % 4096),
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
