//! DR-057 D1 headless table core — logic contract, no node tree.

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::widgets::{SortDir, SortKey, TableColumn, TableState};
use sigil_reactive::Runtime;

#[derive(Clone)]
struct Item {
    name: &'static str,
    count: i64,
}

fn items() -> Vec<Item> {
    vec![
        Item {
            name: "gamma",
            count: 30,
        },
        Item {
            name: "alpha",
            count: 10,
        },
        Item {
            name: "delta",
            count: 30,
        },
        Item {
            name: "beta",
            count: 20,
        },
    ]
}

fn columns() -> Vec<TableColumn<Item>> {
    vec![
        TableColumn::new("name", "Name")
            .sortable(|a: &Item, b: &Item| a.name.cmp(b.name))
            .filterable(|row: &Item, query: &str| row.name.contains(query)),
        TableColumn::new("count", "Count").sortable(|a: &Item, b: &Item| a.count.cmp(&b.count)),
        TableColumn::new("notes", "Notes"),
    ]
}

fn visible_names(state: &TableState<Item>) -> Vec<&'static str> {
    let rows = state.rows().get();
    state
        .visible()
        .get()
        .iter()
        .map(|&index| rows.get(index as usize).unwrap().name)
        .collect()
}

#[test]
fn unsorted_unfiltered_shows_source_order() {
    Runtime::scope("table-source-order", || {
        let state = TableState::new(items(), columns());
        assert_eq!(visible_names(&state), ["gamma", "alpha", "delta", "beta"]);
    });
}

#[test]
fn toggle_sort_cycles_asc_desc_none() {
    Runtime::scope("table-sort-cycle", || {
        let state = TableState::new(items(), columns());
        state.toggle_sort("name");
        assert_eq!(
            state.sort().get(),
            Some(SortKey {
                column: "name",
                dir: SortDir::Ascending,
            })
        );
        assert_eq!(visible_names(&state), ["alpha", "beta", "delta", "gamma"]);
        state.toggle_sort("name");
        assert_eq!(visible_names(&state), ["gamma", "delta", "beta", "alpha"]);
        state.toggle_sort("name");
        assert_eq!(state.sort().get(), None);
        assert_eq!(visible_names(&state), ["gamma", "alpha", "delta", "beta"]);
    });
}

#[test]
fn sort_is_stable_on_ties() {
    Runtime::scope("table-sort-stable", || {
        let state = TableState::new(items(), columns());
        state.toggle_sort("count");
        // gamma and delta tie at 30; source order must hold.
        assert_eq!(visible_names(&state), ["alpha", "beta", "gamma", "delta"]);
    });
}

#[test]
fn toggle_sort_on_unsortable_column_is_ignored() {
    Runtime::scope("table-sort-unsortable", || {
        let state = TableState::new(items(), columns());
        state.toggle_sort("notes");
        assert_eq!(state.sort().get(), None);
    });
}

#[test]
fn quick_filter_matches_any_filterable_column_and_composes_with_sort() {
    Runtime::scope("table-filter", || {
        let state = TableState::new(items(), columns());
        state.quick_filter().set("a".into());
        state.toggle_sort("name");
        // Every name contains 'a' except none — all four match; sorted.
        assert_eq!(visible_names(&state), ["alpha", "beta", "delta", "gamma"]);
        state.quick_filter().set("elt".into());
        assert_eq!(visible_names(&state), ["delta"]);
        state.quick_filter().set(String::new());
        assert_eq!(state.visible().get().len(), 4);
    });
}

#[test]
fn selection_ops_follow_click_shapes() {
    Runtime::scope("table-selection", || {
        let state = TableState::new(items(), columns());
        state.toggle_sort("name"); // visible: alpha(1) beta(3) delta(2) gamma(0)
        state.select_only(1);
        assert!(state.selection().get().contains(&1));
        // Shift-select to delta: the span covers the VISIBLE run
        // alpha → beta → delta, i.e. source indices {1, 3, 2}.
        state.select_range_to(2);
        let selected = state.selection().get();
        assert_eq!(selected.len(), 3);
        assert!(selected.contains(&1) && selected.contains(&3) && selected.contains(&2));
        // Ctrl-toggle beta off.
        state.toggle_selected(3);
        assert!(!state.selection().get().contains(&3));
        state.clear_selection();
        assert!(state.selection().get().is_empty());
    });
}

#[test]
fn set_rows_clears_selection_and_recomputes_visible() {
    Runtime::scope("table-set-rows", || {
        let state = TableState::new(items(), columns());
        state.select_only(0);
        state.toggle_sort("name");
        state.replace_rows(vec![
            Item {
                name: "omega",
                count: 1,
            },
            Item {
                name: "iota",
                count: 2,
            },
        ]);
        assert!(state.selection().get().is_empty());
        assert_eq!(visible_names(&state), ["iota", "omega"]);
    });
}

#[test]
fn hundred_thousand_rows_sort_and_filter_correctly() {
    Runtime::scope("table-100k", || {
        let rows = (0..100_000_u32)
            .map(|i| Item {
                name: if i % 977 == 0 { "needle" } else { "hay" },
                count: i64::from(100_000 - i),
            })
            .collect::<Vec<_>>();
        let state = TableState::new(
            rows,
            vec![
                TableColumn::new("count", "Count")
                    .sortable(|a: &Item, b: &Item| a.count.cmp(&b.count))
                    .filterable(|row: &Item, query: &str| row.name.contains(query)),
            ],
        );
        state.toggle_sort("count");
        let visible = state.visible().get();
        assert_eq!(visible.len(), 100_000);
        assert_eq!(visible.first(), Some(&99_999));
        assert_eq!(visible.last(), Some(&0));

        state.quick_filter().set("needle".into());
        let filtered = state.visible().get();
        assert_eq!(filtered.len(), 100_000_usize.div_ceil(977));
    });
}
