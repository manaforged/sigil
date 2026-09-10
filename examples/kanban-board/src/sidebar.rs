use sigil::prelude::*;

use crate::state::BoardState;

const PROJECT_ROW_HEIGHT: f32 = 36.0;
const PROJECT_VIEWPORT_HEIGHT: f32 = 480.0;

pub(crate) fn sidebar_pane(state: BoardState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().gap(8.0).padding(8.0))
        .class(classes!("h-full rounded-md bg-surface-hover"))
        .child(label("Projects"))
        .child(project_list(state))
}

fn project_list(state: BoardState) -> NodeBuilder {
    let rows: Vec<String> = state.projects.peek();
    container()
        .layout(LayoutStyle::flex_column().grow(1.0))
        .child(
            sigil::widgets::virtual_list(rows, move |idx, name| project_row(state, idx, name))
                .row_height(PROJECT_ROW_HEIGHT)
                .viewport_height(PROJECT_VIEWPORT_HEIGHT)
                .selection(sigil::widgets::SelectionMode::Single),
        )
}

fn project_row(state: BoardState, idx: usize, name: &str) -> NodeBuilder {
    let label_text = name.to_owned();
    container()
        .role(Role::Button)
        .label(name.to_owned())
        .focusable(true)
        .layout(
            LayoutStyle::flex_row()
                .height(PROJECT_ROW_HEIGHT)
                .padding(6.0)
                .align_items(AlignItems::Center),
        )
        .class(classes!("w-full rounded-sm bg-surface"))
        .pointer_down(move |ev| {
            state.select_project(idx);
            ev.stop_propagation();
        })
        .child(label(label_text))
}
