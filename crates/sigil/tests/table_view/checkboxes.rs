use super::*;
use sigil::events::EventRouter;
use sigil::kernel::{Cx, NodeId};

fn check_table(state: TableState<Item>) -> impl IntoNode {
    table(state)
        .selectable_checkboxes()
        .row_height(ROW_H)
        .header_height(HEADER_H)
        .viewport_width(WIDTH)
        .viewport_height(VIEW_H)
}

fn checkbox_nodes(cx: &Cx, root: NodeId) -> Vec<NodeId> {
    fn walk(cx: &Cx, id: NodeId, out: &mut Vec<NodeId>) {
        let Some(node) = cx.get(id) else {
            return;
        };
        if node.a11y.role == Role::CheckBox {
            out.push(id);
        }
        for child in node.children() {
            walk(cx, *child, out);
        }
    }
    let mut out = Vec::new();
    walk(cx, root, &mut out);
    out
}

fn header_checked(cx: &Cx, root: NodeId) -> Option<bool> {
    let header = checkbox_nodes(cx, root)
        .first()
        .copied()
        .expect("header box");
    cx.get(header)
        .and_then(|node| node.a11y.checked)
        .map(|checked| checked.get())
}

const CHECK_X: f32 = 28.0;

#[test]
fn checkbox_column_absent_by_default() {
    sigil::reactive::Runtime::scope("table-view-checks-default-off", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state)
                .row_height(ROW_H)
                .header_height(HEADER_H)
                .viewport_width(WIDTH)
                .viewport_height(VIEW_H),
        );
        let _layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        assert!(
            checkbox_nodes(&cx, root).is_empty(),
            "no checkbox nodes without the opt-in",
        );
        assert_eq!(
            cx.get(root).expect("grid").a11y.column_count,
            Some(2),
            "column count unchanged without the opt-in",
        );
    });
}

#[test]
fn row_checkbox_toggles_exactly_once() {
    sigil::reactive::Runtime::scope("table-view-checks-toggle", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(check_table(state.clone()));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();
        assert_eq!(checkbox_nodes(&cx, root).len(), 5, "header + 4 row boxes");
        assert_eq!(
            cx.get(root).expect("grid").a11y.column_count,
            Some(3),
            "checkbox column counts into the grid",
        );

        let row_y = ROW_H.mul_add(1.5, HEADER_H);
        click(&mut router, &mut cx, &layout, CHECK_X, row_y);
        let selected = state.selection().get();
        assert!(
            selected.len() == 1 && selected.contains(&1),
            "one click checks the row exactly once (a leaked row pointer-down \
             would select-then-toggle back to empty): {selected:?}",
        );

        click(&mut router, &mut cx, &layout, CHECK_X, row_y);
        assert!(
            state.selection().get().is_empty(),
            "second click unchecks the row",
        );

        click(
            &mut router,
            &mut cx,
            &layout,
            100.0,
            ROW_H.mul_add(0.5, HEADER_H),
        );
        let selected = state.selection().get();
        assert!(
            selected.len() == 1 && selected.contains(&0),
            "row click still selects only that row: {selected:?}",
        );
    });
}

#[test]
fn header_checkbox_selects_all_visible_then_clears() {
    sigil::reactive::Runtime::scope("table-view-checks-all", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(check_table(state.clone()));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        click(&mut router, &mut cx, &layout, CHECK_X, HEADER_H / 2.0);
        let selected = state.selection().get();
        assert_eq!(selected.len(), 4, "header check selects every visible row");
        assert_eq!(header_checked(&cx, root), Some(true));

        click(&mut router, &mut cx, &layout, CHECK_X, HEADER_H / 2.0);
        assert!(
            state.selection().get().is_empty(),
            "header click with all selected clears",
        );
        assert_eq!(header_checked(&cx, root), Some(false));
    });
}

#[test]
fn header_checkbox_indeterminate_when_partial() {
    sigil::reactive::Runtime::scope("table-view-checks-partial", || {
        let state = demo_state();
        let mut cx = Cx::new_root();
        let root = cx.add(check_table(state.clone()));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        state.select_only(1);
        assert_eq!(
            header_checked(&cx, root),
            Some(false),
            "partial selection does not read as fully checked",
        );

        click(&mut router, &mut cx, &layout, CHECK_X, HEADER_H / 2.0);
        assert_eq!(
            state.selection().get().len(),
            4,
            "click from partial selects all visible rows",
        );
        assert_eq!(header_checked(&cx, root), Some(true));
    });
}

#[test]
fn header_checkbox_selects_only_visible_rows() {
    sigil::reactive::Runtime::scope("table-view-checks-visible", || {
        let state = demo_state();
        state.quick_filter().set("ta".to_owned());
        let mut cx = Cx::new_root();
        let root = cx.add(check_table(state.clone()));
        let layout = compute(&mut cx, root, VIEWPORT, sigil_kittest::stub_text_measure());
        let mut router = EventRouter::new();

        click(&mut router, &mut cx, &layout, CHECK_X, HEADER_H / 2.0);
        let selected = state.selection().get();
        assert!(
            selected.len() == 2 && selected.contains(&2) && selected.contains(&3),
            "select-all covers visible rows only: {selected:?}",
        );
        assert_eq!(
            header_checked(&cx, root),
            Some(true),
            "every visible row selected reads checked",
        );
    });
}
