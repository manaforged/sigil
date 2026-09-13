#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic)]

use sigil::reactive::Runtime;
use sigil::widgets::{Identifiable, MIN_COL_WIDTH, SortDir, SortKey, TableColumn, TableState};

#[derive(Clone)]
struct Item {
    name: &'static str,
    count: i64,
}

impl Identifiable for Item {
    type Id = &'static str;

    fn id(&self) -> Self::Id {
        self.name
    }
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
            .sort_by(|row: &Item| row.name)
            .filterable(|row: &Item, query: &str| row.name.contains(query)),
        TableColumn::new("count", "Count").sort_by(|row: &Item| row.count),
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
            state.sort_key().get(),
            Some(SortKey {
                column: "name",
                dir: SortDir::Ascending,
            })
        );
        assert_eq!(visible_names(&state), ["alpha", "beta", "delta", "gamma"]);
        state.toggle_sort("name");
        assert_eq!(visible_names(&state), ["gamma", "delta", "beta", "alpha"]);
        state.toggle_sort("name");
        assert_eq!(state.sort_key().get(), None);
        assert_eq!(visible_names(&state), ["gamma", "alpha", "delta", "beta"]);
    });
}

#[test]
fn sort_is_stable_on_ties() {
    Runtime::scope("table-sort-stable", || {
        let state = TableState::new(items(), columns());
        state.toggle_sort("count");
        assert_eq!(visible_names(&state), ["alpha", "beta", "gamma", "delta"]);
    });
}

#[test]
fn toggle_sort_on_unsortable_column_is_ignored() {
    Runtime::scope("table-sort-unsortable", || {
        let state = TableState::new(items(), columns());
        state.toggle_sort("notes");
        assert_eq!(state.sort_key().get(), None);
    });
}

#[test]
fn quick_filter_matches_any_filterable_column_and_composes_with_sort() {
    Runtime::scope("table-filter", || {
        let state = TableState::new(items(), columns());
        state.quick_filter().set("a".into());
        state.toggle_sort("name");
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
        state.toggle_sort("name");
        state.select_only(1);
        assert!(state.selection().get().contains(&1));
        state.select_range_to(2);
        let selected = state.selection().get();
        assert_eq!(selected.len(), 3);
        assert!(selected.contains(&1) && selected.contains(&3) && selected.contains(&2));
        state.toggle_selected(3);
        assert!(!state.selection().get().contains(&3));
        state.clear_selection();
        assert!(state.selection().get().is_empty());
    });
}

#[test]
fn set_rows_keeps_in_range_selection_and_recomputes_visible() {
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
        assert_eq!(state.selected_index(), Some(0));
        assert_eq!(state.selected_row().map(|row| row.name), Some("omega"));
        assert_eq!(visible_names(&state), ["iota", "omega"]);
    });
}

#[test]
fn replace_rows_with_key_keeps_the_named_row() {
    Runtime::scope("table-replace-key", || {
        let state = TableState::identified(items(), columns());
        state.select_only(0);
        assert_eq!(state.selected_row().map(|row| row.name), Some("gamma"));
        state.replace_rows(vec![
            Item {
                name: "omega",
                count: 1,
            },
            Item {
                name: "gamma",
                count: 9,
            },
        ]);
        assert_eq!(state.selected_index(), Some(1));
        assert_eq!(state.selected_row().map(|row| row.name), Some("gamma"));
    });
}

#[test]
fn set_rows_drops_indices_past_the_new_len() {
    Runtime::scope("table-set-rows-prune", || {
        let state = TableState::new(items(), columns());
        state.select_only(3);
        state.replace_rows(vec![Item {
            name: "only",
            count: 1,
        }]);
        assert!(state.selection().get().is_empty());
        assert_eq!(state.selected_index(), None);
    });
}

#[test]
fn source_order_reuses_identity_list_when_len_holds() {
    use std::rc::Rc;

    Runtime::scope("table-identity-reuse", || {
        let state = TableState::new(items(), columns());
        let first = state.visible().peek();
        state.replace_rows(vec![
            Item {
                name: "w",
                count: 1,
            },
            Item {
                name: "x",
                count: 2,
            },
            Item {
                name: "y",
                count: 3,
            },
            Item {
                name: "z",
                count: 4,
            },
        ]);
        let same_len = state.visible().peek();
        assert!(
            Rc::ptr_eq(&first, &same_len),
            "source order keeps the 0..n list when the row count is unchanged"
        );

        state.toggle_sort("name");
        let sorted = state.visible().peek();
        assert!(!Rc::ptr_eq(&first, &sorted), "sort allocates a permutation");
        assert_eq!(visible_names(&state), ["w", "x", "y", "z"]);

        state.toggle_sort("name");
        state.toggle_sort("name");
        let restored = state.visible().peek();
        assert!(
            Rc::ptr_eq(&first, &restored),
            "clearing sort returns the cached source-order list"
        );

        state.replace_rows(vec![Item {
            name: "only",
            count: 0,
        }]);
        let shorter = state.visible().peek();
        assert_eq!(&*shorter, &[0]);
        assert!(!Rc::ptr_eq(&first, &shorter));
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
                    .sort_by(|row: &Item| row.count)
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

#[test]
fn move_column_permutes_visual_order() {
    Runtime::scope("table-move-column", || {
        let state = TableState::new(items(), columns());
        let ids = |s: &TableState<Item>| {
            s.visible_columns()
                .into_iter()
                .map(|(_, col)| col.id())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&state), ["name", "count", "notes"]);
        state.move_column(0, 2);
        assert_eq!(ids(&state), ["count", "notes", "name"]);
        state.move_column(2, 0);
        assert_eq!(ids(&state), ["name", "count", "notes"]);
        state.move_column(9, 0);
        assert_eq!(ids(&state), ["name", "count", "notes"]);
    });
}

#[test]
fn hide_column_keeps_last_visible() {
    Runtime::scope("table-hide-column", || {
        let state = TableState::new(items(), columns());
        state.hide_column("notes");
        let ids = state
            .visible_columns()
            .into_iter()
            .map(|(_, col)| col.id())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["name", "count"]);
        state.hide_column("count");
        state.hide_column("name");
        assert_eq!(state.visible_columns().len(), 1, "last column stays");
        state.show_all_columns();
        assert_eq!(state.visible_columns().len(), 3);
    });
}

#[test]
fn set_column_width_clamps_and_clears() {
    Runtime::scope("table-col-width", || {
        let state = TableState::new(items(), columns());
        state.pin_column_width("name", 12.0);
        assert_eq!(state.column_width("name"), Some(MIN_COL_WIDTH));
        state.pin_column_width("name", 180.0);
        assert_eq!(state.column_width("name"), Some(180.0));
        state.clear_column_width("name");
        assert_eq!(state.column_width("name"), None);
        state.reset_columns();
        assert_eq!(state.visible_columns().len(), 3);
    });
}
