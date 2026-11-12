#![allow(clippy::indexing_slicing, clippy::unwrap_used)]

use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::events::EventRouter;
use crate::events::{Event, MouseButton, MouseButtons, PointerPhase};
use crate::geometry::{Point, Rect, Size};
use crate::kernel::{AccessibleName, Cx, NodeId, container};
use crate::layout::{LayoutEngine, LayoutStyle};
use crate::reactive::Owner;
use crate::widgets::{Layout, Widget};

use super::table;
use crate::widgets::table::{TableColumn, TableState};

#[test]
fn fractional_columns_align_across_headers_rows_and_content_changes() {
    Owner::scope("table-column-alignment", || {
        let state = TableState::new(
            vec![[10.0, 300.0], [250.0, 25.0]],
            vec![
                TableColumn::new("first", "First")
                    .cell(|row: &[f32; 2]| container().layout(LayoutStyle::block().width(row[0]))),
                TableColumn::new("second", "Second")
                    .cell(|row: &[f32; 2]| container().layout(LayoutStyle::block().width(row[1]))),
            ],
        );
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .viewport_width(800.0)
                .viewport_height(180.0)
                .selectable_checkboxes(),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(
            crate::layout::FakeMeasure::fixed(60.0, 20.0),
        ));
        for rows in [
            vec![[10.0, 300.0], [250.0, 25.0]],
            vec![[350.0, 10.0], [10.0, 350.0]],
        ] {
            state.replace_rows(rows);
            cx.flush_reactive_children();
            layout
                .compute(&mut cx, root, Size::fixed(800.0, 300.0))
                .unwrap();
            let columns = column_rects(&cx, &layout, root);
            assert_eq!(columns.len(), 2);
            for cells in columns.values() {
                assert_eq!(cells.len(), 3, "header and both rows");
                let header = cells[0];
                for cell in &cells[1..] {
                    assert!((cell.origin.x - header.origin.x).abs() < 0.01, "{cells:?}");
                    assert!((cell.width - header.width).abs() < 0.01, "{cells:?}");
                }
            }
        }
    });
}

fn column_rects(cx: &Cx, layout: &LayoutEngine, root: NodeId) -> BTreeMap<usize, Vec<Rect>> {
    let mut columns = BTreeMap::<_, Vec<_>>::new();
    cx.visit_preorder(root, |id, node| {
        if matches!(
            node.a11y.role,
            crate::kernel::Role::ColumnHeader | crate::kernel::Role::GridCell
        ) && let Some(column) = node.a11y.column_index
            && column > 1
        {
            columns
                .entry(column)
                .or_default()
                .push(layout.result(id).unwrap().rect());
        }
    });
    columns
}

#[test]
fn empty_message_stays_below_header_and_disappears_when_rows_return() {
    Owner::scope("table-empty-message", || {
        let state = TableState::new(vec![], vec![TableColumn::<u32>::new("value", "Value")]);
        let empty = Rc::new(Cell::new(None));
        let capture = empty.clone();
        let mut cx = Cx::new_root();
        let root = cx.add(
            table(state.clone())
                .fill_height()
                .selectable_checkboxes()
                .empty(move || {
                    let capture = capture.clone();
                    container()
                        .layout(LayoutStyle::flex_column().height(64.0))
                        .added(move |_, id| capture.set(Some(id)))
                }),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(
            crate::layout::FakeMeasure::fixed(60.0, 20.0),
        ));
        relayout(&mut cx, &mut layout, root);
        let id = empty.get().expect("empty content");
        let bounds = layout.result(id).expect("empty bounds").rect();
        assert!(
            bounds.origin.y >= 36.0 && bounds.origin.y < 80.0,
            "{bounds:?}"
        );
        assert!(bounds.origin.y + bounds.height <= 600.0, "{bounds:?}");
        state.replace_rows(vec![1]);
        relayout(&mut cx, &mut layout, root);
        assert!(cx.get(id).is_none());
        let checkbox = find_by_label(&cx, root, "Select row 1").expect("row selection");
        let rect = layout.result(checkbox).expect("selection bounds").rect();
        let position = Point::new(
            rect.origin.x + rect.width / 2.0,
            rect.origin.y + rect.height / 2.0,
        );
        let mut router = EventRouter::new();
        click(&mut cx, &mut router, &layout, position);
        assert!(
            state.selection().peek().contains(&0),
            "empty message container must not intercept row selection"
        );
    });
}

fn relayout(cx: &mut Cx, layout: &mut LayoutEngine, root: NodeId) {
    cx.flush_reactive_children();
    layout
        .compute(cx, root, Size::fixed(800.0, 600.0))
        .expect("layout");
}

fn find_by_label(cx: &Cx, root: NodeId, label: &str) -> Option<NodeId> {
    let mut found = None;
    cx.visit_preorder(root, |id, node| {
        if matches!(&node.a11y.name, AccessibleName::Static(name) if name == label) {
            found = Some(id);
        }
    });
    found
}

fn click(cx: &mut Cx, router: &mut EventRouter, layout: &LayoutEngine, position: Point) {
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        router.route(
            cx,
            layout,
            Event::pointer(phase, position)
                .button(Some(MouseButton::Left))
                .buttons(if phase == PointerPhase::Down {
                    MouseButtons::LEFT
                } else {
                    MouseButtons::empty()
                }),
        );
    }
}
