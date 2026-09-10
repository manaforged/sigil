use std::time::Duration;

use sigil::prelude::*;
use sigil::widgets::{button, label, table};

fn press(ch: &str) -> Shortcut {
    Shortcut::plain(LogicalKey::character(ch))
}

use crate::state::AppState;

const CELL: f32 = 8.0;
const REFRESH: Duration = Duration::from_secs(2);
const LAND: Duration = Duration::from_millis(50);

pub(crate) fn app_tree(state: &AppState) -> NodeBuilder {
    let refresh = state.clone();
    let kill = state.clone();
    let clock = state.clone();
    let inbox = state.clone();
    sigil::every(REFRESH, move || clock.refresh());
    sigil::every(LAND, move || inbox.land());
    container()
        .role(Role::Main)
        .label("strays")
        .layout(LayoutStyle::flex_column().gap(CELL))
        .class(classes!("p-1 w-full h-full bg-surface"))
        .key_capture(press("r"), move |_| refresh.refresh())
        .key_capture(press("k"), move |_| {
            kill.kill_selected();
        })
        .child(heading(state))
        .child(
            table(state.table.clone())
                .row_height(CELL)
                .header_height(CELL)
                .viewport_fill()
                .empty(|| container().child(label("Nothing is listening."))),
        )
        .child(actions(state))
}

fn heading(state: &AppState) -> NodeBuilder {
    let table = state.table.clone();
    let owned = state.clone();
    let summary = derived(move || {
        format!(
            "strays — {} listening, {} stray",
            table.rows().get().len(),
            owned.stray_count()
        )
    });
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(CELL)
                .gap(CELL * 2.0)
                .align_items(AlignItems::Center),
        )
        .child(label(summary).class(classes!("text-on-surface font-bold")))
}

fn actions(state: &AppState) -> NodeBuilder {
    let refresh = state.clone();
    let kill = state.clone();
    let table = state.table.clone();
    let selection = derived(move || {
        table.selected_row().map_or_else(
            || String::from("no row selected"),
            |row| format!("selected {} ({})", row.pid, row.place()),
        )
    });
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(CELL)
                .gap(CELL * 2.0)
                .align_items(AlignItems::Center),
        )
        .child(button("Refresh").activate(move || refresh.refresh()))
        .child(button("Kill").activate(move || {
            kill.kill_selected();
        }))
        .child(label(selection).class(classes!("text-on-surface")))
        .child(
            label("auto every 2s · r refresh · k kill · Ctrl+C quit")
                .class(classes!("text-on-surface")),
        )
}
