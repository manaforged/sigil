use sigil::prelude::*;

use super::spec::{Build, State, node};

pub(crate) const BUILDERS: &[(&str, Build)] = &[
    ("menu", menu_row),
    ("modal", modal_row),
    ("sheet", sheet_row),
    ("tooltip", tooltip_row),
    ("toast", toast_row),
    ("toast_queue", toast_queue_row),
    ("command_menu", command_menu_row),
    ("hint_overlay", hint_overlay_row),
];

fn shown(state: State) -> Signal<bool> {
    signal(state != State::Default)
}

fn menu_row(state: State) -> NodeBuilder {
    node(crate::widget_overlays::menu_demo(shown(state)))
}

fn modal_row(state: State) -> NodeBuilder {
    crate::widget_overlays::modal_demo(shown(state))
}

fn sheet_row(state: State) -> NodeBuilder {
    crate::widget_overlays::sheet_demo(shown(state))
}

fn tooltip_row(_state: State) -> NodeBuilder {
    crate::widget_overlays::tooltip_demo()
}

fn toast_row(state: State) -> NodeBuilder {
    crate::widget_overlays::toast_demo(state == State::Shown)
}

fn toast_queue_row(state: State) -> NodeBuilder {
    crate::widget_overlays::toast_queue_demo(state == State::Shown)
}

fn command_menu_row(_state: State) -> NodeBuilder {
    node(crate::widget_overlays::command_menu_demo())
}

fn hint_overlay_row(_state: State) -> NodeBuilder {
    node(crate::widget_overlays::hint_overlay_demo())
}
