# csv-viewer — friction log

Dogfood v2 of a viewer-class app on Sigil. Built a virtualised CSV table
with a fixed header row, ~5k synthesised rows, single-row click-to-select.

## Time-to-first-paint estimate

About 30 minutes of *typing* — the heartbeat shows `started` to
`impl-green` at roughly half an hour of wall clock. Almost all of that
was reading existing examples (especially `log-tailer`) and the
`virtual_list` mod source to confirm the builder surface. If I had to
build this cold without the sibling `log-tailer` to copy from, it would
have been closer to 90 minutes.

## Discovered by grepping source

Even with the prelude expanded, several things still required a grep:

- `examples/log-tailer/src/view.rs:8` — only place I could see that
  `virtual_list` and `SelectionMode` live in `sigil::widgets::` and
  not `sigil::prelude::*`. The prelude re-exports
  `VirtualListBuilder` (the struct) and `virtual_list` (the fn) but
  not `SelectionMode`, which means every consumer writes the same
  extra `use` line. (`crates/sigil/src/lib.rs:99-107`.)
- `crates/sigil/src/widgets/composition.rs:37` — `status_bar(children: I)`
  takes an iterator-of-nodes shape rather than the more common
  builder-chain shape. Useful but not discoverable from the prelude
  doc.
- `crates/sigil/src/widgets/virtual_list/mod.rs:194` — the
  `.select(|index| …)` shorthand exists and saved me an `effect`
  wrap. Not mentioned in the integration tests or the log-tailer; I
  found it only by reading the builder source top-to-bottom.
- `crates/sigil/src/widgets.rs:34` — `pub use composition::{kv_row,
  status_bar};` — the composition helpers exist but the file name
  doesn't suggest it; I expected them under `layout` or `panel`.

## virtual_list ergonomics

This is the load-bearing tier-2 primitive. Specific notes:

1. **Items must be owned `Vec<T>`, not `&[T]` or `Signal<Vec<T>>`.**
   `crates/sigil/src/widgets/virtual_list/mod.rs:74-78`. For a static
   corpus I have to `state.rows.to_vec()` on every build — a 5k×String
   clone per frame is fine here but is obviously wrong at scale. The
   `auto_pin_to_bottom` placeholder doc on line 218 admits this is on
   the roadmap; the comment promising `Signal<Vec<T>>` should be a
   first-class item-source variant before this gets pushed harder.
2. **The render closure receives `&T` but no row-level layout context.**
   I want to know "am I the selected row?" inside the row builder so
   I can paint a highlight class. Today the row closure has to
   capture the `Signal<Option<usize>>` itself and compute the
   highlight outside the list's selection machinery. The list has
   its own `SelectionState` signal — I should be able to write
   `.row_class(|index, state| if state.is_selected(index) {
   "bg-selection" } else { "" })` without owning a second signal.
3. **`.select(callback)` and `selection_state()` are two parallel
   surfaces** that both subscribe to the same internal state. I used
   `.select` because it was terser, but then my row closure couldn't
   read the live selection (the `selected` signal I bound separately
   is the source of truth, and the list also tracks one internally
   via `SelectionState`). Two stores of the same fact = drift waiting
   to happen.
4. **No column / cell concept.** `virtual_list` is row-of-anything; a
   table is the host's problem. That's fine for the simple case but
   meant I had to hand-roll the header row with the exact same
   column widths as the body row, and any column-width change must
   be made in two places. A thin `virtual_table` wrapper on top of
   `virtual_list` that takes `columns: &[ColumnSpec]` + a per-cell
   render closure would erase the duplication.
5. **No sticky header.** I built my "header above body" by stacking
   two siblings in a flex column — the header doesn't scroll because
   the body is the only thing inside the virtualised viewport. That
   works for vertical scroll. The moment the body needs *horizontal*
   scroll (because columns overflow), the header won't scroll with
   it and the columns won't align.

## Missing widgets / primitives

- **`virtual_table`** — see virtual_list note 4.
- **A way to derive a row's selected state inside the row closure.**
  See virtual_list note 2.
- **`.class_when(signal, "bg-…")`** on `NodeBuilder` — every example I
  copied implements row highlighting by passing a `Signal` *into* the
  row builder and computing the class inline. A `class_when` toggle
  built on a derived signal would fold this into one line.
- **`hover_class`** — a viewer that doesn't show pointer feedback
  feels dead. The interaction layer presumably has `:hover`; a
  utility-class shorthand for "this class is applied on hover" would
  save the manual `InteractionState` plumbing.
- **`fixed_size` width on `text(...)`** — `text_field` has no
  `.layout()`, which forces the wrap-in-container dance the
  log-tailer also flagged. The reason I had a `column_width` helper
  at all is that I wrapped every `label(cell)` in a sized container.

