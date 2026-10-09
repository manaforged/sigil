use sigil::prelude::*;

use super::spec::{Build, State, node, off};

pub(crate) const BUILDERS: &[(&str, Build)] = &[
    ("table", table_row),
    ("virtual_list", virtual_list_row),
    ("virtual_list_fill", virtual_list_fill_row),
    ("list_row", list_row_row),
    ("list_row_tree", tree_rows),
    ("kv_grid", kv_grid_row),
    ("kv_row", kv_row_row),
    ("chart", chart_row),
    ("heatmap", heatmap_row),
    ("diff", diff_row),
];

fn table_row(state: State) -> NodeBuilder {
    node(crate::widget_data::table_demo(state == State::Selected).disabled(off(state)))
}

fn tree_rows(state: State) -> NodeBuilder {
    crate::widget_data::list_row_tree_demo(state == State::Selected)
}

fn virtual_list_row(state: State) -> NodeBuilder {
    node(crate::widget_data::virtual_list_demo(
        state == State::Selected,
    ))
}

fn list_row_row(state: State) -> NodeBuilder {
    node(crate::widget_data::list_row_demo(state == State::Selected))
}

fn kv_grid_row(_state: State) -> NodeBuilder {
    node(crate::widget_data::kv_grid_demo())
}

fn kv_row_row(_state: State) -> NodeBuilder {
    node(crate::widget_data::kv_row_demo())
}

fn virtual_list_fill_row(_state: State) -> NodeBuilder {
    crate::widget_data::virtual_list_fill_demo()
}

fn chart_row(_state: State) -> NodeBuilder {
    node(crate::widget_content::chart_demo())
}

fn heatmap_row(_state: State) -> NodeBuilder {
    node(crate::widget_data::heatmap_demo())
}

fn diff_row(_state: State) -> NodeBuilder {
    node(crate::widget_documents::diff_demo())
}
