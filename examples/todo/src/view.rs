//! To-do app view. Header + add row, filter tabs, the reactive task list,
//! and a status footer. The native menu bar, file dialogs, and tray live in
//! `main.rs`.

use sigil::prelude::*;

use crate::state::{Filter, Todo, TodoState};

/// Build the whole window tree.
pub(crate) fn app_tree(state: &TodoState) -> NodeBuilder {
    let state = *state;
    container()
        .layout(LayoutStyle::flex_column().fill().gap(10.0).padding(16.0))
        .class(classes!("bg-surface"))
        .child(header())
        .child(add_row(state))
        .child(filter_row(state))
        .child(task_list(state))
        .child(footer(state))
}

fn header() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().gap(8.0))
        .child(text("\u{2705} Todos").class(classes!("text-xl font-bold text-on-surface")))
}

fn add_row(state: TodoState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().gap(8.0))
        .child(
            container().layout(LayoutStyle::flex_row().grow(1.0)).child(
                text_field(state.draft)
                    .placeholder("What needs doing?")
                    .submit(move |_| state.add()),
            ),
        )
        .child(button("Add").primary().activate(move || state.add()))
}

fn filter_row(state: TodoState) -> NodeBuilder {
    let count = derived(move || format!("{} active", state.active_count()));
    container()
        .layout(LayoutStyle::flex_row().gap(8.0))
        .child(filter_button(state, "All", Filter::All))
        .child(filter_button(state, "Active", Filter::Active))
        .child(filter_button(state, "Done", Filter::Done))
        .child(
            container()
                .layout(
                    LayoutStyle::flex_row()
                        .grow(1.0)
                        .justify_content(JustifyContent::End),
                )
                .child(label(count).class(classes!("text-on-surface"))),
        )
}

fn filter_button(state: TodoState, name: &'static str, filter: Filter) -> ButtonBuilder {
    button(name).activate(move || state.set_filter(filter))
}

fn task_list(state: TodoState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().grow(1.0).fill_width().gap(6.0))
        .children_reactive(move || {
            state
                .visible()
                .into_iter()
                .map(|(index, todo)| task_row(state, index, todo))
                .collect::<Vec<_>>()
        })
}

fn task_row(state: TodoState, index: usize, todo: Todo) -> NodeBuilder {
    let marker = if todo.done { "[x]" } else { "[ ]" };
    let opacity = if todo.done { "opacity-50" } else { "" };
    container()
        .layout(LayoutStyle::flex_row().fill_width().gap(8.0).padding(8.0))
        .class(classes!("rounded-md bg-surface-hover"))
        .child(button(marker).activate(move || state.toggle(index)))
        .child(
            container()
                .layout(LayoutStyle::flex_row().grow(1.0))
                .class(opacity)
                .child(text(todo.text).class(classes!("text-on-surface"))),
        )
        .child(button("Del").activate(move || state.remove(index)))
}

fn footer(state: TodoState) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().gap(8.0))
        .child(
            container()
                .layout(LayoutStyle::flex_row().grow(1.0))
                .child(label(state.status).class(classes!("text-on-surface"))),
        )
        .child(button("Clear completed").activate(move || state.clear_completed()))
}
