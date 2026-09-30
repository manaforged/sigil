#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{Axis, InteractiveBuilder};

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - plumbing").size(560, 480), build)
}

fn interactive_demo() -> InteractiveBuilder {
    interactive(container().child(text("Focusable row").nowrap()))
        .focusable(true)
        .rounded(Radius::Md)
}

fn axis_drag_demo() -> NodeBuilder {
    let width = signal(120.0_f32);
    let handle = card(text(move || format!("Drag me: {:.0}", width.get())).nowrap());
    axis_drag(
        handle,
        Axis::X,
        move || width.peek(),
        move |start, delta| width.set((start + delta).max(40.0)),
        || {},
    )
}

fn drag_ghost_demo() -> NodeBuilder {
    let drag = DragState::<String>::new();
    let dropped = signal(String::from("Drop a card here"));
    container()
        .layout(LayoutStyle::flex_column().gap(rows(1.0)))
        .child(card(text("Card A").nowrap()).drag_source(drag.clone(), String::from("Card A")))
        .child(card(text(dropped).nowrap()).drop_target(
            drag.clone(),
            |_| true,
            move |item| dropped.set(format!("Dropped {item}")),
        ))
        .child(drag_ghost(drag, |item: &String| {
            text(item.clone()).nowrap()
        }))
}

fn terminal_demo() -> NodeBuilder {
    let lines = signal(vec![String::from("$ echo ready"), String::from("ready")]);
    terminal(lines, move |input| {
        lines.update(|all| all.push(format!("$ {input}")));
    })
}

fn build() -> impl IntoNode {
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
        .child(interactive_demo())
        .child(axis_drag_demo())
        .child(drag_ghost_demo())
        .child(terminal_demo())
}
