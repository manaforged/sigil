use std::time::Duration;

use data_table::TABLE_WIDTH;
use sigil::prelude::*;

use crate::state::App;
use data_table::ledger::Row;

pub const WINDOW_W: f32 = 1280.0;
pub const WINDOW_H: f32 = 1000.0;
pub const ROW_H: f32 = 24.0;
pub const HEADER_H: f32 = 28.0;
pub const VIEWPORT_H: f32 = 864.0;
pub const TICK: Duration = Duration::from_millis(16);

#[must_use]
pub fn app_tree(cx: &Cx, app: &App) -> NodeBuilder {
    let ticker = app.clone();
    cx.interval(TICK, move || ticker.live.tick());
    tree(app)
}

#[must_use]
pub fn tree(app: &App) -> NodeBuilder {
    let dark = app.dark;
    let app = app.clone();
    container()
        .layout(LayoutStyle::block().fixed_size(WINDOW_W, WINDOW_H))
        .children_reactive(move || vec![frame(&app, dark.get())])
}

fn frame(app: &App, dark: bool) -> NodeBuilder {
    let theme = if dark {
        Theme::dark()
    } else {
        Theme::default()
    };
    let edit = app.clone();
    let cancel = app.clone();
    let undo_ctrl = app.clone();
    let undo_meta = app.clone();
    container()
        .role(Role::Main)
        .label("Ledger")
        .theme(theme)
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(WINDOW_W, WINDOW_H)
                .gap(8.0)
                .padding(16.0),
        )
        .class(classes!("bg-surface text-on-surface text-sm"))
        .key_capture(Shortcut::plain(Key::Enter), move |event| {
            if edit.live.editing.peek().is_none() {
                edit.edit_selected();
                event.stop_propagation();
            }
        })
        .key_capture(Shortcut::plain(Key::Escape), move |_| cancel.live.cancel())
        .key_capture(Shortcut::ctrl(LogicalKey::character("z")), move |_| {
            undo_ctrl.live.undo();
        })
        .key_capture(
            Shortcut::new(LogicalKey::character("z"), ModifiersState::META),
            move |_| undo_meta.live.undo(),
        )
        .child(toolbar(app))
        .child(grid(app))
        .child(status(app))
}

fn toolbar(app: &App) -> NodeBuilder {
    let undo = app.clone();
    let theme = app.clone();
    let depth = app.live.undo;
    let undo_label = derived(move || format!("Undo {}", depth.get().len()));
    let sort = app.table.sort_key();
    let sort_label = derived(move || {
        sort.get().map_or_else(
            || String::from("unsorted"),
            |key| format!("sort {} {:?}", key.column, key.dir),
        )
    });
    container()
        .layout(
            LayoutStyle::flex_row()
                .height(32.0)
                .gap(8.0)
                .align_items(AlignItems::Center),
        )
        .child(
            text_field(app.table.quick_filter())
                .placeholder("Filter symbol or name")
                .width(260.0),
        )
        .child(label(sort_label).class(classes!("text-xs")))
        .child(button(undo_label).activate(move || undo.live.undo()))
        .child(button("Theme").activate(move || theme.toggle_theme()))
}

fn grid(app: &App) -> TableBuilder<Row> {
    let menu = app.clone();
    table(app.table.clone())
        .density(Density::Grid)
        .striped()
        .scroll_bind(app.scroll)
        .row_height(ROW_H)
        .header_height(HEADER_H)
        .viewport_width(TABLE_WIDTH)
        .viewport_height(VIEWPORT_H)
        .row_menu(move |row| row_menu(&menu, row))
}

fn row_menu(app: &App, row: &Row) -> Vec<MenuItem> {
    let index = u32::try_from(row.index).unwrap_or(u32::MAX);
    let edit = app.clone();
    let clear = app.clone();
    vec![
        MenuItem::new("Edit last done")
            .shortcut("Enter")
            .select(move || edit.live.edit(index)),
        MenuItem::new("Clear selection").select(move || clear.table.clear_selection()),
    ]
}

fn status(app: &App) -> NodeBuilder {
    let total = app.table.rows().peek().len();
    let visible = app.table.visible();
    let selection = app.table.selection();
    let ticks = app.live.ticks;
    let moved = app.live.moved;
    let line = derived(move || {
        format!(
            "{total} rows · {} shown · {} selected · {} ticks · {} cells moved",
            visible.get().len(),
            selection.get().len(),
            ticks.get(),
            moved.get()
        )
    });
    container()
        .layout(LayoutStyle::block().fill_width().height(20.0))
        .child(label(line).class(classes!("text-xs")))
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "bench_tests.rs"]
mod bench_tests;

#[cfg(test)]
#[path = "scroll_tests.rs"]
mod scroll_tests;

#[cfg(test)]
#[path = "gpu_tests.rs"]
mod gpu_tests;
