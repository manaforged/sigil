//! The frame: a heading, the table, and an action bar.
//!
//! Row and header heights are whole multiples of [`CELL`] so every table
//! row lands on exactly one terminal line.

use std::time::Duration;

use sigil::prelude::*;
use sigil::widgets::{button, label, table};

/// Bare-key binding: no modifiers, one printable character.
fn press(ch: &str) -> Shortcut {
    Shortcut::plain(LogicalKey::character(ch))
}

use crate::state::AppState;

/// Terminal cell size in logical pixels. One row, one line.
const CELL: f32 = 8.0;
/// How often the list re-scans the machine.
const REFRESH: Duration = Duration::from_secs(2);
/// How often a finished scan is collected.
const LAND: Duration = Duration::from_millis(50);

/// Build the whole tree.
pub(crate) fn app_tree(state: &AppState) -> NodeBuilder {
    let refresh = state.clone();
    let kill = state.clone();
    let clock = state.clone();
    let inbox = state.clone();
    // A process can exit while you are looking at it. Re-scan on the frame
    // clock so the list is not a photograph — on a worker thread, because a
    // scan forks `ps` and walks the process table.
    sigil::every(REFRESH, move || clock.refresh());
    // And take the answer when it comes back. Cheap: one signal read.
    sigil::every(LAND, move || inbox.land());
    container()
        .role(Role::Main)
        .label("strays")
        .layout(LayoutStyle::flex_column().gap(CELL))
        .class(classes!("p-1 w-full h-full bg-surface"))
        // Capture phase: a terminal app owns its bare keys before the
        // focused row or button sees them.
        .key_capture(press("r"), move |_| refresh.refresh())
        .key_capture(press("k"), move |_| {
            kill.kill_selected();
        })
        .child(heading(state))
        .child(
            // Fill whatever the terminal gives us: the row list is as long
            // as the machine is busy, and the window resizes under it.
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
