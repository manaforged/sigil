//! View construction for the log tailer.
//!
//! Composes a header bar (filter text field + level picker buttons +
//! auto-scroll toggle), a virtualised log list, and a status bar.

use sigil::prelude::*;
use sigil::widgets::{SelectionMode, virtual_list};

use crate::data::{Level, LogLine};
use crate::state::AppState;

const ROW_HEIGHT: f32 = 24.0;
const VIEWPORT_WIDTH: f32 = 928.0;
const VIEWPORT_HEIGHT: f32 = 520.0;
const LEVEL_CHIP_WIDTH: f32 = 64.0;

/// Build the full log-tailer tree.
pub(crate) fn app_tree(state: AppState) -> NodeBuilder {
    container()
        .role(Role::Main)
        .label("Sigil log tailer")
        .layout(LayoutStyle::flex_column().fixed_size(960.0, 640.0).gap(8.0))
        .class(classes!("p-4 bg-surface"))
        .child(header(state))
        .child(log_list(state))
        .child(status_bar(state))
}

fn header(state: AppState) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(48.0)
                .gap(12.0)
                .align_items(AlignItems::Center)
                .padding(8.0),
        )
        .class(classes!("w-full rounded-md bg-surface-hover"))
        .child(label("Filter:"))
        .child(filter_input(state))
        .child(level_buttons(state))
        .child(auto_scroll_toggle(state))
}

fn filter_input(state: AppState) -> NodeBuilder {
    // text_field has no `.layout()` of its own, so wrap it in a sized
    // container.
    container()
        .layout(LayoutStyle::block().width(240.0).height(32.0))
        .child(
            text_field(state.filter_text)
                .placeholder("substring")
                .change(move |_| {
                    // The signal itself is the source of truth; the
                    // list rebuilds on every render and re-reads
                    // state.filter_text via `state.filtered()`.
                }),
        )
}

fn level_buttons(state: AppState) -> NodeBuilder {
    let mut row = container()
        .layout(
            LayoutStyle::flex_row()
                .gap(4.0)
                .align_items(AlignItems::Center),
        )
        .child(level_button(state, None, "ALL"));
    for level in Level::all() {
        row = row.child(level_button(state, Some(level), level.label()));
    }
    row
}

fn level_button(state: AppState, target: Option<Level>, name: &'static str) -> impl IntoNode {
    let filter = state.level_filter;
    let mut btn = button(name).activate(move || filter.set(target));
    if state.level_filter.peek() == target {
        btn = btn.primary();
    }
    btn
}

fn auto_scroll_toggle(state: AppState) -> impl IntoNode {
    toggle(state.auto_scroll).label("auto-scroll")
}

fn log_list(state: AppState) -> NodeBuilder {
    let lines = state.filtered();
    state.refresh_visible_label(lines.len());

    let auto_scroll = state.auto_scroll.peek();
    let selected = state.selected;
    let mut list = virtual_list(lines, move |index, line: &LogLine| {
        row(index, line, selected)
    })
    .row_height(ROW_HEIGHT)
    .viewport_height(VIEWPORT_HEIGHT)
    .viewport_width(VIEWPORT_WIDTH)
    .selection(SelectionMode::Single);
    // Auto-scroll: when toggled on, seed the scroll signal at the
    // bottom of the content; otherwise the list pins to the top by
    // default. `scroll_to_end` replaces the hand-computed
    // `(total_rows * row_height - viewport_height).max(0.0)` recipe.
    if auto_scroll {
        list = list.scroll_to_end();
    }
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(VIEWPORT_WIDTH, VIEWPORT_HEIGHT)
                .grow(1.0),
        )
        .child(list)
}

fn row(index: usize, line: &LogLine, selected: Signal<Option<usize>>) -> NodeBuilder {
    let text_value = line.format();
    container()
        .role(Role::ListItem)
        .label(text_value.clone())
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(VIEWPORT_WIDTH, ROW_HEIGHT)
                .gap(8.0)
                .align_items(AlignItems::Center)
                .padding(4.0),
        )
        .class(classes!("text-on-surface"))
        .pointer_down(move |_| selected.set(Some(index)))
        .child(
            chip(line.level.label(), line.level.tone())
                .layout(LayoutStyle::block().width(LEVEL_CHIP_WIDTH)),
        )
        .child(label(text_value))
}

fn status_bar(state: AppState) -> NodeBuilder {
    let visible_label = state.visible_label;
    let selected = state.selected;
    let selection_label = derived(move || {
        selected.get().map_or_else(
            || String::from("No selection"),
            |index| format!("Selected row #{index}"),
        )
    });
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(32.0)
                .gap(16.0)
                .align_items(AlignItems::Center)
                .justify_content(JustifyContent::SpaceBetween)
                .padding(8.0),
        )
        .class(classes!("w-full rounded-md bg-surface-hover"))
        .child(label(visible_label))
        .child(label(selection_label))
}
