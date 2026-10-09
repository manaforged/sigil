use sigil::prelude::*;

use super::spec::{Build, State, node};

pub(crate) const BUILDERS: &[(&str, Build)] = &[
    ("panel", panel_row),
    ("card", card_row),
    ("surface", surface_row),
    ("sidebar", sidebar_row),
    ("status_bar", status_row),
    ("app_shell", shell_row),
    ("menu_bar", menu_bar_row),
    ("tabs", tabs_row),
    ("tabs_line", tabs_line_row),
    ("splitter", splitter_row),
    ("scroll", scroll_fixed_row),
    ("scroll_auto", scroll_row),
    ("terminal", terminal_row),
];

fn active(state: State) -> Signal<usize> {
    signal(usize::from(state == State::Selected))
}

fn disabled_tabs(state: State) -> &'static [usize] {
    if state == State::Disabled { &[1] } else { &[] }
}

fn panel_row(_state: State) -> NodeBuilder {
    node(crate::widget_chrome::panel_demo())
}

fn card_row(_state: State) -> NodeBuilder {
    node(crate::widget_chrome::card_demo())
}

fn surface_row(_state: State) -> NodeBuilder {
    node(crate::widget_chrome::surface_demo())
}

fn sidebar_row(_state: State) -> NodeBuilder {
    crate::widget_chrome::sidebar_demo()
}

fn status_row(_state: State) -> NodeBuilder {
    node(crate::widget_chrome::status_bar_demo())
}

fn shell_row(_state: State) -> NodeBuilder {
    crate::widget_chrome::app_shell_demo()
}

fn menu_bar_row(_state: State) -> NodeBuilder {
    node(crate::widget_overlays::menu_bar_demo())
}

fn tabs_row(state: State) -> NodeBuilder {
    node(
        crate::widget_chrome::tabs_demo()
            .active(active(state))
            .disabled(disabled_tabs(state)),
    )
}

fn tabs_line_row(state: State) -> NodeBuilder {
    node(
        crate::widget_chrome::tabs_line_demo()
            .active(active(state))
            .disabled(disabled_tabs(state)),
    )
}

fn splitter_row(_state: State) -> NodeBuilder {
    node(crate::widget_chrome::splitter_demo())
}

fn scroll_fixed_row(_state: State) -> NodeBuilder {
    crate::widget_chrome::scroll_demo()
}

fn scroll_row(_state: State) -> NodeBuilder {
    crate::widget_chrome::scroll_auto_demo()
}

fn terminal_row(_state: State) -> NodeBuilder {
    crate::widget_plumbing::terminal_demo()
}
