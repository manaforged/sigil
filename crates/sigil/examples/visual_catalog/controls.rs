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
    node(button("Save changes").primary().disabled(off(state)))
}

fn button_plain(state: State) -> NodeBuilder {
    node(button("Cancel").disabled(off(state)))
}

fn button_ghost(state: State) -> NodeBuilder {
    node(button("Details").ghost().disabled(off(state)))
}

fn button_pending(_state: State) -> NodeBuilder {
    node(button("Saving").primary().pending(signal(true)))
}

fn icon_button_row(state: State) -> NodeBuilder {
    node(
        icon_button("×", || {})
            .accessible("Reset")
            .disabled(off(state)),
    )
}

fn checkbox_row(state: State) -> NodeBuilder {
    node(checkbox(signal(true)).label("Enabled").disabled(off(state)))
}

fn toggle_row(state: State) -> NodeBuilder {
    node(toggle(signal(true)).label("Locked").disabled(off(state)))
}

fn checkbox_unchecked(state: State) -> NodeBuilder {
    node(
        checkbox(signal(false))
            .label("Enabled")
            .disabled(off(state)),
    )
}

fn toggle_off(state: State) -> NodeBuilder {
    node(toggle(signal(false)).label("Locked").disabled(off(state)))
}

fn radio_row(state: State) -> NodeBuilder {
    node(
        radio_group(signal(String::from("auto")))
            .option("auto", "Automatic")
            .option("manual", "Manual")
            .disabled(off(state)),
    )
}

fn slider_row(state: State) -> NodeBuilder {
    node(
        slider(signal(40.0), 0.0..=100.0)
            .step(5.0)
            .disabled(off(state)),
    )
}

fn interactive_row(state: State) -> NodeBuilder {
    interactive(node(text("Interactive row").nowrap().font_size(14.0)))
        .focusable(true)
        .rounded(Radius::Md)
        .disabled(off(state))
        .into_node_builder()
}

fn progress_row(_state: State) -> NodeBuilder {
    node(progress(signal(60.0), 100.0))
}

fn gauge_row(_state: State) -> NodeBuilder {
    node(gauge(signal(40.0)).max(100.0).label("Battery").warn(0.25))
}

fn spinner_row(_state: State) -> NodeBuilder {
    node(spinner())
}
