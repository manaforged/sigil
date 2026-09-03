#![allow(clippy::indexing_slicing, clippy::unwrap_used)]

use super::*;
use sigil::testing::with_cx;

#[test]
fn commit_then_undo_restores_price() {
    with_cx("ledger-edit", |_| {
        let app = App::new(20);
        let live = app.live;
        let before = live.price(4).unwrap();
        app.table.select_only(4);
        app.edit_selected();
        assert_eq!(live.editing.peek(), Some(4));
        live.commit("123.5");
        assert_eq!(live.price(4), Some(123.5));
        assert_eq!(live.editing.peek(), None);
        assert_eq!(live.undo.peek().len(), 1);
        live.undo();
        assert_eq!(live.price(4), Some(before));
        assert!(live.undo.peek().is_empty());
    });
}

#[test]
fn invalid_draft_is_not_committed() {
    with_cx("ledger-invalid", |_| {
        let live = App::new(20).live;
        let before = live.price(1).unwrap();
        live.edit(1);
        live.draft.set(String::from("-7"));
        assert!(live.invalid().peek());
        live.commit("-7");
        assert_eq!(live.price(1), Some(before));
        assert_eq!(live.editing.peek(), Some(1));
        live.cancel();
        assert_eq!(live.editing.peek(), None);
    });
}

#[test]
fn undo_stack_is_capped() {
    with_cx("ledger-undo-cap", |_| {
        let live = App::new(5).live;
        for i in 0..(UNDO_DEPTH + 10) {
            live.edit(0);
            live.commit(&format!("{i}"));
        }
        assert_eq!(live.undo.peek().len(), UNDO_DEPTH);
    });
}

#[test]
fn tick_counts_issued_updates_without_touching_rows() {
    with_cx("ledger-tick", |_| {
        let app = App::new(400);
        let rows = app.table.rows().peek();
        app.live.tick();
        app.live.tick();
        assert_eq!(app.live.ticks.peek(), 2);
        assert_eq!(app.live.moved.peek(), 20);
        assert!(Rc::ptr_eq(&rows, &app.table.rows().peek()));
    });
}

#[test]
fn filter_and_sort_go_through_the_table() {
    with_cx("ledger-table", |_| {
        let app = App::new(200);
        let symbol = app.table.rows().peek()[7].quote.symbol.clone();
        app.table.quick_filter().set(symbol);
        assert!(app.table.visible().peek().contains(&7));
        assert!(app.table.visible().peek().len() < 200);
        app.table.quick_filter().set(String::new());
        app.table.toggle_sort("last_done");
        let visible = app.table.visible().peek();
        let first = app.live.price(visible[0]).unwrap();
        let last = app.live.price(visible[199]).unwrap();
        assert!(first <= last);
    });
}