## Confusing names

- `LayoutStyle::block()` — comes from CSS but my brain reads
  "scrolling viewport". In a Rust UI framework that doesn't pretend
  to be a browser, `LayoutStyle::leaf()` or `LayoutStyle::box_()`
  would carry intent better.
- `NodeBuilder::class("p-4 bg-surface")` — string-typed utility
  classes feel right for prototyping but the missing/typo case is
  silent. I tried `"bg-selection"` to highlight the selected row,
  shrugged when nothing visible happened, and would have to dig
  through `sigil-style` to find out which class names actually exist.
  A `class!("bg-selection")` macro that validates at compile time
  against the registered atom set would catch typos at edit time.
- `Role::Row` vs `Role::ListItem` — I used `Row` because the table
  is the more accurate semantic, but the log-tailer used `ListItem`
  for its single-column rows. There's a doc on
  `crates/a11y/` somewhere that picks for me; the prelude doesn't
  hint which to reach for.

## Boilerplate copy-paste

Three pieces of the log-tailer I cloned verbatim:

1. The `#[derive(Copy)] AppState { signals... }` pattern with the
   `state.rows = Box::leak(generate(N).into_boxed_slice())`
   `&'static` trick. Every viewer-class app is going to do this; a
   `sigil::state!{ ... }` macro that handles the leak + the
   per-field-signal boilerplate would shave 20 lines off every
   `state.rs`.
2. The `with_cx("scope-name", |cx| { ... })` test scaffolding. Every
   example writes the same pattern.
3. The `derived(move || selected.get().map_or_else(|| ..., |i| ...))`
   for the status-bar selection label. Common enough to deserve a
   `text_for(selected, |opt| ...)` helper.

## Surprised-good

- **`.pointer_down(move |_| selected.set(Some(index)))`** is exactly
  the API I'd write. No `Cx`, no `EventHandler`, no `'static` lifetime
  spelunking — closure captures a `Signal<Option<usize>>` (which is
  `Copy`) and the click event Just Works.
- **Signals are `Copy`** so `AppState` can be `Copy` and pass-by-value
  through the whole view tree. The log-tailer flags this on a comment
  in its `state.rs`; it really does cut the lint noise to near zero.
- **`virtual_list(items, render).row_height(N).viewport_height(M)`** —
  the builder reads top-to-bottom in the order you make the
  decisions. No `Default::default()` struct shape. Good.
- **`.class("p-4 bg-surface rounded-md")`** parsing one string into N
  atoms is friction at the typo-validation level (see Confusing
  names) but is *delightful* to write — far more compact than the
  per-method form.

## 3-5 concrete API change proposals

1. **`virtual_table(columns, rows, cell)`** — wrap `virtual_list` with
   a `columns: &[ColumnSpec { header, width }]` argument and a
   per-cell render closure `Fn(usize_row, usize_col, &T) ->
   NodeBuilder`. Body rows and the pinned header derive from the
   same `ColumnSpec` slice, fixing the dual-declaration footgun. The
   table renders the header internally and the host doesn't author
   it.

2. **Row-aware selection callback**: change the row closure signature
   to `Fn(usize, &T, RowContext) -> NodeBuilder` where `RowContext`
   carries `is_selected: bool`, `is_focused: bool`,
   `is_hovered: bool`. Host writes
   `if ctx.is_selected { row.class("bg-selection") } else { row }`
   without having to thread its own signal through. The list's
   internal `SelectionState` becomes the single source of truth.

3. **`NodeBuilder::class_when(signal, &str)`** — toggle a class string
   based on a boolean-derived signal. Today every example builds a
   `derived(move || if foo.get() { "..." } else { "" })` and feeds
   that into `.class(reactive_class_signal)`; the helper would
   collapse that to one line.

4. **`Signal<Vec<T>>` items source on `virtual_list`** — the
   `auto_pin_to_bottom` doc on line 218 of `virtual_list/mod.rs`
   already admits this is missing. Until it lands, every viewer with
   a dynamic dataset will clone its corpus on every paint.

5. **Compile-time-checked `class!(…)` macro** — accept the same
   utility-class string but resolve atoms at macro-expand time
   against the `sigil-style` atom registry. Unknown atoms become
   compile errors. Pairs naturally with `style!` for the typed
   per-method form. Saves the silent-no-op debugging dance when a
   class name is misspelled.

## Anything skipped

- I did not implement column sorting, filter, or column-width drag.
  The brief said viewer-class read-only; those are the next tier.
- I did not launch the binary interactively — this was a headless
  cargo-only dogfood pass. The `main()` wires the `launch` call
  correctly per the `hello-counter` exemplar but I cannot claim "I
  saw the table render" without screenshots.
