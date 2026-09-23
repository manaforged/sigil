# Widgets

The catalog ships in `sigil`. Every widget constructs from a lowercase
function and chains a builder. Widgets take `class`/`style` styling —
through `NodeStyleExt` for `NodeBuilder`/`TextBuilder` returns, through
inherent builder methods, or through `StyleAccess`. `WidgetLayoutExt`
sizing applies where the builder implements it (containers and most
controls); `virtual_list`, `table`, `slider`, and `code_edit` size via
`.viewport_height()`/`.row_height()` or `.layout()` instead. Widgets
join focus, accessibility, and IME where relevant. See the rustdoc for
the full builder surface.

The family pages carry signatures, options, and behavior notes:

- [Controls](widgets/controls.md) — button, icon_button, checkbox,
  toggle, radio_group, slider.
- [Indicators](widgets/indicators.md) — chip, badge, dot, gauge,
  progress.
- [Fields](widgets/fields.md) — text_field, text_edit, number_field,
  date_field, color_field, combobox, dropdown.
- [Data](widgets/data.md) — table, virtual_list, turn_strip, heatmap.
- [Content](widgets/content.md) — text, label, icon, image, path,
  chart.
- [Documents](widgets/documents.md) — markdown, markdown_edit, html,
  code_edit, mermaid, diff.
- [Chrome](widgets/chrome.md) — app_shell, panel, card, sidebar, tabs,
  splitter, scroll, dividers, surface, status_bar, and the `patterns`
  helpers.
- [Overlays](widgets/overlays.md) — modal, sheet, menu, menu_bar,
  command_menu, tooltip, toast, toast_queue, hint_overlay.
- [Plumbing](widgets/plumbing.md) — interactive, axis_drag, drag/drop,
  clipboard, terminal, transport, floating_text.

## Inputs

- **button(label)** — `label` is `TextContent` (`&str` or
  `Signal<String>`). Chain `.primary()`, `.neutral()`, `.destructive()`,
  `.tone(Tone)`, `.solid()`, `.soft()`, `.disabled(Signal)`, and
  `.activate(Fn())`. State closures are `.hover()`, `.focus()`,
  `.pressed()`, and `.disabled_style()`. Use `.accessible("Play")` when
  the visible label is a glyph.
- **icon_button(glyph, activate)** — a ghost `button` whose label is a
  Unicode glyph. Chain `.accessible("Play")` for an assistive name.
- **button(...).pending(Signal\<bool\>)** — paints disabled and ignores
  activate while the signal is true.
- **checkbox(Signal\<bool\>)** — chain `.label()`, `.tone(Tone)`,
  `.change(Fn(bool))`, and `.disabled(Signal)`. Neutral is gray; every
  other tone uses the accent checked fill.
- **toggle(Signal\<bool\>)** — chain `.label()` and `.change(Fn(bool))`.
- **slider(Signal\<f32\>, Range)** — chain `.step(f32)`,
  `.change(Fn(f32))`, and an orientation. The slider publishes its step
  for AT Increment/Decrement.
- **text_field(Signal\<String\>)** — chain `.placeholder()`,
  `.change(Fn(&str))`, `.submit()`, `.clipboard()`, `.disabled(Signal)`,
  `.invalid_when(Signal\<bool\>)`, `.password()`, and
  `.reveal(Signal<bool>)`. `.change` is a side effect; the signal is
  already two-way. `.password` masks glyphs with `*`; the value stays
  plaintext. `.reveal` shows the plaintext while true. IME preedit is
  inline, and the caret anchor is published. Pass a `Signal<String>` for
  the field value.
- **text_edit(Signal\<String\>)** — multi-line: the same surface plus
  newline entry, a vertical caret (Up/Down), and shift-selection.
- **number_field(Signal\<T\>, Range)** — a numeric text field.
- **date_field(Signal\<Date\>)** — ISO `YYYY-MM-DD`.
- **color_field(Signal\<Rgba8\>)** — hex (`#rgb` or `#rrggbb`) plus a
  swatch.
