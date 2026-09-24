use sigil::prelude::*;
use sigil::widgets::Axis;

use super::spec::{Build, State, node};

pub(crate) const BUILDERS: &[(&str, Build)] =
    &[("axis_drag", axis_drag_row), ("drag_ghost", drag_ghost_row)];

fn axis_drag_row(_state: State) -> NodeBuilder {
    let width = signal(160.0_f32);
    let handle = container()
        .layout(LayoutStyle::flex_column().width(width.peek()))
        .child(card(label(TextContent::compute(move || {
            format!("Width {:.0}", width.get())
        }))));
    node(axis_drag(
        handle,
        Axis::X,
        move || width.peek(),
        move |start, delta| width.set((start + delta).max(80.0)),
        || {},
    ))
}

fn drag_ghost_row(_state: State) -> NodeBuilder {
    let drag = DragState::<String>::new();
    let dropped = signal(String::from("Drop a card here"));
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(8.0))
        .child(card(label("Card A")).drag_source(drag.clone(), String::from("Card A")))
        .child(card(label(dropped)).drop_target(
            drag.clone(),
            |_| true,
            move |item| dropped.set(format!("Dropped {item}")),
        ))
        .child(drag_ghost(drag, |item: &String| card(label(item.clone()))))
}
