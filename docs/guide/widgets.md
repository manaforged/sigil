# Widgets

The catalog ships in `sigil`. Every widget constructs from a lowercase function and chains a builder. Widgets take typed styling: `Paint` methods for color, border, radius, shadow, and font, and `States` methods (`.rest()`, `.hover()`, `.focus()`, `.pressed()`, `.disabled_paint()`) for interaction states on builders that support them. Every builder takes the `Widget` layout, event, and accessibility setters, such as `.width()`, `.height()`, `.grow()`, and `.padding()`; `virtual_list` and `table` also take `.viewport_height()` and `.row_height()`. Focusable widgets publish accessibility roles and accept IME input. The rustdoc lists every builder method.

The family pages carry signatures, options, and behavior notes:

- [Controls](widgets/controls.md): button, checkbox, toggle, radio_group, slider, range_slider, link.
- [Indicators](widgets/indicators.md): chip, dot, spinner, gauge, progress.
- [Fields](widgets/fields.md): text_field, number_field, date_field, time_field, color_field, tag_input, combobox, dropdown.
- [Data](widgets/data.md): table, virtual_list, log_view, heatmap.
- [Content](widgets/content.md): text, label, icon, image, load_image, path, chart, stat, sparkline, canvas.
- [Documents](widgets/documents.md): markdown, markdown_edit, html, code_edit, mermaid, diff.
- [Chrome](widgets/chrome.md): app_shell, card, sidebar, tabs, segmented, tree, splitter, scroll, dividers, surface, status_bar, context_menu, and helpers including badge, skeleton, pagination, breadcrumbs, and stepper.
- [Overlays](widgets/overlays.md): modal, menu, menu_bar, command_menu, tooltip, toast, toast_queue, hint_overlay.
- [Plumbing](widgets/plumbing.md): interactive, axis_drag, drag/drop, clipboard, terminal.

## Inputs

- **button(label)**: `label` is a `Glyph`: text (`&str` or `Signal<String>`) or an `Icon`. Chain `.primary()`, `.neutral()`, `.destructive()`, `.tone(Tone)`, `.solid()`, `.soft()`, `.disabled(Signal)`, and `.activate(Fn())`. State closures are `.hover()`, `.focus()`, `.pressed()`, and `.disabled_paint()`. Use `.accessible("Play")` when the visible label is a glyph.
- **button(Icon::Play)**: a square ghost `button` that shows an icon and is named for it. For a text glyph, write `button("▶").ghost().square().accessible("Play")`.
- **button(...).pending(Signal\<bool\>)**: paints disabled and ignores activate while the value is true. It also takes a `memo` or a closure.
- **checkbox(Signal\<bool\>)**: chain `.label()`, `.tone(Tone)`, `.change(Fn(bool))`, and `.disabled(Signal)`. Neutral is gray; every other tone uses the accent checked fill.
- **toggle(Signal\<bool\>)**: chain `.label()` and `.change(Fn(bool))`.
- **slider(Signal\<f32\>, Range)**: chain `.step(f32)`, `.change(Fn(f32))`, and an orientation. The slider publishes its step for AT Increment/Decrement.
- **text_field(Signal\<String\>)**: chain `.placeholder()`, `.change(Fn(&str))`, `.submit()`, `.clipboard()`, `.disabled(Signal)`, `.invalid(Signal\<bool\>)`, `.password()`, and `.reveal(Signal<bool>)`. `.change` is a side effect; the signal is already two-way. `.password` masks glyphs with `*`; the value stays plaintext. `.reveal` shows the plaintext while true. IME preedit is inline, and the caret anchor is published. Pass a `Signal<String>` for the field value.
- **text_field(input)**: chain `.multiline()` for newline entry, a vertical caret (Up/Down), and shift-selection; or pass a `TextState` directly.
- **number_field(Signal\<T\>, Range)**: a numeric text field.
- **date_field(Signal\<Date\>)**: ISO `YYYY-MM-DD`.
- **color_field(Signal\<Color\>)**: hex (`#rgb` or `#rrggbb`) plus a swatch.
- **dropdown(Signal\<usize\>, options)**: `options` is any `IntoIterator` of strings (such as `["Small", "Large"]`). Chain `.change(Fn(usize))` and `.disabled(Signal)`. It is a trigger plus a positioned popup; focus stays on the trigger.
- **segmented(Signal\<usize\>, labels)**: a framed row of mutually exclusive options where one is active; `labels` is any `IntoIterator` of strings. Chain `.label(name)`.
- **radio_group(Signal\<usize\>, options)**: `options` is any `IntoIterator` of strings, as in `dropdown`. Chain `.accessible(name)` to name the group. Arrow keys wrap, Home and End jump, and the focus ring sits on the checked row.
- **command_menu(Signal\<bool\>)**: a nested action tree shown over the app while the signal is true: `.item(MenuItem::new(label).activate(action))`, `.submenu(...)`, and `.submit(Fn(&str))`. Choosing an item, Escape at the top level, or a backdrop press sets the signal to false. Typing filters and ranks the items of the open level with `fuzzy_score`. For a palette with custom rows, compose `text_field` and `virtual_list` inside `modal` (recipe below).