- **dropdown(Signal\<usize\>, Vec\<String\>)** — chain
  `.change(Fn(usize))` and `.disabled(Signal)`. It is a trigger plus a
  positioned popup; focus stays on the trigger.
- **radio_group(Signal\<String\>)** — chain `.option(key, label)`. Arrow
  keys wrap, Home and End jump, and the focus ring sits on the checked
  row.
- **command_menu()** — a nested action tree: `.item(label, action)`,
  `.submenu(...)`, and `.select(Fn(&str))`. It is not a searchable
  Cmd-K palette. Build that shape from `text_field` plus `virtual_list`
  inside `modal` or `sheet` (recipe below).

## Data and display

- **table(TableState\<Row\>)** — the data tier: sticky sortable header,
  virtualized body (100k rows), selection styling, hover, keyboard
  navigation, and AccessKit grid roles. Build columns with
  `TableColumn::new(id, title)` plus `.sort_by(|row| row.name.clone())`,
  or `.sortable` for a custom `Ordering`. `TableState::sort(id, dir)`
  applies a sort; `sort_key()` is the signal; `toggle_sort` cycles the
  header. Chain `.filterable()`, `.cell(|row| text(...))` (any
  `IntoNode`), `.width(ColWidth)`, and `.min_width(px)`. Implement
  `Identifiable` on the row and construct with
  `TableState::identified(rows, cols)` so `replace_rows` keeps the
  clicked row. `TableState::new` is index-only. `selected_row()` and
  `selected_index()` subscribe with `get`. `.density(Density::Compact)`
  gives 28 px rows. An empty filter shows "No rows"; override it with
  `.empty(|| ...)`.
- **diff(patch)** — a unified-diff viewer. Pass the patch text; the
  widget parses hunks itself. `.layout(DiffLayout::Split)` shows before
  and after side by side with per-line numbers and intraline
  highlighting of the changed range; `DiffLayout::Unified` stacks the
  removed rows above the added rows. Chain `.path(name)` for the header,
  `.status(text)` for the footer, `.max_rows(n)` to cap a long patch,
  and `.choose(|choice| ...)` to make the split/unified control
  switch the view.
- **chart(values)** — a bar chart by default; `.line()` draws a
  polyline. Chain `.tone(Tone)`, `.labels(["Mon", ...])`, and
  `.series(name, values, Tone)` for more marks. The color is theme
  scale step 9. `.width` and `.height` pin the plot.
- **virtual_list(items, render)** — a windowed list. `items` is a `Vec`
  (snapshot) or a `Signal` (live). Default keys are list indices.
  `.identified()` keys by `Identifiable::id`. `.key_by` takes any other
  domain key. `.auto_pin_to_bottom(true)` keeps a live list on the last
  row when it grows.
- **progress(Signal\<f32\>, max)** and **gauge(Signal\<f32\>)**.
- **chip(label, Tone)** — the same `Tone` as buttons.
  `.tone_signal(...)` retints reactively. `.activate(Fn())` makes it a
  button; without `.activate` it stays a label.
- **badge(label, Tone)** — a pill chip (`rounded-full`).
- **field(label, control)** and **validated_field** — a label above a
  control. Use these for forms.
- **icon(glyph)**, **dot(Tone)**, **divider_horizontal()**, and
  **divider_vertical()**.
- **code_edit(...)**, **floating_text(...)** (game FCT), and
  **turn_strip(...)** (game turn order).
- **markdown(Signal\<String\>)** — a read-only document: headings,
  paragraphs, lists, fenced code, pipe tables, plus `**bold**`,
  `` `code` ``, and `[label](url)`. Not an in-place editor.
