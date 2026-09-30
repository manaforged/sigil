use super::*;
use sigil::kernel::Cx;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

fn counted_state(rows: Vec<Arc<Item>>, builds: &Rc<Cell<u32>>) -> TableState<Arc<Item>> {
    let counter = Rc::clone(builds);
    TableState::new(
        rows,
        vec![
            TableColumn::new("name", "Name")
                .width(ColWidth::Fixed(100.0))
                .cell(move |row: &Arc<Item>| {
                    counter.set(counter.get() + 1);
                    container().child(text(row.name.clone()))
                }),
        ],
    )
    .row_eq(Arc::ptr_eq)
}

#[test]
fn replacing_one_row_rebuilds_only_that_row() {
    sigil::reactive::Runtime::scope("table-view-row-rebuild", || {
        let builds = Rc::new(Cell::new(0));
        let rows: Vec<Arc<Item>> = demo_rows().into_iter().map(Arc::new).collect();
        let state = counted_state(rows.clone(), &builds);
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let _ = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let before = builds.get();

        let mut next = rows;
        *next.get_mut(1).unwrap() = Arc::new(item("changed", 10));
        state.replace_rows(next);
        let _ = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());

        assert_eq!(builds.get() - before, 1, "only the replaced row rebuilds");
        assert!(
            glyph_texts(&cx, root).contains(&"changed".to_owned()),
            "the replaced row shows its new value"
        );

        let settled = builds.get();
        state.replace_rows(state.rows().peek());
        let _ = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert_eq!(builds.get(), settled, "the same rows rebuild nothing");
    });
}
