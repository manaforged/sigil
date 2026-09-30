#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use sigil::kernel::{Cx, text};
use sigil::reactive::Runtime;
use sigil::widgets::{Identifiable, TableColumn, TableState, table};
use std::collections::HashSet;
use std::hint::black_box;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
struct Row {
    label: String,
    value: i64,
}

impl Identifiable for Row {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.value.to_string()
    }
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
            .sortable(|left: &Row, right: &Row| left.label.cmp(&right.label))
            .filterable(|row: &Row, query: &str| row.label.contains(query))
            .cell(|row: &Row| text(row.label.clone())),
        TableColumn::new("value", "Value")
            .sort_by(|row: &Row| row.value)
            .cell(|row: &Row| text(row.value.to_string())),
    ]
}

fn bench_table_ops(c: &mut Criterion) {
    c.bench_function("table_sort_toggle_100k", |b| {
        Runtime::scope("bench-table-sort", || {
            let state = TableState::new(rows(100_000), columns());
            b.iter(|| {
                state.toggle_sort("value");
                black_box(state.visible().get().len());
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
                black_box(state.visible().get().len());
            });
        });
    });
}

fn bench_replace(c: &mut Criterion) {
    for selected in [false, true] {
        let name = if selected {
            "table_replace_selected_10k"
        } else {
            "table_replace_unselected_10k"
        };
        c.bench_function(name, |b| {
            Runtime::scope("bench-table-replace", || {
                let rows = rows(10_000);
                let state = TableState::identified(rows.clone(), columns());
                if selected {
                    state.select_only(0);
                }
                b.iter_batched(
                    || rows.clone(),
                    |rows| state.replace_rows(rows),
                    BatchSize::SmallInput,
                );
            });
        });
    }
}

fn bench_shared_replace(c: &mut Criterion) {
    for selected in [0, 1, 10, 10_000] {
        let name = match selected {
            0 => "table_replace_shared_unselected_10k".to_owned(),
            1 => "table_replace_shared_selected_10k".to_owned(),
            _ => format!("table_replace_shared_selected_{selected}_of_10k"),
        };
        c.bench_function(&name, |b| {
            Runtime::scope("bench-table-shared", || {
                let first = Arc::new(rows(10_000));
                let mut second = (*first).clone();
                if let Some(row) = second.first_mut() {
                    row.label = "updated".into();
                }
                let second = Arc::new(second);
                let state = TableState::identified((*first).clone(), columns());
                if selected > 0 {
                    state.select_only(0);
                    state.selection().set(Rc::new((0..selected).collect()));
                }
                let mut flip = false;
                b.iter(|| {
                    flip = !flip;
                    let next = if flip { &first } else { &second };
                    state.replace_rows(Arc::clone(next));
                });
            });
        });
    }
}

fn replacement_rows(count: u32, case: &str) -> (Arc<Vec<Row>>, Arc<Vec<Row>>) {
    let mut first = rows(count);
    if case == "shuffled" {
        for row in &mut first {
            row.label = format!("row-{}", (row.value >> 16) & 7);
        }
    }
    let mut second = first.clone();
    let first_row = second.first_mut().expect("nonempty benchmark rows");
    match case {
        "changed-key" => first_row.label = "updated".into(),
        "ties" => first_row.label = "row-1".into(),
        "reordered" => second.rotate_left(17),
        "reversed" => second.reverse(),
        "shorter" => drop(second.pop()),
        "shuffled" => {
            for row in &mut second {
                row.label = format!("row-{}", ((row.value >> 8) ^ (row.value >> 24)) & 7);
            }
        }
        _ => first_row.value = -1,
    }
    (Arc::new(first), Arc::new(second))
}

fn bench_sorted_replace(c: &mut Criterion) {
    for count in [10_000, 100_000] {
        for case in [
            "stable",
            "changed-key",
            "ties",
            "reordered",
            "reversed",
            "shorter",
            "filtered",
            "shuffled",
        ] {
            c.bench_function(&format!("table_sorted_replace/{case}/{count}"), |b| {
                Runtime::scope("bench-table-sorted", || {
                    let (first, second) = replacement_rows(count, case);
                    let state = TableState::new((*first).clone(), columns());
                    state.toggle_sort("label");
                    if case == "filtered" {
                        state.quick_filter().set("row-4".into());
                    }
                    let mut flip = false;
                    b.iter(|| {
                        flip = !flip;
                        let next = if flip { &first } else { &second };
                        state.replace_rows(Arc::clone(next));
                        black_box(state.visible().peek());
                    });
                });
            });
        }
    }
}

fn bench_selection(c: &mut Criterion) {
    for count in [100, 10_000, 100_000] {
        for case in ["none", "first", "last", "half", "full", "hidden", "no-rows"] {
            c.bench_function(&format!("table_selection/{case}/{count}"), |b| {
                Runtime::scope("bench-table-selection", || {
                    let state = TableState::new(rows(count), columns());
                    let selected = Rc::new(match case {
                        "none" => HashSet::new(),
                        "last" => HashSet::from([count - 1]),
                        "half" => (0..count / 2).collect(),
                        "full" => (0..count).collect(),
                        _ => HashSet::from([0]),
                    });
                    state.selection().set(Rc::clone(&selected));
                    match case {
                        "hidden" => state.quick_filter().set("row-4".into()),
                        "no-rows" => state.quick_filter().set("missing".into()),
                        _ => {}
                    }
                    let mut cx = Cx::new_root();
                    let root = cx.add(
                        table(state.clone())
                            .selectable_checkboxes()
                            .viewport_height(200.0),
                    );
                    cx.flush_reactive_children();
                    b.iter(|| {
                        state
                            .selection()
                            .update(|value| *value = Rc::clone(&selected));
                        black_box(cx.get(root));
                    });
                });
            });
        }
    }
}

fn bench_mounted_replace(c: &mut Criterion) {
    for checkboxes in [false, true] {
        for selected in [0, 1, 100] {
            let name = format!("table_mounted_replace/checkboxes-{checkboxes}/selected-{selected}");
            let mut reported = false;
            c.bench_function(&name, |b| {
                Runtime::scope("bench-table-mounted", || {
                    let first = Arc::new(rows(100));
                    let mut second = (*first).clone();
                    if let Some(row) = second.first_mut() {
                        row.label.push_str("-updated");
                    }
                    let second = Arc::new(second);
                    let state = TableState::new((*first).clone(), columns());
                    state.selection().set(Rc::new((0..selected).collect()));
                    let mut view = table(state.clone()).viewport_height(720.0);
                    if checkboxes {
                        view = view.selectable_checkboxes();
                    }
                    let mut cx = Cx::new_root();
                    let root = cx.add(view);
                    cx.flush_reactive_children();
                    let mut flip = false;
                    let mut update = || {
                        flip = !flip;
                        state.replace_rows(Arc::clone(if flip { &second } else { &first }));
                        cx.flush_reactive_children();
                        black_box(cx.get(root));
                    };
                    if !reported {
                        let allocations = allocation_counter::measure(&mut update);
                        eprintln!(
                            "{name}: allocations={} bytes={}",
                            allocations.count_total, allocations.bytes_total
                        );
                        reported = true;
                    }
                    b.iter(update);
                });
            });
        }
    }
}

criterion_group!(
    benches,
    bench_table_ops,
    bench_replace,
    bench_shared_replace,
    bench_sorted_replace,
    bench_selection,
    bench_mounted_replace
);
criterion_main!(benches);