## Data and display

- **table(TableState\<Row\>)**: the data tier: sticky sortable header, virtualized body (100k rows), selection styling, hover, keyboard navigation, and AccessKit grid roles. Build columns with `TableColumn::new(id, title)` plus `.sort_by(|row| row.name.clone())`, or `.sortable` for a custom `Ordering`. `TableState::sort(id, dir)` applies a sort; `sort_key()` is the signal; `cycle_sort` cycles the header. Chain `.filterable()`, `.cell(|row| text(...))` (any `IntoNode`), `.width(ColumnWidth)`, and `.min_width(px)`. Implement `Identifiable` on the row and construct with `TableState::identified(rows, cols)` so `replace_rows` keeps the clicked row. `TableState::new` is index-only. `selected_row()` and `selected_index()` subscribe with `get`. `.density(Density::Compact)` gives 28 px rows. An empty filter shows "No rows"; override it with `.empty(|| ...)`.
- **diff(patch)**: a unified-diff viewer. Pass the patch text; the widget parses hunks itself. `.view(DiffView::Split)` shows before and after side by side with per-line numbers and intraline highlighting of the changed range; `DiffView::Unified` stacks the removed rows above the added rows. Chain `.path(name)` for the header, `.status(text)` for the footer, and `.max_rows(n)` to cap a long patch.
- **chart(values)**: a bar chart by default; `.line()` draws a polyline. Chain `.tone(Tone)`, `.labels(["Mon", ...])`, and `.series(name, values, Tone)` for more marks. The color is theme scale step 9. `.width` and `.height` pin the plot.
- **stat(label, value)**: a KPI tile in card chrome with optional `.delta(change, tone)`, `.caption(text)`, `.trend(values)`, `.range(low, high)`, and `.tone(Tone)`.
- **sparkline(values)**: a compact inline trend graph. Chain `.mark(SparkMark::Block | SparkMark::Braille)`, `.fill(SparkFill::Solid | SparkFill::Shade)`, `.range(low, high)`, `.tone(Tone)`, `.tones(iter)`, and `.rows(n)`.
- **canvas(record)**: a custom drawing surface whose closure receives `(&mut SceneBuilder, Rect, &Theme)`. Chain `.redraw_on(Signal<u64>)`. `SceneBuilder` is in `sigil::scene`.
- **virtual_list(items, render)**: a windowed list. `items` is a `Vec` (snapshot) or a `Signal` (live). Default keys are list indices. `.identified()` keys by `Identifiable::id`. `.key_by` takes any other domain key. `.follow_end(true)` keeps a live list on the last row when it grows.
- **progress(Signal\<f32\>)** and **gauge(Signal\<f32\>)**. A progress bar is full at 1.0; chain `.max(m)` to change that, and `.label(text)` for a visible caption that is also the accessible name.
- **chip(label, Tone)**: the same `Tone` as buttons. `.tone(..)` also takes a signal or closure. `.activate(Fn())` makes it a button; without `.activate` it stays a label.
- **field(label, control)**: a label above a control; `.error(..)` adds an error line. Use it for forms.
- **icon(glyph)**, **dot(Tone)**, and **divider(Orientation)**.
- **code_edit(lines, caret)**: a line-oriented code editor that renders only the visible rows. See [Documents](widgets/documents.md#code_edit).
- **markdown(Signal\<String\>)**: a read-only document: headings, paragraphs, lists, fenced code, pipe tables, plus `**bold**`, `` `code` ``, and `[label](url)`. Not an in-place editor.
- **html(Signal\<String\>)**: the same nodes as `markdown`, from an HTML subset (`h1`-`h3`, `p`, `ul`/`ol`/`li`, `pre`/`code`, `a`, `strong`/`em`, `table`). Scripts and styles are dropped. Not a browser.
- **markdown_edit(Signal\<String\>)**: a source `text_field(..).multiline()` plus a live `markdown` preview. The caret stays in the source string.
- **terminal(lines, on_input)**: a mono `virtual_list` of PTY rows, pinned to the bottom. Bind a `sigil::app::Session` (feature `pty`). Paint is `Session.lines`. `Session::write` needs `Session::spawn` or `Session::spawn_in`. A focused terminal receives every key except command-key shortcuts.
- **surface(key)**: a layout hole for a native view. After layout, `Host::surfaces` returns the rect. The GPU does not paint it.

## Containers and overlays

- **card(child)**: a padded surface. Chain `.title()` and `.layer(Layer)`.
- **avatar(initials)**: an initials disc.
- **sidebar(child)**: a fixed-width nav column. Chain `.width(len)` and `.resizable(min, max)`.
- **tree(nodes)**: a hierarchical list of `TreeNode`s with `.expanded(Signal<Vec<K>>)`, `.selected(Signal<Option<K>>)`, and `.activate(Fn(&K))`.
- **context_menu(target, items_fn)**: wraps `target` and opens a menu of `items_fn()` at the pointer on right-click.
- **splitter(Orientation)**: an `IntoNode`. Chain `.pane(node).width(px).min_width(px)` on a horizontal splitter, or `.height(px).min_height(px)` on a vertical one, and `.grow(1.0)` for the pane that takes the leftover space; the sash moves the fixed neighbour. Every setter after `.pane(node)` sets that pane; call `.commit()` first to set the splitter. Type a helper as `impl IntoNode`; `.into()` yields a `NodeBuilder`.
- **modal(open)**: a GPU dialog. `open` is a `Signal<bool>`. `.title()`, `.width()`, and `.height()` size the frame. Escape and the focus trap are on. A backdrop click is off unless `.dismiss_on_backdrop()`. The alert is `sigil::app::dialog::message` / `confirm` (feature `dialog`). File and message dialogs return a `Signal<Status<..>>` and never block the frame loop. Feature `dialog` shows an in-app picker on every host; add `native-dialog` for the OS dialog on the window host. Read the result like any spawned task:

  ```rust,ignore
  let picked = sigil::app::dialog::FileDialog::new().filter("CSV", &["csv"]).open_file();
  text(move || picked.with(|status| match status {
      Status::Ok(Some(path)) => path.display().to_string(),
      Status::Ok(None) => String::from("Cancelled"),
      _ => String::from("Choosing"),
  }))
  ```
- **modal(open).side(Side::Right)**: the same builder, docked to an edge. Use `Side::Left` or `Side::Bottom` for the other edges, and `.dismiss_on_backdrop(true)` to close on a backdrop click.
- **toast(message)**: a `Signal<Option<String>>`. `Some` shows a status line; `None` hides it.
- **tabs()**: chain `.tab(label, component)` per tab, and `.close(Fn(usize))`.
- **scroll(inner)**: measures its content; `.extent(px)` sets a known height. Chain `.width()` and `.height()`.
- **menu_bar()**: a GPU app bar. `Paint` methods such as `.background(Token::SurfaceHover)` style the strip. `.menu("File", [MenuItem::new("Open").activate(|| {})])` builds a dropdown. This is the bar you design. The native OS bar is opt-in: `sigil::app::menu::Menu::install` (feature `menu`, next frame).
- **menu(Signal\<bool\>)**: a GPU popup panel (`.item`, `.separator()`). Use it for context menus, not the app bar.
- **tooltip(label)**: chain `.hint()`; it wraps a target child.

`field(label, control)`, `app_shell`, `kv_grid`, and `hint_overlay` are exported from `sigil::prelude` and `sigil::widgets`. Builder types and option types, such as `ButtonBuilder`, `ColumnWidth`, `Density`, `DiffView`, and `SparkMark`, are in `sigil::widgets` only.

To write your own widget with the same setters, see [Custom widgets](recipes.md#custom-widgets).

## Searchable palette (Cmd-K)

`command_menu(open)` filters its own items as you type. For a palette with custom rows, compose a `text_field` and a `virtual_list` inside `modal`:

```rust,no_run
# use sigil::prelude::*;
# const COMMANDS: &[&str] = &["open file", "save file"];
# let _ = {
let open = signal(false);
let query = signal(String::new());
let hits = memo(move || {
    let q = query.get();
    COMMANDS
        .iter()
        .filter(|row| row.contains(&q))
        .cloned()
        .collect::<Vec<_>>()
});
modal(open).title("Command").child(
    container()
        .child(text_field(query).placeholder("Filter…"))
        .child(virtual_list(hits, |_i, row| list_row(*row))),
);
# };
```

## Interaction state support

| Surface | State closures paint? |
|---|---|
| button | yes (all four slots) |
| checkbox, dropdown, slider, tabs, text_field | yes |
| Plain containers, avatar | yes |
| card, sidebar | on `.into_node_builder()` |
| chip | no style slots; `.activate` is a clickable button |
| gauge, progress, dot, icon, divider, scroll, modal, menu, command_menu, tooltip | compile error by design (display-only) |
