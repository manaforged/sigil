# The widget catalog

The catalog ships in `sigil`. Buttons, fields, tables, lists, charts,
menus, and overlays are the default path — not a second crate.

Every widget constructs from a lowercase fn and chains a builder. All of
them take `class`/`style`/fluent styling, `WidgetLayoutExt` sizing, and
participate in focus, a11y, and IME where relevant. Constructor
highlights below; see rustdoc for the full builder surface.

## Inputs

- **button(label)** — `label` is `TextContent` (`&str` or
  `Signal<String>`). `.primary()/.neutral()/.destructive()`,
  `.tone(Tone)`, `.solid()/.soft()`, `.disabled(Signal)`,
  `.activate(Fn())`. Full state-closure support
  (`.hover()/.focus()/.pressed()/.disabled_style()`).
  `.accessible("Play")` when the visible label is a glyph.
- **icon_button(glyph, activate)** — ghost `button` whose label is a
  Unicode glyph. Chain `.accessible("Play")` for assistive name.
- **button(...).pending(Signal\<bool\>)** — paints disabled and ignores
  activate while the signal is true.
- **checkbox(Signal\<bool\>)** — `.label()`, `.tone(Tone)`,
  `.change(Fn(bool))`, `.disabled(Signal)`. Neutral is gray; every
  other tone uses the accent checked fill.
- **toggle(Signal\<bool\>)** — `.label()`, `.change(Fn(bool))`.
- **slider(Signal\<f32\>, Range)** — `.step(f32)`, `.change(Fn(f32))`,
  orientation; publishes step for AT Increment/Decrement.
- **text_field(Signal\<String\>)** — `.placeholder()`,
  `.change(Fn(&str))` (side effect; the signal is already two-way),
  `.submit()`, `.clipboard()`, `.disabled(Signal)`,
  `.invalid_when(Signal\<bool\>)`,
  `.password()` (mask glyphs with `*`; value stays plaintext),
  `.reveal(Signal<bool>)` (show plaintext while true),
  IME (preedit inline, caret anchor published). Pass a `Signal<String>`
  for the field value.
- **text_edit(Signal\<String\>)** — multi-line: same surface plus
  newline entry, vertical caret (↑/↓), shift-selection.
- **number_field(Signal\<T\>, Range)** — numeric text field.
- **date_field(Signal\<Date\>)** — ISO `YYYY-MM-DD`.
- **color_field(Signal\<Rgba8\>)** — hex (`#rgb` / `#rrggbb`) plus a swatch.
- **dropdown(Signal\<usize\>, Vec\<String\>)** — `.change(Fn(usize))`,
  `.disabled(Signal)`; trigger + popup with focus trap.
- **radio_group(Signal\<String\>)** — `.option(key, label)`;
  keyboard: arrows wrap, Home/End, focus ring on checked row.
- **command_menu()** — nested action tree: `.item(label, action)`,
  `.submenu(...)`, `.select(Fn(&str))`. Not a searchable Cmd-K
  palette. That shape is `text_field` + `virtual_list` inside
  `modal` / `sheet` (recipe below).

## Data + display

- **table(TableState\<Row\>)** — the data tier: sticky sortable header,
  virtualized body (100k rows), selection styling + hover, keyboard
  nav, AccessKit grid roles. `TableColumn::new(id, title)` +
  `.sort_by(|row| row.name.clone())` (or `.sortable` for a custom
  `Ordering`). `TableState::sort(id, dir)` applies it;
  `sort_key()` is the signal; `toggle_sort` cycles the header.
  `.filterable()`,
  `.cell(|row| text(…))` (any `IntoNode`), `.width(ColWidth)`,
  `.min_width(px)`. Implement `Identifiable` on the row and
  construct with `TableState::identified(rows, cols)` so `replace_rows`
  keeps the row you clicked. `TableState::new` is index-only.
  `selected_row()` / `selected_index()` subscribe with `get`.
  `.density(Density::Compact)` for 28 px rows. Empty filter shows
  "No rows"; override with `.empty(|| …)`.
- **chart(values)** — bar by default, `.line()` for a polyline.
  `.tone(Tone)`, `.labels(["Mon", …])`, `.series(name, values, Tone)`
  for more marks. Color is the theme scale (step 9). `.width` /
  `.height` pin the plot.
- **virtual_list(items, render)** — windowed list. `items` is a
  `Vec` (snapshot) or a `Signal` (live). Default keys are list
  indices. `.identified()` keys by `Identifiable::id`. `.key_by`
  for any other domain key. `.auto_pin_to_bottom(true)` keeps a
  live list on the last row when it grows.
