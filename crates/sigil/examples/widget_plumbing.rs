#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::Orientation;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - plumbing").size(560, 480), build)
}

pub(crate) fn interactive_demo(disabled: Signal<bool>) -> NodeBuilder {
    interactive(
        container()
            .layout(LayoutStyle::flex_column().fill_width())
            .child(text("Interactive row").nowrap().text_role(TextRole::Body)),
    )
    .focusable(true)
    .rounded(Radius::Md)
    .disabled(disabled)
    .into_node_builder()
}

pub(crate) fn axis_drag_demo() -> NodeBuilder {
    let width = signal(160.0_f32);
    let handle = container()
        .layout(LayoutStyle::flex_column().width(width.peek()))
        .child(card(
            text(move || format!("Width {:.0}", width.get())).nowrap(),
        ));
    axis_drag(
        handle,
        Orientation::Horizontal,
        move || width.peek(),
        move |start, delta| width.set((start + delta).max(80.0)),
        || {},
    )
}

pub(crate) fn drag_ghost_demo() -> NodeBuilder {
    let drag = DragState::<String>::new();
    let dropped = signal(String::from("Drop a card here"));
    container()
        .layout(LayoutStyle::flex_column().fill_width().gap(Space::Sm))
        .child(
            NodeBuilder::from(card(text("Card A").nowrap()))
                .drag_source(drag.clone(), String::from("Card A")),
        )
        .child(NodeBuilder::from(card(text(dropped).nowrap())).drop_target(
            drag.clone(),
            |_| true,
            move |item| dropped.set(format!("Dropped {item}")),
        ))
        .child(drag_ghost(drag, |item: &String| {
            card(text(item.clone()).nowrap())
        }))
}

pub(crate) fn terminal_demo() -> NodeBuilder {
    let lines = vec![
        String::from("$ cargo build"),
        String::from("   Compiling sigil-ui v0.1.0"),
        String::from("    Finished dev profile"),
    ];
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(120.0))
        .child(terminal(signal(lines), |_| {}))
}

fn build() -> impl IntoNode {
    container()
        .role(Role::Main)
        .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
        .child(interactive_demo(signal(false)))
        .child(axis_drag_demo())
        .child(drag_ghost_demo())
        .child(terminal_demo())
}
