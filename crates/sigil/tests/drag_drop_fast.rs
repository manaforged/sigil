#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

use sigil::geometry::Rect;
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil::widgets::{DragDropExt, DragState};
use sigil_kittest::input::{pointer_down, pointer_drag, pointer_move, pointer_up};
use sigil_kittest::{Harness, Query};

fn row_bounds(ui: &mut Harness, name: &str) -> Rect {
    ui.find_all(&Query::role(Role::ListItem))
        .into_iter()
        .find(|node| node.name() == Some(name))
        .and_then(|node| node.bounds())
        .unwrap()
}

fn drag_rows(first: f32, release_below: bool) -> (Option<(u32, u32)>, bool) {
    let dropped = signal(None::<(u32, u32)>);
    let drag: DragState<u32> = DragState::new();
    let probe = drag.clone();
    let mut ui = Harness::new(move || {
        (0..8_u32).fold(column(), |col, id| {
            col.child(
                container()
                    .role(Role::ListItem)
                    .label(format!("row {id}"))
                    .layout(LayoutStyle::block().fill_width().height(28.0))
                    .child(text(format!("row {id}")))
                    .drag_source(drag.clone(), id)
                    .drop_target(
                        drag.clone(),
                        move |from: &u32| *from != id,
                        move |from| dropped.set(Some((from, id))),
                    ),
            )
        })
    })
    .pixels(400, 400);
    ui.settle();
    let from = row_bounds(&mut ui, "row 0");
    let onto = row_bounds(&mut ui, "row 4");
    let (x, fy) = (from.origin.x + 20.0, from.origin.y + from.height / 2.0);
    let ty = onto.origin.y + onto.height / 2.0;
    ui.send([pointer_move(x, fy), pointer_down(x, fy)]);
    ui.send([pointer_drag(x, fy + first)]);
    for t in [0.125_f32, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875, 1.0] {
        ui.send([pointer_drag(x, (ty - fy).mul_add(t, fy))]);
    }
    let release_y = if release_below { 380.0 } else { ty };
    if release_below {
        ui.send([pointer_drag(x, release_y)]);
    }
    ui.send([pointer_up(x, release_y)]);
    ui.settle();
    (dropped.peek(), probe.is_active())
}

fn drop_after_first_move(first: f32) -> Option<(u32, u32)> {
    drag_rows(first, false).0
}

#[test]
fn a_drag_starts_when_the_first_move_stays_on_the_source() {
    Runtime::scope("drag-slow", || {
        assert_eq!(drop_after_first_move(10.0), Some((0, 4)));
    });
}

#[test]
fn a_drag_starts_when_the_first_move_already_left_the_source() {
    Runtime::scope("drag-fast", || {
        assert_eq!(drop_after_first_move(18.0), Some((0, 4)));
    });
}

#[test]
fn a_fast_drag_released_over_empty_space_ends() {
    Runtime::scope("drag-gap", || {
        let (dropped, active) = drag_rows(18.0, true);
        assert_eq!(dropped, None);
        assert!(
            !active,
            "the drag stayed active after the button was released"
        );
    });
}
