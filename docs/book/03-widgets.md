# The widget catalog

Every widget constructs from a lowercase fn and chains a builder. All of
them take `class`/`style`/fluent styling, `WidgetLayoutExt` sizing, and
participate in focus, a11y, and IME where relevant. Constructor
highlights below; see rustdoc for the full builder surface.

## Inputs

- **button(label)** — `.primary()/.neutral()/.destructive()`,
  `.tone(Tone)`, `.solid()/.soft()`, `.disabled(Signal)`,
  `.activate(Fn())`. Full state-closure support
  (`.hover()/.focus()/.pressed()/.disabled_style()`).
- **checkbox(Signal\<bool\>)** — `.label()`, `.tone(CheckboxTone)`,
  `.change(Fn(bool))`, `.disabled(Signal)`.
- **toggle(Signal\<bool\>)** — `.label()`, `.change(Fn(bool))`.
- **slider(Signal\<f32\>, Range)** — `.step(f32)`, `.change(Fn(f32))`,
  orientation; publishes step for AT Increment/Decrement.
- **text_field(Signal\<String\>)** — `.placeholder()`,
  `.change(Fn(&str))`, `.submit()`, `.clipboard()`, `.disabled(Signal)`,
  IME (preedit inline, caret anchor published).
- **text_edit(Signal\<String\>)** — multi-line: same surface plus
  newline entry, vertical caret (↑/↓), shift-selection.
- **number_field(Signal\<T\>, Range)** — numeric text field.
- **dropdown(Signal\<usize\>, Vec\<String\>)** — `.change(Fn(usize))`,
  `.disabled(Signal)`; trigger + popup with focus trap.
- **radio_group(Signal\<String\>)** — `.option(key, label)`;
  keyboard: arrows wrap, Home/End, focus ring on checked row.
- **command_menu()** — `.item(label, action)`, `.submenu(...)`,
  `.select(Fn(&str))`.

## Data + display

- **table(TableState\<Row\>)** — the data tier: sticky sortable header,
  virtualized body (100k rows), selection styling + hover, keyboard
  nav, AccessKit grid roles. `TableColumn::new(id, title)` +
  `.sortable()`, `.filterable()`, `.cell()`, `.width(ColWidth)`.
- **virtual_list(Vec\<T\>, render)** — windowed rendering, uniform or
  variable row heights, `.items_bind(signal)` for reactive items
  (renders only the viewport, in-place scroll), selection modes,
  keyboard nav.
- **progress(Signal\<f32\>, max)** / **gauge(Signal\<f32\>)**.
- **chip(label, ChipTone)** — `.tone_signal(...)` for reactive retint.
- **icon(glyph)**, **dot(Tone)**, **divider_horizontal/vertical()**.
- **code_edit(...)**, **floating_text(...)** (game FCT), **turn_strip(...)** (game turn order).

## Containers + overlays

- **panel()** — `.title()`, `.padding()`, `.elevation()`.
- **splitter(Orientation)** — `.child(node, size)`, `.min_size/.max_size`,
  `.snap_points()`, drag handles.
- **tabs(Signal\<usize\>, Vec\<String\>)** — `.content_for(i, node)`,
  `.close(Fn(usize))`.
- **scroll(inner, extent)** — `.width()/.height()`.
- **modal()** — `.show(Signal)`, `.title()`, `.dismiss_on_backdrop()`,
  focus trap + restore, Escape.
- **menu(Signal\<bool\>)** — `.item(MenuItem::new(..).shortcut(..))`,
  `.separator()`.
- **tooltip(label)** — `.shortcut()`, wraps a target child.

## Interaction state support (the honest table)

| Surface | State closures paint? |
|---|---|
| button | yes (all four slots) |
| checkbox, dropdown, slider, tabs, text_field, text_edit | yes |
| Plain containers | via `interactive()` only |
| gauge, progress, chip, dot, floating_text, turn_strip, icon, divider, panel, scroll, modal, menu, command_menu, tooltip | compile error by design (display-only) |
