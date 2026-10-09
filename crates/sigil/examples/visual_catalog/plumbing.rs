use sigil::prelude::*;

use super::spec::{Build, State, node};

pub(crate) const BUILDERS: &[(&str, Build)] =
    &[("axis_drag", axis_drag_row), ("drag_ghost", drag_ghost_row)];

fn axis_drag_row(_state: State) -> NodeBuilder {
    node(crate::widget_plumbing::axis_drag_demo())
}

fn drag_ghost_row(_state: State) -> NodeBuilder {
    crate::widget_plumbing::drag_ghost_demo()
}