- **progress(Signal\<f32\>, max)** / **gauge(Signal\<f32\>)**.
- **chip(label, Tone)** — same `Tone` as buttons. `.tone_signal(...)`
  for reactive retint. `.activate(Fn())` makes it a button. Without
  `.activate` it stays a label.
- **badge(label, Tone)** — pill chip (`rounded-full`).
- **field(label, control)** / **validated_field** — label above a
  control. Use these for forms.
- **icon(glyph)**, **dot(Tone)**, **divider_horizontal/vertical()**.
- **code_edit(...)**, **floating_text(...)** (game FCT), **turn_strip(...)** (game turn order).
- **markdown(Signal\<String\>)** — read-only document: headings, paragraphs,
  lists, fenced code, pipe tables, plus `**bold**` / `` `code` `` /
  `[label](url)`. Not an in-place editor.
- **html(Signal\<String\>)** — the same nodes as `markdown`, from an HTML
  subset (`h1`–`h3`, `p`, `ul`/`ol`/`li`, `pre`/`code`, `a`, `strong`/`em`,
  `table`). Scripts and styles are dropped. Not a browser.
- **markdown_edit(Signal\<String\>)** — source `text_edit` plus a live
  `markdown` preview. The caret stays in the source string.
- **transport(playing, progress, toggle)** — play/pause `icon_button` plus
  a 0–1 `slider`. Bind a prelude `Player` (feature `audio`). Open a
  file with `Player::play_path`. Chain `.seek(|r| …)` with
  `Player::seek` so a drag jumps the file.
- **terminal(lines, on_input)** — mono `virtual_list` of PTY rows, pinned
  to the bottom. Bind a prelude `Session` (feature `pty`). Paint is
  `Session.lines`. `Session::write` needs `Session::spawn`.
- **surface(key)** — layout hole for a native view. After layout,
  `Host::surfaces` returns the rect. The GPU does not paint it.

## Containers + overlays

- **field(label, control)** lives with the widgets; `patterns::` still
  has `app_shell`, `kv_grid`, `hint_overlay`.
- **card(child)** — padded surface.
- **avatar(initials)** — initials disc.
- **sidebar(child)** / **sidebar_width(px, child)** — fixed-width nav
  column.
- **panel()** — `.title()`, `.padding()`, `.elevation()`.
- **splitter(Orientation)** — `IntoNode`. `.pane(node).initial(px).min(px).grow(1.0)`.
  A grow pane takes leftover; the sash moves the fixed neighbour.
  `.width()` / `.height()` on the chain size the splitter.
  Type a helper as `impl IntoNode`. `.into()` yields `NodeBuilder`.
- **modal(open)** — GPU dialog. `open` is `Signal<bool>`.
  `.title()`, `.width()` / `.height()` size the frame. Escape and
  focus trap on. Backdrop click off unless `.dismiss_on_backdrop()`.
  Native alert is `sigil::dialog::message` / `confirm`.
- **sheet(open)** — same builder, docked. Default
  `.side(Side::Right)`. `.side(Side::Left)` or `.side(Side::Bottom)`.
  Backdrop click dismisses.
- **toast(message)** — `Signal<Option<String>>`. `Some` shows a
  status line; `None` hides it.
- **tabs(Signal\<usize\>, Vec\<String\>)** — `.content_for(i, node)`,
  `.close(Fn(usize))`.
- **scroll(inner, extent)** — `.width()/.height()`.
- **menu_bar()** — GPU app bar. `.class("bg-surface-hover px-3")` styles
  the strip. `.menu("File", [MenuItem::new("Open").select(|| {})])` is a
  dropdown. This is the bar you design. Native OS bar is opt-in:
  `sigil::menu::set_menu_bar` (next frame).
- **menu(Signal\<bool\>)** — GPU popup panel (`.item`, `.separator()`).
  Context menus, not the app bar.
- **tooltip(label)** — `.shortcut()`, wraps a target child.

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

Extract a helper only when a second app copies this.

## Interaction state support (the honest table)

| Surface | State closures paint? |
|---|---|
| button, icon_button | yes (all four slots) |
| checkbox, dropdown, slider, tabs, text_field, text_edit | yes |
| Plain containers, card, avatar, sidebar | via `interactive()` only |
| chip | no style slots; `.activate` is a clickable button |
| gauge, progress, dot, floating_text, turn_strip, icon, divider, panel, scroll, modal, menu, command_menu, tooltip | compile error by design (display-only) |
