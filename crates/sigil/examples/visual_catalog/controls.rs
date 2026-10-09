use sigil::prelude::*;

use super::spec::{Build, State, node, off};

pub(crate) const BUILDERS: &[(&str, Build)] = &[
    ("button", button_primary),
    ("button-secondary", button_plain),
    ("button-ghost", button_ghost),
    ("button-pending", button_pending),
    ("icon_button", icon_button_row),
    ("checkbox", checkbox_row),
    ("checkbox-unchecked", checkbox_unchecked),
    ("toggle", toggle_row),
    ("toggle-off", toggle_off),
    ("radio_group", radio_row),
    ("slider", slider_row),
    ("interactive", interactive_row),
    ("progress", progress_row),
    ("gauge", gauge_row),
    ("spinner", spinner_row),
];

fn button_primary(state: State) -> NodeBuilder {
    node(crate::widget_controls::button_demo().disabled(off(state)))
}

fn button_plain(state: State) -> NodeBuilder {
    node(crate::widget_controls::button_secondary_demo().disabled(off(state)))
}

fn button_ghost(state: State) -> NodeBuilder {
    node(crate::widget_controls::button_ghost_demo().disabled(off(state)))
}

fn button_pending(_state: State) -> NodeBuilder {
    node(crate::widget_controls::button_pending_demo())
}

fn icon_button_row(state: State) -> NodeBuilder {
    node(crate::widget_controls::icon_button_demo().disabled(off(state)))
}

fn checkbox_row(state: State) -> NodeBuilder {
    node(crate::widget_controls::checkbox_demo().disabled(off(state)))
}

fn checkbox_unchecked(state: State) -> NodeBuilder {
    node(crate::widget_controls::checkbox_unchecked_demo().disabled(off(state)))
}

fn toggle_row(state: State) -> NodeBuilder {
    node(crate::widget_controls::toggle_demo().disabled(off(state)))
}

fn toggle_off(state: State) -> NodeBuilder {
    node(crate::widget_controls::toggle_off_demo().disabled(off(state)))
}

fn radio_row(state: State) -> NodeBuilder {
    node(crate::widget_controls::radio_group_demo().disabled(off(state)))
}

fn slider_row(state: State) -> NodeBuilder {
    node(crate::widget_controls::slider_demo().disabled(off(state)))
}

fn interactive_row(state: State) -> NodeBuilder {
    crate::widget_plumbing::interactive_demo()
        .disabled(off(state))
        .into_node_builder()
}

fn progress_row(_state: State) -> NodeBuilder {
    node(crate::widget_indicators::progress_demo())
}

fn gauge_row(_state: State) -> NodeBuilder {
    node(crate::widget_indicators::gauge_demo())
}

fn spinner_row(_state: State) -> NodeBuilder {
    node(crate::widget_indicators::spinner_demo())
}
