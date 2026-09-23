# Data

Lists and grids that present collections. `virtual_list` is the
virtualized primitive; `table` builds a sortable, filterable,
column-managed grid on top of it.

## virtual_list

`virtual_list(items, render) -> VirtualListBuilder<T, R>` where
`render` is `Fn(usize, &T) -> NodeBuilder` and `items` is anything
implementing `IntoList` — a `Vec<T>`, or a `Signal<S>` whose target is
`AsRef<[T]>` (`Signal<Vec<T>>`, `Signal<Arc<Vec<T>>>`, …). Bound signals
re-derive the list when the length changes.

```rust,ignore
virtual_list(rows, |index, row| {
    list_row(index == 3, row.name.clone(), row.detail.clone())
})
.selection(SelectionMode::Single)
.viewport_height(400.0);
```

Only the rows inside the viewport plus an overscan margin are
materialized; scrolling reuses a fixed window. Defaults: row height
32 px, viewport height 480 px, overscan 1 page.

Key options:

- `.row_height(px)` — fixed row height.
- `.item_height(Fn(usize, &T) -> f32)` — variable row heights; the
  scroll model uses measured tops.
- `.viewport_height(px)` / `.viewport_width(px)` / `.viewport_fill()` —
  size the window; `viewport_fill` grows to the parent instead of the
  fixed height.
- `.overscan(pages)` — extra viewport-heights rendered above and below.
- `.key_by(Fn(usize, &T) -> u64)` / `.identified()` — stable row keys
  (`identified` requires `T: Identifiable`) so focus and selection
  survive reordering.
- `.selection(SelectionMode)` — `None`, `Single`, `Multi`, `Extend`.
  `.selection_state(&mut self) -> Signal<SelectionState>` exposes the
  state object (`contains`, `selected_indices`, `click`,
  `ctrl_click`, `shift_click`); `.select(Fn(usize))` is a convenience
  effect over the first selected index.
- `.scroll_bind(Signal<f32>)` / `.scroll_x_bind(scroll_x, max, width)`
  / `.focused_row_bind(Signal<usize>)` — bind scroll and focus to
  external signals.
- `.scroll_into_view(index)`, `.initial_scroll(ScrollAnchor)`
  (`Top`/`Bottom`/`Index`), `.scroll_to_end()` — initial position.
- `.auto_pin_to_bottom(bool)` — keep the view pinned to the last row
  while rows grow (chat/log behavior).
- `.follow(Signal<usize>)` — an effect that scrolls the target row
  into view whenever it changes.
- `.keyboard(bool)` — keyboard is on by default: Arrow Up/Down move
  the focused row, PageUp/PageDown move a page, Home/End jump to the
  ends.

## table

`table(TableState<Row>) -> TableBuilder<Row>`. The body is a virtual
list, so large row sets stay cheap; the header is rendered once.

```rust,ignore
let state = TableState::new(
    rows,
    vec![
        TableColumn::new("name", "Name")
            .sort_by(|r| r.name.clone())
            .filterable(|r, q| r.name.contains(q))
            .cell(|r| label(r.name.clone())),
        TableColumn::new("size", "Size").numeric().cell(|r| label(fmt_size(r))),
    ],
);
table(state).striped().viewport_fill();
```

`TableColumn::new(id: &'static str, title)`:

- `.width(ColWidth::Fixed(px) | ColWidth::Fraction(n))` — columns are
  `Fraction(1.0)` by default; `.min_width(px)` floors at 48 px.
- `.align(ColAlign::End)` / `.numeric()` — `numeric` right-aligns and
  flags the column.
- `.sortable(Fn(&Row, &Row) -> Ordering)` / `.sort_by(key)` — clicking
  the header cycles ascending/descending/off via `toggle_sort`.
- `.filterable(Fn(&Row, &str) -> bool)` — participates in the quick
  filter.
- `.cell(Fn(&Row) -> impl IntoNode)` — the cell content; a column
  without it renders an empty `Role::GridCell`.

`TableState<Row>` is the live model:

- `TableState::new(rows, columns)` — index-based selection.
- `TableState::identified(rows, columns)` — selection tracks
  `Identifiable::id` (`Clone + Eq + Hash`; implemented for `Arc<Row>`
  too), so `replace_rows` keeps the same logical rows selected.
- `replace_rows(Into<Arc<Vec<Row>>>)`, `columns()`,
  `visible() -> Signal<Rc<Vec<u32>>>` (the sorted+filtered index list),
  `selected_index()`, `selected_row()`,
  `selection() -> Signal<Rc<HashSet<u32>>>`.
- Selection writes: `select_only(row)`, `toggle_selected(row)`,
  `select_range_to(row)`, `clear_selection()`.
- `sort_key() -> Signal<Option<SortKey>>`, `toggle_sort(id)`,
  `sort(id, SortDir)`; `quick_filter() -> Signal<String>` drives the
  `.filterable` predicates.
- Column chrome: `visible_columns()`, `move_column(from, to)`,
  `hide_column(id)`/`show_column(id)`/`show_all_columns()`,
  `pin_column_width(id, px)`/`clear_column_width(id)`,
  `reset_columns()`, `column_width(id)`,
  `column_layout() -> Signal<Rc<ColumnLayout>>`.

`TableBuilder` options: `.row_height(px)`, `.header_height(px)`,
`.viewport_height(px)`, `.viewport_width(px)`, `.viewport_fill()`,
`.density(Density)` (sets row+header height and padding together),
`.striped()`, `.scroll_bind(Signal<f32>)`,
`.row_menu(Fn(&Row) -> Vec<MenuItem>)` (context menu),
`.empty(Fn() -> NodeBuilder)` (shown when `visible` is empty),
`.selectable_checkboxes()` (adds a selection column).

## turn_strip

`turn_strip(Signal<Vec<String>>) -> TurnStripBuilder`. A horizontal
row of neutral chips rebuilt from the entries signal; `.active(Signal
<usize>)` applies the active class and focus ring to the matching chip.
Meant for short strips — rounds in an encounter, steps in a wizard —
not for large collections.

## heatmap

`heatmap(grid_w, grid_h, color) -> NodeBuilder` where `color` is
`Fn(usize, usize) -> Rgba8`. Eagerly builds `grid_w × grid_h` fixed
8 px cells in nested flex rows; the color closure runs once at build
time. Not virtualized and not reactive — keep grids small, or rebuild
the node when the data changes.