- **html(Signal\<String\>)** — the same nodes as `markdown`, from an
  HTML subset (`h1`-`h3`, `p`, `ul`/`ol`/`li`, `pre`/`code`, `a`,
  `strong`/`em`, `table`). Scripts and styles are dropped. Not a
  browser.
- **markdown_edit(Signal\<String\>)** — a source `text_edit` plus a live
  `markdown` preview. The caret stays in the source string.
- **transport(playing, progress, toggle)** — a play/pause `icon_button`
  plus a 0-1 `slider`. Bind a prelude `Player` (feature `audio`). Open a
  file with `Player::play_path`. Chain `.seek(|r| ...)` with
  `Player::seek` so a drag jumps the file.
- **terminal(lines, on_input)** — a mono `virtual_list` of PTY rows,
  pinned to the bottom. Bind a prelude `Session` (feature `pty`). Paint
  is `Session.lines`. `Session::write` needs `Session::spawn`.
- **surface(key)** — a layout hole for a native view. After layout,
  `Host::surfaces` returns the rect. The GPU does not paint it.

## Containers and overlays

- **card(child)** — a padded surface.
- **avatar(initials)** — an initials disc.
- **sidebar(child)** and **sidebar_width(px, child)** — a fixed-width
  nav column.
- **panel()** — chain `.title()`, `.padding()`, and `.elevation()`.
- **splitter(Orientation)** — an `IntoNode`. Chain
  `.pane(node).initial(px).min(px).grow(1.0)`. A grow pane takes the
  leftover space; the sash moves the fixed neighbour. `.width()` and
  `.height()` size the splitter. Type a helper as `impl IntoNode`;
  `.into()` yields a `NodeBuilder`.
- **modal(open)** — a GPU dialog. `open` is a `Signal<bool>`. `.title()`,
  `.width()`, and `.height()` size the frame. Escape and the focus trap
  are on. A backdrop click is off unless `.dismiss_on_backdrop()`. The
  native alert is `sigil::dialog::message` / `confirm` (feature `dialog`).
- **sheet(open)** — the same builder, docked. The default is
  `.side(Side::Right)`. Use `.side(Side::Left)` or
  `.side(Side::Bottom)`. A backdrop click dismisses.
- **toast(message)** — a `Signal<Option<String>>`. `Some` shows a status
  line; `None` hides it.
- **tabs(Signal\<usize\>, Vec\<String\>)** — chain `.content_for(i, node)`
  and `.close(Fn(usize))`.
- **scroll(inner, extent)** — chain `.width()` and `.height()`.
- **menu_bar()** — a GPU app bar. `.class("bg-surface-hover px-3")`
  styles the strip. `.menu("File", [MenuItem::new("Open").select(|| {})])`
  builds a dropdown. This is the bar you design. The native OS bar is
  opt-in: `sigil::menu::set_menu_bar` (feature `menu`, next frame).
- **menu(Signal\<bool\>)** — a GPU popup panel (`.item`, `.separator()`).
  Use it for context menus, not the app bar.
- **tooltip(label)** — chain `.shortcut()`; it wraps a target child.

`field(label, control)` lives with the widgets. `patterns::` still has
`app_shell`, `kv_grid`, and `hint_overlay`.

## Searchable palette (Cmd-K)

`command_menu` is nested actions. A filter-as-you-type palette is:

```rust,ignore
let open = signal(false);
let query = signal(String::new());
let hits = derived(move || {
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
        .child(virtual_list(hits, |_i, row| text(row.clone()))),
);
```

## Interaction state support

| Surface | State closures paint? |
|---|---|
| button, icon_button | yes (all four slots) |
| checkbox, dropdown, slider, tabs, text_field, text_edit | yes |
| Plain containers, card, avatar, sidebar | via `interactive()` only |
| chip | no style slots; `.activate` is a clickable button |
| gauge, progress, dot, floating_text, turn_strip, icon, divider, panel, scroll, modal, menu, command_menu, tooltip | compile error by design (display-only) |
