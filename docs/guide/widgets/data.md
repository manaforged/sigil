# Data

Lists and grids that present collections. `virtual_list` is the virtualized primitive; `table` builds a sortable, filterable, column-managed grid on top of it.

## virtual_list

`virtual_list(items, render) -> VirtualListBuilder<T, R>` where `render` is `Fn(usize, &T) -> NodeBuilder` and `items` is a `Vec<T>`, or a `Signal<S>` whose target is `AsRef<[T]>` (`Signal<Vec<T>>`, `Signal<Arc<Vec<T>>>`, …). When a bound signal changes, the list rebuilds the rows in its window. With `.key_by` or `.identified()`, only rows with a new key rebuild.

```rust,no_run
# use sigil::prelude::*;
# struct Row { name: String, detail: String }
# let rows = vec![Row { name: "a.txt".into(), detail: "4 KB".into() }];
# let _ = {
virtual_list(rows, |index, row| {
    list_row(row.name.clone()).selected(index == 3).status(row.detail.clone())
})
.selection(SelectionMode::Single)
.viewport_height(400.0);
# };
```

Only the rows inside the viewport plus an overscan margin are materialized; scrolling reuses a fixed window. Each row sits in a slot of the row height. Defaults: row height 32 px (one cell on the terminal), viewport height 480 px, overscan 1 page. On the terminal, row heights round to whole cells.

Key options:

- `.row_height(len)`: fixed row height. `len` is pixels or a text unit such as `rows(1.0)`.
- `.item_height(Fn(usize, &T) -> f32)`: variable row heights; the scroll model uses measured tops.
- `.viewport_height(len)` / `.viewport_width(px)`: size the window. `.fill()`, `.fill_height()`, or a percent height grows to the parent instead of the fixed height.
- `.overscan(pages)`: extra viewport-heights rendered above and below.
- `.key_by(Fn(usize, &T) -> u64)` / `.identified()`: stable row keys (`identified` requires `T: Identifiable`) so focus and selection survive reordering.
- `.measured()`: each row lays out at the list width and reports its height. The heights go into one cache keyed by the row key (or the index without `key_by`). A row that has not laid out yet uses `.estimate(len)` (default: the row height). The cache clears when the row width changes, and drops the keys of removed rows when the items change. `follow_row` and `follow_end` keep the tail pinned while rows grow and measurements replace estimates. When the list is scrolled up, a height change above the first visible row moves the scroll offset by the same amount, so the visible rows stay still. The host lays out again in the same frame when a measured size changes, so a new height never paints at the old offset.
- `.selection(SelectionMode)`: `None`, `Single`, `Multi`, `Extend`. `.selection_state(&mut self) -> Signal<Selection>` exposes the state object (`contains`, `indices`, `select`, `toggle`, `extend`, `clear`); `.change(Fn(usize))` is a convenience effect over the first selected index.
- `.scroll_y(Signal<f32>)` / `.scroll_x(scroll_x, max, width)`: bind scroll to external signals.
- `.cursor(Signal<usize>)`: the keyboard-focused row. Arrow keys write it, and the list keeps that row in view unless `.follow_row` names another target.
- `.content_height(Signal<f32>)` / `.max_scroll(Signal<f32>)`: the list writes its content height and its largest scroll offset (content height minus the viewport) to these signals. Both use the same scroll model as the list: measured tops, variable heights, or fixed rows. To page from outside the list, write `scroll_y` clamped to `0.0..=max_scroll`. To jump to the tail, write `max_scroll`.
- `.initial_scroll(anchor)` (`ScrollAnchor::Top`, `Bottom`, or a row index): initial position.
- `.follow_end(bool)`: start at the end and keep the view pinned to the last row while rows grow, until the user scrolls up (chat/log behavior).
- `.follow_row(Signal<usize>)`: an effect that scrolls the target row into view whenever it changes. It scrolls the least whole rows that show the row, and uses the measured viewport with `.fill()`. When the content grows, the list follows only if the view was at the bottom and the target is the last row; a list that opens empty and fills later stays at the target row. If the user scrolls up, the view stays, also when a new row moves the target. The list follows again after the view returns to the bottom. A target write with no new rows always scrolls to the target row, so `target.update(|row| *row = last)` jumps to the tail.
- `.end_reached(rows, Fn())`: runs once each time the user scrolls within `rows` rows of the end, for load-more paging. It fires again after the list grows.
- `.keyboard(bool)`: keyboard is on by default: Arrow Up/Down move the focused row, PageUp/PageDown move a page, Home/End jump to the ends. A page uses the row tops, so `.measured()` lists page by the measured heights. The focused list draws the shared focus ring.

## table

`table(TableState<Row>)`. The body is a virtual list, so large row sets stay cheap; the header is rendered once.

```rust,no_run
# use sigil::prelude::*;
# struct Row { name: String, size: u64 }
# let rows = vec![Row { name: "a.txt".into(), size: 4096 }];
# let _ = {
let state = TableState::new(
    rows,
    vec![
        TableColumn::new("name", "Name")
            .sort_by(|r: &Row| r.name.clone())
            .filterable(|r, q| r.name.contains(q))
            .cell(|r| text(r.name.clone()).nowrap()),
        TableColumn::new("size", "Size").numeric().cell(|r: &Row| text(format!("{} B", r.size)).nowrap()),
    ],
);
table(state).striped().fill_height();
# };
```

`TableColumn::new(id: &'static str, title)`:

- `.width(ColumnWidth::Fixed(px) | ColumnWidth::Fraction(n))`: columns are `Fraction(1.0)` by default; `.min_width(px)` floors at 48 px.
- `.align(ColumnAlign::End)` / `.numeric()`: `numeric` right-aligns and flags the column.
- `.sortable(Fn(&Row, &Row) -> Ordering)` / `.sort_by(key)`: clicking the header cycles ascending/descending/off via `cycle_sort`.
- `.filterable(Fn(&Row, &str) -> bool)`: participates in the quick filter.
- `.cell(Fn(&Row) -> impl IntoNode)`: the cell content; a column without it renders an empty `Role::GridCell`.
- `.editable(text, commit)`: edits the cell in place. `text(&Row) -> String` fills the editor and `commit(&mut Row, String)` writes the result into the table's rows. Double-click or Enter on a row starts, Enter commits, Escape cancels, and Tab and Shift-Tab move between editable cells. With `TableState::identified`, an open edit follows its row when the app calls `replace_rows`.

`TableState<Row>` is the live model:

- `TableState::new(rows, columns)`: index-based selection.
- `TableState::identified(rows, columns)`: selection tracks `Identifiable::id` (`Clone + Eq + Hash`; implemented for `Arc<Row>` too), so `replace_rows` keeps the same logical rows selected.
- `replace_rows(Into<Arc<Vec<Row>>>)`, `columns()`, `visible() -> Signal<Rc<Vec<u32>>>` (the sorted+filtered index list), `selected_index()`, `selected_row()`, `selection() -> Signal<Rc<HashSet<u32>>>`.
- Selection writes: `select(row)`, `toggle(row)`, `extend(row)`, `clear()`.
- `sort_key() -> Signal<Option<SortKey>>`, `cycle_sort(id)`, `sort(id, SortDirection)`; `quick_filter() -> Signal<String>` drives the `.filterable` predicates.
- Column chrome: `visible_columns()`, `move_column(from, to)`, `hide_column(id)`/`show_column(id)`/`show_all_columns()`, `pin_column_width(id, px)`/`clear_column_width(id)`, `reset_columns()`, `column_width(id)`.

`table` options: `.row_height(len)`, `.header_height(px)`, `.viewport_height(len)`, `.viewport_width(px)`, `.fill_height()` (takes the height of the parent), `.density(Density)` (sets row+header height and padding together: `Grid` 24/28 px with 8 px padding, `Compact` 28/32 px, `Default` 36/36 px), `.striped()`, `.scroll_y(Signal<f32>)`, `.row_menu(Fn(&Row) -> Vec<MenuItem>)` (context menu), `.empty(Fn() -> NodeBuilder)` (shown when `visible` is empty), `.selectable_checkboxes()` (adds a selection column), `.end_reached(rows, Fn())` (as on `virtual_list`).

The header uses `surface-inset`, a strong bottom border, and extra-small medium-weight `on-surface-muted` titles, so it reads apart from the regular `on-surface` body text. The sort arrow uses the same size and color as the title.

## log_view

`log_view(Log)`. A `virtual_list` of mono text lines that follows the tail until the user scrolls up. `Log::new(max_lines)` keeps the newest lines; `push(text)` splits at line breaks, `clear()` empties it, and `len()` counts the lines. `Log` is `Copy`, so a task can hold it and push while the view shows it.

## heatmap

`heatmap(grid_w, grid_h, color) -> NodeBuilder` where `color` is `Fn(usize, usize) -> Color`. Eagerly builds `grid_w × grid_h` fixed 8 px cells in nested flex rows; the color closure runs once at build time. Not virtualized and not reactive. Keep grids small, or rebuild the node when the data changes.

## Rows and key-value pairs

The row helpers live in [Chrome](chrome.md#small-composites) and fill a data view:

- `list_row(title)` is the usual `virtual_list` row.
- `kv_row(label, value)` and `kv_grid([(key, value)])` show fixed key-value facts.

## States, keys, and accessibility

The visual catalog captures `table` in default, hover, and pressed. It captures `virtual_list` and `list_row` in default, hover, pressed, and selected. `kv_row`, `kv_grid`, and `heatmap` are static.

| Widget | Keys | Role | Name source |
| --- | --- | --- | --- |
| `virtual_list` | Arrow Up and Down, PageUp, PageDown, Home, End (`.keyboard(false)` turns them off) | `ScrollView` | Row content |
| `table` | As `virtual_list` in the body. Enter or Space on a header cell toggles the sort. | `Grid`, `Row`, `ColumnHeader`, `GridCell` | Column titles and cell content |
| `table` with `.selectable_checkboxes()` | Space on the checkbox cell | `CheckBox` in a `GridCell` | The row |
| `heatmap` | none | `Figure` | None |

A selected row keeps the `accent-5` wash. Hover and pressed rows show `surface-hover` and `surface-pressed`.

## Terminal notes

- `.row_height()` and `.viewport_height()` take pixels, not a count of text rows. The terminal host rounds a row height to whole cells, and a row height below one cell still takes one cell. Prefer `.fill()` in terminal layouts so the list takes the height that its parent gives it.
- `.follow_row(Signal<usize>)` and `.fill()` use the measured viewport, so they follow the terminal size.
- The wheel scrolls three lines per notch.
- A heatmap cell of 8 px is smaller than a cell, so a heatmap paints one colored cell per data cell at best. Keep terminal heatmaps small.

## Example

Run it with `cargo run -p sigil-ui --example widget_data`.

```rust,ignore
{{#include ../../../crates/sigil/examples/widget_data.rs}}
```
